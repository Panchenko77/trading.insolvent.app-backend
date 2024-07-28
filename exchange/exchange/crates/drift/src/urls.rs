use trading_model::model::Network;

// copied from drift-labs/protocol-v2/program/drift
pub const DRIFT_PROGRAM_ID: &str = "dRiftyHA39MWEi3m9aunc5MzRF1JYuBsbn6VPcn33UH";
#[derive(Debug, Clone, Copy)]
pub enum Context {
    DevNet,
    MainNet,
}
pub fn get_dlob_endpoint(context: Context) -> &'static str {
    match context {
        Context::DevNet => "https://master.dlob.drift.trade",
        Context::MainNet => "https://dlob.drift.trade",
    }
}

pub fn create_context(network: Network) -> Context {
    match network {
        Network::Mainnet => Context::MainNet,
        Network::Devnet => Context::DevNet,
        _ => panic!("unsupported network: {}", network),
    }
}
pub fn http_to_ws(url: &str) -> Option<String> {
    let url = url
        .replace("https://", "wss://")
        .replace("http://", "ws://");
    Some(url)
}
