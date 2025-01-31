use crate::{contracttype, Address, BytesN, RawVal, Symbol, Vec};

#[contracttype(crate_path = "crate", export = false)]
#[derive(Clone)]
pub struct Context {
    pub contract: BytesN<32>,
    pub fn_name: Symbol,
    pub args: Vec<RawVal>,
}

impl Context {
    pub fn contract(&self) -> Address {
        Address::from_contract_id(&self.contract)
    }
}
