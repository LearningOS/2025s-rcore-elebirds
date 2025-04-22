//! Process management syscalls
use core::{mem::size_of, slice::from_raw_parts};

use crate::{mm::{copy_buffer, translated_byte_buffer, PageTable, VirtAddr}, task::{change_program_brk, current_user_token, exit_current_and_run_next, suspend_current_and_run_next, TASK_MANAGER}, timer::get_time_us};

#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

/// task exits and submit an exit code
pub fn sys_exit(_exit_code: i32) -> ! {
    trace!("kernel: sys_exit");
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

/// get some information about current task, decided by the trace_request
/// 0: get the data in the address of id, seen as *const u8
/// 1: write data to the address of id, seen as *mut u8
/// 2: get the frequency which is the id-th syscall, including this time's call
/// otherwise, return -1
pub fn sys_trace(trace_request: usize, id: usize, data: usize) -> isize {
    trace!("kernel: sys_trace");
    if trace_request == 2 { // get syscall count
        return TASK_MANAGER.get_syscall_count(id) as isize;
    }
    let pt = PageTable::from_token(current_user_token());
    let va = VirtAddr::from(id);
    let pte = match pt.translate(va.floor()) {
        Some(pte) => pte,
        None => return -1,
    };
    if !(pte.is_valid() && pte.user()) {
        return -1;
    }
    let addr = pte.ppn().get_phys_addr(va.page_offset()).0;
    match trace_request {
        0 => {
            if !pte.readable() {
                return -1
            }
            unsafe { *(addr as *const u8) as isize }
        },
        1 => {
            if !pte.writable() {
                return -1
            }
            unsafe { *(addr as *mut u8) = data as u8; }
            0
        },
        _ => -1,
    }
}

/// request physical memory of len bytes, map it to the vitural address of start, and the premission sets like port
pub fn sys_mmap(start: usize, len: usize, port: usize) -> isize {
    trace!("kernel: sys_mmap");
    if port & !0b111 != 0 || port & 0b111 == 0 { // 权限全为0无意义, 且其余位必须为0
        return -1;
    }
    if !VirtAddr::from(start).aligned() { // start 未对齐
        return -1;
    }
    if TASK_MANAGER.get_current_task().request_mem_area(start, len, port) {
        0
    }else {
        -1
    }
}

///  unmap the memory area of start and len bytes
pub fn sys_munmap(start: usize, len: usize) -> isize {
    trace!("kernel: sys_munmap");
    if !VirtAddr::from(start).aligned() { // start 未对齐
        return -1;
    }
    if TASK_MANAGER.get_current_task().delete_mem_area(start, len) {
        0
    } else {
        -1
    }
}
/// change data segment size
pub fn sys_sbrk(size: i32) -> isize {
    trace!("kernel: sys_sbrk");
    if let Some(old_brk) = change_program_brk(size) {
        old_brk as isize
    } else {
        -1
    }
}
