//! # dns-message
//!
//! `dns-message` is the transport-agnostic DNS wire-format layer. It knows how
//! to turn structured DNS questions and answers into bytes, and how to turn
//! bytes back into structured messages. It does not open sockets, retry
//! requests, cache answers, or decide which nameserver to use.
//!
//! Keeping this crate pure is what lets a future resolver send the same DNS
//! message over UDP, TCP, a simulated network stack, or test fixtures.

use std::collections::HashSet;
use std::fmt;

const DNS_HEADER_LEN: usize = 12;
const MAX_LABEL_LEN: usize = 63;
const MAX_ENCODED_NAME_LEN: usize = 255;
const MIN_QUESTION_WIRE_LEN: usize = 5;
const MIN_RECORD_WIRE_LEN: usize = 11;
const MAX_NAME_POINTER_HOPS: usize = 128;

/// A DNS domain name represented as human-readable labels.
///
/// DNS encodes `info.cern.ch` as length-prefixed labels on the wire:
/// `4 info, 4 cern, 2 ch, 0 root`. Keeping labels structured avoids leaking
/// that byte-level encoding into callers.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DnsName {
    pub labels: Vec<String>,
}

impl DnsName {
    /// Parse a dotted ASCII DNS name.
    ///
    /// The first version keeps names ASCII-only. Internationalized domain names
    /// can be handled later by punycode before constructing `DnsName`.
    pub fn from_ascii(input: &str) -> Result<Self, DnsError> {
        let trimmed = input.trim_end_matches('.');
        if input == "." || trimmed.is_empty() {
            return Ok(Self { labels: Vec::new() });
        }

        let mut labels = Vec::new();
        for label in trimmed.split('.') {
            if label.is_empty() {
                return Err(DnsError::Unsupported("empty DNS label"));
            }
            validate_label(label.as_bytes())?;
            labels.push(label.to_string());
        }

        let name = Self { labels };
        validate_encoded_name_len(&name)?;
        Ok(name)
    }

    /// Return true when this is the root name (`.`).
    pub fn is_root(&self) -> bool {
        self.labels.is_empty()
    }
}

impl fmt::Display for DnsName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.labels.is_empty() {
            write!(f, ".")
        } else {
            write!(f, "{}", self.labels.join("."))
        }
    }
}

/// DNS operation code from the header flag word.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DnsOpcode {
    Query,
    Unknown(u8),
}

impl DnsOpcode {
    fn from_bits(bits: u8) -> Self {
        match bits {
            0 => Self::Query,
            other => Self::Unknown(other),
        }
    }

    fn to_bits(self) -> u8 {
        match self {
            Self::Query => 0,
            Self::Unknown(value) => value & 0x0f,
        }
    }
}

/// DNS response code from the low bits of the header flag word.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DnsResponseCode {
    NoError,
    FormatError,
    ServerFailure,
    NameError,
    NotImplemented,
    Refused,
    Unknown(u8),
}

impl DnsResponseCode {
    fn from_bits(bits: u8) -> Self {
        match bits {
            0 => Self::NoError,
            1 => Self::FormatError,
            2 => Self::ServerFailure,
            3 => Self::NameError,
            4 => Self::NotImplemented,
            5 => Self::Refused,
            other => Self::Unknown(other),
        }
    }

    fn to_bits(self) -> u8 {
        match self {
            Self::NoError => 0,
            Self::FormatError => 1,
            Self::ServerFailure => 2,
            Self::NameError => 3,
            Self::NotImplemented => 4,
            Self::Refused => 5,
            Self::Unknown(value) => value & 0x0f,
        }
    }
}

/// The DNS header's packed flag word, exposed as named protocol facts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DnsFlags {
    pub is_response: bool,
    pub opcode: DnsOpcode,
    pub authoritative_answer: bool,
    pub truncated: bool,
    pub recursion_desired: bool,
    pub recursion_available: bool,
    pub response_code: DnsResponseCode,
}

impl DnsFlags {
    pub fn query() -> Self {
        Self {
            is_response: false,
            opcode: DnsOpcode::Query,
            authoritative_answer: false,
            truncated: false,
            recursion_desired: true,
            recursion_available: false,
            response_code: DnsResponseCode::NoError,
        }
    }

    fn parse(word: u16) -> Self {
        Self {
            is_response: word & 0x8000 != 0,
            opcode: DnsOpcode::from_bits(((word >> 11) & 0x0f) as u8),
            authoritative_answer: word & 0x0400 != 0,
            truncated: word & 0x0200 != 0,
            recursion_desired: word & 0x0100 != 0,
            recursion_available: word & 0x0080 != 0,
            response_code: DnsResponseCode::from_bits((word & 0x000f) as u8),
        }
    }

    fn serialize(&self) -> u16 {
        let mut word = 0u16;
        if self.is_response {
            word |= 0x8000;
        }
        word |= (self.opcode.to_bits() as u16) << 11;
        if self.authoritative_answer {
            word |= 0x0400;
        }
        if self.truncated {
            word |= 0x0200;
        }
        if self.recursion_desired {
            word |= 0x0100;
        }
        if self.recursion_available {
            word |= 0x0080;
        }
        word | self.response_code.to_bits() as u16
    }
}

