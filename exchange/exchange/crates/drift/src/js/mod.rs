mod order;
mod position;

pub use order::*;
use std::sync::Arc;

use eyre::{bail, Context, Result};
use path_clean::PathClean;
use serde::de::DeserializeOwned;
use serde::Serialize;
use trading_exchange_core::utils::js::JsProcess;
use trading_exchange_core::utils::zeromq::ZmqClient;

pub const DRIFT_ZEROMQ: [&str; 3] = ["drift-zeromq.ts", "src/drift-zeromq.ts", "js/src/drift-zeromq.ts"];

fn get_drift_js_path() -> Result<String> {
    let path = std::env::var("DRIFT_JS_PATH").unwrap_or_else(|_| ".".to_string());
    let path = std::path::Path::new(&path)
        .canonicalize()
        .with_context(|| format!("failed to find drift js path: {}", path))?;
    for filename in DRIFT_ZEROMQ {
        let path = path.join(filename);
        if path.exists() {
            return Ok(path.to_string_lossy().to_string());
        }
    }
    bail!(
        "failed to find drift js path: {}/{{{:?}}}",
        path.display(),
        DRIFT_ZEROMQ
    )
}

pub const DEFAULT_DRIFT_ZEROMQ_IPC: &str = "drift-zeromq.ipc";

#[derive(Clone)]
pub struct DriftJsClient {
    zmq: ZmqClient,
    // if it drops, the js process exits
    #[allow(dead_code)]
    process: Arc<JsProcess>,
}

impl DriftJsClient {
    pub async fn new() -> Result<Self> {
        let path = get_drift_js_path()?;
        Self::with_js(&path, DEFAULT_DRIFT_ZEROMQ_IPC, "DRIFT").await
    }
    pub async fn with_env(env: &str) -> Result<Self> {
        let path = get_drift_js_path()?;
        let ipc_path = format!("drift-zeromq-{}.ipc", env);
        Self::with_js(&path, &ipc_path, env).await
    }
    pub async fn with_js(js_path: &str, ipc_path: &str, env: &str) -> Result<Self> {
        let ipc_path = std::path::Path::new(ipc_path).clean();
        let (ready_tx, ready_rx) = tokio::sync::oneshot::channel();

        let process = JsProcess::new_with_handler(
            js_path,
            &[ipc_path.to_string_lossy().to_string(), env.to_string()],
            Box::new({
                let mut ready_tx = Some(ready_tx);
                move |output| {
                    if output.contains("[Ready]") {
                        if let Some(ready_tx) = ready_tx.take() {
                            let _ = ready_tx.send(());
                        }
                    }
                }
            }),
        )
        .with_context(|| {
            format!(
                "failed to start drift js process with path: {} and ipc: {}",
                js_path,
                ipc_path.display()
            )
        })?;
        ready_rx.await.map_err(|_| {
            eyre::eyre!(
                "failed to start drift js process with path: {} and ipc: {}",
                js_path,
                ipc_path.display()
            )
        })?;
        let transport = format!("ipc://{}", ipc_path.display());
        let zmq = ZmqClient::new_dealer(&transport).await.with_context(|| {
            format!(
                "failed to connect to drift js process with path: {} and ipc: {}",
                js_path, transport
            )
        })?;
        let this = Self {
            zmq,
            process: Arc::new(process),
        };
        Ok(this)
    }

    pub async fn await_function_call<Resp: DeserializeOwned>(&self, function_name: &str) -> Result<Resp> {
        self.await_function_call_with_params(function_name, ()).await
    }
    pub async fn await_function_call_with_params<Req: serde::Serialize, Resp: DeserializeOwned>(
        &self,
        function_name: &str,
        args: Req,
    ) -> Result<Resp> {
        #[derive(Serialize)]
        struct Call<'a, T> {
            call: &'a str,
            params: &'a T,
        }
        let call = Call {
            call: function_name,
            params: &args,
        };
        let json = serde_json::to_string(&call)?;

        self.zmq
            .request_json(json.as_str())
            .await
            .with_context(|| format!("failed to call function: {}", function_name))
    }
}
