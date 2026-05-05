#![no_std]

use board_vm_ir::CapabilitySet;
use board_vm_runtime::{BoardHal, GpioMode, HalError, Level};

pub const UNO_R4_CLOCK_HZ: u32 = 48_000_000;
pub const UNO_R4_FLASH_BYTES: u32 = 256 * 1024;
pub const UNO_R4_SRAM_BYTES: u32 = 32 * 1024;
pub const UNO_R4_DATA_FLASH_BYTES: u32 = 8 * 1024;
pub const UNO_R4_ONBOARD_LED_PIN: u8 = 13;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnoR4Variant {
    Minima,
    Wifi,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TargetDescriptor {
    pub board_id: &'static str,
    pub display_name: &'static str,
    pub variant: UnoR4Variant,
    pub mcu: &'static str,
    pub core: &'static str,
    pub isa: &'static str,
    pub rust_target: &'static str,
    pub clock_hz: u32,
    pub flash_bytes: u32,
    pub sram_bytes: u32,
    pub data_flash_bytes: u32,
    pub operating_voltage_mv: u16,
    pub onboard_led_pin: u8,
    pub supports_wifi_module: bool,
    pub supports_led_matrix: bool,
    pub capabilities: CapabilitySet,
    pub digital_pins: &'static [DigitalPinDescriptor],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DigitalPinDescriptor {
    pub arduino_pin: u8,
    pub label: &'static str,
    pub supports_pwm: bool,
    pub supports_interrupt: bool,
    pub notes: &'static str,
}

pub const UNO_R4_DIGITAL_PINS: [DigitalPinDescriptor; 14] = [
    DigitalPinDescriptor {
        arduino_pin: 0,
        label: "D0/RX0",
        supports_pwm: false,
        supports_interrupt: false,
        notes: "GPIO 0 / Serial 0 receiver",
    },
    DigitalPinDescriptor {
        arduino_pin: 1,
        label: "D1/TX0",
        supports_pwm: false,
        supports_interrupt: false,
        notes: "GPIO 1 / Serial 0 transmitter",
    },
    DigitalPinDescriptor {
        arduino_pin: 2,
        label: "D2",
        supports_pwm: false,
        supports_interrupt: true,
        notes: "GPIO 2 / external interrupt",
    },
    DigitalPinDescriptor {
        arduino_pin: 3,
        label: "D3",
        supports_pwm: true,
        supports_interrupt: true,
        notes: "GPIO 3 / PWM / external interrupt",
    },
    DigitalPinDescriptor {
        arduino_pin: 4,
        label: "D4",
        supports_pwm: false,
        supports_interrupt: false,
        notes: "GPIO 4 / CAN alternate function on Minima header docs",
    },
    DigitalPinDescriptor {
        arduino_pin: 5,
        label: "D5",
        supports_pwm: true,
        supports_interrupt: false,
        notes: "GPIO 5 / PWM",
    },
    DigitalPinDescriptor {
        arduino_pin: 6,
        label: "D6",
        supports_pwm: true,
        supports_interrupt: false,
        notes: "GPIO 6 / PWM",
    },
    DigitalPinDescriptor {
        arduino_pin: 7,
        label: "D7",
        supports_pwm: false,
        supports_interrupt: false,
        notes: "GPIO 7",
    },
    DigitalPinDescriptor {
        arduino_pin: 8,
        label: "D8",
        supports_pwm: false,
        supports_interrupt: false,
        notes: "GPIO 8",
    },
    DigitalPinDescriptor {
        arduino_pin: 9,
        label: "D9",
        supports_pwm: true,
        supports_interrupt: false,
        notes: "GPIO 9 / PWM",
    },
    DigitalPinDescriptor {
        arduino_pin: 10,
        label: "D10/CS",
        supports_pwm: true,
        supports_interrupt: false,
        notes: "GPIO 10 / PWM / SPI chip select",
    },
    DigitalPinDescriptor {
        arduino_pin: 11,
        label: "D11/COPI",
        supports_pwm: true,
        supports_interrupt: false,
        notes: "GPIO 11 / PWM / SPI controller out",
    },
    DigitalPinDescriptor {
        arduino_pin: 12,
        label: "D12/CIPO",
        supports_pwm: false,
        supports_interrupt: false,
        notes: "GPIO 12 / SPI controller in",
    },
    DigitalPinDescriptor {
        arduino_pin: 13,
        label: "D13/SCK",
        supports_pwm: false,
        supports_interrupt: false,
        notes: "GPIO 13 / SPI clock / onboard LED",
    },
];

pub const UNO_R4_MINIMA: TargetDescriptor = TargetDescriptor {
    board_id: "arduino-uno-r4-minima",
    display_name: "Arduino Uno R4 Minima",
    variant: UnoR4Variant::Minima,
    mcu: "Renesas RA4M1 R7FA4M1AB3CFM",
    core: "Arm Cortex-M4F",
    isa: "Armv7E-M Thumb-2",
    rust_target: "thumbv7em-none-eabihf",
    clock_hz: UNO_R4_CLOCK_HZ,
    flash_bytes: UNO_R4_FLASH_BYTES,
    sram_bytes: UNO_R4_SRAM_BYTES,
    data_flash_bytes: UNO_R4_DATA_FLASH_BYTES,
    operating_voltage_mv: 5000,
    onboard_led_pin: UNO_R4_ONBOARD_LED_PIN,
    supports_wifi_module: false,
    supports_led_matrix: false,
    capabilities: CapabilitySet::blink_mvp(),
    digital_pins: &UNO_R4_DIGITAL_PINS,
};

pub const UNO_R4_WIFI: TargetDescriptor = TargetDescriptor {
    board_id: "arduino-uno-r4-wifi",
    display_name: "Arduino Uno R4 WiFi",
    variant: UnoR4Variant::Wifi,
    mcu: "Renesas RA4M1 R7FA4M1AB3CFM",
    core: "Arm Cortex-M4F",
    isa: "Armv7E-M Thumb-2",
    rust_target: "thumbv7em-none-eabihf",
    clock_hz: UNO_R4_CLOCK_HZ,
    flash_bytes: UNO_R4_FLASH_BYTES,
    sram_bytes: UNO_R4_SRAM_BYTES,
    data_flash_bytes: UNO_R4_DATA_FLASH_BYTES,
    operating_voltage_mv: 5000,
    onboard_led_pin: UNO_R4_ONBOARD_LED_PIN,
    supports_wifi_module: true,
    supports_led_matrix: true,
    capabilities: CapabilitySet::blink_mvp(),
    digital_pins: &UNO_R4_DIGITAL_PINS,
};

pub trait UnoR4Backend {
    fn configure_gpio(&mut self, pin: u8, mode: GpioMode) -> Result<(), HalError>;
    fn write_gpio(&mut self, pin: u8, level: Level) -> Result<(), HalError>;
    fn read_gpio(&mut self, pin: u8) -> Result<Level, HalError>;
    fn sleep_ms(&mut self, duration_ms: u16) -> Result<(), HalError>;
    fn now_ms(&self) -> u32;
}

pub struct UnoR4Board<B>
where
    B: UnoR4Backend,
{
    target: &'static TargetDescriptor,
    backend: B,
}

impl<B> UnoR4Board<B>
where
    B: UnoR4Backend,
{
    pub const fn new(target: &'static TargetDescriptor, backend: B) -> Self {
        Self { target, backend }
    }

    pub fn minima(backend: B) -> Self {
        Self::new(&UNO_R4_MINIMA, backend)
    }

    pub fn wifi(backend: B) -> Self {
        Self::new(&UNO_R4_WIFI, backend)
    }

    pub fn target(&self) -> &'static TargetDescriptor {
        self.target
    }

    pub fn backend(&self) -> &B {
        &self.backend
    }

    pub fn backend_mut(&mut self) -> &mut B {
        &mut self.backend
    }
}