/// The fixed 12-byte DNS message header.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DnsHeader {
    pub id: u16,
    pub flags: DnsFlags,
    pub question_count: u16,
    pub answer_count: u16,
    pub authority_count: u16,
    pub additional_count: u16,
}

/// DNS resource record type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DnsRecordType {
    A,
    NS,
    CNAME,
    SOA,
    PTR,
    MX,
    TXT,
    AAAA,
    Unknown(u16),
}

impl DnsRecordType {
    fn from_u16(value: u16) -> Self {
        match value {
            1 => Self::A,
            2 => Self::NS,
            5 => Self::CNAME,
            6 => Self::SOA,
            12 => Self::PTR,
            15 => Self::MX,
            16 => Self::TXT,
            28 => Self::AAAA,
            other => Self::Unknown(other),
        }
    }

    fn to_u16(self) -> u16 {
        match self {
            Self::A => 1,
            Self::NS => 2,
            Self::CNAME => 5,
            Self::SOA => 6,
            Self::PTR => 12,
            Self::MX => 15,
            Self::TXT => 16,
            Self::AAAA => 28,
            Self::Unknown(value) => value,
        }
    }
}

/// DNS class. The internet class (`IN`) is the normal class for browser DNS.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DnsClass {
    IN,
    Unknown(u16),
}

impl DnsClass {
    fn from_u16(value: u16) -> Self {
        match value {
            1 => Self::IN,
            other => Self::Unknown(other),
        }
    }

    fn to_u16(self) -> u16 {
        match self {
            Self::IN => 1,
            Self::Unknown(value) => value,
        }
    }
}

/// A DNS question from the question section.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DnsQuestion {
    pub name: DnsName,
    pub qtype: DnsRecordType,
    pub qclass: DnsClass,
}

/// The interpreted payload of a DNS resource record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DnsRecordData {
    A([u8; 4]),
    AAAA([u8; 16]),
    CNAME(DnsName),
    Raw(Vec<u8>),
}

/// A DNS resource record from answer, authority, or additional sections.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DnsResourceRecord {
    pub name: DnsName,
    pub rrtype: DnsRecordType,
    pub class: DnsClass,
    pub ttl: u32,
    pub data: DnsRecordData,
}

/// A complete DNS message with all four RFC 1035 sections.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DnsMessage {
    pub header: DnsHeader,
    pub questions: Vec<DnsQuestion>,
    pub answers: Vec<DnsResourceRecord>,
    pub authorities: Vec<DnsResourceRecord>,
    pub additionals: Vec<DnsResourceRecord>,
}

impl DnsMessage {
    /// True when this is a successful response.
    pub fn is_success(&self) -> bool {
        self.header.flags.is_response && self.header.flags.response_code == DnsResponseCode::NoError
    }

    /// Return the first answer matching the requested record type.
    pub fn first_answer_of_type(&self, qtype: DnsRecordType) -> Option<&DnsResourceRecord> {
        self.answers.iter().find(|record| record.rrtype == qtype)
    }

    /// Return all IPv4 answers from the answer section.
    pub fn ipv4_answers(&self) -> Vec<[u8; 4]> {
        self.answers
            .iter()
            .filter_map(|record| match record.data {
                DnsRecordData::A(address) => Some(address),
                _ => None,
            })
            .collect()
    }

    /// Return all IPv6 answers from the answer section.
    pub fn ipv6_answers(&self) -> Vec<[u8; 16]> {
        self.answers
            .iter()
            .filter_map(|record| match record.data {
                DnsRecordData::AAAA(address) => Some(address),
                _ => None,
            })
            .collect()
    }
}

/// Structural errors found while encoding or decoding DNS messages.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DnsError {
    TruncatedHeader,
    UnexpectedEof,
    LabelTooLong { length: usize },
    NameTooLong,
    PointerOutOfBounds { offset: usize },
    PointerLoop,
    NonAsciiLabel,
    InvalidSectionCount,
    Unsupported(&'static str),
}

/// Build a standard recursive single-question DNS query.
pub fn build_query(id: u16, name: DnsName, qtype: DnsRecordType) -> DnsMessage {
    DnsMessage {
        header: DnsHeader {
            id,
            flags: DnsFlags::query(),
            question_count: 1,
            answer_count: 0,
            authority_count: 0,
            additional_count: 0,
        },
        questions: vec![DnsQuestion {
            name,
            qtype,
            qclass: DnsClass::IN,
        }],
        answers: Vec::new(),
        authorities: Vec::new(),
        additionals: Vec::new(),
    }
}

