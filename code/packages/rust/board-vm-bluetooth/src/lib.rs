use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};

use board_vm_client::{RawFrameTransport, TransportError};
use board_vm_protocol::{decode_wire_frame, encode_wire_frame, ProtocolError};
use board_vm_stream::{StreamTransport, StreamTransportError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BluetoothEndpoint {
    BleGatt(BleGattEndpoint),
    Rfcomm(RfcommEndpoint),
}

impl BluetoothEndpoint {
    pub const fn transport(&self) -> BluetoothEndpointTransport {
        match self {
            Self::BleGatt(_) => BluetoothEndpointTransport::BleGatt,
            Self::Rfcomm(_) => BluetoothEndpointTransport::Rfcomm,
        }
    }

    pub fn device(&self) -> &str {
        match self {
            Self::BleGatt(endpoint) => &endpoint.device,
            Self::Rfcomm(endpoint) => &endpoint.device,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BluetoothEndpointTransport {
    BleGatt,
    Rfcomm,
}

pub const BOARD_VM_BLE_SERVICE_UUID: &str = "6e400001-b5a3-f393-e0a9-e50e24dcca9e";
pub const BOARD_VM_BLE_WRITE_CHARACTERISTIC_UUID: &str = "6e400002-b5a3-f393-e0a9-e50e24dcca9e";
pub const BOARD_VM_BLE_NOTIFY_CHARACTERISTIC_UUID: &str = "6e400003-b5a3-f393-e0a9-e50e24dcca9e";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BleGattEndpoint {
    pub endpoint: String,
    pub device: String,
    pub service_uuid: String,
    pub write_characteristic_uuid: String,
    pub notify_characteristic_uuid: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RfcommEndpoint {
    pub endpoint: String,
    pub device: String,
    pub channel: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BluetoothDiscoveredDevice {
    pub id: String,
    pub name: Option<String>,
    pub address: Option<String>,
    pub paired: bool,
    pub service_uuids: Vec<String>,
    pub characteristic_uuids: Vec<String>,
    pub board_vm_rfcomm_channels: Vec<u8>,
}

impl BluetoothDiscoveredDevice {
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: None,
            address: None,
            paired: false,
            service_uuids: Vec::new(),
            characteristic_uuids: Vec::new(),
            board_vm_rfcomm_channels: Vec::new(),
        }
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn with_address(mut self, address: impl Into<String>) -> Self {
        self.address = Some(address.into());
        self
    }

    pub fn paired(mut self, paired: bool) -> Self {
        self.paired = paired;
        self
    }

    pub fn with_service_uuid(mut self, uuid: impl Into<String>) -> Self {
        self.service_uuids.push(uuid.into());
        self
    }

    pub fn with_characteristic_uuid(mut self, uuid: impl Into<String>) -> Self {
        self.characteristic_uuids.push(uuid.into());
        self
    }

    pub fn with_board_vm_rfcomm_channel(mut self, channel: u8) -> Self {
        self.board_vm_rfcomm_channels.push(channel);
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BluetoothEndpointCandidate {
    pub endpoint: BluetoothEndpoint,
    pub device: String,
    pub display_name: String,
    pub paired: bool,
    pub requires_pairing: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BluetoothEndpointError {
    UnsupportedScheme(String),
    MissingDevice,
    MissingServiceUuid,
    MissingWriteCharacteristicUuid,
    MissingNotifyCharacteristicUuid,
    MissingChannel,
    InvalidChannel(String),
    InvalidUuid { field: &'static str, value: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BluetoothDiscoveryError {
    UnsupportedPlatform {
        platform: &'static str,
    },
    CommandUnavailable {
        program: String,
        message: String,
    },
    CommandFailed {
        program: String,
        status: Option<i32>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BluetoothOpenError {
    UnsupportedPlatform { platform: &'static str },
    Backend { message: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BluetoothTransportError {
    Link,
    FrameTooLarge,
    Protocol(ProtocolError),
    Stream(StreamTransportError),
}

impl BluetoothTransportError {
    pub const fn as_transport_error(self) -> TransportError {
        match self {
            Self::Link => TransportError::Io,
            Self::FrameTooLarge => TransportError::ResponseTooLarge,
            Self::Protocol(_) => TransportError::Io,
            Self::Stream(error) => error.as_transport_error(),
        }
    }
}

impl From<StreamTransportError> for BluetoothTransportError {
    fn from(value: StreamTransportError) -> Self {
        Self::Stream(value)
    }
}

pub trait BleGattIo {
    fn write_characteristic(
        &mut self,
        characteristic_uuid: &str,
        bytes: &[u8],
    ) -> Result<(), BluetoothTransportError>;

    fn read_notification(
        &mut self,
        characteristic_uuid: &str,
        out: &mut [u8],
    ) -> Result<usize, BluetoothTransportError>;
}

pub trait BluetoothBackend {
    type BleGattLink: BleGattIo;
    type RfcommStream: Read + Write;

    fn open_ble_gatt(
        &mut self,
        endpoint: &BleGattEndpoint,
    ) -> Result<Self::BleGattLink, BluetoothOpenError>;

    fn open_rfcomm(
        &mut self,
        endpoint: &RfcommEndpoint,
    ) -> Result<Self::RfcommStream, BluetoothOpenError>;
}

pub trait MacosRfcommDeviceResolver {
    fn rfcomm_device_paths(&mut self) -> Result<Vec<String>, BluetoothOpenError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MacosCoreBluetoothBleOpenRequest {
    pub device: String,
    pub service_uuid: String,
    pub write_characteristic_uuid: String,
    pub notify_characteristic_uuid: String,
}

impl MacosCoreBluetoothBleOpenRequest {
    pub fn from_endpoint(endpoint: &BleGattEndpoint) -> Self {
        Self {
            device: endpoint.device.clone(),
            service_uuid: endpoint.service_uuid.clone(),
            write_characteristic_uuid: endpoint.write_characteristic_uuid.clone(),
            notify_characteristic_uuid: endpoint.notify_characteristic_uuid.clone(),
        }
    }
}

pub trait MacosCoreBluetoothBleConnector {
    type BleGattLink: BleGattIo;

    fn open_ble_gatt(
        &mut self,
        request: &MacosCoreBluetoothBleOpenRequest,
    ) -> Result<Self::BleGattLink, BluetoothOpenError>;
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct UnavailableCoreBluetoothBleConnector;

impl MacosCoreBluetoothBleConnector for UnavailableCoreBluetoothBleConnector {
    type BleGattLink = UnsupportedBleGattLink;

    fn open_ble_gatt(
        &mut self,
        request: &MacosCoreBluetoothBleOpenRequest,
    ) -> Result<Self::BleGattLink, BluetoothOpenError> {
        Err(BluetoothOpenError::Backend {
            message: format!(
                "macOS CoreBluetooth BLE GATT adapter is not wired yet for {} service {} write {} notify {}",
                request.device,
                request.service_uuid,
                request.write_characteristic_uuid,
                request.notify_characteristic_uuid
            ),
        })
    }
}

#[cfg(target_os = "macos")]
mod macos_core_bluetooth;

#[cfg(target_os = "macos")]
pub use macos_core_bluetooth::{
    MacosCoreBluetoothBleLink, MacosCoreBluetoothRuntimeBleConnector, MacosCoreBluetoothTimeouts,
};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct MacosDevRfcommDeviceResolver;

impl MacosRfcommDeviceResolver for MacosDevRfcommDeviceResolver {
    fn rfcomm_device_paths(&mut self) -> Result<Vec<String>, BluetoothOpenError> {
        let entries = fs::read_dir("/dev").map_err(|error| BluetoothOpenError::Backend {
            message: format!("failed to scan /dev for macOS RFCOMM devices: {error}"),
        })?;
        let mut paths = Vec::new();
        for entry in entries {
            let entry = entry.map_err(|error| BluetoothOpenError::Backend {
                message: format!("failed to read macOS RFCOMM device entry: {error}"),
            })?;
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if name.starts_with("cu.") || name.starts_with("tty.") {
                paths.push(format!("/dev/{name}"));
            }
        }
        Ok(paths)
    }
}

pub struct MacosBluetoothBackend<
    R = MacosDevRfcommDeviceResolver,
    C = UnavailableCoreBluetoothBleConnector,
> {
    resolver: R,
    ble_connector: C,
}

impl MacosBluetoothBackend<MacosDevRfcommDeviceResolver, UnavailableCoreBluetoothBleConnector> {
    pub fn new() -> Self {
        Self::with_resolver(MacosDevRfcommDeviceResolver)
    }
}

impl Default
    for MacosBluetoothBackend<MacosDevRfcommDeviceResolver, UnavailableCoreBluetoothBleConnector>
{
    fn default() -> Self {
        Self::new()
    }
}

impl<R> MacosBluetoothBackend<R, UnavailableCoreBluetoothBleConnector> {
    pub const fn with_resolver(resolver: R) -> Self {
        Self::with_resolver_and_ble_connector(resolver, UnavailableCoreBluetoothBleConnector)
    }
}

impl<R, C> MacosBluetoothBackend<R, C> {
    pub const fn with_resolver_and_ble_connector(resolver: R, ble_connector: C) -> Self {
        Self {
            resolver,
            ble_connector,
        }
    }

    pub const fn resolver(&self) -> &R {
        &self.resolver
    }

    pub fn resolver_mut(&mut self) -> &mut R {
        &mut self.resolver
    }

    pub const fn ble_connector(&self) -> &C {
        &self.ble_connector
    }

    pub fn ble_connector_mut(&mut self) -> &mut C {
        &mut self.ble_connector
    }
}

impl<R, C> BluetoothBackend for MacosBluetoothBackend<R, C>
where
    R: MacosRfcommDeviceResolver,
    C: MacosCoreBluetoothBleConnector,
{
    type BleGattLink = C::BleGattLink;
    type RfcommStream = File;

    fn open_ble_gatt(
        &mut self,
        endpoint: &BleGattEndpoint,
    ) -> Result<Self::BleGattLink, BluetoothOpenError> {
        let request = MacosCoreBluetoothBleOpenRequest::from_endpoint(endpoint);
        self.ble_connector.open_ble_gatt(&request)
    }

    fn open_rfcomm(
        &mut self,
        endpoint: &RfcommEndpoint,
    ) -> Result<Self::RfcommStream, BluetoothOpenError> {
        let paths = self.resolver.rfcomm_device_paths()?;
        let path = macos_rfcomm_device_path(endpoint, paths).ok_or_else(|| {
            BluetoothOpenError::Backend {
                message: format!(
                    "no macOS RFCOMM serial device found for {} channel {}",
                    endpoint.device, endpoint.channel
                ),
            }
        })?;

        OpenOptions::new()
            .read(true)
            .write(true)
            .open(&path)
            .map_err(|error| BluetoothOpenError::Backend {
                message: format!("failed to open macOS RFCOMM device {path}: {error}"),
            })
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct UnsupportedBluetoothBackend;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct UnsupportedBleGattLink;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct UnsupportedRfcommStream;

impl BluetoothBackend for UnsupportedBluetoothBackend {
    type BleGattLink = UnsupportedBleGattLink;
    type RfcommStream = UnsupportedRfcommStream;

    fn open_ble_gatt(
        &mut self,
        _endpoint: &BleGattEndpoint,
    ) -> Result<Self::BleGattLink, BluetoothOpenError> {
        Err(BluetoothOpenError::UnsupportedPlatform {
            platform: std::env::consts::OS,
        })
    }

    fn open_rfcomm(
        &mut self,
        _endpoint: &RfcommEndpoint,
    ) -> Result<Self::RfcommStream, BluetoothOpenError> {
        Err(BluetoothOpenError::UnsupportedPlatform {
            platform: std::env::consts::OS,
        })
    }
}

impl BleGattIo for UnsupportedBleGattLink {
    fn write_characteristic(
        &mut self,
        _characteristic_uuid: &str,
        _bytes: &[u8],
    ) -> Result<(), BluetoothTransportError> {
        Err(BluetoothTransportError::Link)
    }

    fn read_notification(
        &mut self,
        _characteristic_uuid: &str,
        _out: &mut [u8],
    ) -> Result<usize, BluetoothTransportError> {
        Err(BluetoothTransportError::Link)
    }
}

impl Read for UnsupportedRfcommStream {
    fn read(&mut self, _buf: &mut [u8]) -> io::Result<usize> {
        Err(unsupported_bluetooth_io_error())
    }
}

impl Write for UnsupportedRfcommStream {
    fn write(&mut self, _buf: &[u8]) -> io::Result<usize> {
        Err(unsupported_bluetooth_io_error())
    }

    fn flush(&mut self) -> io::Result<()> {
        Err(unsupported_bluetooth_io_error())
    }
}

pub struct BoardBleGattTransport<L, const WIRE_BYTES: usize = 1024> {
    endpoint: BleGattEndpoint,
    link: L,
    wire: [u8; WIRE_BYTES],
    pending: [u8; WIRE_BYTES],
    pending_len: usize,
}

impl<L, const WIRE_BYTES: usize> BoardBleGattTransport<L, WIRE_BYTES> {
    pub fn new(endpoint: BleGattEndpoint, link: L) -> Self {
        Self {
            endpoint,
            link,
            wire: [0; WIRE_BYTES],
            pending: [0; WIRE_BYTES],
            pending_len: 0,
        }
    }

    pub fn endpoint(&self) -> &BleGattEndpoint {
        &self.endpoint
    }

    pub fn link(&self) -> &L {
        &self.link
    }

    pub fn link_mut(&mut self) -> &mut L {
        &mut self.link
    }

    pub fn into_inner(self) -> L {
        self.link
    }

    pub fn send_raw_frame(&mut self, raw_frame: &[u8]) -> Result<usize, BluetoothTransportError>
    where
        L: BleGattIo,
    {
        let wire_len = encode_wire_frame(raw_frame, &mut self.wire).map_err(map_protocol_error)?;
        self.link.write_characteristic(
            &self.endpoint.write_characteristic_uuid,
            &self.wire[..wire_len],
        )?;
        Ok(wire_len)
    }

    pub fn receive_raw_frame(
        &mut self,
        raw_out: &mut [u8],
    ) -> Result<usize, BluetoothTransportError>
    where
        L: BleGattIo,
    {
        let wire_len = self.read_wire_frame()?;
        decode_wire_frame(&self.wire[..wire_len], raw_out).map_err(map_protocol_error)
    }

    pub fn exchange_raw_frame_checked(
        &mut self,
        raw_request: &[u8],
        raw_response_out: &mut [u8],
    ) -> Result<usize, BluetoothTransportError>
    where
        L: BleGattIo,
    {
        self.send_raw_frame(raw_request)?;
        self.receive_raw_frame(raw_response_out)
    }

    fn read_wire_frame(&mut self) -> Result<usize, BluetoothTransportError>
    where
        L: BleGattIo,
    {
        let mut len = 0;
        loop {
            if self.pending_len > 0 {
                let available = self.wire.len() - len;
                let count = self.pending_len.min(available);
                self.wire[len..len + count].copy_from_slice(&self.pending[..count]);
                if let Some(terminator_offset) =
                    self.pending[..count].iter().position(|byte| *byte == 0)
                {
                    let consumed = terminator_offset + 1;
                    let remaining = self.pending_len - consumed;
                    self.pending.copy_within(consumed..self.pending_len, 0);
                    self.pending_len = remaining;
                    return Ok(len + consumed);
                }
                if count < self.pending_len {
                    return Err(BluetoothTransportError::FrameTooLarge);
                }
                self.pending_len = 0;
                len += count;
            }

            if len >= self.wire.len() {
                return Err(BluetoothTransportError::FrameTooLarge);
            }

            let count = self.link.read_notification(
                &self.endpoint.notify_characteristic_uuid,
                &mut self.wire[len..],
            )?;
            if count == 0 {
                return Err(BluetoothTransportError::Link);
            }
            if count > self.wire.len() - len {
                return Err(BluetoothTransportError::FrameTooLarge);
            }

            let chunk_start = len;
            let chunk_end = len + count;
            let chunk = &self.wire[chunk_start..chunk_end];
            len = chunk_end;
            if let Some(terminator_offset) = chunk.iter().position(|byte| *byte == 0) {
                let frame_len = chunk_start + terminator_offset + 1;
                let extra_start = frame_len;
                self.pending_len = chunk_end - extra_start;
                self.pending[..self.pending_len]
                    .copy_from_slice(&self.wire[extra_start..chunk_end]);
                return Ok(frame_len);
            }
        }
    }
}

impl<L, const WIRE_BYTES: usize> RawFrameTransport for BoardBleGattTransport<L, WIRE_BYTES>
where
    L: BleGattIo,
{
    fn exchange_raw_frame(
        &mut self,
        request: &[u8],
        response_out: &mut [u8],
    ) -> Result<usize, TransportError> {
        self.exchange_raw_frame_checked(request, response_out)
            .map_err(BluetoothTransportError::as_transport_error)
    }
}

pub struct BoardRfcommTransport<S, const WIRE_BYTES: usize = 1024> {
    endpoint: RfcommEndpoint,
    inner: StreamTransport<S, WIRE_BYTES>,
}

impl<S, const WIRE_BYTES: usize> BoardRfcommTransport<S, WIRE_BYTES> {
    pub fn from_stream(endpoint: RfcommEndpoint, stream: S) -> Self {
        Self {
            endpoint,
            inner: StreamTransport::new(stream),
        }
    }

    pub fn endpoint(&self) -> &RfcommEndpoint {
        &self.endpoint
    }

    pub fn stream_transport(&self) -> &StreamTransport<S, WIRE_BYTES> {
        &self.inner
    }

    pub fn stream_transport_mut(&mut self) -> &mut StreamTransport<S, WIRE_BYTES> {
        &mut self.inner
    }

    pub fn into_inner(self) -> S {
        self.inner.into_inner()
    }

    pub fn send_raw_frame(&mut self, raw_frame: &[u8]) -> Result<usize, BluetoothTransportError>
    where
        S: Write,
    {
        Ok(self.inner.send_raw_frame(raw_frame)?)
    }

    pub fn receive_raw_frame(
        &mut self,
        raw_out: &mut [u8],
    ) -> Result<usize, BluetoothTransportError>
    where
        S: Read,
    {
        Ok(self.inner.receive_raw_frame(raw_out)?)
    }

    pub fn exchange_raw_frame_checked(
        &mut self,
        raw_request: &[u8],
        raw_response_out: &mut [u8],
    ) -> Result<usize, BluetoothTransportError>
    where
        S: Read + Write,
    {
        Ok(self
            .inner
            .exchange_raw_frame_checked(raw_request, raw_response_out)?)
    }
}

impl<S, const WIRE_BYTES: usize> RawFrameTransport for BoardRfcommTransport<S, WIRE_BYTES>
where
    S: Read + Write,
{
    fn exchange_raw_frame(
        &mut self,
        request: &[u8],
        response_out: &mut [u8],
    ) -> Result<usize, TransportError> {
        self.exchange_raw_frame_checked(request, response_out)
            .map_err(BluetoothTransportError::as_transport_error)
    }
}

pub enum BoardBluetoothTransport<L, S, const WIRE_BYTES: usize = 1024> {
    BleGatt(BoardBleGattTransport<L, WIRE_BYTES>),
    Rfcomm(BoardRfcommTransport<S, WIRE_BYTES>),
}

impl<L, S, const WIRE_BYTES: usize> BoardBluetoothTransport<L, S, WIRE_BYTES> {
    pub const fn endpoint_transport(&self) -> BluetoothEndpointTransport {
        match self {
            Self::BleGatt(_) => BluetoothEndpointTransport::BleGatt,
            Self::Rfcomm(_) => BluetoothEndpointTransport::Rfcomm,
        }
    }

    pub fn device(&self) -> &str {
        match self {
            Self::BleGatt(transport) => &transport.endpoint().device,
            Self::Rfcomm(transport) => &transport.endpoint().device,
        }
    }
}

impl<L, S, const WIRE_BYTES: usize> RawFrameTransport for BoardBluetoothTransport<L, S, WIRE_BYTES>
where
    L: BleGattIo,
    S: Read + Write,
{
    fn exchange_raw_frame(
        &mut self,
        request: &[u8],
        response_out: &mut [u8],
    ) -> Result<usize, TransportError> {
        match self {
            Self::BleGatt(transport) => transport.exchange_raw_frame(request, response_out),
            Self::Rfcomm(transport) => transport.exchange_raw_frame(request, response_out),
        }
    }
}

pub fn open_bluetooth_endpoint<B, const WIRE_BYTES: usize>(
    backend: &mut B,
    endpoint: BluetoothEndpoint,
) -> Result<BoardBluetoothTransport<B::BleGattLink, B::RfcommStream, WIRE_BYTES>, BluetoothOpenError>
where
    B: BluetoothBackend,
{
    match endpoint {
        BluetoothEndpoint::BleGatt(endpoint) => {
            let link = backend.open_ble_gatt(&endpoint)?;
            Ok(BoardBluetoothTransport::BleGatt(
                BoardBleGattTransport::new(endpoint, link),
            ))
        }
        BluetoothEndpoint::Rfcomm(endpoint) => {
            let stream = backend.open_rfcomm(&endpoint)?;
            Ok(BoardBluetoothTransport::Rfcomm(
                BoardRfcommTransport::from_stream(endpoint, stream),
            ))
        }
    }
}

pub fn macos_rfcomm_device_path<I, P>(endpoint: &RfcommEndpoint, paths: I) -> Option<String>
where
    I: IntoIterator<Item = P>,
    P: AsRef<str>,
{
    let device = endpoint.device.trim();
    if device.starts_with("/dev/") {
        return Some(device.to_owned());
    }

    let normalized_device = normalize_macos_rfcomm_token(device);
    if normalized_device.is_empty() {
        return None;
    }

    let mut matches = paths
        .into_iter()
        .map(|path| path.as_ref().to_owned())
        .filter(|path| {
            let normalized_path = normalize_macos_rfcomm_token(macos_rfcomm_file_name(path));
            normalized_path.contains(&normalized_device)
        })
        .collect::<Vec<_>>();
    matches.sort_by_key(|path| {
        (
            macos_rfcomm_path_rank(path),
            macos_rfcomm_file_name(path).len(),
            path.len(),
        )
    });
    matches.into_iter().next()
}

pub fn parse_bluetooth_endpoint(
    endpoint: &str,
) -> Result<BluetoothEndpoint, BluetoothEndpointError> {
    let endpoint = endpoint.trim();
    if endpoint.starts_with("ble://") {
        return parse_ble_gatt_endpoint(endpoint).map(BluetoothEndpoint::BleGatt);
    }
    if endpoint.starts_with("btspp://") || endpoint.starts_with("rfcomm://") {
        return parse_rfcomm_endpoint(endpoint).map(BluetoothEndpoint::Rfcomm);
    }

    let scheme = endpoint
        .split_once("://")
        .map(|(scheme, _)| scheme)
        .unwrap_or("");
    Err(BluetoothEndpointError::UnsupportedScheme(scheme.to_owned()))
}

pub fn board_vm_ble_endpoint_for_device(
    device: &str,
) -> Result<BleGattEndpoint, BluetoothEndpointError> {
    parse_ble_gatt_endpoint(&format!(
        "ble://{device}?service={BOARD_VM_BLE_SERVICE_UUID}&write={BOARD_VM_BLE_WRITE_CHARACTERISTIC_UUID}&notify={BOARD_VM_BLE_NOTIFY_CHARACTERISTIC_UUID}"
    ))
}

pub fn board_vm_rfcomm_endpoint_for_device(
    device: &str,
    channel: u8,
) -> Result<RfcommEndpoint, BluetoothEndpointError> {
    parse_rfcomm_endpoint(&format!("btspp://{device}:{channel}"))
}

pub fn board_vm_endpoint_candidates(
    devices: &[BluetoothDiscoveredDevice],
) -> Vec<BluetoothEndpointCandidate> {
    let mut candidates = Vec::new();
    for device in devices {
        let Some(selector) = bluetooth_device_selector(device) else {
            continue;
        };
        let display_name = bluetooth_device_display_name(device, &selector);

        if device_supports_board_vm_ble(device) {
            if let Ok(endpoint) = board_vm_ble_endpoint_for_device(&selector) {
                candidates.push(BluetoothEndpointCandidate {
                    endpoint: BluetoothEndpoint::BleGatt(endpoint),
                    device: selector.clone(),
                    display_name: display_name.clone(),
                    paired: device.paired,
                    requires_pairing: !device.paired,
                });
            }
        }

        let mut channels = device.board_vm_rfcomm_channels.clone();
        channels.sort_unstable();
        channels.dedup();
        for channel in channels {
            if let Ok(endpoint) = board_vm_rfcomm_endpoint_for_device(&selector, channel) {
                candidates.push(BluetoothEndpointCandidate {
                    endpoint: BluetoothEndpoint::Rfcomm(endpoint),
                    device: selector.clone(),
                    display_name: display_name.clone(),
                    paired: device.paired,
                    requires_pairing: !device.paired,
                });
            }
        }
    }
    candidates
}

pub fn discover_bluetooth_devices(
) -> Result<Vec<BluetoothDiscoveredDevice>, BluetoothDiscoveryError> {
    discover_bluetooth_devices_impl()
}

#[cfg(target_os = "macos")]
fn discover_bluetooth_devices_impl(
) -> Result<Vec<BluetoothDiscoveredDevice>, BluetoothDiscoveryError> {
    let program = "system_profiler";
    let output = std::process::Command::new(program)
        .arg("SPBluetoothDataType")
        .output()
        .map_err(|error| BluetoothDiscoveryError::CommandUnavailable {
            program: program.to_owned(),
            message: error.to_string(),
        })?;

    if !output.status.success() {
        return Err(BluetoothDiscoveryError::CommandFailed {
            program: program.to_owned(),
            status: output.status.code(),
        });
    }

    Ok(bluetooth_devices_from_macos_system_profiler(
        &String::from_utf8_lossy(&output.stdout),
    ))
}

#[cfg(target_os = "linux")]
fn discover_bluetooth_devices_impl(
) -> Result<Vec<BluetoothDiscoveredDevice>, BluetoothDiscoveryError> {
    let program = "bluetoothctl";
    let output = std::process::Command::new(program)
        .arg("devices")
        .output()
        .map_err(|error| BluetoothDiscoveryError::CommandUnavailable {
            program: program.to_owned(),
            message: error.to_string(),
        })?;

    if !output.status.success() {
        return Err(BluetoothDiscoveryError::CommandFailed {
            program: program.to_owned(),
            status: output.status.code(),
        });
    }

    let mut devices =
        bluetooth_devices_from_bluezctl_devices(&String::from_utf8_lossy(&output.stdout));
    for device in &mut devices {
        let selector = device.address.as_deref().unwrap_or(&device.id).to_owned();
        let Ok(info) = std::process::Command::new(program)
            .arg("info")
            .arg(&selector)
            .output()
        else {
            continue;
        };
        if info.status.success() {
            enrich_bluetooth_device_from_bluezctl_info(
                device,
                &String::from_utf8_lossy(&info.stdout),
            );
        }
    }

    Ok(devices)
}

#[cfg(target_os = "windows")]
fn discover_bluetooth_devices_impl(
) -> Result<Vec<BluetoothDiscoveredDevice>, BluetoothDiscoveryError> {
    let program = "powershell";
    let output = std::process::Command::new(program)
        .arg("-NoProfile")
        .arg("-NonInteractive")
        .arg("-Command")
        .arg(windows_pnp_discovery_script())
        .output()
        .map_err(|error| BluetoothDiscoveryError::CommandUnavailable {
            program: program.to_owned(),
            message: error.to_string(),
        })?;

    if !output.status.success() {
        return Err(BluetoothDiscoveryError::CommandFailed {
            program: program.to_owned(),
            status: output.status.code(),
        });
    }

    Ok(bluetooth_devices_from_windows_pnp_report(
        &String::from_utf8_lossy(&output.stdout),
    ))
}

#[cfg(all(
    not(target_os = "macos"),
    not(target_os = "linux"),
    not(target_os = "windows")
))]
fn discover_bluetooth_devices_impl(
) -> Result<Vec<BluetoothDiscoveredDevice>, BluetoothDiscoveryError> {
    Err(BluetoothDiscoveryError::UnsupportedPlatform {
        platform: std::env::consts::OS,
    })
}

pub fn bluetooth_devices_from_bluezctl_reports(
    devices_report: &str,
    info_reports: &[&str],
) -> Vec<BluetoothDiscoveredDevice> {
    let mut devices = bluetooth_devices_from_bluezctl_devices(devices_report);
    for report in info_reports {
        let Some(info_device) = bluetooth_device_from_bluezctl_info(report) else {
            continue;
        };
        let key = info_device
            .address
            .as_deref()
            .unwrap_or(&info_device.id)
            .to_owned();
        if let Some(device) = devices
            .iter_mut()
            .find(|device| bluez_device_matches(device, &key))
        {
            merge_bluetooth_device(device, info_device);
        } else {
            devices.push(info_device);
        }
    }
    devices
}

pub fn bluetooth_devices_from_windows_pnp_report(report: &str) -> Vec<BluetoothDiscoveredDevice> {
    let mut devices = Vec::new();
    for line in report.lines() {
        let Some(device) = bluetooth_device_from_windows_pnp_line(line) else {
            continue;
        };
        let key = device.address.as_deref().unwrap_or(&device.id).to_owned();
        if let Some(existing) = devices
            .iter_mut()
            .find(|existing| windows_device_matches(existing, &key))
        {
            merge_windows_bluetooth_device(existing, device);
        } else {
            devices.push(device);
        }
    }
    devices
}

pub fn bluetooth_devices_from_macos_system_profiler(
    report: &str,
) -> Vec<BluetoothDiscoveredDevice> {
    let mut devices = Vec::new();
    let mut current = None;
    let mut in_devices = false;
    let mut device_indent = None;

    for raw_line in report.lines() {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }

        let indent = leading_space_count(raw_line);
        if line.starts_with("Devices") && line.ends_with(':') {
            push_finished_bluetooth_device(&mut devices, current.take());
            in_devices = true;
            device_indent = None;
            continue;
        }

        if !in_devices {
            continue;
        }

        let Some(first_device_indent) = device_indent else {
            if let Some(name) = heading_name(line) {
                device_indent = Some(indent);
                current = Some(BluetoothDiscoveredDevice::new(name).with_name(name));
            }
            continue;
        };

        if indent < first_device_indent {
            push_finished_bluetooth_device(&mut devices, current.take());
            in_devices = false;
            device_indent = None;
            continue;
        }

        if indent == first_device_indent {
            if let Some(name) = heading_name(line) {
                push_finished_bluetooth_device(&mut devices, current.take());
                current = Some(BluetoothDiscoveredDevice::new(name).with_name(name));
                continue;
            }
        }

        let Some(device) = current.as_mut() else {
            continue;
        };

        if let Some(address) = field_value(line, "Address") {
            device.address = Some(normalize_bluetooth_address(address));
        } else if let Some(paired) = field_value(line, "Paired") {
            device.paired = yes_no_value(paired);
        } else if let Some(channel) = field_value(line, "RFCOMM Channel") {
            if let Ok(channel) = channel.trim().parse::<u8>() {
                device.board_vm_rfcomm_channels.push(channel);
            }
        } else if let Some(uuid) = field_value(line, "Characteristic UUID") {
            push_unique_uuid(&mut device.characteristic_uuids, uuid);
        } else if let Some(service) = heading_name(line) {
            push_unique_uuid(&mut device.service_uuids, service);
        }
    }

    push_finished_bluetooth_device(&mut devices, current);
    devices
}

pub fn parse_ble_gatt_endpoint(endpoint: &str) -> Result<BleGattEndpoint, BluetoothEndpointError> {
    let body = endpoint.trim().strip_prefix("ble://").ok_or_else(|| {
        BluetoothEndpointError::UnsupportedScheme(endpoint_scheme(endpoint).to_owned())
    })?;
    let (path, query) = body.split_once('?').unwrap_or((body, ""));
    let mut path_parts = path.split('/').filter(|part| !part.is_empty());
    let device = path_parts
        .next()
        .filter(|value| !value.trim().is_empty())
        .ok_or(BluetoothEndpointError::MissingDevice)?;
    let service_uuid = query_value(query, "service")
        .or_else(|| path_parts.next())
        .ok_or(BluetoothEndpointError::MissingServiceUuid)?;
    let write_characteristic_uuid = query_value(query, "write")
        .or_else(|| query_value(query, "tx"))
        .or_else(|| path_parts.next())
        .ok_or(BluetoothEndpointError::MissingWriteCharacteristicUuid)?;
    let notify_characteristic_uuid = query_value(query, "notify")
        .or_else(|| query_value(query, "rx"))
        .or_else(|| path_parts.next())
        .ok_or(BluetoothEndpointError::MissingNotifyCharacteristicUuid)?;

    validate_uuid("service", service_uuid)?;
    validate_uuid("write", write_characteristic_uuid)?;
    validate_uuid("notify", notify_characteristic_uuid)?;

    Ok(BleGattEndpoint {
        endpoint: endpoint.trim().to_owned(),
        device: device.to_owned(),
        service_uuid: service_uuid.to_owned(),
        write_characteristic_uuid: write_characteristic_uuid.to_owned(),
        notify_characteristic_uuid: notify_characteristic_uuid.to_owned(),
    })
}

pub fn parse_rfcomm_endpoint(endpoint: &str) -> Result<RfcommEndpoint, BluetoothEndpointError> {
    let body = endpoint
        .trim()
        .strip_prefix("btspp://")
        .or_else(|| endpoint.trim().strip_prefix("rfcomm://"))
        .ok_or_else(|| {
            BluetoothEndpointError::UnsupportedScheme(endpoint_scheme(endpoint).to_owned())
        })?;
    let (device, channel) = body
        .rsplit_once(':')
        .ok_or(BluetoothEndpointError::MissingChannel)?;
    if device.trim().is_empty() {
        return Err(BluetoothEndpointError::MissingDevice);
    }
    let channel_text = channel;
    let channel = channel_text
        .parse::<u8>()
        .map_err(|_| BluetoothEndpointError::InvalidChannel(channel_text.to_owned()))?;
    if !(1..=30).contains(&channel) {
        return Err(BluetoothEndpointError::InvalidChannel(
            channel_text.to_owned(),
        ));
    }

    Ok(RfcommEndpoint {
        endpoint: endpoint.trim().to_owned(),
        device: device.to_owned(),
        channel,
    })
}

fn map_protocol_error(error: ProtocolError) -> BluetoothTransportError {
    match error {
        ProtocolError::OutputTooSmall | ProtocolError::PayloadTooLarge => {
            BluetoothTransportError::FrameTooLarge
        }
        other => BluetoothTransportError::Protocol(other),
    }
}

fn unsupported_bluetooth_io_error() -> io::Error {
    io::Error::new(
        io::ErrorKind::Other,
        "Board VM Bluetooth backend is unsupported on this platform",
    )
}

fn macos_rfcomm_file_name(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

fn macos_rfcomm_path_rank(path: &str) -> u8 {
    let file_name = macos_rfcomm_file_name(path);
    if file_name.starts_with("cu.") {
        0
    } else if file_name.starts_with("tty.") {
        1
    } else {
        2
    }
}

fn normalize_macos_rfcomm_token(value: &str) -> String {
    value
        .bytes()
        .filter(|byte| byte.is_ascii_alphanumeric())
        .map(|byte| byte.to_ascii_lowercase() as char)
        .collect()
}

fn bluetooth_devices_from_bluezctl_devices(report: &str) -> Vec<BluetoothDiscoveredDevice> {
    report
        .lines()
        .filter_map(bluetooth_device_from_bluezctl_devices_line)
        .collect()
}

fn bluetooth_device_from_bluezctl_devices_line(line: &str) -> Option<BluetoothDiscoveredDevice> {
    let line = line.trim();
    let line = line.strip_prefix("[NEW] ").unwrap_or(line);
    let body = line.strip_prefix("Device ")?;
    let mut parts = body.split_whitespace();
    let address = normalize_bluetooth_address(parts.next()?);
    let name = body[address.len()..].trim();
    let mut device = BluetoothDiscoveredDevice::new(&address).with_address(&address);
    if !name.is_empty() {
        device.name = Some(name.to_owned());
    }
    Some(device)
}

fn bluetooth_device_from_bluezctl_info(report: &str) -> Option<BluetoothDiscoveredDevice> {
    let mut device = None;
    for raw_line in report.lines() {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }

        if let Some(address) = bluez_info_header_address(line) {
            let address = normalize_bluetooth_address(address);
            device = Some(BluetoothDiscoveredDevice::new(&address).with_address(&address));
            continue;
        }

        let Some(device) = device.as_mut() else {
            continue;
        };

        if let Some(name) = field_value(line, "Name") {
            device.name = Some(name.to_owned());
        } else if let Some(alias) = field_value(line, "Alias") {
            if device.name.is_none() {
                device.name = Some(alias.to_owned());
            }
        } else if let Some(paired) = field_value(line, "Paired") {
            device.paired = yes_no_value(paired);
        } else if let Some(channel) = field_value(line, "RFCOMM Channel") {
            if let Ok(channel) = channel.trim().parse::<u8>() {
                device.board_vm_rfcomm_channels.push(channel);
            }
        } else if let Some(uuid) = field_value(line, "Service UUID") {
            push_unique_uuid(&mut device.service_uuids, uuid);
        } else if let Some(uuid) = field_value(line, "Characteristic UUID") {
            push_unique_uuid(&mut device.characteristic_uuids, uuid);
        } else if let Some(uuid) = field_value(line, "UUID").and_then(bluez_uuid_value) {
            push_unique_uuid(&mut device.service_uuids, &uuid);
            if uuid.eq_ignore_ascii_case(BOARD_VM_BLE_WRITE_CHARACTERISTIC_UUID)
                || uuid.eq_ignore_ascii_case(BOARD_VM_BLE_NOTIFY_CHARACTERISTIC_UUID)
            {
                push_unique_uuid(&mut device.characteristic_uuids, &uuid);
            }
        }
    }
    device
}

#[cfg(target_os = "linux")]
fn enrich_bluetooth_device_from_bluezctl_info(
    device: &mut BluetoothDiscoveredDevice,
    report: &str,
) {
    if let Some(info_device) = bluetooth_device_from_bluezctl_info(report) {
        merge_bluetooth_device(device, info_device);
    }
}

fn merge_bluetooth_device(
    device: &mut BluetoothDiscoveredDevice,
    info_device: BluetoothDiscoveredDevice,
) {
    if let Some(name) = info_device.name {
        device.name = Some(name);
    }
    if let Some(address) = info_device.address {
        device.address = Some(address);
    }
    device.paired = info_device.paired;
    for uuid in info_device.service_uuids {
        push_unique_uuid(&mut device.service_uuids, &uuid);
    }
    for uuid in info_device.characteristic_uuids {
        push_unique_uuid(&mut device.characteristic_uuids, &uuid);
    }
    for channel in info_device.board_vm_rfcomm_channels {
        if !device.board_vm_rfcomm_channels.contains(&channel) {
            device.board_vm_rfcomm_channels.push(channel);
        }
    }
}

fn bluez_device_matches(device: &BluetoothDiscoveredDevice, key: &str) -> bool {
    let normalized_key = normalize_bluetooth_address(key);
    device.id.eq_ignore_ascii_case(key)
        || device.id.eq_ignore_ascii_case(&normalized_key)
        || device
            .address
            .as_deref()
            .is_some_and(|address| address.eq_ignore_ascii_case(&normalized_key))
}

fn bluez_info_header_address(line: &str) -> Option<&str> {
    let body = line.strip_prefix("Device ")?;
    body.split_whitespace().next()
}

fn bluez_uuid_value(value: &str) -> Option<String> {
    let value = value.trim();
    if is_short_uuid(value) || is_canonical_uuid(value) {
        return Some(value.to_owned());
    }

    if let Some((_, suffix)) = value.rsplit_once('(') {
        let candidate = suffix.trim().strip_suffix(')')?.trim();
        if is_short_uuid(candidate) || is_canonical_uuid(candidate) {
            return Some(candidate.to_owned());
        }
    }

    value.split_whitespace().find_map(|token| {
        let token = token.trim_matches(|ch| matches!(ch, '(' | ')' | ',' | ';'));
        (is_short_uuid(token) || is_canonical_uuid(token)).then(|| token.to_owned())
    })
}

#[cfg(target_os = "windows")]
fn windows_pnp_discovery_script() -> &'static str {
    r#"$items = Get-CimInstance Win32_PnPEntity | Where-Object {
  $_.PNPClass -eq 'Bluetooth' -or ($_.PNPClass -eq 'Ports' -and $_.Name -like '*Bluetooth*')
}
foreach ($item in $items) {
  $name = (($item.Name -as [string]) -replace "`t", " " -replace "`r|`n", " ").Trim()
  $id = (($item.PNPDeviceID -as [string]) -replace "`t", " " -replace "`r|`n", " ").Trim()
  $status = (($item.Status -as [string]) -replace "`t", " " -replace "`r|`n", " ").Trim()
  if ($id.Length -gt 0) {
    "$name`t$id`t$status"
  }
}"#
}

fn bluetooth_device_from_windows_pnp_line(line: &str) -> Option<BluetoothDiscoveredDevice> {
    let (name, instance_id, status) = windows_pnp_row(line)?;
    let address = windows_instance_address(instance_id);
    let id = address
        .clone()
        .unwrap_or_else(|| instance_id.trim().to_owned());
    if id.trim().is_empty() {
        return None;
    }

    let mut device = BluetoothDiscoveredDevice::new(&id);
    if let Some(address) = address {
        device.address = Some(address);
    }
    if !name.trim().is_empty() {
        device.name = Some(name.trim().to_owned());
    }
    device.paired = windows_pnp_status_is_available(status);

    for uuid in windows_instance_uuids(instance_id) {
        push_unique_uuid(&mut device.service_uuids, &uuid);
        if uuid.eq_ignore_ascii_case(BOARD_VM_BLE_WRITE_CHARACTERISTIC_UUID)
            || uuid.eq_ignore_ascii_case(BOARD_VM_BLE_NOTIFY_CHARACTERISTIC_UUID)
        {
            push_unique_uuid(&mut device.characteristic_uuids, &uuid);
        }
    }

    if let Some(channel) =
        windows_rfcomm_channel_value(instance_id).or_else(|| windows_rfcomm_channel_value(name))
    {
        device.board_vm_rfcomm_channels.push(channel);
    }

    Some(device)
}

fn windows_pnp_row(line: &str) -> Option<(&str, &str, &str)> {
    let mut parts = line.trim().splitn(3, '\t');
    let name = parts.next()?.trim();
    let instance_id = parts.next()?.trim();
    let status = parts.next().unwrap_or("").trim();
    (!instance_id.is_empty()).then_some((name, instance_id, status))
}

fn windows_device_matches(device: &BluetoothDiscoveredDevice, key: &str) -> bool {
    let normalized_key = normalize_bluetooth_address(key);
    device.id.eq_ignore_ascii_case(key)
        || device.id.eq_ignore_ascii_case(&normalized_key)
        || device
            .address
            .as_deref()
            .is_some_and(|address| address.eq_ignore_ascii_case(&normalized_key))
}

fn merge_windows_bluetooth_device(
    device: &mut BluetoothDiscoveredDevice,
    info_device: BluetoothDiscoveredDevice,
) {
    let name = device.name.clone();
    let paired = device.paired || info_device.paired;
    merge_bluetooth_device(device, info_device);
    if name.is_some() {
        device.name = name;
    }
    device.paired = paired;
}

fn windows_instance_address(instance_id: &str) -> Option<String> {
    let upper = instance_id.to_ascii_uppercase();
    for (index, _) in upper.match_indices("DEV_") {
        let mut hex = String::new();
        for ch in upper[index + 4..].chars() {
            if ch.is_ascii_hexdigit() {
                hex.push(ch);
                if hex.len() == 12 {
                    return Some(format_bluetooth_hex_address(&hex));
                }
            } else if matches!(ch, ':' | '-') {
                continue;
            } else {
                break;
            }
        }
    }
    None
}

fn format_bluetooth_hex_address(hex: &str) -> String {
    let mut address = String::with_capacity(17);
    for (index, ch) in hex.chars().take(12).enumerate() {
        if index > 0 && index % 2 == 0 {
            address.push(':');
        }
        address.push(ch.to_ascii_uppercase());
    }
    address
}

fn windows_instance_uuids(instance_id: &str) -> Vec<String> {
    let mut uuids = Vec::new();
    let mut rest = instance_id;
    while let Some(start) = rest.find('{') {
        let after_start = &rest[start + 1..];
        let Some(end) = after_start.find('}') else {
            break;
        };
        push_unique_uuid(&mut uuids, &after_start[..end]);
        rest = &after_start[end + 1..];
    }
    uuids
}

fn windows_rfcomm_channel_value(value: &str) -> Option<u8> {
    ["RFCOMMCHANNEL_", "RFCOMM_CHANNEL_", "CHANNEL_"]
        .into_iter()
        .find_map(|marker| {
            let upper = value.to_ascii_uppercase();
            let (_, suffix) = upper.split_once(marker)?;
            let digits: String = suffix
                .chars()
                .take_while(|ch| ch.is_ascii_digit())
                .collect();
            let channel = digits.parse::<u8>().ok()?;
            (1..=30).contains(&channel).then_some(channel)
        })
}

fn windows_pnp_status_is_available(status: &str) -> bool {
    matches!(
        status.trim().to_ascii_lowercase().as_str(),
        "ok" | "started" | "running"
    )
}

fn endpoint_scheme(endpoint: &str) -> &str {
    endpoint
        .split_once("://")
        .map(|(scheme, _)| scheme)
        .unwrap_or("")
}

fn bluetooth_device_selector(device: &BluetoothDiscoveredDevice) -> Option<String> {
    non_empty_string(device.address.as_deref())
        .or_else(|| non_empty_string(Some(&device.id)))
        .or_else(|| non_empty_string(device.name.as_deref()))
}

fn bluetooth_device_display_name(device: &BluetoothDiscoveredDevice, fallback: &str) -> String {
    non_empty_string(device.name.as_deref()).unwrap_or_else(|| fallback.to_owned())
}

fn non_empty_string(value: Option<&str>) -> Option<String> {
    let value = value?.trim();
    (!value.is_empty()).then(|| value.to_owned())
}

fn device_supports_board_vm_ble(device: &BluetoothDiscoveredDevice) -> bool {
    contains_uuid(&device.service_uuids, BOARD_VM_BLE_SERVICE_UUID)
        || contains_uuid(
            &device.characteristic_uuids,
            BOARD_VM_BLE_WRITE_CHARACTERISTIC_UUID,
        ) && contains_uuid(
            &device.characteristic_uuids,
            BOARD_VM_BLE_NOTIFY_CHARACTERISTIC_UUID,
        )
}

fn contains_uuid(values: &[String], needle: &str) -> bool {
    values
        .iter()
        .any(|value| value.trim().eq_ignore_ascii_case(needle))
}

fn push_finished_bluetooth_device(
    devices: &mut Vec<BluetoothDiscoveredDevice>,
    device: Option<BluetoothDiscoveredDevice>,
) {
    if let Some(device) = device {
        if !device.id.trim().is_empty()
            || device
                .name
                .as_deref()
                .is_some_and(|name| !name.trim().is_empty())
            || device
                .address
                .as_deref()
                .is_some_and(|address| !address.trim().is_empty())
        {
            devices.push(device);
        }
    }
}

fn leading_space_count(line: &str) -> usize {
    line.bytes().take_while(|byte| *byte == b' ').count()
}

fn heading_name(line: &str) -> Option<&str> {
    line.strip_suffix(':')
        .map(str::trim)
        .filter(|name| !name.is_empty())
}

fn field_value<'a>(line: &'a str, field: &str) -> Option<&'a str> {
    let (name, value) = line.split_once(':')?;
    (name.trim().eq_ignore_ascii_case(field) && !value.trim().is_empty()).then(|| value.trim())
}

fn normalize_bluetooth_address(address: &str) -> String {
    address.trim().replace('-', ":").to_ascii_uppercase()
}

fn yes_no_value(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "yes" | "true" | "1"
    )
}

fn push_unique_uuid(values: &mut Vec<String>, uuid: &str) {
    let uuid = uuid.trim();
    if !is_short_uuid(uuid) && !is_canonical_uuid(uuid) {
        return;
    }

    let uuid = uuid.to_ascii_lowercase();
    if !values.iter().any(|value| value == &uuid) {
        values.push(uuid);
    }
}

fn query_value<'a>(query: &'a str, key: &str) -> Option<&'a str> {
    query.split('&').find_map(|pair| {
        let (name, value) = pair.split_once('=')?;
        (name == key && !value.is_empty()).then_some(value)
    })
}

fn validate_uuid(field: &'static str, value: &str) -> Result<(), BluetoothEndpointError> {
    if is_short_uuid(value) || is_canonical_uuid(value) {
        return Ok(());
    }

    Err(BluetoothEndpointError::InvalidUuid {
        field,
        value: value.to_owned(),
    })
}

fn is_short_uuid(value: &str) -> bool {
    matches!(value.len(), 4 | 8) && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn is_canonical_uuid(value: &str) -> bool {
    if value.len() != 36 {
        return false;
    }
    value.bytes().enumerate().all(|(index, byte)| {
        matches!(index, 8 | 13 | 18 | 23) && byte == b'-'
            || !matches!(index, 8 | 13 | 18 | 23) && byte.is_ascii_hexdigit()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;
    use std::io::{self, Cursor};

    const SERVICE_UUID: &str = "6e400001-b5a3-f393-e0a9-e50e24dcca9e";
    const WRITE_UUID: &str = "6e400002-b5a3-f393-e0a9-e50e24dcca9e";
    const NOTIFY_UUID: &str = "6e400003-b5a3-f393-e0a9-e50e24dcca9e";

    #[derive(Default)]
    struct FakeBleGattLink {
        writes: Vec<(String, Vec<u8>)>,
        notifications: VecDeque<Vec<u8>>,
        fail_writes: bool,
        fail_reads: bool,
    }

    impl BleGattIo for FakeBleGattLink {
        fn write_characteristic(
            &mut self,
            characteristic_uuid: &str,
            bytes: &[u8],
        ) -> Result<(), BluetoothTransportError> {
            if self.fail_writes {
                return Err(BluetoothTransportError::Link);
            }
            self.writes
                .push((characteristic_uuid.to_owned(), bytes.to_vec()));
            Ok(())
        }

        fn read_notification(
            &mut self,
            characteristic_uuid: &str,
            out: &mut [u8],
        ) -> Result<usize, BluetoothTransportError> {
            assert_eq!(characteristic_uuid, NOTIFY_UUID);
            if self.fail_reads {
                return Err(BluetoothTransportError::Link);
            }
            let notification = self
                .notifications
                .pop_front()
                .ok_or(BluetoothTransportError::Link)?;
            let count = notification.len();
            out[..count].copy_from_slice(&notification);
            Ok(count)
        }
    }

    struct FakeRfcommStream {
        read: Cursor<Vec<u8>>,
        written: Vec<u8>,
        fail_reads: bool,
        fail_writes: bool,
    }

    impl FakeRfcommStream {
        fn new(read: Vec<u8>) -> Self {
            Self {
                read: Cursor::new(read),
                written: Vec::new(),
                fail_reads: false,
                fail_writes: false,
            }
        }

        fn with_read_failure(mut self) -> Self {
            self.fail_reads = true;
            self
        }

        fn with_write_failure(mut self) -> Self {
            self.fail_writes = true;
            self
        }
    }

    impl Read for FakeRfcommStream {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            if self.fail_reads {
                return Err(io::Error::from(io::ErrorKind::BrokenPipe));
            }
            self.read.read(buf)
        }
    }

    impl Write for FakeRfcommStream {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            if self.fail_writes {
                return Err(io::Error::from(io::ErrorKind::BrokenPipe));
            }
            self.written.extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    #[derive(Default)]
    struct FakeBluetoothBackend {
        ble_notifications: VecDeque<Vec<u8>>,
        rfcomm_read: Vec<u8>,
        ble_opened: Vec<String>,
        rfcomm_opened: Vec<(String, u8)>,
        ble_open_error: Option<BluetoothOpenError>,
        rfcomm_open_error: Option<BluetoothOpenError>,
    }

    impl BluetoothBackend for FakeBluetoothBackend {
        type BleGattLink = FakeBleGattLink;
        type RfcommStream = FakeRfcommStream;

        fn open_ble_gatt(
            &mut self,
            endpoint: &BleGattEndpoint,
        ) -> Result<Self::BleGattLink, BluetoothOpenError> {
            self.ble_opened.push(endpoint.device.clone());
            if let Some(error) = self.ble_open_error.take() {
                return Err(error);
            }
            Ok(FakeBleGattLink {
                writes: Vec::new(),
                notifications: std::mem::take(&mut self.ble_notifications),
                ..FakeBleGattLink::default()
            })
        }

        fn open_rfcomm(
            &mut self,
            endpoint: &RfcommEndpoint,
        ) -> Result<Self::RfcommStream, BluetoothOpenError> {
            self.rfcomm_opened
                .push((endpoint.device.clone(), endpoint.channel));
            if let Some(error) = self.rfcomm_open_error.take() {
                return Err(error);
            }
            Ok(FakeRfcommStream::new(std::mem::take(&mut self.rfcomm_read)))
        }
    }

    #[derive(Default)]
    struct FixedRfcommPaths {
        paths: Vec<String>,
    }

    impl MacosRfcommDeviceResolver for FixedRfcommPaths {
        fn rfcomm_device_paths(&mut self) -> Result<Vec<String>, BluetoothOpenError> {
            Ok(self.paths.clone())
        }
    }

    #[derive(Default)]
    struct FakeCoreBluetoothBleConnector {
        requests: Vec<MacosCoreBluetoothBleOpenRequest>,
        notifications: VecDeque<Vec<u8>>,
    }

    impl MacosCoreBluetoothBleConnector for FakeCoreBluetoothBleConnector {
        type BleGattLink = FakeBleGattLink;

        fn open_ble_gatt(
            &mut self,
            request: &MacosCoreBluetoothBleOpenRequest,
        ) -> Result<Self::BleGattLink, BluetoothOpenError> {
            self.requests.push(request.clone());
            Ok(FakeBleGattLink {
                writes: Vec::new(),
                notifications: std::mem::take(&mut self.notifications),
                ..FakeBleGattLink::default()
            })
        }
    }

    #[test]
    fn parses_ble_gatt_query_endpoint() {
        let endpoint = parse_ble_gatt_endpoint(&format!(
            "ble://AA:BB:CC:DD:EE:FF?service={SERVICE_UUID}&write={WRITE_UUID}&notify={NOTIFY_UUID}"
        ))
        .unwrap();

        assert_eq!(endpoint.device, "AA:BB:CC:DD:EE:FF");
        assert_eq!(endpoint.service_uuid, SERVICE_UUID);
        assert_eq!(endpoint.write_characteristic_uuid, WRITE_UUID);
        assert_eq!(endpoint.notify_characteristic_uuid, NOTIFY_UUID);
    }

    #[test]
    fn parses_ble_gatt_path_endpoint_and_short_uuids() {
        let endpoint = parse_ble_gatt_endpoint("ble://uno-r4-wifi/180f/2a19/2a1a").unwrap();

        assert_eq!(endpoint.device, "uno-r4-wifi");
        assert_eq!(endpoint.service_uuid, "180f");
        assert_eq!(endpoint.write_characteristic_uuid, "2a19");
        assert_eq!(endpoint.notify_characteristic_uuid, "2a1a");
    }

    #[test]
    fn rejects_incomplete_ble_gatt_endpoint() {
        assert_eq!(
            parse_ble_gatt_endpoint("ble://esp32?service=180f").unwrap_err(),
            BluetoothEndpointError::MissingWriteCharacteristicUuid
        );
        assert_eq!(
            parse_ble_gatt_endpoint(&format!(
                "ble://esp32?service={SERVICE_UUID}&write=not-a-uuid&notify={NOTIFY_UUID}"
            ))
            .unwrap_err(),
            BluetoothEndpointError::InvalidUuid {
                field: "write",
                value: "not-a-uuid".to_owned()
            }
        );
    }

    #[test]
    fn parses_bluetooth_classic_rfcomm_endpoints() {
        let btspp = parse_rfcomm_endpoint("btspp://ESP32-BoardVM:1").unwrap();
        let rfcomm = parse_bluetooth_endpoint("rfcomm://AA:BB:CC:DD:EE:FF:12").unwrap();

        assert_eq!(btspp.device, "ESP32-BoardVM");
        assert_eq!(btspp.channel, 1);
        assert_eq!(
            rfcomm,
            BluetoothEndpoint::Rfcomm(RfcommEndpoint {
                endpoint: "rfcomm://AA:BB:CC:DD:EE:FF:12".to_owned(),
                device: "AA:BB:CC:DD:EE:FF".to_owned(),
                channel: 12
            })
        );
    }

    #[test]
    fn rejects_bad_rfcomm_channels() {
        assert_eq!(
            parse_rfcomm_endpoint("btspp://ESP32-BoardVM:0").unwrap_err(),
            BluetoothEndpointError::InvalidChannel("0".to_owned())
        );
        assert_eq!(
            parse_rfcomm_endpoint("btspp://ESP32-BoardVM:31").unwrap_err(),
            BluetoothEndpointError::InvalidChannel("31".to_owned())
        );
        assert_eq!(
            parse_rfcomm_endpoint("btspp://ESP32-BoardVM").unwrap_err(),
            BluetoothEndpointError::MissingChannel
        );
    }

    #[test]
    fn dispatches_supported_bluetooth_endpoint_schemes() {
        assert_eq!(
            parse_bluetooth_endpoint(&format!(
                "ble://esp32?service={SERVICE_UUID}&write={WRITE_UUID}&notify={NOTIFY_UUID}"
            ))
            .unwrap()
            .transport(),
            BluetoothEndpointTransport::BleGatt
        );
        assert_eq!(
            parse_bluetooth_endpoint("btspp://ESP32-BoardVM:3")
                .unwrap()
                .transport(),
            BluetoothEndpointTransport::Rfcomm
        );
        assert_eq!(
            parse_bluetooth_endpoint("tcp://board-vm.local:4170").unwrap_err(),
            BluetoothEndpointError::UnsupportedScheme("tcp".to_owned())
        );
    }

    #[test]
    fn builds_board_vm_ble_endpoint_from_discovered_service() {
        let devices = vec![
            BluetoothDiscoveredDevice::new("ignore-me")
                .with_name("Headphones")
                .with_service_uuid("180f"),
            BluetoothDiscoveredDevice::new("dev-1")
                .with_name("Uno R4 Board VM")
                .with_address("AA:BB:CC:DD:EE:FF")
                .with_service_uuid(BOARD_VM_BLE_SERVICE_UUID.to_ascii_uppercase()),
        ];

        let candidates = board_vm_endpoint_candidates(&devices);

        assert_eq!(candidates.len(), 1);
        let candidate = &candidates[0];
        assert_eq!(candidate.display_name, "Uno R4 Board VM");
        assert_eq!(candidate.device, "AA:BB:CC:DD:EE:FF");
        assert!(!candidate.paired);
        assert!(candidate.requires_pairing);
        match &candidate.endpoint {
            BluetoothEndpoint::BleGatt(endpoint) => {
                assert_eq!(endpoint.device, "AA:BB:CC:DD:EE:FF");
                assert_eq!(endpoint.service_uuid, BOARD_VM_BLE_SERVICE_UUID);
                assert_eq!(
                    endpoint.write_characteristic_uuid,
                    BOARD_VM_BLE_WRITE_CHARACTERISTIC_UUID
                );
                assert_eq!(
                    endpoint.notify_characteristic_uuid,
                    BOARD_VM_BLE_NOTIFY_CHARACTERISTIC_UUID
                );
            }
            other => panic!("expected BLE GATT endpoint, got {other:?}"),
        }
    }

    #[test]
    fn builds_board_vm_rfcomm_candidates_from_discovered_channels() {
        let devices = vec![BluetoothDiscoveredDevice::new("esp32-board-vm")
            .with_name("ESP32 Board VM")
            .paired(true)
            .with_board_vm_rfcomm_channel(3)
            .with_board_vm_rfcomm_channel(3)
            .with_board_vm_rfcomm_channel(31)];

        let candidates = board_vm_endpoint_candidates(&devices);

        assert_eq!(candidates.len(), 1);
        let candidate = &candidates[0];
        assert_eq!(candidate.display_name, "ESP32 Board VM");
        assert_eq!(candidate.device, "esp32-board-vm");
        assert!(candidate.paired);
        assert!(!candidate.requires_pairing);
        match &candidate.endpoint {
            BluetoothEndpoint::Rfcomm(endpoint) => {
                assert_eq!(endpoint.device, "esp32-board-vm");
                assert_eq!(endpoint.channel, 3);
                assert_eq!(endpoint.endpoint, "btspp://esp32-board-vm:3");
            }
            other => panic!("expected RFCOMM endpoint, got {other:?}"),
        }
    }

    #[test]
    fn builds_ble_candidate_when_characteristics_are_discovered_after_service_scan() {
        let devices = vec![BluetoothDiscoveredDevice::new("ble-device")
            .with_characteristic_uuid(BOARD_VM_BLE_WRITE_CHARACTERISTIC_UUID)
            .with_characteristic_uuid(BOARD_VM_BLE_NOTIFY_CHARACTERISTIC_UUID)];

        let candidates = board_vm_endpoint_candidates(&devices);

        assert_eq!(candidates.len(), 1);
        assert!(matches!(
            candidates[0].endpoint,
            BluetoothEndpoint::BleGatt(_)
        ));
    }

    #[test]
    fn parses_macos_system_profiler_bluetooth_devices() {
        let report = r#"
Bluetooth:

      Bluetooth Controller:
          Address: F8-FF-C2-00-00-00

      Devices (Paired, Configured, etc.):
          ESP32 Board VM:
              Address: aa-bb-cc-dd-ee-ff
              Paired: Yes
              Services:
                  Serial Port:
                      RFCOMM Channel: 3
                      RFCOMM Channel: 31
          Uno R4 Board VM:
              Address: 11-22-33-44-55-66
              Paired: No
              Services:
                  6E400001-B5A3-F393-E0A9-E50E24DCCA9E:
          Headphones:
              Address: 00-11-22-33-44-55
              Paired: Yes
              Services:
                  180F:

      Device Cache:
          Ignored:
              Address: FF-FF-FF-FF-FF-FF
"#;

        let devices = bluetooth_devices_from_macos_system_profiler(report);

        assert_eq!(devices.len(), 3);
        assert_eq!(devices[0].name.as_deref(), Some("ESP32 Board VM"));
        assert_eq!(devices[0].address.as_deref(), Some("AA:BB:CC:DD:EE:FF"));
        assert!(devices[0].paired);
        assert_eq!(devices[0].board_vm_rfcomm_channels, vec![3, 31]);
        assert_eq!(devices[1].name.as_deref(), Some("Uno R4 Board VM"));
        assert_eq!(devices[1].address.as_deref(), Some("11:22:33:44:55:66"));
        assert!(!devices[1].paired);
        assert_eq!(
            devices[1].service_uuids,
            vec![BOARD_VM_BLE_SERVICE_UUID.to_owned()]
        );
        assert_eq!(devices[2].service_uuids, vec!["180f".to_owned()]);

        let candidates = board_vm_endpoint_candidates(&devices);

        assert_eq!(candidates.len(), 2);
        assert!(candidates
            .iter()
            .any(|candidate| matches!(candidate.endpoint, BluetoothEndpoint::Rfcomm(_))));
        assert!(candidates
            .iter()
            .any(|candidate| matches!(candidate.endpoint, BluetoothEndpoint::BleGatt(_))));
    }

    #[test]
    fn parses_linux_bluezctl_bluetooth_devices() {
        let devices_report = r#"
Device AA:BB:CC:DD:EE:FF ESP32 Board VM
[NEW] Device 11:22:33:44:55:66 Uno R4 Board VM
Device 00:11:22:33:44:55 Headphones
"#;
        let esp32_info = r#"
Device AA:BB:CC:DD:EE:FF (public)
        Name: ESP32 Board VM
        Alias: ESP32 Board VM
        Paired: yes
        UUID: Serial Port               (00001101-0000-1000-8000-00805f9b34fb)
        RFCOMM Channel: 3
"#;
        let uno_info = r#"
Device 11:22:33:44:55:66 (public)
        Name: Uno R4 Board VM
        Paired: no
        UUID: Nordic UART Service       (6E400001-B5A3-F393-E0A9-E50E24DCCA9E)
        UUID: Vendor specific           (6E400002-B5A3-F393-E0A9-E50E24DCCA9E)
        UUID: Vendor specific           (6E400003-B5A3-F393-E0A9-E50E24DCCA9E)
"#;

        let devices =
            bluetooth_devices_from_bluezctl_reports(devices_report, &[esp32_info, uno_info]);

        assert_eq!(devices.len(), 3);
        assert_eq!(devices[0].name.as_deref(), Some("ESP32 Board VM"));
        assert_eq!(devices[0].address.as_deref(), Some("AA:BB:CC:DD:EE:FF"));
        assert!(devices[0].paired);
        assert_eq!(devices[0].board_vm_rfcomm_channels, vec![3]);
        assert_eq!(devices[1].name.as_deref(), Some("Uno R4 Board VM"));
        assert_eq!(devices[1].address.as_deref(), Some("11:22:33:44:55:66"));
        assert!(!devices[1].paired);
        assert_eq!(
            devices[1].service_uuids,
            vec![
                BOARD_VM_BLE_SERVICE_UUID.to_owned(),
                BOARD_VM_BLE_WRITE_CHARACTERISTIC_UUID.to_owned(),
                BOARD_VM_BLE_NOTIFY_CHARACTERISTIC_UUID.to_owned(),
            ]
        );
        assert_eq!(
            devices[1].characteristic_uuids,
            vec![
                BOARD_VM_BLE_WRITE_CHARACTERISTIC_UUID.to_owned(),
                BOARD_VM_BLE_NOTIFY_CHARACTERISTIC_UUID.to_owned(),
            ]
        );

        let candidates = board_vm_endpoint_candidates(&devices);

        assert_eq!(candidates.len(), 2);
        assert!(candidates.iter().any(|candidate| {
            candidate.device == "AA:BB:CC:DD:EE:FF"
                && matches!(candidate.endpoint, BluetoothEndpoint::Rfcomm(_))
        }));
        assert!(candidates.iter().any(|candidate| {
            candidate.device == "11:22:33:44:55:66"
                && matches!(candidate.endpoint, BluetoothEndpoint::BleGatt(_))
                && candidate.requires_pairing
        }));
    }

    #[test]
    fn parses_windows_pnp_bluetooth_devices() {
        let report = concat!(
            "Uno R4 Board VM\tBTHLEDEVICE\\DEV_112233445566\\8&ABC&0&BLUETOOTHDEVICE_112233445566\tOK\n",
            "Uno R4 Board VM UART\tBTHLEENUM\\{6E400001-B5A3-F393-E0A9-E50E24DCCA9E}_DEV_112233445566\\8&ABC&0&BLUETOOTHDEVICE_112233445566\tOK\n",
            "Uno R4 Board VM TX\tBTHLEENUM\\{6E400002-B5A3-F393-E0A9-E50E24DCCA9E}_DEV_112233445566\\8&ABC&0&BLUETOOTHDEVICE_112233445566\tOK\n",
            "Uno R4 Board VM RX\tBTHLEENUM\\{6E400003-B5A3-F393-E0A9-E50E24DCCA9E}_DEV_112233445566\\8&ABC&0&BLUETOOTHDEVICE_112233445566\tOK\n",
            "ESP32 Board VM\tBTHENUM\\DEV_AABBCCDDEEFF\\7&DEF&0&BLUETOOTHDEVICE_AABBCCDDEEFF\tOK\n",
            "ESP32 Board VM Serial\tBTHENUM\\{00001101-0000-1000-8000-00805F9B34FB}_DEV_AABBCCDDEEFF&RFCOMMCHANNEL_3\\7&DEF&0&BLUETOOTHDEVICE_AABBCCDDEEFF\tOK\n",
            "Headphones\tBTHENUM\\DEV_001122334455\\9&GHI&0&BLUETOOTHDEVICE_001122334455\tError\n",
        );

        let devices = bluetooth_devices_from_windows_pnp_report(report);

        assert_eq!(devices.len(), 3);
        assert_eq!(devices[0].name.as_deref(), Some("Uno R4 Board VM"));
        assert_eq!(devices[0].address.as_deref(), Some("11:22:33:44:55:66"));
        assert!(devices[0].paired);
        assert_eq!(
            devices[0].service_uuids,
            vec![
                BOARD_VM_BLE_SERVICE_UUID.to_owned(),
                BOARD_VM_BLE_WRITE_CHARACTERISTIC_UUID.to_owned(),
                BOARD_VM_BLE_NOTIFY_CHARACTERISTIC_UUID.to_owned(),
            ]
        );
        assert_eq!(
            devices[0].characteristic_uuids,
            vec![
                BOARD_VM_BLE_WRITE_CHARACTERISTIC_UUID.to_owned(),
                BOARD_VM_BLE_NOTIFY_CHARACTERISTIC_UUID.to_owned(),
            ]
        );
        assert_eq!(devices[1].name.as_deref(), Some("ESP32 Board VM"));
        assert_eq!(devices[1].address.as_deref(), Some("AA:BB:CC:DD:EE:FF"));
        assert_eq!(devices[1].board_vm_rfcomm_channels, vec![3]);
        assert!(!devices[2].paired);

        let candidates = board_vm_endpoint_candidates(&devices);

        assert_eq!(candidates.len(), 2);
        assert!(candidates.iter().any(|candidate| {
            candidate.device == "11:22:33:44:55:66"
                && matches!(candidate.endpoint, BluetoothEndpoint::BleGatt(_))
                && !candidate.requires_pairing
        }));
        assert!(candidates.iter().any(|candidate| {
            candidate.device == "AA:BB:CC:DD:EE:FF"
                && matches!(candidate.endpoint, BluetoothEndpoint::Rfcomm(_))
        }));
    }

    #[test]
    fn ble_gatt_transport_exchanges_board_vm_wire_frames() {
        let endpoint = parse_ble_gatt_endpoint(&format!(
            "ble://esp32?service={SERVICE_UUID}&write={WRITE_UUID}&notify={NOTIFY_UUID}"
        ))
        .unwrap();
        let request = [0x01, 0x02, 0x03];
        let response = [0x04, 0x05];
        let mut response_wire = [0u8; 16];
        let response_wire_len = encode_wire_frame(&response, &mut response_wire).unwrap();
        let mut link = FakeBleGattLink::default();
        link.notifications.push_back(response_wire[..1].to_vec());
        let mut final_notification = response_wire[1..response_wire_len].to_vec();
        final_notification.push(0xFF);
        link.notifications.push_back(final_notification);

        let mut transport = BoardBleGattTransport::<_, 32>::new(endpoint, link);
        let mut raw_out = [0u8; 16];
        let response_len = transport
            .exchange_raw_frame_checked(&request, &mut raw_out)
            .unwrap();
        let link = transport.into_inner();
        let (written_uuid, written_wire) = link.writes.first().unwrap();
        let mut decoded_request = [0u8; 16];
        let request_len = decode_wire_frame(written_wire, &mut decoded_request).unwrap();

        assert_eq!(response_len, response.len());
        assert_eq!(&raw_out[..response_len], response);
        assert_eq!(written_uuid, WRITE_UUID);
        assert_eq!(&decoded_request[..request_len], request);
    }

    #[test]
    fn ble_gatt_transport_preserves_extra_notification_bytes() {
        let endpoint = parse_ble_gatt_endpoint(&format!(
            "ble://esp32?service={SERVICE_UUID}&write={WRITE_UUID}&notify={NOTIFY_UUID}"
        ))
        .unwrap();
        let first_response = [0x31, 0x32];
        let second_response = [0x33, 0x34, 0x35];
        let mut first_wire = [0u8; 16];
        let first_wire_len = encode_wire_frame(&first_response, &mut first_wire).unwrap();
        let mut second_wire = [0u8; 16];
        let second_wire_len = encode_wire_frame(&second_response, &mut second_wire).unwrap();
        let split = second_wire_len - 1;
        let mut notification = first_wire[..first_wire_len].to_vec();
        notification.extend_from_slice(&second_wire[..split]);
        let mut link = FakeBleGattLink::default();
        link.notifications.push_back(notification);
        link.notifications
            .push_back(second_wire[split..second_wire_len].to_vec());
        let mut transport = BoardBleGattTransport::<_, 32>::new(endpoint, link);
        let mut raw_out = [0u8; 16];

        let first_len = transport.receive_raw_frame(&mut raw_out).unwrap();
        assert_eq!(&raw_out[..first_len], first_response);
        let second_len = transport.receive_raw_frame(&mut raw_out).unwrap();
        assert_eq!(&raw_out[..second_len], second_response);
        assert!(transport.link().notifications.is_empty());
    }

    #[test]
    fn ble_gatt_transport_rejects_empty_notifications() {
        let endpoint = parse_ble_gatt_endpoint(&format!(
            "ble://esp32?service={SERVICE_UUID}&write={WRITE_UUID}&notify={NOTIFY_UUID}"
        ))
        .unwrap();
        let mut link = FakeBleGattLink::default();
        link.notifications.push_back(Vec::new());
        let mut transport = BoardBleGattTransport::<_, 16>::new(endpoint, link);
        let mut raw_out = [0u8; 16];

        assert_eq!(
            transport.receive_raw_frame(&mut raw_out),
            Err(BluetoothTransportError::Link)
        );
    }

    #[test]
    fn ble_gatt_transport_rejects_unterminated_wire_frames() {
        let endpoint = parse_ble_gatt_endpoint(&format!(
            "ble://esp32?service={SERVICE_UUID}&write={WRITE_UUID}&notify={NOTIFY_UUID}"
        ))
        .unwrap();
        let mut link = FakeBleGattLink::default();
        link.notifications.push_back(vec![0x11, 0x22, 0x33, 0x44]);
        let mut transport = BoardBleGattTransport::<_, 4>::new(endpoint, link);
        let mut raw_out = [0u8; 16];

        assert_eq!(
            transport.receive_raw_frame(&mut raw_out),
            Err(BluetoothTransportError::FrameTooLarge)
        );
    }

    #[test]
    fn ble_gatt_transport_rejects_unterminated_pending_bytes() {
        let endpoint = parse_ble_gatt_endpoint(&format!(
            "ble://esp32?service={SERVICE_UUID}&write={WRITE_UUID}&notify={NOTIFY_UUID}"
        ))
        .unwrap();
        let mut link = FakeBleGattLink::default();
        link.notifications.push_back(vec![0, 0x31, 0x32, 0x33]);
        link.notifications.push_back(vec![0x34]);
        let mut transport = BoardBleGattTransport::<_, 4>::new(endpoint, link);
        let mut raw_out = [0u8; 16];

        assert_eq!(transport.receive_raw_frame(&mut raw_out), Ok(0));
        assert_eq!(
            transport.receive_raw_frame(&mut raw_out),
            Err(BluetoothTransportError::FrameTooLarge)
        );
    }

    #[test]
    fn ble_gatt_raw_transport_maps_frame_overflow_to_response_too_large() {
        let endpoint = parse_ble_gatt_endpoint(&format!(
            "ble://esp32?service={SERVICE_UUID}&write={WRITE_UUID}&notify={NOTIFY_UUID}"
        ))
        .unwrap();
        let mut link = FakeBleGattLink::default();
        link.notifications.push_back(vec![0x11, 0x22, 0x33, 0x44]);
        let mut transport = BoardBleGattTransport::<_, 4>::new(endpoint, link);
        let mut raw_out = [0u8; 16];

        assert_eq!(
            RawFrameTransport::exchange_raw_frame(&mut transport, &[0xAA], &mut raw_out),
            Err(TransportError::ResponseTooLarge)
        );
    }

    #[test]
    fn ble_gatt_raw_transport_maps_oversized_requests_to_response_too_large() {
        let endpoint = parse_ble_gatt_endpoint(&format!(
            "ble://esp32?service={SERVICE_UUID}&write={WRITE_UUID}&notify={NOTIFY_UUID}"
        ))
        .unwrap();
        let link = FakeBleGattLink::default();
        let mut transport = BoardBleGattTransport::<_, 4>::new(endpoint, link);
        let mut raw_out = [0u8; 16];

        assert_eq!(
            RawFrameTransport::exchange_raw_frame(&mut transport, &[0xAA; 8], &mut raw_out),
            Err(TransportError::ResponseTooLarge)
        );
        assert!(transport.link().writes.is_empty());
    }

    #[test]
    fn ble_gatt_raw_transport_maps_write_failures_to_io() {
        let endpoint = parse_ble_gatt_endpoint(&format!(
            "ble://esp32?service={SERVICE_UUID}&write={WRITE_UUID}&notify={NOTIFY_UUID}"
        ))
        .unwrap();
        let link = FakeBleGattLink {
            fail_writes: true,
            ..FakeBleGattLink::default()
        };
        let mut transport = BoardBleGattTransport::<_, 32>::new(endpoint, link);
        let mut raw_out = [0u8; 16];

        assert_eq!(
            RawFrameTransport::exchange_raw_frame(&mut transport, &[0xAA], &mut raw_out),
            Err(TransportError::Io)
        );
    }

    #[test]
    fn ble_gatt_raw_transport_maps_notification_failures_to_io() {
        let endpoint = parse_ble_gatt_endpoint(&format!(
            "ble://esp32?service={SERVICE_UUID}&write={WRITE_UUID}&notify={NOTIFY_UUID}"
        ))
        .unwrap();
        let link = FakeBleGattLink {
            fail_reads: true,
            ..FakeBleGattLink::default()
        };
        let mut transport = BoardBleGattTransport::<_, 32>::new(endpoint, link);
        let mut raw_out = [0u8; 16];

        assert_eq!(
            RawFrameTransport::exchange_raw_frame(&mut transport, &[0xAA], &mut raw_out),
            Err(TransportError::Io)
        );
    }

    #[test]
    fn rfcomm_transport_exchanges_board_vm_wire_frames() {
        let endpoint = parse_rfcomm_endpoint("btspp://ESP32-BoardVM:3").unwrap();
        let request = [0x10, 0x11];
        let response = [0x12, 0x13, 0x14];
        let mut response_wire = [0u8; 16];
        let response_wire_len = encode_wire_frame(&response, &mut response_wire).unwrap();
        let stream = FakeRfcommStream::new(response_wire[..response_wire_len].to_vec());
        let mut transport = BoardRfcommTransport::<_, 32>::from_stream(endpoint, stream);
        let mut raw_out = [0u8; 16];

        let response_len = transport
            .exchange_raw_frame_checked(&request, &mut raw_out)
            .unwrap();
        let stream = transport.into_inner();
        let mut decoded_request = [0u8; 16];
        let request_len = decode_wire_frame(&stream.written, &mut decoded_request).unwrap();

        assert_eq!(response_len, response.len());
        assert_eq!(&raw_out[..response_len], response);
        assert_eq!(&decoded_request[..request_len], request);
    }

    #[test]
    fn rfcomm_raw_transport_maps_frame_overflow_to_response_too_large() {
        let endpoint = parse_rfcomm_endpoint("btspp://ESP32-BoardVM:3").unwrap();
        let stream = FakeRfcommStream::new(vec![0x11, 0x22, 0x33, 0x44]);
        let mut transport = BoardRfcommTransport::<_, 4>::from_stream(endpoint, stream);
        let mut raw_out = [0u8; 16];

        assert_eq!(
            RawFrameTransport::exchange_raw_frame(&mut transport, &[0xAA], &mut raw_out),
            Err(TransportError::ResponseTooLarge)
        );
    }

    #[test]
    fn rfcomm_raw_transport_maps_oversized_requests_to_response_too_large() {
        let endpoint = parse_rfcomm_endpoint("btspp://ESP32-BoardVM:3").unwrap();
        let stream = FakeRfcommStream::new(Vec::new());
        let mut transport = BoardRfcommTransport::<_, 4>::from_stream(endpoint, stream);
        let mut raw_out = [0u8; 16];

        assert_eq!(
            RawFrameTransport::exchange_raw_frame(&mut transport, &[0xAA; 8], &mut raw_out),
            Err(TransportError::ResponseTooLarge)
        );
        assert!(transport.into_inner().written.is_empty());
    }

    #[test]
    fn rfcomm_raw_transport_maps_stream_write_failures_to_io() {
        let endpoint = parse_rfcomm_endpoint("btspp://ESP32-BoardVM:3").unwrap();
        let stream = FakeRfcommStream::new(Vec::new()).with_write_failure();
        let mut transport = BoardRfcommTransport::<_, 32>::from_stream(endpoint, stream);
        let mut raw_out = [0u8; 16];

        assert_eq!(
            RawFrameTransport::exchange_raw_frame(&mut transport, &[0xAA], &mut raw_out),
            Err(TransportError::Io)
        );
    }

    #[test]
    fn rfcomm_raw_transport_maps_stream_read_failures_to_io() {
        let endpoint = parse_rfcomm_endpoint("btspp://ESP32-BoardVM:3").unwrap();
        let stream = FakeRfcommStream::new(Vec::new()).with_read_failure();
        let mut transport = BoardRfcommTransport::<_, 32>::from_stream(endpoint, stream);
        let mut raw_out = [0u8; 16];

        assert_eq!(
            RawFrameTransport::exchange_raw_frame(&mut transport, &[0xAA], &mut raw_out),
            Err(TransportError::Io)
        );
    }

    #[test]
    fn backend_opener_builds_ble_gatt_raw_frame_transport() {
        let endpoint = parse_bluetooth_endpoint(&format!(
            "ble://esp32?service={SERVICE_UUID}&write={WRITE_UUID}&notify={NOTIFY_UUID}"
        ))
        .unwrap();
        let response = [0x21, 0x22];
        let mut response_wire = [0u8; 16];
        let response_wire_len = encode_wire_frame(&response, &mut response_wire).unwrap();
        let mut backend = FakeBluetoothBackend::default();
        backend
            .ble_notifications
            .push_back(response_wire[..response_wire_len].to_vec());

        let mut transport = open_bluetooth_endpoint::<_, 32>(&mut backend, endpoint).unwrap();
        let mut raw_out = [0u8; 16];
        let response_len = transport.exchange_raw_frame(&[0x11], &mut raw_out).unwrap();

        assert_eq!(
            transport.endpoint_transport(),
            BluetoothEndpointTransport::BleGatt
        );
        assert_eq!(transport.device(), "esp32");
        assert_eq!(&raw_out[..response_len], response);
        assert_eq!(backend.ble_opened, vec!["esp32".to_owned()]);
    }

    #[test]
    fn macos_backend_uses_core_bluetooth_ble_connector_for_gatt_open() {
        let endpoint = parse_ble_gatt_endpoint(&format!(
            "ble://esp32?service={SERVICE_UUID}&write={WRITE_UUID}&notify={NOTIFY_UUID}"
        ))
        .unwrap();
        let response = [0x41, 0x42];
        let mut response_wire = [0u8; 16];
        let response_wire_len = encode_wire_frame(&response, &mut response_wire).unwrap();
        let mut backend = MacosBluetoothBackend::with_resolver_and_ble_connector(
            FixedRfcommPaths::default(),
            FakeCoreBluetoothBleConnector {
                requests: Vec::new(),
                notifications: VecDeque::from([response_wire[..response_wire_len].to_vec()]),
            },
        );

        let mut transport = open_bluetooth_endpoint::<_, 32>(
            &mut backend,
            BluetoothEndpoint::BleGatt(endpoint.clone()),
        )
        .unwrap();
        let mut raw_out = [0u8; 16];
        let response_len = transport.exchange_raw_frame(&[0x40], &mut raw_out).unwrap();

        assert_eq!(
            backend.ble_connector().requests,
            vec![MacosCoreBluetoothBleOpenRequest::from_endpoint(&endpoint)]
        );
        assert_eq!(
            transport.endpoint_transport(),
            BluetoothEndpointTransport::BleGatt
        );
        assert_eq!(transport.device(), "esp32");
        assert_eq!(&raw_out[..response_len], response);
        match transport {
            BoardBluetoothTransport::BleGatt(transport) => {
                assert_eq!(transport.link().writes[0].0, WRITE_UUID);
            }
            BoardBluetoothTransport::Rfcomm(_) => panic!("expected BLE GATT transport"),
        }
    }

    #[test]
    fn backend_opener_builds_rfcomm_raw_frame_transport() {
        let endpoint = parse_bluetooth_endpoint("btspp://ESP32-BoardVM:3").unwrap();
        let response = [0x31, 0x32, 0x33];
        let mut response_wire = [0u8; 16];
        let response_wire_len = encode_wire_frame(&response, &mut response_wire).unwrap();
        let mut backend = FakeBluetoothBackend {
            rfcomm_read: response_wire[..response_wire_len].to_vec(),
            ..FakeBluetoothBackend::default()
        };

        let mut transport = open_bluetooth_endpoint::<_, 32>(&mut backend, endpoint).unwrap();
        let mut raw_out = [0u8; 16];
        let response_len = transport.exchange_raw_frame(&[0x30], &mut raw_out).unwrap();

        assert_eq!(
            transport.endpoint_transport(),
            BluetoothEndpointTransport::Rfcomm
        );
        assert_eq!(transport.device(), "ESP32-BoardVM");
        assert_eq!(&raw_out[..response_len], response);
        assert_eq!(backend.rfcomm_opened, vec![("ESP32-BoardVM".to_owned(), 3)]);
    }

    #[test]
    fn backend_opener_propagates_ble_gatt_open_errors() {
        let endpoint = parse_bluetooth_endpoint(&format!(
            "ble://esp32?service={SERVICE_UUID}&write={WRITE_UUID}&notify={NOTIFY_UUID}"
        ))
        .unwrap();
        let mut backend = FakeBluetoothBackend {
            ble_open_error: Some(BluetoothOpenError::Backend {
                message: "BLE adapter unavailable".to_owned(),
            }),
            ..FakeBluetoothBackend::default()
        };

        let result = open_bluetooth_endpoint::<_, 32>(&mut backend, endpoint);

        assert_eq!(
            result.err(),
            Some(BluetoothOpenError::Backend {
                message: "BLE adapter unavailable".to_owned(),
            })
        );
        assert_eq!(backend.ble_opened, vec!["esp32".to_owned()]);
        assert!(backend.rfcomm_opened.is_empty());
    }

    #[test]
    fn backend_opener_propagates_rfcomm_open_errors() {
        let endpoint = parse_bluetooth_endpoint("btspp://ESP32-BoardVM:3").unwrap();
        let mut backend = FakeBluetoothBackend {
            rfcomm_open_error: Some(BluetoothOpenError::Backend {
                message: "RFCOMM device missing".to_owned(),
            }),
            ..FakeBluetoothBackend::default()
        };

        let result = open_bluetooth_endpoint::<_, 32>(&mut backend, endpoint);

        assert_eq!(
            result.err(),
            Some(BluetoothOpenError::Backend {
                message: "RFCOMM device missing".to_owned(),
            })
        );
        assert!(backend.ble_opened.is_empty());
        assert_eq!(backend.rfcomm_opened, vec![("ESP32-BoardVM".to_owned(), 3)]);
    }

    #[test]
    fn unsupported_backend_reports_platform_before_transport_use() {
        let endpoint = parse_bluetooth_endpoint("btspp://ESP32-BoardVM:3").unwrap();
        let mut backend = UnsupportedBluetoothBackend;
        let result = open_bluetooth_endpoint::<_, 32>(&mut backend, endpoint);

        assert!(matches!(
            result.err(),
            Some(BluetoothOpenError::UnsupportedPlatform { platform })
                if platform == std::env::consts::OS
        ));
    }

    #[test]
    fn resolves_macos_rfcomm_device_paths_for_names_and_addresses() {
        let named = parse_rfcomm_endpoint("btspp://ESP32-BoardVM:3").unwrap();
        let addressed = parse_rfcomm_endpoint("btspp://AA:BB:CC:DD:EE:FF:3").unwrap();

        assert_eq!(
            macos_rfcomm_device_path(
                &named,
                [
                    "/dev/tty.ESP32-BoardVM-SPPDev",
                    "/dev/cu.ESP32-BoardVM-SPPDev",
                    "/dev/cu.Unrelated",
                ],
            )
            .as_deref(),
            Some("/dev/cu.ESP32-BoardVM-SPPDev")
        );
        assert_eq!(
            macos_rfcomm_device_path(
                &addressed,
                ["/dev/cu.AA-BB-CC-DD-EE-FF-SPPDev", "/dev/cu.Unrelated"],
            )
            .as_deref(),
            Some("/dev/cu.AA-BB-CC-DD-EE-FF-SPPDev")
        );
    }

    #[test]
    fn macos_backend_opens_resolved_rfcomm_serial_device() {
        let endpoint = parse_rfcomm_endpoint("btspp://ESP32-BoardVM:3").unwrap();
        let path =
            std::env::temp_dir().join(format!("cu.ESP32-BoardVM-SPPDev-{}", std::process::id()));
        std::fs::File::create(&path).unwrap();

        let mut backend = MacosBluetoothBackend::with_resolver(FixedRfcommPaths {
            paths: vec![path.to_string_lossy().into_owned()],
        });
        let opened = backend.open_rfcomm(&endpoint).unwrap();

        drop(opened);
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn macos_backend_keeps_ble_gatt_explicitly_unwired() {
        let endpoint = parse_ble_gatt_endpoint(&format!(
            "ble://esp32?service={SERVICE_UUID}&write={WRITE_UUID}&notify={NOTIFY_UUID}"
        ))
        .unwrap();
        let mut backend = MacosBluetoothBackend::with_resolver(FixedRfcommPaths::default());

        assert!(matches!(
            backend.open_ble_gatt(&endpoint),
            Err(BluetoothOpenError::Backend { message })
                if message.contains("BLE GATT")
        ));
    }
}
