//! Semaphore

use crate::sync::UPSafeCell;
use crate::task::{block_current_and_run_next, current_task, wakeup_task, TaskControlBlock};
use alloc::vec::Vec;
use alloc::{collections::VecDeque, sync::Arc};

use super::Detectable;

/// semaphore structure
pub struct Semaphore {
    /// semaphore inner
    pub inner: UPSafeCell<SemaphoreInner>,
}

pub struct SemaphoreInner {
    pub count: isize,
    pub wait_queue: VecDeque<Arc<TaskControlBlock>>,
    pub allocated_list: Vec<usize>,
}

impl Semaphore {
    /// Create a new semaphore
    pub fn new(res_count: usize) -> Self {
        trace!("kernel: Semaphore::new");
        Self {
            inner: unsafe {
                UPSafeCell::new(SemaphoreInner {
                    count: res_count as isize,
                    wait_queue: VecDeque::new(),
                    allocated_list: Vec::new(),
                })
            },
        }
    }

    /// up operation of semaphore
    pub fn up(&self) {
        trace!("kernel: Semaphore::up");
        let mut inner = self.inner.exclusive_access();

        // 执行up操作的任务可能在allocated_list中，需要删除
        if let Some(pos) = inner.allocated_list.iter().position(|x| *x == current_task().unwrap().get_id()) {
            inner.allocated_list.remove(pos);
        }

        inner.count += 1;
        if inner.count <= 0 {
            if let Some(task) = inner.wait_queue.pop_front() {
                inner.allocated_list.push(task.get_id());
                wakeup_task(task);
            }
        }
    }

    /// down operation of semaphore
    pub fn down(&self) {
        trace!("kernel: Semaphore::down");
        let mut inner = self.inner.exclusive_access();
        inner.count -= 1;
        if inner.count < 0 {
            inner.wait_queue.push_back(current_task().unwrap());
            drop(inner);
            block_current_and_run_next();
        } else {
            inner.allocated_list.push(current_task().unwrap().get_id());
        }
    }
}

impl Detectable for Semaphore {
    /// 锁资源可用数量
    fn get_avaliable(&self) -> usize {
        let inner = self.inner.exclusive_access();
        if inner.count >= 0 {
            inner.count as usize
        } else {
            0
        }
    }

    /// Get the owner of the allocated resource
    fn get_allocated(&self) -> Option<Vec<usize>> {
        let inner = self.inner.exclusive_access();
        if inner.allocated_list.is_empty() {
            None
        } else {
            Some(inner.allocated_list.clone())
        }
    }

    /// Get the requester of the resource
    fn get_needed(&self) -> Option<Vec<usize>> {
        let inner = self.inner.exclusive_access();
        if inner.wait_queue.is_empty() {
            None
        } else {
            Some(inner.wait_queue.iter().map(|task| task.get_id()).collect())
        }
    }
}