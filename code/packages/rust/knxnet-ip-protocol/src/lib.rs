//! Bounded KNXnet/IP Search Request and Search Response framing.

#![forbid(unsafe_code)]

use std::fmt;
use std::net::{Ipv4Addr, SocketAddrV4};

pub const VERSION: &str = "0.1.0";
pub const KNXNET_IP_DEFAULT_PORT: u16 = 3671;
pub const KNXNET_IP_SYSTEM_MULTICAST: Ipv4Addr = Ipv4Addr::new(224, 0, 23, 12);
pub const MAX_KNXNET_IP_DATAGRAM_BYTES: usize = 1_024;

const HEADER_LENGTH: u8 = 6;
const PROTOCOL_VERSION: u8 = 0x10;
const SEARCH_REQUEST: u16 = 0x0201;
const SEARCH_RESPONSE: u16 = 0x0202;
const HPAI_LENGTH: u8 = 8;
const HOST_PROTOCOL_UDP_IPV4: u8 = 1;
const DEVICE_INFO_DIB: u8 = 1;
const SUPPORTED_SERVICE_FAMILIES_DIB: u8 = 2;
const DEVICE_INFO_LENGTH: usize = 54;
const FRIENDLY_NAME_LENGTH: usize = 30;
const CORE_SERVICE_FAMILY: u8 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KnxMedium {
    TwistedPair,
    PowerLine,
    RadioFrequency,
    InternetProtocol,
}

