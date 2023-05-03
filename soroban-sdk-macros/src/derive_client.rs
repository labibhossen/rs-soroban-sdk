use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{spanned::Spanned, Error, FnArg, Path, Type, TypePath};

use crate::syn_ext;

pub fn derive_client(crate_path: &Path, ty: &str, name: &str, fns: &[syn_ext::Fn]) -> TokenStream {
    let ty_str = quote!(#ty).to_string();
    // Map the traits methods to methods for the Client.
    let mut errors = Vec::<Error>::new();
    let fns: Vec<_> = fns
        .iter()
        .map(|f| {
            let fn_ident = &f.ident;
            let fn_try_ident = format_ident!("try_{}", &f.ident);
            let fn_name = fn_ident.to_string();

            // Check for the Env argument.
            let env_input = f.inputs.first().and_then(|a| match a {
                FnArg::Typed(pat_type) => {
                    let ty = &*pat_type.ty;
                    if let Type::Path(TypePath {
                        path: syn::Path { segments, .. },
                        ..
                    }) = ty
                    {
                        if segments.last().map_or(false, |s| s.ident == "Env") {
                            Some(a)
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                }
                FnArg::Receiver(_) => None,
            });

            // Map all remaining inputs.
            let (fn_input_names, fn_input_types): (Vec<_>, Vec<_>) = f
                .inputs
                .iter()
                .skip(if env_input.is_some() { 1 } else { 0 })
                .map(|t| {
                    let ident = match syn_ext::fn_arg_ident(t) {
                        Ok(ident) => ident,
                        Err(_) => {
                            errors.push(Error::new(t.span(), "argument not supported"));
                            format_ident!("")
                        }
                    };
                    (ident, syn_ext::fn_arg_make_ref(t))
                })
                .unzip();
            let fn_output = f.output();
            let fn_try_output = f.try_output(crate_path);
            let fn_attrs = f.attrs;
            quote! {
                #(#fn_attrs)*
                pub fn #fn_ident(&self, #(#fn_input_types),*) -> #fn_output {
                    // TODO: Undo the mock and restore previous auth state after
                    // https://github.com/stellar/rs-soroban-env/issues/785 is
                    // implemented.
                    #[cfg(any(test, feature = "testutils"))]
                    {
                        if let Some(set_auths) = self.set_auths {
                            self.env.set_auths(set_auths);
                        }
                        if let Some(mock_auths) = self.mock_auths {
                            self.env.mock_auths(mock_auths);
                        }
                        if self.mock_all_auths {
                            self.env.mock_all_auths();
                        }
                    }
                    use #crate_path::{IntoVal,FromVal};
                    self.env.invoke_contract(
                        &self.contract_id,
                        &#crate_path::Symbol::new(&self.env, &#fn_name),
                        #crate_path::vec![&self.env, #(#fn_input_names.into_val(&self.env)),*],
                    )
                }

                #(#fn_attrs)*
                pub fn #fn_try_ident(&self, #(#fn_input_types),*) -> #fn_try_output {
                    #[cfg(any(test, feature = "testutils"))]
                    // TODO: Undo the mock and restore previous auth state after
                    // https://github.com/stellar/rs-soroban-env/issues/785 is
                    // implemented.
                    #[cfg(any(test, feature = "testutils"))]
                    {
                        if let Some(set_auths) = self.set_auths {
                            self.env.set_auths(set_auths);
                        }
                        if let Some(mock_auths) = self.mock_auths {
                            self.env.mock_auths(mock_auths);
                        }
                        if self.mock_all_auths {
                            self.env.mock_all_auths();
                        }
                    }
                    use #crate_path::{IntoVal,FromVal};
                    self.env.try_invoke_contract(
                        &self.contract_id,
                        &#crate_path::Symbol::new(&self.env, &#fn_name),
                        #crate_path::vec![&self.env, #(#fn_input_names.into_val(&self.env)),*],
                    )
                }
            }
        })
        .collect();

    // If errors have occurred, render them instead.
    if !errors.is_empty() {
        let compile_errors = errors.iter().map(Error::to_compile_error);
        return quote! { #(#compile_errors)* };
    }

    // Render the Client.
    let client_doc = format!("{name} is a client for calling the contract defined in {ty_str}.");
    let client_ident = format_ident!("{}", name);
    quote! {
        #[doc = #client_doc]
        pub struct #client_ident<'a> {
            pub env: #crate_path::Env,
            pub contract_id: #crate_path::BytesN<32>,
            #[doc(hidden)]
            #[cfg(not(any(test, feature = "testutils")))]
            _phantom: core::marker::PhantomData<&'a ()>,
            #[doc(hidden)]
            #[cfg(any(test, feature = "testutils"))]
            set_auths: Option<&'a [#crate_path::xdr::ContractAuth]>,
            #[doc(hidden)]
            #[cfg(any(test, feature = "testutils"))]
            mock_auths: Option<&'a [#crate_path::testutils::MockAuth<'a>]>,
            #[doc(hidden)]
            #[cfg(any(test, feature = "testutils"))]
            mock_all_auths: bool,
        }

        impl<'a> #client_ident<'a> {
            pub fn new(env: &#crate_path::Env, contract_id: &impl #crate_path::IntoVal<#crate_path::Env, #crate_path::BytesN<32>>) -> Self {
                Self {
                    env: env.clone(),
                    contract_id: contract_id.into_val(env),
                    #[cfg(not(any(test, feature = "testutils")))]
                    _phantom: core::marker::PhantomData,
                    #[cfg(any(test, feature = "testutils"))]
                    set_auths: None,
                    #[cfg(any(test, feature = "testutils"))]
                    mock_auths: None,
                    #[cfg(any(test, feature = "testutils"))]
                    mock_all_auths: false,
                }
            }

            pub fn address(&self) -> #crate_path::Address {
                #crate_path::Address::from_contract_id(&self.contract_id)
            }

            #[cfg(any(test, feature = "testutils"))]
            pub fn set_auths(&self, auths: &'a [#crate_path::xdr::ContractAuth]) -> Self {
                Self {
                    env: self.env.clone(),
                    contract_id: self.contract_id.clone(),
                    set_auths: Some(auths),
                    mock_auths: self.mock_auths.clone(),
                    mock_all_auths: false,
                }
            }

            #[cfg(any(test, feature = "testutils"))]
            pub fn mock_auths(&self, mock_auths: &'a [#crate_path::testutils::MockAuth<'a>]) -> Self {
                Self {
                    env: self.env.clone(),
                    contract_id: self.contract_id.clone(),
                    set_auths: self.set_auths.clone(),
                    mock_auths: Some(mock_auths),
                    mock_all_auths: false,
                }
            }

            #[cfg(any(test, feature = "testutils"))]
            pub fn mock_all_auths(&self) -> Self {
                Self {
                    env: self.env.clone(),
                    contract_id: self.contract_id.clone(),
                    set_auths: None,
                    mock_auths: None,
                    mock_all_auths: true,
                }
            }

            #(#fns)*
        }
    }
}
