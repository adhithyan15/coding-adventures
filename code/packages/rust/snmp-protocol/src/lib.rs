//! Strict bounded SNMPv2c GET request and response framing.

#![forbid(unsafe_code)]

use std::fmt;

pub const VERSION: &str = "0.1.0";
pub const SNMP_V2C_VERSION: i64 = 1;
pub const MAX_COMMUNITY_BYTES: usize = 128;
pub const MAX_OIDS: usize = 32;
pub const MAX_OID_ARCS: usize = 128;
pub const MAX_OID_TEXT_BYTES: usize = 512;
pub const MAX_DATAGRAM_BYTES: usize = 1_472;
pub const MAX_VALUE_BYTES: usize = 1_024;

const TAG_INTEGER: u8 = 0x02;
const TAG_OCTET_STRING: u8 = 0x04;
const TAG_NULL: u8 = 0x05;
const TAG_OBJECT_IDENTIFIER: u8 = 0x06;
const TAG_SEQUENCE: u8 = 0x30;
const TAG_IP_ADDRESS: u8 = 0x40;
const TAG_COUNTER32: u8 = 0x41;
const TAG_GAUGE32: u8 = 0x42;
const TAG_TIME_TICKS: u8 = 0x43;
const TAG_OPAQUE: u8 = 0x44;
const TAG_COUNTER64: u8 = 0x46;
const TAG_GET_REQUEST: u8 = 0xa0;
const TAG_GET_RESPONSE: u8 = 0xa2;
const TAG_NO_SUCH_OBJECT: u8 = 0x80;
const TAG_NO_SUCH_INSTANCE: u8 = 0x81;
const TAG_END_OF_MIB_VIEW: u8 = 0x82;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ObjectIdentifier {
    arcs: Vec<u32>,
}

impl ObjectIdentifier {
    pub fn new(arcs: Vec<u32>) -> Result<Self, SnmpError> {
        validate_oid_arcs(&arcs)?;
        Ok(Self { arcs })
    }

