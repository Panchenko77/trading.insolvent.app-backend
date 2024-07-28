use eyre::Result;
use futures::future::OptionFuture;
use std::sync::atomic::{AtomicU32, Ordering};
use tokio::io::AsyncBufReadExt;
use tokio::process::{Child, ChildStderr, ChildStdout, Command};
use tracing::{debug, debug_span, error, info, Instrument};

static TASK_ID: AtomicU32 = AtomicU32::new(1);

type ExitReceiver = tokio::sync::oneshot::Receiver<()>;
type ExitSender = tokio::sync::oneshot::Sender<()>;
pub type OutputHandler = Box<dyn FnMut(String) + Send>;

struct JsProcessTask {
    stdout: ChildStdout,
    stderr: ChildStderr,
    child: Child,
    command: String,
    task_id: u32,
    exit: ExitReceiver,
    handler: OutputHandler,
}

impl JsProcessTask {
    pub fn new(exit: ExitReceiver, path: &str, args: &[String]) -> Self {
        let program = if path.ends_with(".ts") {
            "ts-node"
        } else {
            "node"
        };
        let command = format!("{} {} {}", program, path, args.join(" "));
        let task_id = TASK_ID.fetch_add(1, Ordering::Relaxed);

        info!(?task_id, "Starting node process: {}", command);
        let mut child = Command::new(program)
            .arg(path)
            .args(args)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .stdin(std::process::Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .expect("failed to start node process");
        let stdout = child.stdout.take().expect("failed to open stdout");
        let stderr = child.stderr.take().expect("failed to open stderr");

        Self {
            stdout,
            stderr,
            child,
            command,
            task_id,
            exit,
            handler: Box::new(|_| {}),
        }
    }
    pub async fn run(self) -> Result<()> {
        let span = debug_span!("js", task_id = self.task_id);
        let JsProcessTask {
            stdout,
            stderr,
            mut child,
            exit,
            mut handler,
            ..
        } = self;
        let mut exit: OptionFuture<_> = Some(exit).into();
        let mut process_live = true;
        let mut stdout = tokio::io::BufReader::new(stdout).lines();
        let mut stdout_live = true;
        let mut stderr = tokio::io::BufReader::new(stderr).lines();
        let mut stderr_live = true;

        async {
            while stdout_live || stderr_live || process_live {
                tokio::select! {
                    line = stdout.next_line(), if stdout_live => {
                        if let Some(line) = line? {
                            debug!("stdout: {}", line);
                            handler(line);
                        } else {
                            stdout_live = false;
                        }
                    }
                    line = stderr.next_line(), if stderr_live => {
                        if let Some(line) = line? {
                            error!("stderr: {}", line);
                            handler(line);
                        } else {
                            stderr_live = false;
                        }
                    }
                    exit = child.wait(), if process_live => {
                        let exit = exit?;
                        if exit.success() {
                            info!("process exited: {:?}", exit);
                        } else {
                            error!("process exited: {:?}", exit);
                        }
                        process_live = false;
                    }
                    // dropping it also has the same effect
                    _ = &mut exit => {
                        exit = None.into();
                        info!("exit signal received, killing process");
                        child.kill().await?;

                    }
                }
            }
            Ok(())
        }
        .instrument(span)
        .await
    }
}

pub struct JsProcess {
    command: String,
    task_id: u32,
    // when this is dropped, the process is killed
    #[allow(dead_code)]
    oneshot: ExitSender,
}

impl JsProcess {
    pub fn new(path: &str, args: &[String]) -> Result<Self> {
        Self::new_with_handler(path, args, Box::new(|_| {}))
    }
    pub fn new_with_handler(path: &str, args: &[String], handler: OutputHandler) -> Result<Self> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        let mut task = JsProcessTask::new(rx, path, args);
        task.handler = handler;
        let command = task.command.clone();
        let task_id = task.task_id;
        {
            let task_id = task.task_id;
            tokio::spawn(async move {
                let result = task.run().await;
                if let Err(err) = result {
                    error!(?task_id, ?err, "js process failed");
                }
            });
        }
        Ok(Self {
            command,
            task_id,
            oneshot: tx,
        })
    }
    pub fn stop(self) {
        drop(self)
    }
}

impl Drop for JsProcess {
    fn drop(&mut self) {
        let task_id = self.task_id;
        info!(?task_id, "JsProcess::drop: {}", self.command);

        // let bt = Backtrace::capture();
        // info!("{}", bt);
    }
}
