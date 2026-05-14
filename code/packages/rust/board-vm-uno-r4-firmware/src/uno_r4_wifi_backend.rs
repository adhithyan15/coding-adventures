use arduino_uno_r4_hal::Delay;
use board_vm_runtime::{GpioMode, HalError, Level};
use board_vm_uno_r4::UnoR4Backend;
use embedded_hal::delay::DelayNs;

use crate::uno_r4_wifi_led::{UnoR4WifiLed, UNO_R4_WIFI_LED_PIN};
use crate::uno_r4_wifi_led_matrix::UnoR4WifiLedMatrix;

pub struct UnoR4WifiLedBackend {
    led: Option<UnoR4WifiLed>,
    matrix: UnoR4WifiLedMatrix,
    delay: Delay,
    now_ms: u32,
}

impl UnoR4WifiLedBackend {
    pub fn new() -> Self {
        Self {
            led: None,
            matrix: UnoR4WifiLedMatrix::new(),
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

    pub fn refresh_led_matrix_once(&mut self) {
        self.matrix.refresh_once();
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

    fn led_matrix_frame(&mut self, frame: [u32; 3]) -> Result<(), HalError> {
        self.matrix.set_frame(frame);
        Ok(())
    }
}

#[cfg(all(target_arch = "arm", board_vm_uno_r4_arduino_usb_link))]
pub struct UnoR4WifiPwmBackend {
    inner: UnoR4WifiLedBackend,
}

#[cfg(all(target_arch = "arm", board_vm_uno_r4_arduino_usb_link))]
impl UnoR4WifiPwmBackend {
    pub fn new() -> Self {
        Self {
            inner: UnoR4WifiLedBackend::new(),
        }
    }

    pub fn set_led(&mut self, level: Level) {
        self.inner.set_led(level);
    }

    pub fn pause_ms(&mut self, duration_ms: u32) {
        self.inner.pause_ms(duration_ms);
    }

    pub fn blink_pattern(&mut self, pulses: u8, on_ms: u32, off_ms: u32) {
        self.inner.blink_pattern(pulses, on_ms, off_ms);
    }

    pub fn refresh_led_matrix_once(&mut self) {
        self.inner.refresh_led_matrix_once();
    }
}

#[cfg(all(target_arch = "arm", board_vm_uno_r4_arduino_usb_link))]
impl Default for UnoR4WifiPwmBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(all(target_arch = "arm", board_vm_uno_r4_arduino_usb_link))]
impl UnoR4Backend for UnoR4WifiPwmBackend {
    fn configure_gpio(&mut self, pin: u8, mode: GpioMode) -> Result<(), HalError> {
        self.inner.configure_gpio(pin, mode)
    }

    fn write_gpio(&mut self, pin: u8, level: Level) -> Result<(), HalError> {
        self.inner.write_gpio(pin, level)
    }

    fn read_gpio(&mut self, pin: u8) -> Result<Level, HalError> {
        self.inner.read_gpio(pin)
    }

    fn sleep_ms(&mut self, duration_ms: u16) -> Result<(), HalError> {
        self.inner.sleep_ms(duration_ms)
    }

    fn now_ms(&self) -> u32 {
        self.inner.now_ms()
    }

    fn supports_pwm(&self) -> bool {
        true
    }

    fn supports_adc(&self) -> bool {
        true
    }

    fn supports_dac(&self) -> bool {
        true
    }

    fn supports_i2c(&self) -> bool {
        true
    }

    fn supports_bootloader_reboot(&self) -> bool {
        true
    }

    fn led_matrix_frame(&mut self, frame: [u32; 3]) -> Result<(), HalError> {
        self.inner.led_matrix_frame(frame)
    }

    fn write_pwm(&mut self, pin: u8, duty: u16) -> Result<(), HalError> {
        if unsafe { board_io_ffi::board_vm_uno_r4_pwm_write(pin, duty) } {
            Ok(())
        } else {
            Err(HalError::UnsupportedMode)
        }
    }

    fn read_adc(&mut self, pin: u8) -> Result<u16, HalError> {
        let mut sample = 0;
        if unsafe { board_io_ffi::board_vm_uno_r4_adc_read(pin, &mut sample) } {
            Ok(sample)
        } else {
            Err(HalError::UnsupportedMode)
        }
    }

    fn write_dac_u12(&mut self, pin: u8, sample: u16) -> Result<(), HalError> {
        if unsafe { board_io_ffi::board_vm_uno_r4_dac_write_u12(pin, sample) } {
            Ok(())
        } else {
            Err(HalError::UnsupportedMode)
        }
    }

    fn open_i2c(&mut self, bus: u8) -> Result<u32, HalError> {
        Ok(0x12_0000 | bus as u32)
    }

    fn write_i2c_u8(&mut self, bus: u8, address: u16, byte: u8) -> Result<(), HalError> {
        if unsafe { board_io_ffi::board_vm_uno_r4_i2c_write_u8(bus, address, byte) } {
            Ok(())
        } else {
            Err(HalError::UnsupportedMode)
        }
    }

    fn write_i2c(&mut self, bus: u8, address: u16, bytes: &[u8]) -> Result<(), HalError> {
        if unsafe {
            board_io_ffi::board_vm_uno_r4_i2c_write(bus, address, bytes.as_ptr(), bytes.len())
        } {
            Ok(())
        } else {
            Err(HalError::UnsupportedMode)
        }
    }

    fn read_i2c_u8(&mut self, bus: u8, address: u16) -> Result<u8, HalError> {
        let mut byte = 0;
        if unsafe { board_io_ffi::board_vm_uno_r4_i2c_read_u8(bus, address, &mut byte) } {
            Ok(byte)
        } else {
            Err(HalError::UnsupportedMode)
        }
    }

    fn reboot_to_bootloader(&mut self) -> Result<(), HalError> {
        board_vm_uno_r4_usb_cdc::reboot_to_bootloader()
    }
}

#[cfg(all(target_arch = "arm", board_vm_uno_r4_arduino_usb_link))]
mod board_io_ffi {
    unsafe extern "C" {
        pub fn board_vm_uno_r4_pwm_write(pin: u8, duty: u16) -> bool;
        pub fn board_vm_uno_r4_adc_read(pin: u8, sample: *mut u16) -> bool;
        pub fn board_vm_uno_r4_dac_write_u12(pin: u8, sample: u16) -> bool;
        pub fn board_vm_uno_r4_i2c_write_u8(bus: u8, address: u16, byte: u8) -> bool;
        pub fn board_vm_uno_r4_i2c_write(
            bus: u8,
            address: u16,
            bytes: *const u8,
            len: usize,
        ) -> bool;
        pub fn board_vm_uno_r4_i2c_read_u8(bus: u8, address: u16, byte: *mut u8) -> bool;
    }
}
