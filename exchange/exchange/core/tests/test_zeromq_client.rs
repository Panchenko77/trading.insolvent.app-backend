use exchange_core::utils::zeromq::{ZmqClient, ZmqRequest, ZmqResponse};
use eyre::Result;
use serde::{Deserialize, Serialize};
use std::process::Stdio;
use tokio::process::{Child, Command};

#[derive(Serialize, Deserialize)]
pub struct Message(pub String);

impl ZmqRequest for Message {
    fn to_string(&self) -> String {
        self.0.clone()
    }
}

impl ZmqResponse for Message {
    fn from_str(s: &str) -> Result<Self> {
        Ok(Message(s.to_string()))
    }
}

#[must_use]
fn spawn_js_server(name: &str, args: Vec<String>) -> Result<Child> {
    let process = Command::new("node")
        .arg(name)
        .args(args)
        .kill_on_drop(true)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn();
    Ok(process?)
}

#[tokio::test]
async fn test_zeromq_client() -> Result<()> {
    let _js_server = spawn_js_server("js/zeromq_echo_server.js", vec![])?;
    let mut client = ZmqClient::new();
    client.subscribe("tcp://127.0.0.1:5555").await?;
    let request = Message("hello".to_string());
    let response: Message = client.request(request).await?;
    assert_eq!(response.0, "hello");

    Ok(())
}

#[tokio::test]
async fn test_zeromq_repl_client() -> Result<()> {
    let ipc_path = "repl_server.ipc";
    let _js_server = spawn_js_server("js/zeromq_repl_server.js", vec![ipc_path.to_string()])?;
    // Wait for the server to start
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    let mut client = ZmqClient::new();
    client.subscribe(&format!("ipc://{}", ipc_path)).await?;

    let request = Message("return 1 + 1".to_string());
    let response: Message = client.request(request).await?;
    assert_eq!(response.0, "2");

    let func_get_google_page = r#"
        const response = await fetch('https://www.google.com');
        const text = await response.text();
        return text;
    "#;
    let request = Message(func_get_google_page.to_string());
    let response: Message = client.request(request).await?;
    assert!(response.0.contains("<!doctype html>"));
    Ok(())
}
