#![cfg_attr(target_arch = "arm", no_std)]
#![cfg_attr(target_arch = "arm", no_main)]

#[cfg(target_arch = "arm")]
mod firmware {
    use board_vm_device::{DeviceStreamPoll, DEFAULT_BACKGROUND_INSTRUCTION_SLICE};
    use board_vm_runtime::Level;
    use board_vm_uno_r4::UnoR4Board;
    use board_vm_uno_r4_firmware::serial_usb_server::{
        serial_usb_endpoint, serve_serial_usb_available,
    };
    use board_vm_uno_r4_firmware::uno_r4_wifi_backend::UnoR4WifiLedBackend;
    use board_vm_uno_r4_usb_cdc::UnoR4WifiSerialUsb;
    use panic_halt as _;

    const BOARD_NONCE: u32 = 0xB04D_1005;

    #[cortex_m_rt::entry]
    fn main() -> ! {
        let mut backend = UnoR4WifiLedBackend::new();
        let mut usb = UnoR4WifiSerialUsb::serial_usb();
        usb.begin();

        backend.blink_pattern(3, 45, 65);
        let board = UnoR4Board::wifi(backend);
        let mut device = board.into_device(BOARD_NONCE);
        let mut endpoint = serial_usb_endpoint(usb);

        loop {
            match serve_serial_usb_available(&mut endpoint, &mut device) {
                Ok(DeviceStreamPoll::Served(_)) => {}
                Ok(DeviceStreamPoll::Idle) => {
                    if device
                        .poll_background(DEFAULT_BACKGROUND_INSTRUCTION_SLICE)
                        .is_err()
                    {
                        let backend = device.hal_mut().backend_mut();
                        backend.blink_pattern(1, 160, 120);
                    }
                }
                Err(_) => {
                    let backend = device.hal_mut().backend_mut();
                    backend.blink_pattern(1, 160, 120);
                }
            }
        }
    }

    #[allow(dead_code)]
    fn fault_loop(mut backend: UnoR4WifiLedBackend) -> ! {
        loop {
            backend.set_led(Level::High);
            backend.pause_ms(900);
            backend.set_led(Level::Low);
            backend.pause_ms(900);
        }
    }
}

#[cfg(not(target_arch = "arm"))]
fn main() {}
