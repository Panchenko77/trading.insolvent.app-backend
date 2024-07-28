pub enum API {
    Info,
    Exchange,
}
impl API {
    pub fn as_str(&self) -> &str {
        match self {
            API::Info => "/info",
            API::Exchange => "/exchange",
        }
    }
}

pub mod agent;

pub mod usd_transfer;

pub mod info;

pub mod exchange;

pub mod websocket;
