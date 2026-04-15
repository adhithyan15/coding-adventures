#![no_std]
#![no_main]

use boot_protocol::{BootInfo, DEFAULT_QEMU_VIRT_UART_BASE};
use core::arch::global_asm;
use core::fmt::Write;
use core::panic::PanicInfo;
use riscv_rt::{spin_forever, zero_bss};
use uart_16550::Uart16550;

const KERNEL_START: u64 = 0x8020_0000;

global_asm!(include_str!("entry.S"));

#[no_mangle]
pub extern "C" fn kernel_main(hart_id: usize, dtb_ptr: usize, boot_info_ptr: usize) -> ! {
    unsafe { zero_bss() };

    let mut uart = unsafe { Uart16550::new(DEFAULT_QEMU_VIRT_UART_BASE as usize) };
    uart.init();

    let boot_info = unsafe { BootInfo::from_ptr(boot_info_ptr) }
        .copied()
        .unwrap_or_else(|| BootInfo::qemu_virt(hart_id as u64, KERNEL_START, KERNEL_START, dtb_ptr as u64));

    let _ = writeln!(uart, "kernel: start");
    let _ = writeln!(uart, "kernel: boot info ok");
    let _ = writeln!(uart, "kernel: hart={} dtb=0x{:x}", hart_id, dtb_ptr);
    let _ = writeln!(uart, "kernel: uart=0x{:x}", boot_info.uart_base);
    let _ = writeln!(uart, "Hello World");

    spin_forever()
}

#[panic_handler]
fn panic(info: &PanicInfo<'_>) -> ! {
    let mut uart = unsafe { Uart16550::new(DEFAULT_QEMU_VIRT_UART_BASE as usize) };
    uart.init();
    let _ = writeln!(uart, "panic: {}", info);
    spin_forever()
}
