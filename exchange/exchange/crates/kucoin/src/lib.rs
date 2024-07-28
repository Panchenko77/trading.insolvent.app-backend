use serde_json::{json, Value};
// pub mod execution;
pub mod market;
pub mod urls;
pub mod symbols;
pub fn encode_subscribe(id: &str, operation: &str, topic: &str) -> Value {
    json!({
        "id": id,
        "type": operation,
         "topic": topic,
        "response": true


        }
    )
}




