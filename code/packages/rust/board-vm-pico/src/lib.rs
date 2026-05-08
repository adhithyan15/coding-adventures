#![no_std]

use board_vm_device::{
    BoardVmDevice, DeviceDescriptor, BLINK_MVP_CAPABILITIES, DEFAULT_MAX_FRAME_PAYLOAD,
};
use board_vm_ir::CapabilitySet;
use board_vm_runtime::{BoardHal, GpioMode, HalError, Level};

pub const PICO_CLOCK_HZ: u32 = 125_000_000;
pub const PICO_FLASH_BYTES: u32 = 2 * 1024 * 1024;
pub const PICO_SRAM_BYTES: u32 = 264 * 1024;
pub const PICO_OPERATING_VOLTAGE_MV: u16 = 3300;
pub const PICO_ONBOARD_LED_PIN: u8 = 25;
pub const PICO_W_WIRELESS_LED_GPIO: u8 = 0;
pub const PICO_VM_RUNTIME_ID: &str = "board-vm-pico";
pub const PICO_VM_MAX_PROGRAM_BYTES: usize = 8192;
pub const PICO_VM_MAX_STACK_VALUES: usize = 32;
pub const PICO_VM_MAX_HANDLES: usize = 16;

pub type PicoDevice<B> = BoardVmDevice<
    'static,
    PicoBoard<B>,
    PICO_VM_MAX_PROGRAM_BYTES,
    PICO_VM_MAX_STACK_VALUES,
    PICO_VM_MAX_HANDLES,