/// Parse raw DNS wire-format bytes into a structured message.
pub fn parse_dns_message(input: &[u8]) -> Result<DnsMessage, DnsError> {
    if input.len() < DNS_HEADER_LEN {
        return Err(DnsError::TruncatedHeader);
    }

    let mut cursor = 0;
    let header = DnsHeader {
        id: read_u16(input, &mut cursor)?,
        flags: DnsFlags::parse(read_u16(input, &mut cursor)?),
        question_count: read_u16(input, &mut cursor)?,
        answer_count: read_u16(input, &mut cursor)?,
        authority_count: read_u16(input, &mut cursor)?,
        additional_count: read_u16(input, &mut cursor)?,
    };

    let questions = parse_questions(input, &mut cursor, header.question_count)?;
    let answers = parse_records(input, &mut cursor, header.answer_count)?;
    let authorities = parse_records(input, &mut cursor, header.authority_count)?;
    let additionals = parse_records(input, &mut cursor, header.additional_count)?;

    Ok(DnsMessage {
        header,
        questions,
        answers,
        authorities,
        additionals,
    })
}

/// Serialize a structured DNS message into wire-format bytes.
///
/// V1 emits uncompressed names. It can encode common browser-facing records and
/// raw unknown records, which is enough for fixtures and simple servers.
pub fn serialize_dns_message(message: &DnsMessage) -> Result<Vec<u8>, DnsError> {
    if message.questions.len() > u16::MAX as usize
        || message.answers.len() > u16::MAX as usize
        || message.authorities.len() > u16::MAX as usize
        || message.additionals.len() > u16::MAX as usize
    {
        return Err(DnsError::InvalidSectionCount);
    }

    let mut output = Vec::new();
    write_u16(&mut output, message.header.id);
    write_u16(&mut output, message.header.flags.serialize());
    write_u16(&mut output, message.questions.len() as u16);
    write_u16(&mut output, message.answers.len() as u16);
    write_u16(&mut output, message.authorities.len() as u16);
    write_u16(&mut output, message.additionals.len() as u16);

    for question in &message.questions {
        write_name(&mut output, &question.name)?;
        write_u16(&mut output, question.qtype.to_u16());
        write_u16(&mut output, question.qclass.to_u16());
    }

    for record in &message.answers {
        write_record(&mut output, record)?;
    }
    for record in &message.authorities {
        write_record(&mut output, record)?;
    }
    for record in &message.additionals {
        write_record(&mut output, record)?;
    }

    Ok(output)
}

fn parse_questions(
    input: &[u8],
    cursor: &mut usize,
    count: u16,
) -> Result<Vec<DnsQuestion>, DnsError> {
    let mut questions = Vec::with_capacity(section_capacity(
        input,
        *cursor,
        count,
        MIN_QUESTION_WIRE_LEN,
    ));
    for _ in 0..count {
        let name = read_name(input, cursor)?;
        let qtype = DnsRecordType::from_u16(read_u16(input, cursor)?);
        let qclass = DnsClass::from_u16(read_u16(input, cursor)?);
        questions.push(DnsQuestion {
            name,
            qtype,
            qclass,
        });
    }
    Ok(questions)
}

fn parse_records(
    input: &[u8],
    cursor: &mut usize,
    count: u16,
) -> Result<Vec<DnsResourceRecord>, DnsError> {
    let mut records =
        Vec::with_capacity(section_capacity(input, *cursor, count, MIN_RECORD_WIRE_LEN));
    for _ in 0..count {
        let name = read_name(input, cursor)?;
        let rrtype = DnsRecordType::from_u16(read_u16(input, cursor)?);
        let class = DnsClass::from_u16(read_u16(input, cursor)?);
        let ttl = read_u32(input, cursor)?;
        let rdlength = read_u16(input, cursor)? as usize;
        if input.len().saturating_sub(*cursor) < rdlength {
            return Err(DnsError::UnexpectedEof);
        }

        let rdata_start = *cursor;
        let rdata_end = rdata_start + rdlength;
        let data = match rrtype {
            DnsRecordType::A => {
                if rdlength != 4 {
                    return Err(DnsError::Unsupported("A record data must be 4 bytes"));
                }
                DnsRecordData::A(input[rdata_start..rdata_end].try_into().unwrap())
            }
            DnsRecordType::AAAA => {
                if rdlength != 16 {
                    return Err(DnsError::Unsupported("AAAA record data must be 16 bytes"));
                }
                DnsRecordData::AAAA(input[rdata_start..rdata_end].try_into().unwrap())
            }
            DnsRecordType::CNAME => {
                let mut data_cursor = rdata_start;
                let cname = read_name(input, &mut data_cursor)?;
                if data_cursor > rdata_end {
                    return Err(DnsError::UnexpectedEof);
                }
                if data_cursor != rdata_end {
                    return Err(DnsError::Unsupported(
                        "CNAME record data must contain exactly one DNS name",
                    ));
                }
                DnsRecordData::CNAME(cname)
            }
            _ => DnsRecordData::Raw(input[rdata_start..rdata_end].to_vec()),
        };

        *cursor = rdata_end;
        records.push(DnsResourceRecord {
            name,
            rrtype,
            class,
            ttl,
            data,
        });
    }
    Ok(records)
}

