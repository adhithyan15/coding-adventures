#![no_std]

use board_vm_device::{
    BoardVmDevice, DeviceDescriptor, BLINK_MVP_CAPABILITIES, DEFAULT_MAX_FRAME_PAYLOAD,
};
use board_vm_ir::CapabilitySet;
use board_vm_runtime::{BoardHal, GpioMode, HalError, Level};

pub const ESP32_CLOCK_HZ: u32 = 240_000_000;
pub const ESP32_FLASH_BYTES: u32 = 4 * 1024 * 1024;
pub const ESP32_SRAM_BYTES: u32 = 520 * 1024;
pub const ESP32_OPERATING_VOLTAGE_MV: u16 = 3300;
pub const ESP32_DEVKIT_V1_ONBOARD_LED_PIN: u8 = 2;
pub const ESP32_VM_RUNTIME_ID: &str = "board-vm-esp32";
pub const ESP32_VM_MAX_PROGRAM_BYTES: usize = 8192;
pub const ESP32_VM_MAX_STACK_VALUES: usize = 32;
pub const ESP32_VM_MAX_HANDLES: usize = 16;

pub type Esp32Device<B> = BoardVmDevice<
    'static,
    Esp32Board<B>,
    ESP32_VM_MAX_PROGRAM_BYTES,
    ESP32_VM_MAX_STACK_VALUES,
    ESP32_VM_MAX_HANDLES,
