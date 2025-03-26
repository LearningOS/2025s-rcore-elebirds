//! Process management syscalls
use crate::{
    task::{exit_current_and_run_next, suspend_current_and_run_next, get_syscall_count},
    timer::get_time_us,
};

#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

/// task exits and submit an exit code
pub fn sys_exit(exit_code: i32) -> ! {
    trace!("[kernel] Application exited with code {}", exit_code);
    exit_current_and_run_next();
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    trace!("kernel: sys_yield");
    suspend_current_and_run_next();
    0
}

/// get time with second and microsecond
pub fn sys_get_time(ts: *mut TimeVal, _tz: usize) -> isize {
    trace!("kernel: sys_get_time");
    let us = get_time_us();
    unsafe {
        *ts = TimeVal {
            sec: us / 1_000_000,
            usec: us % 1_000_000,
        };
    }
    0
}

// TODO: implement the syscall
pub fn sys_trace(trace_request: usize, id: usize, data: usize) -> isize {
    match trace_request {
        0 => unsafe {
            // 读取一个地址的值
            // @param id 任务 id 地址，视为一个指向无符号整数的指针*const u8
            // @param data 无意义
            // @return 读取的值
            *(id as * const u8) as isize
        }
        1 => unsafe {
            // 写入一个地址的值
            // @param id 任务 id 地址，视为一个指向无符号8位整数的指针*mut u8
            // @param data 要写入的值
            // @return 0 表示成功
            *(id as * mut u8) = (data & 0xff) as u8;
            0
        },
        2 => {
            // 查询当前任务调用编号为 id 的系统调用的次数，包括本次调用
            // @param id 系统调用编号
            // @param data 无意义
            // @return 系统调用次数
            get_syscall_count(id) as isize
        },
        _ => -1
    }
}