fn read_name(input: &[u8], cursor: &mut usize) -> Result<DnsName, DnsError> {
    let mut labels = Vec::new();
    let mut offset = *cursor;
    let mut consumed_cursor = None;
    let mut visited = HashSet::new();
    let mut pointer_hops = 0usize;
    let mut encoded_len = 1usize;

    loop {
        if offset >= input.len() {
            return Err(DnsError::UnexpectedEof);
        }
        if !visited.insert(offset) {
            return Err(DnsError::PointerLoop);
        }

        let len = input[offset];
        match len & 0b1100_0000 {
            0b0000_0000 => {
                offset += 1;
                if len == 0 {
                    *cursor = consumed_cursor.unwrap_or(offset);
                    let name = DnsName { labels };
                    validate_encoded_name_len(&name)?;
                    return Ok(name);
                }

                let label_len = len as usize;
                if label_len > MAX_LABEL_LEN {
                    return Err(DnsError::LabelTooLong {
                        length: len as usize,
                    });
                }
                if input.len().saturating_sub(offset) < label_len {
                    return Err(DnsError::UnexpectedEof);
                }

                let label_bytes = &input[offset..offset + label_len];
                validate_label(label_bytes)?;
                encoded_len += 1 + label_len;
                if encoded_len > MAX_ENCODED_NAME_LEN {
                    return Err(DnsError::NameTooLong);
                }

                labels.push(
                    String::from_utf8(label_bytes.to_vec()).map_err(|_| DnsError::NonAsciiLabel)?,
                );
                offset += label_len;
            }
            0b1100_0000 => {
                if input.len().saturating_sub(offset) < 2 {
                    return Err(DnsError::UnexpectedEof);
                }
                if consumed_cursor.is_none() {
                    consumed_cursor = Some(offset + 2);
                }
                pointer_hops += 1;
                if pointer_hops > MAX_NAME_POINTER_HOPS {
                    return Err(DnsError::PointerLoop);
                }
                let pointer = (((len as usize) & 0x3f) << 8) | input[offset + 1] as usize;
                if pointer >= input.len() {
                    return Err(DnsError::PointerOutOfBounds { offset: pointer });
                }
                offset = pointer;
            }
            _ => return Err(DnsError::Unsupported("reserved DNS label prefix")),
        }
    }
}

fn section_capacity(input: &[u8], cursor: usize, count: u16, minimum_entry_len: usize) -> usize {
    // Header counts are attacker-controlled. Reserve only what the remaining
    // bytes could possibly contain; the parser still verifies each entry as it
    // goes so callers get precise EOF errors for malformed sections.
    let possible_entries = input.len().saturating_sub(cursor) / minimum_entry_len;
    usize::from(count).min(possible_entries)
}

fn write_name(output: &mut Vec<u8>, name: &DnsName) -> Result<(), DnsError> {
    validate_encoded_name_len(name)?;
    for label in &name.labels {
        let bytes = label.as_bytes();
        validate_label(bytes)?;
        output.push(bytes.len() as u8);
        output.extend_from_slice(bytes);
    }
    output.push(0);
    Ok(())
}

fn write_record(output: &mut Vec<u8>, record: &DnsResourceRecord) -> Result<(), DnsError> {
    write_name(output, &record.name)?;
    write_u16(output, record.rrtype.to_u16());
    write_u16(output, record.class.to_u16());
    write_u32(output, record.ttl);

    let mut data = Vec::new();
    match &record.data {
        DnsRecordData::A(address) => data.extend_from_slice(address),
        DnsRecordData::AAAA(address) => data.extend_from_slice(address),
        DnsRecordData::CNAME(name) => write_name(&mut data, name)?,
        DnsRecordData::Raw(bytes) => data.extend_from_slice(bytes),
    }

    if data.len() > u16::MAX as usize {
        return Err(DnsError::Unsupported("record data too large"));
    }
    write_u16(output, data.len() as u16);
    output.extend_from_slice(&data);
    Ok(())
}

fn validate_label(label: &[u8]) -> Result<(), DnsError> {
    if label.len() > MAX_LABEL_LEN {
        return Err(DnsError::LabelTooLong {
            length: label.len(),
        });
    }
    if !label.is_ascii() {
        return Err(DnsError::NonAsciiLabel);
    }
    Ok(())
}

fn validate_encoded_name_len(name: &DnsName) -> Result<(), DnsError> {
    let len = name.labels.iter().try_fold(1usize, |acc, label| {
        let label_len = label.as_bytes().len();
        if label_len > MAX_LABEL_LEN {
            return Err(DnsError::LabelTooLong { length: label_len });
        }
        Ok(acc + 1 + label_len)
    })?;
    if len > MAX_ENCODED_NAME_LEN {
        Err(DnsError::NameTooLong)
    } else {
        Ok(())
    }
}

fn read_u16(input: &[u8], cursor: &mut usize) -> Result<u16, DnsError> {
    if input.len().saturating_sub(*cursor) < 2 {
        return Err(DnsError::UnexpectedEof);
    }
    let value = u16::from_be_bytes([input[*cursor], input[*cursor + 1]]);
    *cursor += 2;
    Ok(value)
}

