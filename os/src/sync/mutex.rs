//! Mutex (spin-like and blocking(sleep))

use super::{Detectable, UPSafeCell};
use crate::task::TaskControlBlock;
use crate::task::{block_current_and_run_next, suspend_current_and_run_next};
use crate::task::{current_task, wakeup_task};
use alloc::vec::Vec;
use alloc::vec;
use alloc::{collections::VecDeque, sync::Arc};

/// Mutex trait
pub trait Mutex: Sync + Send + Detectable {
    /// Lock the mutex
    fn lock(&self);
    /// Unlock the mutex
    fn unlock(&self);
}

/// Spinlock Mutex struct
pub struct MutexSpin {
    inner: UPSafeCell<MutexSpinInner>,
}

pub struct MutexSpinInner {
    locked: bool,
    allocated: Option<usize>,
}

impl MutexSpin { // 互斥锁
    /// Create a new spinlock mutex
    pub fn new() -> Self {
        Self {
            inner: unsafe {
                UPSafeCell::new(MutexSpinInner {
                    locked: false,
                    allocated: None,
                })
            },
        }
    }
}

impl Mutex for MutexSpin {
    /// Lock the spinlock mutex
    fn lock(&self) {
        trace!("kernel: MutexSpin::lock");
        loop {
            let mut inner = self.inner.exclusive_access();
            if inner.locked {
                drop(inner);
                suspend_current_and_run_next();
                continue;
            } else {
                inner.allocated = Some(current_task().unwrap().get_id());
                inner.locked = true;
                return;
            }
        }
    }

    fn unlock(&self) {
        trace!("kernel: MutexSpin::unlock");
        self.inner.exclusive_access().locked = false;
    }
}

impl Detectable for MutexSpin {
    /// 锁资源可用数量
    fn get_avaliable(&self) -> usize {
        if self.inner.exclusive_access().locked {
            0
        } else {
            1
        }
    }

    /// 锁资源持有者
    fn get_allocated(&self) -> Option<Vec<usize>> {
        let inner = self.inner.exclusive_access();
        if inner.locked {
            Some(vec![inner.allocated.unwrap()])
        } else {
            None
        }
    }

    /// 锁资源申请者，由于在锁定状态下申请会直接yield，没有等待中的申请者
    fn get_needed(&self) -> Option<Vec<usize>> { 
        None
    }
}

/// Blocking Mutex struct
pub struct MutexBlocking {
    inner: UPSafeCell<MutexBlockingInner>,
}

pub struct MutexBlockingInner {
    locked: bool,
    wait_queue: VecDeque<Arc<TaskControlBlock>>,
    allocated: Option<usize>,
}

impl MutexBlocking { // 阻塞锁
    /// Create a new blocking mutex
    pub fn new() -> Self {
        trace!("kernel: MutexBlocking::new");
        Self {
            inner: unsafe {
                UPSafeCell::new(MutexBlockingInner {
                    locked: false,
                    wait_queue: VecDeque::new(),
                    allocated: None,
                })
            },
        }
    }
}

impl Mutex for MutexBlocking {
    /// lock the blocking mutex
    fn lock(&self) {
        trace!("kernel: MutexBlocking::lock");
        let mut mutex_inner = self.inner.exclusive_access();
        if mutex_inner.locked {
            mutex_inner.wait_queue.push_back(current_task().unwrap());
            drop(mutex_inner);
            block_current_and_run_next();
        } else {
            mutex_inner.allocated = Some(current_task().unwrap().get_id());
            mutex_inner.locked = true;
        }
    }

    /// unlock the blocking mutex
    fn unlock(&self) {
        trace!("kernel: MutexBlocking::unlock");
        let mut mutex_inner = self.inner.exclusive_access();
        assert!(mutex_inner.locked);
        if let Some(waking_task) = mutex_inner.wait_queue.pop_front() {
            wakeup_task(waking_task);
        } else {
            mutex_inner.locked = false;
        }
    }
}

impl Detectable for MutexBlocking {
    /// 锁资源可用数量
    fn get_avaliable(&self) -> usize {
        if self.inner.exclusive_access().locked {
            0
        } else {
            1
        }
    }

    /// 锁资源持有者
    fn get_allocated(&self) -> Option<Vec<usize>> {
        let inner = self.inner.exclusive_access();
        if inner.locked {
            Some(vec![inner.allocated.unwrap()])
        } else {
            None
        }
    }

    /// 锁资源申请者
    fn get_needed(&self) -> Option<Vec<usize>> { 
        let inner = self.inner.exclusive_access();
        if inner.locked {
            Some(inner.wait_queue.iter().map(|task| task.get_id()).collect())
        } else {
            None
        }
    }
}