#![no_std]

use board_vm_device::{
    BoardVmDevice, DeviceDescriptor, BLINK_MVP_CAPABILITIES,
    BLINK_MVP_WITH_ADC_AND_DAC_CAPABILITIES, BLINK_MVP_WITH_ADC_AND_LED_MATRIX_CAPABILITIES,
    BLINK_MVP_WITH_ADC_CAPABILITIES, BLINK_MVP_WITH_ADC_DAC_AND_LED_MATRIX_CAPABILITIES,
    BLINK_MVP_WITH_DAC_AND_LED_MATRIX_CAPABILITIES, BLINK_MVP_WITH_DAC_CAPABILITIES,
    BLINK_MVP_WITH_LED_MATRIX_CAPABILITIES, BLINK_MVP_WITH_PWM_ADC_AND_DAC_CAPABILITIES,
    BLINK_MVP_WITH_PWM_ADC_AND_LED_MATRIX_CAPABILITIES,
    BLINK_MVP_WITH_PWM_ADC_DAC_AND_I2C_CAPABILITIES,
    BLINK_MVP_WITH_PWM_ADC_DAC_AND_LED_MATRIX_CAPABILITIES,
    BLINK_MVP_WITH_PWM_ADC_DAC_I2C_AND_LED_MATRIX_CAPABILITIES,
    BLINK_MVP_WITH_PWM_ADC_DAC_I2C_AND_SPI_CAPABILITIES,
    BLINK_MVP_WITH_PWM_ADC_DAC_I2C_SPI_AND_LED_MATRIX_CAPABILITIES,
    BLINK_MVP_WITH_PWM_ADC_DAC_I2C_SPI_AND_UART_CAPABILITIES,
    BLINK_MVP_WITH_PWM_ADC_DAC_I2C_SPI_UART_AND_LED_MATRIX_CAPABILITIES,
    BLINK_MVP_WITH_PWM_AND_ADC_CAPABILITIES, BLINK_MVP_WITH_PWM_AND_DAC_CAPABILITIES,
    BLINK_MVP_WITH_PWM_AND_LED_MATRIX_CAPABILITIES, BLINK_MVP_WITH_PWM_CAPABILITIES,
    BLINK_MVP_WITH_PWM_DAC_AND_LED_MATRIX_CAPABILITIES, DEFAULT_MAX_FRAME_PAYLOAD,
};
use board_vm_ir::CapabilitySet;
use board_vm_runtime::{BoardHal, ByteBuffer, GpioMode, HalError, Level};

pub const UNO_R4_CLOCK_HZ: u32 = 48_000_000;
pub const UNO_R4_FLASH_BYTES: u32 = 256 * 1024;
pub const UNO_R4_SRAM_BYTES: u32 = 32 * 1024;
pub const UNO_R4_DATA_FLASH_BYTES: u32 = 8 * 1024;
pub const UNO_R4_ONBOARD_LED_PIN: u8 = 13;
pub const UNO_R4_VM_RUNTIME_ID: &str = "board-vm-uno-r4";
pub const UNO_R4_VM_MAX_PROGRAM_BYTES: usize = 4096;
pub const UNO_R4_VM_MAX_STACK_VALUES: usize = 16;
pub const UNO_R4_VM_MAX_HANDLES: usize = 8;

pub type UnoR4Device<B> = BoardVmDevice<
    'static,
    UnoR4Board<B>,
    UNO_R4_VM_MAX_PROGRAM_BYTES,
    UNO_R4_VM_MAX_STACK_VALUES,
    UNO_R4_VM_MAX_HANDLES,