impl<B> BoardHal for UnoR4Board<B>
where
    B: UnoR4Backend,
{
    fn capabilities(&self) -> CapabilitySet {
        self.target.capabilities
    }

    fn gpio_open(&mut self, pin: u16, mode: GpioMode) -> Result<u32, HalError> {
        let pin = normalize_digital_pin(pin)?;
        self.backend.configure_gpio(pin, mode)?;
        Ok(pin as u32)
    }

    fn gpio_write(&mut self, token: u32, level: Level) -> Result<(), HalError> {
        let pin = normalize_digital_pin(token as u16)?;
        self.backend.write_gpio(pin, level)
    }

    fn gpio_read(&mut self, token: u32) -> Result<Level, HalError> {
        let pin = normalize_digital_pin(token as u16)?;
        self.backend.read_gpio(pin)
    }

    fn gpio_close(&mut self, _token: u32) -> Result<(), HalError> {
        Ok(())
    }

    fn sleep_ms(&mut self, duration_ms: u16) -> Result<(), HalError> {
        self.backend.sleep_ms(duration_ms)
    }

    fn now_ms(&self) -> u32 {
        self.backend.now_ms()
    }
}

pub fn digital_pin(pin: u8) -> Option<&'static DigitalPinDescriptor> {
    UNO_R4_DIGITAL_PINS
        .iter()
        .find(|descriptor| descriptor.arduino_pin == pin)
}