fn read_u32(input: &[u8], cursor: &mut usize) -> Result<u32, DnsError> {
    if input.len().saturating_sub(*cursor) < 4 {
        return Err(DnsError::UnexpectedEof);
    }
    let value = u32::from_be_bytes([
        input[*cursor],
        input[*cursor + 1],
        input[*cursor + 2],
        input[*cursor + 3],
    ]);
    *cursor += 4;
    Ok(value)
}

fn write_u16(output: &mut Vec<u8>, value: u16) {
    output.extend_from_slice(&value.to_be_bytes());
}

fn write_u32(output: &mut Vec<u8>, value: u32) {
    output.extend_from_slice(&value.to_be_bytes());
}

#[cfg(test)]
mod tests {
    use super::*;

    fn info_cern_query_bytes() -> Vec<u8> {
        vec![
            0x12, 0x34, 0x01, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x04, b'i',
            b'n', b'f', b'o', 0x04, b'c', b'e', b'r', b'n', 0x02, b'c', b'h', 0x00, 0x00, 0x01,
            0x00, 0x01,
        ]
    }

    fn empty_message() -> DnsMessage {
        DnsMessage {
            header: DnsHeader {
                id: 0x9999,
                flags: DnsFlags::query(),
                question_count: 0,
                answer_count: 0,
                authority_count: 0,
                additional_count: 0,
            },
            questions: vec![],
            answers: vec![],
            authorities: vec![],
            additionals: vec![],
        }
    }

    #[test]
    fn builds_and_serializes_a_query_without_transport_assumptions() {
        let query = build_query(
            0x1234,
            DnsName::from_ascii("info.cern.ch").unwrap(),
            DnsRecordType::A,
        );

        assert_eq!(
            serialize_dns_message(&query).unwrap(),
            info_cern_query_bytes()
        );
    }

    #[test]
    fn parses_a_query_round_trip() {
        let parsed = parse_dns_message(&info_cern_query_bytes()).unwrap();

        assert_eq!(parsed.header.id, 0x1234);
        assert!(!parsed.header.flags.is_response);
        assert!(parsed.header.flags.recursion_desired);
        assert_eq!(parsed.questions.len(), 1);
        assert_eq!(parsed.questions[0].name.to_string(), "info.cern.ch");
        assert_eq!(parsed.questions[0].qtype, DnsRecordType::A);
        assert_eq!(parsed.questions[0].qclass, DnsClass::IN);
    }

    #[test]
    fn root_names_display_and_report_root() {
        let root = DnsName::from_ascii(".").unwrap();
        let trailing_dot = DnsName::from_ascii("example.com.").unwrap();

        assert!(root.is_root());
        assert_eq!(root.to_string(), ".");
        assert!(!trailing_dot.is_root());
        assert_eq!(trailing_dot.to_string(), "example.com");
    }

    #[test]
    fn rejects_empty_internal_label() {
        assert_eq!(
            DnsName::from_ascii("bad..example"),
            Err(DnsError::Unsupported("empty DNS label"))
        );
    }

    #[test]
    fn rejects_names_longer_than_wire_limit() {
        let label = "a".repeat(63);
        let too_long = format!("{label}.{label}.{label}.{label}");

        assert_eq!(DnsName::from_ascii(&too_long), Err(DnsError::NameTooLong));
    }

    #[test]
    fn parses_compressed_a_response() {
        let mut bytes = info_cern_query_bytes();
        bytes[2] = 0x81;
        bytes[3] = 0x80;
        bytes[6] = 0x00;
        bytes[7] = 0x01;
        bytes.extend_from_slice(&[
            0xc0, 0x0c, 0x00, 0x01, 0x00, 0x01, 0x00, 0x00, 0x01, 0x2c, 0x00, 0x04, 188, 184, 21,
            108,
        ]);

        let parsed = parse_dns_message(&bytes).unwrap();

        assert!(parsed.is_success());
        assert_eq!(parsed.answers.len(), 1);
        assert_eq!(parsed.answers[0].name.to_string(), "info.cern.ch");
        assert_eq!(parsed.answers[0].ttl, 300);
        assert_eq!(parsed.ipv4_answers(), vec![[188, 184, 21, 108]]);
    }

    #[test]
    fn parses_cname_response_with_compressed_target() {
        let mut bytes = info_cern_query_bytes();
        bytes[2] = 0x81;
        bytes[3] = 0x80;
        bytes[6] = 0x00;
        bytes[7] = 0x01;
        bytes.extend_from_slice(&[
            0xc0, 0x0c, 0x00, 0x05, 0x00, 0x01, 0x00, 0x00, 0x00, 0x3c, 0x00, 0x08, 0x05, b'a',
            b'l', b'i', b'a', b's', 0xc0, 0x11,
        ]);

        let parsed = parse_dns_message(&bytes).unwrap();

        assert_eq!(parsed.answers[0].rrtype, DnsRecordType::CNAME);
        assert_eq!(
            parsed.answers[0].data,
            DnsRecordData::CNAME(DnsName::from_ascii("alias.cern.ch").unwrap())
        );
    }