    pub fn parse(value: &str) -> Result<Self, SnmpError> {
        if value.is_empty() || value.len() > MAX_OID_TEXT_BYTES {
            return Err(SnmpError::InvalidOid(
                "OID text must be non-empty and bounded".to_string(),
            ));
        }
        let value = value.strip_prefix('.').unwrap_or(value);
        if value.is_empty() {
            return Err(SnmpError::InvalidOid("OID contains no arcs".to_string()));
        }
        let arcs = value
            .split('.')
            .map(|arc| {
                if arc.is_empty() || (arc.len() > 1 && arc.starts_with('0')) {
                    return Err(SnmpError::InvalidOid(
                        "OID arcs must use canonical decimal notation".to_string(),
                    ));
                }
                arc.parse::<u32>().map_err(|_| {
                    SnmpError::InvalidOid("OID arc is not a 32-bit unsigned integer".to_string())
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        Self::new(arcs)
    }

    pub fn arcs(&self) -> &[u32] {
        &self.arcs
    }
}

impl fmt::Display for ObjectIdentifier {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (index, arc) in self.arcs.iter().enumerate() {
            if index > 0 {
                formatter.write_str(".")?;
            }
            write!(formatter, "{arc}")?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GetRequest {
    pub request_id: i32,
    pub oids: Vec<ObjectIdentifier>,
}

impl GetRequest {
    pub fn new(request_id: i32, oids: Vec<ObjectIdentifier>) -> Result<Self, SnmpError> {
        let request = Self { request_id, oids };
        request.validate()?;
        Ok(request)
    }

    fn validate(&self) -> Result<(), SnmpError> {
        if self.request_id <= 0 {
            return Err(SnmpError::InvalidRequestId(self.request_id));
        }
        if self.oids.is_empty() || self.oids.len() > MAX_OIDS {
            return Err(SnmpError::InvalidOidCount(self.oids.len()));
        }
        for oid in &self.oids {
            validate_oid_arcs(oid.arcs())?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SnmpValue {
    Integer(i64),
    OctetString(Vec<u8>),
    ObjectIdentifier(ObjectIdentifier),
    IpAddress([u8; 4]),
    Counter32(u32),
    Gauge32(u32),
    TimeTicks(u32),
    Counter64(u64),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VariableBinding {
    pub oid: ObjectIdentifier,
    pub value: SnmpValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GetResponse {
    pub request_id: i32,
    pub variable_bindings: Vec<VariableBinding>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SnmpError {
    InvalidCommunityLength(usize),
    InvalidRequestId(i32),
    InvalidOidCount(usize),
    InvalidOid(String),
    DatagramTooLarge {
        actual: usize,
        maximum: usize,
    },
    TruncatedBer,
    UnsupportedBerTag(u8),
    IndefiniteLength,
    NonCanonicalLength,
    LengthOverflow,
    TrailingData,
    InvalidInteger,
    IntegerOutOfRange,
    InvalidNull,
    InvalidVersion(i64),
    CommunityMismatch,
    UnexpectedPdu(u8),
    RequestIdMismatch {
        expected: i32,
        actual: i32,
    },
    AgentError {
        status: i64,
        index: i64,
    },
    VariableBindingCount {
        expected: usize,
        actual: usize,
    },
    OidMismatch {
        index: usize,
        expected: ObjectIdentifier,
        actual: ObjectIdentifier,
    },
    ExceptionValue {
        index: usize,
        tag: u8,
    },
    UnsupportedValueTag(u8),
    ValueTooLarge {
        actual: usize,
        maximum: usize,
    },
}

impl fmt::Display for SnmpError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidCommunityLength(actual) => write!(
                formatter,
                "SNMP community must contain between 1 and {MAX_COMMUNITY_BYTES} bytes, got {actual}"
            ),
            Self::InvalidRequestId(actual) => {
                write!(formatter, "SNMP request id must be positive, got {actual}")
            }
            Self::InvalidOidCount(actual) => write!(
                formatter,
                "SNMP GET must contain between 1 and {MAX_OIDS} OIDs, got {actual}"
            ),
            Self::InvalidOid(message) => write!(formatter, "invalid SNMP OID: {message}"),
            Self::DatagramTooLarge { actual, maximum } => write!(
                formatter,
                "SNMP datagram is {actual} bytes, exceeding the {maximum}-byte limit"
            ),
            Self::TruncatedBer => formatter.write_str("truncated BER value"),
            Self::UnsupportedBerTag(tag) => write!(formatter, "unsupported BER tag 0x{tag:02x}"),
            Self::IndefiniteLength => formatter.write_str("indefinite BER lengths are unsupported"),
            Self::NonCanonicalLength => formatter.write_str("non-canonical BER length"),
            Self::LengthOverflow => formatter.write_str("BER length overflow"),
            Self::TrailingData => formatter.write_str("trailing data after SNMP message"),
            Self::InvalidInteger => formatter.write_str("non-canonical BER integer"),
            Self::IntegerOutOfRange => formatter.write_str("BER integer is out of range"),
            Self::InvalidNull => formatter.write_str("SNMP NULL value must be empty"),
            Self::InvalidVersion(actual) => write!(
                formatter,
                "SNMP message version must be {SNMP_V2C_VERSION}, got {actual}"
            ),
            Self::CommunityMismatch => formatter.write_str("SNMP response community mismatch"),
            Self::UnexpectedPdu(tag) => write!(formatter, "unexpected SNMP PDU tag 0x{tag:02x}"),
            Self::RequestIdMismatch { expected, actual } => write!(
                formatter,
                "SNMP response request id mismatch: expected {expected}, got {actual}"
            ),
            Self::AgentError { status, index } => {
                write!(formatter, "SNMP agent error status {status} at index {index}")
            }
            Self::VariableBindingCount { expected, actual } => write!(
                formatter,
                "SNMP response variable-binding count mismatch: expected {expected}, got {actual}"
            ),
            Self::OidMismatch {
                index,
                expected,
                actual,
            } => write!(
                formatter,
                "SNMP response OID mismatch at index {index}: expected {expected}, got {actual}"
            ),
            Self::ExceptionValue { index, tag } => write!(
                formatter,
                "SNMP response exception 0x{tag:02x} at variable-binding index {index}"
            ),
            Self::UnsupportedValueTag(tag) => {
                write!(formatter, "unsupported SNMP value tag 0x{tag:02x}")
            }
            Self::ValueTooLarge { actual, maximum } => write!(
                formatter,
                "SNMP value is {actual} bytes, exceeding the {maximum}-byte limit"
            ),
        }
    }
}

impl std::error::Error for SnmpError {}

pub fn encode_get_request(community: &[u8], request: &GetRequest) -> Result<Vec<u8>, SnmpError> {
    validate_community(community)?;
    request.validate()?;

    let mut bindings = Vec::new();
    for oid in &request.oids {
        validate_oid_arcs(oid.arcs())?;
        let mut binding = Vec::new();
        push_tlv(TAG_OBJECT_IDENTIFIER, &encode_oid(oid)?, &mut binding);
        push_tlv(TAG_NULL, &[], &mut binding);
        push_tlv(TAG_SEQUENCE, &binding, &mut bindings);
    }

    let mut pdu = Vec::new();
    push_tlv(
        TAG_INTEGER,
        &encode_signed(request.request_id as i64),
        &mut pdu,
    );
    push_tlv(TAG_INTEGER, &[0], &mut pdu);
    push_tlv(TAG_INTEGER, &[0], &mut pdu);
    push_tlv(TAG_SEQUENCE, &bindings, &mut pdu);

    let mut message = Vec::new();
    push_tlv(TAG_INTEGER, &[SNMP_V2C_VERSION as u8], &mut message);
    push_tlv(TAG_OCTET_STRING, community, &mut message);
    push_tlv(TAG_GET_REQUEST, &pdu, &mut message);
    let mut encoded = Vec::new();
    push_tlv(TAG_SEQUENCE, &message, &mut encoded);
    if encoded.len() > MAX_DATAGRAM_BYTES {
        return Err(SnmpError::DatagramTooLarge {
            actual: encoded.len(),
            maximum: MAX_DATAGRAM_BYTES,
        });
    }
    Ok(encoded)
}

pub fn decode_get_response(
    bytes: &[u8],
    expected_community: &[u8],
    request: &GetRequest,
) -> Result<GetResponse, SnmpError> {
    validate_community(expected_community)?;
    request.validate()?;
    if bytes.len() > MAX_DATAGRAM_BYTES {
        return Err(SnmpError::DatagramTooLarge {
            actual: bytes.len(),
            maximum: MAX_DATAGRAM_BYTES,
        });
    }
    let mut root = Reader::new(bytes);
    let message = root.read_expected(TAG_SEQUENCE)?;
    root.finish()?;
    let mut message = Reader::new(message);
    let version = decode_signed(message.read_expected(TAG_INTEGER)?)?;
    if version != SNMP_V2C_VERSION {
        return Err(SnmpError::InvalidVersion(version));
    }
    let community = message.read_expected(TAG_OCTET_STRING)?;
    if community != expected_community {
        return Err(SnmpError::CommunityMismatch);
    }
    let pdu = message.read_tlv()?;
    if pdu.tag != TAG_GET_RESPONSE {
        return Err(SnmpError::UnexpectedPdu(pdu.tag));
    }
    message.finish()?;

    let mut pdu = Reader::new(pdu.value);
    let request_id = decode_i32(pdu.read_expected(TAG_INTEGER)?)?;
    if request_id != request.request_id {
        return Err(SnmpError::RequestIdMismatch {
            expected: request.request_id,
            actual: request_id,
        });
    }
    let error_status = decode_signed(pdu.read_expected(TAG_INTEGER)?)?;
    let error_index = decode_signed(pdu.read_expected(TAG_INTEGER)?)?;
    if error_status != 0 || error_index != 0 {
        return Err(SnmpError::AgentError {
            status: error_status,
            index: error_index,
        });
    }
    let bindings = pdu.read_expected(TAG_SEQUENCE)?;
    pdu.finish()?;
    let mut bindings = Reader::new(bindings);
    let mut decoded = Vec::with_capacity(request.oids.len());
    while !bindings.is_finished() {
        let binding = bindings.read_expected(TAG_SEQUENCE)?;
        let mut binding = Reader::new(binding);
        let oid = decode_oid(binding.read_expected(TAG_OBJECT_IDENTIFIER)?)?;
        let value = binding.read_tlv()?;
        binding.finish()?;
        decoded.push((oid, value));
        if decoded.len() > MAX_OIDS {
            return Err(SnmpError::VariableBindingCount {
                expected: request.oids.len(),
                actual: decoded.len(),
            });
        }
    }
    if decoded.len() != request.oids.len() {
        return Err(SnmpError::VariableBindingCount {
            expected: request.oids.len(),
            actual: decoded.len(),
        });
    }

    let mut variable_bindings = Vec::with_capacity(decoded.len());
    for (index, ((actual_oid, value), expected_oid)) in
        decoded.into_iter().zip(&request.oids).enumerate()
    {
        if &actual_oid != expected_oid {
            return Err(SnmpError::OidMismatch {
                index,
                expected: expected_oid.clone(),
                actual: actual_oid,
            });
        }
        variable_bindings.push(VariableBinding {
            oid: expected_oid.clone(),
            value: decode_value(index, value)?,
        });
    }
    Ok(GetResponse {
        request_id,
        variable_bindings,
    })
}

fn decode_value(index: usize, value: Tlv<'_>) -> Result<SnmpValue, SnmpError> {
    if value.value.len() > MAX_VALUE_BYTES {
        return Err(SnmpError::ValueTooLarge {
            actual: value.value.len(),
            maximum: MAX_VALUE_BYTES,
        });
    }
    match value.tag {
        TAG_INTEGER => decode_signed(value.value).map(SnmpValue::Integer),
        TAG_OCTET_STRING => Ok(SnmpValue::OctetString(value.value.to_vec())),
        TAG_OBJECT_IDENTIFIER => decode_oid(value.value).map(SnmpValue::ObjectIdentifier),
        TAG_IP_ADDRESS if value.value.len() == 4 => Ok(SnmpValue::IpAddress([
            value.value[0],
            value.value[1],
            value.value[2],
            value.value[3],
        ])),
        TAG_IP_ADDRESS => Err(SnmpError::UnsupportedValueTag(value.tag)),
        TAG_COUNTER32 => decode_u32(value.value).map(SnmpValue::Counter32),
        TAG_GAUGE32 => decode_u32(value.value).map(SnmpValue::Gauge32),
        TAG_TIME_TICKS => decode_u32(value.value).map(SnmpValue::TimeTicks),
        TAG_COUNTER64 => decode_unsigned(value.value).map(SnmpValue::Counter64),
        TAG_NO_SUCH_OBJECT | TAG_NO_SUCH_INSTANCE | TAG_END_OF_MIB_VIEW => {
            if !value.value.is_empty() {
                return Err(SnmpError::InvalidNull);
            }
            Err(SnmpError::ExceptionValue {
                index,
                tag: value.tag,
            })
        }
        TAG_NULL => Err(SnmpError::ExceptionValue {
            index,
            tag: value.tag,
        }),
        TAG_OPAQUE => Err(SnmpError::UnsupportedValueTag(value.tag)),
        tag => Err(SnmpError::UnsupportedValueTag(tag)),
    }
}

fn validate_community(community: &[u8]) -> Result<(), SnmpError> {
    if community.is_empty() || community.len() > MAX_COMMUNITY_BYTES {
        return Err(SnmpError::InvalidCommunityLength(community.len()));
    }
    Ok(())
}

fn validate_oid_arcs(arcs: &[u32]) -> Result<(), SnmpError> {
    if arcs.len() < 2 || arcs.len() > MAX_OID_ARCS {
        return Err(SnmpError::InvalidOid(format!(
            "OID must contain between 2 and {MAX_OID_ARCS} arcs"
        )));
    }
    if arcs[0] > 2 || (arcs[0] < 2 && arcs[1] > 39) {
        return Err(SnmpError::InvalidOid(
            "first arc must be 0..2 and second arc must be 0..39 when the first arc is 0 or 1"
                .to_string(),
        ));
    }
    if arcs[0] == 2 && arcs[1] > u32::MAX - 80 {
        return Err(SnmpError::InvalidOid(
            "second arc is too large for the first combined subidentifier".to_string(),
        ));
    }
    Ok(())
}

fn encode_oid(oid: &ObjectIdentifier) -> Result<Vec<u8>, SnmpError> {
    validate_oid_arcs(oid.arcs())?;
    let mut bytes = Vec::new();
    encode_subidentifier(oid.arcs[0] as u64 * 40 + oid.arcs[1] as u64, &mut bytes);
    for arc in &oid.arcs[2..] {
        encode_subidentifier(*arc as u64, &mut bytes);
    }
    Ok(bytes)
}

fn decode_oid(bytes: &[u8]) -> Result<ObjectIdentifier, SnmpError> {
    if bytes.is_empty() {
        return Err(SnmpError::InvalidOid("OID BER value is empty".to_string()));
    }
    let mut values = Vec::new();
    let mut position = 0;
    while position < bytes.len() {
        if values.len() >= MAX_OID_ARCS - 1 {
            return Err(SnmpError::InvalidOid(
                "OID contains too many arcs".to_string(),
            ));
        }
        let start = position;
        let mut value = 0_u64;
        loop {
            let byte = *bytes.get(position).ok_or(SnmpError::TruncatedBer)?;
            position += 1;
            if position - start > 5 || value > (u32::MAX as u64 + 80) >> 7 {
                return Err(SnmpError::InvalidOid(
                    "OID subidentifier overflow".to_string(),
                ));
            }
            if position - start == 1 && byte == 0x80 {
                return Err(SnmpError::InvalidOid(
                    "OID subidentifier is not canonical".to_string(),
                ));
            }
            value = (value << 7) | u64::from(byte & 0x7f);
            if byte & 0x80 == 0 {
                break;
            }
        }
        values.push(value);
    }
    let first = values[0];
    let (first_arc, second_arc) = if first < 40 {
        (0, first)
    } else if first < 80 {
        (1, first - 40)
    } else {
        (2, first - 80)
    };
    if second_arc > u32::MAX as u64 {
        return Err(SnmpError::InvalidOid(
            "OID second arc is out of range".to_string(),
        ));
    }
    let mut arcs = vec![first_arc, second_arc as u32];
    for value in values.into_iter().skip(1) {
        if value > u32::MAX as u64 {
            return Err(SnmpError::InvalidOid("OID arc is out of range".to_string()));
        }
        arcs.push(value as u32);
    }
    ObjectIdentifier::new(arcs)
}

fn encode_subidentifier(mut value: u64, output: &mut Vec<u8>) {
    let mut bytes = [0_u8; 10];
    let mut index = bytes.len();
    index -= 1;
    bytes[index] = (value & 0x7f) as u8;
    value >>= 7;
    while value > 0 {
        index -= 1;
        bytes[index] = ((value & 0x7f) as u8) | 0x80;
        value >>= 7;
    }
    output.extend_from_slice(&bytes[index..]);
}

fn encode_signed(value: i64) -> Vec<u8> {
    let bytes = value.to_be_bytes();
    let mut start = 0;
    while start < bytes.len() - 1
        && ((bytes[start] == 0 && bytes[start + 1] & 0x80 == 0)
            || (bytes[start] == 0xff && bytes[start + 1] & 0x80 != 0))
    {
        start += 1;
    }
    bytes[start..].to_vec()
}

fn decode_i32(bytes: &[u8]) -> Result<i32, SnmpError> {
    let value = decode_signed(bytes)?;
    i32::try_from(value).map_err(|_| SnmpError::IntegerOutOfRange)
}

fn decode_signed(bytes: &[u8]) -> Result<i64, SnmpError> {
    if bytes.is_empty() || bytes.len() > 8 {
        return Err(SnmpError::IntegerOutOfRange);
    }
    if bytes.len() > 1
        && ((bytes[0] == 0 && bytes[1] & 0x80 == 0) || (bytes[0] == 0xff && bytes[1] & 0x80 != 0))
    {
        return Err(SnmpError::InvalidInteger);
    }
    let mut value = if bytes[0] & 0x80 == 0 { 0_i64 } else { -1_i64 };
    for byte in bytes {
        value = (value << 8) | i64::from(*byte);
    }
    Ok(value)
}

fn decode_u32(bytes: &[u8]) -> Result<u32, SnmpError> {
    let value = decode_unsigned(bytes)?;
    u32::try_from(value).map_err(|_| SnmpError::IntegerOutOfRange)
}

fn decode_unsigned(bytes: &[u8]) -> Result<u64, SnmpError> {
    if bytes.is_empty() || bytes.len() > 9 || bytes[0] & 0x80 != 0 {
        return Err(SnmpError::IntegerOutOfRange);
    }
    if bytes.len() > 1 && bytes[0] == 0 && bytes[1] & 0x80 == 0 {
        return Err(SnmpError::InvalidInteger);
    }
    let bytes = if bytes.len() == 9 {
        if bytes[0] != 0 {
            return Err(SnmpError::IntegerOutOfRange);
        }
        &bytes[1..]
    } else {
        bytes
    };
    let mut value = 0_u64;
    for byte in bytes {
        value = (value << 8) | u64::from(*byte);
    }
    Ok(value)
}

fn push_tlv(tag: u8, value: &[u8], output: &mut Vec<u8>) {
    output.push(tag);
    encode_length(value.len(), output);
    output.extend_from_slice(value);
}

fn encode_length(length: usize, output: &mut Vec<u8>) {
    if length < 128 {
        output.push(length as u8);
        return;
    }
    let bytes = length.to_be_bytes();
    let start = bytes
        .iter()
        .position(|byte| *byte != 0)
        .unwrap_or(bytes.len() - 1);
    output.push(0x80 | (bytes.len() - start) as u8);
    output.extend_from_slice(&bytes[start..]);
}

#[derive(Debug, Clone, Copy)]
struct Tlv<'a> {
    tag: u8,
    value: &'a [u8],
}

struct Reader<'a> {
    bytes: &'a [u8],
    position: usize,
}

impl<'a> Reader<'a> {
    const fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, position: 0 }
    }

    fn is_finished(&self) -> bool {
        self.position == self.bytes.len()
    }

    fn finish(&self) -> Result<(), SnmpError> {
        if self.is_finished() {
            Ok(())
        } else {
            Err(SnmpError::TrailingData)
        }
    }

    fn read_expected(&mut self, tag: u8) -> Result<&'a [u8], SnmpError> {
        let value = self.read_tlv()?;
        if value.tag != tag {
            return Err(SnmpError::UnsupportedBerTag(value.tag));
        }
        Ok(value.value)
    }

    fn read_tlv(&mut self) -> Result<Tlv<'a>, SnmpError> {
        let tag = *self
            .bytes
            .get(self.position)
            .ok_or(SnmpError::TruncatedBer)?;
        self.position += 1;
        if tag & 0x1f == 0x1f {
            return Err(SnmpError::UnsupportedBerTag(tag));
        }
        let first = *self
            .bytes
            .get(self.position)
            .ok_or(SnmpError::TruncatedBer)?;
        self.position += 1;
        let length = if first & 0x80 == 0 {
            usize::from(first)
        } else {
            let octets = usize::from(first & 0x7f);
            if octets == 0 {
                return Err(SnmpError::IndefiniteLength);
            }
            if octets > std::mem::size_of::<usize>() {
                return Err(SnmpError::LengthOverflow);
            }
            let length_bytes = self
                .bytes
                .get(self.position..self.position + octets)
                .ok_or(SnmpError::TruncatedBer)?;
            self.position += octets;
            if length_bytes[0] == 0 {
                return Err(SnmpError::NonCanonicalLength);
            }
            let mut length = 0_usize;
            for byte in length_bytes {
                length = length
                    .checked_mul(256)
                    .and_then(|value| value.checked_add(usize::from(*byte)))
                    .ok_or(SnmpError::LengthOverflow)?;
            }
            if length < 128 {
                return Err(SnmpError::NonCanonicalLength);
            }
            length
        };
        let end = self
            .position
            .checked_add(length)
            .ok_or(SnmpError::LengthOverflow)?;
        let value = self
            .bytes
            .get(self.position..end)
            .ok_or(SnmpError::TruncatedBer)?;
        self.position = end;
        Ok(Tlv { tag, value })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn oid(value: &str) -> ObjectIdentifier {
        ObjectIdentifier::parse(value).unwrap()
    }

    fn unsigned_bytes(value: u64) -> Vec<u8> {
        let bytes = value.to_be_bytes();
        let start = bytes
            .iter()
            .position(|byte| *byte != 0)
            .unwrap_or(bytes.len() - 1);
        let mut encoded = bytes[start..].to_vec();
        if encoded[0] & 0x80 != 0 {
            encoded.insert(0, 0);
        }
        encoded
    }

    fn response(
        community: &[u8],
        request_id: i32,
        status: i64,
        index: i64,
        values: &[(ObjectIdentifier, u8, Vec<u8>)],
    ) -> Vec<u8> {
        let mut bindings = Vec::new();
        for (oid, tag, value) in values {
            let mut binding = Vec::new();
            push_tlv(
                TAG_OBJECT_IDENTIFIER,
                &encode_oid(oid).unwrap(),
                &mut binding,
            );
            push_tlv(*tag, value, &mut binding);
            push_tlv(TAG_SEQUENCE, &binding, &mut bindings);
        }
        let mut pdu = Vec::new();
        push_tlv(TAG_INTEGER, &encode_signed(request_id as i64), &mut pdu);
        push_tlv(TAG_INTEGER, &encode_signed(status), &mut pdu);
        push_tlv(TAG_INTEGER, &encode_signed(index), &mut pdu);
        push_tlv(TAG_SEQUENCE, &bindings, &mut pdu);
        let mut message = Vec::new();
        push_tlv(TAG_INTEGER, &[1], &mut message);
        push_tlv(TAG_OCTET_STRING, community, &mut message);
        push_tlv(TAG_GET_RESPONSE, &pdu, &mut message);
        let mut encoded = Vec::new();
        push_tlv(TAG_SEQUENCE, &message, &mut encoded);
        encoded
    }

    #[test]
    fn encodes_bounded_v2c_get_request() {
        let request = GetRequest::new(7, vec![oid("1.3.6.1.2.1.1.3.0")]).unwrap();
        let encoded = encode_get_request(b"monitor", &request).unwrap();
        assert_eq!(encoded[0], TAG_SEQUENCE);
        assert!(encoded.windows(7).any(|window| window == b"monitor"));
        assert!(encoded.contains(&TAG_GET_REQUEST));
        assert!(encoded.ends_with(&[TAG_NULL, 0]));
    }

    #[test]
    fn decodes_all_supported_response_values() {
        let oids = [
            "1.3.6.1.2.1.1.3.0",
            "1.3.6.1.2.1.1.5.0",
            "1.3.6.1.2.1.4.20.1.1.1",
            "1.3.6.1.2.1.2.2.1.10.1",
            "1.3.6.1.2.1.2.2.1.5.1",
            "1.3.6.1.2.1.2.2.1.9.1",
            "1.3.6.1.2.1.31.1.1.1.6.1",
            "1.3.6.1.2.1.1.2.0",
        ]
        .map(oid);
        let request = GetRequest::new(91, oids.to_vec()).unwrap();
        let encoded = response(
            b"secret",
            91,
            0,
            0,
            &[
                (oids[0].clone(), TAG_INTEGER, encode_signed(-4)),
                (oids[1].clone(), TAG_OCTET_STRING, b"node-a".to_vec()),
                (oids[2].clone(), TAG_IP_ADDRESS, vec![192, 168, 1, 4]),
                (oids[3].clone(), TAG_COUNTER32, unsigned_bytes(42)),
                (oids[4].clone(), TAG_GAUGE32, unsigned_bytes(1_000)),
                (oids[5].clone(), TAG_TIME_TICKS, unsigned_bytes(12_345)),
                (oids[6].clone(), TAG_COUNTER64, unsigned_bytes(u64::MAX)),
                (
                    oids[7].clone(),
                    TAG_OBJECT_IDENTIFIER,
                    encode_oid(&oid("1.3.6.1.4.1.9")).unwrap(),
                ),
            ],
        );
        let decoded = decode_get_response(&encoded, b"secret", &request).unwrap();
        assert_eq!(decoded.variable_bindings[0].value, SnmpValue::Integer(-4));
        assert_eq!(
            decoded.variable_bindings[1].value,
            SnmpValue::OctetString(b"node-a".to_vec())
        );
        assert_eq!(
            decoded.variable_bindings[2].value,
            SnmpValue::IpAddress([192, 168, 1, 4])
        );
        assert_eq!(decoded.variable_bindings[3].value, SnmpValue::Counter32(42));
        assert_eq!(
            decoded.variable_bindings[4].value,
            SnmpValue::Gauge32(1_000)
        );
        assert_eq!(
            decoded.variable_bindings[5].value,
            SnmpValue::TimeTicks(12_345)
        );
        assert_eq!(
            decoded.variable_bindings[6].value,
            SnmpValue::Counter64(u64::MAX)
        );
        assert_eq!(
            decoded.variable_bindings[7].value,
            SnmpValue::ObjectIdentifier(oid("1.3.6.1.4.1.9"))
        );
    }

    #[test]
    fn rejects_cross_request_responses() {
        let requested = oid("1.3.6.1.2.1.1.3.0");
        let request = GetRequest::new(4, vec![requested.clone()]).unwrap();
        let wrong_community = response(
            b"other",
            4,
            0,
            0,
            &[(requested.clone(), TAG_INTEGER, vec![1])],
        );
        assert_eq!(
            decode_get_response(&wrong_community, b"secret", &request),
            Err(SnmpError::CommunityMismatch)
        );
        let wrong_id = response(
            b"secret",
            5,
            0,
            0,
            &[(requested.clone(), TAG_INTEGER, vec![1])],
        );
        assert!(matches!(
            decode_get_response(&wrong_id, b"secret", &request),
            Err(SnmpError::RequestIdMismatch { .. })
        ));
        let wrong_oid = response(
            b"secret",
            4,
            0,
            0,
            &[(oid("1.3.6.1.2.1.1.5.0"), TAG_INTEGER, vec![1])],
        );
        assert!(matches!(
            decode_get_response(&wrong_oid, b"secret", &request),
            Err(SnmpError::OidMismatch { .. })
        ));
    }

    #[test]
    fn rejects_agent_errors_and_exception_values() {
        let requested = oid("1.3.6.1.2.1.1.3.0");
        let request = GetRequest::new(4, vec![requested.clone()]).unwrap();
        let error = response(
            b"secret",
            4,
            2,
            1,
            &[(requested.clone(), TAG_NULL, Vec::new())],
        );
        assert_eq!(
            decode_get_response(&error, b"secret", &request),
            Err(SnmpError::AgentError {
                status: 2,
                index: 1
            })
        );
        let exception = response(
            b"secret",
            4,
            0,
            0,
            &[(requested, TAG_NO_SUCH_INSTANCE, Vec::new())],
        );
        assert_eq!(
            decode_get_response(&exception, b"secret", &request),
            Err(SnmpError::ExceptionValue {
                index: 0,
                tag: TAG_NO_SUCH_INSTANCE,
            })
        );
    }

    #[test]
    fn rejects_noncanonical_or_trailing_ber() {
        let request = GetRequest::new(1, vec![oid("1.3.6.1.2.1.1.3.0")]).unwrap();
        assert_eq!(
            decode_get_response(&[TAG_SEQUENCE, 0x80, 0, 0], b"x", &request),
            Err(SnmpError::IndefiniteLength)
        );
        assert_eq!(
            decode_get_response(&[TAG_SEQUENCE, 0x81, 0x01, 0], b"x", &request),
            Err(SnmpError::NonCanonicalLength)
        );
        let mut valid = response(
            b"x",
            1,
            0,
            0,
            &[(oid("1.3.6.1.2.1.1.3.0"), TAG_INTEGER, vec![1])],
        );
        valid.push(0);
        assert_eq!(
            decode_get_response(&valid, b"x", &request),
            Err(SnmpError::TrailingData)
        );
    }

    #[test]
    fn validates_oid_and_request_bounds() {
        assert!(ObjectIdentifier::parse("1.40.1").is_err());
        assert!(ObjectIdentifier::parse("1.03.6").is_err());
        assert!(ObjectIdentifier::parse("3.1").is_err());
        assert!(GetRequest::new(0, vec![oid("1.3")]).is_err());
        assert!(GetRequest::new(1, Vec::new()).is_err());
        assert!(GetRequest::new(1, vec![oid("1.3"); MAX_OIDS + 1]).is_err());
        assert!(encode_get_request(&[], &GetRequest::new(1, vec![oid("1.3")]).unwrap()).is_err());
    }
}
