#![cfg_attr(target_arch = "arm", no_std)]
#![cfg_attr(target_arch = "arm", no_main)]

#[cfg(target_arch = "arm")]
mod firmware {
    use board_vm_device::DeviceStreamEndpoint;
    use board_vm_runtime::Level;
    use board_vm_uart::UartByteStream;
    use board_vm_uno_r4::UnoR4Board;
    use board_vm_uno_r4_firmware::uno_r4_wifi_backend::UnoR4WifiLedBackend;
    use board_vm_uno_r4_uart::UnoR4WifiSerialUart;
    use panic_halt as _;

    const BOARD_NONCE: u32 = 0xB04D_1004;

    #[cortex_m_rt::entry]
    fn main() -> ! {
        let mut backend = UnoR4WifiLedBackend::new();
        let uart = match UnoR4WifiSerialUart::new() {
            Ok(uart) => uart,
            Err(_) => fault_loop(backend),
        };

        backend.blink_pattern(2, 45, 65);
        let board = UnoR4Board::wifi(backend);
        let mut device = board.into_device(BOARD_NONCE);
        // This serves the Uno R4 WiFi Arduino Serial1 route on D22/D23. The
        // board's built-in USB port is Arduino SerialUSB and needs a separate
        // USB CDC transport backend.
        let stream = UartByteStream::new(uart);
        let mut endpoint = DeviceStreamEndpoint::<_, 1024, 512, 256>::new(stream);

        loop {
            if endpoint.serve_one(&mut device).is_err() {
                let backend = device.hal_mut().backend_mut();
                backend.blink_pattern(1, 160, 120);
            }
        }
    }

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