    #[test]
    fn parses_aaaa_answers() {
        let name = DnsName::from_ascii("example.com").unwrap();
        let message = DnsMessage {
            header: DnsHeader {
                id: 7,
                flags: DnsFlags {
                    is_response: true,
                    opcode: DnsOpcode::Query,
                    authoritative_answer: false,
                    truncated: false,
                    recursion_desired: true,
                    recursion_available: true,
                    response_code: DnsResponseCode::NoError,
                },
                question_count: 0,
                answer_count: 1,
                authority_count: 0,
                additional_count: 0,
            },
            questions: vec![],
            answers: vec![DnsResourceRecord {
                name,
                rrtype: DnsRecordType::AAAA,
                class: DnsClass::IN,
                ttl: 10,
                data: DnsRecordData::AAAA([
                    0x20, 0x01, 0x0d, 0xb8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1,
                ]),
            }],
            authorities: vec![],
            additionals: vec![],
        };

        let parsed = parse_dns_message(&serialize_dns_message(&message).unwrap()).unwrap();

        assert_eq!(parsed.ipv6_answers().len(), 1);
        assert_eq!(parsed.answers[0].ttl, 10);
    }

    #[test]
    fn preserves_unknown_record_data() {
        let record = DnsResourceRecord {
            name: DnsName::from_ascii("example.com").unwrap(),
            rrtype: DnsRecordType::Unknown(65),
            class: DnsClass::IN,
            ttl: 1,
            data: DnsRecordData::Raw(vec![1, 2, 3]),
        };
        let message = DnsMessage {
            header: DnsHeader {
                id: 1,
                flags: DnsFlags::query(),
                question_count: 0,
                answer_count: 1,
                authority_count: 0,
                additional_count: 0,
            },
            questions: vec![],
            answers: vec![record.clone()],
            authorities: vec![],
            additionals: vec![],
        };

        let parsed = parse_dns_message(&serialize_dns_message(&message).unwrap()).unwrap();

        assert_eq!(parsed.answers[0], record);
    }

    #[test]
    fn serializes_and_parses_authority_and_additional_sections() {
        let mut message = empty_message();
        message.header.flags = DnsFlags {
            is_response: true,
            opcode: DnsOpcode::Query,
            authoritative_answer: true,
            truncated: false,
            recursion_desired: true,
            recursion_available: true,
            response_code: DnsResponseCode::NoError,
        };
        message.authorities.push(DnsResourceRecord {
            name: DnsName::from_ascii("example.com").unwrap(),
            rrtype: DnsRecordType::NS,
            class: DnsClass::Unknown(3),
            ttl: 60,
            data: DnsRecordData::Raw(vec![2, b'n', b's', 0]),
        });
        message.additionals.push(DnsResourceRecord {
            name: DnsName::from_ascii("ns.example.com").unwrap(),
            rrtype: DnsRecordType::A,
            class: DnsClass::IN,
            ttl: 60,
            data: DnsRecordData::A([192, 0, 2, 53]),
        });

        let parsed = parse_dns_message(&serialize_dns_message(&message).unwrap()).unwrap();

        assert!(parsed.header.flags.authoritative_answer);
        assert_eq!(parsed.authorities.len(), 1);
        assert_eq!(parsed.additionals.len(), 1);
        assert_eq!(parsed.authorities[0].class, DnsClass::Unknown(3));
    }

    #[test]
    fn first_answer_of_type_finds_matching_record() {
        let mut message = empty_message();
        message.answers.push(DnsResourceRecord {
            name: DnsName::from_ascii("example.com").unwrap(),
            rrtype: DnsRecordType::A,
            class: DnsClass::IN,
            ttl: 1,
            data: DnsRecordData::A([203, 0, 113, 7]),
        });

        assert!(message.first_answer_of_type(DnsRecordType::A).is_some());
        assert!(message.first_answer_of_type(DnsRecordType::AAAA).is_none());
    }

    #[test]
    fn round_trips_common_question_record_types() {
        let types = [
            DnsRecordType::NS,
            DnsRecordType::CNAME,
            DnsRecordType::SOA,
            DnsRecordType::PTR,
            DnsRecordType::MX,
            DnsRecordType::TXT,
            DnsRecordType::AAAA,
            DnsRecordType::Unknown(65400),
        ];

        for qtype in types {
            let query = build_query(5, DnsName::from_ascii("example.com").unwrap(), qtype);
            let parsed = parse_dns_message(&serialize_dns_message(&query).unwrap()).unwrap();
            assert_eq!(parsed.questions[0].qtype, qtype);
        }
    }