>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PicoVariant {
    Pico,
    PicoW,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OnboardLed {
    Gpio(u8),
    WirelessChipGpio(u8),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TargetDescriptor {
    pub board_id: &'static str,
    pub display_name: &'static str,
    pub variant: PicoVariant,
    pub mcu: &'static str,
    pub core: &'static str,
    pub isa: &'static str,
    pub rust_target: &'static str,
    pub clock_hz: u32,
    pub flash_bytes: u32,
    pub sram_bytes: u32,
    pub operating_voltage_mv: u16,
    pub onboard_led: Option<OnboardLed>,
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

pub const PICO_DIGITAL_PINS: [DigitalPinDescriptor; 29] = [
    pin(0, "GP0", false, "UART0 TX / I2C0 SDA capable"),
    pin(1, "GP1", false, "UART0 RX / I2C0 SCL capable"),
    pin(2, "GP2", false, "GPIO / I2C1 SDA capable"),
    pin(3, "GP3", false, "GPIO / I2C1 SCL capable"),
    pin(4, "GP4", false, "GPIO / I2C0 SDA capable"),
    pin(5, "GP5", false, "GPIO / I2C0 SCL capable"),
    pin(6, "GP6", false, "GPIO / I2C1 SDA capable"),
    pin(7, "GP7", false, "GPIO / I2C1 SCL capable"),
    pin(8, "GP8", false, "GPIO / I2C0 SDA capable"),
    pin(9, "GP9", false, "GPIO / I2C0 SCL capable"),
    pin(10, "GP10", false, "GPIO / SPI1 SCK capable"),
    pin(11, "GP11", false, "GPIO / SPI1 TX capable"),
    pin(12, "GP12", false, "GPIO / SPI1 RX capable"),
    pin(13, "GP13", false, "GPIO / SPI1 CS capable"),
    pin(14, "GP14", false, "GPIO"),
    pin(15, "GP15", false, "GPIO"),
    pin(16, "GP16", false, "GPIO / SPI0 RX capable"),
    pin(17, "GP17", false, "GPIO / SPI0 CS capable"),
    pin(18, "GP18", false, "GPIO / SPI0 SCK capable"),
    pin(19, "GP19", false, "GPIO / SPI0 TX capable"),
    pin(20, "GP20", false, "GPIO"),
    pin(21, "GP21", false, "GPIO"),
    pin(22, "GP22", false, "GPIO"),
    pin(
        23,
        "GP23",
        false,
        "board-internal power-save control on Pico",
    ),
    pin(24, "GP24", false, "board-internal VBUS sense on Pico"),
    pin(25, "GP25/LED", false, "standard Pico onboard LED"),
    pin(26, "GP26/ADC0", true, "ADC0 capable"),
    pin(27, "GP27/ADC1", true, "ADC1 capable"),
    pin(28, "GP28/ADC2", true, "ADC2 capable"),
];

pub const PICO: TargetDescriptor = TargetDescriptor {
    board_id: "raspberry-pi-pico",
    display_name: "Raspberry Pi Pico",
    variant: PicoVariant::Pico,
    mcu: "RP2040",
    core: "Dual-core Arm Cortex-M0+",
    isa: "Armv6-M Thumb",
    rust_target: "thumbv6m-none-eabi",
    clock_hz: PICO_CLOCK_HZ,
    flash_bytes: PICO_FLASH_BYTES,
    sram_bytes: PICO_SRAM_BYTES,
    operating_voltage_mv: PICO_OPERATING_VOLTAGE_MV,
    onboard_led: Some(OnboardLed::Gpio(PICO_ONBOARD_LED_PIN)),
    supports_wifi: false,
    supports_bluetooth: false,
    capabilities: CapabilitySet::blink_mvp(),
    digital_pins: &PICO_DIGITAL_PINS,
};

pub const PICO_W: TargetDescriptor = TargetDescriptor {
    board_id: "raspberry-pi-pico-w",
    display_name: "Raspberry Pi Pico W",
    variant: PicoVariant::PicoW,
    mcu: "RP2040",
    core: "Dual-core Arm Cortex-M0+",
    isa: "Armv6-M Thumb",
    rust_target: "thumbv6m-none-eabi",
    clock_hz: PICO_CLOCK_HZ,
    flash_bytes: PICO_FLASH_BYTES,
    sram_bytes: PICO_SRAM_BYTES,
    operating_voltage_mv: PICO_OPERATING_VOLTAGE_MV,
    onboard_led: Some(OnboardLed::WirelessChipGpio(PICO_W_WIRELESS_LED_GPIO)),
    supports_wifi: true,
    supports_bluetooth: true,
    capabilities: CapabilitySet::blink_mvp(),
    digital_pins: &PICO_DIGITAL_PINS,
};

pub trait PicoBackend {
    fn configure_gpio(&mut self, pin: u8, mode: GpioMode) -> Result<(), HalError>;
    fn write_gpio(&mut self, pin: u8, level: Level) -> Result<(), HalError>;
    fn read_gpio(&mut self, pin: u8) -> Result<Level, HalError>;
    fn sleep_ms(&mut self, duration_ms: u16) -> Result<(), HalError>;
    fn now_ms(&self) -> u32;
}

pub struct PicoBoard<B>
where
    B: PicoBackend,
{
    target: &'static TargetDescriptor,
    backend: B,
}

impl<B> PicoBoard<B>
where
    B: PicoBackend,
{
    pub const fn new(target: &'static TargetDescriptor, backend: B) -> Self {
        Self { target, backend }
    }

    pub fn pico(backend: B) -> Self {
        Self::new(&PICO, backend)
    }

    pub fn pico_w(backend: B) -> Self {
        Self::new(&PICO_W, backend)
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

    pub fn into_device(self, board_nonce: u32) -> PicoDevice<B> {
        let descriptor = pico_device_descriptor(self.target, board_nonce);
        BoardVmDevice::new(descriptor, self)
    }
}

impl<B> BoardHal for PicoBoard<B>
where
    B: PicoBackend,
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

pub fn pico_device_descriptor(
    target: &'static TargetDescriptor,
    board_nonce: u32,
) -> DeviceDescriptor<'static> {
    DeviceDescriptor {
        board_id: target.board_id,
        runtime_id: PICO_VM_RUNTIME_ID,
        board_nonce,
        max_frame_payload: DEFAULT_MAX_FRAME_PAYLOAD,
        supports_store_program: false,
        capabilities: &BLINK_MVP_CAPABILITIES,
    }
}

pub fn pico_device<B>(backend: B, board_nonce: u32) -> PicoDevice<B>
where
    B: PicoBackend,
{
    PicoBoard::pico(backend).into_device(board_nonce)
}

pub fn pico_w_device<B>(backend: B, board_nonce: u32) -> PicoDevice<B>
where
    B: PicoBackend,
{
    PicoBoard::pico_w(backend).into_device(board_nonce)
}

const fn pin(
    gpio: u8,
    label: &'static str,
    supports_adc: bool,
    notes: &'static str,
) -> DigitalPinDescriptor {
    DigitalPinDescriptor {
        gpio,
        label,
        supports_input: true,
        supports_output: true,
        supports_pullup: true,
        supports_pulldown: true,
        supports_adc,
        supports_pwm: true,
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

    impl PicoBackend for FakeBackend {
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
    fn descriptor_targets_rp2040() {
        assert_eq!(PICO.mcu, "RP2040");
        assert_eq!(PICO.core, "Dual-core Arm Cortex-M0+");
        assert_eq!(PICO.rust_target, "thumbv6m-none-eabi");
        assert_eq!(PICO.clock_hz, 125_000_000);
        assert_eq!(PICO.operating_voltage_mv, 3300);
    }

    #[test]
    fn distinguishes_pico_and_pico_w_led_routes() {
        assert_eq!(PICO.onboard_led, Some(OnboardLed::Gpio(25)));
        assert_eq!(PICO_W.onboard_led, Some(OnboardLed::WirelessChipGpio(0)));
        assert!(PICO_W.supports_wifi);
    }

    #[test]
    fn knows_gp25_standard_pico_led_pin() {
        let gp25 = digital_pin(&PICO, 25).unwrap();
        assert_eq!(gp25.label, "GP25/LED");
        assert!(gp25.supports_output);
    }

    #[test]
    fn rejects_non_pico_gpio() {
        assert_eq!(
            normalize_digital_pin(&PICO, 29, GpioMode::Input),
            Err(HalError::InvalidPin)
        );
    }

    #[test]
    fn blink_runs_through_abstract_pico_backend() {
        let board = PicoBoard::pico(FakeBackend::new());
        let mut runtime: Runtime<_, 8, 4> = Runtime::new(board);
        let mut module = [0u8; BLINK_MODULE_LEN];
        let len = write_blink_module(
            BlinkProgram {
                pin: PICO_ONBOARD_LED_PIN,
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
                Event::Configure(25, GpioMode::Output),
                Event::Write(25, Level::High),
                Event::Sleep(100),
                Event::Write(25, Level::Low),
                Event::Sleep(100),
            ]
        );
    }

    #[test]
    fn maps_target_metadata_to_device_descriptor() {
        let descriptor = pico_device_descriptor(&PICO_W, 0xC0DE_2040);

        assert_eq!(descriptor.board_id, PICO_W.board_id);
        assert_eq!(descriptor.runtime_id, PICO_VM_RUNTIME_ID);
        assert_eq!(descriptor.board_nonce, 0xC0DE_2040);
        assert_eq!(descriptor.max_frame_payload, DEFAULT_MAX_FRAME_PAYLOAD);
        assert_eq!(descriptor.capabilities.len(), BLINK_MVP_CAPABILITIES.len());
    }
}