>;

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
    pub i2c_buses: &'static [I2cBusDescriptor],
    pub spi_buses: &'static [SpiBusDescriptor],
    pub uart_buses: &'static [UartBusDescriptor],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DigitalPinDescriptor {
    pub arduino_pin: u8,
    pub label: &'static str,
    pub supports_pwm: bool,
    pub supports_adc: bool,
    pub supports_dac: bool,
    pub supports_interrupt: bool,
    pub notes: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct I2cBusDescriptor {
    pub bus: u8,
    pub name: &'static str,
    pub sda_pin: u8,
    pub scl_pin: u8,
    pub qwiic: bool,
    pub notes: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpiBusDescriptor {
    pub bus: u8,
    pub name: &'static str,
    pub copi_pin: u8,
    pub cipo_pin: u8,
    pub sck_pin: u8,
    pub default_cs_pin: u8,
    pub notes: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UartBusDescriptor {
    pub bus: u8,
    pub name: &'static str,
    pub tx_pin: u8,
    pub rx_pin: u8,
    pub arduino_uart: u8,
    pub internal: bool,
    pub notes: &'static str,
}

pub const UNO_R4_DIGITAL_PINS: [DigitalPinDescriptor; 20] = [
    DigitalPinDescriptor {
        arduino_pin: 0,
        label: "D0/RX0",
        supports_pwm: false,
        supports_adc: false,
        supports_dac: false,
        supports_interrupt: false,
        notes: "GPIO 0 / Serial 0 receiver",
    },
    DigitalPinDescriptor {
        arduino_pin: 1,
        label: "D1/TX0",
        supports_pwm: false,
        supports_adc: false,
        supports_dac: false,
        supports_interrupt: false,
        notes: "GPIO 1 / Serial 0 transmitter",
    },
    DigitalPinDescriptor {
        arduino_pin: 2,
        label: "D2",
        supports_pwm: false,
        supports_adc: false,
        supports_dac: false,
        supports_interrupt: true,
        notes: "GPIO 2 / external interrupt",
    },
    DigitalPinDescriptor {
        arduino_pin: 3,
        label: "D3",
        supports_pwm: true,
        supports_adc: false,
        supports_dac: false,
        supports_interrupt: true,
        notes: "GPIO 3 / PWM / external interrupt",
    },
    DigitalPinDescriptor {
        arduino_pin: 4,
        label: "D4",
        supports_pwm: false,
        supports_adc: false,
        supports_dac: false,
        supports_interrupt: false,
        notes: "GPIO 4 / CAN alternate function on Minima header docs",
    },
    DigitalPinDescriptor {
        arduino_pin: 5,
        label: "D5",
        supports_pwm: true,
        supports_adc: false,
        supports_dac: false,
        supports_interrupt: false,
        notes: "GPIO 5 / PWM",
    },
    DigitalPinDescriptor {
        arduino_pin: 6,
        label: "D6",
        supports_pwm: true,
        supports_adc: false,
        supports_dac: false,
        supports_interrupt: false,
        notes: "GPIO 6 / PWM",
    },
    DigitalPinDescriptor {
        arduino_pin: 7,
        label: "D7",
        supports_pwm: false,
        supports_adc: false,
        supports_dac: false,
        supports_interrupt: false,
        notes: "GPIO 7",
    },
    DigitalPinDescriptor {
        arduino_pin: 8,
        label: "D8",
        supports_pwm: false,
        supports_adc: false,
        supports_dac: false,
        supports_interrupt: false,
        notes: "GPIO 8",
    },
    DigitalPinDescriptor {
        arduino_pin: 9,
        label: "D9",
        supports_pwm: true,
        supports_adc: false,
        supports_dac: false,
        supports_interrupt: false,
        notes: "GPIO 9 / PWM",
    },
    DigitalPinDescriptor {
        arduino_pin: 10,
        label: "D10/CS",
        supports_pwm: true,
        supports_adc: false,
        supports_dac: false,
        supports_interrupt: false,
        notes: "GPIO 10 / PWM / SPI chip select",
    },
    DigitalPinDescriptor {
        arduino_pin: 11,
        label: "D11/COPI",
        supports_pwm: true,
        supports_adc: false,
        supports_dac: false,
        supports_interrupt: false,
        notes: "GPIO 11 / PWM / SPI controller out",
    },
    DigitalPinDescriptor {
        arduino_pin: 12,
        label: "D12/CIPO",
        supports_pwm: false,
        supports_adc: false,
        supports_dac: false,
        supports_interrupt: false,
        notes: "GPIO 12 / SPI controller in",
    },
    DigitalPinDescriptor {
        arduino_pin: 13,
        label: "D13/SCK",
        supports_pwm: false,
        supports_adc: false,
        supports_dac: false,
        supports_interrupt: false,
        notes: "GPIO 13 / SPI clock / onboard LED",
    },
    DigitalPinDescriptor {
        arduino_pin: 14,
        label: "A0/D14",
        supports_pwm: false,
        supports_adc: true,
        supports_dac: true,
        supports_interrupt: false,
        notes: "Analog input A0 / DAC output / digital pin 14",
    },
    DigitalPinDescriptor {
        arduino_pin: 15,
        label: "A1/D15",
        supports_pwm: false,
        supports_adc: true,
        supports_dac: false,
        supports_interrupt: false,
        notes: "Analog input A1 / digital pin 15",
    },
    DigitalPinDescriptor {
        arduino_pin: 16,
        label: "A2/D16",
        supports_pwm: false,
        supports_adc: true,
        supports_dac: false,
        supports_interrupt: false,
        notes: "Analog input A2 / digital pin 16",
    },
    DigitalPinDescriptor {
        arduino_pin: 17,
        label: "A3/D17",
        supports_pwm: false,
        supports_adc: true,
        supports_dac: false,
        supports_interrupt: false,
        notes: "Analog input A3 / digital pin 17",
    },
    DigitalPinDescriptor {
        arduino_pin: 18,
        label: "A4/SDA/D18",
        supports_pwm: false,
        supports_adc: true,
        supports_dac: false,
        supports_interrupt: false,
        notes: "Analog input A4 / I2C SDA / digital pin 18",
    },
    DigitalPinDescriptor {
        arduino_pin: 19,
        label: "A5/SCL/D19",
        supports_pwm: false,
        supports_adc: true,
        supports_dac: false,
        supports_interrupt: false,
        notes: "Analog input A5 / I2C SCL / digital pin 19",
    },
];

pub const UNO_R4_HEADER_I2C_BUS: I2cBusDescriptor = I2cBusDescriptor {
    bus: 0,
    name: "Wire",
    sda_pin: 18,
    scl_pin: 19,
    qwiic: false,
    notes: "Header I2C bus on A4/SDA and A5/SCL",
};

pub const UNO_R4_QWIIC_I2C_BUS: I2cBusDescriptor = I2cBusDescriptor {
    bus: 1,
    name: "Wire1",
    sda_pin: 27,
    scl_pin: 26,
    qwiic: true,
    notes: "UNO R4 WiFi Qwiic I2C bus",
};

pub const UNO_R4_MINIMA_I2C_BUSES: [I2cBusDescriptor; 1] = [UNO_R4_HEADER_I2C_BUS];
pub const UNO_R4_WIFI_I2C_BUSES: [I2cBusDescriptor; 2] =
    [UNO_R4_HEADER_I2C_BUS, UNO_R4_QWIIC_I2C_BUS];

pub const UNO_R4_HEADER_SPI_BUS: SpiBusDescriptor = SpiBusDescriptor {
    bus: 0,
    name: "SPI",
    copi_pin: 11,
    cipo_pin: 12,
    sck_pin: 13,
    default_cs_pin: 10,
    notes:
        "Header SPI bus on D11/COPI, D12/CIPO, and D13/SCK with D10 as the conventional chip select",
};

pub const UNO_R4_SPI_BUSES: [SpiBusDescriptor; 1] = [UNO_R4_HEADER_SPI_BUS];

pub const UNO_R4_MINIMA_HEADER_UART_BUS: UartBusDescriptor = UartBusDescriptor {
    bus: 0,
    name: "Serial1",
    tx_pin: 1,
    rx_pin: 0,
    arduino_uart: 1,
    internal: false,
    notes: "Header UART on D1/TX0 and D0/RX0",
};

pub const UNO_R4_WIFI_D22_D23_UART_BUS: UartBusDescriptor = UartBusDescriptor {
    bus: 0,
    name: "Serial1",
    tx_pin: 22,
    rx_pin: 23,
    arduino_uart: 1,
    internal: false,
    notes: "UNO R4 WiFi UART on D22/TX and D23/RX",
};

pub const UNO_R4_WIFI_HEADER_UART_BUS: UartBusDescriptor = UartBusDescriptor {
    bus: 1,
    name: "Serial2",
    tx_pin: 1,
    rx_pin: 0,
    arduino_uart: 2,
    internal: false,
    notes: "Header UART on D1/TX0 and D0/RX0",
};

pub const UNO_R4_WIFI_MODULE_UART_BUS: UartBusDescriptor = UartBusDescriptor {
    bus: 2,
    name: "Serial3",
    tx_pin: 24,
    rx_pin: 25,
    arduino_uart: 3,
    internal: true,
    notes: "UNO R4 WiFi module UART on D24/TX WIFI and D25/RX WIFI",
};

pub const UNO_R4_MINIMA_UART_BUSES: [UartBusDescriptor; 1] = [UNO_R4_MINIMA_HEADER_UART_BUS];
pub const UNO_R4_WIFI_UART_BUSES: [UartBusDescriptor; 3] = [
    UNO_R4_WIFI_D22_D23_UART_BUS,
    UNO_R4_WIFI_HEADER_UART_BUS,
    UNO_R4_WIFI_MODULE_UART_BUS,
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
    capabilities: CapabilitySet::blink_mvp()
        .with_pwm()
        .with_adc()
        .with_dac()
        .with_i2c()
        .with_uart()
        .with_spi(),
    digital_pins: &UNO_R4_DIGITAL_PINS,
    i2c_buses: &UNO_R4_MINIMA_I2C_BUSES,
    spi_buses: &UNO_R4_SPI_BUSES,
    uart_buses: &UNO_R4_MINIMA_UART_BUSES,
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
    capabilities: CapabilitySet::blink_mvp()
        .with_pwm()
        .with_adc()
        .with_dac()
        .with_i2c()
        .with_spi()
        .with_uart()
        .with_led_matrix(),
    digital_pins: &UNO_R4_DIGITAL_PINS,
    i2c_buses: &UNO_R4_WIFI_I2C_BUSES,
    spi_buses: &UNO_R4_SPI_BUSES,
    uart_buses: &UNO_R4_WIFI_UART_BUSES,
};

pub trait UnoR4Backend {
    fn configure_gpio(&mut self, pin: u8, mode: GpioMode) -> Result<(), HalError>;
    fn write_gpio(&mut self, pin: u8, level: Level) -> Result<(), HalError>;
    fn read_gpio(&mut self, pin: u8) -> Result<Level, HalError>;
    fn sleep_ms(&mut self, duration_ms: u16) -> Result<(), HalError>;
    fn now_ms(&self) -> u32;

    fn supports_pwm(&self) -> bool {
        false
    }

    fn supports_adc(&self) -> bool {
        false
    }

    fn supports_dac(&self) -> bool {
        false
    }

    fn supports_i2c(&self) -> bool {
        false
    }

    fn supports_spi(&self) -> bool {
        false
    }

    fn supports_uart(&self) -> bool {
        false
    }

    fn led_matrix_frame(&mut self, _frame: [u32; 3]) -> Result<(), HalError> {
        Err(HalError::UnsupportedMode)
    }

    fn write_pwm(&mut self, _pin: u8, _duty: u16) -> Result<(), HalError> {
        Err(HalError::UnsupportedMode)
    }

    fn read_adc(&mut self, _pin: u8) -> Result<u16, HalError> {
        Err(HalError::UnsupportedMode)
    }

    fn write_dac_u12(&mut self, _pin: u8, _sample: u16) -> Result<(), HalError> {
        Err(HalError::UnsupportedMode)
    }

    fn open_i2c(&mut self, _bus: u8) -> Result<u32, HalError> {
        Err(HalError::UnsupportedMode)
    }

    fn open_spi(&mut self, _bus: u8) -> Result<u32, HalError> {
        Err(HalError::UnsupportedMode)
    }

    fn open_uart(&mut self, _bus: u8) -> Result<u32, HalError> {
        Err(HalError::UnsupportedMode)
    }

    fn transfer_spi(
        &mut self,
        _bus: u8,
        _cs_pin: u8,
        _write_bytes: &[u8],
        _read_len: u8,
    ) -> Result<ByteBuffer, HalError> {
        Err(HalError::UnsupportedMode)
    }

    fn write_i2c_u8(&mut self, _bus: u8, _address: u16, _byte: u8) -> Result<(), HalError> {
        Err(HalError::UnsupportedMode)
    }

    fn write_i2c(&mut self, _bus: u8, _address: u16, _bytes: &[u8]) -> Result<(), HalError> {
        Err(HalError::UnsupportedMode)
    }

    fn read_i2c_u8(&mut self, _bus: u8, _address: u16) -> Result<u8, HalError> {
        Err(HalError::UnsupportedMode)
    }

    fn read_i2c(&mut self, _bus: u8, _address: u16, _len: u8) -> Result<ByteBuffer, HalError> {
        Err(HalError::UnsupportedMode)
    }

    fn transfer_i2c(
        &mut self,
        _bus: u8,
        _address: u16,
        _write_bytes: &[u8],
        _read_len: u8,
    ) -> Result<ByteBuffer, HalError> {
        Err(HalError::UnsupportedMode)
    }

    fn supports_bootloader_reboot(&self) -> bool {
        false
    }

    fn reboot_to_bootloader(&mut self) -> Result<(), HalError> {
        Err(HalError::UnsupportedMode)
    }
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

    pub fn into_device(self, board_nonce: u32) -> UnoR4Device<B> {
        let capabilities = self.resolved_capabilities();
        let descriptor =
            uno_r4_device_descriptor_for_capabilities(self.target, board_nonce, capabilities);
        BoardVmDevice::new(descriptor, self)
    }

    fn resolved_capabilities(&self) -> CapabilitySet {
        let mut capabilities = self.target.capabilities;
        capabilities.pwm = self.backend.supports_pwm();
        capabilities.adc = self.backend.supports_adc();
        capabilities.dac = self.backend.supports_dac();
        capabilities.i2c = self.backend.supports_i2c();
        capabilities.spi = self.backend.supports_spi();
        capabilities.uart = self.backend.supports_uart();
        capabilities
    }
}

impl<B> BoardHal for UnoR4Board<B>
where
    B: UnoR4Backend,
{
    fn capabilities(&self) -> CapabilitySet {
        self.resolved_capabilities()
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

    fn pwm_write(&mut self, pin: u16, duty: u16) -> Result<(), HalError> {
        let pin = normalize_pwm_pin(pin)?;
        self.backend.write_pwm(pin, duty)
    }

    fn adc_read(&mut self, pin: u16) -> Result<u16, HalError> {
        let pin = normalize_adc_pin(pin)?;
        self.backend.read_adc(pin)
    }

    fn dac_write_u12(&mut self, pin: u16, sample: u16) -> Result<(), HalError> {
        if sample > 0x0fff {
            return Err(HalError::UnsupportedMode);
        }
        let pin = normalize_dac_pin(pin)?;
        self.backend.write_dac_u12(pin, sample)
    }

    fn i2c_open(&mut self, bus: u16) -> Result<u32, HalError> {
        let bus = normalize_i2c_bus(self.target, bus)?;
        self.backend.open_i2c(bus)
    }

    fn spi_open(&mut self, bus: u16) -> Result<u32, HalError> {
        let bus = normalize_spi_bus(self.target, bus)?;
        self.backend.open_spi(bus)
    }

    fn uart_open(&mut self, bus: u16) -> Result<u32, HalError> {
        let bus = normalize_uart_bus(self.target, bus)?;
        self.backend.open_uart(bus)
    }

    fn spi_transfer(
        &mut self,
        token: u32,
        cs_pin: u16,
        write_bytes: &[u8],
        read_len: u8,
    ) -> Result<ByteBuffer, HalError> {
        let bus = normalize_spi_token(self.target, token)?;
        let cs_pin = normalize_digital_pin(cs_pin)?;
        self.backend
            .transfer_spi(bus, cs_pin, write_bytes, read_len)
    }

    fn i2c_write_u8(&mut self, token: u32, address: u16, byte: u8) -> Result<(), HalError> {
        let bus = normalize_i2c_token(self.target, token)?;
        normalize_i2c_address(address)?;
        self.backend.write_i2c_u8(bus, address, byte)
    }

    fn i2c_write(&mut self, token: u32, address: u16, bytes: &[u8]) -> Result<(), HalError> {
        let bus = normalize_i2c_token(self.target, token)?;
        normalize_i2c_address(address)?;
        self.backend.write_i2c(bus, address, bytes)
    }

    fn i2c_read_u8(&mut self, token: u32, address: u16) -> Result<u8, HalError> {
        let bus = normalize_i2c_token(self.target, token)?;
        normalize_i2c_address(address)?;
        self.backend.read_i2c_u8(bus, address)
    }

    fn i2c_read(&mut self, token: u32, address: u16, len: u8) -> Result<ByteBuffer, HalError> {
        let bus = normalize_i2c_token(self.target, token)?;
        normalize_i2c_address(address)?;
        self.backend.read_i2c(bus, address, len)
    }

    fn i2c_transfer(
        &mut self,
        token: u32,
        address: u16,
        write_bytes: &[u8],
        read_len: u8,
    ) -> Result<ByteBuffer, HalError> {
        let bus = normalize_i2c_token(self.target, token)?;
        normalize_i2c_address(address)?;
        self.backend
            .transfer_i2c(bus, address, write_bytes, read_len)
    }

    fn led_matrix_frame(&mut self, frame: [u32; 3]) -> Result<(), HalError> {
        if !self.target.supports_led_matrix {
            return Err(HalError::UnsupportedMode);
        }
        self.backend.led_matrix_frame(frame)
    }

    fn supports_bootloader_reboot(&self) -> bool {
        self.backend.supports_bootloader_reboot()
    }

    fn reboot_to_bootloader(&mut self) -> Result<(), HalError> {
        self.backend.reboot_to_bootloader()
    }
}

pub fn digital_pin(pin: u8) -> Option<&'static DigitalPinDescriptor> {
    UNO_R4_DIGITAL_PINS
        .iter()
        .find(|descriptor| descriptor.arduino_pin == pin)
}

pub fn is_valid_digital_pin(pin: u16) -> bool {
    u8::try_from(pin).ok().and_then(digital_pin).is_some()
}

fn normalize_pwm_pin(pin: u16) -> Result<u8, HalError> {
    let pin = normalize_digital_pin(pin)?;
    match digital_pin(pin) {
        Some(descriptor) if descriptor.supports_pwm => Ok(pin),
        Some(_) => Err(HalError::UnsupportedMode),
        None => Err(HalError::InvalidPin),
    }
}

fn normalize_adc_pin(pin: u16) -> Result<u8, HalError> {
    let pin = normalize_digital_pin(pin)?;
    match digital_pin(pin) {
        Some(descriptor) if descriptor.supports_adc => Ok(pin),
        Some(_) => Err(HalError::UnsupportedMode),
        None => Err(HalError::InvalidPin),
    }
}

fn normalize_dac_pin(pin: u16) -> Result<u8, HalError> {
    let pin = normalize_digital_pin(pin)?;
    match digital_pin(pin) {
        Some(descriptor) if descriptor.supports_dac => Ok(pin),
        Some(_) => Err(HalError::UnsupportedMode),
        None => Err(HalError::InvalidPin),
    }
}

fn normalize_i2c_bus(target: &TargetDescriptor, bus: u16) -> Result<u8, HalError> {
    let bus = u8::try_from(bus).map_err(|_| HalError::InvalidPin)?;
    if target
        .i2c_buses
        .iter()
        .any(|descriptor| descriptor.bus == bus)
    {
        Ok(bus)
    } else {
        Err(HalError::InvalidPin)
    }
}

fn normalize_i2c_token(target: &TargetDescriptor, token: u32) -> Result<u8, HalError> {
    let bus = u16::try_from(token & 0xff).map_err(|_| HalError::InvalidPin)?;
    normalize_i2c_bus(target, bus)
}

fn normalize_spi_bus(target: &TargetDescriptor, bus: u16) -> Result<u8, HalError> {
    let bus = u8::try_from(bus).map_err(|_| HalError::InvalidPin)?;
    if target
        .spi_buses
        .iter()
        .any(|descriptor| descriptor.bus == bus)
    {
        Ok(bus)
    } else {
        Err(HalError::InvalidPin)
    }
}

fn normalize_spi_token(target: &TargetDescriptor, token: u32) -> Result<u8, HalError> {
    let bus = u16::try_from(token & 0xff).map_err(|_| HalError::InvalidPin)?;
    normalize_spi_bus(target, bus)
}

fn normalize_uart_bus(target: &TargetDescriptor, bus: u16) -> Result<u8, HalError> {
    let bus = u8::try_from(bus).map_err(|_| HalError::InvalidPin)?;
    if target
        .uart_buses
        .iter()
        .any(|descriptor| descriptor.bus == bus)
    {
        Ok(bus)
    } else {
        Err(HalError::InvalidPin)
    }
}

fn normalize_i2c_address(address: u16) -> Result<(), HalError> {
    if address <= 0x7f {
        Ok(())
    } else {
        Err(HalError::InvalidPin)
    }
}

pub fn uno_r4_device_descriptor(
    target: &'static TargetDescriptor,
    board_nonce: u32,
) -> DeviceDescriptor<'static> {
    uno_r4_device_descriptor_for_capabilities(target, board_nonce, target.capabilities)
}

fn uno_r4_device_descriptor_for_capabilities(
    target: &'static TargetDescriptor,
    board_nonce: u32,
    capabilities: CapabilitySet,
) -> DeviceDescriptor<'static> {
    DeviceDescriptor {
        board_id: target.board_id,
        runtime_id: UNO_R4_VM_RUNTIME_ID,
        board_nonce,
        max_frame_payload: DEFAULT_MAX_FRAME_PAYLOAD,
        supports_store_program: false,
        capabilities: if target.supports_led_matrix
            && capabilities.pwm
            && capabilities.adc
            && capabilities.dac
            && capabilities.i2c
            && capabilities.spi
            && capabilities.uart
        {
            &BLINK_MVP_WITH_PWM_ADC_DAC_I2C_SPI_UART_AND_LED_MATRIX_CAPABILITIES
        } else if target.supports_led_matrix
            && capabilities.pwm
            && capabilities.adc
            && capabilities.dac
            && capabilities.i2c
            && capabilities.spi
        {
            &BLINK_MVP_WITH_PWM_ADC_DAC_I2C_SPI_AND_LED_MATRIX_CAPABILITIES
        } else if target.supports_led_matrix
            && capabilities.pwm
            && capabilities.adc
            && capabilities.dac
            && capabilities.i2c
        {
            &BLINK_MVP_WITH_PWM_ADC_DAC_I2C_AND_LED_MATRIX_CAPABILITIES
        } else if target.supports_led_matrix
            && capabilities.pwm
            && capabilities.adc
            && capabilities.dac
        {
            &BLINK_MVP_WITH_PWM_ADC_DAC_AND_LED_MATRIX_CAPABILITIES
        } else if target.supports_led_matrix && capabilities.pwm && capabilities.dac {
            &BLINK_MVP_WITH_PWM_DAC_AND_LED_MATRIX_CAPABILITIES
        } else if target.supports_led_matrix && capabilities.adc && capabilities.dac {
            &BLINK_MVP_WITH_ADC_DAC_AND_LED_MATRIX_CAPABILITIES
        } else if target.supports_led_matrix && capabilities.dac {
            &BLINK_MVP_WITH_DAC_AND_LED_MATRIX_CAPABILITIES
        } else if target.supports_led_matrix && capabilities.pwm && capabilities.adc {
            &BLINK_MVP_WITH_PWM_ADC_AND_LED_MATRIX_CAPABILITIES
        } else if target.supports_led_matrix && capabilities.pwm {
            &BLINK_MVP_WITH_PWM_AND_LED_MATRIX_CAPABILITIES
        } else if target.supports_led_matrix && capabilities.adc {
            &BLINK_MVP_WITH_ADC_AND_LED_MATRIX_CAPABILITIES
        } else if target.supports_led_matrix {
            &BLINK_MVP_WITH_LED_MATRIX_CAPABILITIES
        } else if capabilities.pwm
            && capabilities.adc
            && capabilities.dac
            && capabilities.i2c
            && capabilities.spi
            && capabilities.uart
        {
            &BLINK_MVP_WITH_PWM_ADC_DAC_I2C_SPI_AND_UART_CAPABILITIES
        } else if capabilities.pwm
            && capabilities.adc
            && capabilities.dac
            && capabilities.i2c
            && capabilities.spi
        {
            &BLINK_MVP_WITH_PWM_ADC_DAC_I2C_AND_SPI_CAPABILITIES
        } else if capabilities.pwm && capabilities.adc && capabilities.dac && capabilities.i2c {
            &BLINK_MVP_WITH_PWM_ADC_DAC_AND_I2C_CAPABILITIES
        } else if capabilities.pwm && capabilities.adc && capabilities.dac {
            &BLINK_MVP_WITH_PWM_ADC_AND_DAC_CAPABILITIES
        } else if capabilities.pwm && capabilities.dac {
            &BLINK_MVP_WITH_PWM_AND_DAC_CAPABILITIES
        } else if capabilities.adc && capabilities.dac {
            &BLINK_MVP_WITH_ADC_AND_DAC_CAPABILITIES
        } else if capabilities.dac {
            &BLINK_MVP_WITH_DAC_CAPABILITIES
        } else if capabilities.pwm && capabilities.adc {
            &BLINK_MVP_WITH_PWM_AND_ADC_CAPABILITIES
        } else if capabilities.pwm {
            &BLINK_MVP_WITH_PWM_CAPABILITIES
        } else if capabilities.adc {
            &BLINK_MVP_WITH_ADC_CAPABILITIES
        } else {
            &BLINK_MVP_CAPABILITIES
        },
    }
}