    #[test]
    fn parses_unknown_opcode_and_response_code() {
        let bytes = [
            0x00, 0x01, 0xf0, 0x0f, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];

        let parsed = parse_dns_message(&bytes).unwrap();

        assert_eq!(parsed.header.flags.opcode, DnsOpcode::Unknown(14));
        assert_eq!(
            parsed.header.flags.response_code,
            DnsResponseCode::Unknown(15)
        );
    }

    #[test]
    fn serializes_unknown_opcode_and_all_response_codes() {
        for response_code in [
            DnsResponseCode::FormatError,
            DnsResponseCode::ServerFailure,
            DnsResponseCode::NameError,
            DnsResponseCode::NotImplemented,
            DnsResponseCode::Refused,
            DnsResponseCode::Unknown(9),
        ] {
            let mut message = empty_message();
            message.header.flags = DnsFlags {
                is_response: true,
                opcode: DnsOpcode::Unknown(2),
                authoritative_answer: true,
                truncated: true,
                recursion_desired: true,
                recursion_available: true,
                response_code,
            };

            let parsed = parse_dns_message(&serialize_dns_message(&message).unwrap()).unwrap();

            assert_eq!(parsed.header.flags.opcode, DnsOpcode::Unknown(2));
            assert_eq!(parsed.header.flags.response_code, response_code);
            assert!(parsed.header.flags.authoritative_answer);
            assert!(parsed.header.flags.truncated);
        }
    }

    #[test]
    fn parses_nxdomain_response_code() {
        let mut bytes = info_cern_query_bytes();
        bytes[2] = 0x81;
        bytes[3] = 0x83;

        let parsed = parse_dns_message(&bytes).unwrap();

        assert_eq!(
            parsed.header.flags.response_code,
            DnsResponseCode::NameError
        );
        assert!(!parsed.is_success());
    }

    #[test]
    fn exposes_truncated_flag_as_protocol_information() {
        let mut bytes = info_cern_query_bytes();
        bytes[2] = 0x82;
        bytes[3] = 0x80;

        let parsed = parse_dns_message(&bytes).unwrap();

        assert!(parsed.header.flags.truncated);
    }

    #[test]
    fn rejects_truncated_header() {
        assert_eq!(parse_dns_message(&[0; 11]), Err(DnsError::TruncatedHeader));
    }

    #[test]
    fn rejects_unexpected_eof_inside_question() {
        let mut bytes = info_cern_query_bytes();
        bytes.truncate(bytes.len() - 1);

        assert_eq!(parse_dns_message(&bytes), Err(DnsError::UnexpectedEof));
    }

    #[test]
    fn rejects_unexpected_eof_inside_record_header() {
        let mut bytes = info_cern_query_bytes();
        bytes[2] = 0x81;
        bytes[3] = 0x80;
        bytes[6] = 0x00;
        bytes[7] = 0x01;
        bytes.extend_from_slice(&[0xc0, 0x0c, 0x00, 0x01]);

        assert_eq!(parse_dns_message(&bytes), Err(DnsError::UnexpectedEof));
    }

    #[test]
    fn rejects_unexpected_eof_inside_rdata() {
        let mut bytes = info_cern_query_bytes();
        bytes[2] = 0x81;
        bytes[3] = 0x80;
        bytes[6] = 0x00;
        bytes[7] = 0x01;
        bytes.extend_from_slice(&[0xc0, 0x0c, 0x00, 0x01, 0x00, 0x01, 0, 0, 0, 1, 0, 4, 1, 2]);

        assert_eq!(parse_dns_message(&bytes), Err(DnsError::UnexpectedEof));
    }

    #[test]
    fn rejects_pointer_loop() {
        let bytes = vec![
            0x00, 0x01, 0x01, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xc0, 0x0c,
            0x00, 0x01, 0x00, 0x01,
        ];

        assert_eq!(parse_dns_message(&bytes), Err(DnsError::PointerLoop));
    }

    #[test]
    fn rejects_excessive_pointer_chain() {
        let mut bytes = vec![
            0x00, 0x01, 0x01, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];
        for hop in 0..=MAX_NAME_POINTER_HOPS {
            let target = DNS_HEADER_LEN + ((hop + 1) * 2);
            bytes.push(0xc0 | ((target >> 8) as u8 & 0x3f));
            bytes.push((target & 0xff) as u8);
        }

        assert_eq!(parse_dns_message(&bytes), Err(DnsError::PointerLoop));
    }

    #[test]
    fn rejects_pointer_out_of_bounds() {
        let bytes = vec![
            0x00, 0x01, 0x01, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xc0, 0xff,
            0x00, 0x01, 0x00, 0x01,
        ];

        assert_eq!(
            parse_dns_message(&bytes),
            Err(DnsError::PointerOutOfBounds { offset: 255 })
        );
    }

    #[test]
    fn rejects_pointer_missing_second_byte() {
        let bytes = vec![
            0x00, 0x01, 0x01, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xc0,
        ];

        assert_eq!(parse_dns_message(&bytes), Err(DnsError::UnexpectedEof));
    }

    #[test]
    fn rejects_non_ascii_names() {
        assert_eq!(
            DnsName::from_ascii("cafe\u{e9}.example"),
            Err(DnsError::NonAsciiLabel)
        );
    }

