//! Bounded allocation-free HTTP/1 request Host authority encoding.
//!
//! This crate validates a caller-borrowed ASCII DNS name and optional non-zero
//! port, then writes one exact lowercase Host field value into caller storage.

#![forbid(unsafe_code)]

use core::fmt;

/// Largest DNS name admitted by this deliberately narrow boundary.
pub const MAX_HTTP1_REQUEST_HOST_NAME_BYTES: usize = 253;

/// Largest encoded authority: DNS name, colon, and five decimal port digits.
pub const MAX_HTTP1_REQUEST_HOST_AUTHORITY_BYTES: usize = 259;

/// Caller-owned resource limits for one Host field value.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Http1RequestHostAuthorityLimits {
    maximum_host_bytes: u8,
    maximum_authority_bytes: u16,
}

impl Http1RequestHostAuthorityLimits {
    /// Validate hostname and complete-authority ceilings.
    pub fn new(
        maximum_host_bytes: usize,
        maximum_authority_bytes: usize,
    ) -> Result<Self, Http1RequestHostAuthorityError> {
        if maximum_host_bytes == 0
            || maximum_host_bytes > MAX_HTTP1_REQUEST_HOST_NAME_BYTES
            || maximum_authority_bytes == 0
            || maximum_authority_bytes > MAX_HTTP1_REQUEST_HOST_AUTHORITY_BYTES
        {
            return Err(Http1RequestHostAuthorityError::InvalidLimits);
        }
        Ok(Self {
            maximum_host_bytes: maximum_host_bytes as u8,
            maximum_authority_bytes: maximum_authority_bytes as u16,
        })
    }

    #[must_use]
    pub const fn maximum_host_bytes(self) -> usize {
        self.maximum_host_bytes as usize
    }

    #[must_use]
    pub const fn maximum_authority_bytes(self) -> usize {
        self.maximum_authority_bytes as usize
    }
}

/// One validated borrowed DNS authority.
pub struct Http1RequestHostAuthority<'input> {
    host: &'input [u8],
    port: Option<u16>,
    encoded_length: u16,
}

impl<'input> Http1RequestHostAuthority<'input> {
    #[must_use]
    pub const fn host(&self) -> &'input [u8] {
        self.host
    }

    #[must_use]
    pub const fn port(&self) -> Option<u16> {
        self.port
    }

    #[must_use]
    pub const fn encoded_length(&self) -> usize {
        self.encoded_length as usize
    }
}

impl fmt::Debug for Http1RequestHostAuthority<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Http1RequestHostAuthority")
            .field("host", &"[REDACTED]")
            .field("host_length", &self.host.len())
            .field("port", &self.port)
            .field("encoded_length", &self.encoded_length)
            .finish()
    }
}

/// Stable payload-free Host authority failures.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Http1RequestHostAuthorityError {
    InvalidLimits,
    EmptyHost,
    HostTooLong,
    EmptyLabel,
    LabelTooLong,
    InvalidLabel,
    IpLiteral,
    ZeroPort,
    AuthorityTooLong,
    OutputTooSmall,
}

impl fmt::Display for Http1RequestHostAuthorityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::InvalidLimits => "invalid HTTP/1 request Host authority limits",
            Self::EmptyHost => "HTTP/1 request Host DNS name is empty",
            Self::HostTooLong => "HTTP/1 request Host DNS name exceeds its byte limit",
            Self::EmptyLabel => "HTTP/1 request Host DNS name contains an empty label",
            Self::LabelTooLong => "HTTP/1 request Host DNS label exceeds 63 bytes",
            Self::InvalidLabel => "invalid HTTP/1 request Host DNS label",
            Self::IpLiteral => "HTTP/1 request Host authority is an IP literal",
            Self::ZeroPort => "HTTP/1 request Host authority port is zero",
            Self::AuthorityTooLong => "HTTP/1 request Host authority exceeds its byte limit",
            Self::OutputTooSmall => "HTTP/1 request Host authority output is too small",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for Http1RequestHostAuthorityError {}

