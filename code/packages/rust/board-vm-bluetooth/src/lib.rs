use std::io::{Read, Write};

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

pub struct BoardBleGattTransport<L, const WIRE_BYTES: usize = 1024> {
    endpoint: BleGattEndpoint,
    link: L,
    wire: [u8; WIRE_BYTES],
}

impl<L, const WIRE_BYTES: usize> BoardBleGattTransport<L, WIRE_BYTES> {
    pub fn new(endpoint: BleGattEndpoint, link: L) -> Self {
        Self {
            endpoint,
            link,
            wire: [0; WIRE_BYTES],
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
                len = chunk_start + terminator_offset + 1;
                return Ok(len);
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
    }

    impl BleGattIo for FakeBleGattLink {
        fn write_characteristic(
            &mut self,
            characteristic_uuid: &str,
            bytes: &[u8],
        ) -> Result<(), BluetoothTransportError> {
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
    }

    impl FakeRfcommStream {
        fn new(read: Vec<u8>) -> Self {
            Self {
                read: Cursor::new(read),
                written: Vec::new(),
            }
        }
    }

    impl Read for FakeRfcommStream {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            self.read.read(buf)
        }
    }

    impl Write for FakeRfcommStream {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.written.extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
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
}
