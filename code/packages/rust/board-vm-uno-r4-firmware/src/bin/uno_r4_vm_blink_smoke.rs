#![cfg_attr(target_arch = "arm", no_std)]
#![cfg_attr(target_arch = "arm", no_main)]

#[cfg(target_arch = "arm")]
mod firmware {
    use arduino_uno_r4_hal::{
        gpio::{Output, Pin},
        Delay,
    };
    use board_vm_runtime::{GpioMode, HalError, Level, Runtime};
    use board_vm_uno_r4::{UnoR4Backend, UnoR4Board};
    use board_vm_uno_r4_firmware::{run_blink_smoke_once, SMOKE_INSTRUCTION_BUDGET};
    use embedded_hal::delay::DelayNs;
    use panic_halt as _;

    pub struct UnoR4LedBackend {
        led: Option<Pin<'1', 11, Output>>,
        delay: Delay,
        now_ms: u32,
    }

    impl UnoR4LedBackend {
        pub fn new() -> Self {
            Self {
                led: None,
                delay: Delay::new(),
                now_ms: 0,
            }
        }
    }

    impl UnoR4Backend for UnoR4LedBackend {
        fn configure_gpio(&mut self, pin: u8, mode: GpioMode) -> Result<(), HalError> {
            if pin != 13 || mode != GpioMode::Output {
                return Err(HalError::UnsupportedMode);
            }
            self.led = Some(Pin::<'1', 11, Output>::new());
            Ok(())
        }

        fn write_gpio(&mut self, pin: u8, level: Level) -> Result<(), HalError> {
            if pin != 13 {
                return Err(HalError::InvalidPin);
            }
            let led = self.led.as_mut().ok_or(HalError::ResourceBusy)?;
            match level {
                Level::Low => led.set_low(),
                Level::High => led.set_high(),
            }
            Ok(())
        }

        fn read_gpio(&mut self, pin: u8) -> Result<Level, HalError> {
            if pin == 13 {
                Ok(Level::Low)
            } else {
                Err(HalError::InvalidPin)
            }
        }

        fn sleep_ms(&mut self, duration_ms: u16) -> Result<(), HalError> {
            self.delay.delay_ms(duration_ms as u32);
            self.now_ms = self.now_ms.wrapping_add(duration_ms as u32);
            Ok(())
        }

        fn now_ms(&self) -> u32 {
            self.now_ms
        }
    }

    #[cortex_m_rt::entry]
    fn main() -> ! {
        let backend = UnoR4LedBackend::new();
        let board = UnoR4Board::minima(backend);
        let mut runtime: Runtime<_, 16, 8> = Runtime::new(board);

        loop {
            let _ = run_blink_smoke_once(&mut runtime, SMOKE_INSTRUCTION_BUDGET);
        }
    }
}

#[cfg(not(target_arch = "arm"))]
fn main() {}
