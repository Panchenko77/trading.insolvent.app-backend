use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
pub struct Resting {
    pub oid: u64,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Filled {
    pub oid: u64,
    pub total_sz: String,
    pub avg_px: String,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub enum Status {
    Resting(Resting),
    Filled(Filled),
    Error(String),
    Success,
    WaitingForFill,
    WaitingForTrigger,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Statuses {
    pub statuses: Vec<Status>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Data {
    #[serde(rename = "type")]
    pub type_: String,
    pub data: Option<Statuses>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase", tag = "status", content = "response")]
pub enum Response {
    Ok(Data),
    Err(String),
}
