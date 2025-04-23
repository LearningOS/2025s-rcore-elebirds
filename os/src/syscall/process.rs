//! Process management syscalls
//!
use core::{mem::size_of, slice::from_raw_parts};

use alloc::sync::Arc;

use crate::{
    fs::{open_file, OpenFlags},
    mm::{copy_buffer, translated_byte_buffer, translated_refmut, translated_str, VirtAddr},
    task::{
        add_task, current_task, current_user_token, exit_current_and_run_next,
        suspend_current_and_run_next,
    }, timer::get_time_us,
};

#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

pub fn sys_exit(exit_code: i32) -> ! {
    trace!("kernel:pid[{}] sys_exit", current_task().unwrap().pid.0);
    exit_current_and_run_next(exit_code);
    panic!("Unreachable in sys_exit!");
}

pub fn sys_yield() -> isize {
    //trace!("kernel: sys_yield");
    suspend_current_and_run_next();
    0
}

pub fn sys_getpid() -> isize {
    trace!("kernel: sys_getpid pid:{}", current_task().unwrap().pid.0);
    current_task().unwrap().pid.0 as isize
}

pub fn sys_fork() -> isize {
    trace!("kernel:pid[{}] sys_fork", current_task().unwrap().pid.0);
    let current_task = current_task().unwrap();
    let new_task = current_task.fork();
    let new_pid = new_task.pid.0;
    // modify trap context of new_task, because it returns immediately after switching
    let trap_cx = new_task.inner_exclusive_access().get_trap_cx();
    // we do not have to move to next instruction since we have done it before
    // for child process, fork returns 0
    trap_cx.x[10] = 0;
    // add new task to scheduler
    add_task(new_task);
    new_pid as isize
}

pub fn sys_exec(path: *const u8) -> isize {
    trace!("kernel:pid[{}] sys_exec", current_task().unwrap().pid.0);
    let token = current_user_token();
    let path = translated_str(token, path);
    if let Some(app_inode) = open_file(path.as_str(), OpenFlags::RDONLY) {
        let all_data = app_inode.read_all();
        let task = current_task().unwrap();
        task.exec(all_data.as_slice());
        0
    } else {
        -1
    }
}

/// If there is not a child process whose pid is same as given, return -1.
/// Else if there is a child process but it is still running, return -2.
pub fn sys_waitpid(pid: isize, exit_code_ptr: *mut i32) -> isize {
    //trace!("kernel: sys_waitpid");
    let task = current_task().unwrap();
    // find a child process

    // ---- access current PCB exclusively
    let mut inner = task.inner_exclusive_access();
    if !inner
        .children
        .iter()
        .any(|p| pid == -1 || pid as usize == p.getpid())
    {
        return -1;
        // ---- release current PCB
    }
    let pair = inner.children.iter().enumerate().find(|(_, p)| {
        // ++++ temporarily access child PCB exclusively
        p.inner_exclusive_access().is_zombie() && (pid == -1 || pid as usize == p.getpid())
        // ++++ release child PCB
    });
    if let Some((idx, _)) = pair {
        let child = inner.children.remove(idx);
        // confirm that child will be deallocated after being removed from children list
        assert_eq!(Arc::strong_count(&child), 1);
        let found_pid = child.getpid();
        // ++++ temporarily access child PCB exclusively
        let exit_code = child.inner_exclusive_access().exit_code;
        // ++++ release child PCB
        *translated_refmut(inner.memory_set.token(), exit_code_ptr) = exit_code;
        found_pid as isize
    } else {
        -2
    }
    // ---- release current PCB automatically
}

/// get time with second and microsecond
pub fn sys_get_time(ts: *mut TimeVal, _tz: usize) -> isize {
    trace!(
        "kernel:pid[{}] sys_get_time",
        current_task().unwrap().pid.0
    );
    let token = current_user_token();
    let size = size_of::<TimeVal>();
    let tar_buffers = translated_byte_buffer(token, ts as usize as *const u8, size);
 
    let us = get_time_us();
    let (sec, usec) = (us / 1_000_000, us % 1_000_000);
    let val = TimeVal {
        sec,
        usec,
    };
    let ori_buffers = unsafe { from_raw_parts(&val as *const _ as *const u8, size) };
    copy_buffer(tar_buffers, ori_buffers, size);
    0
}

/// request physical memory of len bytes, map it to the vitural address of start, and the premission sets like port
pub fn sys_mmap(start: usize, len: usize, port: usize) -> isize {
    trace!(
        "kernel:pid[{}] sys_mmap NOT IMPLEMENTED",
        current_task().unwrap().pid.0
    );
    if port & !0b111 != 0 || port & 0b111 == 0 { // 权限全为0无意义, 且其余位必须为0
        return -1;
    }
    if !VirtAddr::from(start).aligned() { // start 未对齐
        return -1;
    }
    if current_task().unwrap().inner_exclusive_access().request_mem_area(start, len, port) {
        0
    }else {
        -1
    }
}

///  unmap the memory area of start and len bytes
pub fn sys_munmap(start: usize, len: usize) -> isize {
    trace!(
        "kernel:pid[{}] sys_munmap NOT IMPLEMENTED",
        current_task().unwrap().pid.0
    );
    if !VirtAddr::from(start).aligned() { // start 未对齐
        return -1;
    }
    if current_task().unwrap().inner_exclusive_access().delete_mem_area(start, len) {
        0
    } else {
        -1
    }
}

/// change data segment size
pub fn sys_sbrk(size: i32) -> isize {
    trace!("kernel:pid[{}] sys_sbrk", current_task().unwrap().pid.0);
    if let Some(old_brk) = current_task().unwrap().change_program_brk(size) {
        old_brk as isize
    } else {
        -1
    }
}

/// spawn a new process that will run the program at path
/// HINT: fork + exec =/= spawn
pub fn sys_spawn(path: *const u8) -> isize {
    trace!(
        "kernel:pid[{}] sys_spawn to {:?}",
        current_task().unwrap().pid.0,
        path
    );
    let token = current_user_token();
    let path = translated_str(token, path);
    if let Some(app_inode) = open_file(path.as_str(), OpenFlags::RDONLY) {
        let data_vec = app_inode.read_all();
        let current_task = current_task().unwrap();
        let new_task = current_task.spawn(data_vec.as_slice());
        let new_pid = new_task.pid.0;
        // 更改子进程的trap context，因为它在切换后会立即返回
        let trap_cx = new_task.inner_exclusive_access().get_trap_cx();
        // 不需要移动到下一条指令，因为在切换之前已经完成了
        // 对于子进程，spawn返回0，即需要更改trap_cx.x[10]
        trap_cx.x[10] = 0;
        // 将新任务添加到调度器
        add_task(new_task);
        // 对于父进程，spawn返回子进程的pid
        new_pid as isize
    } else {
        -1
    }
}

/// Set task priority
/// 设置当前进程优先级为 prio
/// 
/// 参数：prio 进程优先级，要求 prio >= 2
/// 
/// 返回值：如果输入合法则返回 prio，否则返回 -1
pub fn sys_set_priority(prio: isize) -> isize {
    trace!(
        "kernel:pid[{}] sys_set_priority to {}",
        current_task().unwrap().pid.0,
        prio
    );
    if prio >= 2 {
        current_task().unwrap().inner_exclusive_access().set_priority(prio as usize);
        prio
    } else {
        -1
    }
}