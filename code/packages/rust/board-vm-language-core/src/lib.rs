//! Rust-owned Board VM host protocol boundary for language frontends.
//!
//! Ruby, Python, Lua, Java, and similar frontends should be thin syntax layers
//! over this crate. The binary protocol, request ids, BVM module shape, COBS
//! framing, and CRC checks stay in Rust, where the board firmware and host CLI
//! already share the same implementation.

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::env;
use std::ffi::CString;
use std::fs;
use std::panic::{self, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::ptr;
use std::slice;
use std::str;

use board_vm_bluetooth::{
    board_vm_endpoint_candidates as board_vm_bluetooth_endpoint_candidates,
    discover_bluetooth_devices as discover_board_vm_bluetooth_devices, macos_rfcomm_device_path,
    open_bluetooth_endpoint, parse_bluetooth_endpoint as parse_board_vm_bluetooth_endpoint,
    BluetoothBackend, BluetoothDiscoveredDevice, BluetoothEndpoint, BluetoothEndpointCandidate,
    BluetoothOpenError,
};
#[cfg(target_os = "macos")]
use board_vm_bluetooth::{
    MacosBluetoothBackend, MacosCoreBluetoothRuntimeBleConnector, MacosDevRfcommDeviceResolver,
    MacosRfcommDeviceResolver,
};
use board_vm_client::{RawFrameTransport, TransportError};
use board_vm_esp_rom::{
    DEFAULT_BAUD_RATE as ESP_DEFAULT_BAUD_RATE,
    DEFAULT_FLASH_BLOCK_SIZE as ESP_DEFAULT_FLASH_BLOCK_SIZE,
    DEFAULT_TIMEOUT_MS as ESP_DEFAULT_TIMEOUT_MS,
};
use board_vm_host::{
    i2c_transfer_module_len, i2c_write_module_len, spi_transfer_module_len, spi_write_module_len,
    storage_write_module_len, write_adc_read_module, write_blink_module, write_can_open_module,
    write_can_read_module, write_can_write_module, write_dac_write_u12_module,
    write_gpio_handle_close_module, write_gpio_handle_read_module, write_gpio_handle_write_module,
    write_gpio_open_module, write_gpio_read_module, write_gpio_write_module, write_i2c_open_module,
    write_i2c_read_module, write_i2c_read_u8_module, write_i2c_transfer_module,
    write_i2c_write_module, write_i2c_write_u8_module, write_led_matrix_frame_module, write_module,
    write_pwm_write_module, write_rtc_now_module, write_rtc_set_module, write_spi_open_module,
    write_spi_read_module, write_spi_transfer_module, write_spi_write_module,
    write_storage_read_module, write_storage_size_module, write_storage_write_module,
    write_time_now_module, write_time_sleep_ms_module, write_uart_open_module,
    write_uart_read_module, write_uart_write_module, write_watchdog_configure_module,
    write_watchdog_kick_module, AdcReadProgram, BlinkProgram, CanOpenProgram, CanReadProgram,
    CanWriteProgram, DacWriteU12Program, GpioHandleCloseProgram, GpioHandleReadProgram,
    GpioHandleWriteProgram, GpioOpenProgram, GpioReadProgram, GpioWriteProgram, HostError,
    HostSession, I2cOpenProgram, I2cReadProgram, I2cReadU8Program, I2cTransferProgram,
    I2cWriteProgram, I2cWriteU8Program, LedMatrixFrameProgram, ModuleSpec, PwmWriteProgram,
    RtcNowProgram, RtcSetProgram, SpiOpenProgram, SpiReadProgram, SpiTransferProgram,
    SpiWriteProgram, StorageReadProgram, StorageSizeProgram, StorageWriteProgram, TimeNowProgram,
    TimeSleepMsProgram, UartOpenProgram, UartReadProgram, UartWriteProgram,
    WatchdogConfigureProgram, WatchdogKickProgram, ADC_READ_MODULE_LEN, BLINK_MODULE_LEN,
    CAN_OPEN_MODULE_LEN, CAN_READ_MODULE_LEN, CAN_WRITE_MODULE_LEN, DAC_WRITE_U12_MODULE_LEN,
    DEFAULT_INSTRUCTION_BUDGET, DEFAULT_PROGRAM_ID, DEFAULT_RUN_FLAGS,
    GPIO_HANDLE_CLOSE_MODULE_LEN, GPIO_HANDLE_READ_MODULE_LEN, GPIO_HANDLE_WRITE_MODULE_LEN,
    GPIO_OPEN_MODULE_LEN, GPIO_READ_MODULE_LEN, GPIO_WRITE_MODULE_LEN, I2C_OPEN_MODULE_LEN,
    I2C_READ_MODULE_LEN, I2C_READ_U8_MODULE_LEN, I2C_WRITE_U8_MODULE_LEN,
    LED_MATRIX_FRAME_MODULE_LEN, PWM_WRITE_MODULE_LEN, RTC_NOW_MODULE_LEN, RTC_SET_MODULE_LEN,
    SPI_OPEN_MODULE_LEN, SPI_READ_MODULE_LEN, STORAGE_READ_MODULE_LEN, STORAGE_SIZE_MODULE_LEN,
    TIME_NOW_MODULE_LEN, TIME_SLEEP_MS_MODULE_LEN, UART_OPEN_MODULE_LEN, UART_READ_MODULE_LEN,
    UART_WRITE_MODULE_LEN, WATCHDOG_CONFIGURE_MODULE_LEN, WATCHDOG_KICK_MODULE_LEN,
};
use board_vm_protocol::{
    decode_caps_report_header, decode_error_payload, decode_frame, decode_hello_ack,
    decode_program_begin, decode_program_chunk, decode_program_end, decode_run_report_header,
    decode_wire_frame, encode_wire_frame, Frame, MessageType, ProgramFormat, ProtocolError,
    RunStatus, Value as ProtocolValue, CAP_FLAG_BOARD_METADATA, CAP_FLAG_BYTECODE_CALLABLE,
    CAP_FLAG_PROTOCOL_FEATURE, FLAG_IS_ERROR_RESPONSE, FLAG_IS_RESPONSE,
};
use board_vm_targets::{
    all_targets, BoardFamily, BoardTargetInfo, CanBusInfo as TargetCanBus,
    DigitalPinInfo as TargetDigitalPin, I2cBusInfo as TargetI2cBus,
    I2cConnectorInfo as TargetI2cConnector, NetworkInterfaceInfo as TargetNetworkInterface,
    NetworkProtocol as TargetNetworkProtocol, OnboardLed as TargetOnboardLed, RtcInfo as TargetRtc,
    SpiBusInfo as TargetSpiBus, UartBusInfo as TargetUartBus, UploadAdapter as TargetUploadAdapter,
    UploadImageFormat as TargetUploadImageFormat, UploadInfo as TargetUploadInfo,
    UploadPortHint as TargetUploadPortHint, UploadResetMethod as TargetUploadResetMethod,
    UploadTransport as TargetUploadTransport, UsbInterfaceInfo as TargetUsbInterface,
    WirelessInterfaceInfo as TargetWirelessInterface, WirelessTransport as TargetWirelessTransport,
};

pub const LANGUAGE_CORE_VERSION_MAJOR: u16 = 0;
pub const LANGUAGE_CORE_VERSION_MINOR: u16 = 1;
pub const LANGUAGE_CORE_VERSION_PATCH: u16 = 0;
pub const LANGUAGE_DEFAULT_RUN_FLAGS: u8 = DEFAULT_RUN_FLAGS;
pub const LANGUAGE_RUN_FLAG_RESET_VM_BEFORE_RUN: u8 =
    board_vm_protocol::RUN_FLAG_RESET_VM_BEFORE_RUN;
pub const LANGUAGE_RUN_FLAG_KEEP_HANDLES_AFTER_RUN: u8 =
    board_vm_protocol::RUN_FLAG_KEEP_HANDLES_AFTER_RUN;
pub const LANGUAGE_RUN_FLAG_BACKGROUND_RUN: u8 = board_vm_protocol::RUN_FLAG_BACKGROUND_RUN;
pub const LANGUAGE_ESP_DEFAULT_FLASH_OFFSET: u32 = 0x1000;
pub const LANGUAGE_ESP_DEFAULT_FLASH_SIZE: u32 = 4 * 1024 * 1024;
pub const LANGUAGE_BOARD_VM_WIRE_PROTOCOL: &str = "board_vm_cobs_crc";
pub const LANGUAGE_SERIAL_DEFAULT_BAUD_RATE: u32 = 115_200;
pub const LANGUAGE_SERIAL_DEFAULT_TIMEOUT_MS: u64 = 1_000;
pub const LANGUAGE_SERIAL_OPEN_SETTLE_MS: u64 = 250;
pub const LANGUAGE_INPUT_CALLBACK_DEFAULT_DEBOUNCE_MS: u16 = 25;
pub const LANGUAGE_INPUT_CALLBACK_DEFAULT_QUEUE_CAPACITY: u8 = 8;
pub const LANGUAGE_INPUT_CALLBACK_DISPATCH_MODEL: &str = "cooperative_event_queue";
pub const LANGUAGE_INPUT_CALLBACK_EVENT_KIND: &str = "digital_input_change";
pub const LANGUAGE_INPUT_CALLBACK_DISPATCH_REASON: &str = "queued_input_event";
pub const ARDUINO_CLI_NATIVE_USB_BOOTLOADER_TOUCH_BAUD: u32 = 1_200;
pub const ARDUINO_CLI_UPLOAD_PORT_PLACEHOLDER: &str = "<port>";
pub const ARDUINO_CLI_UPLOAD_INPUT_FILE_PLACEHOLDER: &str = "<firmware-image>";
pub const ARDUINO_CLI_UPLOAD_PROCESS_STDIN_MODE: &str = "null";
pub const ARDUINO_CLI_UPLOAD_PROCESS_STDOUT_MODE: &str = "piped";
pub const ARDUINO_CLI_UPLOAD_PROCESS_STDERR_MODE: &str = "piped";

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoardVmLanguageStatusCode {
    Ok = 0,
    NullPointer = 1,
    InvalidUtf8 = 2,
    ValueTooLarge = 3,
    OutputTooSmall = 4,
    ProtocolError = 5,
    HostError = 6,
    Panic = 7,
    BluetoothEndpointError = 8,
    BluetoothOpenError = 9,
    TransportError = 10,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BoardVmLanguageStatus {
    pub code: u32,
    pub len: u64,
    pub request_id: u16,
    pub message_type: u8,
    pub flags: u8,
    pub payload_offset: u64,
    pub payload_len: u64,
}

impl BoardVmLanguageStatus {
    pub const fn ok() -> Self {
        Self {
            code: BoardVmLanguageStatusCode::Ok as u32,
            len: 0,
            request_id: 0,
            message_type: 0,
            flags: 0,
            payload_offset: 0,
            payload_len: 0,
        }
    }

    pub const fn err(code: BoardVmLanguageStatusCode) -> Self {
        Self {
            code: code as u32,
            len: 0,
            request_id: 0,
            message_type: 0,
            flags: 0,
            payload_offset: 0,
            payload_len: 0,
        }
    }

    fn written(request_id: u16, len: usize) -> Self {
        Self {
            len: len as u64,
            request_id,
            ..Self::ok()
        }
    }

    fn decoded(frame: &Frame<'_>, raw_base: *const u8, raw_len: usize) -> Self {
        let payload_offset = frame.payload.as_ptr() as usize - raw_base as usize;
        Self {
            len: raw_len as u64,
            request_id: frame.request_id,
            message_type: frame.message_type.0,
            flags: frame.flags,
            payload_offset: payload_offset as u64,
            payload_len: frame.payload.len() as u64,
            code: BoardVmLanguageStatusCode::Ok as u32,
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BoardVmLanguageSession {
    next_request_id: u16,
}

impl BoardVmLanguageSession {
    pub const fn new() -> Self {
        Self { next_request_id: 1 }
    }

    pub const fn with_next_request_id(next_request_id: u16) -> Self {
        Self {
            next_request_id: if next_request_id == 0 {
                1
            } else {
                next_request_id
            },
        }
    }

    pub const fn next_request_id(&self) -> u16 {
        self.next_request_id
    }

    fn host_session(&self) -> HostSession {
        HostSession::with_next_request_id(self.next_request_id)
    }

    fn update_from_host_session(&mut self, host: &HostSession) {
        self.next_request_id = host.next_request_id();
    }
}

impl Default for BoardVmLanguageSession {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BuiltWireFrame {
    pub request_id: u16,
    pub len: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedLanguageResponse {
    pub request_id: u16,
    pub message_type: MessageType,
    pub flags: u8,
    pub payload_len: usize,
    pub body: DecodedLanguageResponseBody,
}

impl DecodedLanguageResponse {
    pub const fn is_response(&self) -> bool {
        self.flags & FLAG_IS_RESPONSE != 0
    }

    pub const fn is_error_response(&self) -> bool {
        self.flags & FLAG_IS_ERROR_RESPONSE != 0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecodedLanguageResponseBody {
    HelloAck(LanguageHelloAck),
    CapsReport(LanguageBoardDescriptor),
    ProgramBegin(LanguageProgramBegin),
    ProgramChunk(LanguageProgramChunk),
    ProgramEnd(LanguageProgramEnd),
    RunReport(LanguageRunReport),
    Error(LanguageBoardError),
    Raw,
}

impl DecodedLanguageResponseBody {
    pub const fn kind(&self) -> &'static str {
        match self {
            Self::HelloAck(_) => "hello_ack",
            Self::CapsReport(_) => "caps_report",
            Self::ProgramBegin(_) => "program_begin",
            Self::ProgramChunk(_) => "program_chunk",
            Self::ProgramEnd(_) => "program_end",
            Self::RunReport(_) => "run_report",
            Self::Error(_) => "error",
            Self::Raw => "raw",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanguageHelloAck {
    pub selected_version: u8,
    pub board_name: String,
    pub runtime_name: String,
    pub host_nonce: u32,
    pub board_nonce: u32,
    pub max_frame_payload: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanguageCapability {
    pub id: u16,
    pub version: u8,
    pub flags: u16,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanguageBoardDescriptor {
    pub board_id: String,
    pub runtime_id: String,
    pub max_program_bytes: u32,
    pub max_stack_values: u8,
    pub max_handles: u8,
    pub supports_store_program: bool,
    pub capabilities: Vec<LanguageCapability>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LanguageBoardFamily {
    Arduino,
    ArduinoUnoR4,
    Esp32,
    RaspberryPiPico,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LanguageOnboardLed {
    Gpio(u8),
    WirelessChipGpio(u8),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LanguageWirelessTransport {
    Wifi,
    BluetoothLe,
    BluetoothClassic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LanguageConnectionTransport {
    Serial,
    Wifi,
    BluetoothLe,
    BluetoothClassic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LanguageHostEndpointTransport {
    SerialPort,
    TcpSocket,
    BluetoothLeGatt,
    BluetoothClassicRfcomm,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanguageWirelessInterface {
    pub transport: LanguageWirelessTransport,
    pub chip: String,
    pub command_transport: bool,
    pub ota_update: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LanguageNetworkProtocol {
    Ipv4,
    Tcp,
    Udp,
    Dns,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanguageNetworkInterface {
    pub interface: u8,
    pub name: String,
    pub transport: LanguageWirelessTransport,
    pub chip: String,
    pub protocols: Vec<LanguageNetworkProtocol>,
    pub max_sockets: u8,
    pub notes: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanguageConnectionOption {
    pub transport: LanguageConnectionTransport,
    pub display_name: String,
    pub command_transport: bool,
    pub ota_update: bool,
    pub requires: String,
    pub endpoint_transport: LanguageHostEndpointTransport,
    pub endpoint_scheme: String,
    pub wire_protocol: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanguageSerialEndpoint {
    pub endpoint: String,
    pub transport: LanguageConnectionTransport,
    pub endpoint_transport: LanguageHostEndpointTransport,
    pub endpoint_scheme: String,
    pub port: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanguageTcpEndpoint {
    pub endpoint: String,
    pub transport: LanguageConnectionTransport,
    pub endpoint_transport: LanguageHostEndpointTransport,
    pub endpoint_scheme: String,
    pub authority: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanguageHostEndpointSummary {
    pub endpoint: String,
    pub transport: LanguageConnectionTransport,
    pub endpoint_transport: LanguageHostEndpointTransport,
    pub endpoint_scheme: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanguageHostEndpointSessionSummary {
    pub endpoint: LanguageHostEndpointSummary,
    pub connection_label: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LanguageHostEndpointParseErrorKind {
    InvalidSerialEndpoint,
    InvalidTcpEndpoint,
    InvalidBluetoothEndpoint,
    UnsupportedScheme,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanguageHostEndpointParseError {
    pub endpoint: String,
    pub kind: LanguageHostEndpointParseErrorKind,
    pub scheme: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanguageBluetoothEndpoint {
    pub endpoint: String,
    pub transport: LanguageConnectionTransport,
    pub endpoint_transport: LanguageHostEndpointTransport,
    pub endpoint_scheme: String,
    pub device: String,
    pub service_uuid: Option<String>,
    pub write_characteristic_uuid: Option<String>,
    pub notify_characteristic_uuid: Option<String>,
    pub channel: Option<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanguageBluetoothDiscoveredDevice {
    pub id: String,
    pub name: Option<String>,
    pub address: Option<String>,
    pub paired: bool,
    pub service_uuids: Vec<String>,
    pub characteristic_uuids: Vec<String>,
    pub board_vm_rfcomm_channels: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanguageBluetoothEndpointCandidate {
    pub endpoint: LanguageBluetoothEndpoint,
    pub device: String,
    pub display_name: String,
    pub paired: bool,
    pub requires_pairing: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanguageBluetoothBackendOpenPlan {
    pub endpoint: LanguageBluetoothEndpoint,
    pub backend: String,
    pub status: String,
    pub stream_path: Option<String>,
    pub native_transport: bool,
    pub message: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanguageSerialRuntimeOpenPlan {
    pub board_id: String,
    pub port: String,
    pub port_source: String,
    pub endpoint: String,
    pub transport: LanguageConnectionTransport,
    pub endpoint_transport: LanguageHostEndpointTransport,
    pub endpoint_scheme: String,
    pub wire_protocol: String,
    pub baud_rate: u32,
    pub timeout_ms: u64,
    pub data_bits: u8,
    pub parity: String,
    pub stop_bits: u8,
    pub flow_control: String,
    pub dtr_on_open: bool,
    pub clear_on_open: bool,
    pub settle_on_open_ms: u64,
    pub hello_after_open: bool,
    pub upload_port_hint: Option<String>,
    pub notes: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanguageTargetInfo {
    pub board_id: String,
    pub display_name: String,
    pub family: LanguageBoardFamily,
    pub runtime_id: String,
    pub mcu: String,
    pub core: String,
    pub rust_target: String,
    pub clock_hz: u32,
    pub operating_voltage_mv: u16,
    pub onboard_led: Option<LanguageOnboardLed>,
    pub led_matrix: Option<LanguageLedMatrix>,
    pub digital_pin_count: usize,
    pub digital_pins: Vec<LanguageDigitalPin>,
    pub i2c_buses: Vec<LanguageI2cBus>,
    pub i2c_connectors: Vec<LanguageI2cConnector>,
    pub spi_buses: Vec<LanguageSpiBus>,
    pub uart_buses: Vec<LanguageUartBus>,
    pub usb_interfaces: Vec<LanguageUsbInterface>,
    pub can_buses: Vec<LanguageCanBus>,
    pub rtc: Option<LanguageRtc>,
    pub wireless: Vec<LanguageWirelessInterface>,
    pub network_interfaces: Vec<LanguageNetworkInterface>,
    pub connection_options: Vec<LanguageConnectionOption>,
    pub upload: Option<LanguageUploadOptions>,
    pub capabilities: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanguageI2cBus {
    pub bus: u8,
    pub name: String,
    pub sda_pin: u8,
    pub scl_pin: u8,
    pub qwiic: bool,
    pub notes: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanguageI2cConnector {
    pub bus: u8,
    pub name: String,
    pub connector: String,
    pub arduino_object: String,
    pub controller: String,
    pub notes: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanguageSpiBus {
    pub bus: u8,
    pub name: String,
    pub copi_pin: u8,
    pub cipo_pin: u8,
    pub sck_pin: u8,
    pub default_cs_pin: u8,
    pub notes: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanguageUartBus {
    pub bus: u8,
    pub name: String,
    pub tx_pin: u8,
    pub rx_pin: u8,
    pub arduino_uart: u8,
    pub internal: bool,
    pub notes: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanguageUsbInterface {
    pub interface: u8,
    pub name: String,
    pub controller: String,
    pub class: String,
    pub native: bool,
    pub upload: bool,
    pub command_transport: bool,
    pub notes: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanguageCanBus {
    pub bus: u8,
    pub name: String,
    pub tx_pin: u8,
    pub rx_pin: u8,
    pub controller: String,
    pub notes: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanguageRtc {
    pub instance: u8,
    pub name: String,
    pub peripheral: String,
    pub notes: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanguageDigitalPin {
    pub pin: u8,
    pub label: String,
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
    pub notes: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LanguageInputCallbackTrigger {
    RisingEdge,
    FallingEdge,
    Change,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LanguageInputCallbackPull {
    Floating,
    PullUp,
    PullDown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LanguageInputCallbackQueuePolicy {
    DropNewest,
    DropOldest,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LanguageInputCallbackOptions {
    pub trigger: LanguageInputCallbackTrigger,
    pub pull: LanguageInputCallbackPull,
    pub debounce_ms: u16,
    pub queue_capacity: u8,
    pub queue_policy: LanguageInputCallbackQueuePolicy,
    pub callback_program_id: u16,
    pub callback_instruction_budget: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanguageInputCallbackPlan {
    pub board_id: String,
    pub pin: u8,
    pub label: String,
    pub trigger: LanguageInputCallbackTrigger,
    pub pull: LanguageInputCallbackPull,
    pub debounce_ms: u16,
    pub queue_capacity: u8,
    pub queue_policy: LanguageInputCallbackQueuePolicy,
    pub callback_program_id: u16,
    pub callback_instruction_budget: u32,
    pub interrupt_backed: bool,
    pub dispatch_model: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LanguageInputCallbackPlanErrorKind {
    UnknownTarget,
    UnknownPin,
    PinDoesNotSupportInput,
    PinDoesNotSupportInterrupt,
    PinDoesNotSupportPull,
    EmptyQueue,
    EmptyCallbackBudget,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanguageInputCallbackPlanError {
    pub selector: String,
    pub pin: u8,
    pub kind: LanguageInputCallbackPlanErrorKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanguageInputCallbackPlanDiagnostic {
    pub selector: String,
    pub pin: u8,
    pub kind: LanguageInputCallbackPlanErrorKind,
    pub kind_name: String,
    pub diagnostic_label: String,
    pub message: String,
    pub error: LanguageInputCallbackPlanError,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LanguageInputCallbackLevel {
    Low,
    High,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanguageInputCallbackEvent {
    pub board_id: String,
    pub pin: u8,
    pub label: String,
    pub event_kind: String,
    pub trigger: LanguageInputCallbackTrigger,
    pub level: LanguageInputCallbackLevel,
    pub sequence: u32,
    pub timestamp_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanguageInputCallbackInvocation {
    pub board_id: String,
    pub pin: u8,
    pub label: String,
    pub event_kind: String,
    pub trigger: LanguageInputCallbackTrigger,
    pub level: LanguageInputCallbackLevel,
    pub callback_program_id: u16,
    pub callback_instruction_budget: u32,
    pub sequence: u32,
    pub timestamp_ms: u64,
    pub debounce_ms: u16,
    pub queue_capacity: u8,
    pub queue_policy: LanguageInputCallbackQueuePolicy,
    pub interrupt_backed: bool,
    pub dispatch_model: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LanguageInputCallbackQueueAction {
    Enqueue,
    DropNewest,
    DropOldestThenEnqueue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanguageInputCallbackQueuePlan {
    pub board_id: String,
    pub pin: u8,
    pub label: String,
    pub event_kind: String,
    pub callback_program_id: u16,
    pub callback_instruction_budget: u32,
    pub sequence: u32,
    pub timestamp_ms: u64,
    pub debounce_ms: u16,
    pub queue_capacity: u8,
    pub queue_depth_before: u8,
    pub queue_depth_after: u8,
    pub queue_policy: LanguageInputCallbackQueuePolicy,
    pub action: LanguageInputCallbackQueueAction,
    pub queued: bool,
    pub dropped_existing_event: bool,
    pub dropped_incoming_event: bool,
    pub dispatch_required: bool,
    pub interrupt_backed: bool,
    pub dispatch_model: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanguageInputCallbackSessionQueueSummary {
    pub endpoint: LanguageHostEndpointSummary,
    pub connection_label: String,
    pub queue_label: String,
    pub action: LanguageInputCallbackQueueAction,
    pub queue_policy: LanguageInputCallbackQueuePolicy,
    pub queued: bool,
    pub dropped_existing_event: bool,
    pub dropped_incoming_event: bool,
    pub dispatch_required: bool,
    pub queue_depth_before: u8,
    pub queue_depth_after: u8,
    pub message: String,
    pub queue_plan: LanguageInputCallbackQueuePlan,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanguageInputCallbackDispatchPlan {
    pub board_id: String,
    pub pin: u8,
    pub label: String,
    pub event_kind: String,
    pub dispatch_reason: String,
    pub callback_program_id: u16,
    pub callback_instruction_budget: u32,
    pub sequence: u32,
    pub timestamp_ms: u64,
    pub queue_depth_after: u8,
    pub queue_action: LanguageInputCallbackQueueAction,
    pub dropped_existing_event: bool,
    pub interrupt_backed: bool,
    pub dispatch_model: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanguageInputCallbackSessionDispatchSummary {
    pub endpoint: LanguageHostEndpointSummary,
    pub connection_label: String,
    pub dispatch_label: String,
    pub dispatch_reason: String,
    pub callback_program_id: u16,
    pub callback_instruction_budget: u32,
    pub sequence: u32,
    pub queue_depth_after: u8,
    pub queue_action: LanguageInputCallbackQueueAction,
    pub dropped_existing_event: bool,
    pub interrupt_backed: bool,
    pub dispatch_model: String,
    pub message: String,
    pub dispatch_plan: LanguageInputCallbackDispatchPlan,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LanguageInputCallbackResultKind {
    Completed,
    BudgetExceeded,
    Incomplete,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanguageInputCallbackResultSummary {
    pub board_id: String,
    pub pin: u8,
    pub label: String,
    pub event_kind: String,
    pub dispatch_reason: String,
    pub callback_program_id: u16,
    pub callback_instruction_budget: u32,
    pub sequence: u32,
    pub timestamp_ms: u64,
    pub queue_depth_after: u8,
    pub queue_action: LanguageInputCallbackQueueAction,
    pub dropped_existing_event: bool,
    pub interrupt_backed: bool,
    pub dispatch_model: String,
    pub run_status: String,
    pub result_kind: LanguageInputCallbackResultKind,
    pub instructions_executed: u32,
    pub elapsed_ms: u32,
    pub completed: bool,
    pub budget_exceeded: bool,
    pub retryable: bool,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanguageInputCallbackTransportResultSummary {
    pub endpoint: LanguageHostEndpointSummary,
    pub connection_label: String,
    pub callback_label: String,
    pub result: LanguageInputCallbackResultSummary,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LanguageInputCallbackCompletionAction {
    Complete,
    KeepRunning,
    DropAfterBudgetExceeded,
    DropAfterFailure,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanguageInputCallbackCompletionPlan {
    pub action: LanguageInputCallbackCompletionAction,
    pub remove_from_queue: bool,
    pub keep_dispatch_scheduled: bool,
    pub terminal: bool,
    pub retryable: bool,
    pub queue_depth_after_completion: u8,
    pub message: String,
    pub result: LanguageInputCallbackResultSummary,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanguageInputCallbackSessionCompletionSummary {
    pub endpoint: LanguageHostEndpointSummary,
    pub connection_label: String,
    pub callback_label: String,
    pub completion_label: String,
    pub action: LanguageInputCallbackCompletionAction,
    pub remove_from_queue: bool,
    pub keep_dispatch_scheduled: bool,
    pub terminal: bool,
    pub retryable: bool,
    pub queue_depth_after_completion: u8,
    pub result: LanguageInputCallbackResultSummary,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanguageInputCallbackSessionLifecycleSummary {
    pub endpoint: LanguageHostEndpointSummary,
    pub connection_label: String,
    pub lifecycle_label: String,
    pub queued: bool,
    pub dispatch_required: bool,
    pub terminal: bool,
    pub retryable: bool,
    pub queue_summary: LanguageInputCallbackSessionQueueSummary,
    pub dispatch_summary: Option<LanguageInputCallbackSessionDispatchSummary>,
    pub result: Option<LanguageInputCallbackResultSummary>,
    pub completion_summary: Option<LanguageInputCallbackSessionCompletionSummary>,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LanguageInputCallbackTransportAction {
    DropBeforeDispatch,
    DispatchCallback,
    CompleteCallback,
    KeepCallbackRunning,
    DropAfterBudgetExceeded,
    DropAfterFailure,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanguageInputCallbackTransportActionSummary {
    pub endpoint: LanguageHostEndpointSummary,
    pub connection_label: String,
    pub action_label: String,
    pub action: LanguageInputCallbackTransportAction,
    pub action_name: String,
    pub queued: bool,
    pub dispatch_required: bool,
    pub terminal: bool,
    pub retryable: bool,
    pub queue_depth_after: u8,
    pub queue_depth_after_completion: Option<u8>,
    pub message: String,
    pub lifecycle_summary: LanguageInputCallbackSessionLifecycleSummary,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanguageInputCallbackTransportEffectSummary {
    pub endpoint: LanguageHostEndpointSummary,
    pub connection_label: String,
    pub effect_label: String,
    pub action: LanguageInputCallbackTransportAction,
    pub action_name: String,
    pub dispatch_callback: bool,
    pub emit_drop: bool,
    pub emit_result: bool,
    pub remove_from_queue: bool,
    pub keep_dispatch_scheduled: bool,
    pub terminal: bool,
    pub retryable: bool,
    pub queue_depth_after_effect: u8,
    pub message: String,
    pub action_summary: LanguageInputCallbackTransportActionSummary,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LanguageInputCallbackTransportReportKind {
    Dispatch,
    Drop,
    Completion,
    Running,
    BudgetExceeded,
    Failure,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanguageInputCallbackTransportReportSummary {
    pub endpoint: LanguageHostEndpointSummary,
    pub connection_label: String,
    pub report_label: String,
    pub report_kind: LanguageInputCallbackTransportReportKind,
    pub report_name: String,
    pub action: LanguageInputCallbackTransportAction,
    pub action_name: String,
    pub dispatch_callback: bool,
    pub emit_report: bool,
    pub emit_drop: bool,
    pub emit_result: bool,
    pub remove_from_queue: bool,
    pub keep_dispatch_scheduled: bool,
    pub terminal: bool,
    pub retryable: bool,
    pub queue_depth_after_report: u8,
    pub message: String,
    pub effect_summary: LanguageInputCallbackTransportEffectSummary,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LanguageInputCallbackTransportEventKind {
    DispatchScheduled,
    CallbackDropped,
    CallbackCompleted,
    CallbackRunning,
    CallbackBudgetExceeded,
    CallbackFailed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanguageInputCallbackTransportEventSummary {
    pub endpoint: LanguageHostEndpointSummary,
    pub connection_label: String,
    pub event_label: String,
    pub event_kind: LanguageInputCallbackTransportEventKind,
    pub event_name: String,
    pub report_kind: LanguageInputCallbackTransportReportKind,
    pub report_name: String,
    pub action: LanguageInputCallbackTransportAction,
    pub action_name: String,
    pub dispatch_callback: bool,
    pub emit_report: bool,
    pub emit_drop: bool,
    pub emit_result: bool,
    pub remove_from_queue: bool,
    pub keep_dispatch_scheduled: bool,
    pub terminal: bool,
    pub retryable: bool,
    pub queue_depth_after_event: u8,
    pub message: String,
    pub report_summary: LanguageInputCallbackTransportReportSummary,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LanguageInputCallbackDiagnosticStage {
    Plan,
    Event,
    QueuePlan,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanguageInputCallbackSessionDiagnostic {
    pub endpoint: LanguageHostEndpointSummary,
    pub connection_label: String,
    pub diagnostic_stage: LanguageInputCallbackDiagnosticStage,
    pub stage_name: String,
    pub kind_name: String,
    pub diagnostic_label: String,
    pub source_diagnostic_label: String,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LanguageInputCallbackQueuePlanErrorKind {
    EmptyQueue,
    QueueDepthExceedsCapacity,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanguageInputCallbackQueuePlanError {
    pub board_id: String,
    pub pin: u8,
    pub callback_program_id: u16,
    pub queue_capacity: u8,
    pub queue_depth: u8,
    pub kind: LanguageInputCallbackQueuePlanErrorKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanguageInputCallbackQueuePlanDiagnostic {
    pub board_id: String,
    pub pin: u8,
    pub callback_program_id: u16,
    pub queue_capacity: u8,
    pub queue_depth: u8,
    pub kind: LanguageInputCallbackQueuePlanErrorKind,
    pub kind_name: String,
    pub diagnostic_label: String,
    pub message: String,
    pub error: LanguageInputCallbackQueuePlanError,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LanguageInputCallbackEventErrorKind {
    BoardMismatch,
    PinMismatch,
    EventKindMismatch,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanguageInputCallbackEventError {
    pub plan_board_id: String,
    pub event_board_id: String,
    pub plan_pin: u8,
    pub event_pin: u8,
    pub event_kind: String,
    pub kind: LanguageInputCallbackEventErrorKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanguageInputCallbackEventDiagnostic {
    pub plan_board_id: String,
    pub event_board_id: String,
    pub plan_pin: u8,
    pub event_pin: u8,
    pub event_kind: String,
    pub kind: LanguageInputCallbackEventErrorKind,
    pub kind_name: String,
    pub diagnostic_label: String,
    pub message: String,
    pub error: LanguageInputCallbackEventError,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LanguageLedMatrix {
    pub rows: u8,
    pub columns: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanguageEspUploadOptions {
    pub board_id: String,
    pub baud_rate: u32,
    pub timeout_ms: u64,
    pub reset_into_bootloader: bool,
    pub offset: u32,
    pub block_size: u32,
    pub flash_size: Option<u32>,
    pub verify_md5: bool,
    pub stay_in_bootloader: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanguagePicoUf2UploadOptions {
    pub board_id: String,
    pub command: String,
    pub volume_label: String,
    pub image_extension: String,
    pub auto_detect_mount: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanguageArduinoCliUploadOptions {
    pub board_id: String,
    pub command: String,
    pub image_format: String,
    pub transport: String,
    pub reset_method: String,
    pub platform_id: String,
    pub fqbn: String,
    pub port_hint: String,
    pub port_selection_step: String,
    pub native_usb: bool,
    pub usb_serial_bridge: bool,
    pub external_serial_adapter: bool,
    pub requires_serial_port: bool,
    pub delegate_reset_to_board_package: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanguageArduinoCliPortDiscovery {
    pub board_id: String,
    pub port_hint: String,
    pub port_selection_step: String,
    pub requires_serial_port: bool,
    pub bootloader_touch_baud: Option<u32>,
    pub expects_port_reenumeration: bool,
    pub wait_for_runtime_rediscovery: bool,
    pub serial_adapter_required: bool,
    pub reset_delegated_to_board_package: bool,
    pub notes: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanguageArduinoCliUploadInvocation {
    pub board_id: String,
    pub executable: String,
    pub subcommand: String,
    pub fqbn: String,
    pub port_hint: String,
    pub port_selection_step: String,
    pub port_flag: String,
    pub fqbn_flag: String,
    pub input_file_flag: String,
    pub input_dir_flag: String,
    pub upload_property_flag: String,
    pub verify_flag: String,
    pub port_placeholder: String,
    pub input_file_placeholder: String,
    pub args_template: Vec<String>,
    pub requires_port: bool,
    pub accepts_input_file: bool,
    pub accepts_input_dir: bool,
    pub accepts_upload_properties: bool,
    pub notes: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanguageArduinoCliUploadCommand {
    pub board_id: String,
    pub executable: String,
    pub args: Vec<String>,
    pub fqbn: String,
    pub port: String,
    pub input_file: String,
    pub upload_properties: Vec<String>,
    pub verify: bool,
    pub port_hint: String,
    pub port_selection_step: String,
    pub notes: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanguageArduinoCliUploadExecutionPlan {
    pub board_id: String,
    pub executable: String,
    pub args: Vec<String>,
    pub fqbn: String,
    pub port: String,
    pub input_file: String,
    pub upload_properties: Vec<String>,
    pub verify: bool,
    pub port_hint: String,
    pub port_selection_step: String,
    pub reset_method: String,
    pub reset_delegated_to_board_package: bool,
    pub bootloader_touch_baud: Option<u32>,
    pub expects_port_reenumeration: bool,
    pub wait_for_runtime_rediscovery: bool,
    pub serial_adapter_required: bool,
    pub steps: Vec<String>,
    pub success_exit_codes: Vec<i32>,
    pub notes: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanguageArduinoCliUploadProcess {
    pub board_id: String,
    pub executable: String,
    pub args: Vec<String>,
    pub env: Vec<(String, String)>,
    pub current_dir: Option<String>,
    pub stdin_mode: String,
    pub stdout_mode: String,
    pub stderr_mode: String,
    pub success_exit_codes: Vec<i32>,
    pub port_hint: String,
    pub port_selection_step: String,
    pub reset_method: String,
    pub reset_delegated_to_board_package: bool,
    pub expects_port_reenumeration: bool,
    pub wait_for_runtime_rediscovery: bool,
    pub serial_adapter_required: bool,
    pub notes: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanguageArduinoCliUploadResult {
    pub board_id: String,
    pub exit_code: i32,
    pub success: bool,
    pub status: String,
    pub failure_kind: Option<String>,
    pub retryable: bool,
    pub needs_port_selection: bool,
    pub needs_board_package_install: bool,
    pub needs_firmware_artifact: bool,
    pub wait_for_runtime_rediscovery: bool,
    pub port_hint: String,
    pub message: String,
    pub diagnostic: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanguageArduinoCliUploadRuntimeHandoff {
    pub board_id: String,
    pub upload_port: String,
    pub runtime_port: String,
    pub runtime_port_source: String,
    pub wait_for_runtime_rediscovery: bool,
    pub port_hint: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanguageUploadOptions {
    pub board_id: String,
    pub adapter: String,
    pub image_format: String,
    pub transport: String,
    pub reset_method: String,
    pub port_hint: Option<String>,
    pub command: String,
    pub platform_id: Option<String>,
    pub fqbn: Option<String>,
    pub notes: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanguageUploadPlan {
    pub board_id: String,
    pub adapter: String,
    pub image_format: String,
    pub transport: String,
    pub reset_method: String,
    pub port_hint: Option<String>,
    pub command: String,
    pub platform_id: Option<String>,
    pub fqbn: Option<String>,
    pub artifact_kind: String,
    pub artifact_extension: Option<String>,
    pub requires_serial_port: bool,
    pub requires_mount_path: bool,
    pub auto_detect_mount: bool,
    pub steps: Vec<String>,
    pub notes: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanguageHostDevice {
    pub id: String,
    pub port: String,
    pub transport: String,
    pub display_name: String,
    pub target: Option<LanguageTargetInfo>,
    pub target_confidence: u8,
    pub bootloader: bool,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanguageProgramBegin {
    pub program_id: u16,
    pub format: ProgramFormat,
    pub total_len: u32,
    pub program_crc32: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanguageProgramChunk {
    pub program_id: u16,
    pub offset: u32,
    pub len: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanguageProgramEnd {
    pub program_id: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanguageRunReport {
    pub program_id: u16,
    pub status: RunStatus,
    pub instructions_executed: u32,
    pub elapsed_ms: u32,
    pub stack_depth: u8,
    pub open_handles: u8,
    pub return_count: u32,
    pub returns: Vec<LanguageValue>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LanguageValue {
    Unit,
    Bool(bool),
    U8(u8),
    U16(u16),
    U32(u32),
    I16(i16),
    Handle(u16),
    Bytes(Vec<u8>),
    String(String),
}

impl LanguageValue {
    pub const fn kind(&self) -> &'static str {
        match self {
            Self::Unit => "unit",
            Self::Bool(_) => "bool",
            Self::U8(_) => "u8",
            Self::U16(_) => "u16",
            Self::U32(_) => "u32",
            Self::I16(_) => "i16",
            Self::Handle(_) => "handle",
            Self::Bytes(_) => "bytes",
            Self::String(_) => "string",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanguageBoardError {
    pub code: u16,
    pub request_id: u16,
    pub program_id: u16,
    pub bytecode_offset: u32,
    pub message: String,
}

pub const fn program_format_name(format: ProgramFormat) -> &'static str {
    match format {
        ProgramFormat::BvmModule => "bvm_module",
    }
}

pub const fn run_status_name(status: RunStatus) -> &'static str {
    match status {
        RunStatus::Halted => "halted",
        RunStatus::Running => "running",
        RunStatus::Stopped => "stopped",
        RunStatus::BudgetExceeded => "budget_exceeded",
        RunStatus::Faulted => "faulted",
    }
}

pub const fn input_callback_queue_action_name(
    action: LanguageInputCallbackQueueAction,
) -> &'static str {
    match action {
        LanguageInputCallbackQueueAction::Enqueue => "enqueue",
        LanguageInputCallbackQueueAction::DropNewest => "drop_newest",
        LanguageInputCallbackQueueAction::DropOldestThenEnqueue => "drop_oldest_then_enqueue",
    }
}

pub const fn input_callback_plan_error_kind_name(
    kind: LanguageInputCallbackPlanErrorKind,
) -> &'static str {
    match kind {
        LanguageInputCallbackPlanErrorKind::UnknownTarget => "unknown_target",
        LanguageInputCallbackPlanErrorKind::UnknownPin => "unknown_pin",
        LanguageInputCallbackPlanErrorKind::PinDoesNotSupportInput => "pin_does_not_support_input",
        LanguageInputCallbackPlanErrorKind::PinDoesNotSupportInterrupt => {
            "pin_does_not_support_interrupt"
        }
        LanguageInputCallbackPlanErrorKind::PinDoesNotSupportPull => "pin_does_not_support_pull",
        LanguageInputCallbackPlanErrorKind::EmptyQueue => "empty_queue",
        LanguageInputCallbackPlanErrorKind::EmptyCallbackBudget => "empty_callback_budget",
    }
}

pub const fn input_callback_queue_plan_error_kind_name(
    kind: LanguageInputCallbackQueuePlanErrorKind,
) -> &'static str {
    match kind {
        LanguageInputCallbackQueuePlanErrorKind::EmptyQueue => "empty_queue",
        LanguageInputCallbackQueuePlanErrorKind::QueueDepthExceedsCapacity => {
            "queue_depth_exceeds_capacity"
        }
    }
}

pub const fn input_callback_event_error_kind_name(
    kind: LanguageInputCallbackEventErrorKind,
) -> &'static str {
    match kind {
        LanguageInputCallbackEventErrorKind::BoardMismatch => "board_mismatch",
        LanguageInputCallbackEventErrorKind::PinMismatch => "pin_mismatch",
        LanguageInputCallbackEventErrorKind::EventKindMismatch => "event_kind_mismatch",
    }
}

pub const fn input_callback_diagnostic_stage_name(
    stage: LanguageInputCallbackDiagnosticStage,
) -> &'static str {
    match stage {
        LanguageInputCallbackDiagnosticStage::Plan => "plan",
        LanguageInputCallbackDiagnosticStage::Event => "event",
        LanguageInputCallbackDiagnosticStage::QueuePlan => "queue_plan",
    }
}

pub const fn input_callback_completion_action_name(
    action: LanguageInputCallbackCompletionAction,
) -> &'static str {
    match action {
        LanguageInputCallbackCompletionAction::Complete => "complete",
        LanguageInputCallbackCompletionAction::KeepRunning => "keep_running",
        LanguageInputCallbackCompletionAction::DropAfterBudgetExceeded => {
            "drop_after_budget_exceeded"
        }
        LanguageInputCallbackCompletionAction::DropAfterFailure => "drop_after_failure",
    }
}

pub const fn input_callback_transport_action_name(
    action: LanguageInputCallbackTransportAction,
) -> &'static str {
    match action {
        LanguageInputCallbackTransportAction::DropBeforeDispatch => "drop_before_dispatch",
        LanguageInputCallbackTransportAction::DispatchCallback => "dispatch_callback",
        LanguageInputCallbackTransportAction::CompleteCallback => "complete_callback",
        LanguageInputCallbackTransportAction::KeepCallbackRunning => "keep_callback_running",
        LanguageInputCallbackTransportAction::DropAfterBudgetExceeded => {
            "drop_after_budget_exceeded"
        }
        LanguageInputCallbackTransportAction::DropAfterFailure => "drop_after_failure",
    }
}

pub const fn input_callback_transport_report_kind_name(
    kind: LanguageInputCallbackTransportReportKind,
) -> &'static str {
    match kind {
        LanguageInputCallbackTransportReportKind::Dispatch => "dispatch",
        LanguageInputCallbackTransportReportKind::Drop => "drop",
        LanguageInputCallbackTransportReportKind::Completion => "completion",
        LanguageInputCallbackTransportReportKind::Running => "running",
        LanguageInputCallbackTransportReportKind::BudgetExceeded => "budget_exceeded",
        LanguageInputCallbackTransportReportKind::Failure => "failure",
    }
}

pub const fn input_callback_transport_event_kind_name(
    kind: LanguageInputCallbackTransportEventKind,
) -> &'static str {
    match kind {
        LanguageInputCallbackTransportEventKind::DispatchScheduled => "dispatch_scheduled",
        LanguageInputCallbackTransportEventKind::CallbackDropped => "callback_dropped",
        LanguageInputCallbackTransportEventKind::CallbackCompleted => "callback_completed",
        LanguageInputCallbackTransportEventKind::CallbackRunning => "callback_running",
        LanguageInputCallbackTransportEventKind::CallbackBudgetExceeded => {
            "callback_budget_exceeded"
        }
        LanguageInputCallbackTransportEventKind::CallbackFailed => "callback_failed",
    }
}

pub const fn capability_bytecode_callable(flags: u16) -> bool {
    flags & CAP_FLAG_BYTECODE_CALLABLE != 0
}

pub const fn capability_protocol_feature(flags: u16) -> bool {
    flags & CAP_FLAG_PROTOCOL_FEATURE != 0
}

pub const fn capability_board_metadata(flags: u16) -> bool {
    flags & CAP_FLAG_BOARD_METADATA != 0
}

pub const fn board_family_name(family: LanguageBoardFamily) -> &'static str {
    match family {
        LanguageBoardFamily::Arduino => "arduino",
        LanguageBoardFamily::ArduinoUnoR4 => "arduino_uno_r4",
        LanguageBoardFamily::Esp32 => "esp32",
        LanguageBoardFamily::RaspberryPiPico => "raspberry_pi_pico",
    }
}

pub const fn onboard_led_kind(led: LanguageOnboardLed) -> &'static str {
    match led {
        LanguageOnboardLed::Gpio(_) => "gpio",
        LanguageOnboardLed::WirelessChipGpio(_) => "wireless_chip_gpio",
    }
}

pub const fn wireless_transport_name(transport: LanguageWirelessTransport) -> &'static str {
    match transport {
        LanguageWirelessTransport::Wifi => "wifi",
        LanguageWirelessTransport::BluetoothLe => "bluetooth_le",
        LanguageWirelessTransport::BluetoothClassic => "bluetooth_classic",
    }
}

pub const fn network_protocol_name(protocol: LanguageNetworkProtocol) -> &'static str {
    match protocol {
        LanguageNetworkProtocol::Ipv4 => "ipv4",
        LanguageNetworkProtocol::Tcp => "tcp",
        LanguageNetworkProtocol::Udp => "udp",
        LanguageNetworkProtocol::Dns => "dns",
    }
}

pub const fn connection_transport_name(transport: LanguageConnectionTransport) -> &'static str {
    match transport {
        LanguageConnectionTransport::Serial => "serial",
        LanguageConnectionTransport::Wifi => "wifi",
        LanguageConnectionTransport::BluetoothLe => "bluetooth_le",
        LanguageConnectionTransport::BluetoothClassic => "bluetooth_classic",
    }
}

pub const fn host_endpoint_transport_name(
    transport: LanguageHostEndpointTransport,
) -> &'static str {
    match transport {
        LanguageHostEndpointTransport::SerialPort => "serial_port",
        LanguageHostEndpointTransport::TcpSocket => "tcp_socket",
        LanguageHostEndpointTransport::BluetoothLeGatt => "bluetooth_le_gatt",
        LanguageHostEndpointTransport::BluetoothClassicRfcomm => "bluetooth_classic_rfcomm",
    }
}

pub const fn host_endpoint_transport_uses_baud_rate(
    transport: LanguageHostEndpointTransport,
) -> bool {
    matches!(transport, LanguageHostEndpointTransport::SerialPort)
}

pub fn host_endpoint_connection_label(
    endpoint: &LanguageHostEndpointSummary,
    baud_rate: u32,
) -> String {
    if host_endpoint_transport_uses_baud_rate(endpoint.endpoint_transport) {
        format!("endpoint={} baud={baud_rate}", endpoint.endpoint)
    } else {
        format!("endpoint={}", endpoint.endpoint)
    }
}

pub fn host_endpoint_session_summary(
    endpoint: &str,
    baud_rate: u32,
) -> Result<LanguageHostEndpointSessionSummary, LanguageHostEndpointParseError> {
    let endpoint = parse_host_endpoint_with_error(endpoint)?;
    let connection_label = host_endpoint_connection_label(&endpoint, baud_rate);
    Ok(LanguageHostEndpointSessionSummary {
        endpoint,
        connection_label,
    })
}

pub const fn upload_adapter_name(adapter: TargetUploadAdapter) -> &'static str {
    match adapter {
        TargetUploadAdapter::ArduinoCli => "arduino_cli",
        TargetUploadAdapter::EspRomSerial => "esp_rom_serial",
        TargetUploadAdapter::PicoUf2MassStorage => "pico_uf2_mass_storage",
    }
}

pub const fn upload_image_format_name(format: TargetUploadImageFormat) -> &'static str {
    match format {
        TargetUploadImageFormat::ArduinoCliBuildOutput => "arduino_cli_build_output",
        TargetUploadImageFormat::EspFlashImage => "esp_flash_image",
        TargetUploadImageFormat::Uf2 => "uf2",
    }
}

pub const fn upload_transport_name(transport: TargetUploadTransport) -> &'static str {
    match transport {
        TargetUploadTransport::Serial => "serial",
        TargetUploadTransport::MassStorage => "mass_storage",
    }
}

pub const fn upload_port_hint_name(hint: TargetUploadPortHint) -> &'static str {
    match hint {
        TargetUploadPortHint::UsbSerialBridge => "usb_serial_bridge",
        TargetUploadPortHint::NativeUsb => "native_usb",
        TargetUploadPortHint::ExternalSerialAdapter => "external_serial_adapter",
        TargetUploadPortHint::EspRomSerial => "esp_rom_serial",
        TargetUploadPortHint::MassStorageBootloader => "mass_storage_bootloader",
    }
}

pub const fn upload_reset_method_name(method: TargetUploadResetMethod) -> &'static str {
    match method {
        TargetUploadResetMethod::ArduinoBoardPackage => "arduino_board_package",
        TargetUploadResetMethod::EspRomBootPins => "esp_rom_boot_pins",
        TargetUploadResetMethod::PicoBootsel => "pico_bootsel",
    }
}

pub fn capability_flag_names(flags: u16, out: &mut [&'static str]) -> usize {
    let mut count = 0;
    if capability_bytecode_callable(flags) {
        count = push_flag_name(out, count, "bytecode_callable");
    }
    if capability_protocol_feature(flags) {
        count = push_flag_name(out, count, "protocol_feature");
    }
    if capability_board_metadata(flags) {
        count = push_flag_name(out, count, "board_metadata");
    }
    count
}

pub fn known_targets() -> Vec<LanguageTargetInfo> {
    all_targets().iter().map(language_target_info).collect()
}

pub fn known_target(board_id: &str) -> Option<LanguageTargetInfo> {
    known_board_target(board_id).map(language_target_info)
}

pub fn detect_target(selector: &str) -> Option<LanguageTargetInfo> {
    detect_board_target(selector).map(language_target_info)
}

pub fn targets_for_upload_selector(selector: &str) -> Vec<LanguageTargetInfo> {
    let normalized = normalize_target_selector(selector);
    if normalized.is_empty() {
        return Vec::new();
    }

    all_targets()
        .iter()
        .filter(|target| upload_selector_matches(target, &normalized))
        .map(language_target_info)
        .collect()
}

fn known_board_target(board_id: &str) -> Option<&'static BoardTargetInfo> {
    all_targets()
        .iter()
        .find(|target| target.board_id == board_id)
}

fn detect_board_target(selector: &str) -> Option<&'static BoardTargetInfo> {
    let normalized = normalize_target_selector(selector);
    if normalized.is_empty() {
        return None;
    }

    for target in all_targets() {
        if normalize_target_selector(target.board_id) == normalized
            || normalize_target_selector(target.display_name) == normalized
        {
            return Some(target);
        }
    }

    if let Some(target) = unique_upload_selector_target(&normalized) {
        return Some(target);
    }

    let board_id = match normalized.as_str() {
        "uno_r4" | "uno_r4_wifi" | "arduino_uno_r4" | "arduino_uno_r4_wifi" => {
            "arduino-uno-r4-wifi"
        }
        "uno_r4_minima" | "arduino_uno_r4_minima" => "arduino-uno-r4-minima",
        "esp" | "esp32" | "esp32_devkit" | "esp32_devkit_v1" | "espressif_esp32" => {
            "esp32-devkit-v1"
        }
        "pico" | "rp2040" | "rpi_pico" | "raspberry_pico" | "raspberry_pi_pico" => {
            "raspberry-pi-pico"
        }
        "pico_w"
        | "picow"
        | "rp2040_w"
        | "rpi_pico_w"
        | "raspberry_pico_w"
        | "raspberry_pi_pico_w" => "raspberry-pi-pico-w",
        _ => return None,
    };
    known_board_target(board_id)
}

fn unique_upload_selector_target(normalized: &str) -> Option<&'static BoardTargetInfo> {
    let mut match_target = None;
    for target in all_targets() {
        if upload_selector_matches(target, normalized) {
            if match_target.is_some() {
                return None;
            }
            match_target = Some(target);
        }
    }
    match_target
}

fn upload_selector_matches(target: &BoardTargetInfo, normalized: &str) -> bool {
    let Some(upload) = target.upload else {
        return false;
    };

    upload
        .fqbn
        .is_some_and(|fqbn| normalize_target_selector(fqbn) == normalized)
        || upload
            .platform_id
            .is_some_and(|platform_id| normalize_target_selector(platform_id) == normalized)
}

pub fn esp_upload_options_for_target(selector: &str) -> Option<LanguageEspUploadOptions> {
    let target = detect_target(selector)?;
    if target.family != LanguageBoardFamily::Esp32 {
        return None;
    }

    Some(LanguageEspUploadOptions {
        board_id: target.board_id,
        baud_rate: ESP_DEFAULT_BAUD_RATE,
        timeout_ms: ESP_DEFAULT_TIMEOUT_MS,
        reset_into_bootloader: true,
        offset: LANGUAGE_ESP_DEFAULT_FLASH_OFFSET,
        block_size: ESP_DEFAULT_FLASH_BLOCK_SIZE,
        flash_size: Some(LANGUAGE_ESP_DEFAULT_FLASH_SIZE),
        verify_md5: true,
        stay_in_bootloader: false,
    })
}

pub fn pico_uf2_upload_options_for_target(selector: &str) -> Option<LanguagePicoUf2UploadOptions> {
    let target = detect_target(selector)?;
    if target.family != LanguageBoardFamily::RaspberryPiPico {
        return None;
    }

    Some(LanguagePicoUf2UploadOptions {
        board_id: target.board_id,
        command: "pico-uf2".to_owned(),
        volume_label: "RPI-RP2".to_owned(),
        image_extension: ".uf2".to_owned(),
        auto_detect_mount: true,
    })
}

pub fn arduino_cli_upload_options_for_target(
    selector: &str,
) -> Option<LanguageArduinoCliUploadOptions> {
    let target = detect_board_target(selector)?;
    let upload = target.upload?;
    if upload.adapter != TargetUploadAdapter::ArduinoCli {
        return None;
    }

    let port_hint = upload.port_hint?;
    Some(LanguageArduinoCliUploadOptions {
        board_id: target.board_id.to_owned(),
        command: upload.command.to_owned(),
        image_format: upload_image_format_name(upload.image_format).to_owned(),
        transport: upload_transport_name(upload.transport).to_owned(),
        reset_method: upload_reset_method_name(upload.reset_method).to_owned(),
        platform_id: upload.platform_id?.to_owned(),
        fqbn: upload.fqbn?.to_owned(),
        port_hint: upload_port_hint_name(port_hint).to_owned(),
        port_selection_step: arduino_cli_port_selection_step(upload.port_hint).to_owned(),
        native_usb: port_hint == TargetUploadPortHint::NativeUsb,
        usb_serial_bridge: port_hint == TargetUploadPortHint::UsbSerialBridge,
        external_serial_adapter: port_hint == TargetUploadPortHint::ExternalSerialAdapter,
        requires_serial_port: upload.transport == TargetUploadTransport::Serial,
        delegate_reset_to_board_package: upload.reset_method
            == TargetUploadResetMethod::ArduinoBoardPackage,
    })
}

pub fn arduino_cli_port_discovery_for_target(
    selector: &str,
) -> Option<LanguageArduinoCliPortDiscovery> {
    let target = detect_board_target(selector)?;
    let upload = target.upload?;
    if upload.adapter != TargetUploadAdapter::ArduinoCli {
        return None;
    }

    let port_hint = upload.port_hint?;
    Some(LanguageArduinoCliPortDiscovery {
        board_id: target.board_id.to_owned(),
        port_hint: upload_port_hint_name(port_hint).to_owned(),
        port_selection_step: arduino_cli_port_selection_step(Some(port_hint)).to_owned(),
        requires_serial_port: upload.transport == TargetUploadTransport::Serial,
        bootloader_touch_baud: arduino_cli_bootloader_touch_baud(Some(port_hint)),
        expects_port_reenumeration: arduino_cli_expects_port_reenumeration(Some(port_hint)),
        wait_for_runtime_rediscovery: arduino_cli_waits_for_runtime_rediscovery(Some(port_hint)),
        serial_adapter_required: port_hint == TargetUploadPortHint::ExternalSerialAdapter,
        reset_delegated_to_board_package: upload.reset_method
            == TargetUploadResetMethod::ArduinoBoardPackage,
        notes: arduino_cli_port_discovery_notes(Some(port_hint)).to_owned(),
    })
}

pub fn arduino_cli_upload_invocation_for_target(
    selector: &str,
) -> Option<LanguageArduinoCliUploadInvocation> {
    let target = detect_board_target(selector)?;
    let upload = target.upload?;
    if upload.adapter != TargetUploadAdapter::ArduinoCli {
        return None;
    }

    let port_hint = upload.port_hint?;
    let fqbn = upload.fqbn?;
    let (executable, subcommand) = arduino_cli_command_parts(upload.command)?;
    Some(LanguageArduinoCliUploadInvocation {
        board_id: target.board_id.to_owned(),
        executable: executable.to_owned(),
        subcommand: subcommand.to_owned(),
        fqbn: fqbn.to_owned(),
        port_hint: upload_port_hint_name(port_hint).to_owned(),
        port_selection_step: arduino_cli_port_selection_step(Some(port_hint)).to_owned(),
        port_flag: "-p".to_owned(),
        fqbn_flag: "-b".to_owned(),
        input_file_flag: "-i".to_owned(),
        input_dir_flag: "--input-dir".to_owned(),
        upload_property_flag: "--upload-property".to_owned(),
        verify_flag: "-t".to_owned(),
        port_placeholder: ARDUINO_CLI_UPLOAD_PORT_PLACEHOLDER.to_owned(),
        input_file_placeholder: ARDUINO_CLI_UPLOAD_INPUT_FILE_PLACEHOLDER.to_owned(),
        args_template: arduino_cli_upload_args_template(fqbn),
        requires_port: upload.transport == TargetUploadTransport::Serial,
        accepts_input_file: true,
        accepts_input_dir: true,
        accepts_upload_properties: true,
        notes: arduino_cli_upload_invocation_notes(port_hint).to_owned(),
    })
}

pub fn arduino_cli_upload_command_for_target(
    selector: &str,
    port: &str,
    input_file: &str,
) -> Option<LanguageArduinoCliUploadCommand> {
    arduino_cli_upload_command_with_options_for_target(selector, port, input_file, &[], false)
}

pub fn arduino_cli_upload_command_with_options_for_target(
    selector: &str,
    port: &str,
    input_file: &str,
    upload_properties: &[&str],
    verify: bool,
) -> Option<LanguageArduinoCliUploadCommand> {
    let invocation = arduino_cli_upload_invocation_for_target(selector)?;
    if port.trim().is_empty()
        || input_file.trim().is_empty()
        || upload_properties
            .iter()
            .any(|property| property.trim().is_empty())
    {
        return None;
    }

    let mut args = vec![
        invocation.subcommand.clone(),
        invocation.port_flag.clone(),
        port.to_owned(),
        invocation.fqbn_flag.clone(),
        invocation.fqbn.clone(),
        invocation.input_file_flag.clone(),
        input_file.to_owned(),
    ];
    for property in upload_properties {
        args.push(invocation.upload_property_flag.clone());
        args.push((*property).to_owned());
    }
    if verify {
        args.push(invocation.verify_flag.clone());
    }

    Some(LanguageArduinoCliUploadCommand {
        board_id: invocation.board_id,
        executable: invocation.executable,
        args,
        fqbn: invocation.fqbn,
        port: port.to_owned(),
        input_file: input_file.to_owned(),
        upload_properties: upload_properties
            .iter()
            .map(|property| (*property).to_owned())
            .collect(),
        verify,
        port_hint: invocation.port_hint,
        port_selection_step: invocation.port_selection_step,
        notes: arduino_cli_upload_command_notes().to_owned(),
    })
}

pub fn arduino_cli_upload_execution_plan_for_target(
    selector: &str,
    port: &str,
    input_file: &str,
) -> Option<LanguageArduinoCliUploadExecutionPlan> {
    arduino_cli_upload_execution_plan_with_options_for_target(
        selector,
        port,
        input_file,
        &[],
        false,
    )
}

pub fn arduino_cli_upload_execution_plan_with_options_for_target(
    selector: &str,
    port: &str,
    input_file: &str,
    upload_properties: &[&str],
    verify: bool,
) -> Option<LanguageArduinoCliUploadExecutionPlan> {
    let command = arduino_cli_upload_command_with_options_for_target(
        selector,
        port,
        input_file,
        upload_properties,
        verify,
    )?;
    let target = detect_board_target(selector)?;
    let upload = target.upload?;
    if upload.adapter != TargetUploadAdapter::ArduinoCli {
        return None;
    }
    let port_hint = upload.port_hint?;
    let options = arduino_cli_upload_options_for_target(selector)?;
    let discovery = arduino_cli_port_discovery_for_target(selector)?;

    Some(LanguageArduinoCliUploadExecutionPlan {
        board_id: command.board_id,
        executable: command.executable,
        args: command.args,
        fqbn: command.fqbn,
        port: command.port,
        input_file: command.input_file,
        upload_properties: command.upload_properties,
        verify: command.verify,
        port_hint: command.port_hint,
        port_selection_step: command.port_selection_step,
        reset_method: options.reset_method,
        reset_delegated_to_board_package: discovery.reset_delegated_to_board_package,
        bootloader_touch_baud: discovery.bootloader_touch_baud,
        expects_port_reenumeration: discovery.expects_port_reenumeration,
        wait_for_runtime_rediscovery: discovery.wait_for_runtime_rediscovery,
        serial_adapter_required: discovery.serial_adapter_required,
        steps: arduino_cli_upload_execution_steps(port_hint),
        success_exit_codes: vec![0],
        notes: arduino_cli_upload_execution_notes(port_hint).to_owned(),
    })
}

pub fn arduino_cli_upload_process_for_target(
    selector: &str,
    port: &str,
    input_file: &str,
) -> Option<LanguageArduinoCliUploadProcess> {
    arduino_cli_upload_process_with_options_for_target(selector, port, input_file, &[], false)
}

pub fn arduino_cli_upload_process_with_options_for_target(
    selector: &str,
    port: &str,
    input_file: &str,
    upload_properties: &[&str],
    verify: bool,
) -> Option<LanguageArduinoCliUploadProcess> {
    let plan = arduino_cli_upload_execution_plan_with_options_for_target(
        selector,
        port,
        input_file,
        upload_properties,
        verify,
    )?;
    Some(arduino_cli_upload_process_for_execution_plan(&plan))
}

pub fn arduino_cli_upload_process_for_execution_plan(
    plan: &LanguageArduinoCliUploadExecutionPlan,
) -> LanguageArduinoCliUploadProcess {
    LanguageArduinoCliUploadProcess {
        board_id: plan.board_id.clone(),
        executable: plan.executable.clone(),
        args: plan.args.clone(),
        env: Vec::new(),
        current_dir: None,
        stdin_mode: ARDUINO_CLI_UPLOAD_PROCESS_STDIN_MODE.to_owned(),
        stdout_mode: ARDUINO_CLI_UPLOAD_PROCESS_STDOUT_MODE.to_owned(),
        stderr_mode: ARDUINO_CLI_UPLOAD_PROCESS_STDERR_MODE.to_owned(),
        success_exit_codes: plan.success_exit_codes.clone(),
        port_hint: plan.port_hint.clone(),
        port_selection_step: plan.port_selection_step.clone(),
        reset_method: plan.reset_method.clone(),
        reset_delegated_to_board_package: plan.reset_delegated_to_board_package,
        expects_port_reenumeration: plan.expects_port_reenumeration,
        wait_for_runtime_rediscovery: plan.wait_for_runtime_rediscovery,
        serial_adapter_required: plan.serial_adapter_required,
        notes: arduino_cli_upload_process_notes(&plan.port_hint).to_owned(),
    }
}

pub fn arduino_cli_upload_result_for_target(
    selector: &str,
    exit_code: i32,
    stdout: &str,
    stderr: &str,
) -> Option<LanguageArduinoCliUploadResult> {
    let options = arduino_cli_upload_options_for_target(selector)?;
    let discovery = arduino_cli_port_discovery_for_target(selector)?;
    Some(arduino_cli_upload_result(
        options.board_id,
        discovery.port_hint,
        discovery.wait_for_runtime_rediscovery,
        exit_code,
        stdout,
        stderr,
    ))
}

pub fn arduino_cli_upload_result_for_execution_plan(
    plan: &LanguageArduinoCliUploadExecutionPlan,
    exit_code: i32,
    stdout: &str,
    stderr: &str,
) -> LanguageArduinoCliUploadResult {
    arduino_cli_upload_result_with_success_exit_codes(
        plan.board_id.clone(),
        plan.port_hint.clone(),
        plan.wait_for_runtime_rediscovery,
        &plan.success_exit_codes,
        exit_code,
        stdout,
        stderr,
    )
}

pub fn arduino_cli_upload_result_for_process_output(
    process: &LanguageArduinoCliUploadProcess,
    exit_code: i32,
    stdout: &str,
    stderr: &str,
) -> LanguageArduinoCliUploadResult {
    arduino_cli_upload_result_with_success_exit_codes(
        process.board_id.clone(),
        process.port_hint.clone(),
        process.wait_for_runtime_rediscovery,
        &process.success_exit_codes,
        exit_code,
        stdout,
        stderr,
    )
}

pub fn arduino_cli_new_upload_port(output: &str) -> Option<String> {
    output.lines().rev().find_map(|line| {
        let (_, port) = line.split_once("New upload port:")?;
        let port = port
            .trim()
            .split_once(" (")
            .map_or_else(|| port.trim(), |(port, _)| port.trim());

        (!port.is_empty()).then(|| port.to_owned())
    })
}

pub fn arduino_cli_upload_runtime_handoff_for_execution_plan(
    plan: &LanguageArduinoCliUploadExecutionPlan,
    exit_code: i32,
    stdout: &str,
    stderr: &str,
) -> Option<LanguageArduinoCliUploadRuntimeHandoff> {
    let result = arduino_cli_upload_result_for_execution_plan(plan, exit_code, stdout, stderr);
    arduino_cli_upload_runtime_handoff(
        plan.board_id.clone(),
        &plan.port,
        plan.port_hint.clone(),
        result.success,
        result.wait_for_runtime_rediscovery,
        stdout,
        stderr,
    )
}

pub fn arduino_cli_upload_runtime_handoff_for_process_output(
    process: &LanguageArduinoCliUploadProcess,
    selected_upload_port: &str,
    exit_code: i32,
    stdout: &str,
    stderr: &str,
) -> Option<LanguageArduinoCliUploadRuntimeHandoff> {
    let result = arduino_cli_upload_result_for_process_output(process, exit_code, stdout, stderr);
    arduino_cli_upload_runtime_handoff(
        process.board_id.clone(),
        selected_upload_port,
        process.port_hint.clone(),
        result.success,
        result.wait_for_runtime_rediscovery,
        stdout,
        stderr,
    )
}

pub fn upload_options_for_target(selector: &str) -> Option<LanguageUploadOptions> {
    let target = detect_board_target(selector)?;
    target
        .upload
        .map(|upload| language_upload_options(target.board_id, upload))
}

pub fn upload_plan_for_target(selector: &str) -> Option<LanguageUploadPlan> {
    let target = detect_board_target(selector)?;
    Some(language_upload_plan(target.board_id, target.upload?))
}

pub fn connection_options_for_target(selector: &str) -> Option<Vec<LanguageConnectionOption>> {
    let target = detect_target(selector)?;
    Some(target.connection_options)
}

pub fn serial_runtime_open_plan_for_target(
    selector: &str,
    runtime_port: &str,
) -> Option<LanguageSerialRuntimeOpenPlan> {
    let target = detect_target(selector)?;
    language_serial_runtime_open_plan(&target, runtime_port, "explicit_runtime_port")
}

pub fn serial_runtime_open_plan_from_upload_handoff(
    handoff: &LanguageArduinoCliUploadRuntimeHandoff,
) -> Option<LanguageSerialRuntimeOpenPlan> {
    let target = known_target(&handoff.board_id)?;
    language_serial_runtime_open_plan(&target, &handoff.runtime_port, &handoff.runtime_port_source)
}

pub const fn default_button_input_callback_options(
    callback_program_id: u16,
    callback_instruction_budget: u32,
) -> LanguageInputCallbackOptions {
    LanguageInputCallbackOptions {
        trigger: LanguageInputCallbackTrigger::FallingEdge,
        pull: LanguageInputCallbackPull::PullUp,
        debounce_ms: LANGUAGE_INPUT_CALLBACK_DEFAULT_DEBOUNCE_MS,
        queue_capacity: LANGUAGE_INPUT_CALLBACK_DEFAULT_QUEUE_CAPACITY,
        queue_policy: LanguageInputCallbackQueuePolicy::DropOldest,
        callback_program_id,
        callback_instruction_budget,
    }
}

pub fn input_callback_plan_for_target(
    selector: &str,
    pin: u8,
    callback_program_id: u16,
    callback_instruction_budget: u32,
) -> Result<LanguageInputCallbackPlan, LanguageInputCallbackPlanError> {
    input_callback_plan_with_options_for_target(
        selector,
        pin,
        default_button_input_callback_options(callback_program_id, callback_instruction_budget),
    )
}

pub fn input_callback_plan_with_options_for_target(
    selector: &str,
    pin: u8,
    options: LanguageInputCallbackOptions,
) -> Result<LanguageInputCallbackPlan, LanguageInputCallbackPlanError> {
    let target = detect_target(selector).ok_or_else(|| {
        input_callback_plan_error(
            selector,
            pin,
            LanguageInputCallbackPlanErrorKind::UnknownTarget,
        )
    })?;
    let digital_pin = target
        .digital_pins
        .iter()
        .find(|digital_pin| digital_pin.pin == pin)
        .ok_or_else(|| {
            input_callback_plan_error(
                selector,
                pin,
                LanguageInputCallbackPlanErrorKind::UnknownPin,
            )
        })?;

    if !digital_pin.supports_input {
        return Err(input_callback_plan_error(
            selector,
            pin,
            LanguageInputCallbackPlanErrorKind::PinDoesNotSupportInput,
        ));
    }

    if !digital_pin.supports_interrupt {
        return Err(input_callback_plan_error(
            selector,
            pin,
            LanguageInputCallbackPlanErrorKind::PinDoesNotSupportInterrupt,
        ));
    }

    if !input_callback_pull_supported(digital_pin, options.pull) {
        return Err(input_callback_plan_error(
            selector,
            pin,
            LanguageInputCallbackPlanErrorKind::PinDoesNotSupportPull,
        ));
    }

    if options.queue_capacity == 0 {
        return Err(input_callback_plan_error(
            selector,
            pin,
            LanguageInputCallbackPlanErrorKind::EmptyQueue,
        ));
    }

    if options.callback_instruction_budget == 0 {
        return Err(input_callback_plan_error(
            selector,
            pin,
            LanguageInputCallbackPlanErrorKind::EmptyCallbackBudget,
        ));
    }

    Ok(LanguageInputCallbackPlan {
        board_id: target.board_id,
        pin: digital_pin.pin,
        label: digital_pin.label.clone(),
        trigger: options.trigger,
        pull: options.pull,
        debounce_ms: options.debounce_ms,
        queue_capacity: options.queue_capacity,
        queue_policy: options.queue_policy,
        callback_program_id: options.callback_program_id,
        callback_instruction_budget: options.callback_instruction_budget,
        interrupt_backed: true,
        dispatch_model: LANGUAGE_INPUT_CALLBACK_DISPATCH_MODEL.to_owned(),
    })
}

pub fn input_callback_event_for_plan(
    plan: &LanguageInputCallbackPlan,
    level: LanguageInputCallbackLevel,
    sequence: u32,
    timestamp_ms: u64,
) -> LanguageInputCallbackEvent {
    LanguageInputCallbackEvent {
        board_id: plan.board_id.clone(),
        pin: plan.pin,
        label: plan.label.clone(),
        event_kind: LANGUAGE_INPUT_CALLBACK_EVENT_KIND.to_owned(),
        trigger: plan.trigger,
        level,
        sequence,
        timestamp_ms,
    }
}

pub fn input_callback_invocation_for_event(
    plan: &LanguageInputCallbackPlan,
    event: &LanguageInputCallbackEvent,
) -> Result<LanguageInputCallbackInvocation, LanguageInputCallbackEventError> {
    if event.board_id != plan.board_id {
        return Err(input_callback_event_error(
            plan,
            event,
            LanguageInputCallbackEventErrorKind::BoardMismatch,
        ));
    }

    if event.pin != plan.pin {
        return Err(input_callback_event_error(
            plan,
            event,
            LanguageInputCallbackEventErrorKind::PinMismatch,
        ));
    }

    if event.event_kind != LANGUAGE_INPUT_CALLBACK_EVENT_KIND {
        return Err(input_callback_event_error(
            plan,
            event,
            LanguageInputCallbackEventErrorKind::EventKindMismatch,
        ));
    }

    Ok(LanguageInputCallbackInvocation {
        board_id: plan.board_id.clone(),
        pin: plan.pin,
        label: plan.label.clone(),
        event_kind: event.event_kind.clone(),
        trigger: event.trigger,
        level: event.level,
        callback_program_id: plan.callback_program_id,
        callback_instruction_budget: plan.callback_instruction_budget,
        sequence: event.sequence,
        timestamp_ms: event.timestamp_ms,
        debounce_ms: plan.debounce_ms,
        queue_capacity: plan.queue_capacity,
        queue_policy: plan.queue_policy,
        interrupt_backed: plan.interrupt_backed,
        dispatch_model: plan.dispatch_model.clone(),
    })
}

pub fn input_callback_queue_plan_for_invocation(
    invocation: &LanguageInputCallbackInvocation,
    queue_depth: u8,
) -> Result<LanguageInputCallbackQueuePlan, LanguageInputCallbackQueuePlanError> {
    if invocation.queue_capacity == 0 {
        return Err(input_callback_queue_plan_error(
            invocation,
            queue_depth,
            LanguageInputCallbackQueuePlanErrorKind::EmptyQueue,
        ));
    }

    if queue_depth > invocation.queue_capacity {
        return Err(input_callback_queue_plan_error(
            invocation,
            queue_depth,
            LanguageInputCallbackQueuePlanErrorKind::QueueDepthExceedsCapacity,
        ));
    }

    let (action, queue_depth_after, queued, dropped_existing_event, dropped_incoming_event) =
        if queue_depth < invocation.queue_capacity {
            (
                LanguageInputCallbackQueueAction::Enqueue,
                queue_depth + 1,
                true,
                false,
                false,
            )
        } else {
            match invocation.queue_policy {
                LanguageInputCallbackQueuePolicy::DropNewest => (
                    LanguageInputCallbackQueueAction::DropNewest,
                    queue_depth,
                    false,
                    false,
                    true,
                ),
                LanguageInputCallbackQueuePolicy::DropOldest => (
                    LanguageInputCallbackQueueAction::DropOldestThenEnqueue,
                    queue_depth,
                    true,
                    true,
                    false,
                ),
            }
        };

    Ok(LanguageInputCallbackQueuePlan {
        board_id: invocation.board_id.clone(),
        pin: invocation.pin,
        label: invocation.label.clone(),
        event_kind: invocation.event_kind.clone(),
        callback_program_id: invocation.callback_program_id,
        callback_instruction_budget: invocation.callback_instruction_budget,
        sequence: invocation.sequence,
        timestamp_ms: invocation.timestamp_ms,
        debounce_ms: invocation.debounce_ms,
        queue_capacity: invocation.queue_capacity,
        queue_depth_before: queue_depth,
        queue_depth_after,
        queue_policy: invocation.queue_policy,
        action,
        queued,
        dropped_existing_event,
        dropped_incoming_event,
        dispatch_required: queued,
        interrupt_backed: invocation.interrupt_backed,
        dispatch_model: invocation.dispatch_model.clone(),
    })
}

pub fn input_callback_session_queue_summary(
    session: &LanguageHostEndpointSessionSummary,
    queue_plan: &LanguageInputCallbackQueuePlan,
) -> LanguageInputCallbackSessionQueueSummary {
    LanguageInputCallbackSessionQueueSummary {
        endpoint: session.endpoint.clone(),
        connection_label: session.connection_label.clone(),
        queue_label: input_callback_session_queue_label(session, queue_plan),
        action: queue_plan.action,
        queue_policy: queue_plan.queue_policy,
        queued: queue_plan.queued,
        dropped_existing_event: queue_plan.dropped_existing_event,
        dropped_incoming_event: queue_plan.dropped_incoming_event,
        dispatch_required: queue_plan.dispatch_required,
        queue_depth_before: queue_plan.queue_depth_before,
        queue_depth_after: queue_plan.queue_depth_after,
        message: input_callback_queue_message(queue_plan.action).to_owned(),
        queue_plan: queue_plan.clone(),
    }
}

pub fn input_callback_dispatch_plan_for_queue_plan(
    queue_plan: &LanguageInputCallbackQueuePlan,
) -> Option<LanguageInputCallbackDispatchPlan> {
    if !queue_plan.dispatch_required {
        return None;
    }

    Some(LanguageInputCallbackDispatchPlan {
        board_id: queue_plan.board_id.clone(),
        pin: queue_plan.pin,
        label: queue_plan.label.clone(),
        event_kind: queue_plan.event_kind.clone(),
        dispatch_reason: LANGUAGE_INPUT_CALLBACK_DISPATCH_REASON.to_owned(),
        callback_program_id: queue_plan.callback_program_id,
        callback_instruction_budget: queue_plan.callback_instruction_budget,
        sequence: queue_plan.sequence,
        timestamp_ms: queue_plan.timestamp_ms,
        queue_depth_after: queue_plan.queue_depth_after,
        queue_action: queue_plan.action,
        dropped_existing_event: queue_plan.dropped_existing_event,
        interrupt_backed: queue_plan.interrupt_backed,
        dispatch_model: queue_plan.dispatch_model.clone(),
    })
}

pub fn input_callback_session_dispatch_summary(
    session: &LanguageHostEndpointSessionSummary,
    dispatch: &LanguageInputCallbackDispatchPlan,
) -> LanguageInputCallbackSessionDispatchSummary {
    LanguageInputCallbackSessionDispatchSummary {
        endpoint: session.endpoint.clone(),
        connection_label: session.connection_label.clone(),
        dispatch_label: input_callback_session_dispatch_label(session, dispatch),
        dispatch_reason: dispatch.dispatch_reason.clone(),
        callback_program_id: dispatch.callback_program_id,
        callback_instruction_budget: dispatch.callback_instruction_budget,
        sequence: dispatch.sequence,
        queue_depth_after: dispatch.queue_depth_after,
        queue_action: dispatch.queue_action,
        dropped_existing_event: dispatch.dropped_existing_event,
        interrupt_backed: dispatch.interrupt_backed,
        dispatch_model: dispatch.dispatch_model.clone(),
        message: input_callback_dispatch_message(dispatch.dropped_existing_event).to_owned(),
        dispatch_plan: dispatch.clone(),
    }
}

pub fn input_callback_result_for_dispatch_plan(
    dispatch: &LanguageInputCallbackDispatchPlan,
    run_status: RunStatus,
    instructions_executed: u32,
    elapsed_ms: u32,
) -> LanguageInputCallbackResultSummary {
    let result_kind = input_callback_result_kind(run_status);
    let budget_exceeded = result_kind == LanguageInputCallbackResultKind::BudgetExceeded;
    let completed = result_kind == LanguageInputCallbackResultKind::Completed;
    let retryable = result_kind == LanguageInputCallbackResultKind::Incomplete;

    LanguageInputCallbackResultSummary {
        board_id: dispatch.board_id.clone(),
        pin: dispatch.pin,
        label: dispatch.label.clone(),
        event_kind: dispatch.event_kind.clone(),
        dispatch_reason: dispatch.dispatch_reason.clone(),
        callback_program_id: dispatch.callback_program_id,
        callback_instruction_budget: dispatch.callback_instruction_budget,
        sequence: dispatch.sequence,
        timestamp_ms: dispatch.timestamp_ms,
        queue_depth_after: dispatch.queue_depth_after,
        queue_action: dispatch.queue_action,
        dropped_existing_event: dispatch.dropped_existing_event,
        interrupt_backed: dispatch.interrupt_backed,
        dispatch_model: dispatch.dispatch_model.clone(),
        run_status: run_status_name(run_status).to_owned(),
        result_kind,
        instructions_executed,
        elapsed_ms,
        completed,
        budget_exceeded,
        retryable,
        message: input_callback_result_message(result_kind).to_owned(),
    }
}

pub fn input_callback_transport_result_summary(
    session: &LanguageHostEndpointSessionSummary,
    result: &LanguageInputCallbackResultSummary,
) -> LanguageInputCallbackTransportResultSummary {
    LanguageInputCallbackTransportResultSummary {
        endpoint: session.endpoint.clone(),
        connection_label: session.connection_label.clone(),
        callback_label: input_callback_transport_result_label(session, result),
        result: result.clone(),
    }
}

pub fn input_callback_completion_plan_for_result(
    result: &LanguageInputCallbackResultSummary,
) -> LanguageInputCallbackCompletionPlan {
    let action = input_callback_completion_action(result.result_kind);
    let remove_from_queue = input_callback_completion_removes_queue_item(action);
    let keep_dispatch_scheduled = action == LanguageInputCallbackCompletionAction::KeepRunning;
    let terminal = remove_from_queue;
    let queue_depth_after_completion = if remove_from_queue {
        result.queue_depth_after.saturating_sub(1)
    } else {
        result.queue_depth_after
    };

    LanguageInputCallbackCompletionPlan {
        action,
        remove_from_queue,
        keep_dispatch_scheduled,
        terminal,
        retryable: result.retryable,
        queue_depth_after_completion,
        message: input_callback_completion_message(action).to_owned(),
        result: result.clone(),
    }
}

pub fn input_callback_session_completion_summary(
    session: &LanguageHostEndpointSessionSummary,
    result: &LanguageInputCallbackResultSummary,
) -> LanguageInputCallbackSessionCompletionSummary {
    let transport = input_callback_transport_result_summary(session, result);
    let completion = input_callback_completion_plan_for_result(result);
    let completion_label = input_callback_session_completion_label(
        &transport.callback_label,
        completion.action,
        completion.queue_depth_after_completion,
    );

    LanguageInputCallbackSessionCompletionSummary {
        endpoint: transport.endpoint,
        connection_label: transport.connection_label,
        callback_label: transport.callback_label,
        completion_label,
        action: completion.action,
        remove_from_queue: completion.remove_from_queue,
        keep_dispatch_scheduled: completion.keep_dispatch_scheduled,
        terminal: completion.terminal,
        retryable: completion.retryable,
        queue_depth_after_completion: completion.queue_depth_after_completion,
        result: completion.result,
    }
}

pub fn input_callback_session_lifecycle_summary(
    session: &LanguageHostEndpointSessionSummary,
    queue_plan: &LanguageInputCallbackQueuePlan,
    run_status: Option<RunStatus>,
    instructions_executed: u32,
    elapsed_ms: u32,
) -> LanguageInputCallbackSessionLifecycleSummary {
    let queue_summary = input_callback_session_queue_summary(session, queue_plan);
    let dispatch_plan = input_callback_dispatch_plan_for_queue_plan(queue_plan);
    let dispatch_summary = dispatch_plan
        .as_ref()
        .map(|dispatch| input_callback_session_dispatch_summary(session, dispatch));
    let result = dispatch_plan.as_ref().and_then(|dispatch| {
        run_status.map(|status| {
            input_callback_result_for_dispatch_plan(
                dispatch,
                status,
                instructions_executed,
                elapsed_ms,
            )
        })
    });
    let completion_summary = result
        .as_ref()
        .map(|result| input_callback_session_completion_summary(session, result));
    let terminal = completion_summary
        .as_ref()
        .map_or(!queue_plan.dispatch_required, |summary| summary.terminal);
    let retryable = completion_summary
        .as_ref()
        .map_or(false, |summary| summary.retryable);
    let message = input_callback_session_lifecycle_message(
        queue_plan.queued,
        queue_plan.dispatch_required,
        completion_summary.as_ref().map(|summary| summary.action),
    )
    .to_owned();

    LanguageInputCallbackSessionLifecycleSummary {
        endpoint: session.endpoint.clone(),
        connection_label: session.connection_label.clone(),
        lifecycle_label: input_callback_session_lifecycle_label(session, queue_plan, terminal),
        queued: queue_plan.queued,
        dispatch_required: queue_plan.dispatch_required,
        terminal,
        retryable,
        queue_summary,
        dispatch_summary,
        result,
        completion_summary,
        message,
    }
}

pub fn input_callback_transport_action_summary(
    lifecycle: &LanguageInputCallbackSessionLifecycleSummary,
) -> LanguageInputCallbackTransportActionSummary {
    let action = input_callback_transport_action_for_lifecycle(lifecycle);
    let queue_depth_after_completion = lifecycle
        .completion_summary
        .as_ref()
        .map(|summary| summary.queue_depth_after_completion);

    LanguageInputCallbackTransportActionSummary {
        endpoint: lifecycle.endpoint.clone(),
        connection_label: lifecycle.connection_label.clone(),
        action_label: input_callback_transport_action_label(lifecycle, action),
        action,
        action_name: input_callback_transport_action_name(action).to_owned(),
        queued: lifecycle.queued,
        dispatch_required: lifecycle.dispatch_required,
        terminal: lifecycle.terminal,
        retryable: lifecycle.retryable,
        queue_depth_after: lifecycle.queue_summary.queue_depth_after,
        queue_depth_after_completion,
        message: input_callback_transport_action_message(action).to_owned(),
        lifecycle_summary: lifecycle.clone(),
    }
}

pub fn input_callback_transport_effect_summary(
    action_summary: &LanguageInputCallbackTransportActionSummary,
) -> LanguageInputCallbackTransportEffectSummary {
    let action = action_summary.action;

    LanguageInputCallbackTransportEffectSummary {
        endpoint: action_summary.endpoint.clone(),
        connection_label: action_summary.connection_label.clone(),
        effect_label: input_callback_transport_effect_label(action_summary),
        action,
        action_name: action_summary.action_name.clone(),
        dispatch_callback: input_callback_transport_effect_dispatches_callback(action),
        emit_drop: input_callback_transport_effect_emits_drop(action),
        emit_result: input_callback_transport_effect_emits_result(action),
        remove_from_queue: input_callback_transport_effect_removes_queue_item(action),
        keep_dispatch_scheduled: input_callback_transport_effect_keeps_dispatch_scheduled(action),
        terminal: action_summary.terminal,
        retryable: action_summary.retryable,
        queue_depth_after_effect: action_summary
            .queue_depth_after_completion
            .unwrap_or(action_summary.queue_depth_after),
        message: input_callback_transport_effect_message(action).to_owned(),
        action_summary: action_summary.clone(),
    }
}

pub fn input_callback_transport_report_summary(
    effect_summary: &LanguageInputCallbackTransportEffectSummary,
) -> LanguageInputCallbackTransportReportSummary {
    let report_kind = input_callback_transport_report_kind(effect_summary.action);

    LanguageInputCallbackTransportReportSummary {
        endpoint: effect_summary.endpoint.clone(),
        connection_label: effect_summary.connection_label.clone(),
        report_label: input_callback_transport_report_label(effect_summary, report_kind),
        report_kind,
        report_name: input_callback_transport_report_kind_name(report_kind).to_owned(),
        action: effect_summary.action,
        action_name: effect_summary.action_name.clone(),
        dispatch_callback: effect_summary.dispatch_callback,
        emit_report: input_callback_transport_report_emits_report(effect_summary),
        emit_drop: effect_summary.emit_drop,
        emit_result: effect_summary.emit_result,
        remove_from_queue: effect_summary.remove_from_queue,
        keep_dispatch_scheduled: effect_summary.keep_dispatch_scheduled,
        terminal: effect_summary.terminal,
        retryable: effect_summary.retryable,
        queue_depth_after_report: effect_summary.queue_depth_after_effect,
        message: input_callback_transport_report_message(report_kind).to_owned(),
        effect_summary: effect_summary.clone(),
    }
}

pub fn input_callback_transport_event_summary(
    report_summary: &LanguageInputCallbackTransportReportSummary,
) -> LanguageInputCallbackTransportEventSummary {
    let event_kind = input_callback_transport_event_kind(report_summary.report_kind);

    LanguageInputCallbackTransportEventSummary {
        endpoint: report_summary.endpoint.clone(),
        connection_label: report_summary.connection_label.clone(),
        event_label: input_callback_transport_event_label(report_summary, event_kind),
        event_kind,
        event_name: input_callback_transport_event_kind_name(event_kind).to_owned(),
        report_kind: report_summary.report_kind,
        report_name: report_summary.report_name.clone(),
        action: report_summary.action,
        action_name: report_summary.action_name.clone(),
        dispatch_callback: report_summary.dispatch_callback,
        emit_report: report_summary.emit_report,
        emit_drop: report_summary.emit_drop,
        emit_result: report_summary.emit_result,
        remove_from_queue: report_summary.remove_from_queue,
        keep_dispatch_scheduled: report_summary.keep_dispatch_scheduled,
        terminal: report_summary.terminal,
        retryable: report_summary.retryable,
        queue_depth_after_event: report_summary.queue_depth_after_report,
        message: input_callback_transport_event_message(event_kind).to_owned(),
        report_summary: report_summary.clone(),
    }
}

pub fn input_callback_plan_diagnostic(
    error: &LanguageInputCallbackPlanError,
) -> LanguageInputCallbackPlanDiagnostic {
    LanguageInputCallbackPlanDiagnostic {
        selector: error.selector.clone(),
        pin: error.pin,
        kind: error.kind,
        kind_name: input_callback_plan_error_kind_name(error.kind).to_owned(),
        diagnostic_label: input_callback_plan_diagnostic_label(error),
        message: input_callback_plan_error_message(error.kind).to_owned(),
        error: error.clone(),
    }
}

pub fn input_callback_event_diagnostic(
    error: &LanguageInputCallbackEventError,
) -> LanguageInputCallbackEventDiagnostic {
    LanguageInputCallbackEventDiagnostic {
        plan_board_id: error.plan_board_id.clone(),
        event_board_id: error.event_board_id.clone(),
        plan_pin: error.plan_pin,
        event_pin: error.event_pin,
        event_kind: error.event_kind.clone(),
        kind: error.kind,
        kind_name: input_callback_event_error_kind_name(error.kind).to_owned(),
        diagnostic_label: input_callback_event_diagnostic_label(error),
        message: input_callback_event_error_message(error.kind).to_owned(),
        error: error.clone(),
    }
}

pub fn input_callback_queue_plan_diagnostic(
    error: &LanguageInputCallbackQueuePlanError,
) -> LanguageInputCallbackQueuePlanDiagnostic {
    LanguageInputCallbackQueuePlanDiagnostic {
        board_id: error.board_id.clone(),
        pin: error.pin,
        callback_program_id: error.callback_program_id,
        queue_capacity: error.queue_capacity,
        queue_depth: error.queue_depth,
        kind: error.kind,
        kind_name: input_callback_queue_plan_error_kind_name(error.kind).to_owned(),
        diagnostic_label: input_callback_queue_plan_diagnostic_label(error),
        message: input_callback_queue_plan_error_message(error.kind).to_owned(),
        error: error.clone(),
    }
}

pub fn input_callback_session_plan_diagnostic(
    session: &LanguageHostEndpointSessionSummary,
    diagnostic: &LanguageInputCallbackPlanDiagnostic,
) -> LanguageInputCallbackSessionDiagnostic {
    input_callback_session_diagnostic(
        session,
        LanguageInputCallbackDiagnosticStage::Plan,
        &diagnostic.kind_name,
        &diagnostic.diagnostic_label,
        &diagnostic.message,
    )
}

pub fn input_callback_session_event_diagnostic(
    session: &LanguageHostEndpointSessionSummary,
    diagnostic: &LanguageInputCallbackEventDiagnostic,
) -> LanguageInputCallbackSessionDiagnostic {
    input_callback_session_diagnostic(
        session,
        LanguageInputCallbackDiagnosticStage::Event,
        &diagnostic.kind_name,
        &diagnostic.diagnostic_label,
        &diagnostic.message,
    )
}

pub fn input_callback_session_queue_plan_diagnostic(
    session: &LanguageHostEndpointSessionSummary,
    diagnostic: &LanguageInputCallbackQueuePlanDiagnostic,
) -> LanguageInputCallbackSessionDiagnostic {
    input_callback_session_diagnostic(
        session,
        LanguageInputCallbackDiagnosticStage::QueuePlan,
        &diagnostic.kind_name,
        &diagnostic.diagnostic_label,
        &diagnostic.message,
    )
}

pub fn parse_serial_endpoint(endpoint: &str) -> Option<LanguageSerialEndpoint> {
    let (scheme, port) = endpoint.split_once("://")?;
    if scheme != "serial" {
        return None;
    }
    let port = port.trim();
    if port.is_empty() {
        return None;
    }
    Some(LanguageSerialEndpoint {
        endpoint: endpoint.to_owned(),
        transport: LanguageConnectionTransport::Serial,
        endpoint_transport: LanguageHostEndpointTransport::SerialPort,
        endpoint_scheme: scheme.to_owned(),
        port: port.to_owned(),
    })
}

pub fn parse_tcp_endpoint(endpoint: &str) -> Option<LanguageTcpEndpoint> {
    let (scheme, authority) = match endpoint.split_once("://") {
        Some(("tcp", authority)) => ("tcp", authority),
        Some(_) => return None,
        None => ("tcp", endpoint),
    };
    let authority = authority.trim();
    if authority.is_empty() {
        return None;
    }
    Some(LanguageTcpEndpoint {
        endpoint: endpoint.to_owned(),
        transport: LanguageConnectionTransport::Wifi,
        endpoint_transport: LanguageHostEndpointTransport::TcpSocket,
        endpoint_scheme: scheme.to_owned(),
        authority: authority.to_owned(),
    })
}

pub fn parse_bluetooth_endpoint(endpoint: &str) -> Option<LanguageBluetoothEndpoint> {
    language_bluetooth_endpoint(parse_board_vm_bluetooth_endpoint(endpoint).ok()?)
}

pub fn parse_host_endpoint(endpoint: &str) -> Option<LanguageHostEndpointSummary> {
    if let Some(endpoint) = parse_serial_endpoint(endpoint) {
        return Some(LanguageHostEndpointSummary {
            endpoint: endpoint.endpoint,
            transport: endpoint.transport,
            endpoint_transport: endpoint.endpoint_transport,
            endpoint_scheme: endpoint.endpoint_scheme,
        });
    }
    if let Some(endpoint) = parse_tcp_endpoint(endpoint) {
        return Some(LanguageHostEndpointSummary {
            endpoint: endpoint.endpoint,
            transport: endpoint.transport,
            endpoint_transport: endpoint.endpoint_transport,
            endpoint_scheme: endpoint.endpoint_scheme,
        });
    }
    let endpoint = parse_bluetooth_endpoint(endpoint)?;
    Some(LanguageHostEndpointSummary {
        endpoint: endpoint.endpoint,
        transport: endpoint.transport,
        endpoint_transport: endpoint.endpoint_transport,
        endpoint_scheme: endpoint.endpoint_scheme,
    })
}

pub fn parse_host_endpoint_with_error(
    endpoint: &str,
) -> Result<LanguageHostEndpointSummary, LanguageHostEndpointParseError> {
    parse_host_endpoint(endpoint).ok_or_else(|| host_endpoint_parse_error(endpoint))
}

pub fn host_endpoint_parse_error(endpoint: &str) -> LanguageHostEndpointParseError {
    let scheme = endpoint.split_once("://").map(|(scheme, _)| scheme);
    let kind = match scheme {
        Some("serial") => LanguageHostEndpointParseErrorKind::InvalidSerialEndpoint,
        None | Some("tcp") => LanguageHostEndpointParseErrorKind::InvalidTcpEndpoint,
        Some("ble") | Some("btspp") | Some("rfcomm") => {
            LanguageHostEndpointParseErrorKind::InvalidBluetoothEndpoint
        }
        Some(_) => LanguageHostEndpointParseErrorKind::UnsupportedScheme,
    };
    LanguageHostEndpointParseError {
        endpoint: endpoint.to_owned(),
        kind,
        scheme: scheme.map(str::to_owned),
    }
}

pub fn bluetooth_endpoint_candidates_from_devices(
    devices: &[LanguageBluetoothDiscoveredDevice],
) -> Vec<LanguageBluetoothEndpointCandidate> {
    let devices: Vec<_> = devices
        .iter()
        .map(|device| BluetoothDiscoveredDevice {
            id: device.id.clone(),
            name: device.name.clone(),
            address: device.address.clone(),
            paired: device.paired,
            service_uuids: device.service_uuids.clone(),
            characteristic_uuids: device.characteristic_uuids.clone(),
            board_vm_rfcomm_channels: device.board_vm_rfcomm_channels.clone(),
        })
        .collect();

    board_vm_bluetooth_endpoint_candidates(&devices)
        .into_iter()
        .filter_map(language_bluetooth_endpoint_candidate)
        .collect()
}

pub fn discover_bluetooth_devices() -> Vec<LanguageBluetoothDiscoveredDevice> {
    discover_board_vm_bluetooth_devices()
        .unwrap_or_default()
        .into_iter()
        .map(language_bluetooth_discovered_device)
        .collect()
}

pub fn discover_bluetooth_endpoint_candidates() -> Vec<LanguageBluetoothEndpointCandidate> {
    let devices = discover_bluetooth_devices();
    bluetooth_endpoint_candidates_from_devices(&devices)
}

pub fn bluetooth_backend_open_plan(endpoint: &str) -> Option<LanguageBluetoothBackendOpenPlan> {
    let endpoint = parse_board_vm_bluetooth_endpoint(endpoint).ok()?;
    match endpoint {
        BluetoothEndpoint::BleGatt(endpoint) => Some(language_bluetooth_ble_open_plan(endpoint)),
        BluetoothEndpoint::Rfcomm(endpoint) => {
            #[cfg(target_os = "macos")]
            {
                let mut resolver = MacosDevRfcommDeviceResolver;
                match resolver.rfcomm_device_paths() {
                    Ok(paths) => Some(language_bluetooth_rfcomm_open_plan_from_paths(
                        endpoint, paths,
                    )),
                    Err(error) => {
                        let endpoint =
                            language_bluetooth_endpoint(BluetoothEndpoint::Rfcomm(endpoint))?;
                        Some(LanguageBluetoothBackendOpenPlan {
                            endpoint,
                            backend: "macos_rfcomm".to_owned(),
                            status: "unavailable".to_owned(),
                            stream_path: None,
                            native_transport: false,
                            message: Some(format!(
                                "macOS RFCOMM device discovery failed: {error:?}"
                            )),
                        })
                    }
                }
            }
            #[cfg(not(target_os = "macos"))]
            {
                let endpoint = language_bluetooth_endpoint(BluetoothEndpoint::Rfcomm(endpoint))?;
                Some(unsupported_bluetooth_backend_open_plan(endpoint))
            }
        }
    }
}

pub fn bluetooth_backend_open_plan_from_rfcomm_paths<I, S>(
    endpoint: &str,
    paths: I,
) -> Option<LanguageBluetoothBackendOpenPlan>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let endpoint = parse_board_vm_bluetooth_endpoint(endpoint).ok()?;
    match endpoint {
        BluetoothEndpoint::BleGatt(endpoint) => Some(language_bluetooth_ble_open_plan(endpoint)),
        BluetoothEndpoint::Rfcomm(endpoint) => Some(
            language_bluetooth_rfcomm_open_plan_from_paths(endpoint, paths),
        ),
    }
}

pub fn bluetooth_transact_wire_frame(
    endpoint: &str,
    wire_frame: &[u8],
    response_out: &mut [u8],
) -> Result<usize, LanguageCoreError> {
    let endpoint = parse_board_vm_bluetooth_endpoint(endpoint)
        .map_err(|_| LanguageCoreError::BluetoothEndpoint)?;
    #[cfg(target_os = "macos")]
    {
        let mut backend = MacosBluetoothBackend::with_resolver_and_ble_connector(
            MacosDevRfcommDeviceResolver,
            MacosCoreBluetoothRuntimeBleConnector::new(),
        );
        bluetooth_transact_wire_frame_with_backend::<_, 4096>(
            &mut backend,
            endpoint,
            wire_frame,
            response_out,
        )
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (endpoint, wire_frame, response_out);
        Err(LanguageCoreError::BluetoothOpen)
    }
}

pub fn bluetooth_transact_wire_frame_with_backend<B, const WIRE_BYTES: usize>(
    backend: &mut B,
    endpoint: BluetoothEndpoint,
    wire_frame: &[u8],
    response_out: &mut [u8],
) -> Result<usize, LanguageCoreError>
where
    B: BluetoothBackend,
{
    let mut transport = open_bluetooth_endpoint::<_, WIRE_BYTES>(backend, endpoint)?;
    let mut raw_request = vec![0u8; wire_frame.len().max(64)];
    let raw_request_len = decode_wire_frame(wire_frame, &mut raw_request)?;
    let mut raw_response = vec![0u8; response_out.len().max(64)];
    let raw_response_len =
        transport.exchange_raw_frame(&raw_request[..raw_request_len], &mut raw_response)?;
    Ok(encode_wire_frame(
        &raw_response[..raw_response_len],
        response_out,
    )?)
}

fn language_bluetooth_rfcomm_open_plan_from_paths<I, S>(
    endpoint: board_vm_bluetooth::RfcommEndpoint,
    paths: I,
) -> LanguageBluetoothBackendOpenPlan
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let stream_path = macos_rfcomm_device_path(&endpoint, paths);
    let device = endpoint.device.clone();
    let channel = endpoint.channel;
    let endpoint = language_bluetooth_endpoint(BluetoothEndpoint::Rfcomm(endpoint))
        .expect("parsed RFCOMM endpoint should map to language metadata");
    match stream_path {
        Some(stream_path) => LanguageBluetoothBackendOpenPlan {
            endpoint,
            backend: "macos_rfcomm".to_owned(),
            status: "ready".to_owned(),
            stream_path: Some(stream_path),
            native_transport: false,
            message: None,
        },
        None => LanguageBluetoothBackendOpenPlan {
            endpoint,
            backend: "macos_rfcomm".to_owned(),
            status: "not_found".to_owned(),
            stream_path: None,
            native_transport: false,
            message: Some(format!(
                "no macOS RFCOMM serial device found for {device} channel {channel}"
            )),
        },
    }
}

fn language_bluetooth_ble_open_plan(
    endpoint: board_vm_bluetooth::BleGattEndpoint,
) -> LanguageBluetoothBackendOpenPlan {
    let endpoint = language_bluetooth_endpoint(BluetoothEndpoint::BleGatt(endpoint))
        .expect("parsed BLE endpoint should map to language metadata");
    #[cfg(target_os = "macos")]
    {
        LanguageBluetoothBackendOpenPlan {
            endpoint,
            backend: "macos_core_bluetooth".to_owned(),
            status: "ready".to_owned(),
            stream_path: None,
            native_transport: true,
            message: None,
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        LanguageBluetoothBackendOpenPlan {
            endpoint,
            backend: "unsupported".to_owned(),
            status: "unsupported".to_owned(),
            stream_path: None,
            native_transport: false,
            message: Some(format!(
                "Board VM Bluetooth BLE GATT opening is unsupported on {}",
                std::env::consts::OS
            )),
        }
    }
}

#[cfg(not(target_os = "macos"))]
fn unsupported_bluetooth_backend_open_plan(
    endpoint: LanguageBluetoothEndpoint,
) -> LanguageBluetoothBackendOpenPlan {
    LanguageBluetoothBackendOpenPlan {
        endpoint,
        backend: "unsupported".to_owned(),
        status: "unsupported".to_owned(),
        stream_path: None,
        native_transport: false,
        message: Some(format!(
            "Board VM Bluetooth backend opening is unsupported on {}",
            std::env::consts::OS
        )),
    }
}

fn language_bluetooth_discovered_device(
    device: BluetoothDiscoveredDevice,
) -> LanguageBluetoothDiscoveredDevice {
    LanguageBluetoothDiscoveredDevice {
        id: device.id,
        name: device.name,
        address: device.address,
        paired: device.paired,
        service_uuids: device.service_uuids,
        characteristic_uuids: device.characteristic_uuids,
        board_vm_rfcomm_channels: device.board_vm_rfcomm_channels,
    }
}

fn language_bluetooth_endpoint(endpoint: BluetoothEndpoint) -> Option<LanguageBluetoothEndpoint> {
    match endpoint {
        BluetoothEndpoint::BleGatt(endpoint) => Some(LanguageBluetoothEndpoint {
            endpoint: endpoint.endpoint,
            transport: LanguageConnectionTransport::BluetoothLe,
            endpoint_transport: LanguageHostEndpointTransport::BluetoothLeGatt,
            endpoint_scheme: "ble".to_owned(),
            device: endpoint.device,
            service_uuid: Some(endpoint.service_uuid),
            write_characteristic_uuid: Some(endpoint.write_characteristic_uuid),
            notify_characteristic_uuid: Some(endpoint.notify_characteristic_uuid),
            channel: None,
        }),
        BluetoothEndpoint::Rfcomm(endpoint) => Some(LanguageBluetoothEndpoint {
            endpoint: endpoint.endpoint.clone(),
            transport: LanguageConnectionTransport::BluetoothClassic,
            endpoint_transport: LanguageHostEndpointTransport::BluetoothClassicRfcomm,
            endpoint_scheme: bluetooth_endpoint_scheme(&endpoint.endpoint).to_owned(),
            device: endpoint.device,
            service_uuid: None,
            write_characteristic_uuid: None,
            notify_characteristic_uuid: None,
            channel: Some(endpoint.channel),
        }),
    }
}

fn language_bluetooth_endpoint_candidate(
    candidate: BluetoothEndpointCandidate,
) -> Option<LanguageBluetoothEndpointCandidate> {
    Some(LanguageBluetoothEndpointCandidate {
        endpoint: language_bluetooth_endpoint(candidate.endpoint)?,
        device: candidate.device,
        display_name: candidate.display_name,
        paired: candidate.paired,
        requires_pairing: candidate.requires_pairing,
    })
}

pub fn discover_devices() -> Vec<LanguageHostDevice> {
    let mut paths = env_device_paths();

    #[cfg(unix)]
    paths.extend(unix_device_paths());

    #[cfg(windows)]
    paths.extend(windows_device_paths());

    discover_devices_from_paths(paths)
}

pub fn discover_devices_from_paths<I, S>(paths: I) -> Vec<LanguageHostDevice>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut devices = BTreeMap::new();
    for path in paths {
        let path = path.as_ref().trim();
        if path.is_empty() || !is_serial_candidate(path) {
            continue;
        }
        let device = classify_host_device(path);
        devices.entry(device_dedupe_key(path)).or_insert(device);
    }
    devices.into_values().collect()
}

pub fn discover_pico_bootsel_mounts() -> Vec<String> {
    discover_pico_bootsel_mounts_in_roots(default_pico_mount_roots())
}

pub fn discover_pico_bootsel_mounts_in_roots<I, P>(roots: I) -> Vec<String>
where
    I: IntoIterator<Item = P>,
    P: AsRef<Path>,
{
    let mut mounts = Vec::new();
    for root in roots {
        let Ok(entries) = fs::read_dir(root.as_ref()) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if is_pico_bootsel_mount(&path) {
                mounts.push(path.to_string_lossy().into_owned());
            }
        }
    }
    mounts.sort();
    mounts.dedup();
    mounts
}

pub fn normalize_target_selector(selector: &str) -> String {
    let mut normalized = String::new();
    let mut last_was_separator = false;
    for character in selector.trim().chars() {
        if character.is_ascii_alphanumeric() {
            normalized.push(character.to_ascii_lowercase());
            last_was_separator = false;
        } else if character == '-' || character == '_' || character.is_ascii_whitespace() {
            if !normalized.is_empty() && !last_was_separator {
                normalized.push('_');
                last_was_separator = true;
            }
        }
    }
    if normalized.ends_with('_') {
        normalized.pop();
    }
    normalized
}

fn language_target_info(target: &BoardTargetInfo) -> LanguageTargetInfo {
    LanguageTargetInfo {
        board_id: target.board_id.to_owned(),
        display_name: target.display_name.to_owned(),
        family: language_family(target.family),
        runtime_id: target.runtime_id.to_owned(),
        mcu: target.mcu.to_owned(),
        core: target.core.to_owned(),
        rust_target: target.rust_target.to_owned(),
        clock_hz: target.clock_hz,
        operating_voltage_mv: target.operating_voltage_mv,
        onboard_led: target.onboard_led.map(language_onboard_led),
        led_matrix: target.led_matrix.map(|matrix| LanguageLedMatrix {
            rows: matrix.rows,
            columns: matrix.columns,
        }),
        digital_pin_count: target.digital_pin_count,
        digital_pins: target
            .digital_pins
            .iter()
            .map(language_digital_pin)
            .collect(),
        i2c_buses: target.i2c_buses.iter().map(language_i2c_bus).collect(),
        i2c_connectors: target
            .i2c_connectors
            .iter()
            .map(language_i2c_connector)
            .collect(),
        spi_buses: target.spi_buses.iter().map(language_spi_bus).collect(),
        uart_buses: target.uart_buses.iter().map(language_uart_bus).collect(),
        usb_interfaces: target
            .usb_interfaces
            .iter()
            .map(language_usb_interface)
            .collect(),
        can_buses: target.can_buses.iter().map(language_can_bus).collect(),
        rtc: target.rtc.map(language_rtc),
        wireless: target
            .wireless
            .iter()
            .map(language_wireless_interface)
            .collect(),
        network_interfaces: target
            .network_interfaces
            .iter()
            .map(language_network_interface)
            .collect(),
        connection_options: language_connection_options(target),
        upload: target
            .upload
            .map(|upload| language_upload_options(target.board_id, upload)),
        capabilities: target
            .capabilities
            .iter()
            .map(|capability| (*capability).to_owned())
            .collect(),
    }
}

fn language_digital_pin(pin: &TargetDigitalPin) -> LanguageDigitalPin {
    LanguageDigitalPin {
        pin: pin.pin,
        label: pin.label.to_owned(),
        supports_input: pin.supports_input,
        supports_output: pin.supports_output,
        supports_pullup: pin.supports_pullup,
        supports_pulldown: pin.supports_pulldown,
        supports_adc: pin.supports_adc,
        supports_pwm: pin.supports_pwm,
        supports_dac: pin.supports_dac,
        supports_touch: pin.supports_touch,
        supports_interrupt: pin.supports_interrupt,
        boot_strap: pin.boot_strap,
        notes: pin.notes.to_owned(),
    }
}

fn input_callback_pull_supported(
    pin: &LanguageDigitalPin,
    pull: LanguageInputCallbackPull,
) -> bool {
    match pull {
        LanguageInputCallbackPull::Floating => true,
        LanguageInputCallbackPull::PullUp => pin.supports_pullup,
        LanguageInputCallbackPull::PullDown => pin.supports_pulldown,
    }
}

fn input_callback_plan_error(
    selector: &str,
    pin: u8,
    kind: LanguageInputCallbackPlanErrorKind,
) -> LanguageInputCallbackPlanError {
    LanguageInputCallbackPlanError {
        selector: selector.to_owned(),
        pin,
        kind,
    }
}

fn input_callback_event_error(
    plan: &LanguageInputCallbackPlan,
    event: &LanguageInputCallbackEvent,
    kind: LanguageInputCallbackEventErrorKind,
) -> LanguageInputCallbackEventError {
    LanguageInputCallbackEventError {
        plan_board_id: plan.board_id.clone(),
        event_board_id: event.board_id.clone(),
        plan_pin: plan.pin,
        event_pin: event.pin,
        event_kind: event.event_kind.clone(),
        kind,
    }
}

fn input_callback_queue_plan_error(
    invocation: &LanguageInputCallbackInvocation,
    queue_depth: u8,
    kind: LanguageInputCallbackQueuePlanErrorKind,
) -> LanguageInputCallbackQueuePlanError {
    LanguageInputCallbackQueuePlanError {
        board_id: invocation.board_id.clone(),
        pin: invocation.pin,
        callback_program_id: invocation.callback_program_id,
        queue_capacity: invocation.queue_capacity,
        queue_depth,
        kind,
    }
}

fn input_callback_plan_error_message(kind: LanguageInputCallbackPlanErrorKind) -> &'static str {
    match kind {
        LanguageInputCallbackPlanErrorKind::UnknownTarget => "Input callback target is not known.",
        LanguageInputCallbackPlanErrorKind::UnknownPin => {
            "Input callback pin is not available on the selected target."
        }
        LanguageInputCallbackPlanErrorKind::PinDoesNotSupportInput => {
            "Input callback pin does not support digital input."
        }
        LanguageInputCallbackPlanErrorKind::PinDoesNotSupportInterrupt => {
            "Input callback pin does not support interrupt-backed callbacks."
        }
        LanguageInputCallbackPlanErrorKind::PinDoesNotSupportPull => {
            "Input callback pull mode is not supported by the pin."
        }
        LanguageInputCallbackPlanErrorKind::EmptyQueue => {
            "Input callback queue capacity must be greater than zero."
        }
        LanguageInputCallbackPlanErrorKind::EmptyCallbackBudget => {
            "Input callback instruction budget must be greater than zero."
        }
    }
}

fn input_callback_plan_diagnostic_label(error: &LanguageInputCallbackPlanError) -> String {
    format!(
        "input_callback_plan selector={} pin={} error={}",
        error.selector,
        error.pin,
        input_callback_plan_error_kind_name(error.kind)
    )
}

fn input_callback_event_error_message(kind: LanguageInputCallbackEventErrorKind) -> &'static str {
    match kind {
        LanguageInputCallbackEventErrorKind::BoardMismatch => {
            "Input callback event board does not match the planned callback board."
        }
        LanguageInputCallbackEventErrorKind::PinMismatch => {
            "Input callback event pin does not match the planned callback pin."
        }
        LanguageInputCallbackEventErrorKind::EventKindMismatch => {
            "Input callback event kind is not supported by this callback planner."
        }
    }
}

fn input_callback_event_diagnostic_label(error: &LanguageInputCallbackEventError) -> String {
    format!(
        "input_callback_event plan_board={} event_board={} plan_pin={} event_pin={} event_kind={} error={}",
        error.plan_board_id,
        error.event_board_id,
        error.plan_pin,
        error.event_pin,
        error.event_kind,
        input_callback_event_error_kind_name(error.kind)
    )
}

fn input_callback_queue_plan_error_message(
    kind: LanguageInputCallbackQueuePlanErrorKind,
) -> &'static str {
    match kind {
        LanguageInputCallbackQueuePlanErrorKind::EmptyQueue => {
            "Input callback queue capacity must be greater than zero."
        }
        LanguageInputCallbackQueuePlanErrorKind::QueueDepthExceedsCapacity => {
            "Input callback queue depth exceeds the configured queue capacity."
        }
    }
}

fn input_callback_queue_plan_diagnostic_label(
    error: &LanguageInputCallbackQueuePlanError,
) -> String {
    format!(
        "input_callback_queue board={} pin={} program={} queue_depth={} queue_capacity={} error={}",
        error.board_id,
        error.pin,
        error.callback_program_id,
        error.queue_depth,
        error.queue_capacity,
        input_callback_queue_plan_error_kind_name(error.kind)
    )
}

fn input_callback_session_diagnostic(
    session: &LanguageHostEndpointSessionSummary,
    stage: LanguageInputCallbackDiagnosticStage,
    kind_name: &str,
    source_diagnostic_label: &str,
    message: &str,
) -> LanguageInputCallbackSessionDiagnostic {
    let stage_name = input_callback_diagnostic_stage_name(stage);
    LanguageInputCallbackSessionDiagnostic {
        endpoint: session.endpoint.clone(),
        connection_label: session.connection_label.clone(),
        diagnostic_stage: stage,
        stage_name: stage_name.to_owned(),
        kind_name: kind_name.to_owned(),
        diagnostic_label: input_callback_session_diagnostic_label(
            session,
            stage_name,
            source_diagnostic_label,
        ),
        source_diagnostic_label: source_diagnostic_label.to_owned(),
        message: message.to_owned(),
    }
}

fn input_callback_session_diagnostic_label(
    session: &LanguageHostEndpointSessionSummary,
    stage_name: &str,
    source_diagnostic_label: &str,
) -> String {
    format!(
        "{} callback_diagnostic_stage={} {}",
        session.connection_label, stage_name, source_diagnostic_label
    )
}

fn input_callback_queue_message(action: LanguageInputCallbackQueueAction) -> &'static str {
    match action {
        LanguageInputCallbackQueueAction::Enqueue => {
            "Input callback enqueued; wake cooperative dispatch."
        }
        LanguageInputCallbackQueueAction::DropNewest => {
            "Input callback dropped because the queue is full and drop-newest policy is active."
        }
        LanguageInputCallbackQueueAction::DropOldestThenEnqueue => {
            "Input callback enqueued after dropping the oldest queued callback."
        }
    }
}

fn input_callback_session_queue_label(
    session: &LanguageHostEndpointSessionSummary,
    queue_plan: &LanguageInputCallbackQueuePlan,
) -> String {
    format!(
        "{} callback={}:{} sequence={} queue_action={} queue_depth_before={} queue_depth_after={}",
        session.connection_label,
        queue_plan.board_id,
        queue_plan.label,
        queue_plan.sequence,
        input_callback_queue_action_name(queue_plan.action),
        queue_plan.queue_depth_before,
        queue_plan.queue_depth_after
    )
}

fn input_callback_dispatch_message(dropped_existing_event: bool) -> &'static str {
    if dropped_existing_event {
        "Input callback dispatch replaces the oldest queued callback."
    } else {
        "Input callback dispatch is ready for the cooperative runner."
    }
}

fn input_callback_session_dispatch_label(
    session: &LanguageHostEndpointSessionSummary,
    dispatch: &LanguageInputCallbackDispatchPlan,
) -> String {
    format!(
        "{} callback={}:{} sequence={} dispatch_reason={} queue_action={} queue_depth_after={} instruction_budget={}",
        session.connection_label,
        dispatch.board_id,
        dispatch.label,
        dispatch.sequence,
        dispatch.dispatch_reason,
        input_callback_queue_action_name(dispatch.queue_action),
        dispatch.queue_depth_after,
        dispatch.callback_instruction_budget
    )
}

fn input_callback_result_kind(run_status: RunStatus) -> LanguageInputCallbackResultKind {
    match run_status {
        RunStatus::Halted => LanguageInputCallbackResultKind::Completed,
        RunStatus::BudgetExceeded => LanguageInputCallbackResultKind::BudgetExceeded,
        RunStatus::Running => LanguageInputCallbackResultKind::Incomplete,
        RunStatus::Stopped | RunStatus::Faulted => LanguageInputCallbackResultKind::Failed,
    }
}

fn input_callback_result_message(kind: LanguageInputCallbackResultKind) -> &'static str {
    match kind {
        LanguageInputCallbackResultKind::Completed => "Input callback completed.",
        LanguageInputCallbackResultKind::BudgetExceeded => {
            "Input callback exhausted its instruction budget."
        }
        LanguageInputCallbackResultKind::Incomplete => {
            "Input callback is still running; keep cooperative dispatch scheduled."
        }
        LanguageInputCallbackResultKind::Failed => "Input callback stopped before completion.",
    }
}

fn input_callback_transport_result_label(
    session: &LanguageHostEndpointSessionSummary,
    result: &LanguageInputCallbackResultSummary,
) -> String {
    format!(
        "{} callback={}:{} sequence={} status={}",
        session.connection_label, result.board_id, result.label, result.sequence, result.run_status
    )
}

fn input_callback_completion_action(
    kind: LanguageInputCallbackResultKind,
) -> LanguageInputCallbackCompletionAction {
    match kind {
        LanguageInputCallbackResultKind::Completed => {
            LanguageInputCallbackCompletionAction::Complete
        }
        LanguageInputCallbackResultKind::BudgetExceeded => {
            LanguageInputCallbackCompletionAction::DropAfterBudgetExceeded
        }
        LanguageInputCallbackResultKind::Incomplete => {
            LanguageInputCallbackCompletionAction::KeepRunning
        }
        LanguageInputCallbackResultKind::Failed => {
            LanguageInputCallbackCompletionAction::DropAfterFailure
        }
    }
}

fn input_callback_completion_removes_queue_item(
    action: LanguageInputCallbackCompletionAction,
) -> bool {
    !matches!(action, LanguageInputCallbackCompletionAction::KeepRunning)
}

fn input_callback_completion_message(
    action: LanguageInputCallbackCompletionAction,
) -> &'static str {
    match action {
        LanguageInputCallbackCompletionAction::Complete => {
            "Input callback completed; remove it from the cooperative queue."
        }
        LanguageInputCallbackCompletionAction::KeepRunning => {
            "Input callback is still running; keep cooperative dispatch scheduled."
        }
        LanguageInputCallbackCompletionAction::DropAfterBudgetExceeded => {
            "Input callback exhausted its budget; drop it from the cooperative queue after reporting."
        }
        LanguageInputCallbackCompletionAction::DropAfterFailure => {
            "Input callback stopped before completion; remove it from the cooperative queue."
        }
    }
}

fn input_callback_session_completion_label(
    callback_label: &str,
    action: LanguageInputCallbackCompletionAction,
    queue_depth_after_completion: u8,
) -> String {
    format!(
        "{callback_label} action={} queue_depth_after_completion={queue_depth_after_completion}",
        input_callback_completion_action_name(action)
    )
}

fn input_callback_session_lifecycle_label(
    session: &LanguageHostEndpointSessionSummary,
    queue_plan: &LanguageInputCallbackQueuePlan,
    terminal: bool,
) -> String {
    format!(
        "{} callback={}:{} sequence={} queued={} dispatch_required={} terminal={}",
        session.connection_label,
        queue_plan.board_id,
        queue_plan.label,
        queue_plan.sequence,
        queue_plan.queued,
        queue_plan.dispatch_required,
        terminal
    )
}

fn input_callback_session_lifecycle_message(
    queued: bool,
    dispatch_required: bool,
    completion_action: Option<LanguageInputCallbackCompletionAction>,
) -> &'static str {
    if let Some(action) = completion_action {
        return input_callback_completion_message(action);
    }
    if queued && dispatch_required {
        "Input callback is queued for cooperative dispatch."
    } else {
        "Input callback was not queued; lifecycle ended before dispatch."
    }
}

fn input_callback_transport_action_for_lifecycle(
    lifecycle: &LanguageInputCallbackSessionLifecycleSummary,
) -> LanguageInputCallbackTransportAction {
    if !lifecycle.queued && !lifecycle.dispatch_required {
        return LanguageInputCallbackTransportAction::DropBeforeDispatch;
    }
    match lifecycle
        .completion_summary
        .as_ref()
        .map(|summary| summary.action)
    {
        Some(LanguageInputCallbackCompletionAction::Complete) => {
            LanguageInputCallbackTransportAction::CompleteCallback
        }
        Some(LanguageInputCallbackCompletionAction::KeepRunning) => {
            LanguageInputCallbackTransportAction::KeepCallbackRunning
        }
        Some(LanguageInputCallbackCompletionAction::DropAfterBudgetExceeded) => {
            LanguageInputCallbackTransportAction::DropAfterBudgetExceeded
        }
        Some(LanguageInputCallbackCompletionAction::DropAfterFailure) => {
            LanguageInputCallbackTransportAction::DropAfterFailure
        }
        None => LanguageInputCallbackTransportAction::DispatchCallback,
    }
}

fn input_callback_transport_action_label(
    lifecycle: &LanguageInputCallbackSessionLifecycleSummary,
    action: LanguageInputCallbackTransportAction,
) -> String {
    let queue_plan = &lifecycle.queue_summary.queue_plan;
    format!(
        "{} callback={}:{} sequence={} transport_action={} terminal={} retryable={}",
        lifecycle.connection_label,
        queue_plan.board_id,
        queue_plan.label,
        queue_plan.sequence,
        input_callback_transport_action_name(action),
        lifecycle.terminal,
        lifecycle.retryable
    )
}

fn input_callback_transport_action_message(
    action: LanguageInputCallbackTransportAction,
) -> &'static str {
    match action {
        LanguageInputCallbackTransportAction::DropBeforeDispatch => {
            "Transport should report the dropped input callback without dispatch."
        }
        LanguageInputCallbackTransportAction::DispatchCallback => {
            "Transport should dispatch the queued input callback."
        }
        LanguageInputCallbackTransportAction::CompleteCallback => {
            "Transport should report completion and remove the callback."
        }
        LanguageInputCallbackTransportAction::KeepCallbackRunning => {
            "Transport should keep the callback scheduled for cooperative dispatch."
        }
        LanguageInputCallbackTransportAction::DropAfterBudgetExceeded => {
            "Transport should report the budget exhaustion and remove the callback."
        }
        LanguageInputCallbackTransportAction::DropAfterFailure => {
            "Transport should report the callback failure and remove the callback."
        }
    }
}

fn input_callback_transport_effect_dispatches_callback(
    action: LanguageInputCallbackTransportAction,
) -> bool {
    matches!(
        action,
        LanguageInputCallbackTransportAction::DispatchCallback
    )
}

fn input_callback_transport_effect_emits_drop(
    action: LanguageInputCallbackTransportAction,
) -> bool {
    matches!(
        action,
        LanguageInputCallbackTransportAction::DropBeforeDispatch
    )
}

fn input_callback_transport_effect_emits_result(
    action: LanguageInputCallbackTransportAction,
) -> bool {
    matches!(
        action,
        LanguageInputCallbackTransportAction::CompleteCallback
            | LanguageInputCallbackTransportAction::KeepCallbackRunning
            | LanguageInputCallbackTransportAction::DropAfterBudgetExceeded
            | LanguageInputCallbackTransportAction::DropAfterFailure
    )
}

fn input_callback_transport_effect_removes_queue_item(
    action: LanguageInputCallbackTransportAction,
) -> bool {
    matches!(
        action,
        LanguageInputCallbackTransportAction::CompleteCallback
            | LanguageInputCallbackTransportAction::DropAfterBudgetExceeded
            | LanguageInputCallbackTransportAction::DropAfterFailure
    )
}

fn input_callback_transport_effect_keeps_dispatch_scheduled(
    action: LanguageInputCallbackTransportAction,
) -> bool {
    matches!(
        action,
        LanguageInputCallbackTransportAction::DispatchCallback
            | LanguageInputCallbackTransportAction::KeepCallbackRunning
    )
}

fn input_callback_transport_effect_label(
    action_summary: &LanguageInputCallbackTransportActionSummary,
) -> String {
    format!(
        "{} dispatch_callback={} emit_drop={} emit_result={} remove_from_queue={} keep_dispatch_scheduled={} queue_depth_after_effect={}",
        action_summary.action_label,
        input_callback_transport_effect_dispatches_callback(action_summary.action),
        input_callback_transport_effect_emits_drop(action_summary.action),
        input_callback_transport_effect_emits_result(action_summary.action),
        input_callback_transport_effect_removes_queue_item(action_summary.action),
        input_callback_transport_effect_keeps_dispatch_scheduled(action_summary.action),
        action_summary
            .queue_depth_after_completion
            .unwrap_or(action_summary.queue_depth_after)
    )
}

fn input_callback_transport_effect_message(
    action: LanguageInputCallbackTransportAction,
) -> &'static str {
    match action {
        LanguageInputCallbackTransportAction::DropBeforeDispatch => {
            "Transport should emit a drop notice without touching the existing queue."
        }
        LanguageInputCallbackTransportAction::DispatchCallback => {
            "Transport should dispatch the queued callback and keep dispatch scheduled."
        }
        LanguageInputCallbackTransportAction::CompleteCallback => {
            "Transport should emit the result and remove the completed callback from the queue."
        }
        LanguageInputCallbackTransportAction::KeepCallbackRunning => {
            "Transport should emit the running result and keep dispatch scheduled."
        }
        LanguageInputCallbackTransportAction::DropAfterBudgetExceeded => {
            "Transport should emit the budget result and remove the callback from the queue."
        }
        LanguageInputCallbackTransportAction::DropAfterFailure => {
            "Transport should emit the failure result and remove the callback from the queue."
        }
    }
}

fn input_callback_transport_report_kind(
    action: LanguageInputCallbackTransportAction,
) -> LanguageInputCallbackTransportReportKind {
    match action {
        LanguageInputCallbackTransportAction::DropBeforeDispatch => {
            LanguageInputCallbackTransportReportKind::Drop
        }
        LanguageInputCallbackTransportAction::DispatchCallback => {
            LanguageInputCallbackTransportReportKind::Dispatch
        }
        LanguageInputCallbackTransportAction::CompleteCallback => {
            LanguageInputCallbackTransportReportKind::Completion
        }
        LanguageInputCallbackTransportAction::KeepCallbackRunning => {
            LanguageInputCallbackTransportReportKind::Running
        }
        LanguageInputCallbackTransportAction::DropAfterBudgetExceeded => {
            LanguageInputCallbackTransportReportKind::BudgetExceeded
        }
        LanguageInputCallbackTransportAction::DropAfterFailure => {
            LanguageInputCallbackTransportReportKind::Failure
        }
    }
}

fn input_callback_transport_report_emits_report(
    effect_summary: &LanguageInputCallbackTransportEffectSummary,
) -> bool {
    effect_summary.emit_drop || effect_summary.emit_result
}

fn input_callback_transport_report_label(
    effect_summary: &LanguageInputCallbackTransportEffectSummary,
    report_kind: LanguageInputCallbackTransportReportKind,
) -> String {
    format!(
        "{} transport_report={} emit_report={} queue_depth_after_report={}",
        effect_summary.effect_label,
        input_callback_transport_report_kind_name(report_kind),
        input_callback_transport_report_emits_report(effect_summary),
        effect_summary.queue_depth_after_effect
    )
}

fn input_callback_transport_report_message(
    report_kind: LanguageInputCallbackTransportReportKind,
) -> &'static str {
    match report_kind {
        LanguageInputCallbackTransportReportKind::Dispatch => {
            "Transport should dispatch the callback runner; no report is emitted yet."
        }
        LanguageInputCallbackTransportReportKind::Drop => {
            "Transport should emit a dropped-callback report."
        }
        LanguageInputCallbackTransportReportKind::Completion => {
            "Transport should emit a completed-callback report."
        }
        LanguageInputCallbackTransportReportKind::Running => {
            "Transport should emit a running-callback report and keep dispatch scheduled."
        }
        LanguageInputCallbackTransportReportKind::BudgetExceeded => {
            "Transport should emit a budget-exceeded callback report."
        }
        LanguageInputCallbackTransportReportKind::Failure => {
            "Transport should emit a failed-callback report."
        }
    }
}

fn input_callback_transport_event_kind(
    report_kind: LanguageInputCallbackTransportReportKind,
) -> LanguageInputCallbackTransportEventKind {
    match report_kind {
        LanguageInputCallbackTransportReportKind::Dispatch => {
            LanguageInputCallbackTransportEventKind::DispatchScheduled
        }
        LanguageInputCallbackTransportReportKind::Drop => {
            LanguageInputCallbackTransportEventKind::CallbackDropped
        }
        LanguageInputCallbackTransportReportKind::Completion => {
            LanguageInputCallbackTransportEventKind::CallbackCompleted
        }
        LanguageInputCallbackTransportReportKind::Running => {
            LanguageInputCallbackTransportEventKind::CallbackRunning
        }
        LanguageInputCallbackTransportReportKind::BudgetExceeded => {
            LanguageInputCallbackTransportEventKind::CallbackBudgetExceeded
        }
        LanguageInputCallbackTransportReportKind::Failure => {
            LanguageInputCallbackTransportEventKind::CallbackFailed
        }
    }
}

fn input_callback_transport_event_label(
    report_summary: &LanguageInputCallbackTransportReportSummary,
    event_kind: LanguageInputCallbackTransportEventKind,
) -> String {
    format!(
        "{} transport_event={} queue_depth_after_event={}",
        report_summary.report_label,
        input_callback_transport_event_kind_name(event_kind),
        report_summary.queue_depth_after_report
    )
}

fn input_callback_transport_event_message(
    event_kind: LanguageInputCallbackTransportEventKind,
) -> &'static str {
    match event_kind {
        LanguageInputCallbackTransportEventKind::DispatchScheduled => {
            "Adapter should schedule the callback dispatch runner."
        }
        LanguageInputCallbackTransportEventKind::CallbackDropped => {
            "Adapter should emit a dropped input callback event."
        }
        LanguageInputCallbackTransportEventKind::CallbackCompleted => {
            "Adapter should emit a completed input callback event."
        }
        LanguageInputCallbackTransportEventKind::CallbackRunning => {
            "Adapter should emit a running input callback event and keep dispatch scheduled."
        }
        LanguageInputCallbackTransportEventKind::CallbackBudgetExceeded => {
            "Adapter should emit a budget-exceeded input callback event."
        }
        LanguageInputCallbackTransportEventKind::CallbackFailed => {
            "Adapter should emit a failed input callback event."
        }
    }
}

fn language_i2c_bus(bus: &TargetI2cBus) -> LanguageI2cBus {
    LanguageI2cBus {
        bus: bus.bus,
        name: bus.name.to_owned(),
        sda_pin: bus.sda_pin,
        scl_pin: bus.scl_pin,
        qwiic: bus.qwiic,
        notes: bus.notes.to_owned(),
    }
}

fn language_i2c_connector(connector: &TargetI2cConnector) -> LanguageI2cConnector {
    LanguageI2cConnector {
        bus: connector.bus,
        name: connector.name.to_owned(),
        connector: connector.connector.to_owned(),
        arduino_object: connector.arduino_object.to_owned(),
        controller: connector.controller.to_owned(),
        notes: connector.notes.to_owned(),
    }
}

fn language_spi_bus(bus: &TargetSpiBus) -> LanguageSpiBus {
    LanguageSpiBus {
        bus: bus.bus,
        name: bus.name.to_owned(),
        copi_pin: bus.copi_pin,
        cipo_pin: bus.cipo_pin,
        sck_pin: bus.sck_pin,
        default_cs_pin: bus.default_cs_pin,
        notes: bus.notes.to_owned(),
    }
}

fn language_uart_bus(bus: &TargetUartBus) -> LanguageUartBus {
    LanguageUartBus {
        bus: bus.bus,
        name: bus.name.to_owned(),
        tx_pin: bus.tx_pin,
        rx_pin: bus.rx_pin,
        arduino_uart: bus.arduino_uart,
        internal: bus.internal,
        notes: bus.notes.to_owned(),
    }
}

fn language_usb_interface(interface: &TargetUsbInterface) -> LanguageUsbInterface {
    LanguageUsbInterface {
        interface: interface.interface,
        name: interface.name.to_owned(),
        controller: interface.controller.to_owned(),
        class: interface.class.to_owned(),
        native: interface.native,
        upload: interface.upload,
        command_transport: interface.command_transport,
        notes: interface.notes.to_owned(),
    }
}

fn language_can_bus(bus: &TargetCanBus) -> LanguageCanBus {
    LanguageCanBus {
        bus: bus.bus,
        name: bus.name.to_owned(),
        tx_pin: bus.tx_pin,
        rx_pin: bus.rx_pin,
        controller: bus.controller.to_owned(),
        notes: bus.notes.to_owned(),
    }
}

fn language_rtc(rtc: TargetRtc) -> LanguageRtc {
    LanguageRtc {
        instance: rtc.instance,
        name: rtc.name.to_owned(),
        peripheral: rtc.peripheral.to_owned(),
        notes: rtc.notes.to_owned(),
    }
}

fn language_wireless_interface(interface: &TargetWirelessInterface) -> LanguageWirelessInterface {
    LanguageWirelessInterface {
        transport: language_wireless_transport(interface.transport),
        chip: interface.chip.to_owned(),
        command_transport: interface.command_transport,
        ota_update: interface.ota_update,
    }
}

fn language_network_interface(interface: &TargetNetworkInterface) -> LanguageNetworkInterface {
    LanguageNetworkInterface {
        interface: interface.interface,
        name: interface.name.to_owned(),
        transport: language_wireless_transport(interface.transport),
        chip: interface.chip.to_owned(),
        protocols: interface
            .protocols
            .iter()
            .map(|protocol| language_network_protocol(*protocol))
            .collect(),
        max_sockets: interface.max_sockets,
        notes: interface.notes.to_owned(),
    }
}

fn language_upload_options(board_id: &str, upload: TargetUploadInfo) -> LanguageUploadOptions {
    LanguageUploadOptions {
        board_id: board_id.to_owned(),
        adapter: upload_adapter_name(upload.adapter).to_owned(),
        image_format: upload_image_format_name(upload.image_format).to_owned(),
        transport: upload_transport_name(upload.transport).to_owned(),
        reset_method: upload_reset_method_name(upload.reset_method).to_owned(),
        port_hint: upload
            .port_hint
            .map(upload_port_hint_name)
            .map(str::to_owned),
        command: upload.command.to_owned(),
        platform_id: upload.platform_id.map(str::to_owned),
        fqbn: upload.fqbn.map(str::to_owned),
        notes: upload.notes.to_owned(),
    }
}

fn arduino_cli_port_selection_step(hint: Option<TargetUploadPortHint>) -> &'static str {
    match hint {
        Some(TargetUploadPortHint::UsbSerialBridge) => "select_usb_serial_port",
        Some(TargetUploadPortHint::NativeUsb) => "select_native_usb_port",
        Some(TargetUploadPortHint::ExternalSerialAdapter) => "select_external_serial_adapter",
        _ => "select_serial_port",
    }
}

fn arduino_cli_bootloader_touch_baud(hint: Option<TargetUploadPortHint>) -> Option<u32> {
    match hint {
        Some(TargetUploadPortHint::NativeUsb) => Some(ARDUINO_CLI_NATIVE_USB_BOOTLOADER_TOUCH_BAUD),
        _ => None,
    }
}

fn arduino_cli_expects_port_reenumeration(hint: Option<TargetUploadPortHint>) -> bool {
    matches!(hint, Some(TargetUploadPortHint::NativeUsb))
}

fn arduino_cli_waits_for_runtime_rediscovery(hint: Option<TargetUploadPortHint>) -> bool {
    matches!(hint, Some(TargetUploadPortHint::NativeUsb))
}

fn arduino_cli_port_discovery_notes(hint: Option<TargetUploadPortHint>) -> &'static str {
    match hint {
        Some(TargetUploadPortHint::NativeUsb) => {
            "Native USB Arduino CLI uploads select the runtime CDC port, then the board package owns reset into the bootloader and runtime port rediscovery."
        }
        Some(TargetUploadPortHint::UsbSerialBridge) => {
            "USB serial bridge Arduino CLI uploads keep the adapter path as the selected serial port while the board package owns reset and programmer behavior."
        }
        Some(TargetUploadPortHint::ExternalSerialAdapter) => {
            "External serial adapter Arduino CLI uploads require the caller to provide the adapter port before the board package handles reset and programmer behavior."
        }
        _ => "Arduino CLI upload port discovery is delegated to the board package.",
    }
}

fn arduino_cli_command_parts(command: &str) -> Option<(&str, &str)> {
    let mut parts = command.split_whitespace();
    let executable = parts.next()?;
    let subcommand = parts.next()?;
    if parts.next().is_some() {
        return None;
    }
    Some((executable, subcommand))
}

fn arduino_cli_upload_args_template(fqbn: &str) -> Vec<String> {
    vec![
        "upload".to_owned(),
        "-p".to_owned(),
        ARDUINO_CLI_UPLOAD_PORT_PLACEHOLDER.to_owned(),
        "-b".to_owned(),
        fqbn.to_owned(),
        "-i".to_owned(),
        ARDUINO_CLI_UPLOAD_INPUT_FILE_PLACEHOLDER.to_owned(),
    ]
}

fn arduino_cli_upload_invocation_notes(hint: TargetUploadPortHint) -> &'static str {
    match hint {
        TargetUploadPortHint::NativeUsb => {
            "Fill the port placeholder with the selected or rediscovered native USB CDC port and the input placeholder with a concrete Board VM firmware image."
        }
        TargetUploadPortHint::UsbSerialBridge => {
            "Fill the port placeholder with the USB serial bridge path and the input placeholder with a concrete Board VM firmware image."
        }
        TargetUploadPortHint::ExternalSerialAdapter => {
            "Fill the port placeholder with the external serial adapter path and the input placeholder with a concrete Board VM firmware image."
        }
        _ => "Fill the port placeholder with the Arduino upload port and the input placeholder with a concrete Board VM firmware image.",
    }
}

fn arduino_cli_upload_command_notes() -> &'static str {
    "Concrete Arduino CLI argv built by Rust after target resolution, port selection, and firmware artifact selection."
}

fn arduino_cli_upload_execution_steps(port_hint: TargetUploadPortHint) -> Vec<String> {
    match port_hint {
        TargetUploadPortHint::NativeUsb => vec![
            "use_selected_native_usb_port".to_owned(),
            "delegate_reset_to_board_package".to_owned(),
            "run_arduino_cli_upload".to_owned(),
            "wait_for_runtime_port_rediscovery".to_owned(),
        ],
        TargetUploadPortHint::UsbSerialBridge => vec![
            "use_selected_usb_serial_bridge".to_owned(),
            "delegate_reset_to_board_package".to_owned(),
            "run_arduino_cli_upload".to_owned(),
            "reuse_selected_serial_port".to_owned(),
        ],
        TargetUploadPortHint::ExternalSerialAdapter => vec![
            "use_selected_external_serial_adapter".to_owned(),
            "delegate_reset_to_board_package".to_owned(),
            "run_arduino_cli_upload".to_owned(),
            "reuse_selected_serial_port".to_owned(),
        ],
        _ => vec![
            "use_selected_serial_port".to_owned(),
            "delegate_reset_to_board_package".to_owned(),
            "run_arduino_cli_upload".to_owned(),
        ],
    }
}

fn arduino_cli_upload_execution_notes(port_hint: TargetUploadPortHint) -> &'static str {
    match port_hint {
        TargetUploadPortHint::NativeUsb => {
            "Rust-owned Arduino CLI execution plan delegates native USB bootloader reset to the board package and expects runtime CDC rediscovery after upload."
        }
        TargetUploadPortHint::UsbSerialBridge => {
            "Rust-owned Arduino CLI execution plan keeps the selected USB serial bridge port stable while the board package handles reset and programmer behavior."
        }
        TargetUploadPortHint::ExternalSerialAdapter => {
            "Rust-owned Arduino CLI execution plan requires the caller-provided external serial adapter port and delegates reset/programmer behavior to the board package."
        }
        _ => "Rust-owned Arduino CLI execution plan for running the generated upload command.",
    }
}

fn arduino_cli_upload_process_notes(port_hint: &str) -> &'static str {
    match port_hint {
        "native_usb" => {
            "Launch this Arduino CLI process with stdout and stderr captured; after a successful native USB upload, wait for the runtime CDC port to reappear before opening Board VM transport."
        }
        "usb_serial_bridge" => {
            "Launch this Arduino CLI process with stdout and stderr captured; after success, reuse the selected USB serial bridge port for Board VM transport."
        }
        "external_serial_adapter" => {
            "Launch this Arduino CLI process with stdout and stderr captured; after success, reuse the caller-selected external serial adapter port for Board VM transport."
        }
        _ => "Launch this Arduino CLI process with stdout and stderr captured, then feed the exit code and captured output back into Rust result parsing.",
    }
}

fn arduino_cli_upload_result(
    board_id: String,
    port_hint: String,
    wait_for_runtime_rediscovery: bool,
    exit_code: i32,
    stdout: &str,
    stderr: &str,
) -> LanguageArduinoCliUploadResult {
    arduino_cli_upload_result_with_success_exit_codes(
        board_id,
        port_hint,
        wait_for_runtime_rediscovery,
        &[0],
        exit_code,
        stdout,
        stderr,
    )
}

fn arduino_cli_upload_result_with_success_exit_codes(
    board_id: String,
    port_hint: String,
    wait_for_runtime_rediscovery: bool,
    success_exit_codes: &[i32],
    exit_code: i32,
    stdout: &str,
    stderr: &str,
) -> LanguageArduinoCliUploadResult {
    let failure_kind = if success_exit_codes.contains(&exit_code) {
        None
    } else if exit_code == 0 {
        Some("command_failed")
    } else {
        arduino_cli_upload_failure_kind(exit_code, stdout, stderr)
    };
    let success = failure_kind.is_none();
    let failure_kind_string = failure_kind.map(str::to_owned);

    LanguageArduinoCliUploadResult {
        board_id,
        exit_code,
        success,
        status: if success { "success" } else { "failed" }.to_owned(),
        failure_kind: failure_kind_string,
        retryable: failure_kind.is_some_and(arduino_cli_upload_failure_retryable),
        needs_port_selection: failure_kind == Some("port_not_found"),
        needs_board_package_install: failure_kind == Some("board_package_missing"),
        needs_firmware_artifact: failure_kind == Some("missing_input_file"),
        wait_for_runtime_rediscovery: success && wait_for_runtime_rediscovery,
        port_hint,
        message: arduino_cli_upload_result_message(failure_kind).to_owned(),
        diagnostic: arduino_cli_upload_diagnostic(stdout, stderr),
    }
}

fn arduino_cli_upload_runtime_handoff(
    board_id: String,
    selected_upload_port: &str,
    port_hint: String,
    success: bool,
    wait_for_runtime_rediscovery: bool,
    stdout: &str,
    stderr: &str,
) -> Option<LanguageArduinoCliUploadRuntimeHandoff> {
    if !success {
        return None;
    }

    let selected_upload_port = selected_upload_port.trim();
    let new_upload_port = wait_for_runtime_rediscovery
        .then(|| {
            arduino_cli_new_upload_port(stdout).or_else(|| arduino_cli_new_upload_port(stderr))
        })
        .flatten();
    let (runtime_port, runtime_port_source) = match new_upload_port {
        Some(port) => (port, "arduino_cli_new_upload_port"),
        None => {
            if selected_upload_port.is_empty() {
                return None;
            }

            (selected_upload_port.to_owned(), "selected_upload_port")
        }
    };

    Some(LanguageArduinoCliUploadRuntimeHandoff {
        board_id,
        upload_port: selected_upload_port.to_owned(),
        runtime_port,
        runtime_port_source: runtime_port_source.to_owned(),
        wait_for_runtime_rediscovery,
        port_hint: port_hint.clone(),
        message: arduino_cli_upload_runtime_handoff_message(
            &port_hint,
            runtime_port_source,
            wait_for_runtime_rediscovery,
        )
        .to_owned(),
    })
}

fn arduino_cli_upload_runtime_handoff_message(
    port_hint: &str,
    runtime_port_source: &str,
    wait_for_runtime_rediscovery: bool,
) -> &'static str {
    match (wait_for_runtime_rediscovery, runtime_port_source, port_hint) {
        (true, "arduino_cli_new_upload_port", _) => {
            "Arduino CLI reported the runtime port after native USB upload; open Board VM transport on that port."
        }
        (true, _, _) => {
            "Arduino CLI did not report a new runtime port; wait for native USB runtime rediscovery before opening Board VM transport."
        }
        (false, _, "usb_serial_bridge") => {
            "Upload used an onboard USB serial bridge; reuse the selected port for Board VM transport."
        }
        (false, _, "external_serial_adapter") => {
            "Upload used an external serial adapter; reuse the selected adapter port for Board VM transport."
        }
        _ => "Reuse the selected upload port for Board VM transport after successful upload.",
    }
}

fn arduino_cli_upload_failure_kind(
    exit_code: i32,
    stdout: &str,
    stderr: &str,
) -> Option<&'static str> {
    if exit_code == 0 {
        return None;
    }

    let output = format!("{stderr}\n{stdout}").to_ascii_lowercase();
    if output.contains("permission denied") || output.contains("access is denied") {
        return Some("port_permission_denied");
    }
    if output.contains("no such file or directory")
        || output.contains("file does not exist")
        || (output.contains("input file")
            && (output.contains("not found") || output.contains("does not exist")))
    {
        return Some("missing_input_file");
    }
    if output.contains("platform not installed")
        || output.contains("core is not installed")
        || output.contains("is not installed")
        || output.contains("no fqbn provided")
        || (output.contains("fqbn") && output.contains("not found"))
    {
        return Some("board_package_missing");
    }
    if output.contains("no upload port provided")
        || output.contains("port not found")
        || output.contains("serial port not found")
        || output.contains("no device found")
        || output.contains("couldn't find a board on the selected port")
    {
        return Some("port_not_found");
    }
    if output.contains("verification failed")
        || output.contains("verify failed")
        || output.contains("checksum")
    {
        return Some("verification_failed");
    }
    if output.contains("not in sync")
        || output.contains("programmer is not responding")
        || output.contains("timed out")
        || output.contains("timeout")
        || output.contains("resource busy")
    {
        return Some("upload_transport_error");
    }

    Some("command_failed")
}

fn arduino_cli_upload_failure_retryable(kind: &str) -> bool {
    matches!(
        kind,
        "port_not_found"
            | "port_permission_denied"
            | "verification_failed"
            | "upload_transport_error"
            | "command_failed"
    )
}

fn arduino_cli_upload_result_message(failure_kind: Option<&str>) -> &'static str {
    match failure_kind {
        None => "Arduino CLI upload completed successfully.",
        Some("port_not_found") => {
            "Arduino CLI could not find the selected upload port; select a fresh port before retrying."
        }
        Some("port_permission_denied") => {
            "Arduino CLI could not access the selected upload port; check permissions or close other serial users."
        }
        Some("missing_input_file") => {
            "Arduino CLI could not read the firmware image; rebuild or select the artifact before retrying."
        }
        Some("board_package_missing") => {
            "Arduino CLI is missing the board package or FQBN needed for this target."
        }
        Some("verification_failed") => {
            "Arduino CLI reported upload verification failure; retry after checking the board connection."
        }
        Some("upload_transport_error") => {
            "Arduino CLI reported a transport or programmer error while talking to the board."
        }
        _ => "Arduino CLI upload failed with an unclassified command error.",
    }
}

fn arduino_cli_upload_diagnostic(stdout: &str, stderr: &str) -> String {
    let stderr = stderr.trim();
    if !stderr.is_empty() {
        return stderr.to_owned();
    }

    stdout.trim().to_owned()
}

fn language_upload_plan(board_id: &str, upload: TargetUploadInfo) -> LanguageUploadPlan {
    let options = language_upload_options(board_id, upload);
    match upload.adapter {
        TargetUploadAdapter::ArduinoCli => LanguageUploadPlan {
            board_id: options.board_id,
            adapter: options.adapter,
            image_format: options.image_format,
            transport: options.transport,
            reset_method: options.reset_method,
            port_hint: options.port_hint,
            command: options.command,
            platform_id: options.platform_id,
            fqbn: options.fqbn,
            artifact_kind: "arduino_cli_build_output".to_owned(),
            artifact_extension: None,
            requires_serial_port: true,
            requires_mount_path: false,
            auto_detect_mount: false,
            steps: vec![
                "resolve_arduino_board_package".to_owned(),
                "build_firmware_artifact".to_owned(),
                arduino_cli_port_selection_step(upload.port_hint).to_owned(),
                "delegate_reset_to_board_package".to_owned(),
                "upload_with_arduino_cli".to_owned(),
            ],
            notes: options.notes,
        },
        TargetUploadAdapter::EspRomSerial => LanguageUploadPlan {
            board_id: options.board_id,
            adapter: options.adapter,
            image_format: options.image_format,
            transport: options.transport,
            reset_method: options.reset_method,
            port_hint: options.port_hint,
            command: options.command,
            platform_id: options.platform_id,
            fqbn: options.fqbn,
            artifact_kind: "esp_flash_image".to_owned(),
            artifact_extension: Some(".bin".to_owned()),
            requires_serial_port: true,
            requires_mount_path: false,
            auto_detect_mount: false,
            steps: vec![
                "select_serial_port".to_owned(),
                "reset_into_rom_bootloader".to_owned(),
                "write_flash_image".to_owned(),
                "verify_md5".to_owned(),
                "reset_into_runtime".to_owned(),
            ],
            notes: options.notes,
        },
        TargetUploadAdapter::PicoUf2MassStorage => LanguageUploadPlan {
            board_id: options.board_id,
            adapter: options.adapter,
            image_format: options.image_format,
            transport: options.transport,
            reset_method: options.reset_method,
            port_hint: options.port_hint,
            command: options.command,
            platform_id: options.platform_id,
            fqbn: options.fqbn,
            artifact_kind: "uf2_file".to_owned(),
            artifact_extension: Some(".uf2".to_owned()),
            requires_serial_port: false,
            requires_mount_path: true,
            auto_detect_mount: true,
            steps: vec![
                "enter_bootsel".to_owned(),
                "discover_bootsel_mount".to_owned(),
                "copy_uf2_to_mount".to_owned(),
                "wait_for_runtime_rediscovery".to_owned(),
            ],
            notes: options.notes,
        },
    }
}

fn language_network_protocol(protocol: TargetNetworkProtocol) -> LanguageNetworkProtocol {
    match protocol {
        TargetNetworkProtocol::Ipv4 => LanguageNetworkProtocol::Ipv4,
        TargetNetworkProtocol::Tcp => LanguageNetworkProtocol::Tcp,
        TargetNetworkProtocol::Udp => LanguageNetworkProtocol::Udp,
        TargetNetworkProtocol::Dns => LanguageNetworkProtocol::Dns,
    }
}

fn language_wireless_transport(transport: TargetWirelessTransport) -> LanguageWirelessTransport {
    match transport {
        TargetWirelessTransport::Wifi => LanguageWirelessTransport::Wifi,
        TargetWirelessTransport::BluetoothLe => LanguageWirelessTransport::BluetoothLe,
        TargetWirelessTransport::BluetoothClassic => LanguageWirelessTransport::BluetoothClassic,
    }
}

fn language_connection_options(target: &BoardTargetInfo) -> Vec<LanguageConnectionOption> {
    let mut options = Vec::new();
    if target.capabilities.contains(&"transport.serial") {
        options.push(LanguageConnectionOption {
            transport: LanguageConnectionTransport::Serial,
            display_name: "USB/serial".to_owned(),
            command_transport: true,
            ota_update: false,
            requires: "serial_port".to_owned(),
            endpoint_transport: LanguageHostEndpointTransport::SerialPort,
            endpoint_scheme: "serial".to_owned(),
            wire_protocol: LANGUAGE_BOARD_VM_WIRE_PROTOCOL.to_owned(),
        });
    }

    for interface in target.wireless {
        let transport = language_connection_transport(interface.transport);
        options.push(LanguageConnectionOption {
            transport,
            display_name: connection_transport_display_name(transport).to_owned(),
            command_transport: interface.command_transport,
            ota_update: interface.ota_update,
            requires: connection_transport_requires(transport).to_owned(),
            endpoint_transport: connection_endpoint_transport(transport),
            endpoint_scheme: connection_endpoint_scheme(transport).to_owned(),
            wire_protocol: LANGUAGE_BOARD_VM_WIRE_PROTOCOL.to_owned(),
        });
    }

    options
}

fn language_serial_runtime_open_plan(
    target: &LanguageTargetInfo,
    runtime_port: &str,
    port_source: &str,
) -> Option<LanguageSerialRuntimeOpenPlan> {
    let runtime_port = runtime_port.trim();
    if runtime_port.is_empty()
        || !target
            .connection_options
            .iter()
            .any(|option| option.transport == LanguageConnectionTransport::Serial)
    {
        return None;
    }

    let upload_port_hint = target
        .upload
        .as_ref()
        .and_then(|upload| upload.port_hint.clone());
    Some(LanguageSerialRuntimeOpenPlan {
        board_id: target.board_id.clone(),
        port: runtime_port.to_owned(),
        port_source: port_source.to_owned(),
        endpoint: serial_runtime_endpoint(runtime_port),
        transport: LanguageConnectionTransport::Serial,
        endpoint_transport: LanguageHostEndpointTransport::SerialPort,
        endpoint_scheme: "serial".to_owned(),
        wire_protocol: LANGUAGE_BOARD_VM_WIRE_PROTOCOL.to_owned(),
        baud_rate: LANGUAGE_SERIAL_DEFAULT_BAUD_RATE,
        timeout_ms: LANGUAGE_SERIAL_DEFAULT_TIMEOUT_MS,
        data_bits: 8,
        parity: "none".to_owned(),
        stop_bits: 1,
        flow_control: "none".to_owned(),
        dtr_on_open: true,
        clear_on_open: true,
        settle_on_open_ms: LANGUAGE_SERIAL_OPEN_SETTLE_MS,
        hello_after_open: true,
        notes: serial_runtime_open_notes(upload_port_hint.as_deref()).to_owned(),
        upload_port_hint,
    })
}

fn serial_runtime_endpoint(port: &str) -> String {
    format!("serial://{port}")
}

fn serial_runtime_open_notes(upload_port_hint: Option<&str>) -> &'static str {
    match upload_port_hint {
        Some("native_usb") => {
            "Open the runtime CDC serial port after native USB rediscovery, clear stale bytes, then send Board VM HELLO."
        }
        Some("usb_serial_bridge") => {
            "Open the selected USB serial bridge port, clear stale bytes, then send Board VM HELLO."
        }
        Some("external_serial_adapter") => {
            "Open the caller-selected external serial adapter, clear stale bytes, then send Board VM HELLO."
        }
        Some("esp_rom_serial") => {
            "Open the ESP runtime serial port after ROM upload reset, clear stale bytes, then send Board VM HELLO."
        }
        _ => {
            "Open the selected runtime serial port, clear stale bytes, then send Board VM HELLO."
        }
    }
}

fn language_connection_transport(
    transport: TargetWirelessTransport,
) -> LanguageConnectionTransport {
    match transport {
        TargetWirelessTransport::Wifi => LanguageConnectionTransport::Wifi,
        TargetWirelessTransport::BluetoothLe => LanguageConnectionTransport::BluetoothLe,
        TargetWirelessTransport::BluetoothClassic => LanguageConnectionTransport::BluetoothClassic,
    }
}

const fn connection_transport_display_name(transport: LanguageConnectionTransport) -> &'static str {
    match transport {
        LanguageConnectionTransport::Serial => "USB/serial",
        LanguageConnectionTransport::Wifi => "Wi-Fi",
        LanguageConnectionTransport::BluetoothLe => "Bluetooth LE",
        LanguageConnectionTransport::BluetoothClassic => "Bluetooth Classic",
    }
}

const fn connection_transport_requires(transport: LanguageConnectionTransport) -> &'static str {
    match transport {
        LanguageConnectionTransport::Serial => "serial_port",
        LanguageConnectionTransport::Wifi => "network_endpoint",
        LanguageConnectionTransport::BluetoothLe
        | LanguageConnectionTransport::BluetoothClassic => "paired_device",
    }
}

const fn connection_endpoint_transport(
    transport: LanguageConnectionTransport,
) -> LanguageHostEndpointTransport {
    match transport {
        LanguageConnectionTransport::Serial => LanguageHostEndpointTransport::SerialPort,
        LanguageConnectionTransport::Wifi => LanguageHostEndpointTransport::TcpSocket,
        LanguageConnectionTransport::BluetoothLe => LanguageHostEndpointTransport::BluetoothLeGatt,
        LanguageConnectionTransport::BluetoothClassic => {
            LanguageHostEndpointTransport::BluetoothClassicRfcomm
        }
    }
}

const fn connection_endpoint_scheme(transport: LanguageConnectionTransport) -> &'static str {
    match transport {
        LanguageConnectionTransport::Serial => "serial",
        LanguageConnectionTransport::Wifi => "tcp",
        LanguageConnectionTransport::BluetoothLe => "ble",
        LanguageConnectionTransport::BluetoothClassic => "btspp",
    }
}

fn bluetooth_endpoint_scheme(endpoint: &str) -> &str {
    endpoint
        .split_once("://")
        .map(|(scheme, _)| scheme)
        .unwrap_or("")
}

fn language_family(family: BoardFamily) -> LanguageBoardFamily {
    match family {
        BoardFamily::Arduino => LanguageBoardFamily::Arduino,
        BoardFamily::ArduinoUnoR4 => LanguageBoardFamily::ArduinoUnoR4,
        BoardFamily::Esp32 => LanguageBoardFamily::Esp32,
        BoardFamily::RaspberryPiPico => LanguageBoardFamily::RaspberryPiPico,
    }
}

fn language_onboard_led(led: TargetOnboardLed) -> LanguageOnboardLed {
    match led {
        TargetOnboardLed::Gpio(pin) => LanguageOnboardLed::Gpio(pin),
        TargetOnboardLed::WirelessChipGpio(pin) => LanguageOnboardLed::WirelessChipGpio(pin),
    }
}

fn env_device_paths() -> Vec<String> {
    env::var_os("BOARD_VM_DEVICE_PATHS")
        .map(|paths| {
            env::split_paths(&paths)
                .map(|path| path.to_string_lossy().into_owned())
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(unix)]
fn unix_device_paths() -> Vec<String> {
    let mut paths = Vec::new();
    collect_matching_dir_paths("/dev", &mut paths, |name| {
        let lower = name.to_ascii_lowercase();
        lower.starts_with("cu.usbmodem")
            || lower.starts_with("tty.usbmodem")
            || lower.starts_with("cu.usbserial")
            || lower.starts_with("tty.usbserial")
            || lower.starts_with("cu.wchusbserial")
            || lower.starts_with("tty.wchusbserial")
            || lower.starts_with("cu.slab_usbtouart")
            || lower.starts_with("tty.slab_usbtouart")
            || lower.starts_with("ttyacm")
            || lower.starts_with("ttyusb")
    });
    collect_matching_dir_paths("/dev/serial/by-id", &mut paths, |_| true);
    paths
}

#[cfg(windows)]
fn windows_device_paths() -> Vec<String> {
    Vec::new()
}

fn collect_matching_dir_paths(dir: &str, paths: &mut Vec<String>, matches: impl Fn(&str) -> bool) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if matches(&name) {
            paths.push(entry.path().to_string_lossy().into_owned());
        }
    }
}

fn default_pico_mount_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    #[cfg(unix)]
    {
        roots.push(PathBuf::from("/Volumes"));
        roots.push(PathBuf::from("/mnt"));
        if let Some(home) = env::var_os("HOME") {
            if let Some(user) = Path::new(&home).file_name() {
                roots.push(PathBuf::from("/media").join(user));
                roots.push(PathBuf::from("/run/media").join(user));
            }
        }
        if let Some(user) = env::var_os("USER") {
            roots.push(PathBuf::from("/media").join(&user));
            roots.push(PathBuf::from("/run/media").join(&user));
        }
    }
    #[cfg(windows)]
    {
        for drive in b'A'..=b'Z' {
            roots.push(PathBuf::from(format!("{}:\\", drive as char)));
        }
    }
    roots
}

fn is_pico_bootsel_mount(path: &Path) -> bool {
    if !path.is_dir() {
        return false;
    }

    let info = path.join("INFO_UF2.TXT");
    if !info.is_file() {
        return false;
    }

    let has_index = path.join("INDEX.HTM").is_file() || path.join("INDEX.HTML").is_file();
    if !has_index {
        return false;
    }

    let Ok(contents) = fs::read_to_string(info) else {
        return false;
    };
    let lower = contents.to_ascii_lowercase();
    lower.contains("uf2 bootloader")
        && (lower.contains("rp2") || lower.contains("rp2040") || lower.contains("raspberry pi"))
}

fn is_serial_candidate(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    lower.contains("usbmodem")
        || lower.contains("usbserial")
        || lower.contains("wchusbserial")
        || lower.contains("slab_usbtouart")
        || lower.contains("ttyacm")
        || lower.contains("ttyusb")
        || lower.contains("cp210")
        || lower.contains("ch340")
        || lower.contains("ftdi")
        || lower.contains("arduino")
        || lower.contains("espressif")
        || lower.contains("esp32")
        || lower.contains("pico")
        || lower.contains("rp2040")
        || lower.starts_with("com")
        || lower.starts_with(r"\\.\com")
}

fn classify_host_device(path: &str) -> LanguageHostDevice {
    let lower = path.to_ascii_lowercase();
    let normalized = normalize_target_selector(path);
    let mut tags = Vec::new();
    push_unique_tag(&mut tags, "serial");

    let (board_id, confidence) = if normalized.contains("pico_w")
        || normalized.contains("picow")
        || normalized.contains("raspberry_pi_pico_w")
    {
        push_unique_tag(&mut tags, "pico");
        push_unique_tag(&mut tags, "rp2040");
        (Some("raspberry-pi-pico-w"), 95)
    } else if normalized.contains("pico") || normalized.contains("rp2040") {
        push_unique_tag(&mut tags, "pico");
        push_unique_tag(&mut tags, "rp2040");
        (Some("raspberry-pi-pico"), 95)
    } else if normalized.contains("esp32")
        || normalized.contains("espressif")
        || normalized.contains("silicon_labs")
        || normalized.contains("cp210")
        || normalized.contains("ch340")
        || normalized.contains("wchusbserial")
        || normalized.contains("slab_usbtouart")
        || normalized.contains("usbserial")
    {
        push_unique_tag(&mut tags, "esp");
        push_unique_tag(&mut tags, "uart");
        (Some("esp32-devkit-v1"), 70)
    } else if normalized.contains("arduino")
        || normalized.contains("uno_r4")
        || normalized.contains("renesas")
    {
        push_unique_tag(&mut tags, "arduino");
        push_unique_tag(&mut tags, "usb_cdc");
        (Some("arduino-uno-r4-wifi"), 85)
    } else {
        if lower.contains("usbmodem") || lower.contains("ttyacm") {
            push_unique_tag(&mut tags, "usb_cdc");
        }
        (None, 0)
    };

    let bootloader = normalized.contains("boot")
        || normalized.contains("bootloader")
        || normalized.contains("cmsis_dap")
        || normalized.contains("daplink")
        || normalized.contains("uf2");
    if bootloader {
        push_unique_tag(&mut tags, "bootloader");
    } else {
        push_unique_tag(&mut tags, "runtime_or_upload");
    }

    let target = board_id.and_then(known_target);
    let target_name = target
        .as_ref()
        .map(|target| target.display_name.as_str())
        .unwrap_or("Board VM serial device");

    LanguageHostDevice {
        id: device_id(path),
        port: path.to_owned(),
        transport: "serial".to_owned(),
        display_name: format!("{target_name} on {path}"),
        target,
        target_confidence: confidence,
        bootloader,
        tags,
    }
}

fn device_dedupe_key(path: &str) -> String {
    fs::canonicalize(path)
        .unwrap_or_else(|_| PathBuf::from(path))
        .to_string_lossy()
        .into_owned()
}

fn device_id(path: &str) -> String {
    let name = Path::new(path)
        .file_name()
        .map(|name| name.to_string_lossy())
        .unwrap_or_else(|| path.into());
    normalize_target_selector(&name).replace('_', "-")
}

fn push_unique_tag(tags: &mut Vec<String>, tag: &str) {
    if !tags.iter().any(|existing| existing == tag) {
        tags.push(tag.to_owned());
    }
}

fn push_flag_name(out: &mut [&'static str], count: usize, name: &'static str) -> usize {
    if let Some(slot) = out.get_mut(count) {
        *slot = name;
        count + 1
    } else {
        count
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LanguageCoreError {
    NullPointer(&'static str),
    InvalidUtf8,
    ValueTooLarge,
    OutputTooSmall,
    Protocol(ProtocolError),
    Host(HostError),
    BluetoothEndpoint,
    BluetoothOpen,
    Transport,
}

impl From<ProtocolError> for LanguageCoreError {
    fn from(value: ProtocolError) -> Self {
        match value {
            ProtocolError::OutputTooSmall | ProtocolError::PayloadTooLarge => Self::OutputTooSmall,
            other => Self::Protocol(other),
        }
    }
}

impl From<HostError> for LanguageCoreError {
    fn from(value: HostError) -> Self {
        match value {
            HostError::OutputTooSmall => Self::OutputTooSmall,
            other => Self::Host(other),
        }
    }
}

impl From<BluetoothOpenError> for LanguageCoreError {
    fn from(_value: BluetoothOpenError) -> Self {
        Self::BluetoothOpen
    }
}

impl From<TransportError> for LanguageCoreError {
    fn from(value: TransportError) -> Self {
        match value {
            TransportError::ResponseTooLarge => Self::OutputTooSmall,
            TransportError::Io => Self::Transport,
        }
    }
}

pub fn build_hello_wire_frame(
    session: &mut BoardVmLanguageSession,
    host_name: &str,
    host_nonce: u32,
    wire_out: &mut [u8],
) -> Result<BuiltWireFrame, LanguageCoreError> {
    let mut payload = vec![0; host_name.len().saturating_add(16)];
    let mut raw = vec![0; host_name.len().saturating_add(32)];
    let mut host = session.host_session();
    let written = host.hello_frame(host_name, host_nonce, &mut payload, &mut raw)?;
    let wire_len = encode_wire_frame(&raw[..written.len], wire_out)?;
    session.update_from_host_session(&host);
    Ok(BuiltWireFrame {
        request_id: written.request_id,
        len: wire_len,
    })
}

pub fn build_caps_query_wire_frame(
    session: &mut BoardVmLanguageSession,
    wire_out: &mut [u8],
) -> Result<BuiltWireFrame, LanguageCoreError> {
    let mut raw = [0u8; 16];
    let mut host = session.host_session();
    let written = host.caps_query_frame(&mut raw)?;
    let wire_len = encode_wire_frame(&raw[..written.len], wire_out)?;
    session.update_from_host_session(&host);
    Ok(BuiltWireFrame {
        request_id: written.request_id,
        len: wire_len,
    })
}

pub fn build_blink_module(
    program: BlinkProgram,
    out: &mut [u8],
) -> Result<usize, LanguageCoreError> {
    Ok(write_blink_module(program, out)?)
}

pub fn build_gpio_read_module(
    program: GpioReadProgram,
    out: &mut [u8],
) -> Result<usize, LanguageCoreError> {
    Ok(write_gpio_read_module(program, out)?)
}

pub fn build_gpio_write_module(
    program: GpioWriteProgram,
    out: &mut [u8],
) -> Result<usize, LanguageCoreError> {
    Ok(write_gpio_write_module(program, out)?)
}

pub fn build_gpio_open_module(
    program: GpioOpenProgram,
    out: &mut [u8],
) -> Result<usize, LanguageCoreError> {
    Ok(write_gpio_open_module(program, out)?)
}

pub fn build_gpio_handle_read_module(
    program: GpioHandleReadProgram,
    out: &mut [u8],
) -> Result<usize, LanguageCoreError> {
    Ok(write_gpio_handle_read_module(program, out)?)
}

pub fn build_gpio_handle_write_module(
    program: GpioHandleWriteProgram,
    out: &mut [u8],
) -> Result<usize, LanguageCoreError> {
    Ok(write_gpio_handle_write_module(program, out)?)
}

pub fn build_gpio_handle_close_module(
    program: GpioHandleCloseProgram,
    out: &mut [u8],
) -> Result<usize, LanguageCoreError> {
    Ok(write_gpio_handle_close_module(program, out)?)
}

pub fn build_time_now_module(
    program: TimeNowProgram,
    out: &mut [u8],
) -> Result<usize, LanguageCoreError> {
    Ok(write_time_now_module(program, out)?)
}

pub fn build_time_sleep_ms_module(
    program: TimeSleepMsProgram,
    out: &mut [u8],
) -> Result<usize, LanguageCoreError> {
    Ok(write_time_sleep_ms_module(program, out)?)
}

pub fn build_led_matrix_frame_module(
    program: LedMatrixFrameProgram,
    out: &mut [u8],
) -> Result<usize, LanguageCoreError> {
    Ok(write_led_matrix_frame_module(program, out)?)
}

pub fn build_pwm_write_module(
    program: PwmWriteProgram,
    out: &mut [u8],
) -> Result<usize, LanguageCoreError> {
    Ok(write_pwm_write_module(program, out)?)
}

pub fn build_adc_read_module(
    program: AdcReadProgram,
    out: &mut [u8],
) -> Result<usize, LanguageCoreError> {
    Ok(write_adc_read_module(program, out)?)
}

pub fn build_dac_write_u12_module(
    program: DacWriteU12Program,
    out: &mut [u8],
) -> Result<usize, LanguageCoreError> {
    Ok(write_dac_write_u12_module(program, out)?)
}

pub fn build_i2c_open_module(
    program: I2cOpenProgram,
    out: &mut [u8],
) -> Result<usize, LanguageCoreError> {
    Ok(write_i2c_open_module(program, out)?)
}

pub fn build_spi_open_module(
    program: SpiOpenProgram,
    out: &mut [u8],
) -> Result<usize, LanguageCoreError> {
    Ok(write_spi_open_module(program, out)?)
}

pub fn build_uart_open_module(
    program: UartOpenProgram,
    out: &mut [u8],
) -> Result<usize, LanguageCoreError> {
    Ok(write_uart_open_module(program, out)?)
}

pub fn build_uart_write_module(
    program: UartWriteProgram,
    out: &mut [u8],
) -> Result<usize, LanguageCoreError> {
    Ok(write_uart_write_module(program, out)?)
}

pub fn build_uart_read_module(
    program: UartReadProgram,
    out: &mut [u8],
) -> Result<usize, LanguageCoreError> {
    Ok(write_uart_read_module(program, out)?)
}

pub fn build_can_open_module(
    program: CanOpenProgram,
    out: &mut [u8],
) -> Result<usize, LanguageCoreError> {
    Ok(write_can_open_module(program, out)?)
}

pub fn build_can_write_module(
    program: CanWriteProgram,
    out: &mut [u8],
) -> Result<usize, LanguageCoreError> {
    Ok(write_can_write_module(program, out)?)
}

pub fn build_can_read_module(
    program: CanReadProgram,
    out: &mut [u8],
) -> Result<usize, LanguageCoreError> {
    Ok(write_can_read_module(program, out)?)
}

pub fn build_rtc_now_module(
    program: RtcNowProgram,
    out: &mut [u8],
) -> Result<usize, LanguageCoreError> {
    Ok(write_rtc_now_module(program, out)?)
}

pub fn build_rtc_set_module(
    program: RtcSetProgram,
    out: &mut [u8],
) -> Result<usize, LanguageCoreError> {
    Ok(write_rtc_set_module(program, out)?)
}

pub fn build_watchdog_configure_module(
    program: WatchdogConfigureProgram,
    out: &mut [u8],
) -> Result<usize, LanguageCoreError> {
    Ok(write_watchdog_configure_module(program, out)?)
}

pub fn build_watchdog_kick_module(
    program: WatchdogKickProgram,
    out: &mut [u8],
) -> Result<usize, LanguageCoreError> {
    Ok(write_watchdog_kick_module(program, out)?)
}

pub fn build_storage_write_module(
    program: StorageWriteProgram<'_>,
    out: &mut [u8],
) -> Result<usize, LanguageCoreError> {
    Ok(write_storage_write_module(program, out)?)
}

pub fn build_storage_read_module(
    program: StorageReadProgram,
    out: &mut [u8],
) -> Result<usize, LanguageCoreError> {
    Ok(write_storage_read_module(program, out)?)
}

pub fn build_storage_size_module(
    program: StorageSizeProgram,
    out: &mut [u8],
) -> Result<usize, LanguageCoreError> {
    Ok(write_storage_size_module(program, out)?)
}

pub fn build_spi_transfer_module(
    program: SpiTransferProgram<'_>,
    out: &mut [u8],
) -> Result<usize, LanguageCoreError> {
    Ok(write_spi_transfer_module(program, out)?)
}

pub fn build_spi_write_module(
    program: SpiWriteProgram<'_>,
    out: &mut [u8],
) -> Result<usize, LanguageCoreError> {
    Ok(write_spi_write_module(program, out)?)
}

pub fn build_spi_read_module(
    program: SpiReadProgram,
    out: &mut [u8],
) -> Result<usize, LanguageCoreError> {
    Ok(write_spi_read_module(program, out)?)
}

pub fn build_i2c_write_u8_module(
    program: I2cWriteU8Program,
    out: &mut [u8],
) -> Result<usize, LanguageCoreError> {
    Ok(write_i2c_write_u8_module(program, out)?)
}

pub fn build_i2c_write_module(
    program: I2cWriteProgram<'_>,
    out: &mut [u8],
) -> Result<usize, LanguageCoreError> {
    Ok(write_i2c_write_module(program, out)?)
}

pub fn build_i2c_read_u8_module(
    program: I2cReadU8Program,
    out: &mut [u8],
) -> Result<usize, LanguageCoreError> {
    Ok(write_i2c_read_u8_module(program, out)?)
}

pub fn build_i2c_read_module(
    program: I2cReadProgram,
    out: &mut [u8],
) -> Result<usize, LanguageCoreError> {
    Ok(write_i2c_read_module(program, out)?)
}

pub fn build_i2c_transfer_module(
    program: I2cTransferProgram<'_>,
    out: &mut [u8],
) -> Result<usize, LanguageCoreError> {
    Ok(write_i2c_transfer_module(program, out)?)
}

pub fn build_raw_module(
    flags: u8,
    max_stack: u8,
    code: &[u8],
    const_pool: &[u8],
    out: &mut [u8],
) -> Result<usize, LanguageCoreError> {
    Ok(write_module(
        ModuleSpec::new(flags, max_stack, code).const_pool(const_pool),
        out,
    )?)
}

pub fn raw_module_len(code_len: u64, const_pool_len: u64) -> Result<usize, LanguageCoreError> {
    if code_len > u32::MAX as u64 || const_pool_len > u32::MAX as u64 {
        return Err(LanguageCoreError::ValueTooLarge);
    }
    let code_len = usize::try_from(code_len).map_err(|_| LanguageCoreError::ValueTooLarge)?;
    let const_pool_len =
        usize::try_from(const_pool_len).map_err(|_| LanguageCoreError::ValueTooLarge)?;
    checked_module_len(code_len, const_pool_len)
}

pub fn build_program_begin_wire_frame(
    session: &mut BoardVmLanguageSession,
    program_id: u16,
    module: &[u8],
    wire_out: &mut [u8],
) -> Result<BuiltWireFrame, LanguageCoreError> {
    let mut payload = [0u8; 16];
    let mut raw = [0u8; 32];
    let mut host = session.host_session();
    let written = host.program_begin_frame(program_id, module, &mut payload, &mut raw)?;
    let wire_len = encode_wire_frame(&raw[..written.len], wire_out)?;
    session.update_from_host_session(&host);
    Ok(BuiltWireFrame {
        request_id: written.request_id,
        len: wire_len,
    })
}

pub fn build_program_chunk_wire_frame(
    session: &mut BoardVmLanguageSession,
    program_id: u16,
    offset: u32,
    chunk: &[u8],
    wire_out: &mut [u8],
) -> Result<BuiltWireFrame, LanguageCoreError> {
    let mut payload = vec![0; chunk.len().saturating_add(16)];
    let mut raw = vec![0; chunk.len().saturating_add(32)];
    let mut host = session.host_session();
    let written = host.program_chunk_frame(program_id, offset, chunk, &mut payload, &mut raw)?;
    let wire_len = encode_wire_frame(&raw[..written.len], wire_out)?;
    session.update_from_host_session(&host);
    Ok(BuiltWireFrame {
        request_id: written.request_id,
        len: wire_len,
    })
}

pub fn build_program_end_wire_frame(
    session: &mut BoardVmLanguageSession,
    program_id: u16,
    wire_out: &mut [u8],
) -> Result<BuiltWireFrame, LanguageCoreError> {
    let mut payload = [0u8; 8];
    let mut raw = [0u8; 16];
    let mut host = session.host_session();
    let written = host.program_end_frame(program_id, &mut payload, &mut raw)?;
    let wire_len = encode_wire_frame(&raw[..written.len], wire_out)?;
    session.update_from_host_session(&host);
    Ok(BuiltWireFrame {
        request_id: written.request_id,
        len: wire_len,
    })
}

pub fn build_store_program_wire_frame(
    session: &mut BoardVmLanguageSession,
    program_id: u16,
    slot: u8,
    boot_policy: u8,
    wire_out: &mut [u8],
) -> Result<BuiltWireFrame, LanguageCoreError> {
    let mut payload = [0u8; 8];
    let mut raw = [0u8; 16];
    let mut host = session.host_session();
    let written = host.store_program_with_boot_policy_frame(
        program_id,
        slot,
        boot_policy,
        &mut payload,
        &mut raw,
    )?;
    let wire_len = encode_wire_frame(&raw[..written.len], wire_out)?;
    session.update_from_host_session(&host);
    Ok(BuiltWireFrame {
        request_id: written.request_id,
        len: wire_len,
    })
}

pub fn build_run_background_wire_frame(
    session: &mut BoardVmLanguageSession,
    program_id: u16,
    instruction_budget: u32,
    wire_out: &mut [u8],
) -> Result<BuiltWireFrame, LanguageCoreError> {
    build_run_wire_frame(
        session,
        program_id,
        DEFAULT_RUN_FLAGS,
        instruction_budget,
        0,
        wire_out,
    )
}

pub fn build_run_wire_frame(
    session: &mut BoardVmLanguageSession,
    program_id: u16,
    flags: u8,
    instruction_budget: u32,
    time_budget_ms: u32,
    wire_out: &mut [u8],
) -> Result<BuiltWireFrame, LanguageCoreError> {
    let mut payload = [0u8; 16];
    let mut raw = [0u8; 32];
    let mut host = session.host_session();
    let written = host.run_frame(
        program_id,
        flags,
        instruction_budget,
        time_budget_ms,
        &mut payload,
        &mut raw,
    )?;
    let wire_len = encode_wire_frame(&raw[..written.len], wire_out)?;
    session.update_from_host_session(&host);
    Ok(BuiltWireFrame {
        request_id: written.request_id,
        len: wire_len,
    })
}

pub fn build_stop_wire_frame(
    session: &mut BoardVmLanguageSession,
    wire_out: &mut [u8],
) -> Result<BuiltWireFrame, LanguageCoreError> {
    let mut raw = [0u8; 16];
    let mut host = session.host_session();
    let written = host.stop_frame(&mut raw)?;
    let wire_len = encode_wire_frame(&raw[..written.len], wire_out)?;
    session.update_from_host_session(&host);
    Ok(BuiltWireFrame {
        request_id: written.request_id,
        len: wire_len,
    })
}

pub fn decode_wire_frame_into_raw(
    wire_frame: &[u8],
    raw_out: &mut [u8],
) -> Result<BoardVmLanguageStatus, LanguageCoreError> {
    let raw_len = decode_wire_frame(wire_frame, raw_out)?;
    let frame = decode_frame(&raw_out[..raw_len])?;
    Ok(BoardVmLanguageStatus::decoded(
        &frame,
        raw_out.as_ptr(),
        raw_len,
    ))
}

pub fn decode_wire_response(
    wire_frame: &[u8],
    raw_out: &mut [u8],
) -> Result<DecodedLanguageResponse, LanguageCoreError> {
    let raw_len = decode_wire_frame(wire_frame, raw_out)?;
    decode_raw_response(&raw_out[..raw_len])
}

pub fn decode_raw_response(raw_frame: &[u8]) -> Result<DecodedLanguageResponse, LanguageCoreError> {
    let frame = decode_frame(raw_frame)?;
    let body = if frame.flags & FLAG_IS_ERROR_RESPONSE != 0 {
        let error = decode_error_payload(frame.payload)?;
        DecodedLanguageResponseBody::Error(LanguageBoardError {
            code: error.code,
            request_id: error.request_id,
            program_id: error.program_id,
            bytecode_offset: error.bytecode_offset,
            message: error.message.to_owned(),
        })
    } else {
        decode_response_body(&frame)?
    };

    Ok(DecodedLanguageResponse {
        request_id: frame.request_id,
        message_type: frame.message_type,
        flags: frame.flags,
        payload_len: frame.payload.len(),
        body,
    })
}

fn decode_response_body(
    frame: &Frame<'_>,
) -> Result<DecodedLanguageResponseBody, LanguageCoreError> {
    match frame.message_type {
        MessageType::HELLO_ACK => {
            let ack = decode_hello_ack(frame.payload)?;
            Ok(DecodedLanguageResponseBody::HelloAck(LanguageHelloAck {
                selected_version: ack.selected_version,
                board_name: ack.board_name.to_owned(),
                runtime_name: ack.runtime_name.to_owned(),
                host_nonce: ack.host_nonce,
                board_nonce: ack.board_nonce,
                max_frame_payload: ack.max_frame_payload,
            }))
        }
        MessageType::CAPS_REPORT => {
            let (header, mut decoder) = decode_caps_report_header(frame.payload)?;
            let mut capabilities = Vec::new();
            for _ in 0..header.capability_count {
                let capability = decoder.read_capability_descriptor()?;
                capabilities.push(LanguageCapability {
                    id: capability.id,
                    version: capability.version,
                    flags: capability.flags,
                    name: capability.name.to_owned(),
                });
            }
            decoder.finish()?;
            Ok(DecodedLanguageResponseBody::CapsReport(
                LanguageBoardDescriptor {
                    board_id: header.board_id.to_owned(),
                    runtime_id: header.runtime_id.to_owned(),
                    max_program_bytes: header.max_program_bytes,
                    max_stack_values: header.max_stack_values,
                    max_handles: header.max_handles,
                    supports_store_program: header.supports_store_program,
                    capabilities,
                },
            ))
        }
        MessageType::PROGRAM_BEGIN => {
            let begin = decode_program_begin(frame.payload)?;
            Ok(DecodedLanguageResponseBody::ProgramBegin(
                LanguageProgramBegin {
                    program_id: begin.program_id,
                    format: begin.format,
                    total_len: begin.total_len,
                    program_crc32: begin.program_crc32,
                },
            ))
        }
        MessageType::PROGRAM_CHUNK => {
            let chunk = decode_program_chunk(frame.payload)?;
            Ok(DecodedLanguageResponseBody::ProgramChunk(
                LanguageProgramChunk {
                    program_id: chunk.program_id,
                    offset: chunk.offset,
                    len: chunk.bytes.len(),
                },
            ))
        }
        MessageType::PROGRAM_END => {
            let end = decode_program_end(frame.payload)?;
            Ok(DecodedLanguageResponseBody::ProgramEnd(
                LanguageProgramEnd {
                    program_id: end.program_id,
                },
            ))
        }
        MessageType::RUN_REPORT => {
            let (report, mut decoder) = decode_run_report_header(frame.payload)?;
            let mut returns = Vec::with_capacity(report.return_count as usize);
            for _ in 0..report.return_count {
                returns.push(language_value_from_protocol(decoder.read_value()?)?);
            }
            decoder.finish()?;
            Ok(DecodedLanguageResponseBody::RunReport(LanguageRunReport {
                program_id: report.program_id,
                status: report.status,
                instructions_executed: report.instructions_executed,
                elapsed_ms: report.elapsed_ms,
                stack_depth: report.stack_depth,
                open_handles: report.open_handles,
                return_count: report.return_count,
                returns,
            }))
        }
        _ => Ok(DecodedLanguageResponseBody::Raw),
    }
}

fn language_value_from_protocol(
    value: ProtocolValue<'_>,
) -> Result<LanguageValue, LanguageCoreError> {
    Ok(match value {
        ProtocolValue::Unit => LanguageValue::Unit,
        ProtocolValue::Bool(value) => LanguageValue::Bool(value),
        ProtocolValue::U8(value) => LanguageValue::U8(value),
        ProtocolValue::U16(value) => LanguageValue::U16(value),
        ProtocolValue::U32(value) => LanguageValue::U32(value),
        ProtocolValue::I16(value) => LanguageValue::I16(value),
        ProtocolValue::Handle(value) => LanguageValue::Handle(value),
        ProtocolValue::Bytes(value) => LanguageValue::Bytes(value.to_vec()),
        ProtocolValue::String(value) => LanguageValue::String(value.to_owned()),
    })
}

thread_local! {
    static LAST_ERROR_CODE: Cell<u32> = const { Cell::new(BoardVmLanguageStatusCode::Ok as u32) };
    static LAST_ERROR_MESSAGE: RefCell<Option<CString>> = const { RefCell::new(None) };
}

#[no_mangle]
pub extern "C" fn board_vm_language_core_version_major() -> u16 {
    LANGUAGE_CORE_VERSION_MAJOR
}

#[no_mangle]
pub extern "C" fn board_vm_language_core_version_minor() -> u16 {
    LANGUAGE_CORE_VERSION_MINOR
}

#[no_mangle]
pub extern "C" fn board_vm_language_core_version_patch() -> u16 {
    LANGUAGE_CORE_VERSION_PATCH
}

#[no_mangle]
pub extern "C" fn board_vm_language_last_error_code() -> u32 {
    LAST_ERROR_CODE.with(Cell::get)
}

#[no_mangle]
pub extern "C" fn board_vm_language_last_error_message() -> *const std::ffi::c_char {
    LAST_ERROR_MESSAGE.with(|slot| {
        slot.borrow()
            .as_ref()
            .map(|message| message.as_ptr())
            .unwrap_or(ptr::null())
    })
}

#[no_mangle]
pub unsafe extern "C" fn board_vm_language_session_init(
    session: *mut BoardVmLanguageSession,
    next_request_id: u16,
) -> BoardVmLanguageStatus {
    catch_status(|| {
        let session = unsafe { mut_ref(session, "board_vm_language_session_init session") }?;
        *session = BoardVmLanguageSession::with_next_request_id(next_request_id);
        Ok(BoardVmLanguageStatus::ok())
    })
}

#[no_mangle]
pub unsafe extern "C" fn board_vm_language_session_next_request_id(
    session: *const BoardVmLanguageSession,
) -> u16 {
    clear_error();
    match panic::catch_unwind(AssertUnwindSafe(|| {
        unsafe { ref_from_ptr(session, "board_vm_language_session_next_request_id session") }
            .map(BoardVmLanguageSession::next_request_id)
            .unwrap_or(0)
    })) {
        Ok(value) => value,
        Err(_) => {
            set_error(
                BoardVmLanguageStatusCode::Panic,
                "board_vm_language_session_next_request_id caught a Rust panic.",
            );
            0
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn board_vm_language_hello_wire(
    session: *mut BoardVmLanguageSession,
    host_name: *const u8,
    host_name_len: u64,
    host_nonce: u32,
    wire_out: *mut u8,
    wire_cap: u64,
) -> BoardVmLanguageStatus {
    catch_status(|| {
        let session = unsafe { mut_ref(session, "board_vm_language_hello_wire session") }?;
        let host_name = unsafe { utf8_from_ptr(host_name, host_name_len, "host_name") }?;
        let wire_out = unsafe { out_slice(wire_out, wire_cap, "wire_out") }?;
        let written = build_hello_wire_frame(session, host_name, host_nonce, wire_out)?;
        Ok(BoardVmLanguageStatus::written(
            written.request_id,
            written.len,
        ))
    })
}

#[no_mangle]
pub unsafe extern "C" fn board_vm_language_caps_query_wire(
    session: *mut BoardVmLanguageSession,
    wire_out: *mut u8,
    wire_cap: u64,
) -> BoardVmLanguageStatus {
    catch_status(|| {
        let session = unsafe { mut_ref(session, "board_vm_language_caps_query_wire session") }?;
        let wire_out = unsafe { out_slice(wire_out, wire_cap, "wire_out") }?;
        let written = build_caps_query_wire_frame(session, wire_out)?;
        Ok(BoardVmLanguageStatus::written(
            written.request_id,
            written.len,
        ))
    })
}

#[no_mangle]
pub unsafe extern "C" fn board_vm_language_blink_module(
    pin: u8,
    high_ms: u16,
    low_ms: u16,
    max_stack: u8,
    module_out: *mut u8,
    module_cap: u64,
) -> BoardVmLanguageStatus {
    catch_status(|| {
        let module_out = unsafe { out_slice(module_out, module_cap, "module_out") }?;
        let len = build_blink_module(
            BlinkProgram {
                pin,
                high_ms,
                low_ms,
                max_stack,
            },
            module_out,
        )?;
        Ok(BoardVmLanguageStatus {
            len: len as u64,
            ..BoardVmLanguageStatus::ok()
        })
    })
}

#[no_mangle]
pub unsafe extern "C" fn board_vm_language_gpio_read_module(
    pin: u8,
    mode: u8,
    max_stack: u8,
    module_out: *mut u8,
    module_cap: u64,
) -> BoardVmLanguageStatus {
    catch_status(|| {
        let module_out = unsafe { out_slice(module_out, module_cap, "module_out") }?;
        let len = build_gpio_read_module(
            GpioReadProgram {
                pin,
                mode,
                max_stack,
            },
            module_out,
        )?;
        Ok(BoardVmLanguageStatus {
            len: len as u64,
            ..BoardVmLanguageStatus::ok()
        })
    })
}

#[no_mangle]
pub unsafe extern "C" fn board_vm_language_gpio_write_module(
    pin: u8,
    value: u8,
    max_stack: u8,
    module_out: *mut u8,
    module_cap: u64,
) -> BoardVmLanguageStatus {
    catch_status(|| {
        let module_out = unsafe { out_slice(module_out, module_cap, "module_out") }?;
        let len = build_gpio_write_module(
            GpioWriteProgram {
                pin,
                value: value != 0,
                max_stack,
            },
            module_out,
        )?;
        Ok(BoardVmLanguageStatus {
            len: len as u64,
            ..BoardVmLanguageStatus::ok()
        })
    })
}

#[no_mangle]
pub unsafe extern "C" fn board_vm_language_gpio_open_module(
    pin: u8,
    mode: u8,
    max_stack: u8,
    module_out: *mut u8,
    module_cap: u64,
) -> BoardVmLanguageStatus {
    catch_status(|| {
        let module_out = unsafe { out_slice(module_out, module_cap, "module_out") }?;
        let len = build_gpio_open_module(
            GpioOpenProgram {
                pin,
                mode,
                max_stack,
            },
            module_out,
        )?;
        Ok(BoardVmLanguageStatus {
            len: len as u64,
            ..BoardVmLanguageStatus::ok()
        })
    })
}

#[no_mangle]
pub unsafe extern "C" fn board_vm_language_gpio_handle_read_module(
    max_stack: u8,
    module_out: *mut u8,
    module_cap: u64,
) -> BoardVmLanguageStatus {
    catch_status(|| {
        let module_out = unsafe { out_slice(module_out, module_cap, "module_out") }?;
        let len = build_gpio_handle_read_module(GpioHandleReadProgram { max_stack }, module_out)?;
        Ok(BoardVmLanguageStatus {
            len: len as u64,
            ..BoardVmLanguageStatus::ok()
        })
    })
}

#[no_mangle]
pub unsafe extern "C" fn board_vm_language_gpio_handle_write_module(
    value: u8,
    max_stack: u8,
    module_out: *mut u8,
    module_cap: u64,
) -> BoardVmLanguageStatus {
    catch_status(|| {
        let module_out = unsafe { out_slice(module_out, module_cap, "module_out") }?;
        let len = build_gpio_handle_write_module(
            GpioHandleWriteProgram {
                value: value != 0,
                max_stack,
            },
            module_out,
        )?;
        Ok(BoardVmLanguageStatus {
            len: len as u64,
            ..BoardVmLanguageStatus::ok()
        })
    })
}

#[no_mangle]
pub unsafe extern "C" fn board_vm_language_gpio_handle_close_module(
    max_stack: u8,
    module_out: *mut u8,
    module_cap: u64,
) -> BoardVmLanguageStatus {
    catch_status(|| {
        let module_out = unsafe { out_slice(module_out, module_cap, "module_out") }?;
        let len = build_gpio_handle_close_module(GpioHandleCloseProgram { max_stack }, module_out)?;
        Ok(BoardVmLanguageStatus {
            len: len as u64,
            ..BoardVmLanguageStatus::ok()
        })
    })
}

#[no_mangle]
pub unsafe extern "C" fn board_vm_language_time_now_module(
    max_stack: u8,
    module_out: *mut u8,
    module_cap: u64,
) -> BoardVmLanguageStatus {
    catch_status(|| {
        let module_out = unsafe { out_slice(module_out, module_cap, "module_out") }?;
        let len = build_time_now_module(TimeNowProgram { max_stack }, module_out)?;
        Ok(BoardVmLanguageStatus {
            len: len as u64,
            ..BoardVmLanguageStatus::ok()
        })
    })
}

#[no_mangle]
pub unsafe extern "C" fn board_vm_language_time_sleep_ms_module(
    duration_ms: u16,
    max_stack: u8,
    module_out: *mut u8,
    module_cap: u64,
) -> BoardVmLanguageStatus {
    catch_status(|| {
        let module_out = unsafe { out_slice(module_out, module_cap, "module_out") }?;
        let len = build_time_sleep_ms_module(
            TimeSleepMsProgram {
                duration_ms,
                max_stack,
            },
            module_out,
        )?;
        Ok(BoardVmLanguageStatus {
            len: len as u64,
            ..BoardVmLanguageStatus::ok()
        })
    })
}

#[no_mangle]
pub unsafe extern "C" fn board_vm_language_led_matrix_frame_module(
    word0: u32,
    word1: u32,
    word2: u32,
    max_stack: u8,
    module_out: *mut u8,
    module_cap: u64,
) -> BoardVmLanguageStatus {
    catch_status(|| {
        let module_out = unsafe { out_slice(module_out, module_cap, "module_out") }?;
        let len = build_led_matrix_frame_module(
            LedMatrixFrameProgram {
                words: [word0, word1, word2],
                max_stack,
            },
            module_out,
        )?;
        Ok(BoardVmLanguageStatus {
            len: len as u64,
            ..BoardVmLanguageStatus::ok()
        })
    })
}

#[no_mangle]
pub unsafe extern "C" fn board_vm_language_pwm_write_module(
    pin: u8,
    duty: u16,
    max_stack: u8,
    module_out: *mut u8,
    module_cap: u64,
) -> BoardVmLanguageStatus {
    catch_status(|| {
        let module_out = unsafe { out_slice(module_out, module_cap, "module_out") }?;
        let len = build_pwm_write_module(
            PwmWriteProgram {
                pin,
                duty,
                max_stack,
            },
            module_out,
        )?;
        Ok(BoardVmLanguageStatus {
            len: len as u64,
            ..BoardVmLanguageStatus::ok()
        })
    })
}

#[no_mangle]
pub unsafe extern "C" fn board_vm_language_adc_read_module(
    pin: u8,
    max_stack: u8,
    module_out: *mut u8,
    module_cap: u64,
) -> BoardVmLanguageStatus {
    catch_status(|| {
        let module_out = unsafe { out_slice(module_out, module_cap, "module_out") }?;
        let len = build_adc_read_module(AdcReadProgram { pin, max_stack }, module_out)?;
        Ok(BoardVmLanguageStatus {
            len: len as u64,
            ..BoardVmLanguageStatus::ok()
        })
    })
}

#[no_mangle]
pub unsafe extern "C" fn board_vm_language_dac_write_u12_module(
    pin: u8,
    sample: u16,
    max_stack: u8,
    module_out: *mut u8,
    module_cap: u64,
) -> BoardVmLanguageStatus {
    catch_status(|| {
        let module_out = unsafe { out_slice(module_out, module_cap, "module_out") }?;
        let len = build_dac_write_u12_module(
            DacWriteU12Program {
                pin,
                sample,
                max_stack,
            },
            module_out,
        )?;
        Ok(BoardVmLanguageStatus {
            len: len as u64,
            ..BoardVmLanguageStatus::ok()
        })
    })
}

#[no_mangle]
pub unsafe extern "C" fn board_vm_language_i2c_open_module(
    bus: u8,
    max_stack: u8,
    module_out: *mut u8,
    module_cap: u64,
) -> BoardVmLanguageStatus {
    catch_status(|| {
        let module_out = unsafe { out_slice(module_out, module_cap, "module_out") }?;
        let len = build_i2c_open_module(I2cOpenProgram { bus, max_stack }, module_out)?;
        Ok(BoardVmLanguageStatus {
            len: len as u64,
            ..BoardVmLanguageStatus::ok()
        })
    })
}

#[no_mangle]
pub unsafe extern "C" fn board_vm_language_spi_open_module(
    bus: u8,
    max_stack: u8,
    module_out: *mut u8,
    module_cap: u64,
) -> BoardVmLanguageStatus {
    catch_status(|| {
        let module_out = unsafe { out_slice(module_out, module_cap, "module_out") }?;
        let len = build_spi_open_module(SpiOpenProgram { bus, max_stack }, module_out)?;
        Ok(BoardVmLanguageStatus {
            len: len as u64,
            ..BoardVmLanguageStatus::ok()
        })
    })
}

#[no_mangle]
pub unsafe extern "C" fn board_vm_language_uart_open_module(
    bus: u8,
    max_stack: u8,
    module_out: *mut u8,
    module_cap: u64,
) -> BoardVmLanguageStatus {
    catch_status(|| {
        let module_out = unsafe { out_slice(module_out, module_cap, "module_out") }?;
        let len = build_uart_open_module(UartOpenProgram { bus, max_stack }, module_out)?;
        Ok(BoardVmLanguageStatus {
            len: len as u64,
            ..BoardVmLanguageStatus::ok()
        })
    })
}

#[no_mangle]
pub unsafe extern "C" fn board_vm_language_uart_write_module(
    byte: u8,
    max_stack: u8,
    module_out: *mut u8,
    module_cap: u64,
) -> BoardVmLanguageStatus {
    catch_status(|| {
        let module_out = unsafe { out_slice(module_out, module_cap, "module_out") }?;
        let len = build_uart_write_module(UartWriteProgram { byte, max_stack }, module_out)?;
        Ok(BoardVmLanguageStatus {
            len: len as u64,
            ..BoardVmLanguageStatus::ok()
        })
    })
}

#[no_mangle]
pub unsafe extern "C" fn board_vm_language_uart_read_module(
    max_stack: u8,
    module_out: *mut u8,
    module_cap: u64,
) -> BoardVmLanguageStatus {
    catch_status(|| {
        let module_out = unsafe { out_slice(module_out, module_cap, "module_out") }?;
        let len = build_uart_read_module(UartReadProgram { max_stack }, module_out)?;
        Ok(BoardVmLanguageStatus {
            len: len as u64,
            ..BoardVmLanguageStatus::ok()
        })
    })
}

#[no_mangle]
pub unsafe extern "C" fn board_vm_language_can_open_module(
    bus: u8,
    max_stack: u8,
    module_out: *mut u8,
    module_cap: u64,
) -> BoardVmLanguageStatus {
    catch_status(|| {
        let module_out = unsafe { out_slice(module_out, module_cap, "module_out") }?;
        let len = build_can_open_module(CanOpenProgram { bus, max_stack }, module_out)?;
        Ok(BoardVmLanguageStatus {
            len: len as u64,
            ..BoardVmLanguageStatus::ok()
        })
    })
}

#[no_mangle]
pub unsafe extern "C" fn board_vm_language_can_write_module(
    byte: u8,
    max_stack: u8,
    module_out: *mut u8,
    module_cap: u64,
) -> BoardVmLanguageStatus {
    catch_status(|| {
        let module_out = unsafe { out_slice(module_out, module_cap, "module_out") }?;
        let len = build_can_write_module(CanWriteProgram { byte, max_stack }, module_out)?;
        Ok(BoardVmLanguageStatus {
            len: len as u64,
            ..BoardVmLanguageStatus::ok()
        })
    })
}

#[no_mangle]
pub unsafe extern "C" fn board_vm_language_can_read_module(
    max_stack: u8,
    module_out: *mut u8,
    module_cap: u64,
) -> BoardVmLanguageStatus {
    catch_status(|| {
        let module_out = unsafe { out_slice(module_out, module_cap, "module_out") }?;
        let len = build_can_read_module(CanReadProgram { max_stack }, module_out)?;
        Ok(BoardVmLanguageStatus {
            len: len as u64,
            ..BoardVmLanguageStatus::ok()
        })
    })
}

#[no_mangle]
pub unsafe extern "C" fn board_vm_language_rtc_now_module(
    max_stack: u8,
    module_out: *mut u8,
    module_cap: u64,
) -> BoardVmLanguageStatus {
    catch_status(|| {
        let module_out = unsafe { out_slice(module_out, module_cap, "module_out") }?;
        let len = build_rtc_now_module(RtcNowProgram { max_stack }, module_out)?;
        Ok(BoardVmLanguageStatus {
            len: len as u64,
            ..BoardVmLanguageStatus::ok()
        })
    })
}

#[no_mangle]
pub unsafe extern "C" fn board_vm_language_rtc_set_module(
    epoch_seconds: u32,
    max_stack: u8,
    module_out: *mut u8,
    module_cap: u64,
) -> BoardVmLanguageStatus {
    catch_status(|| {
        let module_out = unsafe { out_slice(module_out, module_cap, "module_out") }?;
        let len = build_rtc_set_module(
            RtcSetProgram {
                epoch_seconds,
                max_stack,
            },
            module_out,
        )?;
        Ok(BoardVmLanguageStatus {
            len: len as u64,
            ..BoardVmLanguageStatus::ok()
        })
    })
}

#[no_mangle]
pub unsafe extern "C" fn board_vm_language_watchdog_configure_module(
    timeout_ms: u32,
    max_stack: u8,
    module_out: *mut u8,
    module_cap: u64,
) -> BoardVmLanguageStatus {
    catch_status(|| {
        let module_out = unsafe { out_slice(module_out, module_cap, "module_out") }?;
        let len = build_watchdog_configure_module(
            WatchdogConfigureProgram {
                timeout_ms,
                max_stack,
            },
            module_out,
        )?;
        Ok(BoardVmLanguageStatus {
            len: len as u64,
            ..BoardVmLanguageStatus::ok()
        })
    })
}

#[no_mangle]
pub unsafe extern "C" fn board_vm_language_watchdog_kick_module(
    max_stack: u8,
    module_out: *mut u8,
    module_cap: u64,
) -> BoardVmLanguageStatus {
    catch_status(|| {
        let module_out = unsafe { out_slice(module_out, module_cap, "module_out") }?;
        let len = build_watchdog_kick_module(WatchdogKickProgram { max_stack }, module_out)?;
        Ok(BoardVmLanguageStatus {
            len: len as u64,
            ..BoardVmLanguageStatus::ok()
        })
    })
}

#[no_mangle]
pub unsafe extern "C" fn board_vm_language_storage_write_module(
    region: u8,
    offset: u16,
    bytes: *const u8,
    bytes_len: u64,
    max_stack: u8,
    module_out: *mut u8,
    module_cap: u64,
) -> BoardVmLanguageStatus {
    catch_status(|| {
        let bytes = unsafe { in_slice(bytes, bytes_len, "bytes") }?;
        let module_out = unsafe { out_slice(module_out, module_cap, "module_out") }?;
        let len = build_storage_write_module(
            StorageWriteProgram {
                region,
                offset,
                bytes,
                max_stack,
            },
            module_out,
        )?;
        Ok(BoardVmLanguageStatus {
            len: len as u64,
            ..BoardVmLanguageStatus::ok()
        })
    })
}

#[no_mangle]
pub unsafe extern "C" fn board_vm_language_storage_read_module(
    region: u8,
    offset: u16,
    len: u8,
    max_stack: u8,
    module_out: *mut u8,
    module_cap: u64,
) -> BoardVmLanguageStatus {
    catch_status(|| {
        let module_out = unsafe { out_slice(module_out, module_cap, "module_out") }?;
        let len = build_storage_read_module(
            StorageReadProgram {
                region,
                offset,
                len,
                max_stack,
            },
            module_out,
        )?;
        Ok(BoardVmLanguageStatus {
            len: len as u64,
            ..BoardVmLanguageStatus::ok()
        })
    })
}

#[no_mangle]
pub unsafe extern "C" fn board_vm_language_storage_size_module(
    region: u8,
    max_stack: u8,
    module_out: *mut u8,
    module_cap: u64,
) -> BoardVmLanguageStatus {
    catch_status(|| {
        let module_out = unsafe { out_slice(module_out, module_cap, "module_out") }?;
        let len = build_storage_size_module(StorageSizeProgram { region, max_stack }, module_out)?;
        Ok(BoardVmLanguageStatus {
            len: len as u64,
            ..BoardVmLanguageStatus::ok()
        })
    })
}

#[no_mangle]
pub unsafe extern "C" fn board_vm_language_spi_transfer_module(
    cs_pin: u16,
    write_bytes: *const u8,
    write_bytes_len: u64,
    read_len: u8,
    max_stack: u8,
    module_out: *mut u8,
    module_cap: u64,
) -> BoardVmLanguageStatus {
    catch_status(|| {
        let write_bytes = unsafe { in_slice(write_bytes, write_bytes_len, "write_bytes") }?;
        let module_out = unsafe { out_slice(module_out, module_cap, "module_out") }?;
        let len = build_spi_transfer_module(
            SpiTransferProgram {
                cs_pin,
                write_bytes,
                read_len,
                max_stack,
            },
            module_out,
        )?;
        Ok(BoardVmLanguageStatus {
            len: len as u64,
            ..BoardVmLanguageStatus::ok()
        })
    })
}

#[no_mangle]
pub unsafe extern "C" fn board_vm_language_spi_write_module(
    cs_pin: u16,
    bytes: *const u8,
    bytes_len: u64,
    max_stack: u8,
    module_out: *mut u8,
    module_cap: u64,
) -> BoardVmLanguageStatus {
    catch_status(|| {
        let bytes = unsafe { in_slice(bytes, bytes_len, "bytes") }?;
        let module_out = unsafe { out_slice(module_out, module_cap, "module_out") }?;
        let len = build_spi_write_module(
            SpiWriteProgram {
                cs_pin,
                bytes,
                max_stack,
            },
            module_out,
        )?;
        Ok(BoardVmLanguageStatus {
            len: len as u64,
            ..BoardVmLanguageStatus::ok()
        })
    })
}

#[no_mangle]
pub unsafe extern "C" fn board_vm_language_spi_read_module(
    cs_pin: u16,
    read_len: u8,
    max_stack: u8,
    module_out: *mut u8,
    module_cap: u64,
) -> BoardVmLanguageStatus {
    catch_status(|| {
        let module_out = unsafe { out_slice(module_out, module_cap, "module_out") }?;
        let len = build_spi_read_module(
            SpiReadProgram {
                cs_pin,
                len: read_len,
                max_stack,
            },
            module_out,
        )?;
        Ok(BoardVmLanguageStatus {
            len: len as u64,
            ..BoardVmLanguageStatus::ok()
        })
    })
}

#[no_mangle]
pub unsafe extern "C" fn board_vm_language_i2c_write_u8_module(
    address: u16,
    byte: u8,
    max_stack: u8,
    module_out: *mut u8,
    module_cap: u64,
) -> BoardVmLanguageStatus {
    catch_status(|| {
        let module_out = unsafe { out_slice(module_out, module_cap, "module_out") }?;
        let len = build_i2c_write_u8_module(
            I2cWriteU8Program {
                address,
                byte,
                max_stack,
            },
            module_out,
        )?;
        Ok(BoardVmLanguageStatus {
            len: len as u64,
            ..BoardVmLanguageStatus::ok()
        })
    })
}

#[no_mangle]
pub unsafe extern "C" fn board_vm_language_i2c_write_module(
    address: u16,
    bytes: *const u8,
    bytes_len: u64,
    max_stack: u8,
    module_out: *mut u8,
    module_cap: u64,
) -> BoardVmLanguageStatus {
    catch_status(|| {
        let bytes = unsafe { in_slice(bytes, bytes_len, "bytes") }?;
        let module_out = unsafe { out_slice(module_out, module_cap, "module_out") }?;
        let len = build_i2c_write_module(
            I2cWriteProgram {
                address,
                bytes,
                max_stack,
            },
            module_out,
        )?;
        Ok(BoardVmLanguageStatus {
            len: len as u64,
            ..BoardVmLanguageStatus::ok()
        })
    })
}

#[no_mangle]
pub unsafe extern "C" fn board_vm_language_i2c_read_u8_module(
    address: u16,
    max_stack: u8,
    module_out: *mut u8,
    module_cap: u64,
) -> BoardVmLanguageStatus {
    catch_status(|| {
        let module_out = unsafe { out_slice(module_out, module_cap, "module_out") }?;
        let len = build_i2c_read_u8_module(I2cReadU8Program { address, max_stack }, module_out)?;
        Ok(BoardVmLanguageStatus {
            len: len as u64,
            ..BoardVmLanguageStatus::ok()
        })
    })
}

#[no_mangle]
pub unsafe extern "C" fn board_vm_language_i2c_read_module(
    address: u16,
    len: u8,
    max_stack: u8,
    module_out: *mut u8,
    module_cap: u64,
) -> BoardVmLanguageStatus {
    catch_status(|| {
        let module_out = unsafe { out_slice(module_out, module_cap, "module_out") }?;
        let len = build_i2c_read_module(
            I2cReadProgram {
                address,
                len,
                max_stack,
            },
            module_out,
        )?;
        Ok(BoardVmLanguageStatus {
            len: len as u64,
            ..BoardVmLanguageStatus::ok()
        })
    })
}

#[no_mangle]
pub unsafe extern "C" fn board_vm_language_i2c_transfer_module(
    address: u16,
    write_bytes: *const u8,
    write_bytes_len: u64,
    read_len: u8,
    max_stack: u8,
    module_out: *mut u8,
    module_cap: u64,
) -> BoardVmLanguageStatus {
    catch_status(|| {
        let write_bytes = unsafe { in_slice(write_bytes, write_bytes_len, "write_bytes") }?;
        let module_out = unsafe { out_slice(module_out, module_cap, "module_out") }?;
        let len = build_i2c_transfer_module(
            I2cTransferProgram {
                address,
                write_bytes,
                read_len,
                max_stack,
            },
            module_out,
        )?;
        Ok(BoardVmLanguageStatus {
            len: len as u64,
            ..BoardVmLanguageStatus::ok()
        })
    })
}

#[no_mangle]
pub unsafe extern "C" fn board_vm_language_raw_module(
    flags: u8,
    max_stack: u8,
    code: *const u8,
    code_len: u64,
    const_pool: *const u8,
    const_pool_len: u64,
    module_out: *mut u8,
    module_cap: u64,
) -> BoardVmLanguageStatus {
    catch_status(|| {
        let code = unsafe { in_slice(code, code_len, "code") }?;
        let const_pool = unsafe { in_slice(const_pool, const_pool_len, "const_pool") }?;
        let module_out = unsafe { out_slice(module_out, module_cap, "module_out") }?;
        let len = build_raw_module(flags, max_stack, code, const_pool, module_out)?;
        Ok(BoardVmLanguageStatus {
            len: len as u64,
            ..BoardVmLanguageStatus::ok()
        })
    })
}

#[no_mangle]
pub unsafe extern "C" fn board_vm_language_program_begin_wire(
    session: *mut BoardVmLanguageSession,
    program_id: u16,
    module: *const u8,
    module_len: u64,
    wire_out: *mut u8,
    wire_cap: u64,
) -> BoardVmLanguageStatus {
    catch_status(|| {
        let session = unsafe { mut_ref(session, "board_vm_language_program_begin_wire session") }?;
        let module = unsafe { in_slice(module, module_len, "module") }?;
        let wire_out = unsafe { out_slice(wire_out, wire_cap, "wire_out") }?;
        let written = build_program_begin_wire_frame(session, program_id, module, wire_out)?;
        Ok(BoardVmLanguageStatus::written(
            written.request_id,
            written.len,
        ))
    })
}

#[no_mangle]
pub unsafe extern "C" fn board_vm_language_program_chunk_wire(
    session: *mut BoardVmLanguageSession,
    program_id: u16,
    offset: u32,
    chunk: *const u8,
    chunk_len: u64,
    wire_out: *mut u8,
    wire_cap: u64,
) -> BoardVmLanguageStatus {
    catch_status(|| {
        let session = unsafe { mut_ref(session, "board_vm_language_program_chunk_wire session") }?;
        let chunk = unsafe { in_slice(chunk, chunk_len, "chunk") }?;
        let wire_out = unsafe { out_slice(wire_out, wire_cap, "wire_out") }?;
        let written = build_program_chunk_wire_frame(session, program_id, offset, chunk, wire_out)?;
        Ok(BoardVmLanguageStatus::written(
            written.request_id,
            written.len,
        ))
    })
}

#[no_mangle]
pub unsafe extern "C" fn board_vm_language_program_end_wire(
    session: *mut BoardVmLanguageSession,
    program_id: u16,
    wire_out: *mut u8,
    wire_cap: u64,
) -> BoardVmLanguageStatus {
    catch_status(|| {
        let session = unsafe { mut_ref(session, "board_vm_language_program_end_wire session") }?;
        let wire_out = unsafe { out_slice(wire_out, wire_cap, "wire_out") }?;
        let written = build_program_end_wire_frame(session, program_id, wire_out)?;
        Ok(BoardVmLanguageStatus::written(
            written.request_id,
            written.len,
        ))
    })
}

#[no_mangle]
pub unsafe extern "C" fn board_vm_language_store_program_wire(
    session: *mut BoardVmLanguageSession,
    program_id: u16,
    slot: u8,
    boot_policy: u8,
    wire_out: *mut u8,
    wire_cap: u64,
) -> BoardVmLanguageStatus {
    catch_status(|| {
        let session = unsafe { mut_ref(session, "board_vm_language_store_program_wire session") }?;
        let wire_out = unsafe { out_slice(wire_out, wire_cap, "wire_out") }?;
        let written =
            build_store_program_wire_frame(session, program_id, slot, boot_policy, wire_out)?;
        Ok(BoardVmLanguageStatus::written(
            written.request_id,
            written.len,
        ))
    })
}

#[no_mangle]
pub unsafe extern "C" fn board_vm_language_run_background_wire(
    session: *mut BoardVmLanguageSession,
    program_id: u16,
    instruction_budget: u32,
    wire_out: *mut u8,
    wire_cap: u64,
) -> BoardVmLanguageStatus {
    catch_status(|| {
        let session = unsafe { mut_ref(session, "board_vm_language_run_background_wire session") }?;
        let wire_out = unsafe { out_slice(wire_out, wire_cap, "wire_out") }?;
        let written =
            build_run_background_wire_frame(session, program_id, instruction_budget, wire_out)?;
        Ok(BoardVmLanguageStatus::written(
            written.request_id,
            written.len,
        ))
    })
}

#[no_mangle]
pub unsafe extern "C" fn board_vm_language_run_wire(
    session: *mut BoardVmLanguageSession,
    program_id: u16,
    flags: u8,
    instruction_budget: u32,
    time_budget_ms: u32,
    wire_out: *mut u8,
    wire_cap: u64,
) -> BoardVmLanguageStatus {
    catch_status(|| {
        let session = unsafe { mut_ref(session, "board_vm_language_run_wire session") }?;
        let wire_out = unsafe { out_slice(wire_out, wire_cap, "wire_out") }?;
        let written = build_run_wire_frame(
            session,
            program_id,
            flags,
            instruction_budget,
            time_budget_ms,
            wire_out,
        )?;
        Ok(BoardVmLanguageStatus::written(
            written.request_id,
            written.len,
        ))
    })
}

#[no_mangle]
pub unsafe extern "C" fn board_vm_language_stop_wire(
    session: *mut BoardVmLanguageSession,
    wire_out: *mut u8,
    wire_cap: u64,
) -> BoardVmLanguageStatus {
    catch_status(|| {
        let session = unsafe { mut_ref(session, "board_vm_language_stop_wire session") }?;
        let wire_out = unsafe { out_slice(wire_out, wire_cap, "wire_out") }?;
        let written = build_stop_wire_frame(session, wire_out)?;
        Ok(BoardVmLanguageStatus::written(
            written.request_id,
            written.len,
        ))
    })
}

#[no_mangle]
pub unsafe extern "C" fn board_vm_language_decode_wire_frame(
    wire_frame: *const u8,
    wire_frame_len: u64,
    raw_out: *mut u8,
    raw_cap: u64,
) -> BoardVmLanguageStatus {
    catch_status(|| {
        let wire_frame = unsafe { in_slice(wire_frame, wire_frame_len, "wire_frame") }?;
        let raw_out = unsafe { out_slice(raw_out, raw_cap, "raw_out") }?;
        decode_wire_frame_into_raw(wire_frame, raw_out)
    })
}

#[no_mangle]
pub unsafe extern "C" fn board_vm_language_bluetooth_transact_wire_frame(
    endpoint_ptr: *const u8,
    endpoint_len: u64,
    wire_frame: *const u8,
    wire_frame_len: u64,
    response_out: *mut u8,
    response_cap: u64,
) -> BoardVmLanguageStatus {
    catch_status(|| {
        let endpoint = unsafe { utf8_from_ptr(endpoint_ptr, endpoint_len, "endpoint") }?;
        let wire_frame = unsafe { in_slice(wire_frame, wire_frame_len, "wire_frame") }?;
        let response_out = unsafe { out_slice(response_out, response_cap, "response_out") }?;
        let len = bluetooth_transact_wire_frame(endpoint, wire_frame, response_out)?;
        Ok(BoardVmLanguageStatus {
            len: len as u64,
            ..BoardVmLanguageStatus::ok()
        })
    })
}

#[no_mangle]
pub extern "C" fn board_vm_language_default_program_id() -> u16 {
    DEFAULT_PROGRAM_ID
}

#[no_mangle]
pub extern "C" fn board_vm_language_default_instruction_budget() -> u32 {
    DEFAULT_INSTRUCTION_BUDGET
}

#[no_mangle]
pub extern "C" fn board_vm_language_default_run_flags() -> u8 {
    LANGUAGE_DEFAULT_RUN_FLAGS
}

#[no_mangle]
pub extern "C" fn board_vm_language_esp_default_baud_rate() -> u32 {
    ESP_DEFAULT_BAUD_RATE
}

#[no_mangle]
pub extern "C" fn board_vm_language_esp_default_timeout_ms() -> u64 {
    ESP_DEFAULT_TIMEOUT_MS
}

#[no_mangle]
pub extern "C" fn board_vm_language_esp_default_flash_offset() -> u32 {
    LANGUAGE_ESP_DEFAULT_FLASH_OFFSET
}

#[no_mangle]
pub extern "C" fn board_vm_language_esp_default_flash_block_size() -> u32 {
    ESP_DEFAULT_FLASH_BLOCK_SIZE
}

#[no_mangle]
pub extern "C" fn board_vm_language_esp_default_flash_size() -> u32 {
    LANGUAGE_ESP_DEFAULT_FLASH_SIZE
}

#[no_mangle]
pub extern "C" fn board_vm_language_run_flag_reset_vm_before_run() -> u8 {
    LANGUAGE_RUN_FLAG_RESET_VM_BEFORE_RUN
}

#[no_mangle]
pub extern "C" fn board_vm_language_run_flag_keep_handles_after_run() -> u8 {
    LANGUAGE_RUN_FLAG_KEEP_HANDLES_AFTER_RUN
}

#[no_mangle]
pub extern "C" fn board_vm_language_run_flag_background_run() -> u8 {
    LANGUAGE_RUN_FLAG_BACKGROUND_RUN
}

#[no_mangle]
pub extern "C" fn board_vm_language_blink_module_len() -> u64 {
    BLINK_MODULE_LEN as u64
}

#[no_mangle]
pub extern "C" fn board_vm_language_gpio_read_module_len() -> u64 {
    GPIO_READ_MODULE_LEN as u64
}

#[no_mangle]
pub extern "C" fn board_vm_language_gpio_write_module_len() -> u64 {
    GPIO_WRITE_MODULE_LEN as u64
}

#[no_mangle]
pub extern "C" fn board_vm_language_gpio_open_module_len() -> u64 {
    GPIO_OPEN_MODULE_LEN as u64
}

#[no_mangle]
pub extern "C" fn board_vm_language_gpio_handle_read_module_len() -> u64 {
    GPIO_HANDLE_READ_MODULE_LEN as u64
}

#[no_mangle]
pub extern "C" fn board_vm_language_gpio_handle_write_module_len() -> u64 {
    GPIO_HANDLE_WRITE_MODULE_LEN as u64
}

#[no_mangle]
pub extern "C" fn board_vm_language_gpio_handle_close_module_len() -> u64 {
    GPIO_HANDLE_CLOSE_MODULE_LEN as u64
}

#[no_mangle]
pub extern "C" fn board_vm_language_time_now_module_len() -> u64 {
    TIME_NOW_MODULE_LEN as u64
}

#[no_mangle]
pub extern "C" fn board_vm_language_time_sleep_ms_module_len() -> u64 {
    TIME_SLEEP_MS_MODULE_LEN as u64
}

#[no_mangle]
pub extern "C" fn board_vm_language_pwm_write_module_len() -> u64 {
    PWM_WRITE_MODULE_LEN as u64
}

#[no_mangle]
pub extern "C" fn board_vm_language_adc_read_module_len() -> u64 {
    ADC_READ_MODULE_LEN as u64
}

#[no_mangle]
pub extern "C" fn board_vm_language_dac_write_u12_module_len() -> u64 {
    DAC_WRITE_U12_MODULE_LEN as u64
}

#[no_mangle]
pub extern "C" fn board_vm_language_i2c_open_module_len() -> u64 {
    I2C_OPEN_MODULE_LEN as u64
}

#[no_mangle]
pub extern "C" fn board_vm_language_spi_open_module_len() -> u64 {
    SPI_OPEN_MODULE_LEN as u64
}

#[no_mangle]
pub extern "C" fn board_vm_language_uart_open_module_len() -> u64 {
    UART_OPEN_MODULE_LEN as u64
}

#[no_mangle]
pub extern "C" fn board_vm_language_uart_write_module_len() -> u64 {
    UART_WRITE_MODULE_LEN as u64
}

#[no_mangle]
pub extern "C" fn board_vm_language_uart_read_module_len() -> u64 {
    UART_READ_MODULE_LEN as u64
}

#[no_mangle]
pub extern "C" fn board_vm_language_can_open_module_len() -> u64 {
    CAN_OPEN_MODULE_LEN as u64
}

#[no_mangle]
pub extern "C" fn board_vm_language_can_write_module_len() -> u64 {
    CAN_WRITE_MODULE_LEN as u64
}

#[no_mangle]
pub extern "C" fn board_vm_language_can_read_module_len() -> u64 {
    CAN_READ_MODULE_LEN as u64
}

#[no_mangle]
pub extern "C" fn board_vm_language_rtc_now_module_len() -> u64 {
    RTC_NOW_MODULE_LEN as u64
}

#[no_mangle]
pub extern "C" fn board_vm_language_rtc_set_module_len() -> u64 {
    RTC_SET_MODULE_LEN as u64
}

#[no_mangle]
pub extern "C" fn board_vm_language_watchdog_configure_module_len() -> u64 {
    WATCHDOG_CONFIGURE_MODULE_LEN as u64
}

#[no_mangle]
pub extern "C" fn board_vm_language_watchdog_kick_module_len() -> u64 {
    WATCHDOG_KICK_MODULE_LEN as u64
}

#[no_mangle]
pub extern "C" fn board_vm_language_storage_write_module_len(byte_len: u64) -> u64 {
    let Ok(byte_len) = usize::try_from(byte_len) else {
        return 0;
    };
    storage_write_module_len(byte_len).unwrap_or(0) as u64
}

#[no_mangle]
pub extern "C" fn board_vm_language_storage_read_module_len() -> u64 {
    STORAGE_READ_MODULE_LEN as u64
}

#[no_mangle]
pub extern "C" fn board_vm_language_storage_size_module_len() -> u64 {
    STORAGE_SIZE_MODULE_LEN as u64
}

#[no_mangle]
pub extern "C" fn board_vm_language_spi_transfer_module_len(write_len: u64) -> u64 {
    let Ok(write_len) = usize::try_from(write_len) else {
        return 0;
    };
    spi_transfer_module_len(write_len).unwrap_or(0) as u64
}

#[no_mangle]
pub extern "C" fn board_vm_language_spi_write_module_len(byte_len: u64) -> u64 {
    let Ok(byte_len) = usize::try_from(byte_len) else {
        return 0;
    };
    spi_write_module_len(byte_len).unwrap_or(0) as u64
}

#[no_mangle]
pub extern "C" fn board_vm_language_spi_read_module_len() -> u64 {
    SPI_READ_MODULE_LEN as u64
}

#[no_mangle]
pub extern "C" fn board_vm_language_i2c_write_u8_module_len() -> u64 {
    I2C_WRITE_U8_MODULE_LEN as u64
}

#[no_mangle]
pub extern "C" fn board_vm_language_i2c_write_module_len(byte_len: u64) -> u64 {
    let Ok(byte_len) = usize::try_from(byte_len) else {
        return 0;
    };
    i2c_write_module_len(byte_len).unwrap_or(0) as u64
}

#[no_mangle]
pub extern "C" fn board_vm_language_i2c_read_u8_module_len() -> u64 {
    I2C_READ_U8_MODULE_LEN as u64
}

#[no_mangle]
pub extern "C" fn board_vm_language_i2c_read_module_len() -> u64 {
    I2C_READ_MODULE_LEN as u64
}

#[no_mangle]
pub extern "C" fn board_vm_language_i2c_transfer_module_len(write_len: u64) -> u64 {
    let Ok(write_len) = usize::try_from(write_len) else {
        return 0;
    };
    i2c_transfer_module_len(write_len).unwrap_or(0) as u64
}

#[no_mangle]
pub extern "C" fn board_vm_language_led_matrix_frame_module_len() -> u64 {
    LED_MATRIX_FRAME_MODULE_LEN as u64
}

#[no_mangle]
pub extern "C" fn board_vm_language_raw_module_len(code_len: u64, const_pool_len: u64) -> u64 {
    clear_error();
    match raw_module_len(code_len, const_pool_len) {
        Ok(len) => len as u64,
        Err(error) => {
            let code = status_code_for_error(&error);
            set_error(code, error_message(&error));
            0
        }
    }
}

fn catch_status(
    operation: impl FnOnce() -> Result<BoardVmLanguageStatus, LanguageCoreError>,
) -> BoardVmLanguageStatus {
    clear_error();
    match panic::catch_unwind(AssertUnwindSafe(operation)) {
        Ok(Ok(status)) => status,
        Ok(Err(error)) => {
            let code = status_code_for_error(&error);
            set_error(code, error_message(&error));
            BoardVmLanguageStatus::err(code)
        }
        Err(_) => {
            set_error(
                BoardVmLanguageStatusCode::Panic,
                "board-vm-language-core caught a Rust panic before it crossed the C ABI boundary.",
            );
            BoardVmLanguageStatus::err(BoardVmLanguageStatusCode::Panic)
        }
    }
}

fn status_code_for_error(error: &LanguageCoreError) -> BoardVmLanguageStatusCode {
    match error {
        LanguageCoreError::NullPointer(_) => BoardVmLanguageStatusCode::NullPointer,
        LanguageCoreError::InvalidUtf8 => BoardVmLanguageStatusCode::InvalidUtf8,
        LanguageCoreError::ValueTooLarge => BoardVmLanguageStatusCode::ValueTooLarge,
        LanguageCoreError::OutputTooSmall => BoardVmLanguageStatusCode::OutputTooSmall,
        LanguageCoreError::Protocol(_) => BoardVmLanguageStatusCode::ProtocolError,
        LanguageCoreError::Host(_) => BoardVmLanguageStatusCode::HostError,
        LanguageCoreError::BluetoothEndpoint => BoardVmLanguageStatusCode::BluetoothEndpointError,
        LanguageCoreError::BluetoothOpen => BoardVmLanguageStatusCode::BluetoothOpenError,
        LanguageCoreError::Transport => BoardVmLanguageStatusCode::TransportError,
    }
}

fn error_message(error: &LanguageCoreError) -> String {
    match error {
        LanguageCoreError::NullPointer(name) => format!("{name} must not be null."),
        other => format!("{other:?}"),
    }
}

fn clear_error() {
    LAST_ERROR_CODE.with(|slot| slot.set(BoardVmLanguageStatusCode::Ok as u32));
    LAST_ERROR_MESSAGE.with(|slot| *slot.borrow_mut() = None);
}

fn set_error(code: BoardVmLanguageStatusCode, message: impl AsRef<str>) {
    LAST_ERROR_CODE.with(|slot| slot.set(code as u32));
    LAST_ERROR_MESSAGE.with(|slot| *slot.borrow_mut() = Some(sanitize_message(message.as_ref())));
}

fn checked_module_len(code_len: usize, const_pool_len: usize) -> Result<usize, LanguageCoreError> {
    let code_len_len = uleb128_len(code_len)?;
    let const_pool_len_len = uleb128_len(const_pool_len)?;
    8usize
        .checked_add(code_len_len)
        .and_then(|len| len.checked_add(code_len))
        .and_then(|len| len.checked_add(const_pool_len_len))
        .and_then(|len| len.checked_add(const_pool_len))
        .ok_or(LanguageCoreError::ValueTooLarge)
}

fn uleb128_len(value: usize) -> Result<usize, LanguageCoreError> {
    let mut value = u32::try_from(value).map_err(|_| LanguageCoreError::ValueTooLarge)?;
    let mut len = 1usize;
    while value >= 0x80 {
        value >>= 7;
        len += 1;
    }
    Ok(len)
}

fn sanitize_message(message: &str) -> CString {
    match CString::new(message) {
        Ok(message) => message,
        Err(_) => CString::new(message.replace('\0', " "))
            .expect("nul-stripped error message must be a valid CString"),
    }
}

unsafe fn ref_from_ptr<'a, T>(
    ptr: *const T,
    name: &'static str,
) -> Result<&'a T, LanguageCoreError> {
    unsafe { ptr.as_ref() }.ok_or_else(|| {
        set_error(
            BoardVmLanguageStatusCode::NullPointer,
            format!("{name} must not be null."),
        );
        LanguageCoreError::NullPointer(name)
    })
}

unsafe fn mut_ref<'a, T>(ptr: *mut T, name: &'static str) -> Result<&'a mut T, LanguageCoreError> {
    unsafe { ptr.as_mut() }.ok_or_else(|| {
        set_error(
            BoardVmLanguageStatusCode::NullPointer,
            format!("{name} must not be null."),
        );
        LanguageCoreError::NullPointer(name)
    })
}

unsafe fn in_slice<'a>(
    ptr: *const u8,
    len: u64,
    name: &'static str,
) -> Result<&'a [u8], LanguageCoreError> {
    if len == 0 {
        return Ok(&[]);
    }
    if ptr.is_null() {
        set_error(
            BoardVmLanguageStatusCode::NullPointer,
            format!("{name} must not be null when len is non-zero."),
        );
        return Err(LanguageCoreError::NullPointer(name));
    }
    let len = usize::try_from(len).map_err(|_| LanguageCoreError::ValueTooLarge)?;
    Ok(unsafe { slice::from_raw_parts(ptr, len) })
}

unsafe fn out_slice<'a>(
    ptr: *mut u8,
    len: u64,
    name: &'static str,
) -> Result<&'a mut [u8], LanguageCoreError> {
    if ptr.is_null() {
        set_error(
            BoardVmLanguageStatusCode::NullPointer,
            format!("{name} must not be null."),
        );
        return Err(LanguageCoreError::NullPointer(name));
    }
    let len = usize::try_from(len).map_err(|_| LanguageCoreError::ValueTooLarge)?;
    Ok(unsafe { slice::from_raw_parts_mut(ptr, len) })
}

unsafe fn utf8_from_ptr<'a>(
    ptr: *const u8,
    len: u64,
    name: &'static str,
) -> Result<&'a str, LanguageCoreError> {
    let bytes = unsafe { in_slice(ptr, len, name) }?;
    str::from_utf8(bytes).map_err(|_| LanguageCoreError::InvalidUtf8)
}

#[cfg(test)]
mod tests {
    use super::*;
    use board_vm_bluetooth::{
        BleGattEndpoint, BleGattIo, BluetoothOpenError, BluetoothTransportError, RfcommEndpoint,
    };
    use board_vm_loopback::{LoopbackBoard, LOOPBACK_BOARD_ID, LOOPBACK_RUNTIME_ID};
    use board_vm_protocol::{
        decode_program_begin, decode_program_chunk, decode_program_end, decode_run_request,
        encode_caps_report, encode_frame, encode_hello_ack, encode_value, encode_wire_frame,
        CapabilityDescriptor, CapsReportHeader, Frame, HelloAck, MessageType, RunReportHeader,
        RunStatus, Value, FLAG_IS_RESPONSE, GOLDEN_HELLO_WIRE_FRAME_BVM_V1,
        RUN_FLAG_BACKGROUND_RUN, RUN_FLAG_KEEP_HANDLES_AFTER_RUN, RUN_FLAG_RESET_VM_BEFORE_RUN,
    };
    use std::cell::RefCell;
    use std::io::{Read, Write};
    use std::rc::Rc;

    #[test]
    fn hello_wire_frame_matches_protocol_golden_vector() {
        let mut session = BoardVmLanguageSession::with_next_request_id(0x1234);
        let mut wire = [0u8; 64];

        let written = build_hello_wire_frame(&mut session, "bvm", 0x1234_ABCD, &mut wire).unwrap();

        assert_eq!(written.request_id, 0x1234);
        assert_eq!(written.len, GOLDEN_HELLO_WIRE_FRAME_BVM_V1.len());
        assert_eq!(&wire[..written.len], GOLDEN_HELLO_WIRE_FRAME_BVM_V1);
        assert_eq!(session.next_request_id(), 0x1235);
    }

    #[test]
    fn known_targets_are_owned_by_rust_language_core() {
        let targets = known_targets();
        let esp32 = known_target("esp32-devkit-v1").unwrap();
        let uno = known_target("arduino-uno-r4-wifi").unwrap();
        let uno_r3 = known_target("arduino-uno-r3").unwrap();
        let mega = known_target("arduino-mega-2560").unwrap();
        let leonardo = known_target("arduino-leonardo").unwrap();
        let micro = known_target("arduino-micro").unwrap();
        let nano_every = known_target("arduino-nano-every").unwrap();
        let nano_r4 = known_target("arduino-nano-r4").unwrap();
        let nano_iot = known_target("arduino-nano-33-iot").unwrap();
        let nano_ble = known_target("arduino-nano-33-ble-rev2").unwrap();
        let nano_esp32 = known_target("arduino-nano-esp32").unwrap();
        let giga = known_target("arduino-giga-r1-wifi").unwrap();
        let portenta_c33 = known_target("arduino-portenta-c33").unwrap();
        let nicla_vision = known_target("arduino-nicla-vision").unwrap();
        let opta_wifi = known_target("arduino-opta-wifi").unwrap();
        let pico_w = known_target("raspberry-pi-pico-w").unwrap();

        assert!(targets
            .iter()
            .any(|target| target.board_id == esp32.board_id));
        assert!(targets
            .iter()
            .any(|target| target.board_id == mega.board_id));
        assert_eq!(esp32.family, LanguageBoardFamily::Esp32);
        assert_eq!(board_family_name(esp32.family), "esp32");
        assert_eq!(esp32.runtime_id, "board-vm-esp32");
        assert_eq!(esp32.onboard_led, Some(LanguageOnboardLed::Gpio(2)));
        assert_eq!(uno.rtc.as_ref().unwrap().name, "RTC");
        assert_eq!(uno.rtc.as_ref().unwrap().peripheral, "RA4M1 RTC");
        assert!(uno.rtc.as_ref().unwrap().notes.contains("real-time clock"));
        assert!(uno.capabilities.contains(&"rtc.now".to_owned()));
        assert!(uno.capabilities.contains(&"rtc.set".to_owned()));
        assert_eq!(mega.family, LanguageBoardFamily::Arduino);
        assert_eq!(board_family_name(mega.family), "arduino");
        assert_eq!(mega.runtime_id, "board-vm-arduino");
        assert_eq!(mega.digital_pin_count, 70);
        assert!(mega.capabilities.contains(&"transport.serial".to_owned()));
        assert!(mega.capabilities.contains(&"gpio.open".to_owned()));
        assert!(mega.capabilities.contains(&"program.ram_exec".to_owned()));
        assert_eq!(uno_r3.i2c_buses.len(), 1);
        assert_eq!(uno_r3.i2c_buses[0].name, "Wire");
        assert_eq!(uno_r3.i2c_buses[0].sda_pin, 18);
        assert_eq!(uno_r3.i2c_buses[0].scl_pin, 19);
        assert_eq!(uno_r3.spi_buses.len(), 1);
        assert_eq!(uno_r3.spi_buses[0].copi_pin, 11);
        assert_eq!(uno_r3.spi_buses[0].cipo_pin, 12);
        assert_eq!(uno_r3.uart_buses.len(), 1);
        assert_eq!(uno_r3.uart_buses[0].name, "Serial");
        assert_eq!(uno_r3.uart_buses[0].tx_pin, 1);
        assert_eq!(uno_r3.uart_buses[0].rx_pin, 0);
        assert_eq!(mega.i2c_buses.len(), 1);
        assert_eq!(mega.i2c_buses[0].sda_pin, 20);
        assert_eq!(mega.i2c_buses[0].scl_pin, 21);
        assert_eq!(mega.spi_buses.len(), 1);
        assert_eq!(mega.spi_buses[0].copi_pin, 51);
        assert_eq!(mega.spi_buses[0].cipo_pin, 50);
        assert_eq!(mega.spi_buses[0].sck_pin, 52);
        assert_eq!(mega.uart_buses.len(), 4);
        assert_eq!(mega.uart_buses[3].name, "Serial3");
        assert_eq!(mega.uart_buses[3].tx_pin, 14);
        assert_eq!(mega.uart_buses[3].rx_pin, 15);
        assert!(mega.i2c_buses[0].notes.contains("metadata only"));
        assert!(!mega.capabilities.contains(&"i2c.open".to_owned()));
        assert!(!mega.capabilities.contains(&"spi.open".to_owned()));
        assert!(!mega.capabilities.contains(&"uart.open".to_owned()));
        assert_eq!(leonardo.i2c_buses.len(), 1);
        assert_eq!(leonardo.i2c_buses[0].sda_pin, 2);
        assert_eq!(leonardo.i2c_buses[0].scl_pin, 3);
        assert_eq!(leonardo.spi_buses.len(), 1);
        assert_eq!(leonardo.spi_buses[0].copi_pin, 16);
        assert_eq!(leonardo.spi_buses[0].cipo_pin, 14);
        assert_eq!(leonardo.spi_buses[0].sck_pin, 15);
        assert_eq!(leonardo.spi_buses[0].default_cs_pin, 17);
        assert_eq!(leonardo.uart_buses.len(), 1);
        assert_eq!(leonardo.uart_buses[0].name, "Serial1");
        assert_eq!(leonardo.uart_buses[0].arduino_uart, 1);
        assert!(leonardo.uart_buses[0].notes.contains("native USB Serial"));
        assert_eq!(micro.i2c_buses, leonardo.i2c_buses);
        assert_eq!(micro.spi_buses, leonardo.spi_buses);
        assert_eq!(micro.uart_buses, leonardo.uart_buses);
        assert!(!leonardo.capabilities.contains(&"i2c.open".to_owned()));
        assert!(!leonardo.capabilities.contains(&"spi.open".to_owned()));
        assert!(!leonardo.capabilities.contains(&"uart.open".to_owned()));
        assert_eq!(nano_every.i2c_buses.len(), 1);
        assert_eq!(nano_every.i2c_buses[0].sda_pin, 18);
        assert_eq!(nano_every.i2c_buses[0].scl_pin, 19);
        assert_eq!(nano_every.spi_buses.len(), 1);
        assert_eq!(nano_every.spi_buses[0].copi_pin, 11);
        assert_eq!(nano_every.spi_buses[0].cipo_pin, 12);
        assert_eq!(nano_every.spi_buses[0].sck_pin, 13);
        assert_eq!(nano_every.spi_buses[0].default_cs_pin, 10);
        assert_eq!(nano_every.uart_buses.len(), 1);
        assert_eq!(nano_every.uart_buses[0].name, "Serial1");
        assert_eq!(nano_every.uart_buses[0].tx_pin, 1);
        assert_eq!(nano_every.uart_buses[0].rx_pin, 0);
        assert!(nano_every.uart_buses[0].notes.contains("USB serial bridge"));
        assert!(!nano_every.capabilities.contains(&"i2c.open".to_owned()));
        assert!(!nano_every.capabilities.contains(&"spi.open".to_owned()));
        assert!(!nano_every.capabilities.contains(&"uart.open".to_owned()));
        assert_eq!(nano_r4.family, LanguageBoardFamily::Arduino);
        assert_eq!(nano_r4.mcu, "RA4M1");
        assert_eq!(nano_r4.i2c_buses.len(), 1);
        assert_eq!(nano_r4.i2c_buses[0].sda_pin, 18);
        assert_eq!(nano_r4.i2c_buses[0].scl_pin, 19);
        assert!(nano_r4.i2c_buses[0].notes.contains("Qwiic Wire1"));
        assert_eq!(nano_r4.i2c_connectors.len(), 1);
        assert_eq!(nano_r4.i2c_connectors[0].name, "Wire1");
        assert_eq!(nano_r4.i2c_connectors[0].connector, "Qwiic/STEMMA QT");
        assert_eq!(nano_r4.i2c_connectors[0].arduino_object, "Wire1");
        assert_eq!(nano_r4.i2c_connectors[0].controller, "RA4M1 IIC0");
        assert!(nano_r4.i2c_connectors[0].notes.contains("connector-local"));
        assert_eq!(nano_r4.spi_buses.len(), 1);
        assert_eq!(nano_r4.spi_buses[0].copi_pin, 11);
        assert_eq!(nano_r4.spi_buses[0].cipo_pin, 12);
        assert_eq!(nano_r4.spi_buses[0].sck_pin, 13);
        assert_eq!(nano_r4.spi_buses[0].default_cs_pin, 10);
        assert_eq!(nano_r4.uart_buses.len(), 1);
        assert_eq!(nano_r4.uart_buses[0].name, "Serial1");
        assert_eq!(nano_r4.uart_buses[0].tx_pin, 1);
        assert_eq!(nano_r4.uart_buses[0].rx_pin, 0);
        assert!(nano_r4.uart_buses[0].notes.contains("native USB"));
        assert_eq!(nano_r4.usb_interfaces.len(), 1);
        assert_eq!(nano_r4.usb_interfaces[0].name, "Native USB");
        assert_eq!(nano_r4.usb_interfaces[0].controller, "RA4M1 native USB");
        assert_eq!(nano_r4.usb_interfaces[0].class, "CDC serial");
        assert!(nano_r4.usb_interfaces[0].native);
        assert!(nano_r4.usb_interfaces[0].upload);
        assert!(nano_r4.usb_interfaces[0].command_transport);
        assert!(nano_r4.usb_interfaces[0].notes.contains("not a GPIO UART"));
        assert_eq!(nano_r4.can_buses.len(), 1);
        assert_eq!(nano_r4.can_buses[0].name, "CAN0");
        assert_eq!(nano_r4.can_buses[0].tx_pin, 4);
        assert_eq!(nano_r4.can_buses[0].rx_pin, 5);
        assert_eq!(nano_r4.can_buses[0].controller, "RA4M1 CAN0");
        assert!(nano_r4.can_buses[0].notes.contains("external transceiver"));
        assert_eq!(nano_r4.rtc.as_ref().unwrap().name, "RTC");
        assert_eq!(nano_r4.rtc.as_ref().unwrap().peripheral, "RA4M1 RTC");
        assert!(nano_r4
            .rtc
            .as_ref()
            .unwrap()
            .notes
            .contains("metadata only"));
        assert!(!nano_r4.capabilities.contains(&"i2c.open".to_owned()));
        assert!(!nano_r4.capabilities.contains(&"spi.open".to_owned()));
        assert!(!nano_r4.capabilities.contains(&"uart.open".to_owned()));
        assert!(!nano_r4.capabilities.contains(&"can.open".to_owned()));
        assert!(!nano_r4.capabilities.contains(&"can.write".to_owned()));
        assert!(!nano_r4.capabilities.contains(&"can.read".to_owned()));
        assert!(!nano_r4.capabilities.contains(&"rtc.now".to_owned()));
        assert!(!nano_r4.capabilities.contains(&"rtc.set".to_owned()));
        assert_eq!(nano_iot.family, LanguageBoardFamily::Arduino);
        assert_eq!(nano_iot.mcu, "SAMD21G18");
        assert_eq!(nano_ble.family, LanguageBoardFamily::Arduino);
        assert_eq!(nano_ble.mcu, "nRF52840");
        assert_eq!(nano_esp32.family, LanguageBoardFamily::Arduino);
        assert_eq!(nano_esp32.mcu, "ESP32-S3");
        assert_eq!(giga.family, LanguageBoardFamily::Arduino);
        assert_eq!(giga.mcu, "STM32H747XI");
        assert_eq!(portenta_c33.family, LanguageBoardFamily::Arduino);
        assert_eq!(portenta_c33.mcu, "R7FA6M5BH2CBG");
        assert_eq!(portenta_c33.rust_target, "thumbv8m.main-none-eabihf");
        assert_eq!(portenta_c33.digital_pin_count, 7);
        assert_eq!(nicla_vision.family, LanguageBoardFamily::Arduino);
        assert_eq!(nicla_vision.mcu, "STM32H747AII6");
        assert_eq!(opta_wifi.family, LanguageBoardFamily::Arduino);
        assert_eq!(opta_wifi.mcu, "STM32H747XI");
        assert_eq!(opta_wifi.digital_pin_count, 12);
        assert_eq!(opta_wifi.digital_pins[0].label, "I1");
        assert!(!opta_wifi.digital_pins[0].supports_output);
        assert_eq!(opta_wifi.digital_pins[11].label, "O4");
        assert!(opta_wifi.digital_pins[11].supports_output);
        assert_eq!(opta_wifi.upload.as_ref().unwrap().adapter, "arduino_cli");
        assert!(nano_iot.wireless.iter().any(|interface| interface.transport
            == LanguageWirelessTransport::Wifi
            && interface.chip == "u-blox NINA-W102"
            && !interface.command_transport
            && !interface.ota_update));
        assert!(nano_iot.wireless.iter().any(|interface| interface.transport
            == LanguageWirelessTransport::BluetoothLe
            && interface.chip == "u-blox NINA-W102"
            && !interface.command_transport
            && !interface.ota_update));
        assert!(!nano_iot.capabilities.contains(&"transport.wifi".to_owned()));
        assert_eq!(nano_iot.network_interfaces.len(), 1);
        let nano_iot_network = &nano_iot.network_interfaces[0];
        assert_eq!(nano_iot_network.name, "WiFiNINA");
        assert_eq!(nano_iot_network.transport, LanguageWirelessTransport::Wifi);
        assert_eq!(nano_iot_network.chip, "u-blox NINA-W102");
        assert_eq!(
            nano_iot_network.protocols,
            [
                LanguageNetworkProtocol::Ipv4,
                LanguageNetworkProtocol::Tcp,
                LanguageNetworkProtocol::Udp,
                LanguageNetworkProtocol::Dns,
            ]
        );
        assert_eq!(nano_iot_network.max_sockets, 0);
        assert!(nano_iot_network.notes.contains("metadata only"));
        assert!(!nano_iot.capabilities.contains(&"network.ipv4".to_owned()));
        assert!(nano_ble.wireless.iter().any(|interface| interface.transport
            == LanguageWirelessTransport::BluetoothLe
            && interface.chip == "nRF52840"
            && !interface.command_transport
            && !interface.ota_update));
        assert!(!nano_ble
            .capabilities
            .contains(&"transport.bluetooth_le".to_owned()));
        assert!(nano_esp32
            .wireless
            .iter()
            .any(
                |interface| interface.transport == LanguageWirelessTransport::Wifi
                    && interface.chip == "ESP32-S3"
                    && !interface.command_transport
                    && !interface.ota_update
            ));
        assert!(!nano_esp32
            .capabilities
            .contains(&"transport.wifi".to_owned()));
        assert!(giga.wireless.iter().any(|interface| interface.transport
            == LanguageWirelessTransport::Wifi
            && interface.chip == "Arduino onboard WiFi/BLE module"
            && !interface.command_transport
            && !interface.ota_update));
        assert_eq!(giga.network_interfaces.len(), 1);
        assert_eq!(giga.network_interfaces[0].name, "Onboard WiFi");
        assert_eq!(giga.network_interfaces[0].max_sockets, 0);
        assert!(portenta_c33
            .wireless
            .iter()
            .any(
                |interface| interface.transport == LanguageWirelessTransport::Wifi
                    && interface.chip == "ESP32-C3 module"
                    && !interface.command_transport
                    && !interface.ota_update
            ));
        assert_eq!(portenta_c33.network_interfaces.len(), 1);
        assert_eq!(portenta_c33.network_interfaces[0].name, "ESP32-C3 WiFi");
        assert_eq!(portenta_c33.network_interfaces[0].max_sockets, 0);
        assert!(nicla_vision
            .wireless
            .iter()
            .any(
                |interface| interface.transport == LanguageWirelessTransport::BluetoothLe
                    && interface.chip == "Arduino onboard WiFi/BLE module"
                    && !interface.command_transport
                    && !interface.ota_update
            ));
        assert_eq!(nicla_vision.network_interfaces.len(), 1);
        assert_eq!(nicla_vision.network_interfaces[0].name, "Onboard WiFi");
        assert_eq!(nicla_vision.network_interfaces[0].max_sockets, 0);
        assert!(opta_wifi
            .wireless
            .iter()
            .any(
                |interface| interface.transport == LanguageWirelessTransport::Wifi
                    && interface.chip == "Arduino onboard WiFi/BLE module"
                    && !interface.command_transport
                    && !interface.ota_update
            ));
        assert_eq!(opta_wifi.network_interfaces.len(), 1);
        assert_eq!(opta_wifi.network_interfaces[0].name, "Onboard WiFi");
        assert_eq!(opta_wifi.network_interfaces[0].max_sockets, 0);
        assert!(!opta_wifi.capabilities.contains(&"network.ipv4".to_owned()));
        assert_eq!(
            uno.led_matrix,
            Some(LanguageLedMatrix {
                rows: 8,
                columns: 12
            })
        );
        assert_eq!(uno.digital_pin_count, uno.digital_pins.len());
        let uno_d3 = uno.digital_pins.iter().find(|pin| pin.pin == 3).unwrap();
        assert_eq!(uno_d3.label, "D3");
        assert!(uno_d3.supports_pwm);
        assert!(uno_d3.supports_interrupt);
        assert!(!uno_d3.supports_adc);
        let uno_a0 = uno.digital_pins.iter().find(|pin| pin.pin == 14).unwrap();
        assert_eq!(uno_a0.label, "A0/D14");
        assert!(uno_a0.supports_adc);
        assert!(uno_a0.supports_dac);
        assert_eq!(uno.i2c_buses.len(), 2);
        assert_eq!(uno.i2c_buses[0].name, "Wire");
        assert_eq!(uno.i2c_buses[0].sda_pin, 18);
        assert_eq!(uno.i2c_buses[0].scl_pin, 19);
        assert_eq!(uno.i2c_buses[1].name, "Wire1");
        assert!(uno.i2c_buses[1].qwiic);
        assert_eq!(uno.spi_buses.len(), 1);
        assert_eq!(uno.spi_buses[0].name, "SPI");
        assert_eq!(uno.spi_buses[0].copi_pin, 11);
        assert_eq!(uno.spi_buses[0].cipo_pin, 12);
        assert_eq!(uno.spi_buses[0].sck_pin, 13);
        assert_eq!(uno.spi_buses[0].default_cs_pin, 10);
        assert_eq!(uno.uart_buses.len(), 3);
        assert_eq!(uno.uart_buses[0].name, "Serial1");
        assert_eq!(uno.uart_buses[0].tx_pin, 22);
        assert_eq!(uno.uart_buses[0].rx_pin, 23);
        assert_eq!(uno.uart_buses[0].arduino_uart, 1);
        assert!(!uno.uart_buses[0].internal);
        assert_eq!(uno.uart_buses[1].name, "Serial2");
        assert_eq!(uno.uart_buses[1].tx_pin, 1);
        assert_eq!(uno.uart_buses[1].rx_pin, 0);
        assert_eq!(uno.uart_buses[2].name, "Serial3");
        assert_eq!(uno.uart_buses[2].tx_pin, 24);
        assert_eq!(uno.uart_buses[2].rx_pin, 25);
        assert!(uno.uart_buses[2].internal);
        assert!(uno.capabilities.contains(&"pwm.write".to_owned()));
        assert!(uno.capabilities.contains(&"adc.read".to_owned()));
        assert!(uno.capabilities.contains(&"dac.write_u12".to_owned()));
        assert!(uno.capabilities.contains(&"i2c.open".to_owned()));
        assert!(uno.capabilities.contains(&"i2c.write_u8".to_owned()));
        assert!(uno.capabilities.contains(&"i2c.read_u8".to_owned()));
        assert!(uno.capabilities.contains(&"i2c.write".to_owned()));
        assert!(uno.capabilities.contains(&"i2c.read".to_owned()));
        assert!(uno.capabilities.contains(&"i2c.transfer".to_owned()));
        assert!(uno.capabilities.contains(&"spi.open".to_owned()));
        assert!(uno.capabilities.contains(&"spi.transfer".to_owned()));
        assert!(uno.capabilities.contains(&"uart.open".to_owned()));
        assert!(uno.capabilities.contains(&"uart.write".to_owned()));
        assert!(uno.capabilities.contains(&"uart.read".to_owned()));
        assert!(uno.capabilities.contains(&"rtc.now".to_owned()));
        assert!(uno.capabilities.contains(&"rtc.set".to_owned()));
        assert!(uno.capabilities.contains(&"watchdog.configure".to_owned()));
        assert!(uno.capabilities.contains(&"watchdog.kick".to_owned()));
        assert!(uno.capabilities.contains(&"storage.write".to_owned()));
        assert!(uno.capabilities.contains(&"storage.read".to_owned()));
        assert!(uno.capabilities.contains(&"storage.size".to_owned()));
        assert!(uno.capabilities.contains(&"program.store".to_owned()));
        assert!(uno.capabilities.contains(&"network.ipv4".to_owned()));
        assert!(uno.capabilities.contains(&"network.tcp".to_owned()));
        assert!(uno.capabilities.contains(&"network.udp".to_owned()));
        assert!(uno.capabilities.contains(&"network.dns".to_owned()));
        assert!(uno.capabilities.contains(&"network.tcp.open".to_owned()));
        assert!(uno.capabilities.contains(&"network.tcp.write".to_owned()));
        assert!(uno.capabilities.contains(&"network.tcp.read".to_owned()));
        assert!(uno.capabilities.contains(&"network.tcp.close".to_owned()));
        assert!(uno
            .capabilities
            .contains(&"network.tcp.connected".to_owned()));
        assert!(uno
            .capabilities
            .contains(&"network.tcp.available".to_owned()));
        assert!(uno.capabilities.contains(&"network.udp.open".to_owned()));
        assert!(uno.capabilities.contains(&"network.udp.write".to_owned()));
        assert!(uno.capabilities.contains(&"network.udp.read".to_owned()));
        assert!(uno
            .capabilities
            .contains(&"network.udp.write_bytes".to_owned()));
        assert!(uno
            .capabilities
            .contains(&"network.udp.read_bytes".to_owned()));
        assert!(uno
            .capabilities
            .contains(&"network.udp.available".to_owned()));
        assert!(uno.capabilities.contains(&"network.udp.close".to_owned()));
        assert!(uno
            .capabilities
            .contains(&"network.wifi.associate".to_owned()));
        assert!(uno
            .capabilities
            .contains(&"network.wifi.disconnect".to_owned()));
        assert!(uno.capabilities.contains(&"network.wifi.status".to_owned()));
        assert!(uno.capabilities.contains(&"network.dns.resolve".to_owned()));
        assert!(uno
            .capabilities
            .contains(&"network.dns.set_server".to_owned()));
        assert!(uno.capabilities.contains(&"network.dns.query".to_owned()));
        assert!(uno
            .capabilities
            .contains(&"network.dns.response_ipv4".to_owned()));
        assert!(uno
            .capabilities
            .contains(&"network.dns.exchange_udp".to_owned()));
        assert!(uno
            .capabilities
            .contains(&"network.dns.exchange_udp_retry".to_owned()));
        assert!(uno
            .capabilities
            .contains(&"network.dns.exchange_udp_fallback".to_owned()));
        assert_eq!(uno.network_interfaces.len(), 1);
        let uno_network = &uno.network_interfaces[0];
        assert_eq!(uno_network.interface, 0);
        assert_eq!(uno_network.name, "WiFiS3");
        assert_eq!(uno_network.transport, LanguageWirelessTransport::Wifi);
        assert_eq!(uno_network.chip, "ESP32-S3 coprocessor");
        assert_eq!(
            uno_network.protocols,
            [
                LanguageNetworkProtocol::Ipv4,
                LanguageNetworkProtocol::Tcp,
                LanguageNetworkProtocol::Udp,
                LanguageNetworkProtocol::Dns,
            ]
        );
        assert_eq!(uno_network.max_sockets, 4);
        assert!(uno_network.notes.contains("TCP endpoints"));
        assert_eq!(
            known_target("arduino-uno-r4-minima").unwrap().led_matrix,
            None
        );
        assert!(known_target("arduino-uno-r4-minima")
            .unwrap()
            .network_interfaces
            .is_empty());
        assert!(esp32.capabilities.contains(&"gpio.open".to_owned()));
        assert!(esp32
            .capabilities
            .contains(&"transport.bluetooth_classic".to_owned()));
        assert!(esp32.wireless.iter().any(|interface| interface.transport
            == LanguageWirelessTransport::Wifi
            && interface.command_transport
            && interface.ota_update));
        assert!(esp32
            .connection_options
            .iter()
            .any(
                |option| option.transport == LanguageConnectionTransport::BluetoothClassic
                    && option.command_transport
                    && !option.ota_update
                    && option.requires == "paired_device"
            ));
        assert_eq!(
            pico_w.onboard_led,
            Some(LanguageOnboardLed::WirelessChipGpio(0))
        );
        assert!(pico_w.capabilities.contains(&"transport.wifi".to_owned()));
        assert!(known_target("raspberry-pi-pico")
            .unwrap()
            .wireless
            .is_empty());
    }

    #[test]
    fn connection_options_are_owned_by_rust_language_core() {
        let uno = connection_options_for_target("uno-r4-wifi").unwrap();
        let pico = connection_options_for_target("pico").unwrap();

        assert!(uno.iter().any(
            |option| option.transport == LanguageConnectionTransport::Serial
                && option.display_name == "USB/serial"
                && option.command_transport
                && !option.ota_update
                && option.requires == "serial_port"
                && option.endpoint_transport == LanguageHostEndpointTransport::SerialPort
                && option.endpoint_scheme == "serial"
                && option.wire_protocol == LANGUAGE_BOARD_VM_WIRE_PROTOCOL
        ));
        assert!(uno.iter().any(
            |option| option.transport == LanguageConnectionTransport::Wifi
                && option.command_transport
                && option.ota_update
                && option.requires == "network_endpoint"
                && option.endpoint_transport == LanguageHostEndpointTransport::TcpSocket
                && option.endpoint_scheme == "tcp"
        ));
        assert!(uno.iter().any(|option| option.transport
            == LanguageConnectionTransport::BluetoothLe
            && option.command_transport
            && !option.ota_update
            && option.requires == "paired_device"
            && option.endpoint_transport == LanguageHostEndpointTransport::BluetoothLeGatt
            && option.endpoint_scheme == "ble"));
        assert_eq!(
            pico.iter()
                .map(|option| option.transport)
                .collect::<Vec<_>>(),
            vec![LanguageConnectionTransport::Serial]
        );
        assert_eq!(
            connection_transport_name(LanguageConnectionTransport::BluetoothLe),
            "bluetooth_le"
        );
        assert_eq!(
            host_endpoint_transport_name(LanguageHostEndpointTransport::BluetoothClassicRfcomm),
            "bluetooth_classic_rfcomm"
        );
        assert!(connection_options_for_target("not-a-board").is_none());
    }

    #[test]
    fn serial_runtime_open_plans_are_owned_by_rust_language_core() {
        let plan =
            serial_runtime_open_plan_for_target("uno-r4-wifi", "/dev/cu.usbmodem1101").unwrap();
        assert_eq!(plan.board_id, "arduino-uno-r4-wifi");
        assert_eq!(plan.port, "/dev/cu.usbmodem1101");
        assert_eq!(plan.port_source, "explicit_runtime_port");
        assert_eq!(plan.endpoint, "serial:///dev/cu.usbmodem1101");
        assert_eq!(plan.transport, LanguageConnectionTransport::Serial);
        assert_eq!(
            plan.endpoint_transport,
            LanguageHostEndpointTransport::SerialPort
        );
        assert_eq!(plan.endpoint_scheme, "serial");
        assert_eq!(plan.wire_protocol, LANGUAGE_BOARD_VM_WIRE_PROTOCOL);
        assert_eq!(plan.baud_rate, LANGUAGE_SERIAL_DEFAULT_BAUD_RATE);
        assert_eq!(plan.timeout_ms, LANGUAGE_SERIAL_DEFAULT_TIMEOUT_MS);
        assert_eq!(plan.data_bits, 8);
        assert_eq!(plan.parity, "none");
        assert_eq!(plan.stop_bits, 1);
        assert_eq!(plan.flow_control, "none");
        assert!(plan.dtr_on_open);
        assert!(plan.clear_on_open);
        assert_eq!(plan.settle_on_open_ms, LANGUAGE_SERIAL_OPEN_SETTLE_MS);
        assert!(plan.hello_after_open);
        assert_eq!(plan.upload_port_hint.as_deref(), Some("native_usb"));
        assert!(plan.notes.contains("runtime CDC serial port"));

        let endpoint = parse_serial_endpoint(&plan.endpoint).unwrap();
        assert_eq!(endpoint.transport, LanguageConnectionTransport::Serial);
        assert_eq!(
            endpoint.endpoint_transport,
            LanguageHostEndpointTransport::SerialPort
        );
        assert_eq!(endpoint.endpoint_scheme, "serial");
        assert_eq!(endpoint.port, "/dev/cu.usbmodem1101");

        let handoff = arduino_cli_upload_runtime_handoff_for_execution_plan(
            &arduino_cli_upload_execution_plan_for_target(
                "arduino-nano-r4",
                "/dev/cu.usbmodem9070692469E42",
                "/tmp/board-vm-nano-r4.bin",
            )
            .unwrap(),
            0,
            "Resetting board...\nNew upload port: /dev/cu.usbmodem1101 (serial)\n",
            "",
        )
        .unwrap();
        let handoff_plan = serial_runtime_open_plan_from_upload_handoff(&handoff).unwrap();
        assert_eq!(handoff_plan.board_id, "arduino-nano-r4");
        assert_eq!(handoff_plan.port, "/dev/cu.usbmodem1101");
        assert_eq!(handoff_plan.port_source, "arduino_cli_new_upload_port");
        assert_eq!(handoff_plan.upload_port_hint.as_deref(), Some("native_usb"));

        let esp =
            serial_runtime_open_plan_for_target("esp32", "/dev/tty.usbserial-CP2102").unwrap();
        assert_eq!(esp.board_id, "esp32-devkit-v1");
        assert_eq!(esp.upload_port_hint.as_deref(), Some("esp_rom_serial"));
        assert!(esp.notes.contains("ESP runtime serial port"));

        assert!(parse_serial_endpoint("tcp://board-vm.local:4170").is_none());
        assert!(parse_serial_endpoint("serial://   ").is_none());
        assert!(serial_runtime_open_plan_for_target("uno-r4-wifi", "   ").is_none());
        assert!(serial_runtime_open_plan_for_target("not-a-board", "COM7").is_none());
    }

    #[test]
    fn input_callback_plans_are_owned_by_rust_language_core() {
        let plan = input_callback_plan_for_target("uno-r4-wifi", 3, 7, 64).unwrap();

        assert_eq!(plan.board_id, "arduino-uno-r4-wifi");
        assert_eq!(plan.pin, 3);
        assert_eq!(plan.label, "D3");
        assert_eq!(plan.trigger, LanguageInputCallbackTrigger::FallingEdge);
        assert_eq!(plan.pull, LanguageInputCallbackPull::PullUp);
        assert_eq!(
            plan.debounce_ms,
            LANGUAGE_INPUT_CALLBACK_DEFAULT_DEBOUNCE_MS
        );
        assert_eq!(
            plan.queue_capacity,
            LANGUAGE_INPUT_CALLBACK_DEFAULT_QUEUE_CAPACITY
        );
        assert_eq!(
            plan.queue_policy,
            LanguageInputCallbackQueuePolicy::DropOldest
        );
        assert_eq!(plan.callback_program_id, 7);
        assert_eq!(plan.callback_instruction_budget, 64);
        assert!(plan.interrupt_backed);
        assert_eq!(plan.dispatch_model, LANGUAGE_INPUT_CALLBACK_DISPATCH_MODEL);

        let custom = input_callback_plan_with_options_for_target(
            "uno-r4-wifi",
            3,
            LanguageInputCallbackOptions {
                trigger: LanguageInputCallbackTrigger::RisingEdge,
                pull: LanguageInputCallbackPull::Floating,
                debounce_ms: 5,
                queue_capacity: 3,
                queue_policy: LanguageInputCallbackQueuePolicy::DropNewest,
                callback_program_id: 11,
                callback_instruction_budget: 128,
            },
        )
        .unwrap();
        assert_eq!(custom.trigger, LanguageInputCallbackTrigger::RisingEdge);
        assert_eq!(custom.pull, LanguageInputCallbackPull::Floating);
        assert_eq!(custom.debounce_ms, 5);
        assert_eq!(custom.queue_capacity, 3);
        assert_eq!(
            custom.queue_policy,
            LanguageInputCallbackQueuePolicy::DropNewest
        );
        assert_eq!(custom.callback_program_id, 11);
        assert_eq!(custom.callback_instruction_budget, 128);

        let error = input_callback_plan_for_target("not-a-board", 3, 7, 64).unwrap_err();
        assert_eq!(error.selector, "not-a-board");
        assert_eq!(error.pin, 3);
        assert_eq!(
            error.kind,
            LanguageInputCallbackPlanErrorKind::UnknownTarget
        );

        let error = input_callback_plan_for_target("uno-r4-wifi", 250, 7, 64).unwrap_err();
        assert_eq!(error.kind, LanguageInputCallbackPlanErrorKind::UnknownPin);

        let error = input_callback_plan_for_target("arduino-opta-wifi", 8, 7, 64).unwrap_err();
        assert_eq!(
            error.kind,
            LanguageInputCallbackPlanErrorKind::PinDoesNotSupportInput
        );

        let error = input_callback_plan_for_target("uno-r4-wifi", 4, 7, 64).unwrap_err();
        assert_eq!(
            error.kind,
            LanguageInputCallbackPlanErrorKind::PinDoesNotSupportInterrupt
        );

        let error = input_callback_plan_with_options_for_target(
            "uno-r4-wifi",
            3,
            LanguageInputCallbackOptions {
                trigger: LanguageInputCallbackTrigger::Change,
                pull: LanguageInputCallbackPull::PullDown,
                debounce_ms: 10,
                queue_capacity: 2,
                queue_policy: LanguageInputCallbackQueuePolicy::DropOldest,
                callback_program_id: 7,
                callback_instruction_budget: 64,
            },
        )
        .unwrap_err();
        assert_eq!(
            error.kind,
            LanguageInputCallbackPlanErrorKind::PinDoesNotSupportPull
        );

        let error = input_callback_plan_with_options_for_target(
            "uno-r4-wifi",
            3,
            LanguageInputCallbackOptions {
                trigger: LanguageInputCallbackTrigger::Change,
                pull: LanguageInputCallbackPull::PullUp,
                debounce_ms: 10,
                queue_capacity: 0,
                queue_policy: LanguageInputCallbackQueuePolicy::DropOldest,
                callback_program_id: 7,
                callback_instruction_budget: 64,
            },
        )
        .unwrap_err();
        assert_eq!(error.kind, LanguageInputCallbackPlanErrorKind::EmptyQueue);

        let error = input_callback_plan_for_target("uno-r4-wifi", 3, 7, 0).unwrap_err();
        assert_eq!(
            error.kind,
            LanguageInputCallbackPlanErrorKind::EmptyCallbackBudget
        );
    }

    #[test]
    fn input_callback_events_are_owned_by_rust_language_core() {
        let plan = input_callback_plan_for_target("uno-r4-wifi", 3, 7, 64).unwrap();
        let event = input_callback_event_for_plan(&plan, LanguageInputCallbackLevel::Low, 42, 9001);

        assert_eq!(event.board_id, "arduino-uno-r4-wifi");
        assert_eq!(event.pin, 3);
        assert_eq!(event.label, "D3");
        assert_eq!(event.event_kind, LANGUAGE_INPUT_CALLBACK_EVENT_KIND);
        assert_eq!(event.trigger, LanguageInputCallbackTrigger::FallingEdge);
        assert_eq!(event.level, LanguageInputCallbackLevel::Low);
        assert_eq!(event.sequence, 42);
        assert_eq!(event.timestamp_ms, 9001);

        let invocation = input_callback_invocation_for_event(&plan, &event).unwrap();
        assert_eq!(invocation.board_id, "arduino-uno-r4-wifi");
        assert_eq!(invocation.pin, 3);
        assert_eq!(invocation.label, "D3");
        assert_eq!(invocation.event_kind, LANGUAGE_INPUT_CALLBACK_EVENT_KIND);
        assert_eq!(
            invocation.trigger,
            LanguageInputCallbackTrigger::FallingEdge
        );
        assert_eq!(invocation.level, LanguageInputCallbackLevel::Low);
        assert_eq!(invocation.callback_program_id, 7);
        assert_eq!(invocation.callback_instruction_budget, 64);
        assert_eq!(invocation.sequence, 42);
        assert_eq!(invocation.timestamp_ms, 9001);
        assert_eq!(
            invocation.debounce_ms,
            LANGUAGE_INPUT_CALLBACK_DEFAULT_DEBOUNCE_MS
        );
        assert_eq!(
            invocation.queue_capacity,
            LANGUAGE_INPUT_CALLBACK_DEFAULT_QUEUE_CAPACITY
        );
        assert_eq!(
            invocation.queue_policy,
            LanguageInputCallbackQueuePolicy::DropOldest
        );
        assert!(invocation.interrupt_backed);
        assert_eq!(
            invocation.dispatch_model,
            LANGUAGE_INPUT_CALLBACK_DISPATCH_MODEL
        );

        let mut wrong_board = event.clone();
        wrong_board.board_id = "esp32-devkit-v1".to_owned();
        let error = input_callback_invocation_for_event(&plan, &wrong_board).unwrap_err();
        assert_eq!(error.plan_board_id, "arduino-uno-r4-wifi");
        assert_eq!(error.event_board_id, "esp32-devkit-v1");
        assert_eq!(
            error.kind,
            LanguageInputCallbackEventErrorKind::BoardMismatch
        );

        let mut wrong_pin = event.clone();
        wrong_pin.pin = 4;
        let error = input_callback_invocation_for_event(&plan, &wrong_pin).unwrap_err();
        assert_eq!(error.plan_pin, 3);
        assert_eq!(error.event_pin, 4);
        assert_eq!(error.kind, LanguageInputCallbackEventErrorKind::PinMismatch);

        let mut wrong_kind = event;
        wrong_kind.event_kind = "analog_input_change".to_owned();
        let error = input_callback_invocation_for_event(&plan, &wrong_kind).unwrap_err();
        assert_eq!(error.event_kind, "analog_input_change");
        assert_eq!(
            error.kind,
            LanguageInputCallbackEventErrorKind::EventKindMismatch
        );
    }

    #[test]
    fn input_callback_queue_plans_are_owned_by_rust_language_core() {
        let plan = input_callback_plan_for_target("uno-r4-wifi", 3, 7, 64).unwrap();
        let event = input_callback_event_for_plan(&plan, LanguageInputCallbackLevel::Low, 42, 9001);
        let invocation = input_callback_invocation_for_event(&plan, &event).unwrap();

        let queue_plan = input_callback_queue_plan_for_invocation(&invocation, 2).unwrap();
        assert_eq!(queue_plan.board_id, "arduino-uno-r4-wifi");
        assert_eq!(queue_plan.pin, 3);
        assert_eq!(queue_plan.label, "D3");
        assert_eq!(queue_plan.event_kind, LANGUAGE_INPUT_CALLBACK_EVENT_KIND);
        assert_eq!(queue_plan.callback_program_id, 7);
        assert_eq!(queue_plan.callback_instruction_budget, 64);
        assert_eq!(queue_plan.sequence, 42);
        assert_eq!(queue_plan.timestamp_ms, 9001);
        assert_eq!(
            queue_plan.debounce_ms,
            LANGUAGE_INPUT_CALLBACK_DEFAULT_DEBOUNCE_MS
        );
        assert_eq!(
            queue_plan.queue_capacity,
            LANGUAGE_INPUT_CALLBACK_DEFAULT_QUEUE_CAPACITY
        );
        assert_eq!(queue_plan.queue_depth_before, 2);
        assert_eq!(queue_plan.queue_depth_after, 3);
        assert_eq!(
            queue_plan.queue_policy,
            LanguageInputCallbackQueuePolicy::DropOldest
        );
        assert_eq!(queue_plan.action, LanguageInputCallbackQueueAction::Enqueue);
        assert!(queue_plan.queued);
        assert!(!queue_plan.dropped_existing_event);
        assert!(!queue_plan.dropped_incoming_event);
        assert!(queue_plan.dispatch_required);
        assert!(queue_plan.interrupt_backed);
        assert_eq!(
            queue_plan.dispatch_model,
            LANGUAGE_INPUT_CALLBACK_DISPATCH_MODEL
        );

        let full_queue =
            input_callback_queue_plan_for_invocation(&invocation, invocation.queue_capacity)
                .unwrap();
        assert_eq!(
            full_queue.action,
            LanguageInputCallbackQueueAction::DropOldestThenEnqueue
        );
        assert_eq!(
            full_queue.queue_depth_after,
            LANGUAGE_INPUT_CALLBACK_DEFAULT_QUEUE_CAPACITY
        );
        assert!(full_queue.queued);
        assert!(full_queue.dropped_existing_event);
        assert!(!full_queue.dropped_incoming_event);
        assert!(full_queue.dispatch_required);

        let custom = input_callback_plan_with_options_for_target(
            "uno-r4-wifi",
            3,
            LanguageInputCallbackOptions {
                trigger: LanguageInputCallbackTrigger::RisingEdge,
                pull: LanguageInputCallbackPull::Floating,
                debounce_ms: 5,
                queue_capacity: 1,
                queue_policy: LanguageInputCallbackQueuePolicy::DropNewest,
                callback_program_id: 9,
                callback_instruction_budget: 32,
            },
        )
        .unwrap();
        let custom_event =
            input_callback_event_for_plan(&custom, LanguageInputCallbackLevel::High, 77, 12_345);
        let custom_invocation =
            input_callback_invocation_for_event(&custom, &custom_event).unwrap();
        let newest_drop = input_callback_queue_plan_for_invocation(&custom_invocation, 1).unwrap();
        assert_eq!(
            newest_drop.action,
            LanguageInputCallbackQueueAction::DropNewest
        );
        assert_eq!(newest_drop.queue_depth_before, 1);
        assert_eq!(newest_drop.queue_depth_after, 1);
        assert!(!newest_drop.queued);
        assert!(!newest_drop.dropped_existing_event);
        assert!(newest_drop.dropped_incoming_event);
        assert!(!newest_drop.dispatch_required);

        let error = input_callback_queue_plan_for_invocation(&custom_invocation, 2).unwrap_err();
        assert_eq!(
            error.kind,
            LanguageInputCallbackQueuePlanErrorKind::QueueDepthExceedsCapacity
        );
        assert_eq!(error.queue_capacity, 1);
        assert_eq!(error.queue_depth, 2);

        let mut empty_queue = custom_invocation;
        empty_queue.queue_capacity = 0;
        let error = input_callback_queue_plan_for_invocation(&empty_queue, 0).unwrap_err();
        assert_eq!(
            error.kind,
            LanguageInputCallbackQueuePlanErrorKind::EmptyQueue
        );
    }

    #[test]
    fn input_callback_diagnostics_are_owned_by_rust_language_core() {
        let plan_error = input_callback_plan_for_target("not-a-board", 3, 7, 64).unwrap_err();
        let plan_diagnostic = input_callback_plan_diagnostic(&plan_error);
        assert_eq!(
            input_callback_plan_error_kind_name(plan_error.kind),
            "unknown_target"
        );
        assert_eq!(plan_diagnostic.selector, "not-a-board");
        assert_eq!(plan_diagnostic.pin, 3);
        assert_eq!(plan_diagnostic.kind, plan_error.kind);
        assert_eq!(plan_diagnostic.kind_name, "unknown_target");
        assert_eq!(
            plan_diagnostic.diagnostic_label,
            "input_callback_plan selector=not-a-board pin=3 error=unknown_target"
        );
        assert_eq!(
            plan_diagnostic.message,
            "Input callback target is not known."
        );
        assert_eq!(plan_diagnostic.error, plan_error);

        let plan = input_callback_plan_for_target("uno-r4-wifi", 3, 7, 64).unwrap();
        let mut wrong_pin =
            input_callback_event_for_plan(&plan, LanguageInputCallbackLevel::Low, 42, 9001);
        wrong_pin.pin = 4;
        let event_error = input_callback_invocation_for_event(&plan, &wrong_pin).unwrap_err();
        let event_diagnostic = input_callback_event_diagnostic(&event_error);
        assert_eq!(
            input_callback_event_error_kind_name(event_error.kind),
            "pin_mismatch"
        );
        assert_eq!(event_diagnostic.plan_board_id, "arduino-uno-r4-wifi");
        assert_eq!(event_diagnostic.event_board_id, "arduino-uno-r4-wifi");
        assert_eq!(event_diagnostic.plan_pin, 3);
        assert_eq!(event_diagnostic.event_pin, 4);
        assert_eq!(
            event_diagnostic.event_kind,
            LANGUAGE_INPUT_CALLBACK_EVENT_KIND
        );
        assert_eq!(event_diagnostic.kind_name, "pin_mismatch");
        assert_eq!(
            event_diagnostic.diagnostic_label,
            "input_callback_event plan_board=arduino-uno-r4-wifi event_board=arduino-uno-r4-wifi plan_pin=3 event_pin=4 event_kind=digital_input_change error=pin_mismatch"
        );
        assert_eq!(
            event_diagnostic.message,
            "Input callback event pin does not match the planned callback pin."
        );
        assert_eq!(event_diagnostic.error, event_error);

        let custom = input_callback_plan_with_options_for_target(
            "uno-r4-wifi",
            3,
            LanguageInputCallbackOptions {
                trigger: LanguageInputCallbackTrigger::RisingEdge,
                pull: LanguageInputCallbackPull::Floating,
                debounce_ms: 5,
                queue_capacity: 1,
                queue_policy: LanguageInputCallbackQueuePolicy::DropNewest,
                callback_program_id: 9,
                callback_instruction_budget: 32,
            },
        )
        .unwrap();
        let event =
            input_callback_event_for_plan(&custom, LanguageInputCallbackLevel::High, 77, 12_345);
        let invocation = input_callback_invocation_for_event(&custom, &event).unwrap();
        let queue_error = input_callback_queue_plan_for_invocation(&invocation, 2).unwrap_err();
        let queue_diagnostic = input_callback_queue_plan_diagnostic(&queue_error);
        assert_eq!(
            input_callback_queue_plan_error_kind_name(queue_error.kind),
            "queue_depth_exceeds_capacity"
        );
        assert_eq!(queue_diagnostic.board_id, "arduino-uno-r4-wifi");
        assert_eq!(queue_diagnostic.pin, 3);
        assert_eq!(queue_diagnostic.callback_program_id, 9);
        assert_eq!(queue_diagnostic.queue_capacity, 1);
        assert_eq!(queue_diagnostic.queue_depth, 2);
        assert_eq!(queue_diagnostic.kind_name, "queue_depth_exceeds_capacity");
        assert_eq!(
            queue_diagnostic.diagnostic_label,
            "input_callback_queue board=arduino-uno-r4-wifi pin=3 program=9 queue_depth=2 queue_capacity=1 error=queue_depth_exceeds_capacity"
        );
        assert_eq!(
            queue_diagnostic.message,
            "Input callback queue depth exceeds the configured queue capacity."
        );
        assert_eq!(queue_diagnostic.error, queue_error);
    }

    #[test]
    fn input_callback_session_diagnostics_are_owned_by_rust_language_core() {
        let serial_session = host_endpoint_session_summary("serial:///dev/cu.usbmodem1101", 57_600)
            .expect("serial endpoint session");
        let tcp_session = host_endpoint_session_summary("tcp://board-vm.local:4170", 57_600)
            .expect("tcp endpoint session");

        let plan_error = input_callback_plan_for_target("not-a-board", 3, 7, 64).unwrap_err();
        let plan_diagnostic = input_callback_plan_diagnostic(&plan_error);
        let session_plan =
            input_callback_session_plan_diagnostic(&serial_session, &plan_diagnostic);
        assert_eq!(
            input_callback_diagnostic_stage_name(session_plan.diagnostic_stage),
            "plan"
        );
        assert_eq!(
            session_plan.endpoint.endpoint,
            "serial:///dev/cu.usbmodem1101"
        );
        assert_eq!(
            session_plan.endpoint.endpoint_transport,
            LanguageHostEndpointTransport::SerialPort
        );
        assert_eq!(
            session_plan.connection_label,
            "endpoint=serial:///dev/cu.usbmodem1101 baud=57600"
        );
        assert_eq!(session_plan.stage_name, "plan");
        assert_eq!(session_plan.kind_name, "unknown_target");
        assert_eq!(
            session_plan.source_diagnostic_label,
            "input_callback_plan selector=not-a-board pin=3 error=unknown_target"
        );
        assert_eq!(
            session_plan.diagnostic_label,
            "endpoint=serial:///dev/cu.usbmodem1101 baud=57600 callback_diagnostic_stage=plan input_callback_plan selector=not-a-board pin=3 error=unknown_target"
        );
        assert_eq!(session_plan.message, "Input callback target is not known.");

        let plan = input_callback_plan_for_target("uno-r4-wifi", 3, 7, 64).unwrap();
        let mut wrong_pin =
            input_callback_event_for_plan(&plan, LanguageInputCallbackLevel::Low, 42, 9001);
        wrong_pin.pin = 4;
        let event_error = input_callback_invocation_for_event(&plan, &wrong_pin).unwrap_err();
        let event_diagnostic = input_callback_event_diagnostic(&event_error);
        let session_event =
            input_callback_session_event_diagnostic(&tcp_session, &event_diagnostic);
        assert_eq!(session_event.endpoint.endpoint, "tcp://board-vm.local:4170");
        assert_eq!(
            session_event.endpoint.endpoint_transport,
            LanguageHostEndpointTransport::TcpSocket
        );
        assert_eq!(
            session_event.connection_label,
            "endpoint=tcp://board-vm.local:4170"
        );
        assert_eq!(
            session_event.diagnostic_stage,
            LanguageInputCallbackDiagnosticStage::Event
        );
        assert_eq!(session_event.stage_name, "event");
        assert_eq!(session_event.kind_name, "pin_mismatch");
        assert_eq!(
            session_event.source_diagnostic_label,
            "input_callback_event plan_board=arduino-uno-r4-wifi event_board=arduino-uno-r4-wifi plan_pin=3 event_pin=4 event_kind=digital_input_change error=pin_mismatch"
        );
        assert_eq!(
            session_event.diagnostic_label,
            "endpoint=tcp://board-vm.local:4170 callback_diagnostic_stage=event input_callback_event plan_board=arduino-uno-r4-wifi event_board=arduino-uno-r4-wifi plan_pin=3 event_pin=4 event_kind=digital_input_change error=pin_mismatch"
        );

        let custom = input_callback_plan_with_options_for_target(
            "uno-r4-wifi",
            3,
            LanguageInputCallbackOptions {
                trigger: LanguageInputCallbackTrigger::RisingEdge,
                pull: LanguageInputCallbackPull::Floating,
                debounce_ms: 5,
                queue_capacity: 1,
                queue_policy: LanguageInputCallbackQueuePolicy::DropNewest,
                callback_program_id: 9,
                callback_instruction_budget: 32,
            },
        )
        .unwrap();
        let event =
            input_callback_event_for_plan(&custom, LanguageInputCallbackLevel::High, 77, 12_345);
        let invocation = input_callback_invocation_for_event(&custom, &event).unwrap();
        let queue_error = input_callback_queue_plan_for_invocation(&invocation, 2).unwrap_err();
        let queue_diagnostic = input_callback_queue_plan_diagnostic(&queue_error);
        let session_queue =
            input_callback_session_queue_plan_diagnostic(&tcp_session, &queue_diagnostic);
        assert_eq!(
            session_queue.diagnostic_stage,
            LanguageInputCallbackDiagnosticStage::QueuePlan
        );
        assert_eq!(session_queue.stage_name, "queue_plan");
        assert_eq!(session_queue.kind_name, "queue_depth_exceeds_capacity");
        assert_eq!(
            session_queue.source_diagnostic_label,
            "input_callback_queue board=arduino-uno-r4-wifi pin=3 program=9 queue_depth=2 queue_capacity=1 error=queue_depth_exceeds_capacity"
        );
        assert_eq!(
            session_queue.diagnostic_label,
            "endpoint=tcp://board-vm.local:4170 callback_diagnostic_stage=queue_plan input_callback_queue board=arduino-uno-r4-wifi pin=3 program=9 queue_depth=2 queue_capacity=1 error=queue_depth_exceeds_capacity"
        );
        assert_eq!(
            session_queue.message,
            "Input callback queue depth exceeds the configured queue capacity."
        );
    }

    #[test]
    fn input_callback_session_queue_summaries_are_owned_by_rust_language_core() {
        let plan = input_callback_plan_for_target("uno-r4-wifi", 3, 7, 64).unwrap();
        let event = input_callback_event_for_plan(&plan, LanguageInputCallbackLevel::Low, 42, 9001);
        let invocation = input_callback_invocation_for_event(&plan, &event).unwrap();
        let serial_session = host_endpoint_session_summary("serial:///dev/cu.usbmodem1101", 57_600)
            .expect("serial endpoint session");

        let queue_plan = input_callback_queue_plan_for_invocation(&invocation, 2).unwrap();
        let serial = input_callback_session_queue_summary(&serial_session, &queue_plan);
        assert_eq!(serial.endpoint.endpoint, "serial:///dev/cu.usbmodem1101");
        assert_eq!(
            serial.endpoint.endpoint_transport,
            LanguageHostEndpointTransport::SerialPort
        );
        assert_eq!(
            serial.connection_label,
            "endpoint=serial:///dev/cu.usbmodem1101 baud=57600"
        );
        assert_eq!(
            serial.queue_label,
            "endpoint=serial:///dev/cu.usbmodem1101 baud=57600 callback=arduino-uno-r4-wifi:D3 sequence=42 queue_action=enqueue queue_depth_before=2 queue_depth_after=3"
        );
        assert_eq!(serial.action, LanguageInputCallbackQueueAction::Enqueue);
        assert_eq!(
            serial.queue_policy,
            LanguageInputCallbackQueuePolicy::DropOldest
        );
        assert!(serial.queued);
        assert!(!serial.dropped_existing_event);
        assert!(!serial.dropped_incoming_event);
        assert!(serial.dispatch_required);
        assert_eq!(serial.queue_depth_before, 2);
        assert_eq!(serial.queue_depth_after, 3);
        assert_eq!(
            serial.message,
            "Input callback enqueued; wake cooperative dispatch."
        );
        assert_eq!(serial.queue_plan, queue_plan);

        let full_queue =
            input_callback_queue_plan_for_invocation(&invocation, invocation.queue_capacity)
                .unwrap();
        let full = input_callback_session_queue_summary(&serial_session, &full_queue);
        assert_eq!(
            full.queue_label,
            "endpoint=serial:///dev/cu.usbmodem1101 baud=57600 callback=arduino-uno-r4-wifi:D3 sequence=42 queue_action=drop_oldest_then_enqueue queue_depth_before=8 queue_depth_after=8"
        );
        assert_eq!(
            full.action,
            LanguageInputCallbackQueueAction::DropOldestThenEnqueue
        );
        assert!(full.queued);
        assert!(full.dropped_existing_event);
        assert!(!full.dropped_incoming_event);
        assert!(full.dispatch_required);
        assert_eq!(
            full.message,
            "Input callback enqueued after dropping the oldest queued callback."
        );

        let custom = input_callback_plan_with_options_for_target(
            "uno-r4-wifi",
            3,
            LanguageInputCallbackOptions {
                trigger: LanguageInputCallbackTrigger::RisingEdge,
                pull: LanguageInputCallbackPull::Floating,
                debounce_ms: 5,
                queue_capacity: 1,
                queue_policy: LanguageInputCallbackQueuePolicy::DropNewest,
                callback_program_id: 9,
                callback_instruction_budget: 32,
            },
        )
        .unwrap();
        let custom_event =
            input_callback_event_for_plan(&custom, LanguageInputCallbackLevel::High, 77, 12_345);
        let custom_invocation =
            input_callback_invocation_for_event(&custom, &custom_event).unwrap();
        let newest_drop = input_callback_queue_plan_for_invocation(&custom_invocation, 1).unwrap();
        let tcp_session = host_endpoint_session_summary("tcp://board-vm.local:4170", 57_600)
            .expect("tcp endpoint session");
        let tcp = input_callback_session_queue_summary(&tcp_session, &newest_drop);

        assert_eq!(tcp.endpoint.endpoint, "tcp://board-vm.local:4170");
        assert_eq!(
            tcp.endpoint.endpoint_transport,
            LanguageHostEndpointTransport::TcpSocket
        );
        assert_eq!(tcp.connection_label, "endpoint=tcp://board-vm.local:4170");
        assert_eq!(
            tcp.queue_label,
            "endpoint=tcp://board-vm.local:4170 callback=arduino-uno-r4-wifi:D3 sequence=77 queue_action=drop_newest queue_depth_before=1 queue_depth_after=1"
        );
        assert_eq!(tcp.action, LanguageInputCallbackQueueAction::DropNewest);
        assert_eq!(
            tcp.queue_policy,
            LanguageInputCallbackQueuePolicy::DropNewest
        );
        assert!(!tcp.queued);
        assert!(!tcp.dropped_existing_event);
        assert!(tcp.dropped_incoming_event);
        assert!(!tcp.dispatch_required);
        assert_eq!(tcp.queue_depth_before, 1);
        assert_eq!(tcp.queue_depth_after, 1);
        assert_eq!(
            tcp.message,
            "Input callback dropped because the queue is full and drop-newest policy is active."
        );
        assert_eq!(tcp.queue_plan, newest_drop);
    }

    #[test]
    fn input_callback_dispatch_plans_are_owned_by_rust_language_core() {
        let plan = input_callback_plan_for_target("uno-r4-wifi", 3, 7, 64).unwrap();
        let event = input_callback_event_for_plan(&plan, LanguageInputCallbackLevel::Low, 42, 9001);
        let invocation = input_callback_invocation_for_event(&plan, &event).unwrap();

        let queue_plan = input_callback_queue_plan_for_invocation(&invocation, 2).unwrap();
        let dispatch = input_callback_dispatch_plan_for_queue_plan(&queue_plan).unwrap();
        assert_eq!(dispatch.board_id, "arduino-uno-r4-wifi");
        assert_eq!(dispatch.pin, 3);
        assert_eq!(dispatch.label, "D3");
        assert_eq!(dispatch.event_kind, LANGUAGE_INPUT_CALLBACK_EVENT_KIND);
        assert_eq!(
            dispatch.dispatch_reason,
            LANGUAGE_INPUT_CALLBACK_DISPATCH_REASON
        );
        assert_eq!(dispatch.callback_program_id, 7);
        assert_eq!(dispatch.callback_instruction_budget, 64);
        assert_eq!(dispatch.sequence, 42);
        assert_eq!(dispatch.timestamp_ms, 9001);
        assert_eq!(dispatch.queue_depth_after, 3);
        assert_eq!(
            dispatch.queue_action,
            LanguageInputCallbackQueueAction::Enqueue
        );
        assert!(!dispatch.dropped_existing_event);
        assert!(dispatch.interrupt_backed);
        assert_eq!(
            dispatch.dispatch_model,
            LANGUAGE_INPUT_CALLBACK_DISPATCH_MODEL
        );

        let full_queue =
            input_callback_queue_plan_for_invocation(&invocation, invocation.queue_capacity)
                .unwrap();
        let dispatch = input_callback_dispatch_plan_for_queue_plan(&full_queue).unwrap();
        assert_eq!(
            dispatch.queue_action,
            LanguageInputCallbackQueueAction::DropOldestThenEnqueue
        );
        assert_eq!(
            dispatch.queue_depth_after,
            LANGUAGE_INPUT_CALLBACK_DEFAULT_QUEUE_CAPACITY
        );
        assert!(dispatch.dropped_existing_event);

        let custom = input_callback_plan_with_options_for_target(
            "uno-r4-wifi",
            3,
            LanguageInputCallbackOptions {
                trigger: LanguageInputCallbackTrigger::RisingEdge,
                pull: LanguageInputCallbackPull::Floating,
                debounce_ms: 5,
                queue_capacity: 1,
                queue_policy: LanguageInputCallbackQueuePolicy::DropNewest,
                callback_program_id: 9,
                callback_instruction_budget: 32,
            },
        )
        .unwrap();
        let custom_event =
            input_callback_event_for_plan(&custom, LanguageInputCallbackLevel::High, 77, 12_345);
        let custom_invocation =
            input_callback_invocation_for_event(&custom, &custom_event).unwrap();
        let newest_drop = input_callback_queue_plan_for_invocation(&custom_invocation, 1).unwrap();
        assert!(input_callback_dispatch_plan_for_queue_plan(&newest_drop).is_none());
    }

    #[test]
    fn input_callback_session_dispatch_summaries_are_owned_by_rust_language_core() {
        let plan = input_callback_plan_for_target("uno-r4-wifi", 3, 7, 64).unwrap();
        let event = input_callback_event_for_plan(&plan, LanguageInputCallbackLevel::Low, 42, 9001);
        let invocation = input_callback_invocation_for_event(&plan, &event).unwrap();
        let serial_session = host_endpoint_session_summary("serial:///dev/cu.usbmodem1101", 57_600)
            .expect("serial endpoint session");

        let queue_plan = input_callback_queue_plan_for_invocation(&invocation, 2).unwrap();
        let dispatch = input_callback_dispatch_plan_for_queue_plan(&queue_plan).unwrap();
        let serial = input_callback_session_dispatch_summary(&serial_session, &dispatch);

        assert_eq!(serial.endpoint.endpoint, "serial:///dev/cu.usbmodem1101");
        assert_eq!(
            serial.endpoint.endpoint_transport,
            LanguageHostEndpointTransport::SerialPort
        );
        assert_eq!(
            serial.connection_label,
            "endpoint=serial:///dev/cu.usbmodem1101 baud=57600"
        );
        assert_eq!(
            serial.dispatch_label,
            "endpoint=serial:///dev/cu.usbmodem1101 baud=57600 callback=arduino-uno-r4-wifi:D3 sequence=42 dispatch_reason=queued_input_event queue_action=enqueue queue_depth_after=3 instruction_budget=64"
        );
        assert_eq!(
            serial.dispatch_reason,
            LANGUAGE_INPUT_CALLBACK_DISPATCH_REASON
        );
        assert_eq!(serial.callback_program_id, 7);
        assert_eq!(serial.callback_instruction_budget, 64);
        assert_eq!(serial.sequence, 42);
        assert_eq!(serial.queue_depth_after, 3);
        assert_eq!(
            serial.queue_action,
            LanguageInputCallbackQueueAction::Enqueue
        );
        assert!(!serial.dropped_existing_event);
        assert!(serial.interrupt_backed);
        assert_eq!(
            serial.dispatch_model,
            LANGUAGE_INPUT_CALLBACK_DISPATCH_MODEL
        );
        assert_eq!(
            serial.message,
            "Input callback dispatch is ready for the cooperative runner."
        );
        assert_eq!(serial.dispatch_plan, dispatch);

        let full_queue =
            input_callback_queue_plan_for_invocation(&invocation, invocation.queue_capacity)
                .unwrap();
        let full_dispatch = input_callback_dispatch_plan_for_queue_plan(&full_queue).unwrap();
        let tcp_session = host_endpoint_session_summary("tcp://board-vm.local:4170", 57_600)
            .expect("tcp endpoint session");
        let tcp = input_callback_session_dispatch_summary(&tcp_session, &full_dispatch);

        assert_eq!(tcp.endpoint.endpoint, "tcp://board-vm.local:4170");
        assert_eq!(
            tcp.endpoint.endpoint_transport,
            LanguageHostEndpointTransport::TcpSocket
        );
        assert_eq!(tcp.connection_label, "endpoint=tcp://board-vm.local:4170");
        assert_eq!(
            tcp.dispatch_label,
            "endpoint=tcp://board-vm.local:4170 callback=arduino-uno-r4-wifi:D3 sequence=42 dispatch_reason=queued_input_event queue_action=drop_oldest_then_enqueue queue_depth_after=8 instruction_budget=64"
        );
        assert_eq!(
            tcp.queue_action,
            LanguageInputCallbackQueueAction::DropOldestThenEnqueue
        );
        assert!(tcp.dropped_existing_event);
        assert!(tcp.interrupt_backed);
        assert_eq!(
            tcp.message,
            "Input callback dispatch replaces the oldest queued callback."
        );
        assert_eq!(tcp.dispatch_plan, full_dispatch);
    }

    #[test]
    fn input_callback_results_are_owned_by_rust_language_core() {
        let plan = input_callback_plan_for_target("uno-r4-wifi", 3, 7, 64).unwrap();
        let event = input_callback_event_for_plan(&plan, LanguageInputCallbackLevel::Low, 42, 9001);
        let invocation = input_callback_invocation_for_event(&plan, &event).unwrap();
        let queue_plan = input_callback_queue_plan_for_invocation(&invocation, 2).unwrap();
        let dispatch = input_callback_dispatch_plan_for_queue_plan(&queue_plan).unwrap();

        let completed =
            input_callback_result_for_dispatch_plan(&dispatch, RunStatus::Halted, 11, 3);
        assert_eq!(completed.board_id, "arduino-uno-r4-wifi");
        assert_eq!(completed.pin, 3);
        assert_eq!(completed.label, "D3");
        assert_eq!(completed.event_kind, LANGUAGE_INPUT_CALLBACK_EVENT_KIND);
        assert_eq!(
            completed.dispatch_reason,
            LANGUAGE_INPUT_CALLBACK_DISPATCH_REASON
        );
        assert_eq!(completed.callback_program_id, 7);
        assert_eq!(completed.callback_instruction_budget, 64);
        assert_eq!(completed.sequence, 42);
        assert_eq!(completed.timestamp_ms, 9001);
        assert_eq!(completed.queue_depth_after, 3);
        assert_eq!(
            completed.queue_action,
            LanguageInputCallbackQueueAction::Enqueue
        );
        assert!(!completed.dropped_existing_event);
        assert!(completed.interrupt_backed);
        assert_eq!(
            completed.dispatch_model,
            LANGUAGE_INPUT_CALLBACK_DISPATCH_MODEL
        );
        assert_eq!(completed.run_status, "halted");
        assert_eq!(
            completed.result_kind,
            LanguageInputCallbackResultKind::Completed
        );
        assert_eq!(completed.instructions_executed, 11);
        assert_eq!(completed.elapsed_ms, 3);
        assert!(completed.completed);
        assert!(!completed.budget_exceeded);
        assert!(!completed.retryable);
        assert_eq!(completed.message, "Input callback completed.");

        let budget =
            input_callback_result_for_dispatch_plan(&dispatch, RunStatus::BudgetExceeded, 64, 9);
        assert_eq!(budget.run_status, "budget_exceeded");
        assert_eq!(
            budget.result_kind,
            LanguageInputCallbackResultKind::BudgetExceeded
        );
        assert!(!budget.completed);
        assert!(budget.budget_exceeded);
        assert!(!budget.retryable);

        let running = input_callback_result_for_dispatch_plan(&dispatch, RunStatus::Running, 12, 4);
        assert_eq!(running.run_status, "running");
        assert_eq!(
            running.result_kind,
            LanguageInputCallbackResultKind::Incomplete
        );
        assert!(!running.completed);
        assert!(!running.budget_exceeded);
        assert!(running.retryable);

        let stopped = input_callback_result_for_dispatch_plan(&dispatch, RunStatus::Stopped, 6, 2);
        assert_eq!(stopped.run_status, "stopped");
        assert_eq!(stopped.result_kind, LanguageInputCallbackResultKind::Failed);
        assert!(!stopped.completed);
        assert!(!stopped.budget_exceeded);
        assert!(!stopped.retryable);

        let faulted = input_callback_result_for_dispatch_plan(&dispatch, RunStatus::Faulted, 6, 2);
        assert_eq!(faulted.run_status, "faulted");
        assert_eq!(faulted.result_kind, LanguageInputCallbackResultKind::Failed);
    }

    #[test]
    fn input_callback_transport_results_are_owned_by_rust_language_core() {
        let plan = input_callback_plan_for_target("uno-r4-wifi", 3, 7, 64).unwrap();
        let event = input_callback_event_for_plan(&plan, LanguageInputCallbackLevel::Low, 42, 9001);
        let invocation = input_callback_invocation_for_event(&plan, &event).unwrap();
        let queue_plan = input_callback_queue_plan_for_invocation(&invocation, 2).unwrap();
        let dispatch = input_callback_dispatch_plan_for_queue_plan(&queue_plan).unwrap();

        let completed =
            input_callback_result_for_dispatch_plan(&dispatch, RunStatus::Halted, 11, 3);
        let serial_session = host_endpoint_session_summary("serial:///dev/cu.usbmodem1101", 57_600)
            .expect("serial endpoint session");
        let serial = input_callback_transport_result_summary(&serial_session, &completed);

        assert_eq!(serial.endpoint.endpoint, "serial:///dev/cu.usbmodem1101");
        assert_eq!(
            serial.endpoint.transport,
            LanguageConnectionTransport::Serial
        );
        assert_eq!(
            serial.endpoint.endpoint_transport,
            LanguageHostEndpointTransport::SerialPort
        );
        assert_eq!(
            serial.connection_label,
            "endpoint=serial:///dev/cu.usbmodem1101 baud=57600"
        );
        assert_eq!(
            serial.callback_label,
            "endpoint=serial:///dev/cu.usbmodem1101 baud=57600 callback=arduino-uno-r4-wifi:D3 sequence=42 status=halted"
        );
        assert_eq!(serial.result, completed);

        let budget =
            input_callback_result_for_dispatch_plan(&dispatch, RunStatus::BudgetExceeded, 64, 9);
        let tcp_session = host_endpoint_session_summary("tcp://board-vm.local:4170", 57_600)
            .expect("tcp endpoint session");
        let tcp = input_callback_transport_result_summary(&tcp_session, &budget);

        assert_eq!(tcp.endpoint.endpoint, "tcp://board-vm.local:4170");
        assert_eq!(tcp.endpoint.transport, LanguageConnectionTransport::Wifi);
        assert_eq!(
            tcp.endpoint.endpoint_transport,
            LanguageHostEndpointTransport::TcpSocket
        );
        assert_eq!(tcp.connection_label, "endpoint=tcp://board-vm.local:4170");
        assert_eq!(
            tcp.callback_label,
            "endpoint=tcp://board-vm.local:4170 callback=arduino-uno-r4-wifi:D3 sequence=42 status=budget_exceeded"
        );
        assert_eq!(
            tcp.result.result_kind,
            LanguageInputCallbackResultKind::BudgetExceeded
        );
    }

    #[test]
    fn input_callback_completion_plans_are_owned_by_rust_language_core() {
        let plan = input_callback_plan_for_target("uno-r4-wifi", 3, 7, 64).unwrap();
        let event = input_callback_event_for_plan(&plan, LanguageInputCallbackLevel::Low, 42, 9001);
        let invocation = input_callback_invocation_for_event(&plan, &event).unwrap();
        let queue_plan = input_callback_queue_plan_for_invocation(&invocation, 2).unwrap();
        let dispatch = input_callback_dispatch_plan_for_queue_plan(&queue_plan).unwrap();

        let completed =
            input_callback_result_for_dispatch_plan(&dispatch, RunStatus::Halted, 11, 3);
        let completed_plan = input_callback_completion_plan_for_result(&completed);
        assert_eq!(
            completed_plan.action,
            LanguageInputCallbackCompletionAction::Complete
        );
        assert!(completed_plan.remove_from_queue);
        assert!(!completed_plan.keep_dispatch_scheduled);
        assert!(completed_plan.terminal);
        assert!(!completed_plan.retryable);
        assert_eq!(completed_plan.queue_depth_after_completion, 2);
        assert_eq!(completed_plan.result, completed);
        assert_eq!(
            completed_plan.message,
            "Input callback completed; remove it from the cooperative queue."
        );

        let running = input_callback_result_for_dispatch_plan(&dispatch, RunStatus::Running, 12, 4);
        let running_plan = input_callback_completion_plan_for_result(&running);
        assert_eq!(
            running_plan.action,
            LanguageInputCallbackCompletionAction::KeepRunning
        );
        assert!(!running_plan.remove_from_queue);
        assert!(running_plan.keep_dispatch_scheduled);
        assert!(!running_plan.terminal);
        assert!(running_plan.retryable);
        assert_eq!(running_plan.queue_depth_after_completion, 3);

        let budget =
            input_callback_result_for_dispatch_plan(&dispatch, RunStatus::BudgetExceeded, 64, 9);
        let budget_plan = input_callback_completion_plan_for_result(&budget);
        assert_eq!(
            budget_plan.action,
            LanguageInputCallbackCompletionAction::DropAfterBudgetExceeded
        );
        assert!(budget_plan.remove_from_queue);
        assert!(!budget_plan.keep_dispatch_scheduled);
        assert!(budget_plan.terminal);
        assert!(!budget_plan.retryable);
        assert_eq!(budget_plan.queue_depth_after_completion, 2);

        let stopped = input_callback_result_for_dispatch_plan(&dispatch, RunStatus::Stopped, 6, 2);
        let stopped_plan = input_callback_completion_plan_for_result(&stopped);
        assert_eq!(
            stopped_plan.action,
            LanguageInputCallbackCompletionAction::DropAfterFailure
        );
        assert!(stopped_plan.remove_from_queue);
        assert_eq!(stopped_plan.queue_depth_after_completion, 2);

        let mut empty_queue = completed_plan.result.clone();
        empty_queue.queue_depth_after = 0;
        let empty_queue_plan = input_callback_completion_plan_for_result(&empty_queue);
        assert_eq!(empty_queue_plan.queue_depth_after_completion, 0);
    }

    #[test]
    fn input_callback_session_completions_are_owned_by_rust_language_core() {
        let plan = input_callback_plan_for_target("uno-r4-wifi", 3, 7, 64).unwrap();
        let event = input_callback_event_for_plan(&plan, LanguageInputCallbackLevel::Low, 42, 9001);
        let invocation = input_callback_invocation_for_event(&plan, &event).unwrap();
        let queue_plan = input_callback_queue_plan_for_invocation(&invocation, 2).unwrap();
        let dispatch = input_callback_dispatch_plan_for_queue_plan(&queue_plan).unwrap();

        let completed =
            input_callback_result_for_dispatch_plan(&dispatch, RunStatus::Halted, 11, 3);
        let serial_session = host_endpoint_session_summary("serial:///dev/cu.usbmodem1101", 57_600)
            .expect("serial endpoint session");
        let serial = input_callback_session_completion_summary(&serial_session, &completed);

        assert_eq!(serial.endpoint.endpoint, "serial:///dev/cu.usbmodem1101");
        assert_eq!(
            serial.endpoint.endpoint_transport,
            LanguageHostEndpointTransport::SerialPort
        );
        assert_eq!(
            serial.connection_label,
            "endpoint=serial:///dev/cu.usbmodem1101 baud=57600"
        );
        assert_eq!(
            serial.callback_label,
            "endpoint=serial:///dev/cu.usbmodem1101 baud=57600 callback=arduino-uno-r4-wifi:D3 sequence=42 status=halted"
        );
        assert_eq!(
            serial.completion_label,
            "endpoint=serial:///dev/cu.usbmodem1101 baud=57600 callback=arduino-uno-r4-wifi:D3 sequence=42 status=halted action=complete queue_depth_after_completion=2"
        );
        assert_eq!(
            serial.action,
            LanguageInputCallbackCompletionAction::Complete
        );
        assert!(serial.remove_from_queue);
        assert!(!serial.keep_dispatch_scheduled);
        assert!(serial.terminal);
        assert!(!serial.retryable);
        assert_eq!(serial.queue_depth_after_completion, 2);
        assert_eq!(serial.result, completed);

        let running = input_callback_result_for_dispatch_plan(&dispatch, RunStatus::Running, 12, 4);
        let tcp_session = host_endpoint_session_summary("tcp://board-vm.local:4170", 57_600)
            .expect("tcp endpoint session");
        let tcp = input_callback_session_completion_summary(&tcp_session, &running);

        assert_eq!(tcp.endpoint.endpoint, "tcp://board-vm.local:4170");
        assert_eq!(
            tcp.endpoint.endpoint_transport,
            LanguageHostEndpointTransport::TcpSocket
        );
        assert_eq!(tcp.connection_label, "endpoint=tcp://board-vm.local:4170");
        assert_eq!(
            tcp.callback_label,
            "endpoint=tcp://board-vm.local:4170 callback=arduino-uno-r4-wifi:D3 sequence=42 status=running"
        );
        assert_eq!(
            tcp.completion_label,
            "endpoint=tcp://board-vm.local:4170 callback=arduino-uno-r4-wifi:D3 sequence=42 status=running action=keep_running queue_depth_after_completion=3"
        );
        assert_eq!(
            tcp.action,
            LanguageInputCallbackCompletionAction::KeepRunning
        );
        assert!(!tcp.remove_from_queue);
        assert!(tcp.keep_dispatch_scheduled);
        assert!(!tcp.terminal);
        assert!(tcp.retryable);
        assert_eq!(tcp.queue_depth_after_completion, 3);
    }

    #[test]
    fn input_callback_session_lifecycle_summaries_are_owned_by_rust_language_core() {
        let plan = input_callback_plan_for_target("uno-r4-wifi", 3, 7, 64).unwrap();
        let event = input_callback_event_for_plan(&plan, LanguageInputCallbackLevel::Low, 42, 9001);
        let invocation = input_callback_invocation_for_event(&plan, &event).unwrap();
        let queue_plan = input_callback_queue_plan_for_invocation(&invocation, 2).unwrap();
        let serial_session = host_endpoint_session_summary("serial:///dev/cu.usbmodem1101", 57_600)
            .expect("serial endpoint session");
        let completed = input_callback_session_lifecycle_summary(
            &serial_session,
            &queue_plan,
            Some(RunStatus::Halted),
            11,
            3,
        );

        assert_eq!(completed.endpoint.endpoint, "serial:///dev/cu.usbmodem1101");
        assert_eq!(
            completed.endpoint.endpoint_transport,
            LanguageHostEndpointTransport::SerialPort
        );
        assert_eq!(
            completed.connection_label,
            "endpoint=serial:///dev/cu.usbmodem1101 baud=57600"
        );
        assert_eq!(
            completed.lifecycle_label,
            "endpoint=serial:///dev/cu.usbmodem1101 baud=57600 callback=arduino-uno-r4-wifi:D3 sequence=42 queued=true dispatch_required=true terminal=true"
        );
        assert!(completed.queued);
        assert!(completed.dispatch_required);
        assert!(completed.terminal);
        assert!(!completed.retryable);
        assert_eq!(
            completed.queue_summary.queue_label,
            "endpoint=serial:///dev/cu.usbmodem1101 baud=57600 callback=arduino-uno-r4-wifi:D3 sequence=42 queue_action=enqueue queue_depth_before=2 queue_depth_after=3"
        );
        let dispatch = completed
            .dispatch_summary
            .as_ref()
            .expect("dispatch summary");
        assert_eq!(
            dispatch.dispatch_label,
            "endpoint=serial:///dev/cu.usbmodem1101 baud=57600 callback=arduino-uno-r4-wifi:D3 sequence=42 dispatch_reason=queued_input_event queue_action=enqueue queue_depth_after=3 instruction_budget=64"
        );
        let result = completed.result.as_ref().expect("callback result");
        assert_eq!(result.run_status, "halted");
        assert_eq!(
            result.result_kind,
            LanguageInputCallbackResultKind::Completed
        );
        let completion = completed
            .completion_summary
            .as_ref()
            .expect("completion summary");
        assert_eq!(
            completion.action,
            LanguageInputCallbackCompletionAction::Complete
        );
        assert_eq!(
            completed.message,
            "Input callback completed; remove it from the cooperative queue."
        );

        let tcp_session = host_endpoint_session_summary("tcp://board-vm.local:4170", 57_600)
            .expect("tcp endpoint session");
        let pending =
            input_callback_session_lifecycle_summary(&tcp_session, &queue_plan, None, 0, 0);
        assert_eq!(pending.endpoint.endpoint, "tcp://board-vm.local:4170");
        assert_eq!(
            pending.endpoint.endpoint_transport,
            LanguageHostEndpointTransport::TcpSocket
        );
        assert_eq!(
            pending.connection_label,
            "endpoint=tcp://board-vm.local:4170"
        );
        assert_eq!(
            pending.lifecycle_label,
            "endpoint=tcp://board-vm.local:4170 callback=arduino-uno-r4-wifi:D3 sequence=42 queued=true dispatch_required=true terminal=false"
        );
        assert!(pending.queued);
        assert!(pending.dispatch_required);
        assert!(!pending.terminal);
        assert!(!pending.retryable);
        assert!(pending.dispatch_summary.is_some());
        assert!(pending.result.is_none());
        assert!(pending.completion_summary.is_none());
        assert_eq!(
            pending.message,
            "Input callback is queued for cooperative dispatch."
        );

        let custom = input_callback_plan_with_options_for_target(
            "uno-r4-wifi",
            3,
            LanguageInputCallbackOptions {
                trigger: LanguageInputCallbackTrigger::RisingEdge,
                pull: LanguageInputCallbackPull::Floating,
                debounce_ms: 5,
                queue_capacity: 1,
                queue_policy: LanguageInputCallbackQueuePolicy::DropNewest,
                callback_program_id: 9,
                callback_instruction_budget: 32,
            },
        )
        .unwrap();
        let custom_event =
            input_callback_event_for_plan(&custom, LanguageInputCallbackLevel::High, 77, 12_345);
        let custom_invocation =
            input_callback_invocation_for_event(&custom, &custom_event).unwrap();
        let newest_drop = input_callback_queue_plan_for_invocation(&custom_invocation, 1).unwrap();
        let dropped = input_callback_session_lifecycle_summary(
            &tcp_session,
            &newest_drop,
            Some(RunStatus::Halted),
            1,
            1,
        );
        assert_eq!(
            dropped.lifecycle_label,
            "endpoint=tcp://board-vm.local:4170 callback=arduino-uno-r4-wifi:D3 sequence=77 queued=false dispatch_required=false terminal=true"
        );
        assert_eq!(
            dropped.queue_summary.action,
            LanguageInputCallbackQueueAction::DropNewest
        );
        assert!(!dropped.queued);
        assert!(!dropped.dispatch_required);
        assert!(dropped.terminal);
        assert!(!dropped.retryable);
        assert!(dropped.dispatch_summary.is_none());
        assert!(dropped.result.is_none());
        assert!(dropped.completion_summary.is_none());
        assert_eq!(
            dropped.message,
            "Input callback was not queued; lifecycle ended before dispatch."
        );
    }

    #[test]
    fn input_callback_transport_actions_are_owned_by_rust_language_core() {
        let plan = input_callback_plan_for_target("uno-r4-wifi", 3, 7, 64).unwrap();
        let event = input_callback_event_for_plan(&plan, LanguageInputCallbackLevel::Low, 42, 9001);
        let invocation = input_callback_invocation_for_event(&plan, &event).unwrap();
        let queue_plan = input_callback_queue_plan_for_invocation(&invocation, 2).unwrap();
        let serial_session = host_endpoint_session_summary("serial:///dev/cu.usbmodem1101", 57_600)
            .expect("serial endpoint session");
        let completed_lifecycle = input_callback_session_lifecycle_summary(
            &serial_session,
            &queue_plan,
            Some(RunStatus::Halted),
            11,
            3,
        );
        let completed = input_callback_transport_action_summary(&completed_lifecycle);

        assert_eq!(completed.endpoint.endpoint, "serial:///dev/cu.usbmodem1101");
        assert_eq!(
            completed.endpoint.endpoint_transport,
            LanguageHostEndpointTransport::SerialPort
        );
        assert_eq!(
            completed.connection_label,
            "endpoint=serial:///dev/cu.usbmodem1101 baud=57600"
        );
        assert_eq!(
            completed.action,
            LanguageInputCallbackTransportAction::CompleteCallback
        );
        assert_eq!(completed.action_name, "complete_callback");
        assert_eq!(
            completed.action_label,
            "endpoint=serial:///dev/cu.usbmodem1101 baud=57600 callback=arduino-uno-r4-wifi:D3 sequence=42 transport_action=complete_callback terminal=true retryable=false"
        );
        assert!(completed.queued);
        assert!(completed.dispatch_required);
        assert!(completed.terminal);
        assert!(!completed.retryable);
        assert_eq!(completed.queue_depth_after, 3);
        assert_eq!(completed.queue_depth_after_completion, Some(2));
        assert_eq!(
            completed.message,
            "Transport should report completion and remove the callback."
        );
        assert_eq!(completed.lifecycle_summary, completed_lifecycle);

        let tcp_session = host_endpoint_session_summary("tcp://board-vm.local:4170", 57_600)
            .expect("tcp endpoint session");
        let pending_lifecycle =
            input_callback_session_lifecycle_summary(&tcp_session, &queue_plan, None, 0, 0);
        let pending = input_callback_transport_action_summary(&pending_lifecycle);
        assert_eq!(
            pending.action,
            LanguageInputCallbackTransportAction::DispatchCallback
        );
        assert_eq!(pending.action_name, "dispatch_callback");
        assert_eq!(
            pending.action_label,
            "endpoint=tcp://board-vm.local:4170 callback=arduino-uno-r4-wifi:D3 sequence=42 transport_action=dispatch_callback terminal=false retryable=false"
        );
        assert!(pending.queued);
        assert!(pending.dispatch_required);
        assert!(!pending.terminal);
        assert!(!pending.retryable);
        assert_eq!(pending.queue_depth_after, 3);
        assert_eq!(pending.queue_depth_after_completion, None);
        assert_eq!(
            pending.message,
            "Transport should dispatch the queued input callback."
        );

        let running_lifecycle = input_callback_session_lifecycle_summary(
            &tcp_session,
            &queue_plan,
            Some(RunStatus::Running),
            12,
            4,
        );
        let running = input_callback_transport_action_summary(&running_lifecycle);
        assert_eq!(
            running.action,
            LanguageInputCallbackTransportAction::KeepCallbackRunning
        );
        assert_eq!(running.action_name, "keep_callback_running");
        assert!(!running.terminal);
        assert!(running.retryable);
        assert_eq!(running.queue_depth_after_completion, Some(3));
        assert_eq!(
            running.message,
            "Transport should keep the callback scheduled for cooperative dispatch."
        );

        let budget_lifecycle = input_callback_session_lifecycle_summary(
            &tcp_session,
            &queue_plan,
            Some(RunStatus::BudgetExceeded),
            64,
            9,
        );
        let budget = input_callback_transport_action_summary(&budget_lifecycle);
        assert_eq!(
            budget.action,
            LanguageInputCallbackTransportAction::DropAfterBudgetExceeded
        );
        assert_eq!(budget.action_name, "drop_after_budget_exceeded");
        assert!(budget.terminal);
        assert!(!budget.retryable);
        assert_eq!(budget.queue_depth_after_completion, Some(2));
        assert_eq!(
            budget.message,
            "Transport should report the budget exhaustion and remove the callback."
        );

        let stopped_lifecycle = input_callback_session_lifecycle_summary(
            &tcp_session,
            &queue_plan,
            Some(RunStatus::Stopped),
            6,
            2,
        );
        let stopped = input_callback_transport_action_summary(&stopped_lifecycle);
        assert_eq!(
            stopped.action,
            LanguageInputCallbackTransportAction::DropAfterFailure
        );
        assert_eq!(stopped.action_name, "drop_after_failure");
        assert!(stopped.terminal);
        assert!(!stopped.retryable);
        assert_eq!(
            stopped.message,
            "Transport should report the callback failure and remove the callback."
        );

        let custom = input_callback_plan_with_options_for_target(
            "uno-r4-wifi",
            3,
            LanguageInputCallbackOptions {
                trigger: LanguageInputCallbackTrigger::RisingEdge,
                pull: LanguageInputCallbackPull::Floating,
                debounce_ms: 5,
                queue_capacity: 1,
                queue_policy: LanguageInputCallbackQueuePolicy::DropNewest,
                callback_program_id: 9,
                callback_instruction_budget: 32,
            },
        )
        .unwrap();
        let custom_event =
            input_callback_event_for_plan(&custom, LanguageInputCallbackLevel::High, 77, 12_345);
        let custom_invocation =
            input_callback_invocation_for_event(&custom, &custom_event).unwrap();
        let newest_drop = input_callback_queue_plan_for_invocation(&custom_invocation, 1).unwrap();
        let dropped_lifecycle =
            input_callback_session_lifecycle_summary(&tcp_session, &newest_drop, None, 0, 0);
        let dropped = input_callback_transport_action_summary(&dropped_lifecycle);
        assert_eq!(
            dropped.action,
            LanguageInputCallbackTransportAction::DropBeforeDispatch
        );
        assert_eq!(dropped.action_name, "drop_before_dispatch");
        assert_eq!(
            dropped.action_label,
            "endpoint=tcp://board-vm.local:4170 callback=arduino-uno-r4-wifi:D3 sequence=77 transport_action=drop_before_dispatch terminal=true retryable=false"
        );
        assert!(!dropped.queued);
        assert!(!dropped.dispatch_required);
        assert!(dropped.terminal);
        assert!(!dropped.retryable);
        assert_eq!(dropped.queue_depth_after, 1);
        assert_eq!(dropped.queue_depth_after_completion, None);
        assert_eq!(
            dropped.message,
            "Transport should report the dropped input callback without dispatch."
        );
    }

    #[test]
    fn input_callback_transport_effects_are_owned_by_rust_language_core() {
        let plan = input_callback_plan_for_target("uno-r4-wifi", 3, 7, 64).unwrap();
        let event = input_callback_event_for_plan(&plan, LanguageInputCallbackLevel::Low, 42, 9001);
        let invocation = input_callback_invocation_for_event(&plan, &event).unwrap();
        let queue_plan = input_callback_queue_plan_for_invocation(&invocation, 2).unwrap();
        let serial_session = host_endpoint_session_summary("serial:///dev/cu.usbmodem1101", 57_600)
            .expect("serial endpoint session");
        let completed_lifecycle = input_callback_session_lifecycle_summary(
            &serial_session,
            &queue_plan,
            Some(RunStatus::Halted),
            11,
            3,
        );
        let completed_action = input_callback_transport_action_summary(&completed_lifecycle);
        let completed = input_callback_transport_effect_summary(&completed_action);

        assert_eq!(completed.endpoint.endpoint, "serial:///dev/cu.usbmodem1101");
        assert_eq!(
            completed.connection_label,
            "endpoint=serial:///dev/cu.usbmodem1101 baud=57600"
        );
        assert_eq!(
            completed.action,
            LanguageInputCallbackTransportAction::CompleteCallback
        );
        assert_eq!(completed.action_name, "complete_callback");
        assert_eq!(
            completed.effect_label,
            "endpoint=serial:///dev/cu.usbmodem1101 baud=57600 callback=arduino-uno-r4-wifi:D3 sequence=42 transport_action=complete_callback terminal=true retryable=false dispatch_callback=false emit_drop=false emit_result=true remove_from_queue=true keep_dispatch_scheduled=false queue_depth_after_effect=2"
        );
        assert!(!completed.dispatch_callback);
        assert!(!completed.emit_drop);
        assert!(completed.emit_result);
        assert!(completed.remove_from_queue);
        assert!(!completed.keep_dispatch_scheduled);
        assert!(completed.terminal);
        assert!(!completed.retryable);
        assert_eq!(completed.queue_depth_after_effect, 2);
        assert_eq!(
            completed.message,
            "Transport should emit the result and remove the completed callback from the queue."
        );
        assert_eq!(completed.action_summary, completed_action);

        let tcp_session = host_endpoint_session_summary("tcp://board-vm.local:4170", 57_600)
            .expect("tcp endpoint session");
        let pending_lifecycle =
            input_callback_session_lifecycle_summary(&tcp_session, &queue_plan, None, 0, 0);
        let pending_action = input_callback_transport_action_summary(&pending_lifecycle);
        let pending = input_callback_transport_effect_summary(&pending_action);
        assert_eq!(
            pending.action,
            LanguageInputCallbackTransportAction::DispatchCallback
        );
        assert_eq!(pending.action_name, "dispatch_callback");
        assert!(pending.dispatch_callback);
        assert!(!pending.emit_drop);
        assert!(!pending.emit_result);
        assert!(!pending.remove_from_queue);
        assert!(pending.keep_dispatch_scheduled);
        assert!(!pending.terminal);
        assert!(!pending.retryable);
        assert_eq!(pending.queue_depth_after_effect, 3);
        assert_eq!(
            pending.message,
            "Transport should dispatch the queued callback and keep dispatch scheduled."
        );

        let running_lifecycle = input_callback_session_lifecycle_summary(
            &tcp_session,
            &queue_plan,
            Some(RunStatus::Running),
            12,
            4,
        );
        let running_action = input_callback_transport_action_summary(&running_lifecycle);
        let running = input_callback_transport_effect_summary(&running_action);
        assert_eq!(
            running.action,
            LanguageInputCallbackTransportAction::KeepCallbackRunning
        );
        assert!(!running.dispatch_callback);
        assert!(!running.emit_drop);
        assert!(running.emit_result);
        assert!(!running.remove_from_queue);
        assert!(running.keep_dispatch_scheduled);
        assert!(!running.terminal);
        assert!(running.retryable);
        assert_eq!(running.queue_depth_after_effect, 3);
        assert_eq!(
            running.message,
            "Transport should emit the running result and keep dispatch scheduled."
        );

        let budget_lifecycle = input_callback_session_lifecycle_summary(
            &tcp_session,
            &queue_plan,
            Some(RunStatus::BudgetExceeded),
            64,
            9,
        );
        let budget_action = input_callback_transport_action_summary(&budget_lifecycle);
        let budget = input_callback_transport_effect_summary(&budget_action);
        assert_eq!(
            budget.action,
            LanguageInputCallbackTransportAction::DropAfterBudgetExceeded
        );
        assert!(!budget.dispatch_callback);
        assert!(!budget.emit_drop);
        assert!(budget.emit_result);
        assert!(budget.remove_from_queue);
        assert!(!budget.keep_dispatch_scheduled);
        assert!(budget.terminal);
        assert!(!budget.retryable);
        assert_eq!(budget.queue_depth_after_effect, 2);
        assert_eq!(
            budget.message,
            "Transport should emit the budget result and remove the callback from the queue."
        );

        let stopped_lifecycle = input_callback_session_lifecycle_summary(
            &tcp_session,
            &queue_plan,
            Some(RunStatus::Stopped),
            6,
            2,
        );
        let stopped_action = input_callback_transport_action_summary(&stopped_lifecycle);
        let stopped = input_callback_transport_effect_summary(&stopped_action);
        assert_eq!(
            stopped.action,
            LanguageInputCallbackTransportAction::DropAfterFailure
        );
        assert!(stopped.emit_result);
        assert!(stopped.remove_from_queue);
        assert_eq!(stopped.queue_depth_after_effect, 2);
        assert_eq!(
            stopped.message,
            "Transport should emit the failure result and remove the callback from the queue."
        );

        let custom = input_callback_plan_with_options_for_target(
            "uno-r4-wifi",
            3,
            LanguageInputCallbackOptions {
                trigger: LanguageInputCallbackTrigger::RisingEdge,
                pull: LanguageInputCallbackPull::Floating,
                debounce_ms: 5,
                queue_capacity: 1,
                queue_policy: LanguageInputCallbackQueuePolicy::DropNewest,
                callback_program_id: 9,
                callback_instruction_budget: 32,
            },
        )
        .unwrap();
        let custom_event =
            input_callback_event_for_plan(&custom, LanguageInputCallbackLevel::High, 77, 12_345);
        let custom_invocation =
            input_callback_invocation_for_event(&custom, &custom_event).unwrap();
        let newest_drop = input_callback_queue_plan_for_invocation(&custom_invocation, 1).unwrap();
        let dropped_lifecycle =
            input_callback_session_lifecycle_summary(&tcp_session, &newest_drop, None, 0, 0);
        let dropped_action = input_callback_transport_action_summary(&dropped_lifecycle);
        let dropped = input_callback_transport_effect_summary(&dropped_action);
        assert_eq!(
            dropped.action,
            LanguageInputCallbackTransportAction::DropBeforeDispatch
        );
        assert_eq!(
            dropped.effect_label,
            "endpoint=tcp://board-vm.local:4170 callback=arduino-uno-r4-wifi:D3 sequence=77 transport_action=drop_before_dispatch terminal=true retryable=false dispatch_callback=false emit_drop=true emit_result=false remove_from_queue=false keep_dispatch_scheduled=false queue_depth_after_effect=1"
        );
        assert!(!dropped.dispatch_callback);
        assert!(dropped.emit_drop);
        assert!(!dropped.emit_result);
        assert!(!dropped.remove_from_queue);
        assert!(!dropped.keep_dispatch_scheduled);
        assert!(dropped.terminal);
        assert!(!dropped.retryable);
        assert_eq!(dropped.queue_depth_after_effect, 1);
        assert_eq!(
            dropped.message,
            "Transport should emit a drop notice without touching the existing queue."
        );
    }

    #[test]
    fn input_callback_transport_reports_are_owned_by_rust_language_core() {
        let plan = input_callback_plan_for_target("uno-r4-wifi", 3, 7, 64).unwrap();
        let event = input_callback_event_for_plan(&plan, LanguageInputCallbackLevel::Low, 42, 9001);
        let invocation = input_callback_invocation_for_event(&plan, &event).unwrap();
        let queue_plan = input_callback_queue_plan_for_invocation(&invocation, 2).unwrap();
        let serial_session = host_endpoint_session_summary("serial:///dev/cu.usbmodem1101", 57_600)
            .expect("serial endpoint session");
        let completed_lifecycle = input_callback_session_lifecycle_summary(
            &serial_session,
            &queue_plan,
            Some(RunStatus::Halted),
            11,
            3,
        );
        let completed_action = input_callback_transport_action_summary(&completed_lifecycle);
        let completed_effect = input_callback_transport_effect_summary(&completed_action);
        let completed = input_callback_transport_report_summary(&completed_effect);

        assert_eq!(completed.endpoint.endpoint, "serial:///dev/cu.usbmodem1101");
        assert_eq!(
            completed.connection_label,
            "endpoint=serial:///dev/cu.usbmodem1101 baud=57600"
        );
        assert_eq!(
            completed.report_kind,
            LanguageInputCallbackTransportReportKind::Completion
        );
        assert_eq!(completed.report_name, "completion");
        assert_eq!(
            completed.action,
            LanguageInputCallbackTransportAction::CompleteCallback
        );
        assert_eq!(completed.action_name, "complete_callback");
        assert!(!completed.dispatch_callback);
        assert!(completed.emit_report);
        assert!(!completed.emit_drop);
        assert!(completed.emit_result);
        assert!(completed.remove_from_queue);
        assert!(!completed.keep_dispatch_scheduled);
        assert!(completed.terminal);
        assert!(!completed.retryable);
        assert_eq!(completed.queue_depth_after_report, 2);
        assert_eq!(
            completed.message,
            "Transport should emit a completed-callback report."
        );
        assert_eq!(completed.effect_summary, completed_effect);

        let tcp_session = host_endpoint_session_summary("tcp://board-vm.local:4170", 57_600)
            .expect("tcp endpoint session");
        let pending_lifecycle =
            input_callback_session_lifecycle_summary(&tcp_session, &queue_plan, None, 0, 0);
        let pending_action = input_callback_transport_action_summary(&pending_lifecycle);
        let pending_effect = input_callback_transport_effect_summary(&pending_action);
        let pending = input_callback_transport_report_summary(&pending_effect);
        assert_eq!(
            pending.report_kind,
            LanguageInputCallbackTransportReportKind::Dispatch
        );
        assert_eq!(pending.report_name, "dispatch");
        assert!(pending.dispatch_callback);
        assert!(!pending.emit_report);
        assert!(!pending.emit_drop);
        assert!(!pending.emit_result);
        assert!(!pending.remove_from_queue);
        assert!(pending.keep_dispatch_scheduled);
        assert!(!pending.terminal);
        assert!(!pending.retryable);
        assert_eq!(pending.queue_depth_after_report, 3);
        assert_eq!(
            pending.report_label,
            "endpoint=tcp://board-vm.local:4170 callback=arduino-uno-r4-wifi:D3 sequence=42 transport_action=dispatch_callback terminal=false retryable=false dispatch_callback=true emit_drop=false emit_result=false remove_from_queue=false keep_dispatch_scheduled=true queue_depth_after_effect=3 transport_report=dispatch emit_report=false queue_depth_after_report=3"
        );
        assert_eq!(
            pending.message,
            "Transport should dispatch the callback runner; no report is emitted yet."
        );

        let running_lifecycle = input_callback_session_lifecycle_summary(
            &tcp_session,
            &queue_plan,
            Some(RunStatus::Running),
            12,
            4,
        );
        let running_action = input_callback_transport_action_summary(&running_lifecycle);
        let running_effect = input_callback_transport_effect_summary(&running_action);
        let running = input_callback_transport_report_summary(&running_effect);
        assert_eq!(
            running.report_kind,
            LanguageInputCallbackTransportReportKind::Running
        );
        assert_eq!(running.report_name, "running");
        assert!(running.emit_report);
        assert!(running.emit_result);
        assert!(!running.remove_from_queue);
        assert!(running.keep_dispatch_scheduled);
        assert!(running.retryable);
        assert_eq!(
            running.message,
            "Transport should emit a running-callback report and keep dispatch scheduled."
        );

        let budget_lifecycle = input_callback_session_lifecycle_summary(
            &tcp_session,
            &queue_plan,
            Some(RunStatus::BudgetExceeded),
            64,
            9,
        );
        let budget_action = input_callback_transport_action_summary(&budget_lifecycle);
        let budget_effect = input_callback_transport_effect_summary(&budget_action);
        let budget = input_callback_transport_report_summary(&budget_effect);
        assert_eq!(
            budget.report_kind,
            LanguageInputCallbackTransportReportKind::BudgetExceeded
        );
        assert_eq!(budget.report_name, "budget_exceeded");
        assert!(budget.emit_report);
        assert!(budget.emit_result);
        assert!(budget.remove_from_queue);
        assert!(budget.terminal);
        assert_eq!(
            budget.message,
            "Transport should emit a budget-exceeded callback report."
        );

        let stopped_lifecycle = input_callback_session_lifecycle_summary(
            &tcp_session,
            &queue_plan,
            Some(RunStatus::Stopped),
            6,
            2,
        );
        let stopped_action = input_callback_transport_action_summary(&stopped_lifecycle);
        let stopped_effect = input_callback_transport_effect_summary(&stopped_action);
        let stopped = input_callback_transport_report_summary(&stopped_effect);
        assert_eq!(
            stopped.report_kind,
            LanguageInputCallbackTransportReportKind::Failure
        );
        assert_eq!(stopped.report_name, "failure");
        assert!(stopped.emit_report);
        assert!(stopped.emit_result);
        assert!(stopped.remove_from_queue);
        assert_eq!(
            stopped.message,
            "Transport should emit a failed-callback report."
        );

        let custom = input_callback_plan_with_options_for_target(
            "uno-r4-wifi",
            3,
            LanguageInputCallbackOptions {
                trigger: LanguageInputCallbackTrigger::RisingEdge,
                pull: LanguageInputCallbackPull::Floating,
                debounce_ms: 5,
                queue_capacity: 1,
                queue_policy: LanguageInputCallbackQueuePolicy::DropNewest,
                callback_program_id: 9,
                callback_instruction_budget: 32,
            },
        )
        .unwrap();
        let custom_event =
            input_callback_event_for_plan(&custom, LanguageInputCallbackLevel::High, 77, 12_345);
        let custom_invocation =
            input_callback_invocation_for_event(&custom, &custom_event).unwrap();
        let newest_drop = input_callback_queue_plan_for_invocation(&custom_invocation, 1).unwrap();
        let dropped_lifecycle =
            input_callback_session_lifecycle_summary(&tcp_session, &newest_drop, None, 0, 0);
        let dropped_action = input_callback_transport_action_summary(&dropped_lifecycle);
        let dropped_effect = input_callback_transport_effect_summary(&dropped_action);
        let dropped = input_callback_transport_report_summary(&dropped_effect);
        assert_eq!(
            dropped.report_kind,
            LanguageInputCallbackTransportReportKind::Drop
        );
        assert_eq!(dropped.report_name, "drop");
        assert!(!dropped.dispatch_callback);
        assert!(dropped.emit_report);
        assert!(dropped.emit_drop);
        assert!(!dropped.emit_result);
        assert!(!dropped.remove_from_queue);
        assert!(!dropped.keep_dispatch_scheduled);
        assert!(dropped.terminal);
        assert_eq!(dropped.queue_depth_after_report, 1);
        assert_eq!(
            dropped.report_label,
            "endpoint=tcp://board-vm.local:4170 callback=arduino-uno-r4-wifi:D3 sequence=77 transport_action=drop_before_dispatch terminal=true retryable=false dispatch_callback=false emit_drop=true emit_result=false remove_from_queue=false keep_dispatch_scheduled=false queue_depth_after_effect=1 transport_report=drop emit_report=true queue_depth_after_report=1"
        );
        assert_eq!(
            dropped.message,
            "Transport should emit a dropped-callback report."
        );
    }

    #[test]
    fn input_callback_transport_events_are_owned_by_rust_language_core() {
        let plan = input_callback_plan_for_target("uno-r4-wifi", 3, 7, 64).unwrap();
        let event = input_callback_event_for_plan(&plan, LanguageInputCallbackLevel::Low, 42, 9001);
        let invocation = input_callback_invocation_for_event(&plan, &event).unwrap();
        let queue_plan = input_callback_queue_plan_for_invocation(&invocation, 2).unwrap();
        let serial_session = host_endpoint_session_summary("serial:///dev/cu.usbmodem1101", 57_600)
            .expect("serial endpoint session");
        let completed_lifecycle = input_callback_session_lifecycle_summary(
            &serial_session,
            &queue_plan,
            Some(RunStatus::Halted),
            11,
            3,
        );
        let completed_action = input_callback_transport_action_summary(&completed_lifecycle);
        let completed_effect = input_callback_transport_effect_summary(&completed_action);
        let completed_report = input_callback_transport_report_summary(&completed_effect);
        let completed = input_callback_transport_event_summary(&completed_report);

        assert_eq!(completed.endpoint.endpoint, "serial:///dev/cu.usbmodem1101");
        assert_eq!(
            completed.connection_label,
            "endpoint=serial:///dev/cu.usbmodem1101 baud=57600"
        );
        assert_eq!(
            completed.event_kind,
            LanguageInputCallbackTransportEventKind::CallbackCompleted
        );
        assert_eq!(completed.event_name, "callback_completed");
        assert_eq!(
            completed.report_kind,
            LanguageInputCallbackTransportReportKind::Completion
        );
        assert_eq!(completed.report_name, "completion");
        assert_eq!(
            completed.action,
            LanguageInputCallbackTransportAction::CompleteCallback
        );
        assert_eq!(completed.action_name, "complete_callback");
        assert!(!completed.dispatch_callback);
        assert!(completed.emit_report);
        assert!(!completed.emit_drop);
        assert!(completed.emit_result);
        assert!(completed.remove_from_queue);
        assert!(!completed.keep_dispatch_scheduled);
        assert!(completed.terminal);
        assert!(!completed.retryable);
        assert_eq!(completed.queue_depth_after_event, 2);
        assert_eq!(
            completed.message,
            "Adapter should emit a completed input callback event."
        );
        assert_eq!(completed.report_summary, completed_report);

        let tcp_session = host_endpoint_session_summary("tcp://board-vm.local:4170", 57_600)
            .expect("tcp endpoint session");
        let pending_lifecycle =
            input_callback_session_lifecycle_summary(&tcp_session, &queue_plan, None, 0, 0);
        let pending_action = input_callback_transport_action_summary(&pending_lifecycle);
        let pending_effect = input_callback_transport_effect_summary(&pending_action);
        let pending_report = input_callback_transport_report_summary(&pending_effect);
        let pending = input_callback_transport_event_summary(&pending_report);
        assert_eq!(
            pending.event_kind,
            LanguageInputCallbackTransportEventKind::DispatchScheduled
        );
        assert_eq!(pending.event_name, "dispatch_scheduled");
        assert_eq!(
            pending.report_kind,
            LanguageInputCallbackTransportReportKind::Dispatch
        );
        assert!(pending.dispatch_callback);
        assert!(!pending.emit_report);
        assert!(!pending.emit_drop);
        assert!(!pending.emit_result);
        assert!(!pending.remove_from_queue);
        assert!(pending.keep_dispatch_scheduled);
        assert!(!pending.terminal);
        assert!(!pending.retryable);
        assert_eq!(pending.queue_depth_after_event, 3);
        assert_eq!(
            pending.event_label,
            "endpoint=tcp://board-vm.local:4170 callback=arduino-uno-r4-wifi:D3 sequence=42 transport_action=dispatch_callback terminal=false retryable=false dispatch_callback=true emit_drop=false emit_result=false remove_from_queue=false keep_dispatch_scheduled=true queue_depth_after_effect=3 transport_report=dispatch emit_report=false queue_depth_after_report=3 transport_event=dispatch_scheduled queue_depth_after_event=3"
        );
        assert_eq!(
            pending.message,
            "Adapter should schedule the callback dispatch runner."
        );

        let running_lifecycle = input_callback_session_lifecycle_summary(
            &tcp_session,
            &queue_plan,
            Some(RunStatus::Running),
            12,
            4,
        );
        let running_action = input_callback_transport_action_summary(&running_lifecycle);
        let running_effect = input_callback_transport_effect_summary(&running_action);
        let running_report = input_callback_transport_report_summary(&running_effect);
        let running = input_callback_transport_event_summary(&running_report);
        assert_eq!(
            running.event_kind,
            LanguageInputCallbackTransportEventKind::CallbackRunning
        );
        assert_eq!(running.event_name, "callback_running");
        assert!(running.emit_report);
        assert!(running.emit_result);
        assert!(!running.remove_from_queue);
        assert!(running.keep_dispatch_scheduled);
        assert!(running.retryable);
        assert_eq!(
            running.message,
            "Adapter should emit a running input callback event and keep dispatch scheduled."
        );

        let budget_lifecycle = input_callback_session_lifecycle_summary(
            &tcp_session,
            &queue_plan,
            Some(RunStatus::BudgetExceeded),
            64,
            9,
        );
        let budget_action = input_callback_transport_action_summary(&budget_lifecycle);
        let budget_effect = input_callback_transport_effect_summary(&budget_action);
        let budget_report = input_callback_transport_report_summary(&budget_effect);
        let budget = input_callback_transport_event_summary(&budget_report);
        assert_eq!(
            budget.event_kind,
            LanguageInputCallbackTransportEventKind::CallbackBudgetExceeded
        );
        assert_eq!(budget.event_name, "callback_budget_exceeded");
        assert!(budget.emit_report);
        assert!(budget.emit_result);
        assert!(budget.remove_from_queue);
        assert!(budget.terminal);
        assert_eq!(
            budget.message,
            "Adapter should emit a budget-exceeded input callback event."
        );

        let stopped_lifecycle = input_callback_session_lifecycle_summary(
            &tcp_session,
            &queue_plan,
            Some(RunStatus::Stopped),
            6,
            2,
        );
        let stopped_action = input_callback_transport_action_summary(&stopped_lifecycle);
        let stopped_effect = input_callback_transport_effect_summary(&stopped_action);
        let stopped_report = input_callback_transport_report_summary(&stopped_effect);
        let stopped = input_callback_transport_event_summary(&stopped_report);
        assert_eq!(
            stopped.event_kind,
            LanguageInputCallbackTransportEventKind::CallbackFailed
        );
        assert_eq!(stopped.event_name, "callback_failed");
        assert!(stopped.emit_report);
        assert!(stopped.emit_result);
        assert!(stopped.remove_from_queue);
        assert_eq!(
            stopped.message,
            "Adapter should emit a failed input callback event."
        );

        let custom = input_callback_plan_with_options_for_target(
            "uno-r4-wifi",
            3,
            LanguageInputCallbackOptions {
                trigger: LanguageInputCallbackTrigger::RisingEdge,
                pull: LanguageInputCallbackPull::Floating,
                debounce_ms: 5,
                queue_capacity: 1,
                queue_policy: LanguageInputCallbackQueuePolicy::DropNewest,
                callback_program_id: 9,
                callback_instruction_budget: 32,
            },
        )
        .unwrap();
        let custom_event =
            input_callback_event_for_plan(&custom, LanguageInputCallbackLevel::High, 77, 12_345);
        let custom_invocation =
            input_callback_invocation_for_event(&custom, &custom_event).unwrap();
        let newest_drop = input_callback_queue_plan_for_invocation(&custom_invocation, 1).unwrap();
        let dropped_lifecycle =
            input_callback_session_lifecycle_summary(&tcp_session, &newest_drop, None, 0, 0);
        let dropped_action = input_callback_transport_action_summary(&dropped_lifecycle);
        let dropped_effect = input_callback_transport_effect_summary(&dropped_action);
        let dropped_report = input_callback_transport_report_summary(&dropped_effect);
        let dropped = input_callback_transport_event_summary(&dropped_report);
        assert_eq!(
            dropped.event_kind,
            LanguageInputCallbackTransportEventKind::CallbackDropped
        );
        assert_eq!(dropped.event_name, "callback_dropped");
        assert_eq!(
            dropped.report_kind,
            LanguageInputCallbackTransportReportKind::Drop
        );
        assert!(!dropped.dispatch_callback);
        assert!(dropped.emit_report);
        assert!(dropped.emit_drop);
        assert!(!dropped.emit_result);
        assert!(!dropped.remove_from_queue);
        assert!(!dropped.keep_dispatch_scheduled);
        assert!(dropped.terminal);
        assert_eq!(dropped.queue_depth_after_event, 1);
        assert_eq!(
            dropped.event_label,
            "endpoint=tcp://board-vm.local:4170 callback=arduino-uno-r4-wifi:D3 sequence=77 transport_action=drop_before_dispatch terminal=true retryable=false dispatch_callback=false emit_drop=true emit_result=false remove_from_queue=false keep_dispatch_scheduled=false queue_depth_after_effect=1 transport_report=drop emit_report=true queue_depth_after_report=1 transport_event=callback_dropped queue_depth_after_event=1"
        );
        assert_eq!(
            dropped.message,
            "Adapter should emit a dropped input callback event."
        );
    }

    #[test]
    fn tcp_endpoint_metadata_is_owned_by_rust_language_core() {
        let endpoint = parse_tcp_endpoint("tcp://board-vm.local:4170").unwrap();
        assert_eq!(endpoint.endpoint, "tcp://board-vm.local:4170");
        assert_eq!(endpoint.transport, LanguageConnectionTransport::Wifi);
        assert_eq!(
            endpoint.endpoint_transport,
            LanguageHostEndpointTransport::TcpSocket
        );
        assert_eq!(endpoint.endpoint_scheme, "tcp");
        assert_eq!(endpoint.authority, "board-vm.local:4170");

        let bare_endpoint = parse_tcp_endpoint("127.0.0.1:4170").unwrap();
        assert_eq!(bare_endpoint.endpoint, "127.0.0.1:4170");
        assert_eq!(bare_endpoint.endpoint_scheme, "tcp");
        assert_eq!(bare_endpoint.authority, "127.0.0.1:4170");

        assert!(parse_tcp_endpoint("serial:///dev/cu.usbmodem1101").is_none());
        assert!(parse_tcp_endpoint("tcp://   ").is_none());
    }

    #[test]
    fn bluetooth_endpoint_metadata_is_owned_by_rust_language_core() {
        let ble = parse_bluetooth_endpoint("ble://uno-r4-wifi/180f/2a19/2a1a").unwrap();
        let rfcomm = parse_bluetooth_endpoint("btspp://ESP32-BoardVM:3").unwrap();

        assert_eq!(ble.transport, LanguageConnectionTransport::BluetoothLe);
        assert_eq!(
            ble.endpoint_transport,
            LanguageHostEndpointTransport::BluetoothLeGatt
        );
        assert_eq!(ble.endpoint_scheme, "ble");
        assert_eq!(ble.device, "uno-r4-wifi");
        assert_eq!(ble.service_uuid.as_deref(), Some("180f"));
        assert_eq!(ble.write_characteristic_uuid.as_deref(), Some("2a19"));
        assert_eq!(ble.notify_characteristic_uuid.as_deref(), Some("2a1a"));
        assert_eq!(ble.channel, None);

        assert_eq!(
            rfcomm.transport,
            LanguageConnectionTransport::BluetoothClassic
        );
        assert_eq!(
            rfcomm.endpoint_transport,
            LanguageHostEndpointTransport::BluetoothClassicRfcomm
        );
        assert_eq!(rfcomm.endpoint_scheme, "btspp");
        assert_eq!(rfcomm.device, "ESP32-BoardVM");
        assert_eq!(rfcomm.channel, Some(3));
        assert!(parse_bluetooth_endpoint("tcp://board-vm.local:4170").is_none());
    }

    #[test]
    fn host_endpoint_summary_classifies_all_supported_transports() {
        let serial = parse_host_endpoint("serial:///dev/cu.usbmodem1101").unwrap();
        let tcp = parse_host_endpoint("127.0.0.1:4170").unwrap();
        let ble = parse_host_endpoint("ble://uno-r4-wifi/180f/2a19/2a1a").unwrap();
        let rfcomm = parse_host_endpoint("rfcomm://ESP32-BoardVM:3").unwrap();

        assert_eq!(serial.transport, LanguageConnectionTransport::Serial);
        assert_eq!(
            serial.endpoint_transport,
            LanguageHostEndpointTransport::SerialPort
        );
        assert_eq!(serial.endpoint_scheme, "serial");

        assert_eq!(tcp.transport, LanguageConnectionTransport::Wifi);
        assert_eq!(
            tcp.endpoint_transport,
            LanguageHostEndpointTransport::TcpSocket
        );
        assert_eq!(tcp.endpoint_scheme, "tcp");

        assert_eq!(ble.transport, LanguageConnectionTransport::BluetoothLe);
        assert_eq!(
            ble.endpoint_transport,
            LanguageHostEndpointTransport::BluetoothLeGatt
        );
        assert_eq!(ble.endpoint_scheme, "ble");

        assert_eq!(
            rfcomm.transport,
            LanguageConnectionTransport::BluetoothClassic
        );
        assert_eq!(
            rfcomm.endpoint_transport,
            LanguageHostEndpointTransport::BluetoothClassicRfcomm
        );
        assert_eq!(rfcomm.endpoint_scheme, "rfcomm");

        assert!(parse_host_endpoint("serial://   ").is_none());
        assert!(parse_host_endpoint("tcp://   ").is_none());
        assert!(parse_host_endpoint("ble://").is_none());
        assert!(parse_host_endpoint("ws://board-vm.local:4170").is_none());

        assert_eq!(
            parse_host_endpoint_with_error("tcp://board-vm.local:4170")
                .unwrap()
                .endpoint_transport,
            LanguageHostEndpointTransport::TcpSocket
        );

        assert_eq!(
            host_endpoint_connection_label(&serial, 57_600),
            "endpoint=serial:///dev/cu.usbmodem1101 baud=57600"
        );
        assert_eq!(
            host_endpoint_connection_label(&tcp, 57_600),
            "endpoint=127.0.0.1:4170"
        );
        assert_eq!(
            host_endpoint_connection_label(&ble, 57_600),
            "endpoint=ble://uno-r4-wifi/180f/2a19/2a1a"
        );
        assert_eq!(
            host_endpoint_connection_label(&rfcomm, 57_600),
            "endpoint=rfcomm://ESP32-BoardVM:3"
        );

        let session = host_endpoint_session_summary("serial:///dev/cu.usbmodem1101", 57_600)
            .expect("serial endpoint session summary");
        assert_eq!(
            session.endpoint.endpoint_transport,
            LanguageHostEndpointTransport::SerialPort
        );
        assert_eq!(
            session.connection_label,
            "endpoint=serial:///dev/cu.usbmodem1101 baud=57600"
        );
        let session = host_endpoint_session_summary("tcp://board-vm.local:4170", 57_600)
            .expect("tcp endpoint session summary");
        assert_eq!(
            session.endpoint.endpoint_transport,
            LanguageHostEndpointTransport::TcpSocket
        );
        assert_eq!(
            session.connection_label,
            "endpoint=tcp://board-vm.local:4170"
        );
    }

    #[test]
    fn host_endpoint_parse_errors_are_classified_by_rust_language_core() {
        let serial = parse_host_endpoint_with_error("serial://   ").unwrap_err();
        let tcp = parse_host_endpoint_with_error("tcp://   ").unwrap_err();
        let bare_tcp = parse_host_endpoint_with_error("").unwrap_err();
        let bluetooth = parse_host_endpoint_with_error("ble://").unwrap_err();
        let unsupported = parse_host_endpoint_with_error("ws://board-vm.local:4170").unwrap_err();

        assert_eq!(
            serial.kind,
            LanguageHostEndpointParseErrorKind::InvalidSerialEndpoint
        );
        assert_eq!(serial.scheme.as_deref(), Some("serial"));
        assert_eq!(
            tcp.kind,
            LanguageHostEndpointParseErrorKind::InvalidTcpEndpoint
        );
        assert_eq!(tcp.scheme.as_deref(), Some("tcp"));
        assert_eq!(
            bare_tcp.kind,
            LanguageHostEndpointParseErrorKind::InvalidTcpEndpoint
        );
        assert_eq!(bare_tcp.scheme, None);
        assert_eq!(
            bluetooth.kind,
            LanguageHostEndpointParseErrorKind::InvalidBluetoothEndpoint
        );
        assert_eq!(bluetooth.scheme.as_deref(), Some("ble"));
        assert_eq!(
            unsupported.kind,
            LanguageHostEndpointParseErrorKind::UnsupportedScheme
        );
        assert_eq!(unsupported.scheme.as_deref(), Some("ws"));
    }

    #[test]
    fn bluetooth_endpoint_candidates_are_planned_by_rust_language_core() {
        let devices = vec![
            LanguageBluetoothDiscoveredDevice {
                id: "esp32-board-vm".to_owned(),
                name: Some("ESP32 Board VM".to_owned()),
                address: None,
                paired: true,
                service_uuids: vec![],
                characteristic_uuids: vec![],
                board_vm_rfcomm_channels: vec![3, 3, 31],
            },
            LanguageBluetoothDiscoveredDevice {
                id: "uno-r4".to_owned(),
                name: Some("Uno R4 Board VM".to_owned()),
                address: Some("AA:BB:CC:DD:EE:FF".to_owned()),
                paired: false,
                service_uuids: vec!["6E400001-B5A3-F393-E0A9-E50E24DCCA9E".to_owned()],
                characteristic_uuids: vec![],
                board_vm_rfcomm_channels: vec![],
            },
        ];

        let candidates = bluetooth_endpoint_candidates_from_devices(&devices);

        assert_eq!(candidates.len(), 2);
        let rfcomm = candidates
            .iter()
            .find(|candidate| candidate.endpoint.channel == Some(3))
            .unwrap();
        assert_eq!(rfcomm.display_name, "ESP32 Board VM");
        assert_eq!(
            rfcomm.endpoint.endpoint_transport,
            LanguageHostEndpointTransport::BluetoothClassicRfcomm
        );
        assert_eq!(rfcomm.endpoint.endpoint, "btspp://esp32-board-vm:3");
        assert!(rfcomm.paired);
        assert!(!rfcomm.requires_pairing);

        let ble = candidates
            .iter()
            .find(|candidate| candidate.endpoint.service_uuid.is_some())
            .unwrap();
        assert_eq!(ble.display_name, "Uno R4 Board VM");
        assert_eq!(
            ble.endpoint.endpoint_transport,
            LanguageHostEndpointTransport::BluetoothLeGatt
        );
        assert_eq!(ble.device, "AA:BB:CC:DD:EE:FF");
        assert_eq!(
            ble.endpoint.service_uuid.as_deref(),
            Some("6e400001-b5a3-f393-e0a9-e50e24dcca9e")
        );
        assert!(!ble.paired);
        assert!(ble.requires_pairing);
    }

    #[test]
    fn bluetooth_backend_open_plan_is_owned_by_rust_language_core() {
        let plan = bluetooth_backend_open_plan_from_rfcomm_paths(
            "btspp://ESP32-BoardVM:3",
            ["/dev/tty.ESP32-BoardVM", "/dev/cu.ESP32-BoardVM"],
        )
        .unwrap();

        assert_eq!(plan.backend, "macos_rfcomm");
        assert_eq!(plan.status, "ready");
        assert_eq!(
            plan.endpoint.endpoint_transport,
            LanguageHostEndpointTransport::BluetoothClassicRfcomm
        );
        assert_eq!(plan.endpoint.endpoint, "btspp://ESP32-BoardVM:3");
        assert_eq!(plan.stream_path.as_deref(), Some("/dev/cu.ESP32-BoardVM"));
        assert!(!plan.native_transport);
        assert_eq!(plan.message, None);

        let missing = bluetooth_backend_open_plan_from_rfcomm_paths(
            "btspp://ESP32-BoardVM:3",
            ["/dev/cu.NotBoardVM"],
        )
        .unwrap();
        assert_eq!(missing.backend, "macos_rfcomm");
        assert_eq!(missing.status, "not_found");
        assert_eq!(missing.stream_path, None);
        assert!(!missing.native_transport);
        assert!(missing
            .message
            .as_deref()
            .unwrap()
            .contains("no macOS RFCOMM serial device found"));

        let ble = bluetooth_backend_open_plan_from_rfcomm_paths(
            "ble://uno-r4-wifi/180f/2a19/2a1a",
            ["/dev/cu.ESP32-BoardVM"],
        )
        .unwrap();
        assert_eq!(
            ble.endpoint.endpoint_transport,
            LanguageHostEndpointTransport::BluetoothLeGatt
        );
        assert!(ble.stream_path.is_none());
        #[cfg(target_os = "macos")]
        {
            assert_eq!(ble.backend, "macos_core_bluetooth");
            assert_eq!(ble.status, "ready");
            assert!(ble.native_transport);
            assert!(ble.message.is_none());
        }
        #[cfg(not(target_os = "macos"))]
        {
            assert_eq!(ble.status, "unsupported");
            assert!(!ble.native_transport);
            assert!(ble.message.as_deref().unwrap().contains("unsupported"));
        }
    }

    #[test]
    fn bluetooth_transact_wire_frame_is_owned_by_rust_language_core() {
        struct FakeBleLink {
            response: Vec<u8>,
            writes: Vec<Vec<u8>>,
        }

        impl BleGattIo for FakeBleLink {
            fn write_characteristic(
                &mut self,
                _characteristic_uuid: &str,
                bytes: &[u8],
            ) -> Result<(), board_vm_bluetooth::BluetoothTransportError> {
                self.writes.push(bytes.to_vec());
                Ok(())
            }

            fn read_notification(
                &mut self,
                _characteristic_uuid: &str,
                out: &mut [u8],
            ) -> Result<usize, board_vm_bluetooth::BluetoothTransportError> {
                let len = self.response.len();
                out[..len].copy_from_slice(&self.response);
                Ok(len)
            }
        }

        struct FakeRfcommStream;

        impl Read for FakeRfcommStream {
            fn read(&mut self, _buf: &mut [u8]) -> std::io::Result<usize> {
                Ok(0)
            }
        }

        impl Write for FakeRfcommStream {
            fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
                Ok(buf.len())
            }

            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }

        struct FakeBackend {
            response: Vec<u8>,
            opened_device: Option<String>,
        }

        impl BluetoothBackend for FakeBackend {
            type BleGattLink = FakeBleLink;
            type RfcommStream = FakeRfcommStream;

            fn open_ble_gatt(
                &mut self,
                endpoint: &BleGattEndpoint,
            ) -> Result<Self::BleGattLink, BluetoothOpenError> {
                self.opened_device = Some(endpoint.device.clone());
                Ok(FakeBleLink {
                    response: self.response.clone(),
                    writes: Vec::new(),
                })
            }

            fn open_rfcomm(
                &mut self,
                _endpoint: &RfcommEndpoint,
            ) -> Result<Self::RfcommStream, BluetoothOpenError> {
                Err(BluetoothOpenError::UnsupportedPlatform { platform: "test" })
            }
        }

        let endpoint =
            parse_board_vm_bluetooth_endpoint("ble://uno-r4-wifi/180f/2a19/2a1a").unwrap();
        let mut wire_request = [0u8; 32];
        let wire_request_len = encode_wire_frame(b"request", &mut wire_request).unwrap();
        let mut wire_response = [0u8; 32];
        let wire_response_len = encode_wire_frame(b"response", &mut wire_response).unwrap();

        let mut backend = FakeBackend {
            response: wire_response[..wire_response_len].to_vec(),
            opened_device: None,
        };
        let mut out = [0u8; 32];

        let len = bluetooth_transact_wire_frame_with_backend::<_, 32>(
            &mut backend,
            endpoint,
            &wire_request[..wire_request_len],
            &mut out,
        )
        .unwrap();

        assert_eq!(&out[..len], &wire_response[..wire_response_len]);
        assert_eq!(backend.opened_device.as_deref(), Some("uno-r4-wifi"));
    }

    #[test]
    fn bluetooth_wire_smoke_sequence_runs_through_language_core_backend() {
        type SharedLoopbackBoard = Rc<RefCell<LoopbackBoard<512, 8, 8>>>;

        struct LoopbackBleLink {
            board: SharedLoopbackBoard,
            response: Vec<u8>,
            raw_request: [u8; 1024],
            board_payload: [u8; 1024],
            board_frame: [u8; 1024],
        }

        impl LoopbackBleLink {
            fn new(board: SharedLoopbackBoard) -> Self {
                Self {
                    board,
                    response: Vec::new(),
                    raw_request: [0; 1024],
                    board_payload: [0; 1024],
                    board_frame: [0; 1024],
                }
            }
        }

        impl BleGattIo for LoopbackBleLink {
            fn write_characteristic(
                &mut self,
                _characteristic_uuid: &str,
                bytes: &[u8],
            ) -> Result<(), BluetoothTransportError> {
                let raw_len = board_vm_protocol::decode_wire_frame(bytes, &mut self.raw_request)
                    .map_err(BluetoothTransportError::Protocol)?;
                let frame_len = self
                    .board
                    .borrow_mut()
                    .handle_raw_frame(
                        &self.raw_request[..raw_len],
                        &mut self.board_payload,
                        &mut self.board_frame,
                    )
                    .map_err(|_| BluetoothTransportError::Link)?;
                self.response.resize(1024, 0);
                let response_len =
                    encode_wire_frame(&self.board_frame[..frame_len], &mut self.response)
                        .map_err(BluetoothTransportError::Protocol)?;
                self.response.truncate(response_len);
                Ok(())
            }

            fn read_notification(
                &mut self,
                _characteristic_uuid: &str,
                out: &mut [u8],
            ) -> Result<usize, BluetoothTransportError> {
                if out.len() < self.response.len() {
                    return Err(BluetoothTransportError::FrameTooLarge);
                }
                let len = self.response.len();
                out[..len].copy_from_slice(&self.response);
                Ok(len)
            }
        }

        struct LoopbackRfcommStream;

        impl Read for LoopbackRfcommStream {
            fn read(&mut self, _buf: &mut [u8]) -> std::io::Result<usize> {
                Ok(0)
            }
        }

        impl Write for LoopbackRfcommStream {
            fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
                Ok(buf.len())
            }

            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }

        struct LoopbackBackend {
            board: SharedLoopbackBoard,
            opened_devices: Vec<String>,
        }

        impl BluetoothBackend for LoopbackBackend {
            type BleGattLink = LoopbackBleLink;
            type RfcommStream = LoopbackRfcommStream;

            fn open_ble_gatt(
                &mut self,
                endpoint: &BleGattEndpoint,
            ) -> Result<Self::BleGattLink, BluetoothOpenError> {
                self.opened_devices.push(endpoint.device.clone());
                Ok(LoopbackBleLink::new(Rc::clone(&self.board)))
            }

            fn open_rfcomm(
                &mut self,
                _endpoint: &RfcommEndpoint,
            ) -> Result<Self::RfcommStream, BluetoothOpenError> {
                Err(BluetoothOpenError::UnsupportedPlatform { platform: "test" })
            }
        }

        let endpoint =
            parse_board_vm_bluetooth_endpoint("ble://uno-r4-wifi/180f/2a19/2a1a").unwrap();
        let mut backend = LoopbackBackend {
            board: Rc::new(RefCell::new(LoopbackBoard::new())),
            opened_devices: Vec::new(),
        };
        let mut session = BoardVmLanguageSession::new();
        let mut wire = [0u8; 1024];
        let mut response_wire = [0u8; 1024];
        let mut decoded_raw = [0u8; 1024];
        let mut module = [0u8; BLINK_MODULE_LEN];
        let module_len = build_blink_module(
            BlinkProgram {
                pin: 13,
                high_ms: 250,
                low_ms: 250,
                max_stack: 4,
            },
            &mut module,
        )
        .unwrap();

        let mut exchange = |backend: &mut LoopbackBackend,
                            frame_wire: &[u8],
                            request_id: u16,
                            expected: MessageType|
         -> DecodedLanguageResponse {
            let len = bluetooth_transact_wire_frame_with_backend::<_, 1024>(
                backend,
                endpoint.clone(),
                frame_wire,
                &mut response_wire,
            )
            .unwrap();
            let decoded = decode_wire_response(&response_wire[..len], &mut decoded_raw).unwrap();
            assert_eq!(decoded.request_id, request_id);
            assert_eq!(decoded.message_type, expected);
            assert!(decoded.is_response());
            decoded
        };

        let hello =
            build_hello_wire_frame(&mut session, "language-smoke", 0xABCD_1234, &mut wire).unwrap();
        match exchange(
            &mut backend,
            &wire[..hello.len],
            hello.request_id,
            MessageType::HELLO_ACK,
        )
        .body
        {
            DecodedLanguageResponseBody::HelloAck(ack) => {
                assert_eq!(ack.board_name, LOOPBACK_BOARD_ID);
                assert_eq!(ack.runtime_name, LOOPBACK_RUNTIME_ID);
                assert_eq!(ack.host_nonce, 0xABCD_1234);
            }
            other => panic!("unexpected hello response body: {other:?}"),
        }

        let caps = build_caps_query_wire_frame(&mut session, &mut wire).unwrap();
        match exchange(
            &mut backend,
            &wire[..caps.len],
            caps.request_id,
            MessageType::CAPS_REPORT,
        )
        .body
        {
            DecodedLanguageResponseBody::CapsReport(report) => {
                assert_eq!(report.board_id, LOOPBACK_BOARD_ID);
                assert_eq!(report.runtime_id, LOOPBACK_RUNTIME_ID);
                assert_eq!(report.max_program_bytes, 512);
                assert!(report
                    .capabilities
                    .iter()
                    .any(|capability| capability.name == "program.ram_exec"));
            }
            other => panic!("unexpected caps response body: {other:?}"),
        }

        let begin =
            build_program_begin_wire_frame(&mut session, 9, &module[..module_len], &mut wire)
                .unwrap();
        match exchange(
            &mut backend,
            &wire[..begin.len],
            begin.request_id,
            MessageType::PROGRAM_BEGIN,
        )
        .body
        {
            DecodedLanguageResponseBody::ProgramBegin(begin) => {
                assert_eq!(begin.program_id, 9);
                assert_eq!(begin.total_len, module_len as u32);
            }
            other => panic!("unexpected program begin response body: {other:?}"),
        }

        let chunk =
            build_program_chunk_wire_frame(&mut session, 9, 0, &module[..module_len], &mut wire)
                .unwrap();
        match exchange(
            &mut backend,
            &wire[..chunk.len],
            chunk.request_id,
            MessageType::PROGRAM_CHUNK,
        )
        .body
        {
            DecodedLanguageResponseBody::ProgramChunk(chunk) => {
                assert_eq!(chunk.program_id, 9);
                assert_eq!(chunk.offset, 0);
                assert_eq!(chunk.len, module_len);
            }
            other => panic!("unexpected program chunk response body: {other:?}"),
        }

        let end = build_program_end_wire_frame(&mut session, 9, &mut wire).unwrap();
        match exchange(
            &mut backend,
            &wire[..end.len],
            end.request_id,
            MessageType::PROGRAM_END,
        )
        .body
        {
            DecodedLanguageResponseBody::ProgramEnd(end) => {
                assert_eq!(end.program_id, 9);
            }
            other => panic!("unexpected program end response body: {other:?}"),
        }

        let run = build_run_background_wire_frame(&mut session, 9, 200, &mut wire).unwrap();
        match exchange(
            &mut backend,
            &wire[..run.len],
            run.request_id,
            MessageType::RUN_REPORT,
        )
        .body
        {
            DecodedLanguageResponseBody::RunReport(report) => {
                assert_eq!(report.program_id, 9);
                assert_eq!(report.status, RunStatus::Running);
                assert!(report.instructions_executed > 0);
                assert_eq!(report.open_handles, 1);
                assert!(report.returns.is_empty());
            }
            other => panic!("unexpected run response body: {other:?}"),
        }

        assert_eq!(backend.opened_devices.len(), 6);
        assert!(backend
            .opened_devices
            .iter()
            .all(|device| device == "uno-r4-wifi"));
    }

    #[test]
    fn bluetooth_discovery_adapter_is_safe_for_language_frontends() {
        let devices = discover_bluetooth_devices();
        let candidates = discover_bluetooth_endpoint_candidates();

        for device in &devices {
            assert!(!device.id.trim().is_empty());
        }
        for candidate in &candidates {
            assert!(!candidate.endpoint.endpoint.trim().is_empty());
        }
    }

    #[test]
    fn rust_core_detects_targets_from_human_selectors() {
        assert_eq!(
            detect_target("UNO R4 WiFi").unwrap().board_id,
            "arduino-uno-r4-wifi"
        );
        assert_eq!(
            detect_target("arduino:renesas_uno:nanor4")
                .unwrap()
                .board_id,
            "arduino-nano-r4"
        );
        assert_eq!(detect_target("esp32").unwrap().board_id, "esp32-devkit-v1");
        assert_eq!(
            detect_target("pico-w").unwrap().board_id,
            "raspberry-pi-pico-w"
        );
        assert_eq!(
            detect_target("Raspberry Pi Pico").unwrap().rust_target,
            "thumbv6m-none-eabi"
        );
        assert_eq!(
            normalize_target_selector("ESP32 DevKit V1"),
            "esp32_devkit_v1"
        );
        assert!(detect_target("arduino:mbed_opta:opta").is_none());
        assert!(detect_target("definitely-not-a-board").is_none());
    }

    #[test]
    fn rust_core_resolves_upload_selectors_from_target_metadata() {
        let renesas_uno_targets = targets_for_upload_selector("arduino:renesas_uno");
        assert_eq!(renesas_uno_targets.len(), 3);
        assert!(renesas_uno_targets
            .iter()
            .any(|target| target.board_id == "arduino-uno-r4-wifi"));
        assert!(renesas_uno_targets
            .iter()
            .any(|target| target.board_id == "arduino-nano-r4"));

        let mega = targets_for_upload_selector("arduino:avr:mega:cpu=atmega2560");
        assert_eq!(mega.len(), 1);
        assert_eq!(mega[0].board_id, "arduino-mega-2560");

        let opta_targets = targets_for_upload_selector("arduino:mbed_opta:opta");
        assert_eq!(opta_targets.len(), 3);
        assert!(opta_targets
            .iter()
            .any(|target| target.board_id == "arduino-opta-lite"));
        assert!(opta_targets
            .iter()
            .any(|target| target.board_id == "arduino-opta-rs485"));
        assert!(opta_targets
            .iter()
            .any(|target| target.board_id == "arduino-opta-wifi"));

        assert!(targets_for_upload_selector("not-a-board").is_empty());
    }

    #[test]
    fn esp_upload_options_are_owned_by_rust_language_core() {
        let options = esp_upload_options_for_target("esp32").unwrap();

        assert_eq!(options.board_id, "esp32-devkit-v1");
        assert_eq!(options.baud_rate, ESP_DEFAULT_BAUD_RATE);
        assert_eq!(options.timeout_ms, ESP_DEFAULT_TIMEOUT_MS);
        assert!(options.reset_into_bootloader);
        assert_eq!(options.offset, LANGUAGE_ESP_DEFAULT_FLASH_OFFSET);
        assert_eq!(options.block_size, ESP_DEFAULT_FLASH_BLOCK_SIZE);
        assert_eq!(options.flash_size, Some(LANGUAGE_ESP_DEFAULT_FLASH_SIZE));
        assert!(options.verify_md5);
        assert!(!options.stay_in_bootloader);
        assert!(esp_upload_options_for_target("pico").is_none());
    }

    #[test]
    fn pico_uf2_upload_options_are_owned_by_rust_language_core() {
        let options = pico_uf2_upload_options_for_target("pico").unwrap();

        assert_eq!(options.board_id, "raspberry-pi-pico");
        assert_eq!(options.command, "pico-uf2");
        assert_eq!(options.volume_label, "RPI-RP2");
        assert_eq!(options.image_extension, ".uf2");
        assert!(options.auto_detect_mount);
        assert!(pico_uf2_upload_options_for_target("pico-w").is_some());
        assert!(pico_uf2_upload_options_for_target("esp32").is_none());
    }

    #[test]
    fn arduino_cli_upload_options_are_owned_by_rust_language_core() {
        let nano_r4 = arduino_cli_upload_options_for_target("arduino:renesas_uno:nanor4").unwrap();
        assert_eq!(nano_r4.board_id, "arduino-nano-r4");
        assert_eq!(nano_r4.command, "arduino-cli upload");
        assert_eq!(nano_r4.image_format, "arduino_cli_build_output");
        assert_eq!(nano_r4.transport, "serial");
        assert_eq!(nano_r4.reset_method, "arduino_board_package");
        assert_eq!(nano_r4.platform_id, "arduino:renesas_uno");
        assert_eq!(nano_r4.fqbn, "arduino:renesas_uno:nanor4");
        assert_eq!(nano_r4.port_hint, "native_usb");
        assert_eq!(nano_r4.port_selection_step, "select_native_usb_port");
        assert!(nano_r4.native_usb);
        assert!(!nano_r4.usb_serial_bridge);
        assert!(!nano_r4.external_serial_adapter);
        assert!(nano_r4.requires_serial_port);
        assert!(nano_r4.delegate_reset_to_board_package);

        let mega =
            arduino_cli_upload_options_for_target("arduino:avr:mega:cpu=atmega2560").unwrap();
        assert_eq!(mega.board_id, "arduino-mega-2560");
        assert_eq!(mega.platform_id, "arduino:avr");
        assert_eq!(mega.port_hint, "usb_serial_bridge");
        assert_eq!(mega.port_selection_step, "select_usb_serial_port");
        assert!(mega.usb_serial_bridge);
        assert!(!mega.native_usb);

        let pro_mini = arduino_cli_upload_options_for_target("arduino-pro-mini").unwrap();
        assert_eq!(pro_mini.port_hint, "external_serial_adapter");
        assert_eq!(
            pro_mini.port_selection_step,
            "select_external_serial_adapter"
        );
        assert!(pro_mini.external_serial_adapter);

        assert!(arduino_cli_upload_options_for_target("esp32").is_none());
        assert!(arduino_cli_upload_options_for_target("pico").is_none());
        assert!(arduino_cli_upload_options_for_target("not-a-board").is_none());
    }

    #[test]
    fn arduino_cli_port_discovery_is_owned_by_rust_language_core() {
        let nano_r4 = arduino_cli_port_discovery_for_target("arduino:renesas_uno:nanor4").unwrap();
        assert_eq!(nano_r4.board_id, "arduino-nano-r4");
        assert_eq!(nano_r4.port_hint, "native_usb");
        assert_eq!(nano_r4.port_selection_step, "select_native_usb_port");
        assert!(nano_r4.requires_serial_port);
        assert_eq!(
            nano_r4.bootloader_touch_baud,
            Some(ARDUINO_CLI_NATIVE_USB_BOOTLOADER_TOUCH_BAUD)
        );
        assert!(nano_r4.expects_port_reenumeration);
        assert!(nano_r4.wait_for_runtime_rediscovery);
        assert!(!nano_r4.serial_adapter_required);
        assert!(nano_r4.reset_delegated_to_board_package);
        assert!(nano_r4.notes.contains("runtime CDC port"));
        assert!(nano_r4.notes.contains("rediscovery"));

        let mega =
            arduino_cli_port_discovery_for_target("arduino:avr:mega:cpu=atmega2560").unwrap();
        assert_eq!(mega.board_id, "arduino-mega-2560");
        assert_eq!(mega.port_hint, "usb_serial_bridge");
        assert_eq!(mega.port_selection_step, "select_usb_serial_port");
        assert_eq!(mega.bootloader_touch_baud, None);
        assert!(!mega.expects_port_reenumeration);
        assert!(!mega.wait_for_runtime_rediscovery);
        assert!(!mega.serial_adapter_required);
        assert!(mega.notes.contains("adapter path"));

        let pro_mini = arduino_cli_port_discovery_for_target("arduino-pro-mini").unwrap();
        assert_eq!(pro_mini.port_hint, "external_serial_adapter");
        assert_eq!(
            pro_mini.port_selection_step,
            "select_external_serial_adapter"
        );
        assert!(pro_mini.serial_adapter_required);
        assert!(pro_mini.notes.contains("External serial adapter"));
        assert!(pro_mini.notes.contains("provide the adapter port"));

        assert!(arduino_cli_port_discovery_for_target("esp32").is_none());
        assert!(arduino_cli_port_discovery_for_target("pico").is_none());
        assert!(arduino_cli_port_discovery_for_target("not-a-board").is_none());
    }

    #[test]
    fn arduino_cli_upload_invocation_is_owned_by_rust_language_core() {
        let nano_r4 =
            arduino_cli_upload_invocation_for_target("arduino:renesas_uno:nanor4").unwrap();
        assert_eq!(nano_r4.board_id, "arduino-nano-r4");
        assert_eq!(nano_r4.executable, "arduino-cli");
        assert_eq!(nano_r4.subcommand, "upload");
        assert_eq!(nano_r4.fqbn, "arduino:renesas_uno:nanor4");
        assert_eq!(nano_r4.port_hint, "native_usb");
        assert_eq!(nano_r4.port_selection_step, "select_native_usb_port");
        assert_eq!(nano_r4.port_flag, "-p");
        assert_eq!(nano_r4.fqbn_flag, "-b");
        assert_eq!(nano_r4.input_file_flag, "-i");
        assert_eq!(nano_r4.input_dir_flag, "--input-dir");
        assert_eq!(nano_r4.upload_property_flag, "--upload-property");
        assert_eq!(nano_r4.verify_flag, "-t");
        assert_eq!(
            nano_r4.args_template,
            vec![
                "upload",
                "-p",
                ARDUINO_CLI_UPLOAD_PORT_PLACEHOLDER,
                "-b",
                "arduino:renesas_uno:nanor4",
                "-i",
                ARDUINO_CLI_UPLOAD_INPUT_FILE_PLACEHOLDER,
            ]
        );
        assert!(nano_r4.requires_port);
        assert!(nano_r4.accepts_input_file);
        assert!(nano_r4.accepts_input_dir);
        assert!(nano_r4.accepts_upload_properties);
        assert!(nano_r4.notes.contains("native USB CDC port"));

        let mega =
            arduino_cli_upload_invocation_for_target("arduino:avr:mega:cpu=atmega2560").unwrap();
        assert_eq!(mega.board_id, "arduino-mega-2560");
        assert_eq!(mega.fqbn, "arduino:avr:mega:cpu=atmega2560");
        assert_eq!(mega.port_hint, "usb_serial_bridge");
        assert_eq!(mega.port_selection_step, "select_usb_serial_port");
        assert!(mega.notes.contains("USB serial bridge path"));

        let pro_mini = arduino_cli_upload_invocation_for_target("arduino-pro-mini").unwrap();
        assert_eq!(pro_mini.port_hint, "external_serial_adapter");
        assert_eq!(
            pro_mini.port_selection_step,
            "select_external_serial_adapter"
        );
        assert!(pro_mini.notes.contains("external serial adapter path"));

        assert!(arduino_cli_upload_invocation_for_target("esp32").is_none());
        assert!(arduino_cli_upload_invocation_for_target("pico").is_none());
        assert!(arduino_cli_upload_invocation_for_target("not-a-board").is_none());
    }

    #[test]
    fn arduino_cli_upload_command_is_owned_by_rust_language_core() {
        let command = arduino_cli_upload_command_for_target(
            "arduino:renesas_uno:nanor4",
            "/dev/cu.usbmodem101",
            "/tmp/board-vm-nano-r4.bin",
        )
        .unwrap();
        assert_eq!(command.board_id, "arduino-nano-r4");
        assert_eq!(command.executable, "arduino-cli");
        assert_eq!(command.fqbn, "arduino:renesas_uno:nanor4");
        assert_eq!(command.port, "/dev/cu.usbmodem101");
        assert_eq!(command.input_file, "/tmp/board-vm-nano-r4.bin");
        assert_eq!(command.port_hint, "native_usb");
        assert_eq!(command.port_selection_step, "select_native_usb_port");
        assert!(!command.verify);
        assert!(command.upload_properties.is_empty());
        assert_eq!(
            command.args,
            vec![
                "upload",
                "-p",
                "/dev/cu.usbmodem101",
                "-b",
                "arduino:renesas_uno:nanor4",
                "-i",
                "/tmp/board-vm-nano-r4.bin",
            ]
        );
        assert!(command.notes.contains("Concrete Arduino CLI argv"));

        let command = arduino_cli_upload_command_with_options_for_target(
            "arduino-mega-2560",
            "COM7",
            "C:/tmp/mega.hex",
            &["upload.speed=115200", "programmer=arduinoasisp"],
            true,
        )
        .unwrap();
        assert_eq!(command.board_id, "arduino-mega-2560");
        assert_eq!(command.port_hint, "usb_serial_bridge");
        assert!(command.verify);
        assert_eq!(
            command.upload_properties,
            vec!["upload.speed=115200", "programmer=arduinoasisp"]
        );
        assert_eq!(
            command.args,
            vec![
                "upload",
                "-p",
                "COM7",
                "-b",
                "arduino:avr:mega:cpu=atmega2560",
                "-i",
                "C:/tmp/mega.hex",
                "--upload-property",
                "upload.speed=115200",
                "--upload-property",
                "programmer=arduinoasisp",
                "-t",
            ]
        );

        assert!(
            arduino_cli_upload_command_for_target("arduino-pro-mini", "", "/tmp/pro-mini.hex")
                .is_none()
        );
        assert!(arduino_cli_upload_command_for_target(
            "arduino-pro-mini",
            "/dev/cu.usbserial-1420",
            ""
        )
        .is_none());
        assert!(arduino_cli_upload_command_with_options_for_target(
            "arduino-pro-mini",
            "/dev/cu.usbserial-1420",
            "/tmp/pro-mini.hex",
            &[""],
            false,
        )
        .is_none());
        assert!(arduino_cli_upload_command_for_target(
            "esp32",
            "/dev/cu.usbserial-esp32",
            "/tmp/esp32.bin"
        )
        .is_none());
    }

    #[test]
    fn arduino_cli_upload_execution_plan_is_owned_by_rust_language_core() {
        let plan = arduino_cli_upload_execution_plan_for_target(
            "arduino:renesas_uno:nanor4",
            "/dev/cu.usbmodem101",
            "/tmp/board-vm-nano-r4.bin",
        )
        .unwrap();
        assert_eq!(plan.board_id, "arduino-nano-r4");
        assert_eq!(plan.executable, "arduino-cli");
        assert_eq!(plan.fqbn, "arduino:renesas_uno:nanor4");
        assert_eq!(plan.port, "/dev/cu.usbmodem101");
        assert_eq!(plan.input_file, "/tmp/board-vm-nano-r4.bin");
        assert_eq!(plan.port_hint, "native_usb");
        assert_eq!(plan.port_selection_step, "select_native_usb_port");
        assert_eq!(plan.reset_method, "arduino_board_package");
        assert!(plan.reset_delegated_to_board_package);
        assert_eq!(
            plan.bootloader_touch_baud,
            Some(ARDUINO_CLI_NATIVE_USB_BOOTLOADER_TOUCH_BAUD)
        );
        assert!(plan.expects_port_reenumeration);
        assert!(plan.wait_for_runtime_rediscovery);
        assert!(!plan.serial_adapter_required);
        assert_eq!(
            plan.steps,
            vec![
                "use_selected_native_usb_port",
                "delegate_reset_to_board_package",
                "run_arduino_cli_upload",
                "wait_for_runtime_port_rediscovery",
            ]
        );
        assert_eq!(plan.success_exit_codes, vec![0]);
        assert!(plan.notes.contains("native USB bootloader reset"));
        assert_eq!(
            plan.args,
            vec![
                "upload",
                "-p",
                "/dev/cu.usbmodem101",
                "-b",
                "arduino:renesas_uno:nanor4",
                "-i",
                "/tmp/board-vm-nano-r4.bin",
            ]
        );

        let plan = arduino_cli_upload_execution_plan_with_options_for_target(
            "arduino-mega-2560",
            "COM7",
            "C:/tmp/mega.hex",
            &["upload.speed=115200"],
            true,
        )
        .unwrap();
        assert_eq!(plan.board_id, "arduino-mega-2560");
        assert_eq!(plan.port_hint, "usb_serial_bridge");
        assert_eq!(plan.bootloader_touch_baud, None);
        assert!(!plan.expects_port_reenumeration);
        assert!(!plan.wait_for_runtime_rediscovery);
        assert!(!plan.serial_adapter_required);
        assert!(plan.verify);
        assert_eq!(plan.upload_properties, vec!["upload.speed=115200"]);
        assert_eq!(
            plan.steps,
            vec![
                "use_selected_usb_serial_bridge",
                "delegate_reset_to_board_package",
                "run_arduino_cli_upload",
                "reuse_selected_serial_port",
            ]
        );
        assert!(plan.notes.contains("USB serial bridge port"));

        let plan = arduino_cli_upload_execution_plan_for_target(
            "arduino-pro-mini",
            "/dev/cu.usbserial-1420",
            "/tmp/pro-mini.hex",
        )
        .unwrap();
        assert_eq!(plan.port_hint, "external_serial_adapter");
        assert!(plan.serial_adapter_required);
        assert_eq!(
            plan.steps,
            vec![
                "use_selected_external_serial_adapter",
                "delegate_reset_to_board_package",
                "run_arduino_cli_upload",
                "reuse_selected_serial_port",
            ]
        );

        assert!(arduino_cli_upload_execution_plan_for_target(
            "arduino-pro-mini",
            "",
            "/tmp/pro-mini.hex"
        )
        .is_none());
        assert!(arduino_cli_upload_execution_plan_for_target(
            "esp32",
            "/dev/cu.usbserial-esp32",
            "/tmp/esp32.bin"
        )
        .is_none());
    }

    #[test]
    fn arduino_cli_upload_process_is_owned_by_rust_language_core() {
        let process = arduino_cli_upload_process_for_target(
            "arduino:renesas_uno:nanor4",
            "/dev/cu.usbmodem101",
            "/tmp/board-vm-nano-r4.bin",
        )
        .unwrap();
        assert_eq!(process.board_id, "arduino-nano-r4");
        assert_eq!(process.executable, "arduino-cli");
        assert_eq!(
            process.args,
            vec![
                "upload",
                "-p",
                "/dev/cu.usbmodem101",
                "-b",
                "arduino:renesas_uno:nanor4",
                "-i",
                "/tmp/board-vm-nano-r4.bin",
            ]
        );
        assert!(process.env.is_empty());
        assert_eq!(process.current_dir, None);
        assert_eq!(process.stdin_mode, ARDUINO_CLI_UPLOAD_PROCESS_STDIN_MODE);
        assert_eq!(process.stdout_mode, ARDUINO_CLI_UPLOAD_PROCESS_STDOUT_MODE);
        assert_eq!(process.stderr_mode, ARDUINO_CLI_UPLOAD_PROCESS_STDERR_MODE);
        assert_eq!(process.success_exit_codes, vec![0]);
        assert_eq!(process.port_hint, "native_usb");
        assert_eq!(process.port_selection_step, "select_native_usb_port");
        assert_eq!(process.reset_method, "arduino_board_package");
        assert!(process.reset_delegated_to_board_package);
        assert!(process.expects_port_reenumeration);
        assert!(process.wait_for_runtime_rediscovery);
        assert!(!process.serial_adapter_required);
        assert!(process.notes.contains("runtime CDC port"));

        let process = arduino_cli_upload_process_with_options_for_target(
            "arduino-mega-2560",
            "COM7",
            "C:/tmp/mega.hex",
            &["upload.speed=115200"],
            true,
        )
        .unwrap();
        assert_eq!(process.port_hint, "usb_serial_bridge");
        assert!(!process.expects_port_reenumeration);
        assert!(!process.wait_for_runtime_rediscovery);
        assert!(!process.serial_adapter_required);
        assert_eq!(
            process.args,
            vec![
                "upload",
                "-p",
                "COM7",
                "-b",
                "arduino:avr:mega:cpu=atmega2560",
                "-i",
                "C:/tmp/mega.hex",
                "--upload-property",
                "upload.speed=115200",
                "-t",
            ]
        );
        assert!(process.notes.contains("USB serial bridge port"));

        assert!(
            arduino_cli_upload_process_for_target("arduino-pro-mini", "", "/tmp/pro-mini.hex")
                .is_none()
        );
        assert!(arduino_cli_upload_process_for_target(
            "esp32",
            "/dev/cu.usbserial-esp32",
            "/tmp/esp32.bin"
        )
        .is_none());
    }

    #[test]
    fn arduino_cli_upload_results_are_owned_by_rust_language_core() {
        let plan = arduino_cli_upload_execution_plan_for_target(
            "arduino:renesas_uno:nanor4",
            "/dev/cu.usbmodem101",
            "/tmp/board-vm-nano-r4.bin",
        )
        .unwrap();
        let result =
            arduino_cli_upload_result_for_execution_plan(&plan, 0, "Done uploading.\n", "");
        assert_eq!(result.board_id, "arduino-nano-r4");
        assert_eq!(result.exit_code, 0);
        assert!(result.success);
        assert_eq!(result.status, "success");
        assert_eq!(result.failure_kind, None);
        assert!(!result.retryable);
        assert!(!result.needs_port_selection);
        assert!(!result.needs_board_package_install);
        assert!(!result.needs_firmware_artifact);
        assert!(result.wait_for_runtime_rediscovery);
        assert_eq!(result.port_hint, "native_usb");
        assert_eq!(result.message, "Arduino CLI upload completed successfully.");
        assert_eq!(result.diagnostic, "Done uploading.");

        let mut process = arduino_cli_upload_process_for_execution_plan(&plan);
        process.success_exit_codes = vec![0, 42];
        let result =
            arduino_cli_upload_result_for_process_output(&process, 42, "Done uploading.\n", "");
        assert!(result.success);
        assert_eq!(result.failure_kind, None);
        assert!(result.wait_for_runtime_rediscovery);

        let result = arduino_cli_upload_result_for_process_output(
            &process,
            1,
            "",
            "Error: programmer is not responding",
        );
        assert_eq!(
            result.failure_kind.as_deref(),
            Some("upload_transport_error")
        );
        assert!(result.retryable);
        assert!(!result.wait_for_runtime_rediscovery);

        let result = arduino_cli_upload_result_for_target(
            "arduino-mega-2560",
            1,
            "",
            "Error: serial port not found: COM7",
        )
        .unwrap();
        assert_eq!(result.board_id, "arduino-mega-2560");
        assert!(!result.success);
        assert_eq!(result.status, "failed");
        assert_eq!(result.failure_kind.as_deref(), Some("port_not_found"));
        assert!(result.retryable);
        assert!(result.needs_port_selection);
        assert!(!result.needs_board_package_install);
        assert!(!result.needs_firmware_artifact);
        assert!(!result.wait_for_runtime_rediscovery);
        assert_eq!(result.port_hint, "usb_serial_bridge");
        assert_eq!(result.diagnostic, "Error: serial port not found: COM7");

        let result = arduino_cli_upload_result_for_target(
            "arduino-pro-mini",
            1,
            "",
            "Error: open /tmp/pro-mini.hex: no such file or directory",
        )
        .unwrap();
        assert_eq!(result.failure_kind.as_deref(), Some("missing_input_file"));
        assert!(!result.retryable);
        assert!(result.needs_firmware_artifact);
        assert_eq!(result.port_hint, "external_serial_adapter");

        let result = arduino_cli_upload_result_for_target(
            "arduino:renesas_uno:nanor4",
            1,
            "",
            "Error: platform not installed: arduino:renesas_uno",
        )
        .unwrap();
        assert_eq!(
            result.failure_kind.as_deref(),
            Some("board_package_missing")
        );
        assert!(!result.retryable);
        assert!(result.needs_board_package_install);

        let result = arduino_cli_upload_result_for_target(
            "arduino-mega-2560",
            1,
            "",
            "Error: verification failed",
        )
        .unwrap();
        assert_eq!(result.failure_kind.as_deref(), Some("verification_failed"));
        assert!(result.retryable);

        assert!(arduino_cli_upload_result_for_target("esp32", 1, "", "Error").is_none());
    }

    #[test]
    fn arduino_cli_upload_runtime_handoff_is_owned_by_rust_language_core() {
        assert_eq!(
            arduino_cli_new_upload_port(
                "Sketch uses 30720 bytes.\nNew upload port: /dev/cu.usbmodem1101 (serial)\n"
            ),
            Some("/dev/cu.usbmodem1101".to_owned())
        );
        assert_eq!(
            arduino_cli_new_upload_port(
                "New upload port: /dev/cu.usbmodem9070692469E42 (serial)\n\
                 New upload port: /dev/cu.usbmodem1101 (serial)\n"
            ),
            Some("/dev/cu.usbmodem1101".to_owned())
        );
        assert_eq!(
            arduino_cli_new_upload_port("No new serial port found."),
            None
        );

        let native_plan = arduino_cli_upload_execution_plan_for_target(
            "arduino-nano-r4",
            "/dev/cu.usbmodem9070692469E42",
            "/tmp/board-vm-nano-r4.bin",
        )
        .unwrap();
        let handoff = arduino_cli_upload_runtime_handoff_for_execution_plan(
            &native_plan,
            0,
            "Resetting board...\nNew upload port: /dev/cu.usbmodem1101 (serial)\n",
            "",
        )
        .unwrap();
        assert_eq!(handoff.board_id, "arduino-nano-r4");
        assert_eq!(handoff.upload_port, "/dev/cu.usbmodem9070692469E42");
        assert_eq!(handoff.runtime_port, "/dev/cu.usbmodem1101");
        assert_eq!(handoff.runtime_port_source, "arduino_cli_new_upload_port");
        assert!(handoff.wait_for_runtime_rediscovery);
        assert_eq!(handoff.port_hint, "native_usb");
        assert!(handoff.message.contains("reported the runtime port"));

        let stderr_handoff = arduino_cli_upload_runtime_handoff_for_execution_plan(
            &native_plan,
            0,
            "Done uploading.\n",
            "New upload port: /dev/cu.usbmodem2201 (serial)\n",
        )
        .unwrap();
        assert_eq!(stderr_handoff.runtime_port, "/dev/cu.usbmodem2201");

        let fallback = arduino_cli_upload_runtime_handoff_for_execution_plan(
            &native_plan,
            0,
            "Done uploading.\n",
            "",
        )
        .unwrap();
        assert_eq!(fallback.runtime_port, "/dev/cu.usbmodem9070692469E42");
        assert_eq!(fallback.runtime_port_source, "selected_upload_port");
        assert!(fallback.wait_for_runtime_rediscovery);
        assert!(fallback
            .message
            .contains("did not report a new runtime port"));

        let bridge_process = arduino_cli_upload_process_for_target(
            "arduino:avr:mega:cpu=atmega2560",
            "COM7",
            "C:/tmp/mega.hex",
        )
        .unwrap();
        let bridge_handoff = arduino_cli_upload_runtime_handoff_for_process_output(
            &bridge_process,
            "COM7",
            0,
            "Done uploading.\nNew upload port: COM8 (serial)\n",
            "",
        )
        .unwrap();
        assert_eq!(bridge_handoff.board_id, "arduino-mega-2560");
        assert_eq!(bridge_handoff.runtime_port, "COM7");
        assert_eq!(bridge_handoff.runtime_port_source, "selected_upload_port");
        assert!(!bridge_handoff.wait_for_runtime_rediscovery);
        assert_eq!(bridge_handoff.port_hint, "usb_serial_bridge");
        assert!(bridge_handoff.message.contains("USB serial bridge"));

        assert!(arduino_cli_upload_runtime_handoff_for_execution_plan(
            &native_plan,
            1,
            "",
            "Error: programmer is not responding",
        )
        .is_none());
    }

    #[test]
    fn generic_upload_options_are_owned_by_rust_language_core() {
        let opta = upload_options_for_target("arduino-opta-wifi").unwrap();
        assert_eq!(opta.board_id, "arduino-opta-wifi");
        assert_eq!(opta.adapter, "arduino_cli");
        assert_eq!(opta.image_format, "arduino_cli_build_output");
        assert_eq!(opta.transport, "serial");
        assert_eq!(opta.reset_method, "arduino_board_package");
        assert_eq!(opta.port_hint.as_deref(), Some("native_usb"));
        assert_eq!(opta.command, "arduino-cli upload");
        assert_eq!(opta.platform_id.as_deref(), Some("arduino:mbed_opta"));
        assert_eq!(opta.fqbn.as_deref(), Some("arduino:mbed_opta:opta"));

        let nano_r4 = upload_options_for_target("arduino:renesas_uno:nanor4").unwrap();
        assert_eq!(nano_r4.board_id, "arduino-nano-r4");
        assert_eq!(nano_r4.fqbn.as_deref(), Some("arduino:renesas_uno:nanor4"));
        assert_eq!(nano_r4.port_hint.as_deref(), Some("native_usb"));

        let pro_mini = upload_options_for_target("arduino-pro-mini").unwrap();
        assert_eq!(
            pro_mini.port_hint.as_deref(),
            Some("external_serial_adapter")
        );

        let esp32 = upload_options_for_target("esp32").unwrap();
        assert_eq!(esp32.board_id, "esp32-devkit-v1");
        assert_eq!(esp32.adapter, "esp_rom_serial");
        assert_eq!(esp32.image_format, "esp_flash_image");
        assert_eq!(esp32.reset_method, "esp_rom_boot_pins");
        assert_eq!(esp32.port_hint.as_deref(), Some("esp_rom_serial"));
        assert_eq!(esp32.platform_id, None);
        assert_eq!(esp32.fqbn, None);

        let pico = upload_options_for_target("pico").unwrap();
        assert_eq!(pico.adapter, "pico_uf2_mass_storage");
        assert_eq!(pico.image_format, "uf2");
        assert_eq!(pico.transport, "mass_storage");
        assert_eq!(pico.reset_method, "pico_bootsel");
        assert_eq!(pico.port_hint.as_deref(), Some("mass_storage_bootloader"));

        assert!(upload_options_for_target("not-a-board").is_none());
    }

    #[test]
    fn upload_plans_are_owned_by_rust_language_core() {
        let opta = upload_plan_for_target("arduino-opta-wifi").unwrap();
        assert_eq!(opta.board_id, "arduino-opta-wifi");
        assert_eq!(opta.adapter, "arduino_cli");
        assert_eq!(opta.artifact_kind, "arduino_cli_build_output");
        assert_eq!(opta.platform_id.as_deref(), Some("arduino:mbed_opta"));
        assert_eq!(opta.fqbn.as_deref(), Some("arduino:mbed_opta:opta"));
        assert_eq!(opta.port_hint.as_deref(), Some("native_usb"));
        assert_eq!(opta.artifact_extension, None);
        assert!(opta.requires_serial_port);
        assert!(!opta.requires_mount_path);
        assert!(!opta.auto_detect_mount);
        assert_eq!(
            opta.steps,
            vec![
                "resolve_arduino_board_package",
                "build_firmware_artifact",
                "select_native_usb_port",
                "delegate_reset_to_board_package",
                "upload_with_arduino_cli",
            ]
        );

        let mega = upload_plan_for_target("arduino:avr:mega:cpu=atmega2560").unwrap();
        assert_eq!(mega.board_id, "arduino-mega-2560");
        assert_eq!(
            mega.fqbn.as_deref(),
            Some("arduino:avr:mega:cpu=atmega2560")
        );
        assert_eq!(mega.port_hint.as_deref(), Some("usb_serial_bridge"));
        assert!(mega.steps.contains(&"select_usb_serial_port".to_owned()));

        let pro_mini = upload_plan_for_target("arduino-pro-mini").unwrap();
        assert_eq!(
            pro_mini.port_hint.as_deref(),
            Some("external_serial_adapter")
        );
        assert!(pro_mini
            .steps
            .contains(&"select_external_serial_adapter".to_owned()));

        let esp32 = upload_plan_for_target("esp32").unwrap();
        assert_eq!(esp32.adapter, "esp_rom_serial");
        assert_eq!(esp32.artifact_kind, "esp_flash_image");
        assert_eq!(esp32.port_hint.as_deref(), Some("esp_rom_serial"));
        assert_eq!(esp32.platform_id, None);
        assert_eq!(esp32.fqbn, None);
        assert_eq!(esp32.artifact_extension.as_deref(), Some(".bin"));
        assert!(esp32.requires_serial_port);
        assert!(!esp32.requires_mount_path);
        assert!(esp32.steps.contains(&"verify_md5".to_owned()));

        let pico = upload_plan_for_target("pico-w").unwrap();
        assert_eq!(pico.board_id, "raspberry-pi-pico-w");
        assert_eq!(pico.adapter, "pico_uf2_mass_storage");
        assert_eq!(pico.artifact_kind, "uf2_file");
        assert_eq!(pico.port_hint.as_deref(), Some("mass_storage_bootloader"));
        assert_eq!(pico.artifact_extension.as_deref(), Some(".uf2"));
        assert!(!pico.requires_serial_port);
        assert!(pico.requires_mount_path);
        assert!(pico.auto_detect_mount);
        assert!(pico.steps.contains(&"copy_uf2_to_mount".to_owned()));

        assert!(upload_plan_for_target("not-a-board").is_none());
    }

    #[test]
    fn pico_bootsel_mount_discovery_is_owned_by_rust_language_core() {
        let root = unique_temp_dir("language-core-pico-uf2");
        let mount = root.join("RPI-RP2");
        fs::create_dir_all(&mount).unwrap();
        fs::write(
            mount.join("INFO_UF2.TXT"),
            "UF2 Bootloader v3.0\nModel: Raspberry Pi RP2\n",
        )
        .unwrap();
        fs::write(mount.join("INDEX.HTM"), "<html></html>").unwrap();
        fs::create_dir_all(root.join("NOT-PICO")).unwrap();

        let mounts = discover_pico_bootsel_mounts_in_roots([&root]);

        assert_eq!(mounts, vec![mount.to_string_lossy().into_owned()]);
        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn host_device_discovery_classifies_serial_candidates() {
        let devices = discover_devices_from_paths([
            "/dev/cu.usbmodem1101",
            "/dev/tty.usbserial-CP2102-esp32",
            "/dev/serial/by-id/usb-Raspberry_Pi_Pico_E660-DAPLINK-if00",
            "/tmp/not-a-board",
        ]);

        assert_eq!(devices.len(), 3);
        assert_eq!(devices[0].port, "/dev/cu.usbmodem1101");
        assert_eq!(devices[0].target, None);
        assert!(devices[0].tags.contains(&"usb_cdc".to_owned()));

        let pico = devices
            .iter()
            .find(|device| device.port.contains("Raspberry_Pi_Pico"))
            .unwrap();
        assert_eq!(pico.target.as_ref().unwrap().board_id, "raspberry-pi-pico");
        assert!(pico.bootloader);
        assert!(pico.target_confidence >= 90);

        let esp = devices
            .iter()
            .find(|device| device.port.contains("usbserial"))
            .unwrap();
        assert_eq!(esp.target.as_ref().unwrap().board_id, "esp32-devkit-v1");
        assert!(esp.tags.contains(&"uart".to_owned()));
    }

    fn unique_temp_dir(name: &str) -> PathBuf {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("{name}-{}-{nonce}", std::process::id()))
    }

    #[test]
    fn c_abi_builds_blink_upload_run_wire_frames_from_rust_core() {
        let mut session = BoardVmLanguageSession::new();
        let mut module = [0u8; BLINK_MODULE_LEN];
        let mut wire = [0u8; 256];
        let mut raw = [0u8; 256];

        let module_status = unsafe {
            board_vm_language_blink_module(
                13,
                250,
                250,
                4,
                module.as_mut_ptr(),
                module.len() as u64,
            )
        };
        assert_eq!(module_status.code, BoardVmLanguageStatusCode::Ok as u32);
        assert_eq!(module_status.len, BLINK_MODULE_LEN as u64);

        let mut time_now_module = [0u8; TIME_NOW_MODULE_LEN];
        let time_now_status = unsafe {
            board_vm_language_time_now_module(
                1,
                time_now_module.as_mut_ptr(),
                time_now_module.len() as u64,
            )
        };
        assert_eq!(time_now_status.code, BoardVmLanguageStatusCode::Ok as u32);
        assert_eq!(time_now_status.len, TIME_NOW_MODULE_LEN as u64);

        let mut time_sleep_module = [0u8; TIME_SLEEP_MS_MODULE_LEN];
        let time_sleep_status = unsafe {
            board_vm_language_time_sleep_ms_module(
                250,
                1,
                time_sleep_module.as_mut_ptr(),
                time_sleep_module.len() as u64,
            )
        };
        assert_eq!(time_sleep_status.code, BoardVmLanguageStatusCode::Ok as u32);
        assert_eq!(time_sleep_status.len, TIME_SLEEP_MS_MODULE_LEN as u64);

        let mut led_matrix_module = [0u8; LED_MATRIX_FRAME_MODULE_LEN];
        let led_matrix_status = unsafe {
            board_vm_language_led_matrix_frame_module(
                0x3184_A444,
                0x4404_2081,
                0x100A_0040,
                3,
                led_matrix_module.as_mut_ptr(),
                led_matrix_module.len() as u64,
            )
        };
        assert_eq!(led_matrix_status.code, BoardVmLanguageStatusCode::Ok as u32);
        assert_eq!(led_matrix_status.len, LED_MATRIX_FRAME_MODULE_LEN as u64);

        let mut pwm_module = [0u8; PWM_WRITE_MODULE_LEN];
        let pwm_status = unsafe {
            board_vm_language_pwm_write_module(
                3,
                0x8000,
                2,
                pwm_module.as_mut_ptr(),
                pwm_module.len() as u64,
            )
        };
        assert_eq!(pwm_status.code, BoardVmLanguageStatusCode::Ok as u32);
        assert_eq!(pwm_status.len, PWM_WRITE_MODULE_LEN as u64);

        let mut adc_module = [0u8; ADC_READ_MODULE_LEN];
        let adc_status = unsafe {
            board_vm_language_adc_read_module(
                14,
                1,
                adc_module.as_mut_ptr(),
                adc_module.len() as u64,
            )
        };
        assert_eq!(adc_status.code, BoardVmLanguageStatusCode::Ok as u32);
        assert_eq!(adc_status.len, ADC_READ_MODULE_LEN as u64);

        let mut dac_module = [0u8; DAC_WRITE_U12_MODULE_LEN];
        let dac_status = unsafe {
            board_vm_language_dac_write_u12_module(
                14,
                0x0800,
                2,
                dac_module.as_mut_ptr(),
                dac_module.len() as u64,
            )
        };
        assert_eq!(dac_status.code, BoardVmLanguageStatusCode::Ok as u32);
        assert_eq!(dac_status.len, DAC_WRITE_U12_MODULE_LEN as u64);

        let mut i2c_module = [0u8; I2C_OPEN_MODULE_LEN];
        let i2c_status = unsafe {
            board_vm_language_i2c_open_module(
                0,
                2,
                i2c_module.as_mut_ptr(),
                i2c_module.len() as u64,
            )
        };
        assert_eq!(i2c_status.code, BoardVmLanguageStatusCode::Ok as u32);
        assert_eq!(i2c_status.len, I2C_OPEN_MODULE_LEN as u64);

        let mut spi_module = [0u8; SPI_OPEN_MODULE_LEN];
        let spi_status = unsafe {
            board_vm_language_spi_open_module(
                0,
                2,
                spi_module.as_mut_ptr(),
                spi_module.len() as u64,
            )
        };
        assert_eq!(spi_status.code, BoardVmLanguageStatusCode::Ok as u32);
        assert_eq!(spi_status.len, SPI_OPEN_MODULE_LEN as u64);

        let mut uart_module = [0u8; UART_OPEN_MODULE_LEN];
        let uart_status = unsafe {
            board_vm_language_uart_open_module(
                0,
                2,
                uart_module.as_mut_ptr(),
                uart_module.len() as u64,
            )
        };
        assert_eq!(uart_status.code, BoardVmLanguageStatusCode::Ok as u32);
        assert_eq!(uart_status.len, UART_OPEN_MODULE_LEN as u64);

        let mut uart_write_module = [0u8; UART_WRITE_MODULE_LEN];
        let uart_write_status = unsafe {
            board_vm_language_uart_write_module(
                0xa5,
                3,
                uart_write_module.as_mut_ptr(),
                uart_write_module.len() as u64,
            )
        };
        assert_eq!(uart_write_status.code, BoardVmLanguageStatusCode::Ok as u32);
        assert_eq!(
            uart_write_status.len,
            board_vm_language_uart_write_module_len()
        );

        let mut uart_read_module = [0u8; UART_READ_MODULE_LEN];
        let uart_read_status = unsafe {
            board_vm_language_uart_read_module(
                2,
                uart_read_module.as_mut_ptr(),
                uart_read_module.len() as u64,
            )
        };
        assert_eq!(uart_read_status.code, BoardVmLanguageStatusCode::Ok as u32);
        assert_eq!(
            uart_read_status.len,
            board_vm_language_uart_read_module_len()
        );

        let mut can_module = [0u8; CAN_OPEN_MODULE_LEN];
        let can_status = unsafe {
            board_vm_language_can_open_module(
                0,
                2,
                can_module.as_mut_ptr(),
                can_module.len() as u64,
            )
        };
        assert_eq!(can_status.code, BoardVmLanguageStatusCode::Ok as u32);
        assert_eq!(can_status.len, CAN_OPEN_MODULE_LEN as u64);

        let mut can_write_module = [0u8; CAN_WRITE_MODULE_LEN];
        let can_write_status = unsafe {
            board_vm_language_can_write_module(
                0xa5,
                3,
                can_write_module.as_mut_ptr(),
                can_write_module.len() as u64,
            )
        };
        assert_eq!(can_write_status.code, BoardVmLanguageStatusCode::Ok as u32);
        assert_eq!(
            can_write_status.len,
            board_vm_language_can_write_module_len()
        );

        let mut can_read_module = [0u8; CAN_READ_MODULE_LEN];
        let can_read_status = unsafe {
            board_vm_language_can_read_module(
                2,
                can_read_module.as_mut_ptr(),
                can_read_module.len() as u64,
            )
        };
        assert_eq!(can_read_status.code, BoardVmLanguageStatusCode::Ok as u32);
        assert_eq!(can_read_status.len, board_vm_language_can_read_module_len());

        let mut rtc_now_module = [0u8; RTC_NOW_MODULE_LEN];
        let rtc_now_status = unsafe {
            board_vm_language_rtc_now_module(
                1,
                rtc_now_module.as_mut_ptr(),
                rtc_now_module.len() as u64,
            )
        };
        assert_eq!(rtc_now_status.code, BoardVmLanguageStatusCode::Ok as u32);
        assert_eq!(rtc_now_status.len, board_vm_language_rtc_now_module_len());

        let mut rtc_set_module = [0u8; RTC_SET_MODULE_LEN];
        let rtc_set_status = unsafe {
            board_vm_language_rtc_set_module(
                1_700_000_000,
                1,
                rtc_set_module.as_mut_ptr(),
                rtc_set_module.len() as u64,
            )
        };
        assert_eq!(rtc_set_status.code, BoardVmLanguageStatusCode::Ok as u32);
        assert_eq!(rtc_set_status.len, board_vm_language_rtc_set_module_len());

        let mut watchdog_configure_module = [0u8; WATCHDOG_CONFIGURE_MODULE_LEN];
        let watchdog_configure_status = unsafe {
            board_vm_language_watchdog_configure_module(
                2_000,
                1,
                watchdog_configure_module.as_mut_ptr(),
                watchdog_configure_module.len() as u64,
            )
        };
        assert_eq!(
            watchdog_configure_status.code,
            BoardVmLanguageStatusCode::Ok as u32
        );
        assert_eq!(
            watchdog_configure_status.len,
            board_vm_language_watchdog_configure_module_len()
        );

        let mut watchdog_kick_module = [0u8; WATCHDOG_KICK_MODULE_LEN];
        let watchdog_kick_status = unsafe {
            board_vm_language_watchdog_kick_module(
                1,
                watchdog_kick_module.as_mut_ptr(),
                watchdog_kick_module.len() as u64,
            )
        };
        assert_eq!(
            watchdog_kick_status.code,
            BoardVmLanguageStatusCode::Ok as u32
        );
        assert_eq!(
            watchdog_kick_status.len,
            board_vm_language_watchdog_kick_module_len()
        );

        let storage_payload = [0xaa, 0x55];
        let mut storage_write_module = [0u8; board_vm_host::STORAGE_WRITE_MAX_MODULE_LEN];
        let storage_write_status = unsafe {
            board_vm_language_storage_write_module(
                0,
                0x0010,
                storage_payload.as_ptr(),
                storage_payload.len() as u64,
                3,
                storage_write_module.as_mut_ptr(),
                storage_write_module.len() as u64,
            )
        };
        assert_eq!(
            storage_write_status.code,
            BoardVmLanguageStatusCode::Ok as u32
        );
        assert_eq!(
            storage_write_status.len,
            board_vm_language_storage_write_module_len(storage_payload.len() as u64)
        );

        let mut storage_read_module = [0u8; STORAGE_READ_MODULE_LEN];
        let storage_read_status = unsafe {
            board_vm_language_storage_read_module(
                0,
                0x0010,
                2,
                3,
                storage_read_module.as_mut_ptr(),
                storage_read_module.len() as u64,
            )
        };
        assert_eq!(
            storage_read_status.code,
            BoardVmLanguageStatusCode::Ok as u32
        );
        assert_eq!(
            storage_read_status.len,
            board_vm_language_storage_read_module_len()
        );

        let mut storage_size_module = [0u8; STORAGE_SIZE_MODULE_LEN];
        let storage_size_status = unsafe {
            board_vm_language_storage_size_module(
                0,
                1,
                storage_size_module.as_mut_ptr(),
                storage_size_module.len() as u64,
            )
        };
        assert_eq!(
            storage_size_status.code,
            BoardVmLanguageStatusCode::Ok as u32
        );
        assert_eq!(
            storage_size_status.len,
            board_vm_language_storage_size_module_len()
        );

        let spi_transfer_payload = [0x9f];
        let mut spi_transfer_module = [0u8; board_vm_host::SPI_TRANSFER_MAX_MODULE_LEN];
        let spi_transfer_status = unsafe {
            board_vm_language_spi_transfer_module(
                10,
                spi_transfer_payload.as_ptr(),
                spi_transfer_payload.len() as u64,
                3,
                5,
                spi_transfer_module.as_mut_ptr(),
                spi_transfer_module.len() as u64,
            )
        };
        assert_eq!(
            spi_transfer_status.code,
            BoardVmLanguageStatusCode::Ok as u32
        );
        assert_eq!(
            spi_transfer_status.len,
            board_vm_language_spi_transfer_module_len(spi_transfer_payload.len() as u64)
        );

        let spi_write_payload = [0xde, 0xad, 0xbe];
        let mut spi_write_module = [0u8; board_vm_host::SPI_WRITE_MAX_MODULE_LEN];
        let spi_write_status = unsafe {
            board_vm_language_spi_write_module(
                10,
                spi_write_payload.as_ptr(),
                spi_write_payload.len() as u64,
                5,
                spi_write_module.as_mut_ptr(),
                spi_write_module.len() as u64,
            )
        };
        assert_eq!(spi_write_status.code, BoardVmLanguageStatusCode::Ok as u32);
        assert_eq!(
            spi_write_status.len,
            board_vm_language_spi_write_module_len(spi_write_payload.len() as u64)
        );

        let mut spi_read_module = [0u8; SPI_READ_MODULE_LEN];
        let spi_read_status = unsafe {
            board_vm_language_spi_read_module(
                10,
                3,
                5,
                spi_read_module.as_mut_ptr(),
                spi_read_module.len() as u64,
            )
        };
        assert_eq!(spi_read_status.code, BoardVmLanguageStatusCode::Ok as u32);
        assert_eq!(spi_read_status.len, board_vm_language_spi_read_module_len());

        let mut i2c_write_module = [0u8; I2C_WRITE_U8_MODULE_LEN];
        let i2c_write_status = unsafe {
            board_vm_language_i2c_write_u8_module(
                0x3c,
                0xa5,
                4,
                i2c_write_module.as_mut_ptr(),
                i2c_write_module.len() as u64,
            )
        };
        assert_eq!(i2c_write_status.code, BoardVmLanguageStatusCode::Ok as u32);
        assert_eq!(i2c_write_status.len, I2C_WRITE_U8_MODULE_LEN as u64);

        let i2c_payload = [0xde, 0xad, 0xbe];
        let mut i2c_write_bytes_module = [0u8; board_vm_host::I2C_WRITE_MAX_MODULE_LEN];
        let i2c_write_bytes_status = unsafe {
            board_vm_language_i2c_write_module(
                0x3c,
                i2c_payload.as_ptr(),
                i2c_payload.len() as u64,
                4,
                i2c_write_bytes_module.as_mut_ptr(),
                i2c_write_bytes_module.len() as u64,
            )
        };
        assert_eq!(
            i2c_write_bytes_status.code,
            BoardVmLanguageStatusCode::Ok as u32
        );
        assert_eq!(
            i2c_write_bytes_status.len,
            board_vm_language_i2c_write_module_len(i2c_payload.len() as u64)
        );

        let mut i2c_read_module = [0u8; I2C_READ_U8_MODULE_LEN];
        let i2c_read_status = unsafe {
            board_vm_language_i2c_read_u8_module(
                0x3c,
                3,
                i2c_read_module.as_mut_ptr(),
                i2c_read_module.len() as u64,
            )
        };
        assert_eq!(i2c_read_status.code, BoardVmLanguageStatusCode::Ok as u32);
        assert_eq!(i2c_read_status.len, I2C_READ_U8_MODULE_LEN as u64);

        let mut i2c_read_bytes_module = [0u8; I2C_READ_MODULE_LEN];
        let i2c_read_bytes_status = unsafe {
            board_vm_language_i2c_read_module(
                0x3c,
                3,
                4,
                i2c_read_bytes_module.as_mut_ptr(),
                i2c_read_bytes_module.len() as u64,
            )
        };
        assert_eq!(
            i2c_read_bytes_status.code,
            BoardVmLanguageStatusCode::Ok as u32
        );
        assert_eq!(
            i2c_read_bytes_status.len,
            board_vm_language_i2c_read_module_len()
        );

        let i2c_transfer_payload = [0x00, 0x10];
        let mut i2c_transfer_module = [0u8; board_vm_host::I2C_TRANSFER_MAX_MODULE_LEN];
        let i2c_transfer_status = unsafe {
            board_vm_language_i2c_transfer_module(
                0x3c,
                i2c_transfer_payload.as_ptr(),
                i2c_transfer_payload.len() as u64,
                3,
                5,
                i2c_transfer_module.as_mut_ptr(),
                i2c_transfer_module.len() as u64,
            )
        };
        assert_eq!(
            i2c_transfer_status.code,
            BoardVmLanguageStatusCode::Ok as u32
        );
        assert_eq!(
            i2c_transfer_status.len,
            board_vm_language_i2c_transfer_module_len(i2c_transfer_payload.len() as u64)
        );

        let mut gpio_read_module = [0u8; GPIO_READ_MODULE_LEN];
        let gpio_read_status = unsafe {
            board_vm_language_gpio_read_module(
                13,
                board_vm_host::GPIO_MODE_INPUT_PULLUP,
                2,
                gpio_read_module.as_mut_ptr(),
                gpio_read_module.len() as u64,
            )
        };
        assert_eq!(gpio_read_status.code, BoardVmLanguageStatusCode::Ok as u32);
        assert_eq!(gpio_read_status.len, GPIO_READ_MODULE_LEN as u64);

        let mut gpio_write_module = [0u8; GPIO_WRITE_MODULE_LEN];
        let gpio_write_status = unsafe {
            board_vm_language_gpio_write_module(
                13,
                1,
                3,
                gpio_write_module.as_mut_ptr(),
                gpio_write_module.len() as u64,
            )
        };
        assert_eq!(gpio_write_status.code, BoardVmLanguageStatusCode::Ok as u32);
        assert_eq!(gpio_write_status.len, GPIO_WRITE_MODULE_LEN as u64);

        let mut gpio_open_module = [0u8; GPIO_OPEN_MODULE_LEN];
        let gpio_open_status = unsafe {
            board_vm_language_gpio_open_module(
                13,
                board_vm_host::GPIO_MODE_OUTPUT,
                2,
                gpio_open_module.as_mut_ptr(),
                gpio_open_module.len() as u64,
            )
        };
        assert_eq!(gpio_open_status.code, BoardVmLanguageStatusCode::Ok as u32);
        assert_eq!(gpio_open_status.len, GPIO_OPEN_MODULE_LEN as u64);

        let mut gpio_handle_read_module = [0u8; GPIO_HANDLE_READ_MODULE_LEN];
        let gpio_handle_read_status = unsafe {
            board_vm_language_gpio_handle_read_module(
                2,
                gpio_handle_read_module.as_mut_ptr(),
                gpio_handle_read_module.len() as u64,
            )
        };
        assert_eq!(
            gpio_handle_read_status.code,
            BoardVmLanguageStatusCode::Ok as u32
        );
        assert_eq!(
            gpio_handle_read_status.len,
            GPIO_HANDLE_READ_MODULE_LEN as u64
        );

        let mut gpio_handle_write_module = [0u8; GPIO_HANDLE_WRITE_MODULE_LEN];
        let gpio_handle_write_status = unsafe {
            board_vm_language_gpio_handle_write_module(
                1,
                3,
                gpio_handle_write_module.as_mut_ptr(),
                gpio_handle_write_module.len() as u64,
            )
        };
        assert_eq!(
            gpio_handle_write_status.code,
            BoardVmLanguageStatusCode::Ok as u32
        );
        assert_eq!(
            gpio_handle_write_status.len,
            GPIO_HANDLE_WRITE_MODULE_LEN as u64
        );

        let mut gpio_handle_close_module = [0u8; GPIO_HANDLE_CLOSE_MODULE_LEN];
        let gpio_handle_close_status = unsafe {
            board_vm_language_gpio_handle_close_module(
                1,
                gpio_handle_close_module.as_mut_ptr(),
                gpio_handle_close_module.len() as u64,
            )
        };
        assert_eq!(
            gpio_handle_close_status.code,
            BoardVmLanguageStatusCode::Ok as u32
        );
        assert_eq!(
            gpio_handle_close_status.len,
            GPIO_HANDLE_CLOSE_MODULE_LEN as u64
        );

        let begin = unsafe {
            board_vm_language_program_begin_wire(
                &mut session,
                7,
                module.as_ptr(),
                module.len() as u64,
                wire.as_mut_ptr(),
                wire.len() as u64,
            )
        };
        assert_eq!(begin.code, BoardVmLanguageStatusCode::Ok as u32);
        assert_eq!(begin.request_id, 1);
        let decoded = decode_wire_frame_into_raw(&wire[..begin.len as usize], &mut raw).unwrap();
        assert_eq!(decoded.message_type, MessageType::PROGRAM_BEGIN.0);
        let frame = decode_frame(&raw[..decoded.len as usize]).unwrap();
        assert_eq!(decode_program_begin(frame.payload).unwrap().program_id, 7);

        let chunk = unsafe {
            board_vm_language_program_chunk_wire(
                &mut session,
                7,
                0,
                module.as_ptr(),
                module.len() as u64,
                wire.as_mut_ptr(),
                wire.len() as u64,
            )
        };
        assert_eq!(chunk.request_id, 2);
        let decoded = decode_wire_frame_into_raw(&wire[..chunk.len as usize], &mut raw).unwrap();
        assert_eq!(decoded.message_type, MessageType::PROGRAM_CHUNK.0);
        let frame = decode_frame(&raw[..decoded.len as usize]).unwrap();
        let chunk_payload = decode_program_chunk(frame.payload).unwrap();
        assert_eq!(chunk_payload.offset, 0);
        assert_eq!(chunk_payload.bytes, &module);

        let end = unsafe {
            board_vm_language_program_end_wire(
                &mut session,
                7,
                wire.as_mut_ptr(),
                wire.len() as u64,
            )
        };
        assert_eq!(end.request_id, 3);
        let decoded = decode_wire_frame_into_raw(&wire[..end.len as usize], &mut raw).unwrap();
        assert_eq!(decoded.message_type, MessageType::PROGRAM_END.0);
        let frame = decode_frame(&raw[..decoded.len as usize]).unwrap();
        assert_eq!(decode_program_end(frame.payload).unwrap().program_id, 7);

        let store = unsafe {
            board_vm_language_store_program_wire(
                &mut session,
                7,
                2,
                board_vm_protocol::BOOT_RUN_AT_BOOT,
                wire.as_mut_ptr(),
                wire.len() as u64,
            )
        };
        assert_eq!(store.request_id, 4);
        let decoded = decode_wire_frame_into_raw(&wire[..store.len as usize], &mut raw).unwrap();
        assert_eq!(decoded.message_type, MessageType::STORE_PROGRAM.0);
        let frame = decode_frame(&raw[..decoded.len as usize]).unwrap();
        let store_payload = board_vm_protocol::decode_store_program(frame.payload).unwrap();
        assert_eq!(store_payload.program_id, 7);
        assert_eq!(store_payload.slot, 2);
        assert_eq!(
            store_payload.boot_policy,
            board_vm_protocol::BOOT_RUN_AT_BOOT
        );

        let run = unsafe {
            board_vm_language_run_background_wire(
                &mut session,
                7,
                123,
                wire.as_mut_ptr(),
                wire.len() as u64,
            )
        };
        assert_eq!(run.request_id, 5);
        let decoded = decode_wire_frame_into_raw(&wire[..run.len as usize], &mut raw).unwrap();
        assert_eq!(decoded.message_type, MessageType::RUN.0);
        let frame = decode_frame(&raw[..decoded.len as usize]).unwrap();
        let run_payload = decode_run_request(frame.payload).unwrap();
        assert_eq!(run_payload.instruction_budget, 123);
        assert_eq!(
            run_payload.flags,
            RUN_FLAG_RESET_VM_BEFORE_RUN | RUN_FLAG_BACKGROUND_RUN
        );

        let run = unsafe {
            board_vm_language_run_wire(
                &mut session,
                7,
                RUN_FLAG_KEEP_HANDLES_AFTER_RUN,
                456,
                250,
                wire.as_mut_ptr(),
                wire.len() as u64,
            )
        };
        assert_eq!(run.request_id, 6);
        let decoded = decode_wire_frame_into_raw(&wire[..run.len as usize], &mut raw).unwrap();
        assert_eq!(decoded.message_type, MessageType::RUN.0);
        let frame = decode_frame(&raw[..decoded.len as usize]).unwrap();
        let run_payload = decode_run_request(frame.payload).unwrap();
        assert_eq!(run_payload.program_id, 7);
        assert_eq!(run_payload.flags, RUN_FLAG_KEEP_HANDLES_AFTER_RUN);
        assert_eq!(run_payload.instruction_budget, 456);
        assert_eq!(run_payload.time_budget_ms, 250);

        let stop = unsafe {
            board_vm_language_stop_wire(&mut session, wire.as_mut_ptr(), wire.len() as u64)
        };
        assert_eq!(stop.request_id, 7);
        let decoded = decode_wire_frame_into_raw(&wire[..stop.len as usize], &mut raw).unwrap();
        assert_eq!(decoded.message_type, MessageType::STOP.0);
        let frame = decode_frame(&raw[..decoded.len as usize]).unwrap();
        assert!(frame.payload.is_empty());
    }

    #[test]
    fn rust_core_builds_raw_module_from_code_and_const_pool() {
        let code = [0x16, 0x00, 0x00, 0x02, 0x50];
        let const_pool = [0xAA, 0x55];
        let expected_len = raw_module_len(code.len() as u64, const_pool.len() as u64).unwrap();
        let mut module = vec![0u8; expected_len];

        let len = build_raw_module(0, 1, &code, &const_pool, &mut module).unwrap();

        assert_eq!(len, expected_len);
        assert_eq!(
            module,
            [
                0x42, 0x56, 0x4D, 0x31, 0x01, 0x00, 0x01, 0x00, 0x05, 0x16, 0x00, 0x00, 0x02, 0x50,
                0x02, 0xAA, 0x55,
            ]
        );
    }

    #[test]
    fn c_abi_decode_wire_frame_reports_payload_offset_and_len() {
        let report = RunReportHeader {
            program_id: 1,
            status: RunStatus::BudgetExceeded,
            instructions_executed: 12,
            elapsed_ms: 20,
            stack_depth: 1,
            open_handles: 1,
            return_count: 0,
        };
        let mut payload = [0u8; 32];
        let payload_len =
            board_vm_protocol::encode_run_report_header(&report, &mut payload).unwrap();
        let mut raw = [0u8; 64];
        let raw_len = encode_frame(
            &Frame {
                flags: FLAG_IS_RESPONSE,
                message_type: MessageType::RUN_REPORT,
                request_id: 9,
                payload: &payload[..payload_len],
            },
            &mut raw,
        )
        .unwrap();
        let mut wire = [0u8; 96];
        let wire_len = encode_wire_frame(&raw[..raw_len], &mut wire).unwrap();
        let mut decoded_raw = [0u8; 64];

        let status = unsafe {
            board_vm_language_decode_wire_frame(
                wire.as_ptr(),
                wire_len as u64,
                decoded_raw.as_mut_ptr(),
                decoded_raw.len() as u64,
            )
        };

        assert_eq!(status.code, BoardVmLanguageStatusCode::Ok as u32);
        assert_eq!(status.len, raw_len as u64);
        assert_eq!(status.message_type, MessageType::RUN_REPORT.0);
        assert_eq!(status.request_id, 9);
        assert_eq!(status.payload_len, payload_len as u64);
        assert!(status.payload_offset > 0);
    }

    #[test]
    fn rust_core_decodes_structured_response_bodies() {
        let hello = HelloAck {
            selected_version: 1,
            board_name: "uno-r4-wifi",
            runtime_name: "board-vm",
            host_nonce: 0xAABB_CCDD,
            board_nonce: 0x1122_3344,
            max_frame_payload: 512,
        };
        let mut payload = [0u8; 128];
        let payload_len = encode_hello_ack(&hello, &mut payload).unwrap();
        let decoded = decode_response_fixture(MessageType::HELLO_ACK, 11, &payload[..payload_len]);
        assert_eq!(decoded.request_id, 11);
        assert!(decoded.is_response());
        assert_eq!(decoded.body.kind(), "hello_ack");
        match decoded.body {
            DecodedLanguageResponseBody::HelloAck(ack) => {
                assert_eq!(ack.board_name, "uno-r4-wifi");
                assert_eq!(ack.runtime_name, "board-vm");
                assert_eq!(ack.host_nonce, 0xAABB_CCDD);
                assert_eq!(ack.max_frame_payload, 512);
            }
            other => panic!("unexpected hello response body: {other:?}"),
        }

        let caps = CapsReportHeader {
            board_id: "arduino:uno-r4-wifi",
            runtime_id: "board-vm-rust",
            max_program_bytes: 1024,
            max_stack_values: 16,
            max_handles: 4,
            supports_store_program: true,
            capability_count: 1,
        };
        let capabilities = [CapabilityDescriptor {
            id: board_vm_protocol::CAP_PROGRAM_RAM_EXEC,
            version: 1,
            flags: board_vm_protocol::CAP_FLAG_BYTECODE_CALLABLE,
            name: "program.ram.exec",
        }];
        let payload_len = encode_caps_report(&caps, &capabilities, &mut payload).unwrap();
        let decoded =
            decode_response_fixture(MessageType::CAPS_REPORT, 12, &payload[..payload_len]);
        match decoded.body {
            DecodedLanguageResponseBody::CapsReport(report) => {
                assert_eq!(report.board_id, "arduino:uno-r4-wifi");
                assert!(report.supports_store_program);
                assert_eq!(report.capabilities.len(), 1);
                assert_eq!(report.capabilities[0].name, "program.ram.exec");
            }
            other => panic!("unexpected caps response body: {other:?}"),
        }

        let run = RunReportHeader {
            program_id: 7,
            status: RunStatus::Halted,
            instructions_executed: 42,
            elapsed_ms: 8,
            stack_depth: 2,
            open_handles: 1,
            return_count: 1,
        };
        let mut payload_len =
            board_vm_protocol::encode_run_report_header(&run, &mut payload).unwrap();
        payload_len += encode_value(&Value::U32(1234), &mut payload[payload_len..]).unwrap();
        let decoded = decode_response_fixture(MessageType::RUN_REPORT, 13, &payload[..payload_len]);
        match decoded.body {
            DecodedLanguageResponseBody::RunReport(report) => {
                assert_eq!(report.program_id, 7);
                assert_eq!(report.status, RunStatus::Halted);
                assert_eq!(report.instructions_executed, 42);
                assert_eq!(report.return_count, 1);
                assert_eq!(report.returns, vec![LanguageValue::U32(1234)]);
            }
            other => panic!("unexpected run response body: {other:?}"),
        }

        let run = RunReportHeader {
            program_id: 8,
            status: RunStatus::Halted,
            instructions_executed: 43,
            elapsed_ms: 9,
            stack_depth: 0,
            open_handles: 0,
            return_count: 1,
        };
        let mut payload_len =
            board_vm_protocol::encode_run_report_header(&run, &mut payload).unwrap();
        payload_len +=
            encode_value(&Value::Bytes(&[0xCA, 0xFE]), &mut payload[payload_len..]).unwrap();
        let decoded = decode_response_fixture(MessageType::RUN_REPORT, 14, &payload[..payload_len]);
        match decoded.body {
            DecodedLanguageResponseBody::RunReport(report) => {
                assert_eq!(report.program_id, 8);
                assert_eq!(report.returns, vec![LanguageValue::Bytes(vec![0xCA, 0xFE])]);
            }
            other => panic!("unexpected bytes run response body: {other:?}"),
        }
    }

    #[test]
    fn c_abi_reports_null_output_buffers_without_unwinding() {
        let status = unsafe { board_vm_language_blink_module(13, 250, 250, 4, ptr::null_mut(), 0) };

        assert_ne!(status.code, BoardVmLanguageStatusCode::Ok as u32);
        assert_eq!(
            board_vm_language_last_error_code(),
            BoardVmLanguageStatusCode::NullPointer as u32
        );
        let message = unsafe {
            std::ffi::CStr::from_ptr(board_vm_language_last_error_message())
                .to_string_lossy()
                .into_owned()
        };
        assert!(message.contains("module_out"));
    }

    #[test]
    fn rust_core_names_capability_flags_for_language_bindings() {
        let mut names = [""; 3];
        let count = capability_flag_names(
            board_vm_protocol::CAP_FLAG_BYTECODE_CALLABLE
                | board_vm_protocol::CAP_FLAG_PROTOCOL_FEATURE
                | board_vm_protocol::CAP_FLAG_BOARD_METADATA,
            &mut names,
        );

        assert_eq!(
            &names[..count],
            &["bytecode_callable", "protocol_feature", "board_metadata"]
        );

        let mut short = [""; 1];
        let count = capability_flag_names(
            board_vm_protocol::CAP_FLAG_BYTECODE_CALLABLE
                | board_vm_protocol::CAP_FLAG_PROTOCOL_FEATURE,
            &mut short,
        );
        assert_eq!(count, 1);
        assert_eq!(short[0], "bytecode_callable");
    }

    fn decode_response_fixture(
        message_type: MessageType,
        request_id: u16,
        payload: &[u8],
    ) -> DecodedLanguageResponse {
        let mut raw = [0u8; 256];
        let raw_len = encode_frame(
            &Frame {
                flags: FLAG_IS_RESPONSE,
                message_type,
                request_id,
                payload,
            },
            &mut raw,
        )
        .unwrap();
        let mut wire = [0u8; 320];
        let wire_len = encode_wire_frame(&raw[..raw_len], &mut wire).unwrap();
        let mut decoded_raw = [0u8; 256];
        decode_wire_response(&wire[..wire_len], &mut decoded_raw).unwrap()
    }
}