impl KnxMedium {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::TwistedPair => "tp1",
            Self::PowerLine => "pl110",
            Self::RadioFrequency => "rf",
            Self::InternetProtocol => "ip",
        }
    }

    fn from_code(code: u8) -> Result<Self, KnxnetIpError> {
        match code {
            0x02 => Ok(Self::TwistedPair),
            0x04 => Ok(Self::PowerLine),
            0x10 => Ok(Self::RadioFrequency),
            0x20 => Ok(Self::InternetProtocol),
            _ => Err(KnxnetIpError::UnsupportedMedium(code)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct SupportedServiceFamily {
    pub family_id: u8,
    pub version: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchResponse {
    pub control_endpoint: SocketAddrV4,
    pub medium: KnxMedium,
    pub programming_mode: bool,
    pub individual_address: u16,
    pub project_id: u16,
    pub installation_id: u8,
    pub serial_number: [u8; 6],
    pub routing_multicast_address: Option<Ipv4Addr>,
    pub mac_address: [u8; 6],
    pub friendly_name: String,
    pub supported_service_families: Vec<SupportedServiceFamily>,
}

impl SearchResponse {
    pub fn individual_address_string(&self) -> String {
        format!(
            "{}.{}.{}",
            self.individual_address >> 12,
            (self.individual_address >> 8) & 0x0f,
            self.individual_address & 0xff
        )
    }

    pub fn serial_number_hex(&self) -> String {
        hex_bytes(&self.serial_number)
    }

    pub fn mac_address_hex(&self) -> String {
        self.mac_address
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<Vec<_>>()
            .join(":")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KnxnetIpError {
    InvalidResponseEndpoint(SocketAddrV4),
    DatagramTooShort { actual: usize },
    DatagramTooLarge { actual: usize, maximum: usize },
    HeaderLength(u8),
    ProtocolVersion(u8),
    ServiceType(u16),
    TotalLength { declared: usize, actual: usize },
    HpaiLength(u8),
    HostProtocol(u8),
    InvalidControlEndpoint(SocketAddrV4),
    TruncatedDib,
    DibLength { dib_type: u8, length: usize },
    DuplicateDib(u8),
    MissingDib(u8),
    UnsupportedMedium(u8),
    ZeroSerialNumber,
    RoutingMulticastAddress(Ipv4Addr),
    FriendlyName,
    MissingCoreServiceFamily,
}

impl fmt::Display for KnxnetIpError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidResponseEndpoint(endpoint) => write!(
                formatter,
                "KNXnet/IP response endpoint must be explicit unicast UDP/IPv4, got {endpoint}"
            ),
            Self::DatagramTooShort { actual } => {
                write!(formatter, "KNXnet/IP datagram is too short: {actual} bytes")
            }
            Self::DatagramTooLarge { actual, maximum } => write!(
                formatter,
                "KNXnet/IP datagram is {actual} bytes, exceeding the {maximum}-byte limit"
            ),
            Self::HeaderLength(actual) => {
                write!(formatter, "KNXnet/IP header length must be 6, got {actual}")
            }
            Self::ProtocolVersion(actual) => write!(
                formatter,
                "KNXnet/IP protocol version must be 0x10, got 0x{actual:02x}"
            ),
            Self::ServiceType(actual) => write!(
                formatter,
                "KNXnet/IP service type must be Search Response (0x0202), got 0x{actual:04x}"
            ),
            Self::TotalLength { declared, actual } => write!(
                formatter,
                "KNXnet/IP total length declares {declared} bytes but datagram contains {actual}"
            ),
            Self::HpaiLength(actual) => {
                write!(formatter, "KNXnet/IP HPAI length must be 8, got {actual}")
            }
            Self::HostProtocol(actual) => write!(
                formatter,
                "KNXnet/IP HPAI host protocol must be UDP/IPv4 (1), got {actual}"
            ),
            Self::InvalidControlEndpoint(endpoint) => write!(
                formatter,
                "KNXnet/IP Search Response contains invalid control endpoint {endpoint}"
            ),
            Self::TruncatedDib => write!(formatter, "KNXnet/IP DIB is truncated"),
            Self::DibLength { dib_type, length } => write!(
                formatter,
                "KNXnet/IP DIB type {dib_type} has invalid length {length}"
            ),
            Self::DuplicateDib(dib_type) => {
                write!(
                    formatter,
                    "KNXnet/IP Search Response repeats DIB type {dib_type}"
                )
            }
            Self::MissingDib(dib_type) => {
                write!(
                    formatter,
                    "KNXnet/IP Search Response is missing DIB type {dib_type}"
                )
            }
            Self::UnsupportedMedium(actual) => {
                write!(formatter, "unsupported KNX medium code 0x{actual:02x}")
            }
            Self::ZeroSerialNumber => {
                write!(
                    formatter,
                    "KNXnet/IP Device Information has an all-zero serial number"
                )
            }
            Self::RoutingMulticastAddress(address) => write!(
                formatter,
                "KNXnet/IP routing address must be multicast or unspecified, got {address}"
            ),
            Self::FriendlyName => write!(
                formatter,
                "KNXnet/IP friendly name contains embedded NUL or control bytes"
            ),
            Self::MissingCoreServiceFamily => write!(
                formatter,
                "KNXnet/IP Search Response does not advertise the Core service family"
            ),
        }
    }
}

impl std::error::Error for KnxnetIpError {}

pub fn encode_search_request(response_endpoint: SocketAddrV4) -> Result<Vec<u8>, KnxnetIpError> {
    validate_explicit_unicast(response_endpoint)
        .map_err(|_| KnxnetIpError::InvalidResponseEndpoint(response_endpoint))?;
    let total_length = usize::from(HEADER_LENGTH) + usize::from(HPAI_LENGTH);
    let mut bytes = Vec::with_capacity(total_length);
    encode_header(SEARCH_REQUEST, total_length, &mut bytes);
    encode_hpai(response_endpoint, &mut bytes);
    Ok(bytes)
}

pub fn decode_search_response(bytes: &[u8]) -> Result<SearchResponse, KnxnetIpError> {
    if bytes.len() < usize::from(HEADER_LENGTH) + usize::from(HPAI_LENGTH) + 2 {
        return Err(KnxnetIpError::DatagramTooShort {
            actual: bytes.len(),
        });
    }
    if bytes.len() > MAX_KNXNET_IP_DATAGRAM_BYTES {
        return Err(KnxnetIpError::DatagramTooLarge {
            actual: bytes.len(),
            maximum: MAX_KNXNET_IP_DATAGRAM_BYTES,
        });
    }
    if bytes[0] != HEADER_LENGTH {
        return Err(KnxnetIpError::HeaderLength(bytes[0]));
    }
    if bytes[1] != PROTOCOL_VERSION {
        return Err(KnxnetIpError::ProtocolVersion(bytes[1]));
    }
    let service_type = u16::from_be_bytes([bytes[2], bytes[3]]);
    if service_type != SEARCH_RESPONSE {
        return Err(KnxnetIpError::ServiceType(service_type));
    }
    let declared = usize::from(u16::from_be_bytes([bytes[4], bytes[5]]));
    if declared != bytes.len() {
        return Err(KnxnetIpError::TotalLength {
            declared,
            actual: bytes.len(),
        });
    }

    let mut offset = usize::from(HEADER_LENGTH);
    let control_endpoint = decode_hpai(bytes, &mut offset)?;
    let mut device_info = None;
    let mut service_families = None;
    while offset < bytes.len() {
        let length = usize::from(*bytes.get(offset).ok_or(KnxnetIpError::TruncatedDib)?);
        let dib_type = *bytes.get(offset + 1).ok_or(KnxnetIpError::TruncatedDib)?;
        if length < 2 || offset + length > bytes.len() {
            return Err(KnxnetIpError::DibLength { dib_type, length });
        }
        let dib = &bytes[offset..offset + length];
        match dib_type {
            DEVICE_INFO_DIB => {
                if device_info.replace(decode_device_info(dib)?).is_some() {
                    return Err(KnxnetIpError::DuplicateDib(dib_type));
                }
            }
            SUPPORTED_SERVICE_FAMILIES_DIB => {
                if service_families.is_some() {
                    return Err(KnxnetIpError::DuplicateDib(dib_type));
                }
                service_families = Some(decode_service_families(dib)?);
            }
            _ => {}
        }
        offset += length;
    }

    let device = device_info.ok_or(KnxnetIpError::MissingDib(DEVICE_INFO_DIB))?;
    let supported_service_families =
        service_families.ok_or(KnxnetIpError::MissingDib(SUPPORTED_SERVICE_FAMILIES_DIB))?;
    if !supported_service_families
        .iter()
        .any(|family| family.family_id == CORE_SERVICE_FAMILY)
    {
        return Err(KnxnetIpError::MissingCoreServiceFamily);
    }

    Ok(SearchResponse {
        control_endpoint,
        medium: device.medium,
        programming_mode: device.programming_mode,
        individual_address: device.individual_address,
        project_id: device.project_id,
        installation_id: device.installation_id,
        serial_number: device.serial_number,
        routing_multicast_address: device.routing_multicast_address,
        mac_address: device.mac_address,
        friendly_name: device.friendly_name,
        supported_service_families,
    })
}

#[derive(Debug)]
struct DeviceInfo {
    medium: KnxMedium,
    programming_mode: bool,
    individual_address: u16,
    project_id: u16,
    installation_id: u8,
    serial_number: [u8; 6],
    routing_multicast_address: Option<Ipv4Addr>,
    mac_address: [u8; 6],
    friendly_name: String,
}

fn encode_header(service_type: u16, total_length: usize, output: &mut Vec<u8>) {
    output.extend_from_slice(&[HEADER_LENGTH, PROTOCOL_VERSION]);
    output.extend_from_slice(&service_type.to_be_bytes());
    output.extend_from_slice(&(total_length as u16).to_be_bytes());
}

fn encode_hpai(endpoint: SocketAddrV4, output: &mut Vec<u8>) {
    output.extend_from_slice(&[HPAI_LENGTH, HOST_PROTOCOL_UDP_IPV4]);
    output.extend_from_slice(&endpoint.ip().octets());
    output.extend_from_slice(&endpoint.port().to_be_bytes());
}

fn decode_hpai(bytes: &[u8], offset: &mut usize) -> Result<SocketAddrV4, KnxnetIpError> {
    let hpai = bytes
        .get(*offset..*offset + usize::from(HPAI_LENGTH))
        .ok_or(KnxnetIpError::TruncatedDib)?;
    if hpai[0] != HPAI_LENGTH {
        return Err(KnxnetIpError::HpaiLength(hpai[0]));
    }
    if hpai[1] != HOST_PROTOCOL_UDP_IPV4 {
        return Err(KnxnetIpError::HostProtocol(hpai[1]));
    }
    let endpoint = SocketAddrV4::new(
        Ipv4Addr::new(hpai[2], hpai[3], hpai[4], hpai[5]),
        u16::from_be_bytes([hpai[6], hpai[7]]),
    );
    validate_explicit_unicast(endpoint)
        .map_err(|_| KnxnetIpError::InvalidControlEndpoint(endpoint))?;
    *offset += usize::from(HPAI_LENGTH);
    Ok(endpoint)
}

fn decode_device_info(dib: &[u8]) -> Result<DeviceInfo, KnxnetIpError> {
    if dib.len() != DEVICE_INFO_LENGTH {
        return Err(KnxnetIpError::DibLength {
            dib_type: DEVICE_INFO_DIB,
            length: dib.len(),
        });
    }
    let serial_number = dib[8..14].try_into().expect("fixed serial slice");
    if serial_number == [0; 6] {
        return Err(KnxnetIpError::ZeroSerialNumber);
    }
    let routing = Ipv4Addr::new(dib[14], dib[15], dib[16], dib[17]);
    let routing_multicast_address = if routing.is_unspecified() {
        None
    } else if routing.is_multicast() {
        Some(routing)
    } else {
        return Err(KnxnetIpError::RoutingMulticastAddress(routing));
    };
    let project_installation = u16::from_be_bytes([dib[6], dib[7]]);
    Ok(DeviceInfo {
        medium: KnxMedium::from_code(dib[2])?,
        programming_mode: dib[3] & 1 != 0,
        individual_address: u16::from_be_bytes([dib[4], dib[5]]),
        project_id: project_installation >> 4,
        installation_id: (project_installation & 0x0f) as u8,
        serial_number,
        routing_multicast_address,
        mac_address: dib[18..24].try_into().expect("fixed MAC slice"),
        friendly_name: decode_friendly_name(&dib[24..24 + FRIENDLY_NAME_LENGTH])?,
    })
}

fn decode_service_families(dib: &[u8]) -> Result<Vec<SupportedServiceFamily>, KnxnetIpError> {
    if dib.len() < 4 || !dib.len().is_multiple_of(2) {
        return Err(KnxnetIpError::DibLength {
            dib_type: SUPPORTED_SERVICE_FAMILIES_DIB,
            length: dib.len(),
        });
    }
    let mut families = dib[2..]
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| SupportedServiceFamily {
            family_id: pair[0],
            version: pair[1],
        })
        .collect::<Vec<_>>();
    families.sort_unstable();
    families.dedup();
    Ok(families)
}

fn decode_friendly_name(bytes: &[u8]) -> Result<String, KnxnetIpError> {
    let end = bytes
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(bytes.len());
    if bytes[end..].iter().any(|byte| *byte != 0)
        || bytes[..end]
            .iter()
            .any(|byte| *byte < 0x20 || *byte == 0x7f)
    {
        return Err(KnxnetIpError::FriendlyName);
    }
    Ok(bytes[..end].iter().map(|byte| char::from(*byte)).collect())
}

fn validate_explicit_unicast(endpoint: SocketAddrV4) -> Result<(), ()> {
    let ip = endpoint.ip();
    if endpoint.port() == 0 || ip.is_unspecified() || ip.is_broadcast() || ip.is_multicast() {
        Err(())
    } else {
        Ok(())
    }
}

fn hex_bytes(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn response() -> Vec<u8> {
        let mut bytes = vec![
            6, 0x10, 0x02, 0x02, 0, 74, 8, 1, 192, 0, 2, 10, 0x0e, 0x57, 54, 1, 0x02, 1, 0x11,
            0x0a, 0x12, 0x33, 1, 2, 3, 4, 5, 6, 224, 0, 23, 12, 0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff,
        ];
        let mut name = [0u8; FRIENDLY_NAME_LENGTH];
        name[..13].copy_from_slice(b"KNX Interface");
        bytes.extend_from_slice(&name);
        bytes.extend_from_slice(&[6, 2, 2, 1, 4, 1]);
        bytes
    }

    #[test]
    fn encodes_search_request_with_explicit_response_endpoint() {
        let endpoint: SocketAddrV4 = "192.0.2.5:50000".parse().unwrap();
        assert_eq!(
            encode_search_request(endpoint).unwrap(),
            [6, 0x10, 0x02, 0x01, 0, 14, 8, 1, 192, 0, 2, 5, 0xc3, 0x50]
        );
        assert!(encode_search_request("0.0.0.0:50000".parse().unwrap()).is_err());
        assert!(encode_search_request("224.0.23.12:3671".parse().unwrap()).is_err());
    }

    #[test]
    fn decodes_search_response_device_and_service_information() {
        let decoded = decode_search_response(&response()).unwrap();
        assert_eq!(decoded.control_endpoint.to_string(), "192.0.2.10:3671");
        assert_eq!(decoded.medium, KnxMedium::TwistedPair);
        assert!(decoded.programming_mode);
        assert_eq!(decoded.individual_address_string(), "1.1.10");
        assert_eq!(decoded.project_id, 0x123);
        assert_eq!(decoded.installation_id, 3);
        assert_eq!(decoded.serial_number_hex(), "010203040506");
        assert_eq!(
            decoded.routing_multicast_address,
            Some(KNXNET_IP_SYSTEM_MULTICAST)
        );
        assert_eq!(decoded.mac_address_hex(), "aa:bb:cc:dd:ee:ff");
        assert_eq!(decoded.friendly_name, "KNX Interface");
        assert_eq!(decoded.supported_service_families.len(), 2);
    }

    #[test]
    fn rejects_header_and_length_mismatches() {
        let mut bytes = response();
        bytes[0] = 7;
        assert!(matches!(
            decode_search_response(&bytes),
            Err(KnxnetIpError::HeaderLength(7))
        ));
        let mut bytes = response();
        bytes[3] = 1;
        assert!(matches!(
            decode_search_response(&bytes),
            Err(KnxnetIpError::ServiceType(0x0201))
        ));
        let mut bytes = response();
        bytes[5] -= 1;
        assert!(matches!(
            decode_search_response(&bytes),
            Err(KnxnetIpError::TotalLength { .. })
        ));
    }

    #[test]
    fn rejects_invalid_hpai_and_device_information() {
        let mut bytes = response();
        bytes[7] = 2;
        assert!(matches!(
            decode_search_response(&bytes),
            Err(KnxnetIpError::HostProtocol(2))
        ));
        let mut bytes = response();
        bytes[22..28].fill(0);
        assert!(matches!(
            decode_search_response(&bytes),
            Err(KnxnetIpError::ZeroSerialNumber)
        ));
        let mut bytes = response();
        bytes[28..32].copy_from_slice(&[192, 0, 2, 1]);
        assert!(matches!(
            decode_search_response(&bytes),
            Err(KnxnetIpError::RoutingMulticastAddress(_))
        ));
    }

    #[test]
    fn rejects_missing_or_duplicate_required_dibs() {
        let mut missing_services = response();
        missing_services.truncate(68);
        missing_services[4..6].copy_from_slice(&68u16.to_be_bytes());
        assert!(matches!(
            decode_search_response(&missing_services),
            Err(KnxnetIpError::MissingDib(2))
        ));

        let mut duplicate = response();
        duplicate.extend_from_slice(&[6, 2, 2, 1, 4, 1]);
        let length = duplicate.len() as u16;
        duplicate[4..6].copy_from_slice(&length.to_be_bytes());
        assert!(matches!(
            decode_search_response(&duplicate),
            Err(KnxnetIpError::DuplicateDib(2))
        ));
    }

    #[test]
    fn accepts_unknown_well_formed_dibs_but_requires_core_family() {
        let mut bytes = response();
        bytes.extend_from_slice(&[4, 0x7f, 1, 2]);
        let length = bytes.len() as u16;
        bytes[4..6].copy_from_slice(&length.to_be_bytes());
        assert!(decode_search_response(&bytes).is_ok());

        let mut missing_core = response();
        missing_core[70] = 3;
        assert!(matches!(
            decode_search_response(&missing_core),
            Err(KnxnetIpError::MissingCoreServiceFamily)
        ));
    }
}
