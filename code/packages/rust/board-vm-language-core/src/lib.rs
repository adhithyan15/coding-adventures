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
    write_adc_read_module, write_blink_module, write_dac_write_u12_module,
    write_gpio_handle_close_module, write_gpio_handle_read_module, write_gpio_handle_write_module,
    write_gpio_open_module, write_gpio_read_module, write_gpio_write_module, write_i2c_open_module,
    write_i2c_write_u8_module, write_led_matrix_frame_module, write_module, write_pwm_write_module,
    write_time_now_module, write_time_sleep_ms_module, AdcReadProgram, BlinkProgram,
    DacWriteU12Program, GpioHandleCloseProgram, GpioHandleReadProgram, GpioHandleWriteProgram,
    GpioOpenProgram, GpioReadProgram, GpioWriteProgram, HostError, HostSession, I2cOpenProgram,
    I2cWriteU8Program, LedMatrixFrameProgram, ModuleSpec, PwmWriteProgram, TimeNowProgram,
    TimeSleepMsProgram, ADC_READ_MODULE_LEN, BLINK_MODULE_LEN, DAC_WRITE_U12_MODULE_LEN,
    DEFAULT_INSTRUCTION_BUDGET, DEFAULT_PROGRAM_ID, DEFAULT_RUN_FLAGS,
    GPIO_HANDLE_CLOSE_MODULE_LEN, GPIO_HANDLE_READ_MODULE_LEN, GPIO_HANDLE_WRITE_MODULE_LEN,
    GPIO_OPEN_MODULE_LEN, GPIO_READ_MODULE_LEN, GPIO_WRITE_MODULE_LEN, I2C_OPEN_MODULE_LEN,
    I2C_WRITE_U8_MODULE_LEN, LED_MATRIX_FRAME_MODULE_LEN, PWM_WRITE_MODULE_LEN,
    TIME_NOW_MODULE_LEN, TIME_SLEEP_MS_MODULE_LEN,
};
use board_vm_protocol::{
    decode_caps_report_header, decode_error_payload, decode_frame, decode_hello_ack,
    decode_program_begin, decode_program_chunk, decode_program_end, decode_run_report_header,
    decode_wire_frame, encode_wire_frame, Frame, MessageType, ProgramFormat, ProtocolError,
    RunStatus, Value as ProtocolValue, CAP_FLAG_BOARD_METADATA, CAP_FLAG_BYTECODE_CALLABLE,
    CAP_FLAG_PROTOCOL_FEATURE, FLAG_IS_ERROR_RESPONSE, FLAG_IS_RESPONSE,
};
use board_vm_targets::{
    all_targets, BoardFamily, BoardTargetInfo, DigitalPinInfo as TargetDigitalPin,
    I2cBusInfo as TargetI2cBus, OnboardLed as TargetOnboardLed,
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
    pub wireless: Vec<LanguageWirelessInterface>,
    pub connection_options: Vec<LanguageConnectionOption>,
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
    all_targets()
        .iter()
        .find(|target| target.board_id == board_id)
        .map(language_target_info)
}

pub fn detect_target(selector: &str) -> Option<LanguageTargetInfo> {
    let normalized = normalize_target_selector(selector);
    if normalized.is_empty() {
        return None;
    }

    for target in all_targets() {
        if normalize_target_selector(target.board_id) == normalized
            || normalize_target_selector(target.display_name) == normalized
        {
            return Some(language_target_info(target));
        }
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
    known_target(board_id)
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

pub fn connection_options_for_target(selector: &str) -> Option<Vec<LanguageConnectionOption>> {
    let target = detect_target(selector)?;
    Some(target.connection_options)
}

pub fn parse_bluetooth_endpoint(endpoint: &str) -> Option<LanguageBluetoothEndpoint> {
    language_bluetooth_endpoint(parse_board_vm_bluetooth_endpoint(endpoint).ok()?)
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
        wireless: target
            .wireless
            .iter()
            .map(language_wireless_interface)
            .collect(),
        connection_options: language_connection_options(target),
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

fn language_wireless_interface(interface: &TargetWirelessInterface) -> LanguageWirelessInterface {
    LanguageWirelessInterface {
        transport: language_wireless_transport(interface.transport),
        chip: interface.chip.to_owned(),
        command_transport: interface.command_transport,
        ota_update: interface.ota_update,
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

pub fn build_i2c_write_u8_module(
    program: I2cWriteU8Program,
    out: &mut [u8],
) -> Result<usize, LanguageCoreError> {
    Ok(write_i2c_write_u8_module(program, out)?)
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
pub extern "C" fn board_vm_language_i2c_write_u8_module_len() -> u64 {
    I2C_WRITE_U8_MODULE_LEN as u64
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
        let pico_w = known_target("raspberry-pi-pico-w").unwrap();

        assert!(targets
            .iter()
            .any(|target| target.board_id == esp32.board_id));
        assert_eq!(esp32.family, LanguageBoardFamily::Esp32);
        assert_eq!(board_family_name(esp32.family), "esp32");
        assert_eq!(esp32.runtime_id, "board-vm-esp32");
        assert_eq!(esp32.onboard_led, Some(LanguageOnboardLed::Gpio(2)));
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
        assert!(uno.capabilities.contains(&"pwm.write".to_owned()));
        assert!(uno.capabilities.contains(&"adc.read".to_owned()));
        assert!(uno.capabilities.contains(&"dac.write_u12".to_owned()));
        assert!(uno.capabilities.contains(&"i2c.open".to_owned()));
        assert!(uno.capabilities.contains(&"i2c.write_u8".to_owned()));
        assert_eq!(
            known_target("arduino-uno-r4-minima").unwrap().led_matrix,
            None
        );
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
        assert!(detect_target("definitely-not-a-board").is_none());
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
        let code = [0x00];
        let const_pool = [0xAA, 0x55];
        let expected_len = raw_module_len(code.len() as u64, const_pool.len() as u64).unwrap();
        let mut module = vec![0u8; expected_len];

        let len = build_raw_module(0, 1, &code, &const_pool, &mut module).unwrap();

        assert_eq!(len, expected_len);
        assert_eq!(
            module,
            [0x42, 0x56, 0x4D, 0x31, 0x01, 0x00, 0x01, 0x00, 0x01, 0x00, 0x02, 0xAA, 0x55,]
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
