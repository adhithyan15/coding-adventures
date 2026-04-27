#![no_main]
#![no_std]

use os_kernel::Kernel;
use uefi::prelude::*;

#[entry]
fn efi_main() -> Status {
    uefi::helpers::init().unwrap();

    let mut kernel = Kernel::new();
    kernel.boot();
    uefi::println!("kernel: booted");

    kernel.enter_running_state();
    uefi::println!("kernel: running");

    loop {
        core::hint::spin_loop();
    }
}
