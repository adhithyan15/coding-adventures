use arduino_uno_r4_hal::Delay;
use board_vm_runtime::{GpioMode, HalError, Level};
use board_vm_uno_r4::UnoR4Backend;
use embedded_hal::delay::DelayNs;

use crate::uno_r4_wifi_led::{UnoR4WifiLed, UNO_R4_WIFI_LED_PIN};

pub struct UnoR4WifiLedBackend {
    led: Option<UnoR4WifiLed>,
    delay: Delay,
    now_ms: u32,
}

impl UnoR4WifiLedBackend {
    pub fn new() -> Self {
        Self {
            led: None,
            delay: Delay::new(),
            now_ms: 0,
        }
    }

    pub fn set_led(&mut self, level: Level) {
        let led = self.led.get_or_insert_with(UnoR4WifiLed::configure_output);
        match level {
            Level::Low => led.set_low(),
            Level::High => led.set_high(),
        }
    }

    pub fn pause_ms(&mut self, duration_ms: u32) {
        self.delay.delay_ms(duration_ms);
        self.now_ms = self.now_ms.wrapping_add(duration_ms);
    }

    pub fn blink_pattern(&mut self, pulses: u8, on_ms: u32, off_ms: u32) {
        for _ in 0..pulses {
            self.set_led(Level::High);
            self.pause_ms(on_ms);
            self.set_led(Level::Low);
            self.pause_ms(off_ms);
        }
    }
}

impl Default for UnoR4WifiLedBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl UnoR4Backend for UnoR4WifiLedBackend {
    fn configure_gpio(&mut self, pin: u8, mode: GpioMode) -> Result<(), HalError> {
        if pin != UNO_R4_WIFI_LED_PIN || mode != GpioMode::Output {
            return Err(HalError::UnsupportedMode);
        }
        self.led = Some(UnoR4WifiLed::configure_output());
        Ok(())
    }

    fn write_gpio(&mut self, pin: u8, level: Level) -> Result<(), HalError> {
        if pin != UNO_R4_WIFI_LED_PIN {
            return Err(HalError::InvalidPin);
        }
        self.set_led(level);
        Ok(())
    }

    fn read_gpio(&mut self, pin: u8) -> Result<Level, HalError> {
        if pin == UNO_R4_WIFI_LED_PIN {
            Ok(Level::Low)
        } else {
            Err(HalError::InvalidPin)
        }
    }

    fn sleep_ms(&mut self, duration_ms: u16) -> Result<(), HalError> {
        self.pause_ms(duration_ms as u32);
        Ok(())
    }

    fn now_ms(&self) -> u32 {
        self.now_ms
    }
}
