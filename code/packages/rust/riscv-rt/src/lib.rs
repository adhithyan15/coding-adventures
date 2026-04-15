#![no_std]

use core::hint::spin_loop;
use core::ptr::{addr_of_mut, write_volatile};

pub const DEFAULT_BOOT_STACK_SIZE: usize = 16 * 1024;

/// # Safety
///
/// Must be called only after the linker symbols are valid for the current image.
pub unsafe fn zero_bss() {
    unsafe extern "C" {
        static mut __bss_start: u8;
        static mut __bss_end: u8;
    }

    let mut cursor = addr_of_mut!(__bss_start);
    let end = addr_of_mut!(__bss_end);

    while (cursor as usize) < (end as usize) {
        unsafe { write_volatile(cursor, 0) };
        cursor = unsafe { cursor.add(1) };
    }
}

#[cfg(any(target_arch = "riscv32", target_arch = "riscv64"))]
pub fn fence_i() {
    unsafe { core::arch::asm!("fence.i", options(nostack)) };
}

#[cfg(not(any(target_arch = "riscv32", target_arch = "riscv64")))]
pub fn fence_i() {}

pub fn spin_forever() -> ! {
    loop {
        spin_loop();
    }
}
