mod builder;
pub use builder::*;
mod registry;
pub use registry::*;

#[derive(Debug, Copy, Clone)]
pub enum TaskState {
    Pending,
    Running,
    Finished,
    Cancelled,
    Error,
    Panic,
}
impl TaskState {
    pub fn print(&self, name: &str) {
        match self {
            TaskState::Pending => {
                tracing::info!("Task {} is pending", name);
            }
            TaskState::Running => {
                tracing::info!("Task {} is running", name);
            }
            TaskState::Cancelled => {
                tracing::info!("Task {} is cancelled", name);
            }
            TaskState::Finished => {
                tracing::info!("Task {} is finished", name);
            }
            TaskState::Error => {
                tracing::error!("Task {} is errored", name);
            }
            TaskState::Panic => {
                tracing::error!("Task {} is panicked", name);
            }
        }
    }
}

pub struct Task {
    handle: std::thread::JoinHandle<TaskState>,
}
impl Task {
    pub fn new(handle: std::thread::JoinHandle<TaskState>) -> Self {
        Self { handle }
    }
    pub fn join_blocking(self) -> TaskState {
        self.handle.join().unwrap_or(TaskState::Panic)
    }
}
