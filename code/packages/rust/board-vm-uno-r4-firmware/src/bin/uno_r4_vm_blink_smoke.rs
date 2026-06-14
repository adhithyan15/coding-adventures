#![cfg_attr(target_arch = "arm", no_std)]
#![cfg_attr(target_arch = "arm", no_main)]

#[cfg(target_arch = "arm")]
mod firmware {
    use board_vm_runtime::Runtime;
    use board_vm_uno_r4::UnoR4Board;
    use board_vm_uno_r4_firmware::{
        run_blink_smoke_once, uno_r4_wifi_backend::UnoR4WifiLedBackend, SMOKE_INSTRUCTION_BUDGET,
    };
    use panic_halt as _;

    #[cortex_m_rt::entry]
    fn main() -> ! {
        let backend = UnoR4WifiLedBackend::new();
        let board = UnoR4Board::wifi(backend);
        let mut runtime: Runtime<_, 16, 8> = Runtime::new(board);

        loop {
            let _ = run_blink_smoke_once(&mut runtime, SMOKE_INSTRUCTION_BUDGET);
        }
    }
}

#[cfg(not(target_arch = "arm"))]
fn main() {}
