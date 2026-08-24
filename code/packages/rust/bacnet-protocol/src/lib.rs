//! Bounded BACnet/IP discovery and Device-object property framing.

#![forbid(unsafe_code)]

use std::fmt;
use std::net::{Ipv4Addr, SocketAddrV4};

pub const VERSION: &str = "0.1.0";
pub const BACNET_IP_DEFAULT_PORT: u16 = 0xbac0;
pub const BACNET_IP_TYPE: u8 = 0x81;
pub const MAX_BACNET_IP_DATAGRAM_BYTES: usize = 1497;
pub const MAX_DEVICE_INSTANCE: u32 = 0x3f_ffff;

const BVLC_HEADER_BYTES: usize = 4;
const BVLC_FORWARDED_ORIGIN_BYTES: usize = 6;
const NPDU_VERSION: u8 = 1;
const APDU_CONFIRMED_REQUEST: u8 = 0x00;
const APDU_COMPLEX_ACK: u8 = 0x30;
const APDU_UNCONFIRMED_REQUEST: u8 = 0x10;
const SERVICE_I_AM: u8 = 0;
const SERVICE_READ_PROPERTY: u8 = 12;
const SERVICE_WHO_IS: u8 = 8;
const OBJECT_TYPE_DEVICE: u16 = 8;
const MAX_CHARACTER_STRING_BYTES: usize = 128;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DeviceProperty {
    ApplicationSoftwareVersion,
    FirmwareRevision,
    ModelName,
    ObjectName,
    ProtocolVersion,
    SystemStatus,
    VendorIdentifier,
    VendorName,
}

impl DeviceProperty {
    pub const ALL: [Self; 8] = [
        Self::ObjectName,
        Self::SystemStatus,
        Self::VendorName,
        Self::VendorIdentifier,
        Self::ModelName,
        Self::FirmwareRevision,
        Self::ApplicationSoftwareVersion,
        Self::ProtocolVersion,
    ];

    pub const fn id(self) -> u32 {
        match self {
            Self::ApplicationSoftwareVersion => 12,
            Self::FirmwareRevision => 44,
            Self::ModelName => 70,
            Self::ObjectName => 77,
            Self::ProtocolVersion => 98,
            Self::SystemStatus => 112,
            Self::VendorIdentifier => 120,
            Self::VendorName => 121,
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ApplicationSoftwareVersion => "application_software_version",
            Self::FirmwareRevision => "firmware_revision",
            Self::ModelName => "model_name",
            Self::ObjectName => "object_name",
            Self::ProtocolVersion => "protocol_version",
            Self::SystemStatus => "system_status",
            Self::VendorIdentifier => "vendor_identifier",
            Self::VendorName => "vendor_name",
        }
    }