/// Validate one borrowed ASCII DNS name and optional explicit port.
pub fn validate_http1_request_host_authority<'input>(
    host: &'input [u8],
    port: Option<u16>,
    limits: Http1RequestHostAuthorityLimits,
) -> Result<Http1RequestHostAuthority<'input>, Http1RequestHostAuthorityError> {
    if host.is_empty() {
        return Err(Http1RequestHostAuthorityError::EmptyHost);
    }
    if host.len() > limits.maximum_host_bytes() {
        return Err(Http1RequestHostAuthorityError::HostTooLong);
    }
    validate_dns_name(host)?;
    if is_numeric_address_literal(host) {
        return Err(Http1RequestHostAuthorityError::IpLiteral);
    }
    if port == Some(0) {
        return Err(Http1RequestHostAuthorityError::ZeroPort);
    }

    let encoded_length = host
        .len()
        .checked_add(port.map_or(0, |port| 1 + decimal_length(port)))
        .ok_or(Http1RequestHostAuthorityError::AuthorityTooLong)?;
    if encoded_length > limits.maximum_authority_bytes() {
        return Err(Http1RequestHostAuthorityError::AuthorityTooLong);
    }

    Ok(Http1RequestHostAuthority {
        host,
        port,
        encoded_length: encoded_length as u16,
    })
}

fn is_numeric_address_literal(host: &[u8]) -> bool {
    let mut component_count = 0usize;
    for component in host.split(|byte| *byte == b'.') {
        component_count += 1;
        if component_count > 4 || !is_numeric_address_component(component) {
            return false;
        }
    }
    component_count != 0
}

fn is_numeric_address_component(component: &[u8]) -> bool {
    if let Some(hexadecimal) = component
        .strip_prefix(b"0x")
        .or_else(|| component.strip_prefix(b"0X"))
    {
        !hexadecimal.is_empty() && hexadecimal.iter().all(u8::is_ascii_hexdigit)
    } else {
        !component.is_empty() && component.iter().all(u8::is_ascii_digit)
    }
}

/// Encode one validated authority into caller-owned storage.
///
/// The output remains unchanged when its capacity is insufficient.
pub fn encode_http1_request_host_authority<'output>(
    authority: &Http1RequestHostAuthority<'_>,
    output: &'output mut [u8],
) -> Result<&'output mut [u8], Http1RequestHostAuthorityError> {
    let encoded_length = authority.encoded_length();
    if output.len() < encoded_length {
        return Err(Http1RequestHostAuthorityError::OutputTooSmall);
    }

    for (destination, source) in output[..authority.host.len()]
        .iter_mut()
        .zip(authority.host.iter().copied())
    {
        *destination = source.to_ascii_lowercase();
    }
    let mut offset = authority.host.len();
    if let Some(port) = authority.port {
        output[offset] = b':';
        offset += 1;
        write_decimal(port, &mut output[offset..encoded_length]);
    }
    Ok(&mut output[..encoded_length])
}

fn validate_dns_name(host: &[u8]) -> Result<(), Http1RequestHostAuthorityError> {
    for label in host.split(|byte| *byte == b'.') {
        if label.is_empty() {
            return Err(Http1RequestHostAuthorityError::EmptyLabel);
        }
        if label.len() > 63 {
            return Err(Http1RequestHostAuthorityError::LabelTooLong);
        }
        if !label[0].is_ascii_alphanumeric()
            || !label[label.len() - 1].is_ascii_alphanumeric()
            || !label
                .iter()
                .copied()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
        {
            return Err(Http1RequestHostAuthorityError::InvalidLabel);
        }
    }
    Ok(())
}

const fn decimal_length(port: u16) -> usize {
    match port {
        0..=9 => 1,
        10..=99 => 2,
        100..=999 => 3,
        1_000..=9_999 => 4,
        _ => 5,
    }
}

fn write_decimal(mut port: u16, output: &mut [u8]) {
    for digit in output.iter_mut().rev() {
        *digit = b'0' + (port % 10) as u8;
        port /= 10;
    }
    debug_assert_eq!(port, 0);
}