pub fn minima_device<B>(backend: B, board_nonce: u32) -> UnoR4Device<B>
where
    B: UnoR4Backend,
{
    UnoR4Board::minima(backend).into_device(board_nonce)
}

pub fn wifi_device<B>(backend: B, board_nonce: u32) -> UnoR4Device<B>
where
    B: UnoR4Backend,
{
    UnoR4Board::wifi(backend).into_device(board_nonce)
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
    use board_vm_host::{write_blink_module, BlinkProgram, HostSession, DEFAULT_PROGRAM_ID};
    use board_vm_ir::MAX_BYTE_BUFFER_LEN;
    use board_vm_protocol::{
        decode_caps_report_header, decode_frame, decode_hello_ack, decode_run_report_header,
        MessageType, RunStatus as ProtocolRunStatus,
    };
    use board_vm_runtime::{RunStatus, Runtime};
    use std::vec;
    use std::vec::Vec;

    const BLINK_CODE: &[u8] = &[
        0x12, 0x0d, 0x12, 0x01, 0x40, 0x01, 0x20, 0x11, 0x40, 0x02, 0x13, 0xfa, 0x00, 0x40, 0x10,
        0x20, 0x10, 0x40, 0x02, 0x13, 0xfa, 0x00, 0x40, 0x10, 0x30, 0xec,
    ];

    #[derive(Debug, Clone, PartialEq, Eq)]
    enum Event {
        Configure(u8, GpioMode),
        Write(u8, Level),
        Sleep(u16),
        PwmWrite(u8, u16),
        AdcRead(u8),
        DacWriteU12(u8, u16),
        I2cOpen(u8),
        I2cWriteU8(u8, u16, u8),
        I2cWrite(u8, u16, Vec<u8>),
        I2cReadU8(u8, u16),
        I2cRead(u8, u16, u8),
        I2cTransfer(u8, u16, Vec<u8>, u8),
        SpiOpen(u8),
        SpiTransfer(u8, u8, Vec<u8>, u8),
        UartOpen(u8),
        LedMatrixFrame([u32; 3]),
        BootloaderReboot,
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

        fn supports_spi(&self) -> bool {
            true
        }

        fn supports_uart(&self) -> bool {
            true
        }

        fn supports_bootloader_reboot(&self) -> bool {
            true
        }

        fn write_pwm(&mut self, pin: u8, duty: u16) -> Result<(), HalError> {
            self.events.push(Event::PwmWrite(pin, duty));
            Ok(())
        }

        fn read_adc(&mut self, pin: u8) -> Result<u16, HalError> {
            self.events.push(Event::AdcRead(pin));
            Ok(0x02aa)
        }

        fn write_dac_u12(&mut self, pin: u8, sample: u16) -> Result<(), HalError> {
            self.events.push(Event::DacWriteU12(pin, sample));
            Ok(())
        }

        fn open_i2c(&mut self, bus: u8) -> Result<u32, HalError> {
            self.events.push(Event::I2cOpen(bus));
            Ok(0x1_2000 | bus as u32)
        }

        fn open_spi(&mut self, bus: u8) -> Result<u32, HalError> {
            self.events.push(Event::SpiOpen(bus));
            Ok(0x2_2000 | bus as u32)
        }

        fn open_uart(&mut self, bus: u8) -> Result<u32, HalError> {
            self.events.push(Event::UartOpen(bus));
            Ok(0x3_2000 | bus as u32)
        }

        fn transfer_spi(
            &mut self,
            bus: u8,
            cs_pin: u8,
            write_bytes: &[u8],
            read_len: u8,
        ) -> Result<ByteBuffer, HalError> {
            self.events.push(Event::SpiTransfer(
                bus,
                cs_pin,
                write_bytes.to_vec(),
                read_len,
            ));
            let mut response = [0u8; MAX_BYTE_BUFFER_LEN];
            response[0] = 0x9f;
            response[1] = 0x01;
            response[2] = 0x02;
            ByteBuffer::from_slice(&response[..read_len as usize])
                .map_err(|_| HalError::UnsupportedMode)
        }

        fn write_i2c_u8(&mut self, bus: u8, address: u16, byte: u8) -> Result<(), HalError> {
            self.events.push(Event::I2cWriteU8(bus, address, byte));
            Ok(())
        }

        fn write_i2c(&mut self, bus: u8, address: u16, bytes: &[u8]) -> Result<(), HalError> {
            self.events
                .push(Event::I2cWrite(bus, address, bytes.to_vec()));
            Ok(())
        }

        fn read_i2c_u8(&mut self, bus: u8, address: u16) -> Result<u8, HalError> {
            self.events.push(Event::I2cReadU8(bus, address));
            Ok(0x5a)
        }

        fn read_i2c(&mut self, bus: u8, address: u16, len: u8) -> Result<ByteBuffer, HalError> {
            self.events.push(Event::I2cRead(bus, address, len));
            ByteBuffer::from_slice(&[0xca, 0xfe, 0x42][..len as usize])
                .map_err(|_| HalError::UnsupportedMode)
        }

        fn transfer_i2c(
            &mut self,
            bus: u8,
            address: u16,
            write_bytes: &[u8],
            read_len: u8,
        ) -> Result<ByteBuffer, HalError> {
            self.events.push(Event::I2cTransfer(
                bus,
                address,
                write_bytes.to_vec(),
                read_len,
            ));
            ByteBuffer::from_slice(&[0x11, 0x22, 0x33][..read_len as usize])
                .map_err(|_| HalError::UnsupportedMode)
        }

        fn led_matrix_frame(&mut self, frame: [u32; 3]) -> Result<(), HalError> {
            self.events.push(Event::LedMatrixFrame(frame));
            Ok(())
        }

        fn reboot_to_bootloader(&mut self) -> Result<(), HalError> {
            self.events.push(Event::BootloaderReboot);
            Ok(())
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
    fn knows_uno_r4_analog_header_pins() {
        let a0 = digital_pin(14).unwrap();
        assert_eq!(a0.label, "A0/D14");
        assert!(a0.supports_adc);
        assert!(a0.supports_dac);
        assert!(!a0.supports_pwm);
        assert!(is_valid_digital_pin(19));
    }

    #[test]
    fn knows_uno_r4_i2c_buses() {
        assert_eq!(UNO_R4_MINIMA.i2c_buses, &[UNO_R4_HEADER_I2C_BUS]);
        assert_eq!(UNO_R4_WIFI.i2c_buses.len(), 2);
        assert_eq!(UNO_R4_WIFI.i2c_buses[0].name, "Wire");
        assert_eq!(UNO_R4_WIFI.i2c_buses[0].sda_pin, 18);
        assert_eq!(UNO_R4_WIFI.i2c_buses[0].scl_pin, 19);
        assert_eq!(UNO_R4_WIFI.i2c_buses[1].name, "Wire1");
        assert!(UNO_R4_WIFI.i2c_buses[1].qwiic);
    }

    #[test]
    fn knows_uno_r4_spi_buses() {
        assert_eq!(UNO_R4_MINIMA.spi_buses, &[UNO_R4_HEADER_SPI_BUS]);
        assert_eq!(UNO_R4_WIFI.spi_buses, &[UNO_R4_HEADER_SPI_BUS]);
        assert_eq!(UNO_R4_WIFI.spi_buses[0].name, "SPI");
        assert_eq!(UNO_R4_WIFI.spi_buses[0].copi_pin, 11);
        assert_eq!(UNO_R4_WIFI.spi_buses[0].cipo_pin, 12);
        assert_eq!(UNO_R4_WIFI.spi_buses[0].sck_pin, 13);
        assert_eq!(UNO_R4_WIFI.spi_buses[0].default_cs_pin, 10);
    }

    #[test]
    fn knows_uno_r4_uart_buses() {
        assert_eq!(UNO_R4_MINIMA.uart_buses, &[UNO_R4_MINIMA_HEADER_UART_BUS]);
        assert_eq!(UNO_R4_WIFI.uart_buses.len(), 3);
        assert_eq!(UNO_R4_WIFI.uart_buses[0].name, "Serial1");
        assert_eq!(UNO_R4_WIFI.uart_buses[0].tx_pin, 22);
        assert_eq!(UNO_R4_WIFI.uart_buses[0].rx_pin, 23);
        assert_eq!(UNO_R4_WIFI.uart_buses[0].arduino_uart, 1);
        assert!(!UNO_R4_WIFI.uart_buses[0].internal);
        assert_eq!(UNO_R4_WIFI.uart_buses[1].name, "Serial2");
        assert_eq!(UNO_R4_WIFI.uart_buses[1].tx_pin, 1);
        assert_eq!(UNO_R4_WIFI.uart_buses[1].rx_pin, 0);
        assert_eq!(UNO_R4_WIFI.uart_buses[2].name, "Serial3");
        assert_eq!(UNO_R4_WIFI.uart_buses[2].tx_pin, 24);
        assert_eq!(UNO_R4_WIFI.uart_buses[2].rx_pin, 25);
        assert!(UNO_R4_WIFI.uart_buses[2].internal);
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

    #[test]
    fn maps_target_metadata_to_device_descriptor() {
        let descriptor = uno_r4_device_descriptor(&UNO_R4_WIFI, 0xA11C_E001);

        assert_eq!(descriptor.board_id, UNO_R4_WIFI.board_id);
        assert_eq!(descriptor.runtime_id, UNO_R4_VM_RUNTIME_ID);
        assert_eq!(descriptor.board_nonce, 0xA11C_E001);
        assert_eq!(descriptor.max_frame_payload, DEFAULT_MAX_FRAME_PAYLOAD);
        assert_eq!(
            descriptor.capabilities.len(),
            BLINK_MVP_WITH_PWM_ADC_DAC_I2C_SPI_UART_AND_LED_MATRIX_CAPABILITIES.len()
        );
        assert!(UNO_R4_WIFI
            .capabilities
            .supports(board_vm_ir::CAP_PWM_WRITE));
        assert!(UNO_R4_WIFI.capabilities.supports(board_vm_ir::CAP_ADC_READ));
        assert!(UNO_R4_WIFI
            .capabilities
            .supports(board_vm_ir::CAP_DAC_WRITE_U12));
        assert!(UNO_R4_WIFI.capabilities.supports(board_vm_ir::CAP_I2C_OPEN));
        assert!(UNO_R4_WIFI
            .capabilities
            .supports(board_vm_ir::CAP_I2C_WRITE_U8));
        assert!(UNO_R4_WIFI
            .capabilities
            .supports(board_vm_ir::CAP_I2C_READ_U8));
        assert!(UNO_R4_WIFI
            .capabilities
            .supports(board_vm_ir::CAP_I2C_WRITE));
        assert!(UNO_R4_WIFI.capabilities.supports(board_vm_ir::CAP_I2C_READ));
        assert!(UNO_R4_WIFI
            .capabilities
            .supports(board_vm_ir::CAP_I2C_TRANSFER));
        assert!(UNO_R4_WIFI.capabilities.supports(board_vm_ir::CAP_SPI_OPEN));
        assert!(UNO_R4_WIFI
            .capabilities
            .supports(board_vm_ir::CAP_SPI_TRANSFER));
        assert!(UNO_R4_WIFI
            .capabilities
            .supports(board_vm_ir::CAP_UART_OPEN));
        assert!(UNO_R4_MINIMA
            .capabilities
            .supports(board_vm_ir::CAP_PWM_WRITE));
        assert!(UNO_R4_MINIMA
            .capabilities
            .supports(board_vm_ir::CAP_ADC_READ));
        assert!(UNO_R4_MINIMA
            .capabilities
            .supports(board_vm_ir::CAP_DAC_WRITE_U12));
        assert!(UNO_R4_MINIMA
            .capabilities
            .supports(board_vm_ir::CAP_I2C_OPEN));
        assert!(UNO_R4_MINIMA
            .capabilities
            .supports(board_vm_ir::CAP_I2C_WRITE_U8));
        assert!(UNO_R4_MINIMA
            .capabilities
            .supports(board_vm_ir::CAP_I2C_READ_U8));
        assert!(UNO_R4_MINIMA
            .capabilities
            .supports(board_vm_ir::CAP_I2C_WRITE));
        assert!(UNO_R4_MINIMA
            .capabilities
            .supports(board_vm_ir::CAP_I2C_READ));
        assert!(UNO_R4_MINIMA
            .capabilities
            .supports(board_vm_ir::CAP_I2C_TRANSFER));
        assert!(UNO_R4_MINIMA
            .capabilities
            .supports(board_vm_ir::CAP_SPI_OPEN));
        assert!(UNO_R4_MINIMA
            .capabilities
            .supports(board_vm_ir::CAP_SPI_TRANSFER));
        assert!(UNO_R4_MINIMA
            .capabilities
            .supports(board_vm_ir::CAP_UART_OPEN));
        assert!(UNO_R4_WIFI
            .capabilities
            .supports(board_vm_ir::CAP_LED_MATRIX_FRAME));
        assert!(!UNO_R4_MINIMA
            .capabilities
            .supports(board_vm_ir::CAP_LED_MATRIX_FRAME));
    }

    #[test]
    fn led_matrix_frame_runs_through_wifi_backend() {
        let mut board = UnoR4Board::wifi(FakeBackend::new());
        board
            .led_matrix_frame([0x3184_A444, 0x4404_2081, 0x100A_0040])
            .unwrap();

        assert_eq!(
            board.backend().events,
            vec![Event::LedMatrixFrame([
                0x3184_A444,
                0x4404_2081,
                0x100A_0040,
            ])]
        );
    }

    #[test]
    fn pwm_write_runs_through_pwm_pin_metadata() {
        let mut board = UnoR4Board::wifi(FakeBackend::new());
        board.pwm_write(3, 0x8000).unwrap();

        assert_eq!(board.backend().events, vec![Event::PwmWrite(3, 0x8000)]);
    }

    #[test]
    fn pwm_write_rejects_non_pwm_pin() {
        let mut board = UnoR4Board::wifi(FakeBackend::new());

        assert_eq!(board.pwm_write(2, 0x8000), Err(HalError::UnsupportedMode));
        assert_eq!(board.pwm_write(99, 0x8000), Err(HalError::InvalidPin));
    }

    #[test]
    fn adc_read_runs_through_analog_pin_metadata() {
        let mut board = UnoR4Board::wifi(FakeBackend::new());

        assert_eq!(board.adc_read(14).unwrap(), 0x02aa);
        assert_eq!(board.backend().events, vec![Event::AdcRead(14)]);
    }

    #[test]
    fn adc_read_rejects_non_adc_pin() {
        let mut board = UnoR4Board::wifi(FakeBackend::new());

        assert_eq!(board.adc_read(13), Err(HalError::UnsupportedMode));
        assert_eq!(board.adc_read(99), Err(HalError::InvalidPin));
    }

    #[test]
    fn dac_write_u12_runs_through_dac_pin_metadata() {
        let mut board = UnoR4Board::wifi(FakeBackend::new());

        board.dac_write_u12(14, 0x0800).unwrap();
        assert_eq!(board.backend().events, vec![Event::DacWriteU12(14, 0x0800)]);
    }

    #[test]
    fn dac_write_u12_rejects_non_dac_pin() {
        let mut board = UnoR4Board::wifi(FakeBackend::new());

        assert_eq!(
            board.dac_write_u12(15, 0x0800),
            Err(HalError::UnsupportedMode)
        );
        assert_eq!(
            board.dac_write_u12(14, 0x1000),
            Err(HalError::UnsupportedMode)
        );
        assert_eq!(board.dac_write_u12(99, 0x0800), Err(HalError::InvalidPin));
    }

    #[test]
    fn i2c_open_runs_through_bus_metadata() {
        let mut board = UnoR4Board::wifi(FakeBackend::new());

        assert_eq!(board.i2c_open(1).unwrap(), 0x1_2001);
        assert_eq!(board.backend().events, vec![Event::I2cOpen(1)]);
    }

    #[test]
    fn i2c_open_rejects_unknown_bus() {
        let mut board = UnoR4Board::minima(FakeBackend::new());

        assert_eq!(board.i2c_open(1), Err(HalError::InvalidPin));
        assert_eq!(board.i2c_open(99), Err(HalError::InvalidPin));
    }

    #[test]
    fn spi_open_runs_through_bus_metadata() {
        let mut board = UnoR4Board::wifi(FakeBackend::new());

        assert_eq!(board.spi_open(0).unwrap(), 0x2_2000);
        assert_eq!(board.backend().events, vec![Event::SpiOpen(0)]);
    }

    #[test]
    fn spi_open_rejects_unknown_bus() {
        let mut board = UnoR4Board::wifi(FakeBackend::new());

        assert_eq!(board.spi_open(1), Err(HalError::InvalidPin));
        assert_eq!(board.spi_open(99), Err(HalError::InvalidPin));
    }

    #[test]
    fn uart_open_runs_through_bus_metadata() {
        let mut board = UnoR4Board::wifi(FakeBackend::new());

        assert_eq!(board.uart_open(2).unwrap(), 0x3_2002);
        assert_eq!(board.backend().events, vec![Event::UartOpen(2)]);
    }

    #[test]
    fn uart_open_rejects_unknown_bus() {
        let mut board = UnoR4Board::minima(FakeBackend::new());

        assert_eq!(board.uart_open(1), Err(HalError::InvalidPin));
        assert_eq!(board.uart_open(99), Err(HalError::InvalidPin));
    }

    #[test]
    fn spi_transfer_runs_through_bus_metadata_and_chip_select_pin() {
        let mut board = UnoR4Board::wifi(FakeBackend::new());

        assert_eq!(
            board.spi_transfer(0x2_2000, 10, &[0x9f], 3).unwrap(),
            ByteBuffer::from_slice(&[0x9f, 0x01, 0x02]).unwrap()
        );
        assert_eq!(
            board.backend().events,
            vec![Event::SpiTransfer(0, 10, vec![0x9f], 3)]
        );
    }

    #[test]
    fn spi_transfer_rejects_unknown_bus_or_chip_select_pin() {
        let mut board = UnoR4Board::wifi(FakeBackend::new());

        assert_eq!(
            board.spi_transfer(0x2_2001, 10, &[0x9f], 3),
            Err(HalError::InvalidPin)
        );
        assert_eq!(
            board.spi_transfer(0x2_2000, 99, &[0x9f], 3),
            Err(HalError::InvalidPin)
        );
    }

    #[test]
    fn i2c_write_u8_runs_through_bus_metadata() {
        let mut board = UnoR4Board::wifi(FakeBackend::new());

        board.i2c_write_u8(0x1_2001, 0x3c, 0xa5).unwrap();

        assert_eq!(
            board.backend().events,
            vec![Event::I2cWriteU8(1, 0x3c, 0xa5)]
        );
    }

    #[test]
    fn i2c_write_u8_rejects_unknown_bus_or_address() {
        let mut board = UnoR4Board::minima(FakeBackend::new());

        assert_eq!(
            board.i2c_write_u8(0x1_2001, 0x3c, 0xa5),
            Err(HalError::InvalidPin)
        );
        assert_eq!(
            board.i2c_write_u8(0x1_2000, 0x80, 0xa5),
            Err(HalError::InvalidPin)
        );
    }

    #[test]
    fn i2c_write_runs_through_bus_metadata() {
        let mut board = UnoR4Board::wifi(FakeBackend::new());

        board
            .i2c_write(0x1_2001, 0x3c, &[0xde, 0xad, 0xbe])
            .unwrap();

        assert_eq!(
            board.backend().events,
            vec![Event::I2cWrite(1, 0x3c, vec![0xde, 0xad, 0xbe])]
        );
    }

    #[test]
    fn i2c_write_rejects_unknown_bus_or_address() {
        let mut board = UnoR4Board::minima(FakeBackend::new());

        assert_eq!(
            board.i2c_write(0x1_2001, 0x3c, &[0xde, 0xad, 0xbe]),
            Err(HalError::InvalidPin)
        );
        assert_eq!(
            board.i2c_write(0x1_2000, 0x80, &[0xde, 0xad, 0xbe]),
            Err(HalError::InvalidPin)
        );
    }

    #[test]
    fn i2c_read_u8_runs_through_bus_metadata() {
        let mut board = UnoR4Board::wifi(FakeBackend::new());

        assert_eq!(board.i2c_read_u8(0x1_2001, 0x3c).unwrap(), 0x5a);
        assert_eq!(board.backend().events, vec![Event::I2cReadU8(1, 0x3c)]);
    }

    #[test]
    fn i2c_read_u8_rejects_unknown_bus_or_address() {
        let mut board = UnoR4Board::minima(FakeBackend::new());

        assert_eq!(board.i2c_read_u8(0x1_2001, 0x3c), Err(HalError::InvalidPin));
        assert_eq!(board.i2c_read_u8(0x1_2000, 0x80), Err(HalError::InvalidPin));
    }

    #[test]
    fn i2c_read_runs_through_bus_metadata() {
        let mut board = UnoR4Board::wifi(FakeBackend::new());

        assert_eq!(
            board.i2c_read(0x1_2001, 0x3c, 3).unwrap(),
            ByteBuffer::from_slice(&[0xca, 0xfe, 0x42]).unwrap()
        );
        assert_eq!(board.backend().events, vec![Event::I2cRead(1, 0x3c, 3)]);
    }

    #[test]
    fn i2c_read_rejects_unknown_bus_or_address() {
        let mut board = UnoR4Board::minima(FakeBackend::new());

        assert_eq!(board.i2c_read(0x1_2001, 0x3c, 3), Err(HalError::InvalidPin));
        assert_eq!(board.i2c_read(0x1_2000, 0x80, 3), Err(HalError::InvalidPin));
    }

    #[test]
    fn i2c_transfer_runs_through_bus_metadata() {
        let mut board = UnoR4Board::wifi(FakeBackend::new());

        assert_eq!(
            board
                .i2c_transfer(0x1_2001, 0x3c, &[0x00, 0x10], 3)
                .unwrap(),
            ByteBuffer::from_slice(&[0x11, 0x22, 0x33]).unwrap()
        );
        assert_eq!(
            board.backend().events,
            vec![Event::I2cTransfer(1, 0x3c, vec![0x00, 0x10], 3)]
        );
    }

    #[test]
    fn i2c_transfer_rejects_unknown_bus_or_address() {
        let mut board = UnoR4Board::minima(FakeBackend::new());

        assert_eq!(
            board.i2c_transfer(0x1_2001, 0x3c, &[0x00, 0x10], 3),
            Err(HalError::InvalidPin)
        );
        assert_eq!(
            board.i2c_transfer(0x1_2000, 0x80, &[0x00, 0x10], 3),
            Err(HalError::InvalidPin)
        );
    }

    #[test]
    fn led_matrix_frame_rejects_minima() {
        let mut board = UnoR4Board::minima(FakeBackend::new());
        assert_eq!(
            board.led_matrix_frame([0, 0, 0]),
            Err(HalError::UnsupportedMode)
        );
    }

    #[test]
    fn bootloader_reboot_runs_through_backend() {
        let mut board = UnoR4Board::wifi(FakeBackend::new());

        assert!(board.supports_bootloader_reboot());
        board.reboot_to_bootloader().unwrap();

        assert_eq!(board.backend().events, vec![Event::BootloaderReboot]);
    }

    #[test]
    fn host_can_upload_and_run_blink_through_uno_r4_device() {
        let mut device = minima_device(FakeBackend::new(), 0xB04D_1001);
        let mut session = HostSession::new();
        let mut host_payload = [0u8; 128];
        let mut request = [0u8; 256];
        let mut device_payload = [0u8; 512];
        let mut response = [0u8; 768];

        let hello = session
            .hello_frame("uno-r4-test", 0xCAFE_BABE, &mut host_payload, &mut request)
            .unwrap();
        let response_len = device
            .handle_raw_frame(&request[..hello.len], &mut device_payload, &mut response)
            .unwrap();
        let frame = decode_frame(&response[..response_len]).unwrap();
        assert_eq!(frame.message_type, MessageType::HELLO_ACK);
        let ack = decode_hello_ack(frame.payload).unwrap();
        assert_eq!(ack.board_name, UNO_R4_MINIMA.board_id);
        assert_eq!(ack.runtime_name, UNO_R4_VM_RUNTIME_ID);
        assert_eq!(ack.host_nonce, 0xCAFE_BABE);

        let caps = session.caps_query_frame(&mut request).unwrap();
        let response_len = device
            .handle_raw_frame(&request[..caps.len], &mut device_payload, &mut response)
            .unwrap();
        let frame = decode_frame(&response[..response_len]).unwrap();
        let (header, mut decoder) = decode_caps_report_header(frame.payload).unwrap();
        assert_eq!(header.board_id, UNO_R4_MINIMA.board_id);
        assert_eq!(header.runtime_id, UNO_R4_VM_RUNTIME_ID);
        assert_eq!(header.max_program_bytes, UNO_R4_VM_MAX_PROGRAM_BYTES as u32);
        assert_eq!(header.max_stack_values, UNO_R4_VM_MAX_STACK_VALUES as u8);
        assert_eq!(header.max_handles, UNO_R4_VM_MAX_HANDLES as u8);
        for _ in 0..header.capability_count {
            decoder.read_capability_descriptor().unwrap();
        }
        decoder.finish().unwrap();

        let mut module = [0u8; board_vm_host::BLINK_MODULE_LEN];
        let module_len = write_blink_module(BlinkProgram::onboard_led(), &mut module).unwrap();
        let module = &module[..module_len];
        let begin = session
            .program_begin_frame(DEFAULT_PROGRAM_ID, module, &mut host_payload, &mut request)
            .unwrap();
        device
            .handle_raw_frame(&request[..begin.len], &mut device_payload, &mut response)
            .unwrap();
        let chunk = session
            .program_chunk_frame(
                DEFAULT_PROGRAM_ID,
                0,
                module,
                &mut host_payload,
                &mut request,
            )
            .unwrap();
        device
            .handle_raw_frame(&request[..chunk.len], &mut device_payload, &mut response)
            .unwrap();
        let end = session
            .program_end_frame(DEFAULT_PROGRAM_ID, &mut host_payload, &mut request)
            .unwrap();
        device
            .handle_raw_frame(&request[..end.len], &mut device_payload, &mut response)
            .unwrap();

        let run = session
            .run_background_frame(DEFAULT_PROGRAM_ID, 100, &mut host_payload, &mut request)
            .unwrap();
        let response_len = device
            .handle_raw_frame(&request[..run.len], &mut device_payload, &mut response)
            .unwrap();
        let frame = decode_frame(&response[..response_len]).unwrap();
        let (report, decoder) = decode_run_report_header(frame.payload).unwrap();
        decoder.finish().unwrap();
        assert_eq!(report.status, ProtocolRunStatus::Running);
        assert_eq!(report.open_handles, 1);
        assert_eq!(
            device.hal().backend().events[..5],
            [
                Event::Configure(13, GpioMode::Output),
                Event::Write(13, Level::High),
                Event::Sleep(250),
                Event::Write(13, Level::Low),
                Event::Sleep(250),
            ]
        );
    }
}
