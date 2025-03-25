//! SBI call wrappers

use core::arch::asm;

const SBI_CONSOLE_PUTCHAR: usize = 1;

/// general sbi call
#[inline(always)]
fn sbi_call(which: usize, arg0: usize, arg1: usize, arg2: usize) -> usize {
    let mut ret;
    unsafe {
        // 先将对应的参数放入寄存器，再调用li x16, 0; ecall，最终将x10的值保存到ret中
        asm!(
            "li x16, 0",
            "ecall",
            inlateout("x10") arg0 => ret,
            in("x11") arg1,
            in("x12") arg2,
            in("x17") which,
        );
        /* 实际上类似于于：
        asm!(
            "ecall",
            inout("x10") arg0 => ret,
            in("x11") arg1,
            in("x12") arg2,
            in("x16") 0,
            in("x17") which,
        );
        li x16, 0的作用其实也就是将x16寄存器清零，本身只是ecall指令预先传参罢了，和其他的inout/in/没有什么区别
         */
    }
    ret
}

/// use sbi call to putchar in console (qemu uart handler)
pub fn console_putchar(c: usize) {
    sbi_call(SBI_CONSOLE_PUTCHAR, c, 0, 0);
}

use crate::board::QEMUExit;
/// use sbi call to shutdown the kernel
pub fn shutdown() -> ! {
    crate::board::QEMU_EXIT_HANDLE.exit_failure();
}
