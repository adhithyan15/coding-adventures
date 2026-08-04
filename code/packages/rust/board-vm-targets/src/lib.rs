#![no_std]
// The wide argument lists mirror the target/instruction ABI shape 1:1;
// collapsing them into a struct would obscure that mapping.
#![allow(clippy::too_many_arguments)]

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoardFamily {
    Arduino,
    ArduinoUnoR4,
    Esp32,
    RaspberryPiPico,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OnboardLed {
    Gpio(u8),
    WirelessChipGpio(u8),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WirelessTransport {
    Wifi,
    BluetoothLe,
    BluetoothClassic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WirelessInterfaceInfo {
    pub transport: WirelessTransport,
    pub chip: &'static str,
    pub command_transport: bool,
    pub ota_update: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkProtocol {
    Ipv4,
    Tcp,
    Udp,
    Dns,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NetworkInterfaceInfo {
    pub interface: u8,
    pub name: &'static str,
    pub transport: WirelessTransport,
    pub chip: &'static str,
    pub protocols: &'static [NetworkProtocol],
    pub max_sockets: u8,
    pub notes: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LedMatrixInfo {
    pub rows: u8,
    pub columns: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct I2cBusInfo {
    pub bus: u8,
    pub name: &'static str,
    pub sda_pin: u8,
    pub scl_pin: u8,
    pub qwiic: bool,
    pub notes: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct I2cConnectorInfo {
    pub bus: u8,
    pub name: &'static str,
    pub connector: &'static str,
    pub arduino_object: &'static str,
    pub controller: &'static str,
    pub notes: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpiBusInfo {
    pub bus: u8,
    pub name: &'static str,
    pub copi_pin: u8,
    pub cipo_pin: u8,
    pub sck_pin: u8,
    pub default_cs_pin: u8,
    pub notes: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UartBusInfo {
    pub bus: u8,
    pub name: &'static str,
    pub tx_pin: u8,
    pub rx_pin: u8,
    pub arduino_uart: u8,
    pub internal: bool,
    pub notes: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UsbInterfaceInfo {
    pub interface: u8,
    pub name: &'static str,
    pub controller: &'static str,
    pub class: &'static str,
    pub native: bool,
    pub upload: bool,
    pub command_transport: bool,
    pub notes: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CanBusInfo {
    pub bus: u8,
    pub name: &'static str,
    pub tx_pin: u8,
    pub rx_pin: u8,
    pub controller: &'static str,
    pub notes: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RtcInfo {
    pub instance: u8,
    pub name: &'static str,
    pub peripheral: &'static str,
    pub notes: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WatchdogInfo {
    pub instance: u8,
    pub name: &'static str,
    pub peripheral: &'static str,
    pub notes: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StorageRegionInfo {
    pub region: u8,
    pub name: &'static str,
    pub kind: &'static str,
    pub bytes: u32,
    pub notes: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UploadAdapter {
    ArduinoCli,
    EspRomSerial,
    PicoUf2MassStorage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UploadImageFormat {
    ArduinoCliBuildOutput,
    EspFlashImage,
    Uf2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UploadTransport {
    Serial,
    MassStorage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UploadPortHint {
    UsbSerialBridge,
    NativeUsb,
    ExternalSerialAdapter,
    EspRomSerial,
    MassStorageBootloader,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UploadResetMethod {
    ArduinoBoardPackage,
    EspRomBootPins,
    PicoBootsel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UploadInfo {
    pub adapter: UploadAdapter,
    pub image_format: UploadImageFormat,
    pub transport: UploadTransport,
    pub reset_method: UploadResetMethod,
    pub port_hint: Option<UploadPortHint>,
    pub command: &'static str,
    pub platform_id: Option<&'static str>,
    pub fqbn: Option<&'static str>,
    pub notes: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DigitalPinInfo {
    pub pin: u8,
    pub label: &'static str,
    pub supports_input: bool,
    pub supports_output: bool,
    pub supports_pullup: bool,
    pub supports_pulldown: bool,
    pub supports_adc: bool,
    pub supports_pwm: bool,
    pub supports_dac: bool,
    pub supports_touch: bool,
    pub supports_interrupt: bool,
    pub boot_strap: bool,
    pub notes: &'static str,
}

impl DigitalPinInfo {
    const fn placeholder() -> Self {
        Self {
            pin: 0,
            label: "",
            supports_input: false,
            supports_output: false,
            supports_pullup: false,
            supports_pulldown: false,
            supports_adc: false,
            supports_pwm: false,
            supports_dac: false,
            supports_touch: false,
            supports_interrupt: false,
            boot_strap: false,
            notes: "",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BoardTargetInfo {
    pub board_id: &'static str,
    pub display_name: &'static str,
    pub family: BoardFamily,
    pub runtime_id: &'static str,
    pub mcu: &'static str,
    pub core: &'static str,
    pub rust_target: &'static str,
    pub clock_hz: u32,
    pub operating_voltage_mv: u16,
    pub onboard_led: Option<OnboardLed>,
    pub led_matrix: Option<LedMatrixInfo>,
    pub digital_pin_count: usize,
    pub digital_pins: &'static [DigitalPinInfo],
    pub i2c_buses: &'static [I2cBusInfo],
    pub i2c_connectors: &'static [I2cConnectorInfo],
    pub spi_buses: &'static [SpiBusInfo],
    pub uart_buses: &'static [UartBusInfo],
    pub usb_interfaces: &'static [UsbInterfaceInfo],
    pub can_buses: &'static [CanBusInfo],
    pub rtc: Option<RtcInfo>,
    pub watchdog: Option<WatchdogInfo>,
    pub storage_regions: &'static [StorageRegionInfo],
    pub wireless: &'static [WirelessInterfaceInfo],
    pub network_interfaces: &'static [NetworkInterfaceInfo],
    pub upload: Option<UploadInfo>,
    pub capabilities: &'static [&'static str],
}

pub const BLINK_MVP_CAPABILITIES: [&str; 8] = [
    "transport.serial",
    "gpio.open",
    "gpio.write",
    "gpio.read",
    "gpio.close",
    "time.sleep_ms",
    "time.now_ms",
    "program.ram_exec",
];

pub const UNO_R4_MINIMA_CAPABILITIES: [&str; 33] = [
    "transport.serial",
    "gpio.open",
    "gpio.write",
    "gpio.read",
    "gpio.close",
    "time.sleep_ms",
    "time.now_ms",
    "pwm.write",
    "adc.read",
    "dac.write_u12",
    "i2c.open",
    "i2c.write_u8",
    "i2c.read_u8",
    "i2c.write",
    "i2c.read",
    "i2c.transfer",
    "spi.open",
    "spi.transfer",
    "uart.open",
    "uart.write",
    "uart.read",
    "can.open",
    "can.write",
    "can.read",
    "rtc.now",
    "rtc.set",
    "watchdog.configure",
    "watchdog.kick",
    "storage.write",
    "storage.read",
    "storage.size",
    "program.ram_exec",
    "program.store",
];

pub const UNO_R4_WIFI_CAPABILITIES: [&str; 64] = [
    "transport.serial",
    "transport.wifi",
    "transport.bluetooth_le",
    "ota.wifi",
    "network.ipv4",
    "network.tcp",
    "network.udp",
    "network.dns",
    "network.tcp.open",
    "network.tcp.write",
    "network.tcp.read",
    "network.tcp.close",
    "network.tcp.connected",
    "network.tcp.available",
    "network.udp.open",
    "network.udp.write",
    "network.udp.read",
    "network.udp.write_bytes",
    "network.udp.read_bytes",
    "network.udp.available",
    "network.udp.close",
    "network.wifi.associate",
    "network.wifi.disconnect",
    "network.wifi.status",
    "network.dns.resolve",
    "network.dns.set_server",
    "network.dns.query",
    "network.dns.response_ipv4",
    "network.dns.exchange_udp",
    "network.dns.exchange_udp_retry",
    "network.dns.exchange_udp_fallback",
    "gpio.open",
    "gpio.write",
    "gpio.read",
    "gpio.close",
    "time.sleep_ms",
    "time.now_ms",
    "pwm.write",
    "adc.read",
    "dac.write_u12",
    "i2c.open",
    "i2c.write_u8",
    "i2c.read_u8",
    "i2c.write",
    "i2c.read",
    "i2c.transfer",
    "spi.open",
    "spi.transfer",
    "uart.open",
    "uart.write",
    "uart.read",
    "can.open",
    "can.write",
    "can.read",
    "rtc.now",
    "rtc.set",
    "watchdog.configure",
    "watchdog.kick",
    "storage.write",
    "storage.read",
    "storage.size",
    "led_matrix.frame",
    "program.ram_exec",
    "program.store",
];

pub const ESP32_CAPABILITIES: [&str; 12] = [
    "transport.serial",
    "transport.wifi",
    "transport.bluetooth_le",
    "transport.bluetooth_classic",
    "ota.wifi",
    "gpio.open",
    "gpio.write",
    "gpio.read",
    "gpio.close",
    "time.sleep_ms",
    "time.now_ms",
    "program.ram_exec",
];

pub const PICO_W_CAPABILITIES: [&str; 12] = [
    "transport.serial",
    "transport.wifi",
    "transport.bluetooth_le",
    "transport.bluetooth_classic",
    "ota.wifi",
    "gpio.open",
    "gpio.write",
    "gpio.read",
    "gpio.close",
    "time.sleep_ms",
    "time.now_ms",
    "program.ram_exec",
];

pub const UNO_R4_WIFI_WIRELESS: [WirelessInterfaceInfo; 2] = [
    WirelessInterfaceInfo {
        transport: WirelessTransport::Wifi,
        chip: "ESP32-S3 coprocessor",
        command_transport: true,
        ota_update: true,
    },
    WirelessInterfaceInfo {
        transport: WirelessTransport::BluetoothLe,
        chip: "ESP32-S3 coprocessor",
        command_transport: true,
        ota_update: false,
    },
];

pub const UNO_R4_WIFI_LED_MATRIX: LedMatrixInfo = LedMatrixInfo {
    rows: 8,
    columns: 12,
};

pub const UNO_R4_WIFI_NETWORK_PROTOCOLS: [NetworkProtocol; 4] = [
    NetworkProtocol::Ipv4,
    NetworkProtocol::Tcp,
    NetworkProtocol::Udp,
    NetworkProtocol::Dns,
];

pub const UNO_R4_WIFI_NETWORK_INTERFACES: [NetworkInterfaceInfo; 1] = [NetworkInterfaceInfo {
    interface: 0,
    name: "WiFiS3",
    transport: WirelessTransport::Wifi,
    chip: "ESP32-S3 coprocessor",
    protocols: &UNO_R4_WIFI_NETWORK_PROTOCOLS,
    max_sockets: 4,
    notes: "Onboard WiFiS3 network interface; Board VM commands use the shared COBS/CRC wire protocol over TCP endpoints.",
}];

pub const ESP32_WIRELESS: [WirelessInterfaceInfo; 3] = [
    WirelessInterfaceInfo {
        transport: WirelessTransport::Wifi,
        chip: "ESP32-WROOM-32",
        command_transport: true,
        ota_update: true,
    },
    WirelessInterfaceInfo {
        transport: WirelessTransport::BluetoothLe,
        chip: "ESP32-WROOM-32",
        command_transport: true,
        ota_update: false,
    },
    WirelessInterfaceInfo {
        transport: WirelessTransport::BluetoothClassic,
        chip: "ESP32-WROOM-32",
        command_transport: true,
        ota_update: false,
    },
];

pub const PICO_W_WIRELESS: [WirelessInterfaceInfo; 3] = [
    WirelessInterfaceInfo {
        transport: WirelessTransport::Wifi,
        chip: "Infineon CYW43439",
        command_transport: true,
        ota_update: true,
    },
    WirelessInterfaceInfo {
        transport: WirelessTransport::BluetoothLe,
        chip: "Infineon CYW43439",
        command_transport: true,
        ota_update: false,
    },
    WirelessInterfaceInfo {
        transport: WirelessTransport::BluetoothClassic,
        chip: "Infineon CYW43439",
        command_transport: true,
        ota_update: false,
    },
];

pub const ARDUINO_NINA_W102_WIRELESS: [WirelessInterfaceInfo; 2] = [
    WirelessInterfaceInfo {
        transport: WirelessTransport::Wifi,
        chip: "u-blox NINA-W102",
        command_transport: false,
        ota_update: false,
    },
    WirelessInterfaceInfo {
        transport: WirelessTransport::BluetoothLe,
        chip: "u-blox NINA-W102",
        command_transport: false,
        ota_update: false,
    },
];

pub const ARDUINO_NRF52840_WIRELESS: [WirelessInterfaceInfo; 1] = [WirelessInterfaceInfo {
    transport: WirelessTransport::BluetoothLe,
    chip: "nRF52840",
    command_transport: false,
    ota_update: false,
}];

pub const ARDUINO_NANO_ESP32_WIRELESS: [WirelessInterfaceInfo; 2] = [
    WirelessInterfaceInfo {
        transport: WirelessTransport::Wifi,
        chip: "ESP32-S3",
        command_transport: false,
        ota_update: false,
    },
    WirelessInterfaceInfo {
        transport: WirelessTransport::BluetoothLe,
        chip: "ESP32-S3",
        command_transport: false,
        ota_update: false,
    },
];

pub const ARDUINO_WIFI_BLE_MODULE_WIRELESS: [WirelessInterfaceInfo; 2] = [
    WirelessInterfaceInfo {
        transport: WirelessTransport::Wifi,
        chip: "Arduino onboard WiFi/BLE module",
        command_transport: false,
        ota_update: false,
    },
    WirelessInterfaceInfo {
        transport: WirelessTransport::BluetoothLe,
        chip: "Arduino onboard WiFi/BLE module",
        command_transport: false,
        ota_update: false,
    },
];

pub const ARDUINO_ESP32_C3_WIRELESS: [WirelessInterfaceInfo; 2] = [
    WirelessInterfaceInfo {
        transport: WirelessTransport::Wifi,
        chip: "ESP32-C3 module",
        command_transport: false,
        ota_update: false,
    },
    WirelessInterfaceInfo {
        transport: WirelessTransport::BluetoothLe,
        chip: "ESP32-C3 module",
        command_transport: false,
        ota_update: false,
    },
];

pub const ARDUINO_NRF52832_WIRELESS: [WirelessInterfaceInfo; 1] = [WirelessInterfaceInfo {
    transport: WirelessTransport::BluetoothLe,
    chip: "nRF52832",
    command_transport: false,
    ota_update: false,
}];

pub const ARDUINO_NINA_W102_NETWORK_INTERFACES: [NetworkInterfaceInfo; 1] =
    [NetworkInterfaceInfo {
        interface: 0,
        name: "WiFiNINA",
        transport: WirelessTransport::Wifi,
        chip: "u-blox NINA-W102",
        protocols: &UNO_R4_WIFI_NETWORK_PROTOCOLS,
        max_sockets: 0,
        notes: "Physical Arduino WiFiNINA interface metadata only; Board VM network bytecode adapter is not enabled for shared Arduino boards yet.",
    }];

pub const ARDUINO_NANO_ESP32_NETWORK_INTERFACES: [NetworkInterfaceInfo; 1] =
    [NetworkInterfaceInfo {
        interface: 0,
        name: "WiFi",
        transport: WirelessTransport::Wifi,
        chip: "ESP32-S3",
        protocols: &UNO_R4_WIFI_NETWORK_PROTOCOLS,
        max_sockets: 0,
        notes: "Physical Arduino Nano ESP32 WiFi interface metadata only; Board VM network bytecode adapter is not enabled for this board yet.",
    }];

pub const ARDUINO_WIFI_BLE_MODULE_NETWORK_INTERFACES: [NetworkInterfaceInfo; 1] =
    [NetworkInterfaceInfo {
        interface: 0,
        name: "Onboard WiFi",
        transport: WirelessTransport::Wifi,
        chip: "Arduino onboard WiFi/BLE module",
        protocols: &UNO_R4_WIFI_NETWORK_PROTOCOLS,
        max_sockets: 0,
        notes: "Physical Arduino Pro/Nicla/Opta WiFi interface metadata only; Board VM network bytecode adapter is not enabled for this board yet.",
    }];

pub const ARDUINO_ESP32_C3_NETWORK_INTERFACES: [NetworkInterfaceInfo; 1] =
    [NetworkInterfaceInfo {
        interface: 0,
        name: "ESP32-C3 WiFi",
        transport: WirelessTransport::Wifi,
        chip: "ESP32-C3 module",
        protocols: &UNO_R4_WIFI_NETWORK_PROTOCOLS,
        max_sockets: 0,
        notes: "Physical Portenta C33 ESP32-C3 WiFi interface metadata only; Board VM network bytecode adapter is not enabled for this board yet.",
    }];

const fn arduino_cli_upload(
    platform_id: &'static str,
    fqbn: &'static str,
    port_hint: UploadPortHint,
) -> UploadInfo {
    UploadInfo {
        adapter: UploadAdapter::ArduinoCli,
        image_format: UploadImageFormat::ArduinoCliBuildOutput,
        transport: UploadTransport::Serial,
        reset_method: UploadResetMethod::ArduinoBoardPackage,
        port_hint: Some(port_hint),
        command: "arduino-cli upload",
        platform_id: Some(platform_id),
        fqbn: Some(fqbn),
        notes: "Board-specific Arduino CLI platform/FQBN owns bootloader reset, programmer, and final firmware artifact shape.",
    }
}

pub const UNO_R4_MINIMA_UPLOAD: UploadInfo = arduino_cli_upload(
    "arduino:renesas_uno",
    "arduino:renesas_uno:minima",
    UploadPortHint::NativeUsb,
);

pub const UNO_R4_WIFI_UPLOAD: UploadInfo = arduino_cli_upload(
    "arduino:renesas_uno",
    "arduino:renesas_uno:unor4wifi",
    UploadPortHint::NativeUsb,
);

pub const ARDUINO_CLI_UPLOAD: UploadInfo = UploadInfo {
    adapter: UploadAdapter::ArduinoCli,
    image_format: UploadImageFormat::ArduinoCliBuildOutput,
    transport: UploadTransport::Serial,
    reset_method: UploadResetMethod::ArduinoBoardPackage,
    port_hint: None,
    command: "arduino-cli upload",
    platform_id: None,
    fqbn: None,
    notes:
        "Board-specific Arduino CLI package owns bootloader reset, programmer, and final firmware artifact shape.",
};

pub const ESP_ROM_SERIAL_UPLOAD: UploadInfo = UploadInfo {
    adapter: UploadAdapter::EspRomSerial,
    image_format: UploadImageFormat::EspFlashImage,
    transport: UploadTransport::Serial,
    reset_method: UploadResetMethod::EspRomBootPins,
    port_hint: Some(UploadPortHint::EspRomSerial),
    command: "esp-rom",
    platform_id: None,
    fqbn: None,
    notes:
        "Serial ESP ROM flashing path with boot-pin reset and image-layout metadata owned by Rust.",
};

pub const PICO_UF2_UPLOAD: UploadInfo = UploadInfo {
    adapter: UploadAdapter::PicoUf2MassStorage,
    image_format: UploadImageFormat::Uf2,
    transport: UploadTransport::MassStorage,
    reset_method: UploadResetMethod::PicoBootsel,
    port_hint: Some(UploadPortHint::MassStorageBootloader),
    command: "pico-uf2",
    platform_id: None,
    fqbn: None,
    notes: "UF2 copy-to-volume flashing path with BOOTSEL mount discovery owned by Rust.",
};

pub const UNO_R4_DIGITAL_PINS: [DigitalPinInfo; 20] =
    map_uno_r4_digital_pins(board_vm_uno_r4::UNO_R4_DIGITAL_PINS);
pub const UNO_R4_MINIMA_I2C_BUSES: [I2cBusInfo; 1] =
    map_uno_r4_i2c_buses(board_vm_uno_r4::UNO_R4_MINIMA_I2C_BUSES);
pub const UNO_R4_WIFI_I2C_BUSES: [I2cBusInfo; 2] =
    map_uno_r4_i2c_buses(board_vm_uno_r4::UNO_R4_WIFI_I2C_BUSES);
pub const UNO_R4_SPI_BUSES: [SpiBusInfo; 1] =
    map_uno_r4_spi_buses(board_vm_uno_r4::UNO_R4_SPI_BUSES);
pub const UNO_R4_MINIMA_UART_BUSES: [UartBusInfo; 1] =
    map_uno_r4_uart_buses(board_vm_uno_r4::UNO_R4_MINIMA_UART_BUSES);
pub const UNO_R4_WIFI_UART_BUSES: [UartBusInfo; 3] =
    map_uno_r4_uart_buses(board_vm_uno_r4::UNO_R4_WIFI_UART_BUSES);
pub const UNO_R4_CAN_BUSES: [CanBusInfo; 1] =
    map_uno_r4_can_buses(board_vm_uno_r4::UNO_R4_CAN_BUSES);
pub const UNO_R4_RTC: RtcInfo = uno_r4_rtc(board_vm_uno_r4::UNO_R4_RTC);
pub const UNO_R4_WATCHDOG: WatchdogInfo = uno_r4_watchdog(board_vm_uno_r4::UNO_R4_WATCHDOG);
pub const UNO_R4_STORAGE_REGIONS: [StorageRegionInfo; 1] =
    map_uno_r4_storage_regions(board_vm_uno_r4::UNO_R4_STORAGE_REGIONS);

pub const ESP32_DIGITAL_PINS: [DigitalPinInfo; 30] =
    map_esp32_digital_pins(board_vm_esp32::ESP32_DEVKIT_V1_DIGITAL_PINS);

pub const PICO_DIGITAL_PINS: [DigitalPinInfo; 29] =
    map_pico_digital_pins(board_vm_pico::PICO_DIGITAL_PINS);

pub const ARDUINO_UNO_R3_DIGITAL_PINS: [DigitalPinInfo; 20] =
    map_arduino_digital_pins(board_vm_arduino::ARDUINO_STANDARD_DIGITAL_PINS);
pub const ARDUINO_NANO_CLASSIC_DIGITAL_PINS: [DigitalPinInfo; 20] =
    map_arduino_digital_pins(board_vm_arduino::ARDUINO_STANDARD_DIGITAL_PINS);
pub const ARDUINO_PRO_MINI_DIGITAL_PINS: [DigitalPinInfo; 20] =
    map_arduino_digital_pins(board_vm_arduino::ARDUINO_STANDARD_DIGITAL_PINS);
pub const ARDUINO_MEGA_2560_DIGITAL_PINS: [DigitalPinInfo; 70] =
    map_arduino_digital_pins(board_vm_arduino::ARDUINO_MEGA_2560_DIGITAL_PINS);
pub const ARDUINO_LEONARDO_DIGITAL_PINS: [DigitalPinInfo; 24] =
    map_arduino_digital_pins(board_vm_arduino::ARDUINO_LEONARDO_DIGITAL_PINS);
pub const ARDUINO_MICRO_DIGITAL_PINS: [DigitalPinInfo; 24] =
    map_arduino_digital_pins(board_vm_arduino::ARDUINO_LEONARDO_DIGITAL_PINS);
pub const ARDUINO_DUE_DIGITAL_PINS: [DigitalPinInfo; 76] =
    map_arduino_digital_pins(board_vm_arduino::ARDUINO_DUE_DIGITAL_PINS);
pub const ARDUINO_ZERO_DIGITAL_PINS: [DigitalPinInfo; 20] =
    map_arduino_digital_pins(board_vm_arduino::ARDUINO_STANDARD_DIGITAL_PINS);
pub const ARDUINO_MKR_WIFI_1010_DIGITAL_PINS: [DigitalPinInfo; 20] =
    map_arduino_digital_pins(board_vm_arduino::ARDUINO_STANDARD_DIGITAL_PINS);
pub const ARDUINO_NANO_EVERY_DIGITAL_PINS: [DigitalPinInfo; 22] =
    map_arduino_digital_pins(board_vm_arduino::ARDUINO_NANO_EVERY_DIGITAL_PINS);
pub const ARDUINO_NANO_R4_DIGITAL_PINS: [DigitalPinInfo; 22] =
    map_arduino_digital_pins(board_vm_arduino::ARDUINO_NANO_R4_DIGITAL_PINS);
pub const ARDUINO_NANO_33_IOT_DIGITAL_PINS: [DigitalPinInfo; 20] =
    map_arduino_digital_pins(board_vm_arduino::ARDUINO_STANDARD_DIGITAL_PINS);
pub const ARDUINO_NANO_33_BLE_REV2_DIGITAL_PINS: [DigitalPinInfo; 22] =
    map_arduino_digital_pins(board_vm_arduino::ARDUINO_NANO_33_BLE_DIGITAL_PINS);
pub const ARDUINO_NANO_RP2040_CONNECT_DIGITAL_PINS: [DigitalPinInfo; 22] =
    map_arduino_digital_pins(board_vm_arduino::ARDUINO_NANO_RP2040_DIGITAL_PINS);
pub const ARDUINO_NANO_ESP32_DIGITAL_PINS: [DigitalPinInfo; 22] =
    map_arduino_digital_pins(board_vm_arduino::ARDUINO_NANO_ESP32_DIGITAL_PINS);
pub const ARDUINO_GIGA_R1_WIFI_DIGITAL_PINS: [DigitalPinInfo; 76] =
    map_arduino_digital_pins(board_vm_arduino::ARDUINO_GIGA_R1_DIGITAL_PINS);
pub const ARDUINO_PORTENTA_H7_DIGITAL_PINS: [DigitalPinInfo; 80] =
    map_arduino_digital_pins(board_vm_arduino::ARDUINO_PORTENTA_H7_DIGITAL_PINS);
pub const ARDUINO_PORTENTA_H7_LITE_DIGITAL_PINS: [DigitalPinInfo; 80] =
    map_arduino_digital_pins(board_vm_arduino::ARDUINO_PORTENTA_H7_DIGITAL_PINS);
pub const ARDUINO_PORTENTA_H7_LITE_CONNECTED_DIGITAL_PINS: [DigitalPinInfo; 80] =
    map_arduino_digital_pins(board_vm_arduino::ARDUINO_PORTENTA_H7_DIGITAL_PINS);
pub const ARDUINO_PORTENTA_C33_DIGITAL_PINS: [DigitalPinInfo; 7] =
    map_arduino_digital_pins(board_vm_arduino::ARDUINO_PORTENTA_C33_DIGITAL_PINS);
pub const ARDUINO_NICLA_VISION_DIGITAL_PINS: [DigitalPinInfo; 12] =
    map_arduino_digital_pins(board_vm_arduino::ARDUINO_NICLA_DIGITAL_PINS);
pub const ARDUINO_NICLA_SENSE_ME_DIGITAL_PINS: [DigitalPinInfo; 12] =
    map_arduino_digital_pins(board_vm_arduino::ARDUINO_NICLA_DIGITAL_PINS);
pub const ARDUINO_NICLA_VOICE_DIGITAL_PINS: [DigitalPinInfo; 12] =
    map_arduino_digital_pins(board_vm_arduino::ARDUINO_NICLA_DIGITAL_PINS);
pub const ARDUINO_OPTA_LITE_DIGITAL_PINS: [DigitalPinInfo; 12] =
    map_arduino_digital_pins(board_vm_arduino::ARDUINO_OPTA_TERMINAL_PINS);
pub const ARDUINO_OPTA_RS485_DIGITAL_PINS: [DigitalPinInfo; 12] =
    map_arduino_digital_pins(board_vm_arduino::ARDUINO_OPTA_TERMINAL_PINS);
pub const ARDUINO_OPTA_WIFI_DIGITAL_PINS: [DigitalPinInfo; 12] =
    map_arduino_digital_pins(board_vm_arduino::ARDUINO_OPTA_TERMINAL_PINS);

pub const ARDUINO_STANDARD_I2C_BUSES: [I2cBusInfo; 1] = [I2cBusInfo {
    bus: 0,
    name: "Wire",
    sda_pin: 18,
    scl_pin: 19,
    qwiic: false,
    notes: "Arduino A4/A5 I2C header metadata only; shared Arduino runtime does not enable i2c.* bytecode adapters yet.",
}];

pub const ARDUINO_STANDARD_SPI_BUSES: [SpiBusInfo; 1] = [SpiBusInfo {
    bus: 0,
    name: "SPI",
    copi_pin: 11,
    cipo_pin: 12,
    sck_pin: 13,
    default_cs_pin: 10,
    notes: "Arduino D11/D12/D13 SPI header metadata only; shared Arduino runtime does not enable spi.* bytecode adapters yet.",
}];

pub const ARDUINO_STANDARD_UART_BUSES: [UartBusInfo; 1] = [UartBusInfo {
    bus: 0,
    name: "Serial",
    tx_pin: 1,
    rx_pin: 0,
    arduino_uart: 0,
    internal: false,
    notes: "Arduino D1/D0 hardware serial metadata only; upload transport and runtime serial command channel stay adapter-owned.",
}];

pub const ARDUINO_MEGA_2560_I2C_BUSES: [I2cBusInfo; 1] = [I2cBusInfo {
    bus: 0,
    name: "Wire",
    sda_pin: 20,
    scl_pin: 21,
    qwiic: false,
    notes: "Arduino Mega SDA/SCL header metadata only; shared Arduino runtime does not enable i2c.* bytecode adapters yet.",
}];

pub const ARDUINO_MEGA_2560_SPI_BUSES: [SpiBusInfo; 1] = [SpiBusInfo {
    bus: 0,
    name: "SPI",
    copi_pin: 51,
    cipo_pin: 50,
    sck_pin: 52,
    default_cs_pin: 53,
    notes: "Arduino Mega SPI header metadata only; shared Arduino runtime does not enable spi.* bytecode adapters yet.",
}];

pub const ARDUINO_MEGA_2560_UART_BUSES: [UartBusInfo; 4] = [
    UartBusInfo {
        bus: 0,
        name: "Serial",
        tx_pin: 1,
        rx_pin: 0,
        arduino_uart: 0,
        internal: false,
        notes: "Arduino Mega primary D1/D0 serial metadata only; upload transport and runtime serial command channel stay adapter-owned.",
    },
    UartBusInfo {
        bus: 1,
        name: "Serial1",
        tx_pin: 18,
        rx_pin: 19,
        arduino_uart: 1,
        internal: false,
        notes: "Arduino Mega Serial1 metadata only; shared Arduino runtime does not enable uart.* bytecode adapters yet.",
    },
    UartBusInfo {
        bus: 2,
        name: "Serial2",
        tx_pin: 16,
        rx_pin: 17,
        arduino_uart: 2,
        internal: false,
        notes: "Arduino Mega Serial2 metadata only; shared Arduino runtime does not enable uart.* bytecode adapters yet.",
    },
    UartBusInfo {
        bus: 3,
        name: "Serial3",
        tx_pin: 14,
        rx_pin: 15,
        arduino_uart: 3,
        internal: false,
        notes: "Arduino Mega Serial3 metadata only; shared Arduino runtime does not enable uart.* bytecode adapters yet.",
    },
];

pub const ARDUINO_USB_AVR_I2C_BUSES: [I2cBusInfo; 1] = [I2cBusInfo {
    bus: 0,
    name: "Wire",
    sda_pin: 2,
    scl_pin: 3,
    qwiic: false,
    notes: "Arduino ATmega32U4 D2/D3 I2C metadata only; shared Arduino runtime does not enable i2c.* bytecode adapters yet.",
}];

pub const ARDUINO_USB_AVR_SPI_BUSES: [SpiBusInfo; 1] = [SpiBusInfo {
    bus: 0,
    name: "SPI",
    copi_pin: 16,
    cipo_pin: 14,
    sck_pin: 15,
    default_cs_pin: 17,
    notes: "Arduino ATmega32U4 ICSP SPI metadata only; shared Arduino runtime does not enable spi.* bytecode adapters yet.",
}];

pub const ARDUINO_USB_AVR_UART_BUSES: [UartBusInfo; 1] = [UartBusInfo {
    bus: 0,
    name: "Serial1",
    tx_pin: 1,
    rx_pin: 0,
    arduino_uart: 1,
    internal: false,
    notes: "Arduino ATmega32U4 D1/D0 hardware serial metadata only; native USB Serial remains upload/transport metadata, not a GPIO UART.",
}];

pub const ARDUINO_NANO_EVERY_I2C_BUSES: [I2cBusInfo; 1] = [I2cBusInfo {
    bus: 0,
    name: "Wire",
    sda_pin: 18,
    scl_pin: 19,
    qwiic: false,
    notes: "Arduino Nano Every A4/A5 I2C metadata only; shared Arduino runtime does not enable i2c.* bytecode adapters yet.",
}];

pub const ARDUINO_NANO_EVERY_SPI_BUSES: [SpiBusInfo; 1] = [SpiBusInfo {
    bus: 0,
    name: "SPI",
    copi_pin: 11,
    cipo_pin: 12,
    sck_pin: 13,
    default_cs_pin: 10,
    notes: "Arduino Nano Every D11/D12/D13 SPI metadata only; shared Arduino runtime does not enable spi.* bytecode adapters yet.",
}];

pub const ARDUINO_NANO_EVERY_UART_BUSES: [UartBusInfo; 1] = [UartBusInfo {
    bus: 0,
    name: "Serial1",
    tx_pin: 1,
    rx_pin: 0,
    arduino_uart: 1,
    internal: false,
    notes: "Arduino Nano Every D1/D0 Serial1 metadata only; USB serial bridge remains upload/transport metadata, not a separate GPIO UART.",
}];

pub const ARDUINO_NANO_R4_I2C_BUSES: [I2cBusInfo; 1] = [I2cBusInfo {
    bus: 0,
    name: "Wire",
    sda_pin: 18,
    scl_pin: 19,
    qwiic: false,
    notes: "Arduino Nano R4 A4/A5 Wire header metadata only; the separate Qwiic Wire1 connector is exposed as connector-local metadata.",
}];

pub const ARDUINO_NANO_R4_I2C_CONNECTORS: [I2cConnectorInfo; 1] = [I2cConnectorInfo {
    bus: 1,
    name: "Wire1",
    connector: "Qwiic/STEMMA QT",
    arduino_object: "Wire1",
    controller: "RA4M1 IIC0",
    notes: "Arduino Nano R4 connector-local Qwiic/STEMMA QT I2C metadata only; shared Arduino runtime does not enable i2c.* bytecode adapters yet.",
}];

pub const ARDUINO_NANO_R4_SPI_BUSES: [SpiBusInfo; 1] = [SpiBusInfo {
    bus: 0,
    name: "SPI",
    copi_pin: 11,
    cipo_pin: 12,
    sck_pin: 13,
    default_cs_pin: 10,
    notes: "Arduino Nano R4 D11/D12/D13 SPI metadata only; shared Arduino runtime does not enable spi.* bytecode adapters yet.",
}];

pub const ARDUINO_NANO_R4_UART_BUSES: [UartBusInfo; 1] = [UartBusInfo {
    bus: 0,
    name: "Serial1",
    tx_pin: 1,
    rx_pin: 0,
    arduino_uart: 1,
    internal: false,
    notes: "Arduino Nano R4 D1/D0 serial metadata only; native USB remains upload/transport metadata, not a separate GPIO UART.",
}];

pub const ARDUINO_NANO_R4_USB_INTERFACES: [UsbInterfaceInfo; 1] = [UsbInterfaceInfo {
    interface: 0,
    name: "Native USB",
    controller: "RA4M1 native USB",
    class: "CDC serial",
    native: true,
    upload: true,
    command_transport: true,
    notes: "Arduino Nano R4 native USB serial/upload metadata only; this endpoint is not a GPIO UART and board-specific firmware adapters still own deeper USB behavior.",
}];

pub const ARDUINO_NANO_R4_CAN_BUSES: [CanBusInfo; 1] = [CanBusInfo {
    bus: 0,
    name: "CAN0",
    tx_pin: 4,
    rx_pin: 5,
    controller: "RA4M1 CAN0",
    notes: "Arduino Nano R4 D4/D5 CAN metadata only; external transceiver required and shared Arduino runtime does not enable can.* bytecode adapters yet.",
}];

pub const ARDUINO_NANO_R4_RTC: RtcInfo = RtcInfo {
    instance: 0,
    name: "RTC",
    peripheral: "RA4M1 RTC",
    notes: "Arduino Nano R4 RA4M1 real-time clock metadata only; shared Arduino runtime does not enable rtc.* bytecode adapters yet.",
};

const fn map_arduino_digital_pins<const N: usize>(
    source: [board_vm_arduino::DigitalPinDescriptor; N],
) -> [DigitalPinInfo; N] {
    let mut pins = [DigitalPinInfo::placeholder(); N];
    let mut index = 0;
    while index < N {
        pins[index] = arduino_digital_pin(source[index]);
        index += 1;
    }
    pins
}

const fn arduino_digital_pin(pin: board_vm_arduino::DigitalPinDescriptor) -> DigitalPinInfo {
    DigitalPinInfo {
        pin: pin.pin,
        label: pin.label,
        supports_input: pin.supports_input,
        supports_output: pin.supports_output,
        supports_pullup: pin.supports_pullup,
        supports_pulldown: pin.supports_pulldown,
        supports_adc: pin.supports_adc,
        supports_pwm: pin.supports_pwm,
        supports_dac: false,
        supports_touch: false,
        supports_interrupt: pin.supports_input,
        boot_strap: false,
        notes: pin.notes,
    }
}

const fn map_uno_r4_digital_pins<const N: usize>(
    source: [board_vm_uno_r4::DigitalPinDescriptor; N],
) -> [DigitalPinInfo; N] {
    let mut pins = [DigitalPinInfo::placeholder(); N];
    let mut index = 0;
    while index < N {
        pins[index] = uno_r4_digital_pin(source[index]);
        index += 1;
    }
    pins
}

const fn uno_r4_digital_pin(pin: board_vm_uno_r4::DigitalPinDescriptor) -> DigitalPinInfo {
    DigitalPinInfo {
        pin: pin.arduino_pin,
        label: pin.label,
        supports_input: true,
        supports_output: true,
        supports_pullup: true,
        supports_pulldown: false,
        supports_adc: pin.supports_adc,
        supports_pwm: pin.supports_pwm,
        supports_dac: pin.supports_dac,
        supports_touch: false,
        supports_interrupt: pin.supports_interrupt,
        boot_strap: false,
        notes: pin.notes,
    }
}

const fn map_uno_r4_i2c_buses<const N: usize>(
    source: [board_vm_uno_r4::I2cBusDescriptor; N],
) -> [I2cBusInfo; N] {
    let mut buses = [I2cBusInfo {
        bus: 0,
        name: "",
        sda_pin: 0,
        scl_pin: 0,
        qwiic: false,
        notes: "",
    }; N];
    let mut index = 0;
    while index < N {
        let bus = source[index];
        buses[index] = I2cBusInfo {
            bus: bus.bus,
            name: bus.name,
            sda_pin: bus.sda_pin,
            scl_pin: bus.scl_pin,
            qwiic: bus.qwiic,
            notes: bus.notes,
        };
        index += 1;
    }
    buses
}

const fn map_uno_r4_spi_buses<const N: usize>(
    source: [board_vm_uno_r4::SpiBusDescriptor; N],
) -> [SpiBusInfo; N] {
    let mut buses = [SpiBusInfo {
        bus: 0,
        name: "",
        copi_pin: 0,
        cipo_pin: 0,
        sck_pin: 0,
        default_cs_pin: 0,
        notes: "",
    }; N];
    let mut index = 0;
    while index < N {
        let bus = source[index];
        buses[index] = SpiBusInfo {
            bus: bus.bus,
            name: bus.name,
            copi_pin: bus.copi_pin,
            cipo_pin: bus.cipo_pin,
            sck_pin: bus.sck_pin,
            default_cs_pin: bus.default_cs_pin,
            notes: bus.notes,
        };
        index += 1;
    }
    buses
}

const fn map_uno_r4_uart_buses<const N: usize>(
    source: [board_vm_uno_r4::UartBusDescriptor; N],
) -> [UartBusInfo; N] {
    let mut buses = [UartBusInfo {
        bus: 0,
        name: "",
        tx_pin: 0,
        rx_pin: 0,
        arduino_uart: 0,
        internal: false,
        notes: "",
    }; N];
    let mut index = 0;
    while index < N {
        let bus = source[index];
        buses[index] = UartBusInfo {
            bus: bus.bus,
            name: bus.name,
            tx_pin: bus.tx_pin,
            rx_pin: bus.rx_pin,
            arduino_uart: bus.arduino_uart,
            internal: bus.internal,
            notes: bus.notes,
        };
        index += 1;
    }
    buses
}

const fn map_uno_r4_can_buses<const N: usize>(
    source: [board_vm_uno_r4::CanBusDescriptor; N],
) -> [CanBusInfo; N] {
    let mut buses = [CanBusInfo {
        bus: 0,
        name: "",
        tx_pin: 0,
        rx_pin: 0,
        controller: "",
        notes: "",
    }; N];
    let mut index = 0;
    while index < N {
        let bus = source[index];
        buses[index] = CanBusInfo {
            bus: bus.bus,
            name: bus.name,
            tx_pin: bus.tx_pin,
            rx_pin: bus.rx_pin,
            controller: bus.controller,
            notes: bus.notes,
        };
        index += 1;
    }
    buses
}

const fn uno_r4_rtc(rtc: board_vm_uno_r4::RtcDescriptor) -> RtcInfo {
    RtcInfo {
        instance: rtc.instance,
        name: rtc.name,
        peripheral: rtc.peripheral,
        notes: rtc.notes,
    }
}

const fn uno_r4_watchdog(watchdog: board_vm_uno_r4::WatchdogDescriptor) -> WatchdogInfo {
    WatchdogInfo {
        instance: watchdog.instance,
        name: watchdog.name,
        peripheral: watchdog.peripheral,
        notes: watchdog.notes,
    }
}

const fn map_uno_r4_storage_regions<const N: usize>(
    source: [board_vm_uno_r4::StorageRegionDescriptor; N],
) -> [StorageRegionInfo; N] {
    let mut regions = [StorageRegionInfo {
        region: 0,
        name: "",
        kind: "",
        bytes: 0,
        notes: "",
    }; N];
    let mut index = 0;
    while index < N {
        let region = source[index];
        regions[index] = StorageRegionInfo {
            region: region.region,
            name: region.name,
            kind: region.kind,
            bytes: region.bytes,
            notes: region.notes,
        };
        index += 1;
    }
    regions
}

const fn map_esp32_digital_pins<const N: usize>(
    source: [board_vm_esp32::DigitalPinDescriptor; N],
) -> [DigitalPinInfo; N] {
    let mut pins = [DigitalPinInfo::placeholder(); N];
    let mut index = 0;
    while index < N {
        pins[index] = esp32_digital_pin(source[index]);
        index += 1;
    }
    pins
}

const fn esp32_digital_pin(pin: board_vm_esp32::DigitalPinDescriptor) -> DigitalPinInfo {
    DigitalPinInfo {
        pin: pin.gpio,
        label: pin.label,
        supports_input: pin.supports_input,
        supports_output: pin.supports_output,
        supports_pullup: pin.supports_pullup,
        supports_pulldown: pin.supports_pulldown,
        supports_adc: pin.supports_adc,
        supports_pwm: pin.supports_output,
        supports_dac: false,
        supports_touch: pin.supports_touch,
        supports_interrupt: pin.supports_input,
        boot_strap: pin.boot_strap,
        notes: pin.notes,
    }
}

const fn map_pico_digital_pins<const N: usize>(
    source: [board_vm_pico::DigitalPinDescriptor; N],
) -> [DigitalPinInfo; N] {
    let mut pins = [DigitalPinInfo::placeholder(); N];
    let mut index = 0;
    while index < N {
        pins[index] = pico_digital_pin(source[index]);
        index += 1;
    }
    pins
}

const fn pico_digital_pin(pin: board_vm_pico::DigitalPinDescriptor) -> DigitalPinInfo {
    DigitalPinInfo {
        pin: pin.gpio,
        label: pin.label,
        supports_input: pin.supports_input,
        supports_output: pin.supports_output,
        supports_pullup: pin.supports_pullup,
        supports_pulldown: pin.supports_pulldown,
        supports_adc: pin.supports_adc,
        supports_pwm: pin.supports_pwm,
        supports_dac: false,
        supports_touch: false,
        supports_interrupt: pin.supports_input,
        boot_strap: false,
        notes: pin.notes,
    }
}

const fn arduino_cli_port_hint(hint: board_vm_arduino::ArduinoCliPortHint) -> UploadPortHint {
    match hint {
        board_vm_arduino::ArduinoCliPortHint::UsbSerialBridge => UploadPortHint::UsbSerialBridge,
        board_vm_arduino::ArduinoCliPortHint::NativeUsb => UploadPortHint::NativeUsb,
        board_vm_arduino::ArduinoCliPortHint::ExternalSerialAdapter => {
            UploadPortHint::ExternalSerialAdapter
        }
    }
}

const fn arduino_board_target(
    target: board_vm_arduino::ArduinoTargetDescriptor,
    digital_pins: &'static [DigitalPinInfo],
    wireless: &'static [WirelessInterfaceInfo],
    network_interfaces: &'static [NetworkInterfaceInfo],
) -> BoardTargetInfo {
    arduino_board_target_with_buses(
        target,
        digital_pins,
        &[],
        &[],
        &[],
        wireless,
        network_interfaces,
    )
}

const fn arduino_board_target_with_buses(
    target: board_vm_arduino::ArduinoTargetDescriptor,
    digital_pins: &'static [DigitalPinInfo],
    i2c_buses: &'static [I2cBusInfo],
    spi_buses: &'static [SpiBusInfo],
    uart_buses: &'static [UartBusInfo],
    wireless: &'static [WirelessInterfaceInfo],
    network_interfaces: &'static [NetworkInterfaceInfo],
) -> BoardTargetInfo {
    arduino_board_target_with_buses_and_can(
        target,
        digital_pins,
        i2c_buses,
        spi_buses,
        uart_buses,
        &[],
        wireless,
        network_interfaces,
    )
}

const fn arduino_board_target_with_buses_and_can(
    target: board_vm_arduino::ArduinoTargetDescriptor,
    digital_pins: &'static [DigitalPinInfo],
    i2c_buses: &'static [I2cBusInfo],
    spi_buses: &'static [SpiBusInfo],
    uart_buses: &'static [UartBusInfo],
    can_buses: &'static [CanBusInfo],
    wireless: &'static [WirelessInterfaceInfo],
    network_interfaces: &'static [NetworkInterfaceInfo],
) -> BoardTargetInfo {
    arduino_board_target_with_buses_can_and_rtc(
        target,
        digital_pins,
        i2c_buses,
        &[],
        spi_buses,
        uart_buses,
        &[],
        can_buses,
        None,
        wireless,
        network_interfaces,
    )
}

const fn arduino_board_target_with_buses_can_and_rtc(
    target: board_vm_arduino::ArduinoTargetDescriptor,
    digital_pins: &'static [DigitalPinInfo],
    i2c_buses: &'static [I2cBusInfo],
    i2c_connectors: &'static [I2cConnectorInfo],
    spi_buses: &'static [SpiBusInfo],
    uart_buses: &'static [UartBusInfo],
    usb_interfaces: &'static [UsbInterfaceInfo],
    can_buses: &'static [CanBusInfo],
    rtc: Option<RtcInfo>,
    wireless: &'static [WirelessInterfaceInfo],
    network_interfaces: &'static [NetworkInterfaceInfo],
) -> BoardTargetInfo {
    BoardTargetInfo {
        board_id: target.board_id,
        display_name: target.display_name,
        family: BoardFamily::Arduino,
        runtime_id: board_vm_arduino::ARDUINO_VM_RUNTIME_ID,
        mcu: target.mcu,
        core: target.core,
        rust_target: target.rust_target.name(),
        clock_hz: target.clock_hz,
        operating_voltage_mv: target.operating_voltage_mv,
        onboard_led: match target.onboard_led_pin {
            Some(pin) => Some(OnboardLed::Gpio(pin)),
            None => None,
        },
        led_matrix: None,
        digital_pin_count: digital_pins.len(),
        digital_pins,
        i2c_buses,
        i2c_connectors,
        spi_buses,
        uart_buses,
        usb_interfaces,
        can_buses,
        rtc,
        watchdog: None,
        storage_regions: &[],
        wireless,
        network_interfaces,
        upload: Some(arduino_cli_upload(
            target.arduino_cli.platform_id,
            target.arduino_cli.fqbn,
            arduino_cli_port_hint(target.arduino_cli.port_hint),
        )),
        capabilities: &BLINK_MVP_CAPABILITIES,
    }
}

pub const BOARD_TARGETS: [BoardTargetInfo; 31] = [
    BoardTargetInfo {
        board_id: board_vm_uno_r4::UNO_R4_MINIMA.board_id,
        display_name: board_vm_uno_r4::UNO_R4_MINIMA.display_name,
        family: BoardFamily::ArduinoUnoR4,
        runtime_id: board_vm_uno_r4::UNO_R4_VM_RUNTIME_ID,
        mcu: board_vm_uno_r4::UNO_R4_MINIMA.mcu,
        core: board_vm_uno_r4::UNO_R4_MINIMA.core,
        rust_target: board_vm_uno_r4::UNO_R4_MINIMA.rust_target,
        clock_hz: board_vm_uno_r4::UNO_R4_MINIMA.clock_hz,
        operating_voltage_mv: board_vm_uno_r4::UNO_R4_MINIMA.operating_voltage_mv,
        onboard_led: Some(OnboardLed::Gpio(
            board_vm_uno_r4::UNO_R4_MINIMA.onboard_led_pin,
        )),
        led_matrix: None,
        digital_pin_count: UNO_R4_DIGITAL_PINS.len(),
        digital_pins: &UNO_R4_DIGITAL_PINS,
        i2c_buses: &UNO_R4_MINIMA_I2C_BUSES,
        i2c_connectors: &[],
        spi_buses: &UNO_R4_SPI_BUSES,
        uart_buses: &UNO_R4_MINIMA_UART_BUSES,
        usb_interfaces: &[],
        can_buses: &UNO_R4_CAN_BUSES,
        rtc: Some(UNO_R4_RTC),
        watchdog: Some(UNO_R4_WATCHDOG),
        storage_regions: &UNO_R4_STORAGE_REGIONS,
        wireless: &[],
        network_interfaces: &[],
        upload: Some(UNO_R4_MINIMA_UPLOAD),
        capabilities: &UNO_R4_MINIMA_CAPABILITIES,
    },
    BoardTargetInfo {
        board_id: board_vm_uno_r4::UNO_R4_WIFI.board_id,
        display_name: board_vm_uno_r4::UNO_R4_WIFI.display_name,
        family: BoardFamily::ArduinoUnoR4,
        runtime_id: board_vm_uno_r4::UNO_R4_VM_RUNTIME_ID,
        mcu: board_vm_uno_r4::UNO_R4_WIFI.mcu,
        core: board_vm_uno_r4::UNO_R4_WIFI.core,
        rust_target: board_vm_uno_r4::UNO_R4_WIFI.rust_target,
        clock_hz: board_vm_uno_r4::UNO_R4_WIFI.clock_hz,
        operating_voltage_mv: board_vm_uno_r4::UNO_R4_WIFI.operating_voltage_mv,
        onboard_led: Some(OnboardLed::Gpio(
            board_vm_uno_r4::UNO_R4_WIFI.onboard_led_pin,
        )),
        led_matrix: if board_vm_uno_r4::UNO_R4_WIFI.supports_led_matrix {
            Some(UNO_R4_WIFI_LED_MATRIX)
        } else {
            None
        },
        digital_pin_count: UNO_R4_DIGITAL_PINS.len(),
        digital_pins: &UNO_R4_DIGITAL_PINS,
        i2c_buses: &UNO_R4_WIFI_I2C_BUSES,
        i2c_connectors: &[],
        spi_buses: &UNO_R4_SPI_BUSES,
        uart_buses: &UNO_R4_WIFI_UART_BUSES,
        usb_interfaces: &[],
        can_buses: &UNO_R4_CAN_BUSES,
        rtc: Some(UNO_R4_RTC),
        watchdog: Some(UNO_R4_WATCHDOG),
        storage_regions: &UNO_R4_STORAGE_REGIONS,
        wireless: &UNO_R4_WIFI_WIRELESS,
        network_interfaces: &UNO_R4_WIFI_NETWORK_INTERFACES,
        upload: Some(UNO_R4_WIFI_UPLOAD),
        capabilities: &UNO_R4_WIFI_CAPABILITIES,
    },
    arduino_board_target_with_buses(
        board_vm_arduino::ARDUINO_UNO_R3,
        &ARDUINO_UNO_R3_DIGITAL_PINS,
        &ARDUINO_STANDARD_I2C_BUSES,
        &ARDUINO_STANDARD_SPI_BUSES,
        &ARDUINO_STANDARD_UART_BUSES,
        &[],
        &[],
    ),
    arduino_board_target_with_buses(
        board_vm_arduino::ARDUINO_NANO_CLASSIC,
        &ARDUINO_NANO_CLASSIC_DIGITAL_PINS,
        &ARDUINO_STANDARD_I2C_BUSES,
        &ARDUINO_STANDARD_SPI_BUSES,
        &ARDUINO_STANDARD_UART_BUSES,
        &[],
        &[],
    ),
    arduino_board_target_with_buses(
        board_vm_arduino::ARDUINO_PRO_MINI,
        &ARDUINO_PRO_MINI_DIGITAL_PINS,
        &ARDUINO_STANDARD_I2C_BUSES,
        &ARDUINO_STANDARD_SPI_BUSES,
        &ARDUINO_STANDARD_UART_BUSES,
        &[],
        &[],
    ),
    arduino_board_target_with_buses(
        board_vm_arduino::ARDUINO_MEGA_2560,
        &ARDUINO_MEGA_2560_DIGITAL_PINS,
        &ARDUINO_MEGA_2560_I2C_BUSES,
        &ARDUINO_MEGA_2560_SPI_BUSES,
        &ARDUINO_MEGA_2560_UART_BUSES,
        &[],
        &[],
    ),
    arduino_board_target_with_buses(
        board_vm_arduino::ARDUINO_LEONARDO,
        &ARDUINO_LEONARDO_DIGITAL_PINS,
        &ARDUINO_USB_AVR_I2C_BUSES,
        &ARDUINO_USB_AVR_SPI_BUSES,
        &ARDUINO_USB_AVR_UART_BUSES,
        &[],
        &[],
    ),
    arduino_board_target_with_buses(
        board_vm_arduino::ARDUINO_MICRO,
        &ARDUINO_MICRO_DIGITAL_PINS,
        &ARDUINO_USB_AVR_I2C_BUSES,
        &ARDUINO_USB_AVR_SPI_BUSES,
        &ARDUINO_USB_AVR_UART_BUSES,
        &[],
        &[],
    ),
    arduino_board_target(
        board_vm_arduino::ARDUINO_DUE,
        &ARDUINO_DUE_DIGITAL_PINS,
        &[],
        &[],
    ),
    arduino_board_target(
        board_vm_arduino::ARDUINO_ZERO,
        &ARDUINO_ZERO_DIGITAL_PINS,
        &[],
        &[],
    ),
    arduino_board_target(
        board_vm_arduino::ARDUINO_MKR_WIFI_1010,
        &ARDUINO_MKR_WIFI_1010_DIGITAL_PINS,
        &ARDUINO_NINA_W102_WIRELESS,
        &ARDUINO_NINA_W102_NETWORK_INTERFACES,
    ),
    arduino_board_target_with_buses(
        board_vm_arduino::ARDUINO_NANO_EVERY,
        &ARDUINO_NANO_EVERY_DIGITAL_PINS,
        &ARDUINO_NANO_EVERY_I2C_BUSES,
        &ARDUINO_NANO_EVERY_SPI_BUSES,
        &ARDUINO_NANO_EVERY_UART_BUSES,
        &[],
        &[],
    ),
    arduino_board_target_with_buses_can_and_rtc(
        board_vm_arduino::ARDUINO_NANO_R4,
        &ARDUINO_NANO_R4_DIGITAL_PINS,
        &ARDUINO_NANO_R4_I2C_BUSES,
        &ARDUINO_NANO_R4_I2C_CONNECTORS,
        &ARDUINO_NANO_R4_SPI_BUSES,
        &ARDUINO_NANO_R4_UART_BUSES,
        &ARDUINO_NANO_R4_USB_INTERFACES,
        &ARDUINO_NANO_R4_CAN_BUSES,
        Some(ARDUINO_NANO_R4_RTC),
        &[],
        &[],
    ),
    arduino_board_target(
        board_vm_arduino::ARDUINO_NANO_33_IOT,
        &ARDUINO_NANO_33_IOT_DIGITAL_PINS,
        &ARDUINO_NINA_W102_WIRELESS,
        &ARDUINO_NINA_W102_NETWORK_INTERFACES,
    ),
    arduino_board_target(
        board_vm_arduino::ARDUINO_NANO_33_BLE_REV2,
        &ARDUINO_NANO_33_BLE_REV2_DIGITAL_PINS,
        &ARDUINO_NRF52840_WIRELESS,
        &[],
    ),
    arduino_board_target(
        board_vm_arduino::ARDUINO_NANO_RP2040_CONNECT,
        &ARDUINO_NANO_RP2040_CONNECT_DIGITAL_PINS,
        &ARDUINO_NINA_W102_WIRELESS,
        &ARDUINO_NINA_W102_NETWORK_INTERFACES,
    ),
    arduino_board_target(
        board_vm_arduino::ARDUINO_NANO_ESP32,
        &ARDUINO_NANO_ESP32_DIGITAL_PINS,
        &ARDUINO_NANO_ESP32_WIRELESS,
        &ARDUINO_NANO_ESP32_NETWORK_INTERFACES,
    ),
    arduino_board_target(
        board_vm_arduino::ARDUINO_GIGA_R1_WIFI,
        &ARDUINO_GIGA_R1_WIFI_DIGITAL_PINS,
        &ARDUINO_WIFI_BLE_MODULE_WIRELESS,
        &ARDUINO_WIFI_BLE_MODULE_NETWORK_INTERFACES,
    ),
    arduino_board_target(
        board_vm_arduino::ARDUINO_PORTENTA_H7,
        &ARDUINO_PORTENTA_H7_DIGITAL_PINS,
        &[],
        &[],
    ),
    arduino_board_target(
        board_vm_arduino::ARDUINO_PORTENTA_H7_LITE,
        &ARDUINO_PORTENTA_H7_LITE_DIGITAL_PINS,
        &[],
        &[],
    ),
    arduino_board_target(
        board_vm_arduino::ARDUINO_PORTENTA_H7_LITE_CONNECTED,
        &ARDUINO_PORTENTA_H7_LITE_CONNECTED_DIGITAL_PINS,
        &ARDUINO_WIFI_BLE_MODULE_WIRELESS,
        &ARDUINO_WIFI_BLE_MODULE_NETWORK_INTERFACES,
    ),
    arduino_board_target(
        board_vm_arduino::ARDUINO_PORTENTA_C33,
        &ARDUINO_PORTENTA_C33_DIGITAL_PINS,
        &ARDUINO_ESP32_C3_WIRELESS,
        &ARDUINO_ESP32_C3_NETWORK_INTERFACES,
    ),
    arduino_board_target(
        board_vm_arduino::ARDUINO_NICLA_VISION,
        &ARDUINO_NICLA_VISION_DIGITAL_PINS,
        &ARDUINO_WIFI_BLE_MODULE_WIRELESS,
        &ARDUINO_WIFI_BLE_MODULE_NETWORK_INTERFACES,
    ),
    arduino_board_target(
        board_vm_arduino::ARDUINO_NICLA_SENSE_ME,
        &ARDUINO_NICLA_SENSE_ME_DIGITAL_PINS,
        &ARDUINO_NRF52832_WIRELESS,
        &[],
    ),
    arduino_board_target(
        board_vm_arduino::ARDUINO_NICLA_VOICE,
        &ARDUINO_NICLA_VOICE_DIGITAL_PINS,
        &ARDUINO_NRF52832_WIRELESS,
        &[],
    ),
    arduino_board_target(
        board_vm_arduino::ARDUINO_OPTA_LITE,
        &ARDUINO_OPTA_LITE_DIGITAL_PINS,
        &[],
        &[],
    ),
    arduino_board_target(
        board_vm_arduino::ARDUINO_OPTA_RS485,
        &ARDUINO_OPTA_RS485_DIGITAL_PINS,
        &[],
        &[],
    ),
    arduino_board_target(
        board_vm_arduino::ARDUINO_OPTA_WIFI,
        &ARDUINO_OPTA_WIFI_DIGITAL_PINS,
        &ARDUINO_WIFI_BLE_MODULE_WIRELESS,
        &ARDUINO_WIFI_BLE_MODULE_NETWORK_INTERFACES,
    ),
    BoardTargetInfo {
        board_id: board_vm_esp32::ESP32_DEVKIT_V1.board_id,
        display_name: board_vm_esp32::ESP32_DEVKIT_V1.display_name,
        family: BoardFamily::Esp32,
        runtime_id: board_vm_esp32::ESP32_VM_RUNTIME_ID,
        mcu: board_vm_esp32::ESP32_DEVKIT_V1.mcu,
        core: board_vm_esp32::ESP32_DEVKIT_V1.core,
        rust_target: board_vm_esp32::ESP32_DEVKIT_V1.rust_target,
        clock_hz: board_vm_esp32::ESP32_DEVKIT_V1.clock_hz,
        operating_voltage_mv: board_vm_esp32::ESP32_DEVKIT_V1.operating_voltage_mv,
        onboard_led: match board_vm_esp32::ESP32_DEVKIT_V1.onboard_led_pin {
            Some(pin) => Some(OnboardLed::Gpio(pin)),
            None => None,
        },
        led_matrix: None,
        digital_pin_count: ESP32_DIGITAL_PINS.len(),
        digital_pins: &ESP32_DIGITAL_PINS,
        i2c_buses: &[],
        i2c_connectors: &[],
        spi_buses: &[],
        uart_buses: &[],
        usb_interfaces: &[],
        can_buses: &[],
        rtc: None,
        watchdog: None,
        storage_regions: &[],
        wireless: &ESP32_WIRELESS,
        network_interfaces: &[],
        upload: Some(ESP_ROM_SERIAL_UPLOAD),
        capabilities: &ESP32_CAPABILITIES,
    },
    BoardTargetInfo {
        board_id: board_vm_pico::PICO.board_id,
        display_name: board_vm_pico::PICO.display_name,
        family: BoardFamily::RaspberryPiPico,
        runtime_id: board_vm_pico::PICO_VM_RUNTIME_ID,
        mcu: board_vm_pico::PICO.mcu,
        core: board_vm_pico::PICO.core,
        rust_target: board_vm_pico::PICO.rust_target,
        clock_hz: board_vm_pico::PICO.clock_hz,
        operating_voltage_mv: board_vm_pico::PICO.operating_voltage_mv,
        onboard_led: match board_vm_pico::PICO.onboard_led {
            Some(board_vm_pico::OnboardLed::Gpio(pin)) => Some(OnboardLed::Gpio(pin)),
            Some(board_vm_pico::OnboardLed::WirelessChipGpio(pin)) => {
                Some(OnboardLed::WirelessChipGpio(pin))
            }
            None => None,
        },
        led_matrix: None,
        digital_pin_count: PICO_DIGITAL_PINS.len(),
        digital_pins: &PICO_DIGITAL_PINS,
        i2c_buses: &[],
        i2c_connectors: &[],
        spi_buses: &[],
        uart_buses: &[],
        usb_interfaces: &[],
        can_buses: &[],
        rtc: None,
        watchdog: None,
        storage_regions: &[],
        wireless: &[],
        network_interfaces: &[],
        upload: Some(PICO_UF2_UPLOAD),
        capabilities: &BLINK_MVP_CAPABILITIES,
    },
    BoardTargetInfo {
        board_id: board_vm_pico::PICO_W.board_id,
        display_name: board_vm_pico::PICO_W.display_name,
        family: BoardFamily::RaspberryPiPico,
        runtime_id: board_vm_pico::PICO_VM_RUNTIME_ID,
        mcu: board_vm_pico::PICO_W.mcu,
        core: board_vm_pico::PICO_W.core,
        rust_target: board_vm_pico::PICO_W.rust_target,
        clock_hz: board_vm_pico::PICO_W.clock_hz,
        operating_voltage_mv: board_vm_pico::PICO_W.operating_voltage_mv,
        onboard_led: match board_vm_pico::PICO_W.onboard_led {
            Some(board_vm_pico::OnboardLed::Gpio(pin)) => Some(OnboardLed::Gpio(pin)),
            Some(board_vm_pico::OnboardLed::WirelessChipGpio(pin)) => {
                Some(OnboardLed::WirelessChipGpio(pin))
            }
            None => None,
        },
        led_matrix: None,
        digital_pin_count: PICO_DIGITAL_PINS.len(),
        digital_pins: &PICO_DIGITAL_PINS,
        i2c_buses: &[],
        i2c_connectors: &[],
        spi_buses: &[],
        uart_buses: &[],
        usb_interfaces: &[],
        can_buses: &[],
        rtc: None,
        watchdog: None,
        storage_regions: &[],
        wireless: &PICO_W_WIRELESS,
        network_interfaces: &[],
        upload: Some(PICO_UF2_UPLOAD),
        capabilities: &PICO_W_CAPABILITIES,
    },
];

pub fn all_targets() -> &'static [BoardTargetInfo] {
    &BOARD_TARGETS
}

pub fn find_target(board_id: &str) -> Option<&'static BoardTargetInfo> {
    BOARD_TARGETS
        .iter()
        .find(|target| target.board_id == board_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_lists_current_board_families() {
        assert!(find_target("arduino-uno-r4-wifi").is_some());
        assert!(find_target("arduino-uno-r3").is_some());
        assert!(find_target("arduino-mega-2560").is_some());
        assert!(find_target("arduino-due").is_some());
        assert!(find_target("arduino-nano-r4").is_some());
        assert!(find_target("arduino-nano-33-ble-rev2").is_some());
        assert!(find_target("arduino-nano-rp2040-connect").is_some());
        assert!(find_target("arduino-nano-esp32").is_some());
        assert!(find_target("arduino-giga-r1-wifi").is_some());
        assert!(find_target("arduino-portenta-h7").is_some());
        assert!(find_target("arduino-portenta-c33").is_some());
        assert!(find_target("arduino-nicla-vision").is_some());
        assert!(find_target("arduino-nicla-sense-me").is_some());
        assert!(find_target("arduino-nicla-voice").is_some());
        assert!(find_target("arduino-opta-lite").is_some());
        assert!(find_target("arduino-opta-rs485").is_some());
        assert!(find_target("arduino-opta-wifi").is_some());
        assert!(find_target("esp32-devkit-v1").is_some());
        assert!(find_target("raspberry-pi-pico").is_some());
        assert!(find_target("raspberry-pi-pico-w").is_some());
    }

    #[test]
    fn registry_imports_every_shared_arduino_target() {
        for arduino_target in board_vm_arduino::ARDUINO_TARGETS.iter() {
            let target = find_target(arduino_target.board_id).unwrap();
            assert_eq!(target.display_name, arduino_target.display_name);
            assert_eq!(target.family, BoardFamily::Arduino);
            assert_eq!(target.runtime_id, board_vm_arduino::ARDUINO_VM_RUNTIME_ID);
            assert_eq!(target.mcu, arduino_target.mcu);
            assert_eq!(target.rust_target, arduino_target.rust_target.name());
            assert_eq!(target.digital_pin_count, arduino_target.digital_pins.len());
            assert_eq!(
                target.upload.unwrap().platform_id,
                Some(arduino_target.arduino_cli.platform_id)
            );
            assert_eq!(
                target.upload.unwrap().fqbn,
                Some(arduino_target.arduino_cli.fqbn)
            );
            assert_eq!(
                target.upload.unwrap().port_hint,
                Some(arduino_cli_port_hint(arduino_target.arduino_cli.port_hint))
            );
            assert!(target.capabilities.contains(&"transport.serial"));
            assert!(target.capabilities.contains(&"gpio.open"));
            assert!(target.capabilities.contains(&"gpio.write"));
            assert!(target.capabilities.contains(&"time.sleep_ms"));
            assert!(target.capabilities.contains(&"program.ram_exec"));
        }
    }

    #[test]
    fn registry_exposes_upload_profiles() {
        let uno = find_target("arduino-uno-r4-wifi").unwrap();
        assert_eq!(uno.upload, Some(UNO_R4_WIFI_UPLOAD));
        assert_eq!(uno.upload.unwrap().platform_id, Some("arduino:renesas_uno"));
        assert_eq!(
            uno.upload.unwrap().fqbn,
            Some("arduino:renesas_uno:unor4wifi")
        );
        assert_eq!(
            uno.upload.unwrap().port_hint,
            Some(UploadPortHint::NativeUsb)
        );

        let opta = find_target("arduino-opta-wifi").unwrap();
        assert_eq!(opta.upload.unwrap().command, "arduino-cli upload");
        assert_eq!(opta.upload.unwrap().platform_id, Some("arduino:mbed_opta"));
        assert_eq!(opta.upload.unwrap().fqbn, Some("arduino:mbed_opta:opta"));
        assert_eq!(
            opta.upload.unwrap().port_hint,
            Some(UploadPortHint::NativeUsb)
        );
        assert_eq!(
            opta.upload.unwrap().reset_method,
            UploadResetMethod::ArduinoBoardPackage
        );

        let pro_mini = find_target("arduino-pro-mini").unwrap();
        assert_eq!(
            pro_mini.upload.unwrap().port_hint,
            Some(UploadPortHint::ExternalSerialAdapter)
        );

        let nano_esp32 = find_target("arduino-nano-esp32").unwrap();
        assert_eq!(
            nano_esp32.upload.unwrap().platform_id,
            Some("arduino:esp32")
        );
        assert_eq!(
            nano_esp32.upload.unwrap().fqbn,
            Some("arduino:esp32:nano_nora")
        );

        let esp32 = find_target("esp32-devkit-v1").unwrap();
        assert_eq!(esp32.upload, Some(ESP_ROM_SERIAL_UPLOAD));
        assert_eq!(esp32.upload.unwrap().fqbn, None);
        assert_eq!(
            esp32.upload.unwrap().port_hint,
            Some(UploadPortHint::EspRomSerial)
        );
        assert_eq!(
            esp32.upload.unwrap().image_format,
            UploadImageFormat::EspFlashImage
        );

        let pico = find_target("raspberry-pi-pico").unwrap();
        assert_eq!(pico.upload, Some(PICO_UF2_UPLOAD));
        assert_eq!(pico.upload.unwrap().transport, UploadTransport::MassStorage);
        assert_eq!(
            pico.upload.unwrap().port_hint,
            Some(UploadPortHint::MassStorageBootloader)
        );
    }

    #[test]
    fn shared_arduino_targets_cover_more_than_uno_r4() {
        let arduino_targets = BOARD_TARGETS
            .iter()
            .filter(|target| target.family == BoardFamily::Arduino)
            .count();
        assert_eq!(arduino_targets, board_vm_arduino::ARDUINO_TARGETS.len());
        assert!(find_target("arduino-uno-r3").unwrap().digital_pin_count >= 20);
        assert!(find_target("arduino-mega-2560").unwrap().digital_pin_count >= 70);
        assert!(find_target("arduino-due").unwrap().digital_pin_count >= 76);
        assert_eq!(find_target("arduino-nano-r4").unwrap().mcu, "RA4M1");
        assert_eq!(
            find_target("arduino-nano-33-ble-rev2").unwrap().mcu,
            "nRF52840"
        );
        assert_eq!(find_target("arduino-nano-esp32").unwrap().mcu, "ESP32-S3");
        assert_eq!(
            find_target("arduino-giga-r1-wifi").unwrap().mcu,
            "STM32H747XI"
        );
        assert_eq!(
            find_target("arduino-portenta-c33").unwrap().mcu,
            "R7FA6M5BH2CBG"
        );
        assert_eq!(
            find_target("arduino-portenta-c33")
                .unwrap()
                .digital_pin_count,
            7
        );
        assert_eq!(
            find_target("arduino-nicla-vision").unwrap().mcu,
            "STM32H747AII6"
        );
        assert_eq!(find_target("arduino-nicla-voice").unwrap().mcu, "nRF52832");
        assert_eq!(find_target("arduino-opta-wifi").unwrap().mcu, "STM32H747XI");
        assert_eq!(
            find_target("arduino-opta-wifi").unwrap().digital_pins[0].label,
            "I1"
        );
        assert_eq!(
            find_target("arduino-opta-wifi").unwrap().digital_pins[11].label,
            "O4"
        );
        assert!(!find_target("arduino-opta-wifi").unwrap().digital_pins[0].supports_output);
        assert!(find_target("arduino-opta-wifi").unwrap().digital_pins[11].supports_output);
    }

    #[test]
    fn registry_preserves_led_routes() {
        assert_eq!(
            find_target("raspberry-pi-pico").unwrap().onboard_led,
            Some(OnboardLed::Gpio(25))
        );
        assert_eq!(
            find_target("raspberry-pi-pico-w").unwrap().onboard_led,
            Some(OnboardLed::WirelessChipGpio(0))
        );
        assert_eq!(
            find_target("esp32-devkit-v1").unwrap().onboard_led,
            Some(OnboardLed::Gpio(2))
        );
    }

    #[test]
    fn registry_exposes_board_local_led_matrix_metadata() {
        assert_eq!(
            find_target("arduino-uno-r4-wifi").unwrap().led_matrix,
            Some(LedMatrixInfo {
                rows: 8,
                columns: 12
            })
        );
        assert_eq!(
            find_target("arduino-uno-r4-minima").unwrap().led_matrix,
            None
        );
        assert_eq!(find_target("raspberry-pi-pico-w").unwrap().led_matrix, None);
    }

    #[test]
    fn registry_exposes_i2c_bus_metadata() {
        let minima = find_target("arduino-uno-r4-minima").unwrap();
        assert_eq!(minima.i2c_buses.len(), 1);
        assert_eq!(minima.i2c_buses[0].name, "Wire");
        assert_eq!(minima.i2c_buses[0].sda_pin, 18);
        assert_eq!(minima.i2c_buses[0].scl_pin, 19);
        assert!(!minima.i2c_buses[0].qwiic);

        let wifi = find_target("arduino-uno-r4-wifi").unwrap();
        assert_eq!(wifi.i2c_buses.len(), 2);
        assert_eq!(wifi.i2c_buses[1].name, "Wire1");
        assert_eq!(wifi.i2c_buses[1].sda_pin, 27);
        assert_eq!(wifi.i2c_buses[1].scl_pin, 26);
        assert!(wifi.i2c_buses[1].qwiic);

        let uno_r3 = find_target("arduino-uno-r3").unwrap();
        let nano = find_target("arduino-nano-classic").unwrap();
        let pro_mini = find_target("arduino-pro-mini").unwrap();
        let mega = find_target("arduino-mega-2560").unwrap();
        let leonardo = find_target("arduino-leonardo").unwrap();
        let micro = find_target("arduino-micro").unwrap();
        let nano_every = find_target("arduino-nano-every").unwrap();
        let nano_r4 = find_target("arduino-nano-r4").unwrap();
        assert_eq!(uno_r3.i2c_buses, &ARDUINO_STANDARD_I2C_BUSES);
        assert_eq!(nano.i2c_buses, &ARDUINO_STANDARD_I2C_BUSES);
        assert_eq!(pro_mini.i2c_buses, &ARDUINO_STANDARD_I2C_BUSES);
        assert_eq!(mega.i2c_buses, &ARDUINO_MEGA_2560_I2C_BUSES);
        assert_eq!(mega.i2c_buses[0].sda_pin, 20);
        assert_eq!(mega.i2c_buses[0].scl_pin, 21);
        assert_eq!(leonardo.i2c_buses, &ARDUINO_USB_AVR_I2C_BUSES);
        assert_eq!(micro.i2c_buses, &ARDUINO_USB_AVR_I2C_BUSES);
        assert_eq!(leonardo.i2c_buses[0].sda_pin, 2);
        assert_eq!(leonardo.i2c_buses[0].scl_pin, 3);
        assert_eq!(nano_every.i2c_buses, &ARDUINO_NANO_EVERY_I2C_BUSES);
        assert_eq!(nano_every.i2c_buses[0].sda_pin, 18);
        assert_eq!(nano_every.i2c_buses[0].scl_pin, 19);
        assert_eq!(nano_r4.i2c_buses, &ARDUINO_NANO_R4_I2C_BUSES);
        assert_eq!(nano_r4.i2c_buses[0].sda_pin, 18);
        assert_eq!(nano_r4.i2c_buses[0].scl_pin, 19);
        assert!(nano_r4.i2c_buses[0].notes.contains("Qwiic Wire1"));
        assert_eq!(nano_r4.i2c_connectors, &ARDUINO_NANO_R4_I2C_CONNECTORS);
        assert_eq!(nano_r4.i2c_connectors[0].name, "Wire1");
        assert_eq!(nano_r4.i2c_connectors[0].connector, "Qwiic/STEMMA QT");
        assert_eq!(nano_r4.i2c_connectors[0].arduino_object, "Wire1");
        assert_eq!(nano_r4.i2c_connectors[0].controller, "RA4M1 IIC0");
        assert!(nano_r4.i2c_connectors[0].notes.contains("connector-local"));
        for target in [
            uno_r3, nano, pro_mini, mega, leonardo, micro, nano_every, nano_r4,
        ] {
            assert!(target.i2c_buses[0].notes.contains("metadata only"));
            assert!(!target.capabilities.contains(&"i2c.open"));
            assert!(!target.capabilities.contains(&"i2c.transfer"));
        }

        assert!(find_target("esp32-devkit-v1").unwrap().i2c_buses.is_empty());
    }

    #[test]
    fn registry_exposes_usb_interface_metadata() {
        let nano_r4 = find_target("arduino-nano-r4").unwrap();
        assert_eq!(nano_r4.usb_interfaces, &ARDUINO_NANO_R4_USB_INTERFACES);
        assert_eq!(nano_r4.usb_interfaces[0].name, "Native USB");
        assert_eq!(nano_r4.usb_interfaces[0].controller, "RA4M1 native USB");
        assert_eq!(nano_r4.usb_interfaces[0].class, "CDC serial");
        assert!(nano_r4.usb_interfaces[0].native);
        assert!(nano_r4.usb_interfaces[0].upload);
        assert!(nano_r4.usb_interfaces[0].command_transport);
        assert!(nano_r4.usb_interfaces[0].notes.contains("not a GPIO UART"));

        let nano_every = find_target("arduino-nano-every").unwrap();
        assert!(nano_every.usb_interfaces.is_empty());
        assert_eq!(
            nano_every.upload.unwrap().port_hint,
            Some(UploadPortHint::UsbSerialBridge)
        );
    }

    #[test]
    fn registry_exposes_spi_bus_metadata() {
        let uno = find_target("arduino-uno-r4-wifi").unwrap();
        assert_eq!(uno.spi_buses.len(), 1);
        assert_eq!(uno.spi_buses[0].name, "SPI");
        assert_eq!(uno.spi_buses[0].copi_pin, 11);
        assert_eq!(uno.spi_buses[0].cipo_pin, 12);
        assert_eq!(uno.spi_buses[0].sck_pin, 13);
        assert_eq!(uno.spi_buses[0].default_cs_pin, 10);

        let uno_r3 = find_target("arduino-uno-r3").unwrap();
        let nano = find_target("arduino-nano-classic").unwrap();
        let pro_mini = find_target("arduino-pro-mini").unwrap();
        let mega = find_target("arduino-mega-2560").unwrap();
        let leonardo = find_target("arduino-leonardo").unwrap();
        let micro = find_target("arduino-micro").unwrap();
        let nano_every = find_target("arduino-nano-every").unwrap();
        let nano_r4 = find_target("arduino-nano-r4").unwrap();
        assert_eq!(uno_r3.spi_buses, &ARDUINO_STANDARD_SPI_BUSES);
        assert_eq!(nano.spi_buses, &ARDUINO_STANDARD_SPI_BUSES);
        assert_eq!(pro_mini.spi_buses, &ARDUINO_STANDARD_SPI_BUSES);
        assert_eq!(mega.spi_buses, &ARDUINO_MEGA_2560_SPI_BUSES);
        assert_eq!(mega.spi_buses[0].copi_pin, 51);
        assert_eq!(mega.spi_buses[0].cipo_pin, 50);
        assert_eq!(mega.spi_buses[0].sck_pin, 52);
        assert_eq!(mega.spi_buses[0].default_cs_pin, 53);
        assert_eq!(leonardo.spi_buses, &ARDUINO_USB_AVR_SPI_BUSES);
        assert_eq!(micro.spi_buses, &ARDUINO_USB_AVR_SPI_BUSES);
        assert_eq!(leonardo.spi_buses[0].copi_pin, 16);
        assert_eq!(leonardo.spi_buses[0].cipo_pin, 14);
        assert_eq!(leonardo.spi_buses[0].sck_pin, 15);
        assert_eq!(leonardo.spi_buses[0].default_cs_pin, 17);
        assert_eq!(nano_every.spi_buses, &ARDUINO_NANO_EVERY_SPI_BUSES);
        assert_eq!(nano_every.spi_buses[0].copi_pin, 11);
        assert_eq!(nano_every.spi_buses[0].cipo_pin, 12);
        assert_eq!(nano_every.spi_buses[0].sck_pin, 13);
        assert_eq!(nano_every.spi_buses[0].default_cs_pin, 10);
        assert_eq!(nano_r4.spi_buses, &ARDUINO_NANO_R4_SPI_BUSES);
        assert_eq!(nano_r4.spi_buses[0].copi_pin, 11);
        assert_eq!(nano_r4.spi_buses[0].cipo_pin, 12);
        assert_eq!(nano_r4.spi_buses[0].sck_pin, 13);
        assert_eq!(nano_r4.spi_buses[0].default_cs_pin, 10);
        for target in [
            uno_r3, nano, pro_mini, mega, leonardo, micro, nano_every, nano_r4,
        ] {
            assert!(target.spi_buses[0].notes.contains("metadata only"));
            assert!(!target.capabilities.contains(&"spi.open"));
            assert!(!target.capabilities.contains(&"spi.transfer"));
        }

        assert!(find_target("esp32-devkit-v1").unwrap().spi_buses.is_empty());
    }

    #[test]
    fn registry_exposes_uart_bus_metadata() {
        let minima = find_target("arduino-uno-r4-minima").unwrap();
        assert_eq!(minima.uart_buses.len(), 1);
        assert_eq!(minima.uart_buses[0].name, "Serial1");
        assert_eq!(minima.uart_buses[0].tx_pin, 1);
        assert_eq!(minima.uart_buses[0].rx_pin, 0);
        assert_eq!(minima.uart_buses[0].arduino_uart, 1);
        assert!(!minima.uart_buses[0].internal);

        let wifi = find_target("arduino-uno-r4-wifi").unwrap();
        assert_eq!(wifi.uart_buses.len(), 3);
        assert_eq!(wifi.uart_buses[0].name, "Serial1");
        assert_eq!(wifi.uart_buses[0].tx_pin, 22);
        assert_eq!(wifi.uart_buses[0].rx_pin, 23);
        assert_eq!(wifi.uart_buses[1].name, "Serial2");
        assert_eq!(wifi.uart_buses[1].tx_pin, 1);
        assert_eq!(wifi.uart_buses[1].rx_pin, 0);
        assert_eq!(wifi.uart_buses[2].name, "Serial3");
        assert_eq!(wifi.uart_buses[2].tx_pin, 24);
        assert_eq!(wifi.uart_buses[2].rx_pin, 25);
        assert!(wifi.uart_buses[2].internal);

        let uno_r3 = find_target("arduino-uno-r3").unwrap();
        let nano = find_target("arduino-nano-classic").unwrap();
        let pro_mini = find_target("arduino-pro-mini").unwrap();
        let mega = find_target("arduino-mega-2560").unwrap();
        let leonardo = find_target("arduino-leonardo").unwrap();
        let micro = find_target("arduino-micro").unwrap();
        let nano_every = find_target("arduino-nano-every").unwrap();
        let nano_r4 = find_target("arduino-nano-r4").unwrap();
        assert_eq!(uno_r3.uart_buses, &ARDUINO_STANDARD_UART_BUSES);
        assert_eq!(nano.uart_buses, &ARDUINO_STANDARD_UART_BUSES);
        assert_eq!(pro_mini.uart_buses, &ARDUINO_STANDARD_UART_BUSES);
        assert_eq!(mega.uart_buses, &ARDUINO_MEGA_2560_UART_BUSES);
        assert_eq!(mega.uart_buses.len(), 4);
        assert_eq!(mega.uart_buses[1].name, "Serial1");
        assert_eq!(mega.uart_buses[1].tx_pin, 18);
        assert_eq!(mega.uart_buses[1].rx_pin, 19);
        assert_eq!(mega.uart_buses[3].name, "Serial3");
        assert_eq!(mega.uart_buses[3].tx_pin, 14);
        assert_eq!(mega.uart_buses[3].rx_pin, 15);
        assert_eq!(leonardo.uart_buses, &ARDUINO_USB_AVR_UART_BUSES);
        assert_eq!(micro.uart_buses, &ARDUINO_USB_AVR_UART_BUSES);
        assert_eq!(leonardo.uart_buses[0].name, "Serial1");
        assert_eq!(leonardo.uart_buses[0].arduino_uart, 1);
        assert!(!leonardo.uart_buses[0].internal);
        assert!(leonardo.uart_buses[0].notes.contains("native USB Serial"));
        assert_eq!(nano_every.uart_buses, &ARDUINO_NANO_EVERY_UART_BUSES);
        assert_eq!(nano_every.uart_buses[0].name, "Serial1");
        assert_eq!(nano_every.uart_buses[0].arduino_uart, 1);
        assert!(nano_every.uart_buses[0].notes.contains("USB serial bridge"));
        assert_eq!(nano_r4.uart_buses, &ARDUINO_NANO_R4_UART_BUSES);
        assert_eq!(nano_r4.uart_buses[0].name, "Serial1");
        assert_eq!(nano_r4.uart_buses[0].arduino_uart, 1);
        assert!(nano_r4.uart_buses[0].notes.contains("native USB"));
        for target in [
            uno_r3, nano, pro_mini, mega, leonardo, micro, nano_every, nano_r4,
        ] {
            assert!(target.uart_buses[0].notes.contains("metadata"));
            assert!(!target.capabilities.contains(&"uart.open"));
            assert!(!target.capabilities.contains(&"uart.write"));
            assert!(!target.capabilities.contains(&"uart.read"));
        }

        assert!(find_target("esp32-devkit-v1")
            .unwrap()
            .uart_buses
            .is_empty());
    }

    #[test]
    fn registry_exposes_can_bus_metadata() {
        let minima = find_target("arduino-uno-r4-minima").unwrap();
        assert_eq!(minima.can_buses.len(), 1);
        assert_eq!(minima.can_buses[0].name, "CAN0");
        assert_eq!(minima.can_buses[0].tx_pin, 10);
        assert_eq!(minima.can_buses[0].rx_pin, 13);
        assert_eq!(minima.can_buses[0].controller, "RA4M1 CAN0");

        let wifi = find_target("arduino-uno-r4-wifi").unwrap();
        assert_eq!(wifi.can_buses, minima.can_buses);
        assert!(wifi.can_buses[0].notes.contains("SPI"));

        let nano_r4 = find_target("arduino-nano-r4").unwrap();
        assert_eq!(nano_r4.can_buses, &ARDUINO_NANO_R4_CAN_BUSES);
        assert_eq!(nano_r4.can_buses[0].name, "CAN0");
        assert_eq!(nano_r4.can_buses[0].tx_pin, 4);
        assert_eq!(nano_r4.can_buses[0].rx_pin, 5);
        assert_eq!(nano_r4.can_buses[0].controller, "RA4M1 CAN0");
        assert!(nano_r4.can_buses[0].notes.contains("external transceiver"));
        assert!(nano_r4.can_buses[0].notes.contains("metadata only"));
        assert!(!nano_r4.capabilities.contains(&"can.open"));
        assert!(!nano_r4.capabilities.contains(&"can.write"));
        assert!(!nano_r4.capabilities.contains(&"can.read"));

        assert!(find_target("esp32-devkit-v1").unwrap().can_buses.is_empty());
        assert!(find_target("raspberry-pi-pico")
            .unwrap()
            .can_buses
            .is_empty());
        assert!(find_target("raspberry-pi-pico-w")
            .unwrap()
            .can_buses
            .is_empty());
    }

    #[test]
    fn registry_exposes_rtc_metadata() {
        let minima = find_target("arduino-uno-r4-minima").unwrap();
        assert_eq!(minima.rtc, Some(UNO_R4_RTC));
        assert_eq!(minima.rtc.unwrap().instance, 0);
        assert_eq!(minima.rtc.unwrap().name, "RTC");
        assert_eq!(minima.rtc.unwrap().peripheral, "RA4M1 RTC");

        let wifi = find_target("arduino-uno-r4-wifi").unwrap();
        assert_eq!(wifi.rtc, minima.rtc);
        assert!(wifi.rtc.unwrap().notes.contains("real-time clock"));
        assert!(minima.capabilities.contains(&"rtc.now"));
        assert!(minima.capabilities.contains(&"rtc.set"));
        assert!(wifi.capabilities.contains(&"rtc.now"));
        assert!(wifi.capabilities.contains(&"rtc.set"));

        let nano_r4 = find_target("arduino-nano-r4").unwrap();
        assert_eq!(nano_r4.rtc, Some(ARDUINO_NANO_R4_RTC));
        assert_eq!(nano_r4.rtc.unwrap().instance, 0);
        assert_eq!(nano_r4.rtc.unwrap().name, "RTC");
        assert_eq!(nano_r4.rtc.unwrap().peripheral, "RA4M1 RTC");
        assert!(nano_r4.rtc.unwrap().notes.contains("metadata only"));
        assert!(!nano_r4.capabilities.contains(&"rtc.now"));
        assert!(!nano_r4.capabilities.contains(&"rtc.set"));

        assert_eq!(find_target("esp32-devkit-v1").unwrap().rtc, None);
        assert_eq!(find_target("raspberry-pi-pico").unwrap().rtc, None);
        assert_eq!(find_target("raspberry-pi-pico-w").unwrap().rtc, None);
    }

    #[test]
    fn registry_exposes_watchdog_metadata() {
        let minima = find_target("arduino-uno-r4-minima").unwrap();
        assert_eq!(minima.watchdog, Some(UNO_R4_WATCHDOG));
        assert_eq!(minima.watchdog.unwrap().instance, 0);
        assert_eq!(minima.watchdog.unwrap().name, "WDT");
        assert_eq!(minima.watchdog.unwrap().peripheral, "RA4M1 WDT");

        let wifi = find_target("arduino-uno-r4-wifi").unwrap();
        assert_eq!(wifi.watchdog, minima.watchdog);
        assert!(wifi.watchdog.unwrap().notes.contains("watchdog timer"));
        assert!(minima.capabilities.contains(&"watchdog.configure"));
        assert!(minima.capabilities.contains(&"watchdog.kick"));
        assert!(wifi.capabilities.contains(&"watchdog.configure"));
        assert!(wifi.capabilities.contains(&"watchdog.kick"));

        assert_eq!(find_target("esp32-devkit-v1").unwrap().watchdog, None);
        assert_eq!(find_target("raspberry-pi-pico").unwrap().watchdog, None);
        assert_eq!(find_target("raspberry-pi-pico-w").unwrap().watchdog, None);
    }

    #[test]
    fn registry_exposes_storage_region_metadata() {
        let minima = find_target("arduino-uno-r4-minima").unwrap();
        assert_eq!(minima.storage_regions, &UNO_R4_STORAGE_REGIONS);
        assert_eq!(minima.storage_regions.len(), 1);
        assert_eq!(minima.storage_regions[0].region, 0);
        assert_eq!(minima.storage_regions[0].name, "EEPROM emulation");
        assert_eq!(minima.storage_regions[0].kind, "data flash");
        assert_eq!(
            minima.storage_regions[0].bytes,
            board_vm_uno_r4::UNO_R4_DATA_FLASH_BYTES
        );

        let wifi = find_target("arduino-uno-r4-wifi").unwrap();
        assert_eq!(wifi.storage_regions, minima.storage_regions);
        assert!(wifi.storage_regions[0].notes.contains("storage.write"));
        assert!(wifi.storage_regions[0].notes.contains("storage.read"));
        assert!(wifi.storage_regions[0].notes.contains("storage.size"));
        assert!(minima.capabilities.contains(&"storage.write"));
        assert!(minima.capabilities.contains(&"storage.read"));
        assert!(minima.capabilities.contains(&"storage.size"));
        assert!(minima.capabilities.contains(&"program.store"));
        assert!(wifi.capabilities.contains(&"storage.write"));
        assert!(wifi.capabilities.contains(&"storage.read"));
        assert!(wifi.capabilities.contains(&"storage.size"));
        assert!(wifi.capabilities.contains(&"program.store"));

        assert!(find_target("esp32-devkit-v1")
            .unwrap()
            .storage_regions
            .is_empty());
        assert!(find_target("raspberry-pi-pico")
            .unwrap()
            .storage_regions
            .is_empty());
        assert!(find_target("raspberry-pi-pico-w")
            .unwrap()
            .storage_regions
            .is_empty());
    }

    #[test]
    fn registry_exposes_digital_pin_capability_metadata() {
        let uno = find_target("arduino-uno-r4-wifi").unwrap();
        assert_eq!(uno.digital_pin_count, uno.digital_pins.len());

        let d3 = uno.digital_pins.iter().find(|pin| pin.pin == 3).unwrap();
        assert_eq!(d3.label, "D3");
        assert!(d3.supports_input);
        assert!(d3.supports_output);
        assert!(d3.supports_pullup);
        assert!(!d3.supports_pulldown);
        assert!(d3.supports_pwm);
        assert!(d3.supports_interrupt);
        assert!(!d3.supports_adc);

        let d13 = uno.digital_pins.iter().find(|pin| pin.pin == 13).unwrap();
        assert!(d13.notes.contains("onboard LED"));
        assert!(!d13.supports_pwm);

        let a0 = uno.digital_pins.iter().find(|pin| pin.pin == 14).unwrap();
        assert_eq!(a0.label, "A0/D14");
        assert!(a0.supports_adc);
        assert!(a0.supports_dac);
        assert!(!a0.supports_pwm);

        let pico = find_target("raspberry-pi-pico").unwrap();
        let adc0 = pico.digital_pins.iter().find(|pin| pin.pin == 26).unwrap();
        assert!(adc0.supports_adc);
        assert!(adc0.supports_pwm);

        let esp32 = find_target("esp32-devkit-v1").unwrap();
        let boot = esp32.digital_pins.iter().find(|pin| pin.pin == 0).unwrap();
        assert!(boot.boot_strap);
        assert!(boot.supports_touch);
    }

    #[test]
    fn registry_exposes_common_runtime_capabilities() {
        let pico = find_target("raspberry-pi-pico").unwrap();
        assert_eq!(pico.runtime_id, "board-vm-pico");
        assert!(pico.capabilities.contains(&"transport.serial"));
        assert!(pico.capabilities.contains(&"gpio.open"));
        assert!(pico.capabilities.contains(&"program.ram_exec"));
        assert!(!pico.capabilities.contains(&"pwm.write"));

        let uno = find_target("arduino-uno-r4-wifi").unwrap();
        assert!(uno.capabilities.contains(&"pwm.write"));
        assert!(uno.capabilities.contains(&"adc.read"));
        assert!(uno.capabilities.contains(&"dac.write_u12"));
        assert!(uno.capabilities.contains(&"i2c.open"));
        assert!(uno.capabilities.contains(&"i2c.write_u8"));
        assert!(uno.capabilities.contains(&"i2c.read_u8"));
        assert!(uno.capabilities.contains(&"i2c.write"));
        assert!(uno.capabilities.contains(&"i2c.read"));
        assert!(uno.capabilities.contains(&"i2c.transfer"));
        assert!(uno.capabilities.contains(&"spi.open"));
        assert!(uno.capabilities.contains(&"spi.transfer"));
        assert!(uno.capabilities.contains(&"uart.open"));
        assert!(uno.capabilities.contains(&"uart.write"));
        assert!(uno.capabilities.contains(&"uart.read"));
        assert!(uno.capabilities.contains(&"can.open"));
        assert!(uno.capabilities.contains(&"can.write"));
        assert!(uno.capabilities.contains(&"can.read"));
        assert!(uno.capabilities.contains(&"storage.write"));
        assert!(uno.capabilities.contains(&"storage.read"));
        assert!(uno.capabilities.contains(&"storage.size"));
        assert!(uno.capabilities.contains(&"program.store"));
    }

    #[test]
    fn registry_exposes_wireless_descriptors_for_wireless_boards() {
        let uno_r4_minima = find_target("arduino-uno-r4-minima").unwrap();
        let uno_r4_wifi = find_target("arduino-uno-r4-wifi").unwrap();
        let uno_r3 = find_target("arduino-uno-r3").unwrap();
        let mkr_wifi = find_target("arduino-mkr-wifi-1010").unwrap();
        let nano_r4 = find_target("arduino-nano-r4").unwrap();
        let nano_iot = find_target("arduino-nano-33-iot").unwrap();
        let nano_ble = find_target("arduino-nano-33-ble-rev2").unwrap();
        let nano_rp2040 = find_target("arduino-nano-rp2040-connect").unwrap();
        let nano_esp32 = find_target("arduino-nano-esp32").unwrap();
        let giga = find_target("arduino-giga-r1-wifi").unwrap();
        let portenta_h7_lite = find_target("arduino-portenta-h7-lite").unwrap();
        let portenta_h7_lite_connected = find_target("arduino-portenta-h7-lite-connected").unwrap();
        let portenta_c33 = find_target("arduino-portenta-c33").unwrap();
        let nicla_vision = find_target("arduino-nicla-vision").unwrap();
        let nicla_sense = find_target("arduino-nicla-sense-me").unwrap();
        let nicla_voice = find_target("arduino-nicla-voice").unwrap();
        let opta_rs485 = find_target("arduino-opta-rs485").unwrap();
        let opta_wifi = find_target("arduino-opta-wifi").unwrap();
        let esp32 = find_target("esp32-devkit-v1").unwrap();
        let pico = find_target("raspberry-pi-pico").unwrap();
        let pico_w = find_target("raspberry-pi-pico-w").unwrap();

        assert!(uno_r4_minima.wireless.is_empty());
        assert!(uno_r4_minima.network_interfaces.is_empty());
        assert!(uno_r3.wireless.is_empty());
        assert!(nano_r4.wireless.is_empty());
        assert!(portenta_h7_lite.wireless.is_empty());
        assert!(opta_rs485.wireless.is_empty());
        assert!(pico.wireless.is_empty());
        assert!(uno_r4_wifi.capabilities.contains(&"transport.wifi"));
        assert!(uno_r4_wifi.capabilities.contains(&"transport.bluetooth_le"));
        assert!(uno_r4_wifi.capabilities.contains(&"led_matrix.frame"));
        assert!(!uno_r4_wifi
            .capabilities
            .contains(&"transport.bluetooth_classic"));
        assert!(esp32.capabilities.contains(&"transport.bluetooth_classic"));
        assert!(pico_w.capabilities.contains(&"transport.wifi"));
        assert!(pico_w.capabilities.contains(&"ota.wifi"));
        assert!(pico_w
            .wireless
            .iter()
            .any(|interface| interface.transport == WirelessTransport::Wifi
                && interface.command_transport
                && interface.ota_update));

        for target in [
            mkr_wifi,
            nano_iot,
            nano_rp2040,
            nano_esp32,
            giga,
            portenta_h7_lite_connected,
            portenta_c33,
            nicla_vision,
            opta_wifi,
        ] {
            assert!(target
                .wireless
                .iter()
                .any(|interface| interface.transport == WirelessTransport::Wifi
                    && !interface.command_transport
                    && !interface.ota_update));
            assert!(!target.capabilities.contains(&"transport.wifi"));
        }

        for target in [nano_ble, nicla_sense, nicla_voice] {
            assert!(target.wireless.iter().any(|interface| interface.transport
                == WirelessTransport::BluetoothLe
                && !interface.command_transport
                && !interface.ota_update));
            assert!(!target.capabilities.contains(&"transport.bluetooth_le"));
            assert!(target.network_interfaces.is_empty());
        }

        assert_eq!(nano_iot.wireless, &ARDUINO_NINA_W102_WIRELESS);
        assert_eq!(nano_ble.wireless, &ARDUINO_NRF52840_WIRELESS);
        assert_eq!(nano_esp32.wireless, &ARDUINO_NANO_ESP32_WIRELESS);
        assert_eq!(portenta_c33.wireless, &ARDUINO_ESP32_C3_WIRELESS);
        assert_eq!(nicla_sense.wireless, &ARDUINO_NRF52832_WIRELESS);
    }

    #[test]
    fn registry_exposes_shared_arduino_network_interface_metadata() {
        let mkr_wifi = find_target("arduino-mkr-wifi-1010").unwrap();
        let nano_iot = find_target("arduino-nano-33-iot").unwrap();
        let nano_ble = find_target("arduino-nano-33-ble-rev2").unwrap();
        let nano_rp2040 = find_target("arduino-nano-rp2040-connect").unwrap();
        let nano_esp32 = find_target("arduino-nano-esp32").unwrap();
        let giga = find_target("arduino-giga-r1-wifi").unwrap();
        let portenta_c33 = find_target("arduino-portenta-c33").unwrap();
        let nicla_vision = find_target("arduino-nicla-vision").unwrap();
        let opta_wifi = find_target("arduino-opta-wifi").unwrap();

        assert_eq!(
            mkr_wifi.network_interfaces,
            &ARDUINO_NINA_W102_NETWORK_INTERFACES
        );
        assert_eq!(
            nano_iot.network_interfaces,
            &ARDUINO_NINA_W102_NETWORK_INTERFACES
        );
        assert_eq!(
            nano_rp2040.network_interfaces,
            &ARDUINO_NINA_W102_NETWORK_INTERFACES
        );
        assert_eq!(
            nano_esp32.network_interfaces,
            &ARDUINO_NANO_ESP32_NETWORK_INTERFACES
        );
        assert_eq!(
            portenta_c33.network_interfaces,
            &ARDUINO_ESP32_C3_NETWORK_INTERFACES
        );

        for target in [giga, nicla_vision, opta_wifi] {
            assert_eq!(
                target.network_interfaces,
                &ARDUINO_WIFI_BLE_MODULE_NETWORK_INTERFACES
            );
        }

        assert!(nano_ble.network_interfaces.is_empty());

        for target in [
            mkr_wifi,
            nano_iot,
            nano_rp2040,
            nano_esp32,
            giga,
            portenta_c33,
            nicla_vision,
            opta_wifi,
        ] {
            let interface = target.network_interfaces[0];
            assert_eq!(interface.interface, 0);
            assert_eq!(interface.transport, WirelessTransport::Wifi);
            assert_eq!(interface.protocols, &UNO_R4_WIFI_NETWORK_PROTOCOLS);
            assert_eq!(interface.max_sockets, 0);
            assert!(interface.notes.contains("metadata only"));
            assert!(!target.capabilities.contains(&"network.ipv4"));
            assert!(!target.capabilities.contains(&"network.tcp"));
            assert!(!target.capabilities.contains(&"network.udp"));
            assert!(!target.capabilities.contains(&"network.dns"));
        }
    }

    #[test]
    fn registry_exposes_uno_r4_wifi_network_metadata() {
        let uno_r4_wifi = find_target("arduino-uno-r4-wifi").unwrap();

        assert!(uno_r4_wifi.capabilities.contains(&"network.ipv4"));
        assert!(uno_r4_wifi.capabilities.contains(&"network.tcp"));
        assert!(uno_r4_wifi.capabilities.contains(&"network.udp"));
        assert!(uno_r4_wifi.capabilities.contains(&"network.dns"));
        assert!(uno_r4_wifi.capabilities.contains(&"network.tcp.open"));
        assert!(uno_r4_wifi.capabilities.contains(&"network.tcp.write"));
        assert!(uno_r4_wifi.capabilities.contains(&"network.tcp.read"));
        assert!(uno_r4_wifi.capabilities.contains(&"network.tcp.close"));
        assert!(uno_r4_wifi.capabilities.contains(&"network.tcp.connected"));
        assert!(uno_r4_wifi.capabilities.contains(&"network.tcp.available"));
        assert!(uno_r4_wifi.capabilities.contains(&"network.udp.open"));
        assert!(uno_r4_wifi.capabilities.contains(&"network.udp.write"));
        assert!(uno_r4_wifi.capabilities.contains(&"network.udp.read"));
        assert!(uno_r4_wifi
            .capabilities
            .contains(&"network.udp.write_bytes"));
        assert!(uno_r4_wifi.capabilities.contains(&"network.udp.read_bytes"));
        assert!(uno_r4_wifi.capabilities.contains(&"network.udp.available"));
        assert!(uno_r4_wifi.capabilities.contains(&"network.udp.close"));
        assert!(uno_r4_wifi.capabilities.contains(&"network.wifi.associate"));
        assert!(uno_r4_wifi
            .capabilities
            .contains(&"network.wifi.disconnect"));
        assert!(uno_r4_wifi.capabilities.contains(&"network.wifi.status"));
        assert!(uno_r4_wifi.capabilities.contains(&"network.dns.resolve"));
        assert!(uno_r4_wifi.capabilities.contains(&"network.dns.set_server"));
        assert!(uno_r4_wifi.capabilities.contains(&"network.dns.query"));
        assert!(uno_r4_wifi
            .capabilities
            .contains(&"network.dns.response_ipv4"));
        assert!(uno_r4_wifi
            .capabilities
            .contains(&"network.dns.exchange_udp"));
        assert!(uno_r4_wifi
            .capabilities
            .contains(&"network.dns.exchange_udp_retry"));
        assert!(uno_r4_wifi
            .capabilities
            .contains(&"network.dns.exchange_udp_fallback"));
        assert_eq!(
            uno_r4_wifi.network_interfaces,
            &UNO_R4_WIFI_NETWORK_INTERFACES
        );

        let interface = uno_r4_wifi.network_interfaces[0];
        assert_eq!(interface.interface, 0);
        assert_eq!(interface.name, "WiFiS3");
        assert_eq!(interface.transport, WirelessTransport::Wifi);
        assert_eq!(interface.chip, "ESP32-S3 coprocessor");
        assert_eq!(
            interface.protocols,
            &[
                NetworkProtocol::Ipv4,
                NetworkProtocol::Tcp,
                NetworkProtocol::Udp,
                NetworkProtocol::Dns,
            ]
        );
        assert_eq!(interface.max_sockets, 4);
        assert!(interface.notes.contains("COBS/CRC"));
    }
}
