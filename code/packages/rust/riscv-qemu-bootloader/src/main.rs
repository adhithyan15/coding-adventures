#![no_std]
#![no_main]

use boot_protocol::{BootInfo, DEFAULT_QEMU_VIRT_UART_BASE};
use core::arch::global_asm;
use core::fmt::Write;
use core::panic::PanicInfo;
use riscv_rt::{spin_forever, zero_bss};
use uart_16550::Uart16550;

const KERNEL_LOAD_ADDR: u64 = 0x8020_0000;

global_asm!(include_str!("entry.S"));

#[no_mangle]
pub extern "C" fn bootloader_main(hart_id: usize, dtb_ptr: usize, _opaque: usize) -> ! {
    unsafe { zero_bss() };

    let mut uart = unsafe { Uart16550::new(DEFAULT_QEMU_VIRT_UART_BASE as usize) };
    uart.init();

    let boot_info = BootInfo::qemu_virt(hart_id as u64, KERNEL_LOAD_ADDR, KERNEL_LOAD_ADDR, dtb_ptr as u64);

    let _ = writeln!(uart, "bootloader: start");
    let _ = writeln!(uart, "bootloader: uart ok");
    let _ = writeln!(uart, "bootloader: dtb=0x{:x}", dtb_ptr);
    let _ = writeln!(uart, "bootloader: next kernel load addr=0x{:x}", boot_info.kernel_start);
    let _ = writeln!(uart, "bootloader: kernel copy/jump wiring is next");

    spin_forever()
}

#[panic_handler]
fn panic(info: &PanicInfo<'_>) -> ! {
    let mut uart = unsafe { Uart16550::new(DEFAULT_QEMU_VIRT_UART_BASE as usize) };
    uart.init();
    let _ = writeln!(uart, "panic: {}", info);
    spin_forever()
}
