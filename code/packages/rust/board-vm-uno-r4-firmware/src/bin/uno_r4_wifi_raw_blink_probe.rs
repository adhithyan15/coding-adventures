#![cfg_attr(target_arch = "arm", no_std)]
#![cfg_attr(target_arch = "arm", no_main)]

#[cfg(target_arch = "arm")]
mod firmware {
    use arduino_uno_r4_hal::Delay;
    use board_vm_uno_r4_firmware::uno_r4_wifi_led::UnoR4WifiLed;
    use embedded_hal::delay::DelayNs;
    use panic_halt as _;

    #[cortex_m_rt::entry]
    fn main() -> ! {
        let mut led = UnoR4WifiLed::configure_output();
        let mut delay = Delay::new();

        loop {
            led.set_high();
            delay.delay_ms(500);
            led.set_low();
            delay.delay_ms(500);
        }
    }
}

#[cfg(not(target_arch = "arm"))]
fn main() {}
