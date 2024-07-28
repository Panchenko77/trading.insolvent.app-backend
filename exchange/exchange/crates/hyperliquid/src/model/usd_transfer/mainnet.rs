use ethers::contract::{Eip712, EthAbiType};

#[derive(Eip712, Clone, EthAbiType)]
#[eip712(
    name = "Exchange",
    version = "1",
    chain_id = 42161,
    verifying_contract = "0x0000000000000000000000000000000000000000"
)]
pub struct UsdTransferSignPayload {
    pub destination: String,
    pub amount: String,
    pub time: u64,
}