pub fn is_valid_digital_pin(pin: u16) -> bool {
    pin <= 13
}

fn normalize_digital_pin(pin: u16) -> Result<u8, HalError> {
    if is_valid_digital_pin(pin) {
        Ok(pin as u8)
    } else {
        Err(HalError::InvalidPin)
    }
}

#[cfg(test)]
extern crate std;

#[cfg(test)]
mod tests {
    use super::*;
    use board_vm_runtime::{RunStatus, Runtime};
    use std::vec;
    use std::vec::Vec;

    const BLINK_CODE: &[u8] = &[
        0x12, 0x0d, 0x12, 0x01, 0x40, 0x01, 0x20, 0x11, 0x40, 0x02, 0x13, 0xfa, 0x00, 0x40, 0x10,
        0x20, 0x10, 0x40, 0x02, 0x13, 0xfa, 0x00, 0x40, 0x10, 0x30, 0xec,
    ];

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum Event {
        Configure(u8, GpioMode),
        Write(u8, Level),
        Sleep(u16),
    }

    struct FakeBackend {
        events: Vec<Event>,
        now_ms: u32,
    }

    impl FakeBackend {
        fn new() -> Self {
            Self {
                events: Vec::new(),
                now_ms: 0,
            }
        }
    }

    impl UnoR4Backend for FakeBackend {
        fn configure_gpio(&mut self, pin: u8, mode: GpioMode) -> Result<(), HalError> {
            self.events.push(Event::Configure(pin, mode));
            Ok(())
        }

        fn write_gpio(&mut self, pin: u8, level: Level) -> Result<(), HalError> {
            self.events.push(Event::Write(pin, level));
            Ok(())
        }

        fn read_gpio(&mut self, _pin: u8) -> Result<Level, HalError> {
            Ok(Level::Low)
        }

        fn sleep_ms(&mut self, duration_ms: u16) -> Result<(), HalError> {
            self.now_ms += duration_ms as u32;
            self.events.push(Event::Sleep(duration_ms));
            Ok(())
        }

        fn now_ms(&self) -> u32 {
            self.now_ms
        }
    }

    #[test]
    fn descriptor_targets_cortex_m4f() {
        assert_eq!(UNO_R4_MINIMA.core, "Arm Cortex-M4F");
        assert_eq!(UNO_R4_MINIMA.isa, "Armv7E-M Thumb-2");
        assert_eq!(UNO_R4_MINIMA.rust_target, "thumbv7em-none-eabihf");
        assert_eq!(UNO_R4_MINIMA.clock_hz, 48_000_000);
        assert_eq!(UNO_R4_MINIMA.flash_bytes, 256 * 1024);
        assert_eq!(UNO_R4_MINIMA.sram_bytes, 32 * 1024);
    }

    #[test]
    fn knows_uno_r4_d13_led_pin() {
        let d13 = digital_pin(13).unwrap();
        assert_eq!(d13.label, "D13/SCK");
        assert!(d13.notes.contains("onboard LED"));
    }

    #[test]
    fn blink_runs_through_abstract_uno_r4_backend() {
        let board = UnoR4Board::minima(FakeBackend::new());
        let mut runtime: Runtime<_, 8, 4> = Runtime::new(board);
        let report = runtime.run_code(BLINK_CODE, 13).unwrap();

        assert_eq!(report.status, RunStatus::BudgetExceeded);
        assert_eq!(
            runtime.hal().backend().events,
            vec![
                Event::Configure(13, GpioMode::Output),
                Event::Write(13, Level::High),
                Event::Sleep(250),
                Event::Write(13, Level::Low),
                Event::Sleep(250),
            ]
        );
    }

    #[test]
    fn rejects_non_uno_digital_pin() {
        let mut board = UnoR4Board::wifi(FakeBackend::new());
        assert_eq!(
            board.gpio_open(99, GpioMode::Output),
            Err(HalError::InvalidPin)
        );
    }
}
