use crate::task::{Task, TaskState};
use core_affinity::CoreId;
use eyre::ContextCompat;
use futures::future::LocalBoxFuture;
use futures::FutureExt;
use std::future::Future;
use tokio_util::sync::CancellationToken;

pub type AsyncFnOnce = Box<dyn FnOnce() -> LocalBoxFuture<'static, TaskState> + Send>;
pub struct TaskBuilder {
    pub name: String,
    core_id: Option<CoreId>,
    run: Option<AsyncFnOnce>,
    on_drop: Option<Box<dyn FnOnce(TaskState) + Send>>,
    cancel: Option<CancellationToken>,
}

impl TaskBuilder {
    pub fn new(name: String) -> Self {
        TaskBuilder {
            name,
            core_id: None,
            run: None,
            on_drop: None,
            cancel: None,
        }
    }
    pub fn with_task<F, Fut>(mut self, run: F) -> Self
    where
        F: FnOnce() -> Fut + Send + 'static,
        Fut: Future<Output = ()> + 'static,
    {
        self.run = Some(Box::new(move || {
            let fut = run();
            async move {
                fut.await;
                TaskState::Finished
            }
            .boxed_local()
        }));
        self
    }
    pub fn with_task_status<F, Fut>(mut self, run: F) -> Self
    where
        F: FnOnce() -> Fut + Send + 'static,
        Fut: Future<Output = TaskState> + 'static,
    {
        self.run = Some(Box::new(move || run().boxed_local()));
        self
    }
    pub fn with_task_result<F, Fut>(mut self, run: F) -> Self
    where
        F: FnOnce() -> Fut + Send + 'static,
        Fut: Future<Output = Result<(), eyre::Report>> + 'static,
    {
        let name = self.name.clone();
        self.run = Some(Box::new(move || {
            let fut = run();
            async move {
                match fut.await {
                    Ok(..) => TaskState::Finished,
                    Err(e) => {
                        tracing::error!("Task ({name}) errored: {:?}", e);
                        TaskState::Error
                    }
                }
            }
            .boxed_local()
        }));
        self
    }
    pub fn with_future<Fut>(mut self, run: Fut) -> Self
    where
        Fut: Future<Output = ()> + Send + 'static,
    {
        self.run = Some(Box::new(move || {
            async move {
                run.await;
                TaskState::Finished
            }
            .boxed_local()
        }));
        self
    }
    pub fn with_future_result<Fut>(mut self, run: Fut) -> Self
    where
        Fut: Future<Output = Result<(), eyre::Report>> + Send + 'static,
    {
        self.run = Some(Box::new(move || {
            async move {
                match run.await {
                    Ok(..) => TaskState::Finished,
                    Err(e) => {
                        tracing::error!("Task errored: {:?}", e);
                        TaskState::Error
                    }
                }
            }
            .boxed_local()
        }));
        self
    }
    pub fn with_core_id(mut self, core_id: Option<CoreId>) -> Self {
        self.core_id = core_id;
        self
    }
    pub fn with_on_drop<F>(mut self, on_drop: F) -> Self
    where
        F: FnOnce(TaskState) + Send + 'static,
    {
        self.on_drop = Some(Box::new(on_drop));
        self
    }
    pub fn with_cancel_token(mut self, cancel: CancellationToken) -> Self {
        self.cancel = Some(cancel);
        self
    }
    pub fn spawn(self) -> Task {
        let TaskBuilder {
            name,
            core_id,
            run,
            on_drop,
            cancel,
        } = self;

        let mut run = run.with_context(|| format!("Task of {} is not set", name)).unwrap();

        if let Some(cancel) = cancel {
            run = Box::new(move || {
                let cancel = cancel.clone();
                async move {
                    tokio::select! {
                        result = run() => result,
                        _ = cancel.cancelled() => TaskState::Cancelled
                    }
                }
                .boxed_local()
            });
        }
        let join = std::thread::Builder::new()
            .name(name.clone())
            .spawn(move || {
                let mut helper = DropHelper {
                    name,
                    // set the initial state to Panic so that we can report if the task panics
                    state: TaskState::Panic,
                    on_drop,
                };
                // set core affinity
                if let Some(core_id) = core_id {
                    core_affinity::set_for_current(core_id);
                }

                let state = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .unwrap()
                    .block_on(run());
                helper.state = state;
                state
            })
            .unwrap();
        Task::new(join)
    }
}

struct DropHelper {
    name: String,
    state: TaskState,
    on_drop: Option<Box<dyn FnOnce(TaskState) + Send>>,
}
impl Drop for DropHelper {
    fn drop(&mut self) {
        let state = std::mem::replace(&mut self.state, TaskState::Finished);
        state.print(&self.name);
        if let Some(on_drop) = self.on_drop.take() {
            on_drop(state);
        }
    }
}