    #[test]
    fn rejects_non_ascii_wire_label() {
        let bytes = vec![
            0x00, 0x01, 0x01, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0xff,
            0x00, 0x00, 0x01, 0x00, 0x01,
        ];

        assert_eq!(parse_dns_message(&bytes), Err(DnsError::NonAsciiLabel));
    }

    #[test]
    fn rejects_reserved_wire_label_prefix() {
        let bytes = vec![
            0x00, 0x01, 0x01, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x40, 0x00,
            0x01, 0x00, 0x01,
        ];

        assert_eq!(
            parse_dns_message(&bytes),
            Err(DnsError::Unsupported("reserved DNS label prefix"))
        );
    }

    #[test]
    fn rejects_wire_name_longer_than_limit() {
        let label = vec![b'a'; 63];
        let mut bytes = vec![
            0x00, 0x01, 0x01, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];
        for _ in 0..4 {
            bytes.push(63);
            bytes.extend_from_slice(&label);
        }
        bytes.extend_from_slice(&[0x00, 0x00, 0x01, 0x00, 0x01]);

        assert_eq!(parse_dns_message(&bytes), Err(DnsError::NameTooLong));
    }

    #[test]
    fn rejects_labels_longer_than_sixty_three_octets() {
        let long = "a".repeat(64);

        assert_eq!(
            DnsName::from_ascii(&format!("{long}.example")),
            Err(DnsError::LabelTooLong { length: 64 })
        );
    }

    #[test]
    fn rejects_cname_record_with_trailing_rdata() {
        let mut bytes = info_cern_query_bytes();
        bytes[2] = 0x81;
        bytes[3] = 0x80;
        bytes[6] = 0x00;
        bytes[7] = 0x01;
        bytes.extend_from_slice(&[
            0xc0, 0x0c, 0x00, 0x05, 0x00, 0x01, 0, 0, 0, 1, 0, 3, 0xc0, 0x0c, 0xff,
        ]);

        assert_eq!(
            parse_dns_message(&bytes),
            Err(DnsError::Unsupported(
                "CNAME record data must contain exactly one DNS name"
            ))
        );
    }

    #[test]
    fn encodes_root_name() {
        let query = build_query(1, DnsName::from_ascii(".").unwrap(), DnsRecordType::NS);
        let bytes = serialize_dns_message(&query).unwrap();

        assert_eq!(bytes[12], 0);
    }

    #[test]
    fn rejects_wrong_a_record_length() {
        let mut bytes = info_cern_query_bytes();
        bytes[2] = 0x81;
        bytes[3] = 0x80;
        bytes[6] = 0x00;
        bytes[7] = 0x01;
        bytes.extend_from_slice(&[
            0xc0, 0x0c, 0x00, 0x01, 0x00, 0x01, 0, 0, 0, 1, 0, 3, 1, 2, 3,
        ]);

        assert_eq!(
            parse_dns_message(&bytes),
            Err(DnsError::Unsupported("A record data must be 4 bytes"))
        );
    }

    #[test]
    fn rejects_wrong_aaaa_record_length() {
        let mut bytes = info_cern_query_bytes();
        bytes[2] = 0x81;
        bytes[3] = 0x80;
        bytes[6] = 0x00;
        bytes[7] = 0x01;
        bytes.extend_from_slice(&[0xc0, 0x0c, 0x00, 0x1c, 0x00, 0x01, 0, 0, 0, 1, 0, 15]);
        bytes.extend_from_slice(&[0; 15]);

        assert_eq!(
            parse_dns_message(&bytes),
            Err(DnsError::Unsupported("AAAA record data must be 16 bytes"))
        );
    }

    #[test]
    fn rejects_too_many_questions_before_serializing() {
        let mut message = empty_message();
        let question = DnsQuestion {
            name: DnsName::from_ascii(".").unwrap(),
            qtype: DnsRecordType::A,
            qclass: DnsClass::IN,
        };
        message.questions = vec![question; u16::MAX as usize + 1];

        assert_eq!(
            serialize_dns_message(&message),
            Err(DnsError::InvalidSectionCount)
        );
    }

    #[test]
    fn rejects_huge_question_count_without_large_preallocation() {
        let bytes = vec![
            0x00, 0x01, 0x01, 0x00, 0xff, 0xff, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];

        assert_eq!(parse_dns_message(&bytes), Err(DnsError::UnexpectedEof));
    }

    #[test]
    fn rejects_record_data_larger_than_dns_length_field() {
        let mut message = empty_message();
        message.answers.push(DnsResourceRecord {
            name: DnsName::from_ascii(".").unwrap(),
            rrtype: DnsRecordType::Unknown(65000),
            class: DnsClass::IN,
            ttl: 0,
            data: DnsRecordData::Raw(vec![0; u16::MAX as usize + 1]),
        });

        assert_eq!(
            serialize_dns_message(&message),
            Err(DnsError::Unsupported("record data too large"))
        );
    }
}