    const fn value_tag(self) -> u8 {
        match self {
            Self::ApplicationSoftwareVersion
            | Self::FirmwareRevision
            | Self::ModelName
            | Self::ObjectName
            | Self::VendorName => 7,
            Self::ProtocolVersion | Self::VendorIdentifier => 2,
            Self::SystemStatus => 9,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReadPropertyRequest {
    pub invoke_id: u8,
    pub device_instance: u32,
    pub property: DeviceProperty,
}

impl ReadPropertyRequest {
    pub fn new(
        invoke_id: u8,
        device_instance: u32,
        property: DeviceProperty,
    ) -> Result<Self, BacnetError> {
        if device_instance > MAX_DEVICE_INSTANCE {
            return Err(BacnetError::InvalidDeviceInstance(device_instance));
        }
        Ok(Self {
            invoke_id,
            device_instance,
            property,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReadPropertyValue {
    CharacterString(String),
    Enumerated(u32),
    Unsigned(u32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WhoIsRequest {
    All,
    Range { low_limit: u32, high_limit: u32 },
}

impl WhoIsRequest {
    fn validate(self) -> Result<Self, BacnetError> {
        match self {
            Self::All => Ok(self),
            Self::Range {
                low_limit,
                high_limit,
            } if low_limit <= high_limit && high_limit <= MAX_DEVICE_INSTANCE => Ok(self),
            Self::Range {
                low_limit,
                high_limit,
            } => Err(BacnetError::InvalidWhoIsRange {
                low_limit,
                high_limit,
            }),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BvlcFunction {
    ForwardedNpdu,
    OriginalUnicastNpdu,
    OriginalBroadcastNpdu,
}

impl BvlcFunction {
    pub const fn code(self) -> u8 {
        match self {
            Self::ForwardedNpdu => 0x04,
            Self::OriginalUnicastNpdu => 0x0a,
            Self::OriginalBroadcastNpdu => 0x0b,
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ForwardedNpdu => "forwarded_npdu",
            Self::OriginalUnicastNpdu => "original_unicast_npdu",
            Self::OriginalBroadcastNpdu => "original_broadcast_npdu",
        }
    }

    fn from_code(code: u8) -> Result<Self, BacnetError> {
        match code {
            0x04 => Ok(Self::ForwardedNpdu),
            0x0a => Ok(Self::OriginalUnicastNpdu),
            0x0b => Ok(Self::OriginalBroadcastNpdu),
            _ => Err(BacnetError::UnsupportedBvlcFunction(code)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SegmentationSupport {
    SegmentedBoth,
    SegmentedTransmit,
    SegmentedReceive,
    NoSegmentation,
}

impl SegmentationSupport {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::SegmentedBoth => "segmented_both",
            Self::SegmentedTransmit => "segmented_transmit",
            Self::SegmentedReceive => "segmented_receive",
            Self::NoSegmentation => "no_segmentation",
        }
    }

    fn from_value(value: u32) -> Result<Self, BacnetError> {
        match value {
            0 => Ok(Self::SegmentedBoth),
            1 => Ok(Self::SegmentedTransmit),
            2 => Ok(Self::SegmentedReceive),
            3 => Ok(Self::NoSegmentation),
            _ => Err(BacnetError::InvalidSegmentation(value)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IAmResponse {
    pub device_instance: u32,
    pub max_apdu_length_accepted: u16,
    pub segmentation_supported: SegmentationSupport,
    pub vendor_id: u16,
    pub bvlc_function: BvlcFunction,
    pub forwarded_from: Option<SocketAddrV4>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BacnetError {
    InvalidWhoIsRange { low_limit: u32, high_limit: u32 },
    InvalidDeviceInstance(u32),
    DatagramTooShort { actual: usize },
    DatagramTooLarge { actual: usize, maximum: usize },
    BvlcType(u8),
    UnsupportedBvlcFunction(u8),
    BvlcLength { declared: usize, actual: usize },
    NpduVersion(u8),
    ReservedNpduControl(u8),
    NetworkLayerMessage,
    TruncatedNpduAddress,
    ApduType(u8),
    ServiceChoice(u8),
    ApplicationTag { expected: u8, actual: u8 },
    InvalidApplicationLength { tag: u8, length: usize },
    TruncatedApplicationValue { tag: u8 },
    ObjectType(u16),
    UnsignedOverflow { field: &'static str, value: u32 },
    InvalidSegmentation(u32),
    InvokeId { expected: u8, actual: u8 },
    ObjectInstance { expected: u32, actual: u32 },
    PropertyIdentifier { expected: u32, actual: u32 },
    ContextTag { expected: u8, actual: u8 },
    OpeningTag { expected: u8, actual: u8 },
    ClosingTag { expected: u8, actual: u8 },
    CharacterSet(u8),
    CharacterStringLength { length: usize, maximum: usize },
    NonPrintableCharacter(u8),
    TrailingBytes(usize),
}

impl fmt::Display for BacnetError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidWhoIsRange {
                low_limit,
                high_limit,
            } => write!(
                formatter,
                "BACnet Who-Is range {low_limit}..={high_limit} is invalid"
            ),
            Self::InvalidDeviceInstance(actual) => write!(
                formatter,
                "BACnet Device instance {actual} exceeds {MAX_DEVICE_INSTANCE}"
            ),
            Self::DatagramTooShort { actual } => {
                write!(formatter, "BACnet/IP datagram is too short: {actual} bytes")
            }
            Self::DatagramTooLarge { actual, maximum } => write!(
                formatter,
                "BACnet/IP datagram is {actual} bytes, exceeding the {maximum}-byte limit"
            ),
            Self::BvlcType(actual) => {
                write!(
                    formatter,
                    "BACnet/IP BVLC type must be 0x81, got 0x{actual:02x}"
                )
            }
            Self::UnsupportedBvlcFunction(actual) => write!(
                formatter,
                "unsupported BACnet/IP BVLC function 0x{actual:02x}"
            ),
            Self::BvlcLength { declared, actual } => write!(
                formatter,
                "BACnet/IP BVLC length declares {declared} bytes but datagram contains {actual}"
            ),
            Self::NpduVersion(actual) => {
                write!(formatter, "BACnet NPDU version must be 1, got {actual}")
            }
            Self::ReservedNpduControl(actual) => {
                write!(
                    formatter,
                    "BACnet NPDU control has reserved bits set: 0x{actual:02x}"
                )
            }
            Self::NetworkLayerMessage => {
                write!(formatter, "BACnet I-Am must be an application message")
            }
            Self::TruncatedNpduAddress => {
                write!(formatter, "BACnet NPDU address fields are truncated")
            }
            Self::ApduType(actual) => {
                write!(formatter, "unexpected BACnet APDU type 0x{actual:02x}")
            }
            Self::ServiceChoice(actual) => {
                write!(formatter, "unexpected BACnet APDU service choice {actual}")
            }
            Self::ApplicationTag { expected, actual } => write!(
                formatter,
                "BACnet application tag must be {expected}, got {actual}"
            ),
            Self::InvalidApplicationLength { tag, length } => write!(
                formatter,
                "BACnet application tag {tag} has unsupported length {length}"
            ),
            Self::TruncatedApplicationValue { tag } => {
                write!(formatter, "BACnet application tag {tag} is truncated")
            }
            Self::ObjectType(actual) => {
                write!(
                    formatter,
                    "BACnet I-Am object type must be Device (8), got {actual}"
                )
            }
            Self::UnsignedOverflow { field, value } => {
                write!(formatter, "BACnet {field} value {value} exceeds u16")
            }
            Self::InvalidSegmentation(actual) => {
                write!(formatter, "BACnet segmentation value {actual} is invalid")
            }
            Self::InvokeId { expected, actual } => write!(
                formatter,
                "BACnet invoke id mismatch: expected {expected}, got {actual}"
            ),
            Self::ObjectInstance { expected, actual } => write!(
                formatter,
                "BACnet Device instance mismatch: expected {expected}, got {actual}"
            ),
            Self::PropertyIdentifier { expected, actual } => write!(
                formatter,
                "BACnet property identifier mismatch: expected {expected}, got {actual}"
            ),
            Self::ContextTag { expected, actual } => write!(
                formatter,
                "BACnet context tag mismatch: expected {expected}, got {actual}"
            ),
            Self::OpeningTag { expected, actual } => write!(
                formatter,
                "BACnet opening tag mismatch: expected {expected}, got {actual}"
            ),
            Self::ClosingTag { expected, actual } => write!(
                formatter,
                "BACnet closing tag mismatch: expected {expected}, got {actual}"
            ),
            Self::CharacterSet(actual) => write!(
                formatter,
                "BACnet character string must use ANSI X3.4 encoding 0, got {actual}"
            ),
            Self::CharacterStringLength { length, maximum } => write!(
                formatter,
                "BACnet character string length {length} is outside 1..={maximum} bytes"
            ),
            Self::NonPrintableCharacter(actual) => write!(
                formatter,
                "BACnet character string contains non-printable byte 0x{actual:02x}"
            ),
            Self::TrailingBytes(actual) => {
                write!(formatter, "BACnet I-Am contains {actual} trailing bytes")
            }
        }
    }
}

impl std::error::Error for BacnetError {}

pub fn encode_who_is(request: WhoIsRequest) -> Result<Vec<u8>, BacnetError> {
    let request = request.validate()?;
    let mut apdu = vec![APDU_UNCONFIRMED_REQUEST, SERVICE_WHO_IS];
    if let WhoIsRequest::Range {
        low_limit,
        high_limit,
    } = request
    {
        encode_context_unsigned(0, low_limit, &mut apdu);
        encode_context_unsigned(1, high_limit, &mut apdu);
    }

    let length = BVLC_HEADER_BYTES + 2 + apdu.len();
    let mut bytes = Vec::with_capacity(length);
    bytes.push(BACNET_IP_TYPE);
    bytes.push(BvlcFunction::OriginalBroadcastNpdu.code());
    bytes.extend_from_slice(&(length as u16).to_be_bytes());
    bytes.extend_from_slice(&[NPDU_VERSION, 0]);
    bytes.extend_from_slice(&apdu);
    Ok(bytes)
}

pub fn encode_read_property(request: ReadPropertyRequest) -> Result<Vec<u8>, BacnetError> {
    let request =
        ReadPropertyRequest::new(request.invoke_id, request.device_instance, request.property)?;
    let object_id = (u32::from(OBJECT_TYPE_DEVICE) << 22) | request.device_instance;
    let mut apdu = vec![
        APDU_CONFIRMED_REQUEST,
        0x05,
        request.invoke_id,
        SERVICE_READ_PROPERTY,
        0x0c,
    ];
    apdu.extend_from_slice(&object_id.to_be_bytes());
    encode_context_unsigned(1, request.property.id(), &mut apdu);

    let length = BVLC_HEADER_BYTES + 2 + apdu.len();
    let mut bytes = Vec::with_capacity(length);
    bytes.push(BACNET_IP_TYPE);
    bytes.push(BvlcFunction::OriginalUnicastNpdu.code());
    bytes.extend_from_slice(&(length as u16).to_be_bytes());
    bytes.extend_from_slice(&[NPDU_VERSION, 0x04]);
    bytes.extend_from_slice(&apdu);
    Ok(bytes)
}

pub fn decode_read_property_ack(
    bytes: &[u8],
    request: ReadPropertyRequest,
) -> Result<ReadPropertyValue, BacnetError> {
    let request =
        ReadPropertyRequest::new(request.invoke_id, request.device_instance, request.property)?;
    let (function, mut offset, control) = decode_bvlc_npdu(bytes)?;
    if function != BvlcFunction::OriginalUnicastNpdu {
        return Err(BacnetError::UnsupportedBvlcFunction(function.code()));
    }
    if control & 0x04 != 0 {
        return Err(BacnetError::ReservedNpduControl(control));
    }

    let apdu_type = *bytes.get(offset).ok_or(BacnetError::ApduType(0xff))?;
    if apdu_type != APDU_COMPLEX_ACK {
        return Err(BacnetError::ApduType(apdu_type));
    }
    offset += 1;
    let invoke_id = *bytes.get(offset).ok_or(BacnetError::InvokeId {
        expected: request.invoke_id,
        actual: 0,
    })?;
    if invoke_id != request.invoke_id {
        return Err(BacnetError::InvokeId {
            expected: request.invoke_id,
            actual: invoke_id,
        });
    }
    offset += 1;
    let service = *bytes.get(offset).ok_or(BacnetError::ServiceChoice(0xff))?;
    if service != SERVICE_READ_PROPERTY {
        return Err(BacnetError::ServiceChoice(service));
    }
    offset += 1;

    let object_id = read_context_object_identifier(bytes, &mut offset, 0)?;
    let object_type = (object_id >> 22) as u16;
    if object_type != OBJECT_TYPE_DEVICE {
        return Err(BacnetError::ObjectType(object_type));
    }
    let device_instance = object_id & MAX_DEVICE_INSTANCE;
    if device_instance != request.device_instance {
        return Err(BacnetError::ObjectInstance {
            expected: request.device_instance,
            actual: device_instance,
        });
    }
    let property_id = read_context_unsigned(bytes, &mut offset, 1)?;
    if property_id != request.property.id() {
        return Err(BacnetError::PropertyIdentifier {
            expected: request.property.id(),
            actual: property_id,
        });
    }
    read_opening_tag(bytes, &mut offset, 3)?;
    let value = match request.property.value_tag() {
        7 => ReadPropertyValue::CharacterString(read_character_string(bytes, &mut offset)?),
        9 => ReadPropertyValue::Enumerated(read_application_unsigned(bytes, &mut offset, 9)?),
        _ => ReadPropertyValue::Unsigned(read_application_unsigned(bytes, &mut offset, 2)?),
    };
    read_closing_tag(bytes, &mut offset, 3)?;
    if offset != bytes.len() {
        return Err(BacnetError::TrailingBytes(bytes.len() - offset));
    }
    Ok(value)
}

pub fn decode_i_am(bytes: &[u8]) -> Result<IAmResponse, BacnetError> {
    if bytes.len() < BVLC_HEADER_BYTES + 2 + 2 {
        return Err(BacnetError::DatagramTooShort {
            actual: bytes.len(),
        });
    }
    if bytes.len() > MAX_BACNET_IP_DATAGRAM_BYTES {
        return Err(BacnetError::DatagramTooLarge {
            actual: bytes.len(),
            maximum: MAX_BACNET_IP_DATAGRAM_BYTES,
        });
    }
    if bytes[0] != BACNET_IP_TYPE {
        return Err(BacnetError::BvlcType(bytes[0]));
    }
    let function = BvlcFunction::from_code(bytes[1])?;
    let declared = usize::from(u16::from_be_bytes([bytes[2], bytes[3]]));
    if declared != bytes.len() {
        return Err(BacnetError::BvlcLength {
            declared,
            actual: bytes.len(),
        });
    }

    let (mut offset, forwarded_from) = if function == BvlcFunction::ForwardedNpdu {
        if bytes.len() < BVLC_HEADER_BYTES + BVLC_FORWARDED_ORIGIN_BYTES + 4 {
            return Err(BacnetError::DatagramTooShort {
                actual: bytes.len(),
            });
        }
        let address = Ipv4Addr::new(bytes[4], bytes[5], bytes[6], bytes[7]);
        let port = u16::from_be_bytes([bytes[8], bytes[9]]);
        (
            BVLC_HEADER_BYTES + BVLC_FORWARDED_ORIGIN_BYTES,
            Some(SocketAddrV4::new(address, port)),
        )
    } else {
        (BVLC_HEADER_BYTES, None)
    };

    if bytes.get(offset).copied() != Some(NPDU_VERSION) {
        return Err(BacnetError::NpduVersion(
            bytes.get(offset).copied().unwrap_or(0),
        ));
    }
    offset += 1;
    let control = *bytes.get(offset).ok_or(BacnetError::TruncatedNpduAddress)?;
    offset += 1;
    if control & 0x50 != 0 {
        return Err(BacnetError::ReservedNpduControl(control));
    }
    if control & 0x80 != 0 {
        return Err(BacnetError::NetworkLayerMessage);
    }
    if control & 0x20 != 0 {
        offset = skip_npdu_address(bytes, offset, true)?;
    }
    if control & 0x08 != 0 {
        offset = skip_npdu_address(bytes, offset, false)?;
    }

    let apdu_type = *bytes.get(offset).ok_or(BacnetError::ApduType(0))?;
    if apdu_type != APDU_UNCONFIRMED_REQUEST {
        return Err(BacnetError::ApduType(apdu_type));
    }
    offset += 1;
    let service = *bytes.get(offset).ok_or(BacnetError::ServiceChoice(0xff))?;
    if service != SERVICE_I_AM {
        return Err(BacnetError::ServiceChoice(service));
    }
    offset += 1;

    let object_id = read_object_identifier(bytes, &mut offset)?;
    let object_type = (object_id >> 22) as u16;
    if object_type != OBJECT_TYPE_DEVICE {
        return Err(BacnetError::ObjectType(object_type));
    }
    let device_instance = object_id & MAX_DEVICE_INSTANCE;
    let max_apdu = read_application_unsigned(bytes, &mut offset, 2)?;
    let max_apdu_length_accepted =
        u16::try_from(max_apdu).map_err(|_| BacnetError::UnsignedOverflow {
            field: "max APDU length",
            value: max_apdu,
        })?;
    let segmentation = read_application_unsigned(bytes, &mut offset, 9)?;
    let vendor = read_application_unsigned(bytes, &mut offset, 2)?;
    let vendor_id = u16::try_from(vendor).map_err(|_| BacnetError::UnsignedOverflow {
        field: "vendor id",
        value: vendor,
    })?;
    if offset != bytes.len() {
        return Err(BacnetError::TrailingBytes(bytes.len() - offset));
    }

    Ok(IAmResponse {
        device_instance,
        max_apdu_length_accepted,
        segmentation_supported: SegmentationSupport::from_value(segmentation)?,
        vendor_id,
        bvlc_function: function,
        forwarded_from,
    })
}

fn encode_context_unsigned(tag: u8, value: u32, output: &mut Vec<u8>) {
    let bytes = value.to_be_bytes();
    let first = bytes.iter().position(|byte| *byte != 0).unwrap_or(3);
    let value_bytes = &bytes[first..];
    output.push((tag << 4) | 0x08 | value_bytes.len() as u8);
    output.extend_from_slice(value_bytes);
}

fn decode_bvlc_npdu(bytes: &[u8]) -> Result<(BvlcFunction, usize, u8), BacnetError> {
    if bytes.len() < BVLC_HEADER_BYTES + 3 {
        return Err(BacnetError::DatagramTooShort {
            actual: bytes.len(),
        });
    }
    if bytes.len() > MAX_BACNET_IP_DATAGRAM_BYTES {
        return Err(BacnetError::DatagramTooLarge {
            actual: bytes.len(),
            maximum: MAX_BACNET_IP_DATAGRAM_BYTES,
        });
    }
    if bytes[0] != BACNET_IP_TYPE {
        return Err(BacnetError::BvlcType(bytes[0]));
    }
    let function = BvlcFunction::from_code(bytes[1])?;
    let declared = usize::from(u16::from_be_bytes([bytes[2], bytes[3]]));
    if declared != bytes.len() {
        return Err(BacnetError::BvlcLength {
            declared,
            actual: bytes.len(),
        });
    }
    let mut offset = if function == BvlcFunction::ForwardedNpdu {
        BVLC_HEADER_BYTES + BVLC_FORWARDED_ORIGIN_BYTES
    } else {
        BVLC_HEADER_BYTES
    };
    if bytes.get(offset).copied() != Some(NPDU_VERSION) {
        return Err(BacnetError::NpduVersion(
            bytes.get(offset).copied().unwrap_or(0),
        ));
    }
    offset += 1;
    let control = *bytes.get(offset).ok_or(BacnetError::TruncatedNpduAddress)?;
    offset += 1;
    if control & 0x50 != 0 {
        return Err(BacnetError::ReservedNpduControl(control));
    }
    if control & 0x80 != 0 {
        return Err(BacnetError::NetworkLayerMessage);
    }
    if control & 0x20 != 0 {
        offset = skip_npdu_address(bytes, offset, true)?;
    }
    if control & 0x08 != 0 {
        offset = skip_npdu_address(bytes, offset, false)?;
    }
    Ok((function, offset, control))
}

fn read_context_object_identifier(
    bytes: &[u8],
    offset: &mut usize,
    expected_tag: u8,
) -> Result<u32, BacnetError> {
    let header = *bytes
        .get(*offset)
        .ok_or(BacnetError::TruncatedApplicationValue { tag: expected_tag })?;
    let actual_tag = header >> 4;
    if actual_tag != expected_tag || header & 0x08 == 0 || header & 0x07 != 4 {
        return Err(BacnetError::ContextTag {
            expected: expected_tag,
            actual: actual_tag,
        });
    }
    *offset += 1;
    let value = bytes
        .get(*offset..*offset + 4)
        .ok_or(BacnetError::TruncatedApplicationValue { tag: expected_tag })?;
    *offset += 4;
    Ok(u32::from_be_bytes([value[0], value[1], value[2], value[3]]))
}

fn read_context_unsigned(
    bytes: &[u8],
    offset: &mut usize,
    expected_tag: u8,
) -> Result<u32, BacnetError> {
    let header = *bytes
        .get(*offset)
        .ok_or(BacnetError::TruncatedApplicationValue { tag: expected_tag })?;
    let actual_tag = header >> 4;
    if actual_tag != expected_tag || header & 0x08 == 0 {
        return Err(BacnetError::ContextTag {
            expected: expected_tag,
            actual: actual_tag,
        });
    }
    let length = usize::from(header & 0x07);
    if !(1..=4).contains(&length) {
        return Err(BacnetError::InvalidApplicationLength {
            tag: expected_tag,
            length,
        });
    }
    *offset += 1;
    let value = bytes
        .get(*offset..*offset + length)
        .ok_or(BacnetError::TruncatedApplicationValue { tag: expected_tag })?;
    *offset += length;
    Ok(value
        .iter()
        .fold(0u32, |total, byte| (total << 8) | u32::from(*byte)))
}

fn read_opening_tag(bytes: &[u8], offset: &mut usize, expected_tag: u8) -> Result<(), BacnetError> {
    let header = *bytes.get(*offset).ok_or(BacnetError::OpeningTag {
        expected: expected_tag,
        actual: 0xff,
    })?;
    if header != (expected_tag << 4) | 0x0e {
        return Err(BacnetError::OpeningTag {
            expected: expected_tag,
            actual: header >> 4,
        });
    }
    *offset += 1;
    Ok(())
}

fn read_closing_tag(bytes: &[u8], offset: &mut usize, expected_tag: u8) -> Result<(), BacnetError> {
    let header = *bytes.get(*offset).ok_or(BacnetError::ClosingTag {
        expected: expected_tag,
        actual: 0xff,
    })?;
    if header != (expected_tag << 4) | 0x0f {
        return Err(BacnetError::ClosingTag {
            expected: expected_tag,
            actual: header >> 4,
        });
    }
    *offset += 1;
    Ok(())
}

fn read_character_string(bytes: &[u8], offset: &mut usize) -> Result<String, BacnetError> {
    let header = *bytes
        .get(*offset)
        .ok_or(BacnetError::TruncatedApplicationValue { tag: 7 })?;
    if header >> 4 != 7 || header & 0x08 != 0 {
        return Err(BacnetError::ApplicationTag {
            expected: 7,
            actual: header >> 4,
        });
    }
    *offset += 1;
    let mut length = usize::from(header & 0x07);
    if length == 5 {
        length = usize::from(
            *bytes
                .get(*offset)
                .ok_or(BacnetError::TruncatedApplicationValue { tag: 7 })?,
        );
        *offset += 1;
    }
    if length < 2 || length - 1 > MAX_CHARACTER_STRING_BYTES {
        return Err(BacnetError::CharacterStringLength {
            length: length.saturating_sub(1),
            maximum: MAX_CHARACTER_STRING_BYTES,
        });
    }
    let value = bytes
        .get(*offset..*offset + length)
        .ok_or(BacnetError::TruncatedApplicationValue { tag: 7 })?;
    *offset += length;
    if value[0] != 0 {
        return Err(BacnetError::CharacterSet(value[0]));
    }
    if let Some(byte) = value[1..]
        .iter()
        .copied()
        .find(|byte| !(0x20..=0x7e).contains(byte))
    {
        return Err(BacnetError::NonPrintableCharacter(byte));
    }
    Ok(String::from_utf8(value[1..].to_vec()).expect("printable ASCII is valid UTF-8"))
}

fn skip_npdu_address(
    bytes: &[u8],
    mut offset: usize,
    destination: bool,
) -> Result<usize, BacnetError> {
    if bytes.len().saturating_sub(offset) < 3 {
        return Err(BacnetError::TruncatedNpduAddress);
    }
    offset += 2;
    let address_length = usize::from(bytes[offset]);
    offset += 1;
    offset = offset
        .checked_add(address_length)
        .filter(|end| *end <= bytes.len())
        .ok_or(BacnetError::TruncatedNpduAddress)?;
    if destination {
        offset = offset
            .checked_add(1)
            .filter(|end| *end <= bytes.len())
            .ok_or(BacnetError::TruncatedNpduAddress)?;
    }
    Ok(offset)
}

fn read_object_identifier(bytes: &[u8], offset: &mut usize) -> Result<u32, BacnetError> {
    let header = *bytes
        .get(*offset)
        .ok_or(BacnetError::TruncatedApplicationValue { tag: 12 })?;
    if header != 0xc4 {
        return Err(BacnetError::ApplicationTag {
            expected: 12,
            actual: header >> 4,
        });
    }
    *offset += 1;
    let value = bytes
        .get(*offset..*offset + 4)
        .ok_or(BacnetError::TruncatedApplicationValue { tag: 12 })?;
    *offset += 4;
    Ok(u32::from_be_bytes([value[0], value[1], value[2], value[3]]))
}

fn read_application_unsigned(
    bytes: &[u8],
    offset: &mut usize,
    expected_tag: u8,
) -> Result<u32, BacnetError> {
    let header = *bytes
        .get(*offset)
        .ok_or(BacnetError::TruncatedApplicationValue { tag: expected_tag })?;
    let actual_tag = header >> 4;
    if actual_tag != expected_tag || header & 0x08 != 0 {
        return Err(BacnetError::ApplicationTag {
            expected: expected_tag,
            actual: actual_tag,
        });
    }
    let length = usize::from(header & 0x07);
    if !(1..=4).contains(&length) {
        return Err(BacnetError::InvalidApplicationLength {
            tag: expected_tag,
            length,
        });
    }
    *offset += 1;
    let value = bytes
        .get(*offset..*offset + length)
        .ok_or(BacnetError::TruncatedApplicationValue { tag: expected_tag })?;
    *offset += length;
    Ok(value
        .iter()
        .fold(0u32, |total, byte| (total << 8) | u32::from(*byte)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn i_am() -> Vec<u8> {
        vec![
            0x81, 0x0a, 0x00, 0x14, 0x01, 0x00, 0x10, 0x00, 0xc4, 0x02, 0x00, 0x00, 0x7b, 0x22,
            0x05, 0xc4, 0x91, 0x03, 0x21, 0x63,
        ]
    }

    fn read_property_ack(request: ReadPropertyRequest, value: &[u8]) -> Vec<u8> {
        let object_id = (u32::from(OBJECT_TYPE_DEVICE) << 22) | request.device_instance;
        let mut bytes = vec![
            0x81,
            0x0a,
            0,
            0,
            1,
            0,
            APDU_COMPLEX_ACK,
            request.invoke_id,
            SERVICE_READ_PROPERTY,
            0x0c,
        ];
        bytes.extend_from_slice(&object_id.to_be_bytes());
        encode_context_unsigned(1, request.property.id(), &mut bytes);
        bytes.push(0x3e);
        bytes.extend_from_slice(value);
        bytes.push(0x3f);
        let length = bytes.len() as u16;
        bytes[2..4].copy_from_slice(&length.to_be_bytes());
        bytes
    }

    #[test]
    fn encodes_local_broadcast_who_is() {
        assert_eq!(
            encode_who_is(WhoIsRequest::All).unwrap(),
            [0x81, 0x0b, 0, 8, 1, 0, 0x10, 0x08]
        );
    }

    #[test]
    fn encodes_bounded_who_is_range() {
        assert_eq!(
            encode_who_is(WhoIsRequest::Range {
                low_limit: 1,
                high_limit: 0x01_0203,
            })
            .unwrap(),
            [0x81, 0x0b, 0, 14, 1, 0, 0x10, 0x08, 0x09, 1, 0x1b, 1, 2, 3,]
        );
        assert!(matches!(
            encode_who_is(WhoIsRequest::Range {
                low_limit: 8,
                high_limit: 7,
            }),
            Err(BacnetError::InvalidWhoIsRange { .. })
        ));
    }

    #[test]
    fn encodes_correlated_device_read_property() {
        let request = ReadPropertyRequest::new(7, 123, DeviceProperty::ObjectName).unwrap();
        assert_eq!(
            encode_read_property(request).unwrap(),
            [0x81, 0x0a, 0, 17, 1, 0x04, 0, 0x05, 7, 12, 0x0c, 0x02, 0, 0, 123, 0x19, 77,]
        );
        assert_eq!(
            ReadPropertyRequest::new(1, MAX_DEVICE_INSTANCE + 1, DeviceProperty::ObjectName),
            Err(BacnetError::InvalidDeviceInstance(MAX_DEVICE_INSTANCE + 1))
        );
    }

    #[test]
    fn decodes_fixed_device_property_value_types() {
        let name = ReadPropertyRequest::new(7, 123, DeviceProperty::ObjectName).unwrap();
        assert_eq!(
            decode_read_property_ack(
                &read_property_ack(name, &[0x75, 6, 0, b'A', b'H', b'U', b'-', b'1']),
                name,
            )
            .unwrap(),
            ReadPropertyValue::CharacterString("AHU-1".to_string())
        );

        let status = ReadPropertyRequest::new(8, 123, DeviceProperty::SystemStatus).unwrap();
        assert_eq!(
            decode_read_property_ack(&read_property_ack(status, &[0x91, 2]), status).unwrap(),
            ReadPropertyValue::Enumerated(2)
        );

        let version = ReadPropertyRequest::new(9, 123, DeviceProperty::ProtocolVersion).unwrap();
        assert_eq!(
            decode_read_property_ack(&read_property_ack(version, &[0x21, 1]), version).unwrap(),
            ReadPropertyValue::Unsigned(1)
        );
    }

    #[test]
    fn rejects_uncorrelated_or_unsafe_property_values() {
        let request = ReadPropertyRequest::new(7, 123, DeviceProperty::ObjectName).unwrap();
        let mut wrong_invoke =
            read_property_ack(request, &[0x75, 6, 0, b'A', b'H', b'U', b'-', b'1']);
        wrong_invoke[7] = 8;
        assert_eq!(
            decode_read_property_ack(&wrong_invoke, request),
            Err(BacnetError::InvokeId {
                expected: 7,
                actual: 8,
            })
        );

        let mut reply_expects_reply =
            read_property_ack(request, &[0x75, 6, 0, b'A', b'H', b'U', b'-', b'1']);
        reply_expects_reply[5] = 0x04;
        assert_eq!(
            decode_read_property_ack(&reply_expects_reply, request),
            Err(BacnetError::ReservedNpduControl(0x04))
        );

        let wrong_property = ReadPropertyRequest::new(7, 123, DeviceProperty::ModelName).unwrap();
        assert_eq!(
            decode_read_property_ack(
                &read_property_ack(wrong_property, &[0x75, 6, 0, b'A', b'H', b'U', b'-', b'1']),
                request,
            ),
            Err(BacnetError::PropertyIdentifier {
                expected: DeviceProperty::ObjectName.id(),
                actual: DeviceProperty::ModelName.id(),
            })
        );

        assert_eq!(
            decode_read_property_ack(
                &read_property_ack(request, &[0x75, 6, 5, b'A', b'H', b'U', b'-', b'1']),
                request,
            ),
            Err(BacnetError::CharacterSet(5))
        );
    }

    #[test]
    fn decodes_direct_i_am() {
        assert_eq!(
            decode_i_am(&i_am()).unwrap(),
            IAmResponse {
                device_instance: 123,
                max_apdu_length_accepted: 1476,
                segmentation_supported: SegmentationSupport::NoSegmentation,
                vendor_id: 99,
                bvlc_function: BvlcFunction::OriginalUnicastNpdu,
                forwarded_from: None,
            }
        );
    }

    #[test]
    fn decodes_forwarded_i_am_origin() {
        let mut bytes = i_am();
        bytes[1] = BvlcFunction::ForwardedNpdu.code();
        bytes.splice(4..4, [192, 0, 2, 7, 0xba, 0xc0]);
        let length = bytes.len() as u16;
        bytes[2..4].copy_from_slice(&length.to_be_bytes());
        let decoded = decode_i_am(&bytes).unwrap();
        assert_eq!(
            decoded.forwarded_from,
            Some(SocketAddrV4::new(Ipv4Addr::new(192, 0, 2, 7), 0xbac0))
        );
        assert_eq!(decoded.bvlc_function, BvlcFunction::ForwardedNpdu);
    }

    #[test]
    fn rejects_wrong_bvlc_npdu_and_service_fields() {
        let mut bytes = i_am();
        bytes[0] = 0x82;
        assert_eq!(decode_i_am(&bytes), Err(BacnetError::BvlcType(0x82)));

        let mut bytes = i_am();
        bytes[3] -= 1;
        assert!(matches!(
            decode_i_am(&bytes),
            Err(BacnetError::BvlcLength { .. })
        ));

        let mut bytes = i_am();
        bytes[5] = 0x80;
        assert_eq!(decode_i_am(&bytes), Err(BacnetError::NetworkLayerMessage));

        let mut bytes = i_am();
        bytes[7] = 1;
        assert_eq!(decode_i_am(&bytes), Err(BacnetError::ServiceChoice(1)));
    }

    #[test]
    fn rejects_non_device_and_malformed_application_values() {
        let mut bytes = i_am();
        bytes[9..13].copy_from_slice(&0x0040_007bu32.to_be_bytes());
        assert_eq!(decode_i_am(&bytes), Err(BacnetError::ObjectType(1)));

        let mut bytes = i_am();
        bytes[17] = 4;
        assert_eq!(
            decode_i_am(&bytes),
            Err(BacnetError::InvalidSegmentation(4))
        );

        let mut bytes = i_am();
        bytes.push(0);
        let length = bytes.len() as u16;
        bytes[2..4].copy_from_slice(&length.to_be_bytes());
        assert_eq!(decode_i_am(&bytes), Err(BacnetError::TrailingBytes(1)));
    }
}
