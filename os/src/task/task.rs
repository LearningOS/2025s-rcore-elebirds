//! Types related to task management

use super::TaskContext;

#[derive(Clone, Copy)]
pub struct SyscallStats {
    pub syscall_id: usize,
    pub count: usize,
}

impl SyscallStats {
    pub fn new(syscall_id: usize) -> Self {
        Self {
            syscall_id,
            count: 0,
        }
    }
}

/// The task control block (TCB) of a task.
#[derive(Copy, Clone)]
pub struct TaskControlBlock {
    /// The task status in it's lifecycle
    pub task_status: TaskStatus,
    /// The task context
    pub task_cx: TaskContext,
    /// 系统调用统计
    pub syscall_stats: [SyscallStats; crate::syscall::SYSCALL_COUNT],
}

/// The status of a task
#[derive(Copy, Clone, PartialEq)]
pub enum TaskStatus {
    /// uninitialized
    UnInit,
    /// ready to run
    Ready,
    /// running
    Running,
    /// exited
    Exited,
}
