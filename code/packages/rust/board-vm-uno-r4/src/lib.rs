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
    BLINK_MVP_WITH_PWM_ADC_DAC_I2C_SPI_UART_AND_CAN_CAPABILITIES,
    BLINK_MVP_WITH_PWM_ADC_DAC_I2C_SPI_UART_AND_LED_MATRIX_CAPABILITIES,
    BLINK_MVP_WITH_PWM_ADC_DAC_I2C_SPI_UART_AND_RTC_CAPABILITIES,
    BLINK_MVP_WITH_PWM_ADC_DAC_I2C_SPI_UART_CAN_AND_LED_MATRIX_CAPABILITIES,
    BLINK_MVP_WITH_PWM_ADC_DAC_I2C_SPI_UART_CAN_AND_RTC_CAPABILITIES,
    BLINK_MVP_WITH_PWM_ADC_DAC_I2C_SPI_UART_CAN_RTC_AND_LED_MATRIX_CAPABILITIES,
    BLINK_MVP_WITH_PWM_ADC_DAC_I2C_SPI_UART_CAN_RTC_AND_WATCHDOG_CAPABILITIES,
    BLINK_MVP_WITH_PWM_ADC_DAC_I2C_SPI_UART_CAN_RTC_WATCHDOG_AND_LED_MATRIX_CAPABILITIES,
    BLINK_MVP_WITH_PWM_ADC_DAC_I2C_SPI_UART_CAN_RTC_WATCHDOG_AND_STORAGE_CAPABILITIES,
    BLINK_MVP_WITH_PWM_ADC_DAC_I2C_SPI_UART_CAN_RTC_WATCHDOG_STORAGE_AND_LED_MATRIX_CAPABILITIES,
    BLINK_MVP_WITH_PWM_ADC_DAC_I2C_SPI_UART_CAN_RTC_WATCHDOG_STORAGE_NETWORK_TCP_AND_LED_MATRIX_CAPABILITIES,
    BLINK_MVP_WITH_PWM_ADC_DAC_I2C_SPI_UART_CAN_RTC_WATCHDOG_STORAGE_NETWORK_TCP_UDP_AND_LED_MATRIX_CAPABILITIES,
    BLINK_MVP_WITH_PWM_ADC_DAC_I2C_SPI_UART_RTC_AND_LED_MATRIX_CAPABILITIES,
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
pub const UNO_R4_PROGRAM_STORE_REGION: u16 = 0;
pub const UNO_R4_PROGRAM_STORE_SLOT: u8 = 0;
pub const UNO_R4_PROGRAM_STORE_MAGIC: [u8; 4] = *b"BVMS";
pub const UNO_R4_PROGRAM_STORE_LAYOUT_VERSION: u8 = 1;
pub const UNO_R4_PROGRAM_STORE_FORMAT_BVM_MODULE: u8 = 1;
pub const UNO_R4_PROGRAM_STORE_HEADER_BYTES: usize = 20;

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
    pub can_buses: &'static [CanBusDescriptor],
    pub storage_regions: &'static [StorageRegionDescriptor],
    pub rtc: Option<RtcDescriptor>,
    pub watchdog: Option<WatchdogDescriptor>,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CanBusDescriptor {
    pub bus: u8,
    pub name: &'static str,
    pub tx_pin: u8,
    pub rx_pin: u8,
    pub controller: &'static str,
    pub notes: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RtcDescriptor {
    pub instance: u8,
    pub name: &'static str,
    pub peripheral: &'static str,
    pub notes: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WatchdogDescriptor {
    pub instance: u8,
    pub name: &'static str,
    pub peripheral: &'static str,
    pub notes: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StorageRegionDescriptor {
    pub region: u8,
    pub name: &'static str,
    pub kind: &'static str,
    pub bytes: u32,
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

pub const UNO_R4_CAN_BUS: CanBusDescriptor = CanBusDescriptor {
    bus: 0,
    name: "CAN0",
    tx_pin: 10,
    rx_pin: 13,
    controller: "RA4M1 CAN0",
    notes: "CAN0 on D10/TX and D13/RX; conflicts with header SPI CS/SCK and onboard LED",
};

pub const UNO_R4_CAN_BUSES: [CanBusDescriptor; 1] = [UNO_R4_CAN_BUS];

pub const UNO_R4_RTC: RtcDescriptor = RtcDescriptor {
    instance: 0,
    name: "RTC",
    peripheral: "RA4M1 RTC",
    notes: "Single real-time clock instance exposed by the UNO R4 core",
};

pub const UNO_R4_WATCHDOG: WatchdogDescriptor = WatchdogDescriptor {
    instance: 0,
    name: "WDT",
    peripheral: "RA4M1 WDT",
    notes: "Renesas watchdog timer exposed through the UNO R4 core WDT library",
};

pub const UNO_R4_EEPROM_STORAGE: StorageRegionDescriptor = StorageRegionDescriptor {
    region: 0,
    name: "EEPROM emulation",
    kind: "data flash",
    bytes: UNO_R4_DATA_FLASH_BYTES,
    notes: "Flash-backed EEPROM storage area exposed by the UNO R4 core through storage.write and storage.read",
};

pub const UNO_R4_STORAGE_REGIONS: [StorageRegionDescriptor; 1] = [UNO_R4_EEPROM_STORAGE];

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
        .with_spi()
        .with_can()
        .with_rtc()
        .with_watchdog()
        .with_storage(),
    digital_pins: &UNO_R4_DIGITAL_PINS,
    i2c_buses: &UNO_R4_MINIMA_I2C_BUSES,
    spi_buses: &UNO_R4_SPI_BUSES,
    uart_buses: &UNO_R4_MINIMA_UART_BUSES,
    can_buses: &UNO_R4_CAN_BUSES,
    storage_regions: &UNO_R4_STORAGE_REGIONS,
    rtc: Some(UNO_R4_RTC),
    watchdog: Some(UNO_R4_WATCHDOG),
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
        .with_led_matrix()
        .with_can()
        .with_rtc()
        .with_watchdog()
        .with_storage()
        .with_network_tcp()
        .with_network_udp(),
    digital_pins: &UNO_R4_DIGITAL_PINS,
    i2c_buses: &UNO_R4_WIFI_I2C_BUSES,
    spi_buses: &UNO_R4_SPI_BUSES,
    uart_buses: &UNO_R4_WIFI_UART_BUSES,
    can_buses: &UNO_R4_CAN_BUSES,
    storage_regions: &UNO_R4_STORAGE_REGIONS,
    rtc: Some(UNO_R4_RTC),
    watchdog: Some(UNO_R4_WATCHDOG),
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

    fn supports_can(&self) -> bool {
        false
    }

    fn supports_rtc(&self) -> bool {
        false
    }

    fn supports_watchdog(&self) -> bool {
        false
    }

    fn supports_storage(&self) -> bool {
        false
    }

    fn supports_network_tcp(&self) -> bool {
        false
    }

    fn supports_network_udp(&self) -> bool {
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

    fn write_uart(&mut self, _bus: u8, _byte: u8) -> Result<(), HalError> {
        Err(HalError::UnsupportedMode)
    }

    fn read_uart(&mut self, _bus: u8) -> Result<u8, HalError> {
        Err(HalError::UnsupportedMode)
    }

    fn open_can(&mut self, _bus: u8) -> Result<u32, HalError> {
        Err(HalError::UnsupportedMode)
    }

    fn write_can(&mut self, _bus: u8, _byte: u8) -> Result<(), HalError> {
        Err(HalError::UnsupportedMode)
    }

    fn read_can(&mut self, _bus: u8) -> Result<u8, HalError> {
        Err(HalError::UnsupportedMode)
    }

    fn rtc_now(&mut self) -> Result<u32, HalError> {
        Err(HalError::UnsupportedMode)
    }

    fn rtc_set(&mut self, _epoch_seconds: u32) -> Result<(), HalError> {
        Err(HalError::UnsupportedMode)
    }

    fn watchdog_configure(&mut self, _timeout_ms: u32) -> Result<(), HalError> {
        Err(HalError::UnsupportedMode)
    }

    fn watchdog_kick(&mut self) -> Result<(), HalError> {
        Err(HalError::UnsupportedMode)
    }

    fn storage_write(&mut self, _region: u8, _offset: u16, _bytes: &[u8]) -> Result<(), HalError> {
        Err(HalError::UnsupportedMode)
    }

    fn storage_read(
        &mut self,
        _region: u8,
        _offset: u16,
        _len: u8,
    ) -> Result<ByteBuffer, HalError> {
        Err(HalError::UnsupportedMode)
    }

    fn open_network_tcp(
        &mut self,
        _interface: u8,
        _remote_ipv4: u32,
        _remote_port: u16,
    ) -> Result<u32, HalError> {
        Err(HalError::UnsupportedMode)
    }

    fn write_network_tcp(&mut self, _socket: u8, _byte: u8) -> Result<(), HalError> {
        Err(HalError::UnsupportedMode)
    }

    fn read_network_tcp(&mut self, _socket: u8) -> Result<u8, HalError> {
        Err(HalError::UnsupportedMode)
    }

    fn close_network_tcp(&mut self, _socket: u8) -> Result<(), HalError> {
        Err(HalError::UnsupportedMode)
    }

    fn open_network_udp(
        &mut self,
        _interface: u8,
        _remote_ipv4: u32,
        _remote_port: u16,
    ) -> Result<u32, HalError> {
        Err(HalError::UnsupportedMode)
    }

    fn write_network_udp(&mut self, _socket: u8, _byte: u8) -> Result<(), HalError> {
        Err(HalError::UnsupportedMode)
    }

    fn read_network_udp(&mut self, _socket: u8) -> Result<u8, HalError> {
        Err(HalError::UnsupportedMode)
    }

    fn close_network_udp(&mut self, _socket: u8) -> Result<(), HalError> {
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
        capabilities.can = self.backend.supports_can();
        capabilities.rtc = self.backend.supports_rtc();
        capabilities.watchdog = self.backend.supports_watchdog();
        capabilities.storage = self.backend.supports_storage();
        capabilities.network_tcp =
            self.target.supports_wifi_module && self.backend.supports_network_tcp();
        capabilities.network_udp =
            self.target.supports_wifi_module && self.backend.supports_network_udp();
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

    fn uart_write(&mut self, token: u32, byte: u8) -> Result<(), HalError> {
        let bus = normalize_uart_token(self.target, token)?;
        self.backend.write_uart(bus, byte)
    }

    fn uart_read(&mut self, token: u32) -> Result<u8, HalError> {
        let bus = normalize_uart_token(self.target, token)?;
        self.backend.read_uart(bus)
    }

    fn can_open(&mut self, bus: u16) -> Result<u32, HalError> {
        let bus = normalize_can_bus(self.target, bus)?;
        self.backend.open_can(bus)
    }

    fn can_write(&mut self, token: u32, byte: u8) -> Result<(), HalError> {
        let bus = normalize_can_token(self.target, token)?;
        self.backend.write_can(bus, byte)
    }

    fn can_read(&mut self, token: u32) -> Result<u8, HalError> {
        let bus = normalize_can_token(self.target, token)?;
        self.backend.read_can(bus)
    }

    fn rtc_now(&mut self) -> Result<u32, HalError> {
        if self.target.rtc.is_none() {
            return Err(HalError::UnsupportedMode);
        }
        self.backend.rtc_now()
    }

    fn rtc_set(&mut self, epoch_seconds: u32) -> Result<(), HalError> {
        if self.target.rtc.is_none() {
            return Err(HalError::UnsupportedMode);
        }
        self.backend.rtc_set(epoch_seconds)
    }

    fn watchdog_configure(&mut self, timeout_ms: u32) -> Result<(), HalError> {
        if self.target.watchdog.is_none() {
            return Err(HalError::UnsupportedMode);
        }
        self.backend.watchdog_configure(timeout_ms)
    }

    fn watchdog_kick(&mut self) -> Result<(), HalError> {
        if self.target.watchdog.is_none() {
            return Err(HalError::UnsupportedMode);
        }
        self.backend.watchdog_kick()
    }

    fn storage_write(&mut self, region: u16, offset: u16, bytes: &[u8]) -> Result<(), HalError> {
        if bytes.len() > u8::MAX as usize {
            return Err(HalError::InvalidPin);
        }
        let region = normalize_storage_region(self.target, region, offset, bytes.len() as u8)?;
        self.backend.storage_write(region, offset, bytes)
    }

    fn storage_read(&mut self, region: u16, offset: u16, len: u8) -> Result<ByteBuffer, HalError> {
        let region = normalize_storage_region(self.target, region, offset, len)?;
        self.backend.storage_read(region, offset, len)
    }

    fn network_tcp_open(
        &mut self,
        interface: u16,
        remote_ipv4: u32,
        remote_port: u16,
    ) -> Result<u32, HalError> {
        if !self.target.supports_wifi_module {
            return Err(HalError::UnsupportedMode);
        }
        let interface = normalize_network_interface(interface)?;
        self.backend
            .open_network_tcp(interface, remote_ipv4, remote_port)
    }

    fn network_tcp_write(&mut self, token: u32, byte: u8) -> Result<(), HalError> {
        let socket = normalize_network_socket(token)?;
        self.backend.write_network_tcp(socket, byte)
    }

    fn network_tcp_read(&mut self, token: u32) -> Result<u8, HalError> {
        let socket = normalize_network_socket(token)?;
        self.backend.read_network_tcp(socket)
    }

    fn network_tcp_close(&mut self, token: u32) -> Result<(), HalError> {
        let socket = normalize_network_socket(token)?;
        self.backend.close_network_tcp(socket)
    }

    fn network_udp_open(
        &mut self,
        interface: u16,
        remote_ipv4: u32,
        remote_port: u16,
    ) -> Result<u32, HalError> {
        if !self.target.supports_wifi_module {
            return Err(HalError::UnsupportedMode);
        }
        let interface = normalize_network_interface(interface)?;
        self.backend
            .open_network_udp(interface, remote_ipv4, remote_port)
    }

    fn network_udp_write(&mut self, token: u32, byte: u8) -> Result<(), HalError> {
        let socket = normalize_network_socket(token)?;
        self.backend.write_network_udp(socket, byte)
    }

    fn network_udp_read(&mut self, token: u32) -> Result<u8, HalError> {
        let socket = normalize_network_socket(token)?;
        self.backend.read_network_udp(socket)
    }

    fn network_udp_close(&mut self, token: u32) -> Result<(), HalError> {
        let socket = normalize_network_socket(token)?;
        self.backend.close_network_udp(socket)
    }

    fn store_program(
        &mut self,
        program_id: u16,
        slot: u8,
        boot_policy: u8,
        module: &[u8],
    ) -> Result<(), HalError> {
        if slot != UNO_R4_PROGRAM_STORE_SLOT {
            return Err(HalError::InvalidPin);
        }
        let total_len = UNO_R4_PROGRAM_STORE_HEADER_BYTES
            .checked_add(module.len())
            .ok_or(HalError::InvalidPin)?;
        if total_len > u16::MAX as usize {
            return Err(HalError::InvalidPin);
        }
        let Some(descriptor) = self
            .target
            .storage_regions
            .iter()
            .find(|storage| storage.region == UNO_R4_PROGRAM_STORE_REGION as u8)
        else {
            return Err(HalError::InvalidPin);
        };
        if total_len > descriptor.bytes as usize {
            return Err(HalError::InvalidPin);
        }

        let module_len = module.len() as u32;
        let module_crc32 = crc32_ieee(module);
        let mut header = [0u8; UNO_R4_PROGRAM_STORE_HEADER_BYTES];
        header[0..4].copy_from_slice(&UNO_R4_PROGRAM_STORE_MAGIC);
        header[4] = UNO_R4_PROGRAM_STORE_LAYOUT_VERSION;
        header[5] = slot;
        header[6] = boot_policy;
        header[7] = UNO_R4_PROGRAM_STORE_FORMAT_BVM_MODULE;
        header[8..10].copy_from_slice(&program_id.to_le_bytes());
        header[12..16].copy_from_slice(&module_len.to_le_bytes());
        header[16..20].copy_from_slice(&module_crc32.to_le_bytes());

        self.storage_write(UNO_R4_PROGRAM_STORE_REGION, 0, &header)?;
        let mut offset = UNO_R4_PROGRAM_STORE_HEADER_BYTES as u16;
        for chunk in module.chunks(u8::MAX as usize) {
            self.storage_write(UNO_R4_PROGRAM_STORE_REGION, offset, chunk)?;
            offset = offset
                .checked_add(chunk.len() as u16)
                .ok_or(HalError::InvalidPin)?;
        }
        Ok(())
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

fn normalize_uart_token(target: &TargetDescriptor, token: u32) -> Result<u8, HalError> {
    let bus = u16::try_from(token & 0xff).map_err(|_| HalError::InvalidPin)?;
    normalize_uart_bus(target, bus)
}

fn normalize_can_bus(target: &TargetDescriptor, bus: u16) -> Result<u8, HalError> {
    let bus = u8::try_from(bus).map_err(|_| HalError::InvalidPin)?;
    if target
        .can_buses
        .iter()
        .any(|descriptor| descriptor.bus == bus)
    {
        Ok(bus)
    } else {
        Err(HalError::InvalidPin)
    }
}

fn normalize_can_token(target: &TargetDescriptor, token: u32) -> Result<u8, HalError> {
    let bus = u16::try_from(token & 0xff).map_err(|_| HalError::InvalidPin)?;
    normalize_can_bus(target, bus)
}

fn normalize_i2c_address(address: u16) -> Result<(), HalError> {
    if address <= 0x7f {
        Ok(())
    } else {
        Err(HalError::InvalidPin)
    }
}

fn normalize_storage_region(
    target: &TargetDescriptor,
    region: u16,
    offset: u16,
    len: u8,
) -> Result<u8, HalError> {
    if region > u8::MAX as u16 {
        return Err(HalError::InvalidPin);
    }
    let Some(descriptor) = target
        .storage_regions
        .iter()
        .find(|storage| storage.region == region as u8)
    else {
        return Err(HalError::InvalidPin);
    };
    let end = (offset as u32)
        .checked_add(len as u32)
        .ok_or(HalError::InvalidPin)?;
    if end <= descriptor.bytes {
        Ok(region as u8)
    } else {
        Err(HalError::InvalidPin)
    }
}

fn normalize_network_interface(interface: u16) -> Result<u8, HalError> {
    if interface == 0 {
        Ok(0)
    } else {
        Err(HalError::InvalidPin)
    }
}

fn normalize_network_socket(token: u32) -> Result<u8, HalError> {
    u8::try_from(token & 0xff).map_err(|_| HalError::InvalidPin)
}

fn crc32_ieee(bytes: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for byte in bytes {
        crc ^= *byte as u32;
        for _ in 0..8 {
            let mask = 0u32.wrapping_sub(crc & 1);
            crc = (crc >> 1) ^ (0xEDB8_8320 & mask);
        }
    }
    !crc
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
        supports_store_program: capabilities.storage,
        capabilities: if target.supports_wifi_module
            && target.supports_led_matrix
            && capabilities.pwm
            && capabilities.adc
            && capabilities.dac
            && capabilities.i2c
            && capabilities.spi
            && capabilities.uart
            && capabilities.can
            && capabilities.rtc
            && capabilities.watchdog
            && capabilities.storage
            && capabilities.network_tcp
            && capabilities.network_udp
        {
            &BLINK_MVP_WITH_PWM_ADC_DAC_I2C_SPI_UART_CAN_RTC_WATCHDOG_STORAGE_NETWORK_TCP_UDP_AND_LED_MATRIX_CAPABILITIES
        } else if target.supports_wifi_module
            && target.supports_led_matrix
            && capabilities.pwm
            && capabilities.adc
            && capabilities.dac
            && capabilities.i2c
            && capabilities.spi
            && capabilities.uart
            && capabilities.can
            && capabilities.rtc
            && capabilities.watchdog
            && capabilities.storage
            && capabilities.network_tcp
        {
            &BLINK_MVP_WITH_PWM_ADC_DAC_I2C_SPI_UART_CAN_RTC_WATCHDOG_STORAGE_NETWORK_TCP_AND_LED_MATRIX_CAPABILITIES
        } else if target.supports_led_matrix
            && capabilities.pwm
            && capabilities.adc
            && capabilities.dac
            && capabilities.i2c
            && capabilities.spi
            && capabilities.uart
            && capabilities.can
            && capabilities.rtc
            && capabilities.watchdog
            && capabilities.storage
        {
            &BLINK_MVP_WITH_PWM_ADC_DAC_I2C_SPI_UART_CAN_RTC_WATCHDOG_STORAGE_AND_LED_MATRIX_CAPABILITIES
        } else if target.supports_led_matrix
            && capabilities.pwm
            && capabilities.adc
            && capabilities.dac
            && capabilities.i2c
            && capabilities.spi
            && capabilities.uart
            && capabilities.can
            && capabilities.rtc
            && capabilities.watchdog
        {
            &BLINK_MVP_WITH_PWM_ADC_DAC_I2C_SPI_UART_CAN_RTC_WATCHDOG_AND_LED_MATRIX_CAPABILITIES
        } else if target.supports_led_matrix
            && capabilities.pwm
            && capabilities.adc
            && capabilities.dac
            && capabilities.i2c
            && capabilities.spi
            && capabilities.uart
            && capabilities.can
            && capabilities.rtc
        {
            &BLINK_MVP_WITH_PWM_ADC_DAC_I2C_SPI_UART_CAN_RTC_AND_LED_MATRIX_CAPABILITIES
        } else if target.supports_led_matrix
            && capabilities.pwm
            && capabilities.adc
            && capabilities.dac
            && capabilities.i2c
            && capabilities.spi
            && capabilities.uart
            && capabilities.can
        {
            &BLINK_MVP_WITH_PWM_ADC_DAC_I2C_SPI_UART_CAN_AND_LED_MATRIX_CAPABILITIES
        } else if target.supports_led_matrix
            && capabilities.pwm
            && capabilities.adc
            && capabilities.dac
            && capabilities.i2c
            && capabilities.spi
            && capabilities.uart
            && capabilities.rtc
        {
            &BLINK_MVP_WITH_PWM_ADC_DAC_I2C_SPI_UART_RTC_AND_LED_MATRIX_CAPABILITIES
        } else if target.supports_led_matrix
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
            && capabilities.can
            && capabilities.rtc
            && capabilities.watchdog
            && capabilities.storage
        {
            &BLINK_MVP_WITH_PWM_ADC_DAC_I2C_SPI_UART_CAN_RTC_WATCHDOG_AND_STORAGE_CAPABILITIES
        } else if capabilities.pwm
            && capabilities.adc
            && capabilities.dac
            && capabilities.i2c
            && capabilities.spi
            && capabilities.uart
            && capabilities.can
            && capabilities.rtc
            && capabilities.watchdog
        {
            &BLINK_MVP_WITH_PWM_ADC_DAC_I2C_SPI_UART_CAN_RTC_AND_WATCHDOG_CAPABILITIES
        } else if capabilities.pwm
            && capabilities.adc
            && capabilities.dac
            && capabilities.i2c
            && capabilities.spi
            && capabilities.uart
            && capabilities.can
            && capabilities.rtc
        {
            &BLINK_MVP_WITH_PWM_ADC_DAC_I2C_SPI_UART_CAN_AND_RTC_CAPABILITIES
        } else if capabilities.pwm
            && capabilities.adc
            && capabilities.dac
            && capabilities.i2c
            && capabilities.spi
            && capabilities.uart
            && capabilities.can
        {
            &BLINK_MVP_WITH_PWM_ADC_DAC_I2C_SPI_UART_AND_CAN_CAPABILITIES
        } else if capabilities.pwm
            && capabilities.adc
            && capabilities.dac
            && capabilities.i2c
            && capabilities.spi
            && capabilities.uart
            && capabilities.rtc
        {
            &BLINK_MVP_WITH_PWM_ADC_DAC_I2C_SPI_UART_AND_RTC_CAPABILITIES
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
        UartWrite(u8, u8),
        UartRead(u8),
        CanOpen(u8),
        CanWrite(u8, u8),
        CanRead(u8),
        RtcNow,
        RtcSet(u32),
        WatchdogConfigure(u32),
        WatchdogKick,
        StorageWrite(u8, u16, Vec<u8>),
        StorageRead(u8, u16, u8),
        NetworkTcpOpen(u8, u32, u16),
        NetworkTcpWrite(u8, u8),
        NetworkTcpRead(u8),
        NetworkTcpClose(u8),
        NetworkUdpOpen(u8, u32, u16),
        NetworkUdpWrite(u8, u8),
        NetworkUdpRead(u8),
        NetworkUdpClose(u8),
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

        fn supports_can(&self) -> bool {
            true
        }

        fn supports_rtc(&self) -> bool {
            true
        }

        fn supports_watchdog(&self) -> bool {
            true
        }

        fn supports_storage(&self) -> bool {
            true
        }

        fn supports_network_tcp(&self) -> bool {
            true
        }

        fn supports_network_udp(&self) -> bool {
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

        fn write_uart(&mut self, bus: u8, byte: u8) -> Result<(), HalError> {
            self.events.push(Event::UartWrite(bus, byte));
            Ok(())
        }

        fn read_uart(&mut self, bus: u8) -> Result<u8, HalError> {
            self.events.push(Event::UartRead(bus));
            Ok(0x5a)
        }

        fn open_can(&mut self, bus: u8) -> Result<u32, HalError> {
            self.events.push(Event::CanOpen(bus));
            Ok(0x4_2000 | bus as u32)
        }

        fn write_can(&mut self, bus: u8, byte: u8) -> Result<(), HalError> {
            self.events.push(Event::CanWrite(bus, byte));
            Ok(())
        }

        fn read_can(&mut self, bus: u8) -> Result<u8, HalError> {
            self.events.push(Event::CanRead(bus));
            Ok(0x5a)
        }

        fn rtc_now(&mut self) -> Result<u32, HalError> {
            self.events.push(Event::RtcNow);
            Ok(1_700_000_000)
        }

        fn rtc_set(&mut self, epoch_seconds: u32) -> Result<(), HalError> {
            self.events.push(Event::RtcSet(epoch_seconds));
            Ok(())
        }

        fn watchdog_configure(&mut self, timeout_ms: u32) -> Result<(), HalError> {
            self.events.push(Event::WatchdogConfigure(timeout_ms));
            Ok(())
        }

        fn watchdog_kick(&mut self) -> Result<(), HalError> {
            self.events.push(Event::WatchdogKick);
            Ok(())
        }

        fn storage_write(&mut self, region: u8, offset: u16, bytes: &[u8]) -> Result<(), HalError> {
            self.events
                .push(Event::StorageWrite(region, offset, bytes.to_vec()));
            Ok(())
        }

        fn storage_read(
            &mut self,
            region: u8,
            offset: u16,
            len: u8,
        ) -> Result<ByteBuffer, HalError> {
            self.events.push(Event::StorageRead(region, offset, len));
            let pattern = [0xde, 0xad, 0xbe, 0xef];
            let mut bytes = vec![0u8; len as usize];
            for (index, byte) in bytes.iter_mut().enumerate() {
                *byte = pattern[index % pattern.len()];
            }
            ByteBuffer::from_slice(&bytes).map_err(|_| HalError::UnsupportedMode)
        }

        fn open_network_tcp(
            &mut self,
            interface: u8,
            remote_ipv4: u32,
            remote_port: u16,
        ) -> Result<u32, HalError> {
            self.events
                .push(Event::NetworkTcpOpen(interface, remote_ipv4, remote_port));
            Ok(0x5_2002)
        }

        fn write_network_tcp(&mut self, socket: u8, byte: u8) -> Result<(), HalError> {
            self.events.push(Event::NetworkTcpWrite(socket, byte));
            Ok(())
        }

        fn read_network_tcp(&mut self, socket: u8) -> Result<u8, HalError> {
            self.events.push(Event::NetworkTcpRead(socket));
            Ok(0x42)
        }

        fn close_network_tcp(&mut self, socket: u8) -> Result<(), HalError> {
            self.events.push(Event::NetworkTcpClose(socket));
            Ok(())
        }

        fn open_network_udp(
            &mut self,
            interface: u8,
            remote_ipv4: u32,
            remote_port: u16,
        ) -> Result<u32, HalError> {
            self.events
                .push(Event::NetworkUdpOpen(interface, remote_ipv4, remote_port));
            Ok(0x6_2003)
        }

        fn write_network_udp(&mut self, socket: u8, byte: u8) -> Result<(), HalError> {
            self.events.push(Event::NetworkUdpWrite(socket, byte));
            Ok(())
        }

        fn read_network_udp(&mut self, socket: u8) -> Result<u8, HalError> {
            self.events.push(Event::NetworkUdpRead(socket));
            Ok(0x43)
        }

        fn close_network_udp(&mut self, socket: u8) -> Result<(), HalError> {
            self.events.push(Event::NetworkUdpClose(socket));
            Ok(())
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
    fn knows_uno_r4_can_buses() {
        assert_eq!(UNO_R4_MINIMA.can_buses, &[UNO_R4_CAN_BUS]);
        assert_eq!(UNO_R4_WIFI.can_buses, &[UNO_R4_CAN_BUS]);

        let can = UNO_R4_WIFI.can_buses[0];
        assert_eq!(can.name, "CAN0");
        assert_eq!(can.tx_pin, 10);
        assert_eq!(can.rx_pin, 13);
        assert_eq!(can.controller, "RA4M1 CAN0");
        assert!(can.notes.contains("SPI"));
        assert!(can.notes.contains("onboard LED"));
    }

    #[test]
    fn knows_uno_r4_rtc() {
        assert_eq!(UNO_R4_MINIMA.rtc, Some(UNO_R4_RTC));
        assert_eq!(UNO_R4_WIFI.rtc, Some(UNO_R4_RTC));

        let rtc = UNO_R4_WIFI.rtc.unwrap();
        assert_eq!(rtc.instance, 0);
        assert_eq!(rtc.name, "RTC");
        assert_eq!(rtc.peripheral, "RA4M1 RTC");
        assert!(rtc.notes.contains("real-time clock"));
    }

    #[test]
    fn knows_uno_r4_watchdog() {
        assert_eq!(UNO_R4_MINIMA.watchdog, Some(UNO_R4_WATCHDOG));
        assert_eq!(UNO_R4_WIFI.watchdog, Some(UNO_R4_WATCHDOG));

        let watchdog = UNO_R4_WIFI.watchdog.unwrap();
        assert_eq!(watchdog.instance, 0);
        assert_eq!(watchdog.name, "WDT");
        assert_eq!(watchdog.peripheral, "RA4M1 WDT");
        assert!(watchdog.notes.contains("watchdog timer"));
    }

    #[test]
    fn knows_uno_r4_storage_regions() {
        assert_eq!(UNO_R4_MINIMA.storage_regions, &UNO_R4_STORAGE_REGIONS);
        assert_eq!(UNO_R4_WIFI.storage_regions, &UNO_R4_STORAGE_REGIONS);

        let storage = UNO_R4_WIFI.storage_regions[0];
        assert_eq!(storage.region, 0);
        assert_eq!(storage.name, "EEPROM emulation");
        assert_eq!(storage.kind, "data flash");
        assert_eq!(storage.bytes, UNO_R4_DATA_FLASH_BYTES);
        assert!(storage.notes.contains("storage.write"));
        assert!(storage.notes.contains("storage.read"));
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
        assert!(descriptor.supports_store_program);
        assert_eq!(
            descriptor.capabilities.len(),
            BLINK_MVP_WITH_PWM_ADC_DAC_I2C_SPI_UART_CAN_RTC_WATCHDOG_STORAGE_NETWORK_TCP_UDP_AND_LED_MATRIX_CAPABILITIES
                .len()
        );
        assert!(descriptor
            .capabilities
            .iter()
            .any(
                |capability| capability.id == board_vm_protocol::CAP_PROGRAM_STORE
                    && capability.name == "program.store"
            ));
        assert!(descriptor.capabilities.iter().any(|capability| {
            capability.id == board_vm_ir::CAP_NETWORK_TCP_OPEN
                && capability.name == "network.tcp.open"
        }));
        assert!(descriptor.capabilities.iter().any(|capability| {
            capability.id == board_vm_ir::CAP_NETWORK_UDP_OPEN
                && capability.name == "network.udp.open"
        }));
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
        assert!(UNO_R4_WIFI
            .capabilities
            .supports(board_vm_ir::CAP_UART_WRITE));
        assert!(UNO_R4_WIFI
            .capabilities
            .supports(board_vm_ir::CAP_UART_READ));
        assert!(UNO_R4_WIFI.capabilities.supports(board_vm_ir::CAP_CAN_OPEN));
        assert!(UNO_R4_WIFI
            .capabilities
            .supports(board_vm_ir::CAP_CAN_WRITE));
        assert!(UNO_R4_WIFI.capabilities.supports(board_vm_ir::CAP_CAN_READ));
        assert!(UNO_R4_WIFI.capabilities.supports(board_vm_ir::CAP_RTC_NOW));
        assert!(UNO_R4_WIFI.capabilities.supports(board_vm_ir::CAP_RTC_SET));
        assert!(UNO_R4_WIFI
            .capabilities
            .supports(board_vm_ir::CAP_WATCHDOG_CONFIGURE));
        assert!(UNO_R4_WIFI
            .capabilities
            .supports(board_vm_ir::CAP_WATCHDOG_KICK));
        assert!(UNO_R4_WIFI
            .capabilities
            .supports(board_vm_ir::CAP_STORAGE_WRITE));
        assert!(UNO_R4_WIFI
            .capabilities
            .supports(board_vm_ir::CAP_STORAGE_READ));
        assert!(UNO_R4_WIFI
            .capabilities
            .supports(board_vm_ir::CAP_NETWORK_TCP_OPEN));
        assert!(UNO_R4_WIFI
            .capabilities
            .supports(board_vm_ir::CAP_NETWORK_TCP_WRITE));
        assert!(UNO_R4_WIFI
            .capabilities
            .supports(board_vm_ir::CAP_NETWORK_TCP_READ));
        assert!(UNO_R4_WIFI
            .capabilities
            .supports(board_vm_ir::CAP_NETWORK_TCP_CLOSE));
        assert!(UNO_R4_WIFI
            .capabilities
            .supports(board_vm_ir::CAP_NETWORK_UDP_OPEN));
        assert!(UNO_R4_WIFI
            .capabilities
            .supports(board_vm_ir::CAP_NETWORK_UDP_WRITE));
        assert!(UNO_R4_WIFI
            .capabilities
            .supports(board_vm_ir::CAP_NETWORK_UDP_READ));
        assert!(UNO_R4_WIFI
            .capabilities
            .supports(board_vm_ir::CAP_NETWORK_UDP_CLOSE));
        assert!(!UNO_R4_MINIMA
            .capabilities
            .supports(board_vm_ir::CAP_NETWORK_TCP_OPEN));
        assert!(!UNO_R4_MINIMA
            .capabilities
            .supports(board_vm_ir::CAP_NETWORK_UDP_OPEN));
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
        assert!(UNO_R4_MINIMA
            .capabilities
            .supports(board_vm_ir::CAP_UART_WRITE));
        assert!(UNO_R4_MINIMA
            .capabilities
            .supports(board_vm_ir::CAP_UART_READ));
        assert!(UNO_R4_MINIMA
            .capabilities
            .supports(board_vm_ir::CAP_CAN_OPEN));
        assert!(UNO_R4_MINIMA
            .capabilities
            .supports(board_vm_ir::CAP_CAN_WRITE));
        assert!(UNO_R4_MINIMA
            .capabilities
            .supports(board_vm_ir::CAP_CAN_READ));
        assert!(UNO_R4_MINIMA
            .capabilities
            .supports(board_vm_ir::CAP_RTC_NOW));
        assert!(UNO_R4_MINIMA
            .capabilities
            .supports(board_vm_ir::CAP_RTC_SET));
        assert!(UNO_R4_MINIMA
            .capabilities
            .supports(board_vm_ir::CAP_WATCHDOG_CONFIGURE));
        assert!(UNO_R4_MINIMA
            .capabilities
            .supports(board_vm_ir::CAP_WATCHDOG_KICK));
        assert!(UNO_R4_MINIMA
            .capabilities
            .supports(board_vm_ir::CAP_STORAGE_WRITE));
        assert!(UNO_R4_MINIMA
            .capabilities
            .supports(board_vm_ir::CAP_STORAGE_READ));
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
    fn uart_write_runs_through_bus_metadata() {
        let mut board = UnoR4Board::wifi(FakeBackend::new());

        board.uart_write(0x3_2002, 0xa5).unwrap();

        assert_eq!(board.backend().events, vec![Event::UartWrite(2, 0xa5)]);
    }

    #[test]
    fn uart_write_rejects_unknown_bus() {
        let mut board = UnoR4Board::minima(FakeBackend::new());

        assert_eq!(board.uart_write(0x3_2001, 0xa5), Err(HalError::InvalidPin));
        assert_eq!(board.uart_write(0x3_2099, 0xa5), Err(HalError::InvalidPin));
    }

    #[test]
    fn uart_read_runs_through_bus_metadata() {
        let mut board = UnoR4Board::wifi(FakeBackend::new());

        assert_eq!(board.uart_read(0x3_2002).unwrap(), 0x5a);
        assert_eq!(board.backend().events, vec![Event::UartRead(2)]);
    }

    #[test]
    fn uart_read_rejects_unknown_bus() {
        let mut board = UnoR4Board::minima(FakeBackend::new());

        assert_eq!(board.uart_read(0x3_2001), Err(HalError::InvalidPin));
        assert_eq!(board.uart_read(0x3_2099), Err(HalError::InvalidPin));
    }

    #[test]
    fn can_open_runs_through_bus_metadata() {
        let mut board = UnoR4Board::wifi(FakeBackend::new());

        assert_eq!(board.can_open(0).unwrap(), 0x4_2000);
        assert_eq!(board.backend().events, vec![Event::CanOpen(0)]);
    }

    #[test]
    fn can_open_rejects_unknown_bus() {
        let mut board = UnoR4Board::minima(FakeBackend::new());

        assert_eq!(board.can_open(1), Err(HalError::InvalidPin));
        assert_eq!(board.can_open(99), Err(HalError::InvalidPin));
    }

    #[test]
    fn can_write_runs_through_bus_metadata() {
        let mut board = UnoR4Board::wifi(FakeBackend::new());

        board.can_write(0x4_2000, 0xa5).unwrap();

        assert_eq!(board.backend().events, vec![Event::CanWrite(0, 0xa5)]);
    }

    #[test]
    fn can_write_rejects_unknown_bus() {
        let mut board = UnoR4Board::minima(FakeBackend::new());

        assert_eq!(board.can_write(0x4_2001, 0xa5), Err(HalError::InvalidPin));
        assert_eq!(board.can_write(0x4_2099, 0xa5), Err(HalError::InvalidPin));
    }

    #[test]
    fn can_read_runs_through_bus_metadata() {
        let mut board = UnoR4Board::wifi(FakeBackend::new());

        assert_eq!(board.can_read(0x4_2000).unwrap(), 0x5a);
        assert_eq!(board.backend().events, vec![Event::CanRead(0)]);
    }

    #[test]
    fn can_read_rejects_unknown_bus() {
        let mut board = UnoR4Board::minima(FakeBackend::new());

        assert_eq!(board.can_read(0x4_2001), Err(HalError::InvalidPin));
        assert_eq!(board.can_read(0x4_2099), Err(HalError::InvalidPin));
    }

    #[test]
    fn rtc_now_runs_through_backend() {
        let mut board = UnoR4Board::wifi(FakeBackend::new());

        assert_eq!(board.rtc_now().unwrap(), 1_700_000_000);
        assert_eq!(board.backend().events, vec![Event::RtcNow]);
    }

    #[test]
    fn rtc_set_runs_through_backend() {
        let mut board = UnoR4Board::wifi(FakeBackend::new());

        board.rtc_set(1_700_000_001).unwrap();

        assert_eq!(board.backend().events, vec![Event::RtcSet(1_700_000_001)]);
    }

    #[test]
    fn watchdog_configure_runs_through_backend() {
        let mut board = UnoR4Board::wifi(FakeBackend::new());

        board.watchdog_configure(2_000).unwrap();

        assert_eq!(
            board.backend().events,
            vec![Event::WatchdogConfigure(2_000)]
        );
    }

    #[test]
    fn watchdog_kick_runs_through_backend() {
        let mut board = UnoR4Board::wifi(FakeBackend::new());

        board.watchdog_kick().unwrap();

        assert_eq!(board.backend().events, vec![Event::WatchdogKick]);
    }

    #[test]
    fn storage_write_runs_through_region_metadata() {
        let mut board = UnoR4Board::wifi(FakeBackend::new());

        board.storage_write(0, 0x0010, &[0xaa, 0x55]).unwrap();

        assert_eq!(
            board.backend().events,
            vec![Event::StorageWrite(0, 0x0010, vec![0xaa, 0x55])]
        );
    }

    #[test]
    fn storage_read_runs_through_region_metadata() {
        let mut board = UnoR4Board::wifi(FakeBackend::new());

        let bytes = board.storage_read(0, 0x0010, 2).unwrap();

        assert_eq!(bytes.as_slice(), &[0xde, 0xad]);
        assert_eq!(
            board.backend().events,
            vec![Event::StorageRead(0, 0x0010, 2)]
        );
    }

    #[test]
    fn network_tcp_runs_through_wifi_backend() {
        let mut board = UnoR4Board::wifi(FakeBackend::new());

        let token = board.network_tcp_open(0, 0xc0a8_012a, 8080).unwrap();
        board.network_tcp_write(token, 0x41).unwrap();
        let byte = board.network_tcp_read(token).unwrap();
        board.network_tcp_close(token).unwrap();

        assert_eq!(byte, 0x42);
        assert_eq!(
            board.backend().events,
            vec![
                Event::NetworkTcpOpen(0, 0xc0a8_012a, 8080),
                Event::NetworkTcpWrite(2, 0x41),
                Event::NetworkTcpRead(2),
                Event::NetworkTcpClose(2),
            ]
        );
    }

    #[test]
    fn network_udp_runs_through_wifi_backend() {
        let mut board = UnoR4Board::wifi(FakeBackend::new());

        let token = board.network_udp_open(0, 0xc0a8_012a, 0x1235).unwrap();
        board.network_udp_write(token, 0x41).unwrap();
        let byte = board.network_udp_read(token).unwrap();
        board.network_udp_close(token).unwrap();

        assert_eq!(byte, 0x43);
        assert_eq!(
            board.backend().events,
            vec![
                Event::NetworkUdpOpen(0, 0xc0a8_012a, 0x1235),
                Event::NetworkUdpWrite(3, 0x41),
                Event::NetworkUdpRead(3),
                Event::NetworkUdpClose(3),
            ]
        );
    }

    #[test]
    fn network_sockets_reject_non_wifi_target_or_unknown_interface() {
        let mut minima = UnoR4Board::minima(FakeBackend::new());
        assert_eq!(
            minima.network_tcp_open(0, 0xc0a8_012a, 8080),
            Err(HalError::UnsupportedMode)
        );
        assert_eq!(
            minima.network_udp_open(0, 0xc0a8_012a, 0x1235),
            Err(HalError::UnsupportedMode)
        );

        let mut wifi = UnoR4Board::wifi(FakeBackend::new());
        assert_eq!(
            wifi.network_tcp_open(1, 0xc0a8_012a, 8080),
            Err(HalError::InvalidPin)
        );
        assert_eq!(
            wifi.network_udp_open(1, 0xc0a8_012a, 0x1235),
            Err(HalError::InvalidPin)
        );
    }

    #[test]
    fn storage_access_rejects_unknown_region_or_out_of_bounds_range() {
        let mut board = UnoR4Board::wifi(FakeBackend::new());

        assert_eq!(
            board.storage_write(1, 0, &[0xaa]),
            Err(HalError::InvalidPin)
        );
        assert_eq!(
            board.storage_read(0, UNO_R4_DATA_FLASH_BYTES as u16, 1),
            Err(HalError::InvalidPin)
        );
        assert_eq!(
            board.storage_write(0, 0, &[0xaa; u8::MAX as usize + 1]),
            Err(HalError::InvalidPin)
        );
    }

    #[test]
    fn store_program_writes_header_and_module_chunks_to_storage_region() {
        let mut board = UnoR4Board::wifi(FakeBackend::new());
        let mut module = vec![0xab; 300];
        module[0] = b'B';
        module[1] = b'V';
        module[2] = b'M';
        module[3] = b'1';

        board.store_program(7, 0, 2, &module).unwrap();

        let events = &board.backend().events;
        assert_eq!(events.len(), 3);
        let Event::StorageWrite(region, offset, header) = &events[0] else {
            panic!("expected program-store header write");
        };
        assert_eq!((*region, *offset), (0, 0));
        assert_eq!(&header[0..4], &UNO_R4_PROGRAM_STORE_MAGIC);
        assert_eq!(header[4], UNO_R4_PROGRAM_STORE_LAYOUT_VERSION);
        assert_eq!(header[5], 0);
        assert_eq!(header[6], 2);
        assert_eq!(header[7], UNO_R4_PROGRAM_STORE_FORMAT_BVM_MODULE);
        assert_eq!(u16::from_le_bytes([header[8], header[9]]), 7);
        assert_eq!(
            u32::from_le_bytes([header[12], header[13], header[14], header[15]]),
            module.len() as u32
        );
        assert_eq!(
            u32::from_le_bytes([header[16], header[17], header[18], header[19]]),
            crc32_ieee(&module)
        );
        assert_eq!(
            events[1],
            Event::StorageWrite(
                0,
                UNO_R4_PROGRAM_STORE_HEADER_BYTES as u16,
                module[..255].to_vec()
            )
        );
        assert_eq!(
            events[2],
            Event::StorageWrite(
                0,
                UNO_R4_PROGRAM_STORE_HEADER_BYTES as u16 + 255,
                module[255..].to_vec()
            )
        );
    }

    #[test]
    fn store_program_rejects_nonzero_slot_for_initial_uno_r4_layout() {
        let mut board = UnoR4Board::wifi(FakeBackend::new());

        assert_eq!(
            board.store_program(7, 1, 2, &[0x42]),
            Err(HalError::InvalidPin)
        );
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
        let mut device_payload = [0u8; 1024];
        let mut response = [0u8; 1280];

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
