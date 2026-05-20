#![no_std]

use board_vm_device::{BoardVmDevice, BLINK_MVP_CAPABILITIES, DEFAULT_MAX_FRAME_PAYLOAD};
use board_vm_ir::CapabilitySet;
use board_vm_runtime::{BoardHal, GpioMode, HalError, Level};

pub const ARDUINO_VM_RUNTIME_ID: &str = "board-vm-arduino";
pub const ARDUINO_VM_MAX_PROGRAM_BYTES: usize = 1024;
pub const ARDUINO_VM_MAX_STACK_VALUES: usize = 8;
pub const ARDUINO_VM_MAX_HANDLES: usize = 4;

pub type ArduinoDevice<B> = BoardVmDevice<
    'static,
    ArduinoBoard<B>,
    ARDUINO_VM_MAX_PROGRAM_BYTES,
    ARDUINO_VM_MAX_STACK_VALUES,
    ARDUINO_VM_MAX_HANDLES,
>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArduinoFamily {
    ClassicAvr,
    MegaAvr,
    Sam,
    Samd,
    RenesasRa,
    Mbed,
    Nordic,
    Rp2040,
    Esp32,
    Stm32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeProfile {
    Tiny,
    Small,
    Full,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RustTargetStatus {
    Stable(&'static str),
    Nightly(&'static str),
    BackendNeeded(&'static str),
}

impl RustTargetStatus {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Stable(target) | Self::Nightly(target) | Self::BackendNeeded(target) => target,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArduinoTargetDescriptor {
    pub board_id: &'static str,
    pub display_name: &'static str,
    pub family: ArduinoFamily,
    pub arduino_cli: ArduinoCliUploadDescriptor,
    pub runtime_profile: RuntimeProfile,
    pub mcu: &'static str,
    pub core: &'static str,
    pub isa: &'static str,
    pub rust_target: RustTargetStatus,
    pub clock_hz: u32,
    pub flash_bytes: u32,
    pub sram_bytes: u32,
    pub operating_voltage_mv: u16,
    pub onboard_led_pin: Option<u8>,
    pub capabilities: CapabilitySet,
    pub digital_pins: &'static [DigitalPinDescriptor],
    pub notes: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArduinoCliUploadDescriptor {
    pub platform_id: &'static str,
    pub fqbn: &'static str,
}

const fn arduino_cli(platform_id: &'static str, fqbn: &'static str) -> ArduinoCliUploadDescriptor {
    ArduinoCliUploadDescriptor { platform_id, fqbn }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DigitalPinDescriptor {
    pub pin: u8,
    pub label: &'static str,
    pub supports_input: bool,
    pub supports_output: bool,
    pub supports_pullup: bool,
    pub supports_pulldown: bool,
    pub supports_adc: bool,
    pub supports_pwm: bool,
    pub notes: &'static str,
}

impl DigitalPinDescriptor {
    pub const fn supports_mode(self, mode: GpioMode) -> bool {
        match mode {
            GpioMode::Input => self.supports_input,
            GpioMode::Output => self.supports_output,
            GpioMode::InputPullup => self.supports_input && self.supports_pullup,
            GpioMode::InputPulldown => self.supports_input && self.supports_pulldown,
        }
    }
}

pub trait ArduinoBackend {
    fn gpio_open(&mut self, pin: u8, _mode: GpioMode) -> Result<u32, HalError> {
        Ok(pin as u32)
    }

    fn gpio_write(&mut self, _token: u32, _level: Level) -> Result<(), HalError> {
        Ok(())
    }

    fn gpio_read(&mut self, _token: u32) -> Result<Level, HalError> {
        Ok(Level::Low)
    }

    fn gpio_close(&mut self, _token: u32) -> Result<(), HalError> {
        Ok(())
    }

    fn sleep_ms(&mut self, _duration_ms: u16) -> Result<(), HalError> {
        Ok(())
    }

    fn now_ms(&self) -> u32 {
        0
    }
}

pub struct ArduinoBoard<B> {
    target: &'static ArduinoTargetDescriptor,
    backend: B,
}

impl<B> ArduinoBoard<B> {
    pub const fn new(target: &'static ArduinoTargetDescriptor, backend: B) -> Self {
        Self { target, backend }
    }

    pub const fn target(&self) -> &'static ArduinoTargetDescriptor {
        self.target
    }

    pub const fn backend(&self) -> &B {
        &self.backend
    }

    pub fn backend_mut(&mut self) -> &mut B {
        &mut self.backend
    }
}

impl<B> BoardHal for ArduinoBoard<B>
where
    B: ArduinoBackend,
{
    fn capabilities(&self) -> CapabilitySet {
        self.target.capabilities
    }

    fn gpio_open(&mut self, pin: u16, mode: GpioMode) -> Result<u32, HalError> {
        let pin = normalize_digital_pin(self.target, pin, mode)?;
        self.backend.gpio_open(pin, mode)
    }

    fn gpio_write(&mut self, token: u32, level: Level) -> Result<(), HalError> {
        validate_token(self.target, token)?;
        self.backend.gpio_write(token, level)
    }

    fn gpio_read(&mut self, token: u32) -> Result<Level, HalError> {
        validate_token(self.target, token)?;
        self.backend.gpio_read(token)
    }

    fn gpio_close(&mut self, token: u32) -> Result<(), HalError> {
        validate_token(self.target, token)?;
        self.backend.gpio_close(token)
    }

    fn sleep_ms(&mut self, duration_ms: u16) -> Result<(), HalError> {
        self.backend.sleep_ms(duration_ms)
    }

    fn now_ms(&self) -> u32 {
        self.backend.now_ms()
    }
}

pub const fn blink_capabilities() -> CapabilitySet {
    CapabilitySet::blink_mvp()
}

pub fn digital_pin(
    target: &'static ArduinoTargetDescriptor,
    pin: u16,
) -> Option<DigitalPinDescriptor> {
    if pin > u8::MAX as u16 {
        return None;
    }
    let pin = pin as u8;
    let mut index = 0;
    while index < target.digital_pins.len() {
        if target.digital_pins[index].pin == pin {
            return Some(target.digital_pins[index]);
        }
        index += 1;
    }
    None
}

pub fn is_valid_digital_pin(
    target: &'static ArduinoTargetDescriptor,
    pin: u16,
    mode: GpioMode,
) -> bool {
    digital_pin(target, pin).is_some_and(|descriptor| descriptor.supports_mode(mode))
}

pub fn normalize_digital_pin(
    target: &'static ArduinoTargetDescriptor,
    pin: u16,
    mode: GpioMode,
) -> Result<u8, HalError> {
    let descriptor = digital_pin(target, pin).ok_or(HalError::InvalidPin)?;
    if descriptor.supports_mode(mode) {
        Ok(descriptor.pin)
    } else {
        Err(HalError::UnsupportedMode)
    }
}

fn validate_token(target: &'static ArduinoTargetDescriptor, token: u32) -> Result<u8, HalError> {
    if token > u16::MAX as u32 {
        return Err(HalError::InvalidPin);
    }
    digital_pin(target, token as u16)
        .map(|descriptor| descriptor.pin)
        .ok_or(HalError::InvalidPin)
}

const fn pin(
    pin: u8,
    label: &'static str,
    supports_adc: bool,
    supports_pwm: bool,
) -> DigitalPinDescriptor {
    DigitalPinDescriptor {
        pin,
        label,
        supports_input: true,
        supports_output: true,
        supports_pullup: true,
        supports_pulldown: false,
        supports_adc,
        supports_pwm,
        notes: "Arduino abstract GPIO pin; concrete backend owns MCU port mapping",
    }
}

const fn generated_pin(pin: u8, supports_adc: bool, supports_pwm: bool) -> DigitalPinDescriptor {
    DigitalPinDescriptor {
        pin,
        label: "GPIO",
        supports_input: true,
        supports_output: true,
        supports_pullup: true,
        supports_pulldown: false,
        supports_adc,
        supports_pwm,
        notes:
            "Generated Arduino-family GPIO descriptor; backend maps board header pin to MCU port",
    }
}

const fn generated_pins<const N: usize>(
    analog_start: u8,
    analog_count: u8,
) -> [DigitalPinDescriptor; N] {
    let mut pins = [DigitalPinDescriptor {
        pin: 0,
        label: "GPIO",
        supports_input: true,
        supports_output: true,
        supports_pullup: true,
        supports_pulldown: false,
        supports_adc: false,
        supports_pwm: false,
        notes:
            "Generated Arduino-family GPIO descriptor; backend maps board header pin to MCU port",
    }; N];
    let mut index = 0;
    while index < N {
        let pin = index as u8;
        pins[index] = generated_pin(
            pin,
            pin >= analog_start && pin < analog_start + analog_count,
            pin == 3 || pin == 5 || pin == 6 || pin == 9 || pin == 10 || pin == 11,
        );
        index += 1;
    }
    pins
}

pub const ARDUINO_STANDARD_DIGITAL_PINS: [DigitalPinDescriptor; 20] = [
    pin(0, "D0/RX", false, false),
    pin(1, "D1/TX", false, false),
    pin(2, "D2", false, false),
    pin(3, "D3/PWM", false, true),
    pin(4, "D4", false, false),
    pin(5, "D5/PWM", false, true),
    pin(6, "D6/PWM", false, true),
    pin(7, "D7", false, false),
    pin(8, "D8", false, false),
    pin(9, "D9/PWM", false, true),
    pin(10, "D10/PWM/SS", false, true),
    pin(11, "D11/PWM/MOSI", false, true),
    pin(12, "D12/MISO", false, false),
    pin(13, "D13/SCK/LED", false, false),
    pin(14, "A0/D14", true, false),
    pin(15, "A1/D15", true, false),
    pin(16, "A2/D16", true, false),
    pin(17, "A3/D17", true, false),
    pin(18, "A4/D18/SDA", true, false),
    pin(19, "A5/D19/SCL", true, false),
];

pub const ARDUINO_MEGA_2560_DIGITAL_PINS: [DigitalPinDescriptor; 70] = generated_pins::<70>(54, 16);
pub const ARDUINO_LEONARDO_DIGITAL_PINS: [DigitalPinDescriptor; 24] = generated_pins::<24>(18, 6);
pub const ARDUINO_DUE_DIGITAL_PINS: [DigitalPinDescriptor; 76] = generated_pins::<76>(54, 12);
pub const ARDUINO_NANO_EVERY_DIGITAL_PINS: [DigitalPinDescriptor; 22] = generated_pins::<22>(14, 8);
pub const ARDUINO_NANO_R4_DIGITAL_PINS: [DigitalPinDescriptor; 22] = generated_pins::<22>(14, 8);
pub const ARDUINO_NANO_33_BLE_DIGITAL_PINS: [DigitalPinDescriptor; 22] =
    generated_pins::<22>(14, 8);
pub const ARDUINO_NANO_RP2040_DIGITAL_PINS: [DigitalPinDescriptor; 22] =
    generated_pins::<22>(14, 8);
pub const ARDUINO_NANO_ESP32_DIGITAL_PINS: [DigitalPinDescriptor; 22] = generated_pins::<22>(14, 8);
pub const ARDUINO_GIGA_R1_DIGITAL_PINS: [DigitalPinDescriptor; 76] = generated_pins::<76>(54, 12);
pub const ARDUINO_PORTENTA_H7_DIGITAL_PINS: [DigitalPinDescriptor; 80] =
    generated_pins::<80>(54, 12);
pub const ARDUINO_PORTENTA_C33_DIGITAL_PINS: [DigitalPinDescriptor; 7] = generated_pins::<7>(7, 0);
pub const ARDUINO_NICLA_DIGITAL_PINS: [DigitalPinDescriptor; 12] = generated_pins::<12>(10, 2);
pub const ARDUINO_OPTA_TERMINAL_PINS: [DigitalPinDescriptor; 12] = [
    DigitalPinDescriptor {
        pin: 0,
        label: "I1",
        supports_input: true,
        supports_output: false,
        supports_pullup: false,
        supports_pulldown: false,
        supports_adc: true,
        supports_pwm: false,
        notes:
            "Opta industrial analog/digital input terminal; backend owns terminal voltage mapping",
    },
    DigitalPinDescriptor {
        pin: 1,
        label: "I2",
        supports_input: true,
        supports_output: false,
        supports_pullup: false,
        supports_pulldown: false,
        supports_adc: true,
        supports_pwm: false,
        notes:
            "Opta industrial analog/digital input terminal; backend owns terminal voltage mapping",
    },
    DigitalPinDescriptor {
        pin: 2,
        label: "I3",
        supports_input: true,
        supports_output: false,
        supports_pullup: false,
        supports_pulldown: false,
        supports_adc: true,
        supports_pwm: false,
        notes:
            "Opta industrial analog/digital input terminal; backend owns terminal voltage mapping",
    },
    DigitalPinDescriptor {
        pin: 3,
        label: "I4",
        supports_input: true,
        supports_output: false,
        supports_pullup: false,
        supports_pulldown: false,
        supports_adc: true,
        supports_pwm: false,
        notes:
            "Opta industrial analog/digital input terminal; backend owns terminal voltage mapping",
    },
    DigitalPinDescriptor {
        pin: 4,
        label: "I5",
        supports_input: true,
        supports_output: false,
        supports_pullup: false,
        supports_pulldown: false,
        supports_adc: true,
        supports_pwm: false,
        notes:
            "Opta industrial analog/digital input terminal; backend owns terminal voltage mapping",
    },
    DigitalPinDescriptor {
        pin: 5,
        label: "I6",
        supports_input: true,
        supports_output: false,
        supports_pullup: false,
        supports_pulldown: false,
        supports_adc: true,
        supports_pwm: false,
        notes:
            "Opta industrial analog/digital input terminal; backend owns terminal voltage mapping",
    },
    DigitalPinDescriptor {
        pin: 6,
        label: "I7",
        supports_input: true,
        supports_output: false,
        supports_pullup: false,
        supports_pulldown: false,
        supports_adc: true,
        supports_pwm: false,
        notes:
            "Opta industrial analog/digital input terminal; backend owns terminal voltage mapping",
    },
    DigitalPinDescriptor {
        pin: 7,
        label: "I8",
        supports_input: true,
        supports_output: false,
        supports_pullup: false,
        supports_pulldown: false,
        supports_adc: true,
        supports_pwm: false,
        notes:
            "Opta industrial analog/digital input terminal; backend owns terminal voltage mapping",
    },
    DigitalPinDescriptor {
        pin: 8,
        label: "O1",
        supports_input: false,
        supports_output: true,
        supports_pullup: false,
        supports_pulldown: false,
        supports_adc: false,
        supports_pwm: false,
        notes: "Opta relay output terminal; backend owns relay actuation mapping",
    },
    DigitalPinDescriptor {
        pin: 9,
        label: "O2",
        supports_input: false,
        supports_output: true,
        supports_pullup: false,
        supports_pulldown: false,
        supports_adc: false,
        supports_pwm: false,
        notes: "Opta relay output terminal; backend owns relay actuation mapping",
    },
    DigitalPinDescriptor {
        pin: 10,
        label: "O3",
        supports_input: false,
        supports_output: true,
        supports_pullup: false,
        supports_pulldown: false,
        supports_adc: false,
        supports_pwm: false,
        notes: "Opta relay output terminal; backend owns relay actuation mapping",
    },
    DigitalPinDescriptor {
        pin: 11,
        label: "O4",
        supports_input: false,
        supports_output: true,
        supports_pullup: false,
        supports_pulldown: false,
        supports_adc: false,
        supports_pwm: false,
        notes: "Opta relay output terminal; backend owns relay actuation mapping",
    },
];

pub const ARDUINO_UNO_R3: ArduinoTargetDescriptor = ArduinoTargetDescriptor {
    board_id: "arduino-uno-r3",
    display_name: "Arduino UNO R3",
    family: ArduinoFamily::ClassicAvr,
    arduino_cli: arduino_cli("arduino:avr", "arduino:avr:uno"),
    runtime_profile: RuntimeProfile::Tiny,
    mcu: "ATmega328P",
    core: "8-bit AVR",
    isa: "avr",
    rust_target: RustTargetStatus::BackendNeeded("avr-atmega328p"),
    clock_hz: 16_000_000,
    flash_bytes: 32 * 1024,
    sram_bytes: 2 * 1024,
    operating_voltage_mv: 5000,
    onboard_led_pin: Some(13),
    capabilities: blink_capabilities(),
    digital_pins: &ARDUINO_STANDARD_DIGITAL_PINS,
    notes: "Classic Arduino Uno backend contract; tiny profile starts with GPIO/time over the shared Arduino HAL shape.",
};

pub const ARDUINO_NANO_CLASSIC: ArduinoTargetDescriptor = ArduinoTargetDescriptor {
    board_id: "arduino-nano-classic",
    display_name: "Arduino Nano",
    family: ArduinoFamily::ClassicAvr,
    arduino_cli: arduino_cli("arduino:avr", "arduino:avr:nano:cpu=atmega328"),
    runtime_profile: RuntimeProfile::Tiny,
    mcu: "ATmega328P",
    core: "8-bit AVR",
    isa: "avr",
    rust_target: RustTargetStatus::BackendNeeded("avr-atmega328p"),
    clock_hz: 16_000_000,
    flash_bytes: 32 * 1024,
    sram_bytes: 2 * 1024,
    operating_voltage_mv: 5000,
    onboard_led_pin: Some(13),
    capabilities: blink_capabilities(),
    digital_pins: &ARDUINO_STANDARD_DIGITAL_PINS,
    notes: "Classic Nano backend contract; shares the ATmega328P tiny-profile backend with Uno R3.",
};

pub const ARDUINO_PRO_MINI: ArduinoTargetDescriptor = ArduinoTargetDescriptor {
    board_id: "arduino-pro-mini",
    display_name: "Arduino Pro Mini",
    family: ArduinoFamily::ClassicAvr,
    arduino_cli: arduino_cli("arduino:avr", "arduino:avr:pro:cpu=16MHzatmega328"),
    runtime_profile: RuntimeProfile::Tiny,
    mcu: "ATmega328P",
    core: "8-bit AVR",
    isa: "avr",
    rust_target: RustTargetStatus::BackendNeeded("avr-atmega328p"),
    clock_hz: 16_000_000,
    flash_bytes: 32 * 1024,
    sram_bytes: 2 * 1024,
    operating_voltage_mv: 5000,
    onboard_led_pin: Some(13),
    capabilities: blink_capabilities(),
    digital_pins: &ARDUINO_STANDARD_DIGITAL_PINS,
    notes: "External serial adapter expected; backend contract is still the same tiny-profile AVR GPIO/time path.",
};

pub const ARDUINO_MEGA_2560: ArduinoTargetDescriptor = ArduinoTargetDescriptor {
    board_id: "arduino-mega-2560",
    display_name: "Arduino Mega 2560 Rev3",
    family: ArduinoFamily::ClassicAvr,
    arduino_cli: arduino_cli("arduino:avr", "arduino:avr:mega:cpu=atmega2560"),
    runtime_profile: RuntimeProfile::Tiny,
    mcu: "ATmega2560",
    core: "8-bit AVR",
    isa: "avr",
    rust_target: RustTargetStatus::BackendNeeded("avr-atmega2560"),
    clock_hz: 16_000_000,
    flash_bytes: 256 * 1024,
    sram_bytes: 8 * 1024,
    operating_voltage_mv: 5000,
    onboard_led_pin: Some(13),
    capabilities: blink_capabilities(),
    digital_pins: &ARDUINO_MEGA_2560_DIGITAL_PINS,
    notes: "Larger AVR backend contract with more GPIO/UART surface than Uno R3.",
};

pub const ARDUINO_LEONARDO: ArduinoTargetDescriptor = ArduinoTargetDescriptor {
    board_id: "arduino-leonardo",
    display_name: "Arduino Leonardo",
    family: ArduinoFamily::ClassicAvr,
    arduino_cli: arduino_cli("arduino:avr", "arduino:avr:leonardo"),
    runtime_profile: RuntimeProfile::Tiny,
    mcu: "ATmega32U4",
    core: "8-bit AVR with USB",
    isa: "avr",
    rust_target: RustTargetStatus::BackendNeeded("avr-atmega32u4"),
    clock_hz: 16_000_000,
    flash_bytes: 32 * 1024,
    sram_bytes: 2_560,
    operating_voltage_mv: 5000,
    onboard_led_pin: Some(13),
    capabilities: blink_capabilities(),
    digital_pins: &ARDUINO_LEONARDO_DIGITAL_PINS,
    notes: "ATmega32U4 backend contract keeps native USB boards separate from Uno R3 serial assumptions.",
};

pub const ARDUINO_MICRO: ArduinoTargetDescriptor = ArduinoTargetDescriptor {
    board_id: "arduino-micro",
    display_name: "Arduino Micro",
    family: ArduinoFamily::ClassicAvr,
    arduino_cli: arduino_cli("arduino:avr", "arduino:avr:micro"),
    runtime_profile: RuntimeProfile::Tiny,
    mcu: "ATmega32U4",
    core: "8-bit AVR with USB",
    isa: "avr",
    rust_target: RustTargetStatus::BackendNeeded("avr-atmega32u4"),
    clock_hz: 16_000_000,
    flash_bytes: 32 * 1024,
    sram_bytes: 2_560,
    operating_voltage_mv: 5000,
    onboard_led_pin: Some(13),
    capabilities: blink_capabilities(),
    digital_pins: &ARDUINO_LEONARDO_DIGITAL_PINS,
    notes: "Micro uses the same ATmega32U4 backend family as Leonardo with a different board form factor.",
};

pub const ARDUINO_DUE: ArduinoTargetDescriptor = ArduinoTargetDescriptor {
    board_id: "arduino-due",
    display_name: "Arduino Due",
    family: ArduinoFamily::Sam,
    arduino_cli: arduino_cli("arduino:sam", "arduino:sam:arduino_due_x"),
    runtime_profile: RuntimeProfile::Full,
    mcu: "SAM3X8E",
    core: "Arm Cortex-M3",
    isa: "armv7-m",
    rust_target: RustTargetStatus::Stable("thumbv7m-none-eabi"),
    clock_hz: 84_000_000,
    flash_bytes: 512 * 1024,
    sram_bytes: 96 * 1024,
    operating_voltage_mv: 3300,
    onboard_led_pin: Some(13),
    capabilities: blink_capabilities(),
    digital_pins: &ARDUINO_DUE_DIGITAL_PINS,
    notes: "SAM backend contract proves the Arduino line is not restricted to AVR or Renesas RA.",
};

pub const ARDUINO_ZERO: ArduinoTargetDescriptor = ArduinoTargetDescriptor {
    board_id: "arduino-zero",
    display_name: "Arduino Zero",
    family: ArduinoFamily::Samd,
    arduino_cli: arduino_cli("arduino:samd", "arduino:samd:arduino_zero_native"),
    runtime_profile: RuntimeProfile::Small,
    mcu: "SAMD21G18",
    core: "Arm Cortex-M0+",
    isa: "armv6-m",
    rust_target: RustTargetStatus::Stable("thumbv6m-none-eabi"),
    clock_hz: 48_000_000,
    flash_bytes: 256 * 1024,
    sram_bytes: 32 * 1024,
    operating_voltage_mv: 3300,
    onboard_led_pin: Some(13),
    capabilities: blink_capabilities(),
    digital_pins: &ARDUINO_STANDARD_DIGITAL_PINS,
    notes: "SAMD21 backend contract for Zero/MKR-class Arduino boards.",
};

pub const ARDUINO_MKR_WIFI_1010: ArduinoTargetDescriptor = ArduinoTargetDescriptor {
    board_id: "arduino-mkr-wifi-1010",
    display_name: "Arduino MKR WiFi 1010",
    family: ArduinoFamily::Samd,
    arduino_cli: arduino_cli("arduino:samd", "arduino:samd:mkrwifi1010"),
    runtime_profile: RuntimeProfile::Small,
    mcu: "SAMD21G18",
    core: "Arm Cortex-M0+ with u-blox NINA-W102",
    isa: "armv6-m",
    rust_target: RustTargetStatus::Stable("thumbv6m-none-eabi"),
    clock_hz: 48_000_000,
    flash_bytes: 256 * 1024,
    sram_bytes: 32 * 1024,
    operating_voltage_mv: 3300,
    onboard_led_pin: Some(6),
    capabilities: blink_capabilities(),
    digital_pins: &ARDUINO_STANDARD_DIGITAL_PINS,
    notes: "MKR WiFi backend starts with the shared SAMD21 GPIO/time path; wireless is a later capability tranche.",
};

pub const ARDUINO_NANO_EVERY: ArduinoTargetDescriptor = ArduinoTargetDescriptor {
    board_id: "arduino-nano-every",
    display_name: "Arduino Nano Every",
    family: ArduinoFamily::MegaAvr,
    arduino_cli: arduino_cli("arduino:megaavr", "arduino:megaavr:nona4809:mode=off"),
    runtime_profile: RuntimeProfile::Tiny,
    mcu: "ATmega4809",
    core: "8-bit megaAVR 0-series",
    isa: "avr",
    rust_target: RustTargetStatus::BackendNeeded("avr-atmega4809"),
    clock_hz: 20_000_000,
    flash_bytes: 48 * 1024,
    sram_bytes: 6 * 1024,
    operating_voltage_mv: 5000,
    onboard_led_pin: Some(13),
    capabilities: blink_capabilities(),
    digital_pins: &ARDUINO_NANO_EVERY_DIGITAL_PINS,
    notes: "megaAVR backend contract keeps newer Nano AVR boards out of the Uno R3 bucket.",
};

pub const ARDUINO_NANO_R4: ArduinoTargetDescriptor = ArduinoTargetDescriptor {
    board_id: "arduino-nano-r4",
    display_name: "Arduino Nano R4",
    family: ArduinoFamily::RenesasRa,
    arduino_cli: arduino_cli("arduino:renesas_uno", "arduino:renesas_uno:nanor4"),
    runtime_profile: RuntimeProfile::Full,
    mcu: "RA4M1",
    core: "Arm Cortex-M4F",
    isa: "armv7e-m",
    rust_target: RustTargetStatus::Stable("thumbv7em-none-eabihf"),
    clock_hz: 48_000_000,
    flash_bytes: 256 * 1024,
    sram_bytes: 32 * 1024,
    operating_voltage_mv: 5000,
    onboard_led_pin: Some(13),
    capabilities: blink_capabilities(),
    digital_pins: &ARDUINO_NANO_R4_DIGITAL_PINS,
    notes: "Renesas RA Nano-family backend contract; separate from Uno R4 despite the shared RA4M1 MCU class.",
};

pub const ARDUINO_NANO_33_IOT: ArduinoTargetDescriptor = ArduinoTargetDescriptor {
    board_id: "arduino-nano-33-iot",
    display_name: "Arduino Nano 33 IoT",
    family: ArduinoFamily::Samd,
    arduino_cli: arduino_cli("arduino:samd", "arduino:samd:nano_33_iot"),
    runtime_profile: RuntimeProfile::Small,
    mcu: "SAMD21G18",
    core: "Arm Cortex-M0+ with u-blox NINA-W102",
    isa: "armv6-m",
    rust_target: RustTargetStatus::Stable("thumbv6m-none-eabi"),
    clock_hz: 48_000_000,
    flash_bytes: 256 * 1024,
    sram_bytes: 32 * 1024,
    operating_voltage_mv: 3300,
    onboard_led_pin: Some(13),
    capabilities: blink_capabilities(),
    digital_pins: &ARDUINO_STANDARD_DIGITAL_PINS,
    notes: "Nano 33 IoT backend contract shares SAMD21 runtime support with Zero/MKR but stays discoverable as its own board.",
};

pub const ARDUINO_NANO_33_BLE_REV2: ArduinoTargetDescriptor = ArduinoTargetDescriptor {
    board_id: "arduino-nano-33-ble-rev2",
    display_name: "Arduino Nano 33 BLE Rev2",
    family: ArduinoFamily::Nordic,
    arduino_cli: arduino_cli("arduino:mbed_nano", "arduino:mbed_nano:nano33ble"),
    runtime_profile: RuntimeProfile::Full,
    mcu: "nRF52840",
    core: "Arm Cortex-M4F with Bluetooth LE",
    isa: "armv7e-m",
    rust_target: RustTargetStatus::Stable("thumbv7em-none-eabihf"),
    clock_hz: 64_000_000,
    flash_bytes: 1024 * 1024,
    sram_bytes: 256 * 1024,
    operating_voltage_mv: 3300,
    onboard_led_pin: Some(13),
    capabilities: blink_capabilities(),
    digital_pins: &ARDUINO_NANO_33_BLE_DIGITAL_PINS,
    notes: "Nordic nRF52 Arduino backend contract; Bluetooth capabilities land in a later descriptor tranche.",
};

pub const ARDUINO_NANO_RP2040_CONNECT: ArduinoTargetDescriptor = ArduinoTargetDescriptor {
    board_id: "arduino-nano-rp2040-connect",
    display_name: "Arduino Nano RP2040 Connect",
    family: ArduinoFamily::Rp2040,
    arduino_cli: arduino_cli(
        "arduino:mbed_nano",
        "arduino:mbed_nano:nanorp2040connect",
    ),
    runtime_profile: RuntimeProfile::Full,
    mcu: "RP2040",
    core: "dual Arm Cortex-M0+ with u-blox NINA-W102",
    isa: "armv6-m",
    rust_target: RustTargetStatus::Stable("thumbv6m-none-eabi"),
    clock_hz: 133_000_000,
    flash_bytes: 16 * 1024 * 1024,
    sram_bytes: 264 * 1024,
    operating_voltage_mv: 3300,
    onboard_led_pin: Some(13),
    capabilities: blink_capabilities(),
    digital_pins: &ARDUINO_NANO_RP2040_DIGITAL_PINS,
    notes: "Arduino RP2040 backend contract reuses the Board VM RP2040 direction without pretending it is a Raspberry Pi Pico board.",
};

pub const ARDUINO_NANO_ESP32: ArduinoTargetDescriptor = ArduinoTargetDescriptor {
    board_id: "arduino-nano-esp32",
    display_name: "Arduino Nano ESP32",
    family: ArduinoFamily::Esp32,
    arduino_cli: arduino_cli("arduino:esp32", "arduino:esp32:nano_nora"),
    runtime_profile: RuntimeProfile::Full,
    mcu: "ESP32-S3",
    core: "Xtensa LX7",
    isa: "xtensa-lx7",
    rust_target: RustTargetStatus::Nightly("xtensa-esp32s3-none-elf"),
    clock_hz: 240_000_000,
    flash_bytes: 16 * 1024 * 1024,
    sram_bytes: 512 * 1024,
    operating_voltage_mv: 3300,
    onboard_led_pin: None,
    capabilities: blink_capabilities(),
    digital_pins: &ARDUINO_NANO_ESP32_DIGITAL_PINS,
    notes: "Arduino Nano ESP32 backend contract tracks Arduino's ESP32-S3 board separately from generic ESP32 DevKit descriptors.",
};

pub const ARDUINO_GIGA_R1_WIFI: ArduinoTargetDescriptor = ArduinoTargetDescriptor {
    board_id: "arduino-giga-r1-wifi",
    display_name: "Arduino GIGA R1 WiFi",
    family: ArduinoFamily::Stm32,
    arduino_cli: arduino_cli("arduino:mbed_giga", "arduino:mbed_giga:giga"),
    runtime_profile: RuntimeProfile::Full,
    mcu: "STM32H747XI",
    core: "dual Arm Cortex-M7/M4",
    isa: "armv7e-m",
    rust_target: RustTargetStatus::Stable("thumbv7em-none-eabihf"),
    clock_hz: 480_000_000,
    flash_bytes: 2 * 1024 * 1024,
    sram_bytes: 1024 * 1024,
    operating_voltage_mv: 3300,
    onboard_led_pin: None,
    capabilities: blink_capabilities(),
    digital_pins: &ARDUINO_GIGA_R1_DIGITAL_PINS,
    notes: "STM32H7 Arduino backend contract; richer storage/network/peripheral capabilities should land as descriptor-backed follow-ups.",
};

pub const ARDUINO_PORTENTA_H7: ArduinoTargetDescriptor = ArduinoTargetDescriptor {
    board_id: "arduino-portenta-h7",
    display_name: "Arduino Portenta H7",
    family: ArduinoFamily::Mbed,
    arduino_cli: arduino_cli("arduino:mbed_portenta", "arduino:mbed_portenta:envie_m7"),
    runtime_profile: RuntimeProfile::Full,
    mcu: "STM32H747XI",
    core: "dual Arm Cortex-M7/M4",
    isa: "armv7e-m",
    rust_target: RustTargetStatus::Stable("thumbv7em-none-eabihf"),
    clock_hz: 480_000_000,
    flash_bytes: 2 * 1024 * 1024,
    sram_bytes: 1024 * 1024,
    operating_voltage_mv: 3300,
    onboard_led_pin: Some(66),
    capabilities: blink_capabilities(),
    digital_pins: &ARDUINO_PORTENTA_H7_DIGITAL_PINS,
    notes: "Portenta H7 backend contract keeps high-end Arduino Pro hardware in the target matrix.",
};

pub const ARDUINO_PORTENTA_H7_LITE: ArduinoTargetDescriptor = ArduinoTargetDescriptor {
    board_id: "arduino-portenta-h7-lite",
    display_name: "Arduino Portenta H7 Lite",
    family: ArduinoFamily::Mbed,
    arduino_cli: arduino_cli("arduino:mbed_portenta", "arduino:mbed_portenta:envie_m7"),
    runtime_profile: RuntimeProfile::Full,
    mcu: "STM32H747XI",
    core: "dual Arm Cortex-M7/M4",
    isa: "armv7e-m",
    rust_target: RustTargetStatus::Stable("thumbv7em-none-eabihf"),
    clock_hz: 480_000_000,
    flash_bytes: 2 * 1024 * 1024,
    sram_bytes: 1024 * 1024,
    operating_voltage_mv: 3300,
    onboard_led_pin: Some(66),
    capabilities: blink_capabilities(),
    digital_pins: &ARDUINO_PORTENTA_H7_DIGITAL_PINS,
    notes: "Portenta H7 Lite descriptor keeps lower-cost H7 modules visible without routing them through the base Portenta H7 board id.",
};

pub const ARDUINO_PORTENTA_H7_LITE_CONNECTED: ArduinoTargetDescriptor =
    ArduinoTargetDescriptor {
        board_id: "arduino-portenta-h7-lite-connected",
        display_name: "Arduino Portenta H7 Lite Connected",
        family: ArduinoFamily::Mbed,
        arduino_cli: arduino_cli("arduino:mbed_portenta", "arduino:mbed_portenta:envie_m7"),
        runtime_profile: RuntimeProfile::Full,
        mcu: "STM32H747XI",
        core: "dual Arm Cortex-M7/M4 with WiFi/BLE module",
        isa: "armv7e-m",
        rust_target: RustTargetStatus::Stable("thumbv7em-none-eabihf"),
        clock_hz: 480_000_000,
        flash_bytes: 2 * 1024 * 1024,
        sram_bytes: 1024 * 1024,
        operating_voltage_mv: 3300,
        onboard_led_pin: Some(66),
        capabilities: blink_capabilities(),
        digital_pins: &ARDUINO_PORTENTA_H7_DIGITAL_PINS,
        notes: "Portenta H7 Lite Connected keeps wireless-capable H7 modules discoverable; network capabilities are a later descriptor tranche.",
    };

pub const ARDUINO_PORTENTA_C33: ArduinoTargetDescriptor = ArduinoTargetDescriptor {
    board_id: "arduino-portenta-c33",
    display_name: "Arduino Portenta C33",
    family: ArduinoFamily::RenesasRa,
    arduino_cli: arduino_cli(
        "arduino:renesas_portenta",
        "arduino:renesas_portenta:portenta_c33",
    ),
    runtime_profile: RuntimeProfile::Full,
    mcu: "R7FA6M5BH2CBG",
    core: "Arm Cortex-M33 with ESP32-C3 WiFi/BLE module",
    isa: "armv8-m.main",
    rust_target: RustTargetStatus::Stable("thumbv8m.main-none-eabihf"),
    clock_hz: 200_000_000,
    flash_bytes: 2 * 1024 * 1024,
    sram_bytes: 512 * 1024,
    operating_voltage_mv: 3300,
    onboard_led_pin: None,
    capabilities: blink_capabilities(),
    digital_pins: &ARDUINO_PORTENTA_C33_DIGITAL_PINS,
    notes: "Renesas RA6M5 Pro-family descriptor; firmware/upload support should be implemented as a Portenta C33 adapter, not through Uno R4.",
};

pub const ARDUINO_NICLA_VISION: ArduinoTargetDescriptor = ArduinoTargetDescriptor {
    board_id: "arduino-nicla-vision",
    display_name: "Arduino Nicla Vision",
    family: ArduinoFamily::Mbed,
    arduino_cli: arduino_cli("arduino:mbed_nicla", "arduino:mbed_nicla:nicla_vision"),
    runtime_profile: RuntimeProfile::Full,
    mcu: "STM32H747AII6",
    core: "dual Arm Cortex-M7/M4 with camera and WiFi/BLE module",
    isa: "armv7e-m",
    rust_target: RustTargetStatus::Stable("thumbv7em-none-eabihf"),
    clock_hz: 480_000_000,
    flash_bytes: 2 * 1024 * 1024,
    sram_bytes: 1024 * 1024,
    operating_voltage_mv: 3300,
    onboard_led_pin: None,
    capabilities: blink_capabilities(),
    digital_pins: &ARDUINO_NICLA_DIGITAL_PINS,
    notes: "Nicla Vision descriptor keeps vision-class Arduino hardware in the shared backend matrix; camera, microphone, and wireless capabilities are later tranches.",
};

pub const ARDUINO_NICLA_SENSE_ME: ArduinoTargetDescriptor = ArduinoTargetDescriptor {
    board_id: "arduino-nicla-sense-me",
    display_name: "Arduino Nicla Sense ME",
    family: ArduinoFamily::Nordic,
    arduino_cli: arduino_cli("arduino:mbed_nicla", "arduino:mbed_nicla:nicla_sense"),
    runtime_profile: RuntimeProfile::Small,
    mcu: "nRF52832",
    core: "Arm Cortex-M4F with Bosch smart sensor hub",
    isa: "armv7e-m",
    rust_target: RustTargetStatus::Stable("thumbv7em-none-eabihf"),
    clock_hz: 64_000_000,
    flash_bytes: 512 * 1024,
    sram_bytes: 64 * 1024,
    operating_voltage_mv: 1800,
    onboard_led_pin: None,
    capabilities: blink_capabilities(),
    digital_pins: &ARDUINO_NICLA_DIGITAL_PINS,
    notes: "Nicla Sense ME descriptor tracks the nRF52832 Arduino core separately from Nano BLE and leaves Bosch sensor fusion as a later backend capability.",
};

pub const ARDUINO_NICLA_VOICE: ArduinoTargetDescriptor = ArduinoTargetDescriptor {
    board_id: "arduino-nicla-voice",
    display_name: "Arduino Nicla Voice",
    family: ArduinoFamily::Nordic,
    arduino_cli: arduino_cli("arduino:mbed_nicla", "arduino:mbed_nicla:nicla_voice"),
    runtime_profile: RuntimeProfile::Small,
    mcu: "nRF52832",
    core: "Arm Cortex-M4F with Syntiant NDP120",
    isa: "armv7e-m",
    rust_target: RustTargetStatus::Stable("thumbv7em-none-eabihf"),
    clock_hz: 64_000_000,
    flash_bytes: 512 * 1024,
    sram_bytes: 64 * 1024,
    operating_voltage_mv: 1800,
    onboard_led_pin: None,
    capabilities: blink_capabilities(),
    digital_pins: &ARDUINO_NICLA_DIGITAL_PINS,
    notes: "Nicla Voice descriptor keeps voice/ML Arduino hardware discoverable; NDP120 and microphone paths are later capability tranches.",
};

pub const ARDUINO_OPTA_LITE: ArduinoTargetDescriptor = ArduinoTargetDescriptor {
    board_id: "arduino-opta-lite",
    display_name: "Arduino Opta Lite",
    family: ArduinoFamily::Stm32,
    arduino_cli: arduino_cli("arduino:mbed_opta", "arduino:mbed_opta:opta"),
    runtime_profile: RuntimeProfile::Full,
    mcu: "STM32H747XI",
    core: "dual Arm Cortex-M7/M4 with Ethernet",
    isa: "armv7e-m",
    rust_target: RustTargetStatus::Stable("thumbv7em-none-eabihf"),
    clock_hz: 480_000_000,
    flash_bytes: 2 * 1024 * 1024,
    sram_bytes: 1024 * 1024,
    operating_voltage_mv: 24_000,
    onboard_led_pin: None,
    capabilities: blink_capabilities(),
    digital_pins: &ARDUINO_OPTA_TERMINAL_PINS,
    notes: "Industrial Opta Lite descriptor models terminal inputs and relays as board-local GPIO abstractions; Ethernet and PLC metadata are later tranches.",
};

pub const ARDUINO_OPTA_RS485: ArduinoTargetDescriptor = ArduinoTargetDescriptor {
    board_id: "arduino-opta-rs485",
    display_name: "Arduino Opta RS485",
    family: ArduinoFamily::Stm32,
    arduino_cli: arduino_cli("arduino:mbed_opta", "arduino:mbed_opta:opta"),
    runtime_profile: RuntimeProfile::Full,
    mcu: "STM32H747XI",
    core: "dual Arm Cortex-M7/M4 with Ethernet and RS-485",
    isa: "armv7e-m",
    rust_target: RustTargetStatus::Stable("thumbv7em-none-eabihf"),
    clock_hz: 480_000_000,
    flash_bytes: 2 * 1024 * 1024,
    sram_bytes: 1024 * 1024,
    operating_voltage_mv: 24_000,
    onboard_led_pin: None,
    capabilities: blink_capabilities(),
    digital_pins: &ARDUINO_OPTA_TERMINAL_PINS,
    notes: "Opta RS485 descriptor keeps industrial serial hardware distinct from generic STM32 and Uno R4 adapters.",
};

pub const ARDUINO_OPTA_WIFI: ArduinoTargetDescriptor = ArduinoTargetDescriptor {
    board_id: "arduino-opta-wifi",
    display_name: "Arduino Opta WiFi",
    family: ArduinoFamily::Stm32,
    arduino_cli: arduino_cli("arduino:mbed_opta", "arduino:mbed_opta:opta"),
    runtime_profile: RuntimeProfile::Full,
    mcu: "STM32H747XI",
    core: "dual Arm Cortex-M7/M4 with Ethernet, RS-485, and WiFi/BLE",
    isa: "armv7e-m",
    rust_target: RustTargetStatus::Stable("thumbv7em-none-eabihf"),
    clock_hz: 480_000_000,
    flash_bytes: 2 * 1024 * 1024,
    sram_bytes: 1024 * 1024,
    operating_voltage_mv: 24_000,
    onboard_led_pin: None,
    capabilities: blink_capabilities(),
    digital_pins: &ARDUINO_OPTA_TERMINAL_PINS,
    notes: "Opta WiFi descriptor keeps PLC wireless hardware discoverable; Ethernet, RS-485, WiFi, and BLE capabilities are later tranches.",
};

pub const ARDUINO_TARGETS: [ArduinoTargetDescriptor; 26] = [
    ARDUINO_UNO_R3,
    ARDUINO_NANO_CLASSIC,
    ARDUINO_PRO_MINI,
    ARDUINO_MEGA_2560,
    ARDUINO_LEONARDO,
    ARDUINO_MICRO,
    ARDUINO_DUE,
    ARDUINO_ZERO,
    ARDUINO_MKR_WIFI_1010,
    ARDUINO_NANO_EVERY,
    ARDUINO_NANO_R4,
    ARDUINO_NANO_33_IOT,
    ARDUINO_NANO_33_BLE_REV2,
    ARDUINO_NANO_RP2040_CONNECT,
    ARDUINO_NANO_ESP32,
    ARDUINO_GIGA_R1_WIFI,
    ARDUINO_PORTENTA_H7,
    ARDUINO_PORTENTA_H7_LITE,
    ARDUINO_PORTENTA_H7_LITE_CONNECTED,
    ARDUINO_PORTENTA_C33,
    ARDUINO_NICLA_VISION,
    ARDUINO_NICLA_SENSE_ME,
    ARDUINO_NICLA_VOICE,
    ARDUINO_OPTA_LITE,
    ARDUINO_OPTA_RS485,
    ARDUINO_OPTA_WIFI,
];

pub fn arduino_device_descriptor(
    target: &'static ArduinoTargetDescriptor,
    board_nonce: u32,
) -> board_vm_device::DeviceDescriptor<'static> {
    board_vm_device::DeviceDescriptor {
        board_id: target.board_id,
        runtime_id: ARDUINO_VM_RUNTIME_ID,
        board_nonce,
        max_frame_payload: DEFAULT_MAX_FRAME_PAYLOAD,
        supports_store_program: false,
        capabilities: &BLINK_MVP_CAPABILITIES,
    }
}

#[cfg(test)]
extern crate std;

#[cfg(test)]
mod tests {
    use super::*;
    use board_vm_host::{write_blink_module, BlinkProgram, BLINK_MODULE_LEN};
    use board_vm_ir::parse_module;
    use board_vm_runtime::{RunStatus, Runtime};
    use std::vec;
    use std::vec::Vec;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum Event {
        Configure(u8, GpioMode),
        Write(u32, Level),
        Sleep(u16),
    }

    #[derive(Default)]
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

    impl ArduinoBackend for FakeBackend {
        fn gpio_open(&mut self, pin: u8, mode: GpioMode) -> Result<u32, HalError> {
            self.events.push(Event::Configure(pin, mode));
            Ok(pin as u32)
        }

        fn gpio_write(&mut self, token: u32, level: Level) -> Result<(), HalError> {
            self.events.push(Event::Write(token, level));
            Ok(())
        }

        fn sleep_ms(&mut self, duration_ms: u16) -> Result<(), HalError> {
            self.events.push(Event::Sleep(duration_ms));
            self.now_ms += duration_ms as u32;
            Ok(())
        }

        fn now_ms(&self) -> u32 {
            self.now_ms
        }
    }

    #[test]
    fn target_list_includes_multiple_arduino_backend_families() {
        assert!(ARDUINO_TARGETS
            .iter()
            .any(|target| target.family == ArduinoFamily::ClassicAvr));
        assert!(ARDUINO_TARGETS
            .iter()
            .any(|target| target.family == ArduinoFamily::Samd));
        assert!(ARDUINO_TARGETS
            .iter()
            .any(|target| target.family == ArduinoFamily::RenesasRa));
        assert!(ARDUINO_TARGETS
            .iter()
            .any(|target| target.family == ArduinoFamily::Nordic));
        assert!(ARDUINO_TARGETS
            .iter()
            .any(|target| target.family == ArduinoFamily::Rp2040));
        assert!(ARDUINO_TARGETS
            .iter()
            .any(|target| target.family == ArduinoFamily::Esp32));
        assert!(ARDUINO_TARGETS
            .iter()
            .any(|target| target.family == ArduinoFamily::Stm32));
        assert!(ARDUINO_TARGETS
            .iter()
            .any(|target| target.family == ArduinoFamily::Mbed));
        assert!(ARDUINO_TARGETS
            .iter()
            .all(|target| target.digital_pins.iter().any(|pin| pin.supports_output)));
    }

    #[test]
    fn every_arduino_target_runs_blink_through_shared_backend() {
        for target in ARDUINO_TARGETS.iter() {
            let pin = target.onboard_led_pin.unwrap_or_else(|| {
                target
                    .digital_pins
                    .iter()
                    .find(|pin| pin.supports_output)
                    .unwrap()
                    .pin
            });
            let board = ArduinoBoard::new(target, FakeBackend::new());
            let mut runtime: Runtime<_, 8, 4> = Runtime::new(board);
            let mut module = [0u8; BLINK_MODULE_LEN];
            let len = write_blink_module(
                BlinkProgram {
                    pin,
                    high_ms: 25,
                    low_ms: 25,
                    max_stack: 4,
                },
                &mut module,
            )
            .unwrap();
            let module = parse_module(&module[..len]).unwrap();
            let report = runtime.run_module(&module, 13).unwrap();

            assert_eq!(
                report.status,
                RunStatus::BudgetExceeded,
                "{}",
                target.board_id
            );
            assert_eq!(
                runtime.hal().backend().events,
                vec![
                    Event::Configure(pin, GpioMode::Output),
                    Event::Write(pin as u32, Level::High),
                    Event::Sleep(25),
                    Event::Write(pin as u32, Level::Low),
                    Event::Sleep(25),
                ],
                "{}",
                target.board_id
            );
        }
    }

    #[test]
    fn backend_rejects_unknown_pins_before_backend_io() {
        let mut board = ArduinoBoard::new(&ARDUINO_UNO_R3, FakeBackend::new());
        assert_eq!(
            board.gpio_open(255, GpioMode::Output),
            Err(HalError::InvalidPin)
        );
        assert!(board.backend().events.is_empty());
    }

    #[test]
    fn descriptors_map_to_device_descriptor_contract() {
        let descriptor = arduino_device_descriptor(&ARDUINO_MEGA_2560, 0xA0D1_0001);
        assert_eq!(descriptor.board_id, ARDUINO_MEGA_2560.board_id);
        assert_eq!(descriptor.runtime_id, ARDUINO_VM_RUNTIME_ID);
        assert_eq!(descriptor.board_nonce, 0xA0D1_0001);
        assert_eq!(descriptor.max_frame_payload, DEFAULT_MAX_FRAME_PAYLOAD);
        assert!(!descriptor.supports_store_program);
        assert_eq!(descriptor.capabilities.len(), BLINK_MVP_CAPABILITIES.len());
    }
}