>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Esp32Variant {
    DevKitV1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TargetDescriptor {
    pub board_id: &'static str,
    pub display_name: &'static str,
    pub variant: Esp32Variant,
    pub module: &'static str,
    pub mcu: &'static str,
    pub core: &'static str,
    pub isa: &'static str,
    pub rust_target: &'static str,
    pub clock_hz: u32,
    pub flash_bytes: u32,
    pub sram_bytes: u32,
    pub operating_voltage_mv: u16,
    pub onboard_led_pin: Option<u8>,
    pub supports_wifi: bool,
    pub supports_bluetooth: bool,
    pub capabilities: CapabilitySet,
    pub digital_pins: &'static [DigitalPinDescriptor],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DigitalPinDescriptor {
    pub gpio: u8,
    pub label: &'static str,
    pub supports_input: bool,
    pub supports_output: bool,
    pub supports_pullup: bool,
    pub supports_pulldown: bool,
    pub supports_adc: bool,
    pub supports_touch: bool,
    pub boot_strap: bool,
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

pub const ESP32_DEVKIT_V1_DIGITAL_PINS: [DigitalPinDescriptor; 30] = [
    pin(
        0,
        "GPIO0/BOOT",
        true,
        true,
        true,
        true,
        false,
        true,
        true,
        "boot strap; avoid changing during reset",
    ),
    pin(
        1,
        "GPIO1/TX0",
        true,
        true,
        true,
        true,
        false,
        false,
        false,
        "UART0 transmit; shared with flashing console",
    ),
    pin(
        2,
        "GPIO2/LED",
        true,
        true,
        true,
        true,
        false,
        true,
        true,
        "common onboard LED; boot strap on many modules",
    ),
    pin(
        3,
        "GPIO3/RX0",
        true,
        true,
        true,
        true,
        false,
        false,
        false,
        "UART0 receive; shared with flashing console",
    ),
    pin(
        4,
        "GPIO4",
        true,
        true,
        true,
        true,
        false,
        true,
        true,
        "GPIO / touch-capable",
    ),
    pin(
        5,
        "GPIO5",
        true,
        true,
        true,
        true,
        false,
        false,
        true,
        "GPIO / common SPI CS; boot strap",
    ),
    pin(
        12,
        "GPIO12",
        true,
        true,
        true,
        true,
        true,
        true,
        true,
        "ADC2 / touch; boot strap",
    ),
    pin(
        13,
        "GPIO13",
        true,
        true,
        true,
        true,
        true,
        true,
        false,
        "ADC2 / touch",
    ),
    pin(
        14,
        "GPIO14",
        true,
        true,
        true,
        true,
        true,
        true,
        false,
        "ADC2 / touch",
    ),
    pin(
        15,
        "GPIO15",
        true,
        true,
        true,
        true,
        true,
        true,
        true,
        "ADC2 / touch; boot strap",
    ),
    pin(
        16, "GPIO16", true, true, true, true, false, false, false, "GPIO",
    ),
    pin(
        17, "GPIO17", true, true, true, true, false, false, false, "GPIO",
    ),
    pin(
        18,
        "GPIO18/SCK",
        true,
        true,
        true,
        true,
        false,
        false,
        false,
        "common SPI clock",
    ),
    pin(
        19,
        "GPIO19/MISO",
        true,
        true,
        true,
        true,
        false,
        false,
        false,
        "common SPI input",
    ),
    pin(
        21,
        "GPIO21/SDA",
        true,
        true,
        true,
        true,
        false,
        false,
        false,
        "common I2C SDA",
    ),
    pin(
        22,
        "GPIO22/SCL",
        true,
        true,
        true,
        true,
        false,
        false,
        false,
        "common I2C SCL",
    ),
    pin(
        23,
        "GPIO23/MOSI",
        true,
        true,
        true,
        true,
        false,
        false,
        false,
        "common SPI output",
    ),
    pin(
        25,
        "GPIO25/DAC1",
        true,
        true,
        true,
        true,
        true,
        false,
        false,
        "ADC2 / DAC1",
    ),
    pin(
        26,
        "GPIO26/DAC2",
        true,
        true,
        true,
        true,
        true,
        false,
        false,
        "ADC2 / DAC2",
    ),
    pin(
        27,
        "GPIO27",
        true,
        true,
        true,
        true,
        true,
        true,
        false,
        "ADC2 / touch",
    ),
    pin(
        32,
        "GPIO32",
        true,
        true,
        true,
        true,
        true,
        true,
        false,
        "ADC1 / touch",
    ),
    pin(
        33,
        "GPIO33",
        true,
        true,
        true,
        true,
        true,
        true,
        false,
        "ADC1 / touch",
    ),
    pin(
        34,
        "GPIO34/ADC1",
        true,
        false,
        false,
        false,
        true,
        false,
        false,
        "input-only; no internal pulls",
    ),
    pin(
        35,
        "GPIO35/ADC1",
        true,
        false,
        false,
        false,
        true,
        false,
        false,
        "input-only; no internal pulls",
    ),
    pin(
        36,
        "GPIO36/VP",
        true,
        false,
        false,
        false,
        true,
        false,
        false,
        "input-only; no internal pulls",
    ),
    pin(
        39,
        "GPIO39/VN",
        true,
        false,
        false,
        false,
        true,
        false,
        false,
        "input-only; no internal pulls",
    ),
    pin(
        6,
        "GPIO6/FLASH",
        false,
        false,
        false,
        false,
        false,
        false,
        false,
        "reserved for module flash",
    ),
    pin(
        7,
        "GPIO7/FLASH",
        false,
        false,
        false,
        false,
        false,
        false,
        false,
        "reserved for module flash",
    ),
    pin(
        8,
        "GPIO8/FLASH",
        false,
        false,
        false,
        false,
        false,
        false,
        false,
        "reserved for module flash",
    ),
    pin(
        11,
        "GPIO11/FLASH",
        false,
        false,
        false,
        false,
        false,
        false,
        false,
        "reserved for module flash",
    ),
];

pub const ESP32_DEVKIT_V1: TargetDescriptor = TargetDescriptor {
    board_id: "esp32-devkit-v1",
    display_name: "ESP32 DevKit V1 / ESP32-WROOM-32",
    variant: Esp32Variant::DevKitV1,
    module: "ESP32-WROOM-32",
    mcu: "Espressif ESP32",
    core: "Dual-core Xtensa LX6",
    isa: "Xtensa LX6",
    rust_target: "xtensa-esp32-none-elf",
    clock_hz: ESP32_CLOCK_HZ,
    flash_bytes: ESP32_FLASH_BYTES,
    sram_bytes: ESP32_SRAM_BYTES,
    operating_voltage_mv: ESP32_OPERATING_VOLTAGE_MV,
    onboard_led_pin: Some(ESP32_DEVKIT_V1_ONBOARD_LED_PIN),
    supports_wifi: true,
    supports_bluetooth: true,
    capabilities: CapabilitySet::blink_mvp(),
    digital_pins: &ESP32_DEVKIT_V1_DIGITAL_PINS,
};

pub trait Esp32Backend {
    fn configure_gpio(&mut self, pin: u8, mode: GpioMode) -> Result<(), HalError>;
    fn write_gpio(&mut self, pin: u8, level: Level) -> Result<(), HalError>;
    fn read_gpio(&mut self, pin: u8) -> Result<Level, HalError>;
    fn sleep_ms(&mut self, duration_ms: u16) -> Result<(), HalError>;
    fn now_ms(&self) -> u32;
}

pub struct Esp32Board<B>
where
    B: Esp32Backend,
{
    target: &'static TargetDescriptor,
    backend: B,
}

impl<B> Esp32Board<B>
where
    B: Esp32Backend,
{
    pub const fn new(target: &'static TargetDescriptor, backend: B) -> Self {
        Self { target, backend }
    }

    pub fn devkit_v1(backend: B) -> Self {
        Self::new(&ESP32_DEVKIT_V1, backend)
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

    pub fn into_device(self, board_nonce: u32) -> Esp32Device<B> {
        let descriptor = esp32_device_descriptor(self.target, board_nonce);
        BoardVmDevice::new(descriptor, self)
    }
}

impl<B> BoardHal for Esp32Board<B>
where
    B: Esp32Backend,
{
    fn capabilities(&self) -> CapabilitySet {
        self.target.capabilities
    }

    fn gpio_open(&mut self, pin: u16, mode: GpioMode) -> Result<u32, HalError> {
        let pin = normalize_digital_pin(self.target, pin, mode)?;
        self.backend.configure_gpio(pin, mode)?;
        Ok(pin as u32)
    }

    fn gpio_write(&mut self, token: u32, level: Level) -> Result<(), HalError> {
        let descriptor = digital_pin(self.target, token as u16).ok_or(HalError::InvalidPin)?;
        if !descriptor.supports_output {
            return Err(HalError::UnsupportedMode);
        }
        self.backend.write_gpio(descriptor.gpio, level)
    }

    fn gpio_read(&mut self, token: u32) -> Result<Level, HalError> {
        let descriptor = digital_pin(self.target, token as u16).ok_or(HalError::InvalidPin)?;
        if !descriptor.supports_input {
            return Err(HalError::UnsupportedMode);
        }
        self.backend.read_gpio(descriptor.gpio)
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

pub fn digital_pin(
    target: &'static TargetDescriptor,
    pin: u16,
) -> Option<&'static DigitalPinDescriptor> {
    target
        .digital_pins
        .iter()
        .find(|descriptor| descriptor.gpio as u16 == pin)
}

pub fn is_valid_digital_pin(target: &'static TargetDescriptor, pin: u16, mode: GpioMode) -> bool {
    digital_pin(target, pin)
        .map(|descriptor| descriptor.supports_mode(mode))
        .unwrap_or(false)
}

pub fn esp32_device_descriptor(
    target: &'static TargetDescriptor,
    board_nonce: u32,
) -> DeviceDescriptor<'static> {
    DeviceDescriptor {
        board_id: target.board_id,
        runtime_id: ESP32_VM_RUNTIME_ID,
        board_nonce,
        max_frame_payload: DEFAULT_MAX_FRAME_PAYLOAD,
        supports_store_program: false,
        capabilities: &BLINK_MVP_CAPABILITIES,
    }
}

pub fn devkit_v1_device<B>(backend: B, board_nonce: u32) -> Esp32Device<B>
where
    B: Esp32Backend,
{
    Esp32Board::devkit_v1(backend).into_device(board_nonce)
}

const fn pin(
    gpio: u8,
    label: &'static str,
    supports_input: bool,
    supports_output: bool,
    supports_pullup: bool,
    supports_pulldown: bool,
    supports_adc: bool,
    supports_touch: bool,
    boot_strap: bool,
    notes: &'static str,
) -> DigitalPinDescriptor {
    DigitalPinDescriptor {
        gpio,
        label,
        supports_input,
        supports_output,
        supports_pullup,
        supports_pulldown,
        supports_adc,
        supports_touch,
        boot_strap,
        notes,
    }
}

fn normalize_digital_pin(
    target: &'static TargetDescriptor,
    pin: u16,
    mode: GpioMode,
) -> Result<u8, HalError> {
    let descriptor = digital_pin(target, pin).ok_or(HalError::InvalidPin)?;
    if descriptor.supports_mode(mode) {
        Ok(descriptor.gpio)
    } else {
        Err(HalError::UnsupportedMode)
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

    impl Esp32Backend for FakeBackend {
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
    fn descriptor_targets_xtensa_esp32() {
        assert_eq!(ESP32_DEVKIT_V1.core, "Dual-core Xtensa LX6");
        assert_eq!(ESP32_DEVKIT_V1.rust_target, "xtensa-esp32-none-elf");
        assert_eq!(ESP32_DEVKIT_V1.clock_hz, 240_000_000);
        assert_eq!(ESP32_DEVKIT_V1.operating_voltage_mv, 3300);
        assert!(ESP32_DEVKIT_V1.supports_wifi);
    }

    #[test]
    fn knows_common_gpio2_led_pin() {
        let gpio2 = digital_pin(&ESP32_DEVKIT_V1, 2).unwrap();
        assert_eq!(gpio2.label, "GPIO2/LED");
        assert!(gpio2.supports_output);
        assert_eq!(ESP32_DEVKIT_V1.onboard_led_pin, Some(2));
    }

    #[test]
    fn rejects_flash_reserved_and_input_only_output_modes() {
        assert_eq!(
            normalize_digital_pin(&ESP32_DEVKIT_V1, 6, GpioMode::Input),
            Err(HalError::UnsupportedMode)
        );
        assert_eq!(
            normalize_digital_pin(&ESP32_DEVKIT_V1, 34, GpioMode::Output),
            Err(HalError::UnsupportedMode)
        );
        assert_eq!(
            normalize_digital_pin(&ESP32_DEVKIT_V1, 99, GpioMode::Input),
            Err(HalError::InvalidPin)
        );
    }

    #[test]
    fn blink_runs_through_abstract_esp32_backend() {
        let board = Esp32Board::devkit_v1(FakeBackend::new());
        let mut runtime: Runtime<_, 8, 4> = Runtime::new(board);
        let mut module = [0u8; BLINK_MODULE_LEN];
        let len = write_blink_module(
            BlinkProgram {
                pin: ESP32_DEVKIT_V1_ONBOARD_LED_PIN,
                high_ms: 100,
                low_ms: 100,
                max_stack: 4,
            },
            &mut module,
        )
        .unwrap();
        let module = parse_module(&module[..len]).unwrap();
        let report = runtime.run_module(&module, 13).unwrap();

        assert_eq!(report.status, RunStatus::BudgetExceeded);
        assert_eq!(
            runtime.hal().backend().events,
            vec![
                Event::Configure(2, GpioMode::Output),
                Event::Write(2, Level::High),
                Event::Sleep(100),
                Event::Write(2, Level::Low),
                Event::Sleep(100),
            ]
        );
    }

    #[test]
    fn maps_target_metadata_to_device_descriptor() {
        let descriptor = esp32_device_descriptor(&ESP32_DEVKIT_V1, 0xE532_0001);

        assert_eq!(descriptor.board_id, ESP32_DEVKIT_V1.board_id);
        assert_eq!(descriptor.runtime_id, ESP32_VM_RUNTIME_ID);
        assert_eq!(descriptor.board_nonce, 0xE532_0001);
        assert_eq!(descriptor.max_frame_payload, DEFAULT_MAX_FRAME_PAYLOAD);
        assert_eq!(descriptor.capabilities.len(), BLINK_MVP_CAPABILITIES.len());
    }
}
