//! # Bounded TLS 1.3 cookie framing and retry echo evidence
//!
//! RFC 9846 section 4.3.2 defines one deliberately small structure:
//!
//! ```text
//! struct {
//!     opaque cookie<1..2^16-1>;
//! } Cookie;
//! ```
//!
//! The two-byte length belongs to the extension data. This crate validates
//! that exact shape without allocation, preserves the complete borrowed wire
//! value, and can encode it into caller-owned storage. A second boundary binds
//! the cookie received in `HelloRetryRequest` to the byte-for-byte copy in the
//! retried `ClientHello`.
//!
//! Cookie bytes can carry authenticated server state, even though this client
//! treats them as opaque. Debug and error values therefore disclose neither
//! those bytes nor their lengths. This crate does not authenticate cookie
//! contents or decide whether extension type 44 is legal in a message.

#![forbid(unsafe_code)]

use core::fmt;

/// Bytes occupied by the cookie vector's two-byte length prefix.
pub const COOKIE_LENGTH_PREFIX: usize = 2;

/// Largest cookie payload representable by RFC 9846's `opaque<1..2^16-1>`.
pub const MAX_COOKIE_LENGTH: usize = u16::MAX as usize;

/// Caller-selected public work and buffering bound for one cookie.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CookieLimits {
    maximum_cookie_length: u16,
}

impl CookieLimits {
    /// Validate a non-zero ceiling representable by the wire format.
    pub fn new(maximum_cookie_length: usize) -> Result<Self, CookieError> {
        let maximum_cookie_length = u16::try_from(maximum_cookie_length)
            .map_err(|_| CookieError::InvalidMaximumCookieLength)?;
        if maximum_cookie_length == 0 {
            return Err(CookieError::InvalidMaximumCookieLength);
        }
        Ok(Self {
            maximum_cookie_length,
        })
    }

    /// Return the validated cookie payload ceiling.
    #[must_use]
    pub const fn maximum_cookie_length(self) -> usize {
        self.maximum_cookie_length as usize
    }
}

/// Stable cookie failures retaining no cookie bytes, lengths, or offsets.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CookieError {
    /// The configured maximum was zero or exceeded the wire maximum.
    InvalidMaximumCookieLength,
    /// Extension data did not contain a complete two-byte length.
    TruncatedLength,
    /// The declared or caller-supplied cookie payload was empty.
    EmptyCookie,
    /// The declared or caller-supplied payload exceeded the selected limit.
    CookieTooLong,
    /// The declared payload did not consume the complete extension data.
    LengthMismatch,
    /// Caller-owned output could not hold the complete encoded value.
    OutputTooSmall,
    /// The second ClientHello did not copy the exact retry cookie.
    EchoMismatch,
}

impl fmt::Display for CookieError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::InvalidMaximumCookieLength => "invalid maximum TLS cookie length",
            Self::TruncatedLength => "truncated TLS cookie length",
            Self::EmptyCookie => "empty TLS cookie",
            Self::CookieTooLong => "TLS cookie exceeds the configured limit",
            Self::LengthMismatch => "TLS cookie length mismatch",
            Self::OutputTooSmall => "output is too small for the TLS cookie",
            Self::EchoMismatch => "retried ClientHello changed the TLS cookie",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for CookieError {}

/// One exact borrowed TLS 1.3 `Cookie` extension-data value.
///
/// The value intentionally derives neither [`Copy`] nor [`Clone`]. Later
/// retry policy can therefore retain this exact parse instead of silently
/// substituting another byte slice.
#[derive(Eq, PartialEq)]
pub struct Tls13Cookie<'input> {
    extension_data: &'input [u8],
    cookie: &'input [u8],
}

impl fmt::Debug for Tls13Cookie<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("Tls13Cookie")
    }
}

impl<'input> Tls13Cookie<'input> {
    /// Borrow the complete canonical extension data, including its length.
    #[must_use]
    pub const fn extension_data(&self) -> &'input [u8] {
        self.extension_data
    }

    /// Borrow the exact opaque cookie payload.
    #[must_use]
    pub const fn cookie(&self) -> &'input [u8] {
        self.cookie
    }

    /// Return the validated payload length.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.cookie.len()
    }

    /// Return whether the payload is empty.
    ///
    /// A successfully constructed value is never empty.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.cookie.is_empty()
    }
}

/// Decode one complete RFC 9846 `Cookie` extension-data value.
///
/// The declared length is checked against the caller's limit before payload
/// availability. A peer cannot make the caller buffer an over-policy cookie
/// merely by declaring a large vector and sending no payload.
pub fn decode_cookie(
    extension_data: &[u8],
    limits: CookieLimits,
) -> Result<Tls13Cookie<'_>, CookieError> {
    let [length_high, length_low, cookie @ ..] = extension_data else {
        return Err(CookieError::TruncatedLength);
    };
    let declared_length = usize::from(u16::from_be_bytes([*length_high, *length_low]));
    validate_cookie_length(declared_length, limits)?;
    if cookie.len() != declared_length {
        return Err(CookieError::LengthMismatch);
    }

    Ok(Tls13Cookie {
        extension_data,
        cookie,
    })
}

/// Encode one opaque cookie into caller-owned output.
///
/// All length and capacity checks finish before the first output byte changes.
/// Bytes after the returned exact extension-data value remain untouched.
pub fn encode_cookie<'output>(
    cookie: &[u8],
    limits: CookieLimits,
    output: &'output mut [u8],
) -> Result<Tls13Cookie<'output>, CookieError> {
    validate_cookie_length(cookie.len(), limits)?;
    let encoded_length = COOKIE_LENGTH_PREFIX + cookie.len();
    if output.len() < encoded_length {
        return Err(CookieError::OutputTooSmall);
    }

    output[..COOKIE_LENGTH_PREFIX].copy_from_slice(&(cookie.len() as u16).to_be_bytes());
    output[COOKIE_LENGTH_PREFIX..encoded_length].copy_from_slice(cookie);
    let extension_data = &output[..encoded_length];
    Ok(Tls13Cookie {
        extension_data,
        cookie: &extension_data[COOKIE_LENGTH_PREFIX..],
    })
}

/// Exact retry-cookie echo evidence retaining both non-copyable parses.
pub struct EchoedCookie<'retry, 'client> {
    hello_retry_request: Tls13Cookie<'retry>,
    client_hello: Tls13Cookie<'client>,
}

impl fmt::Debug for EchoedCookie<'_, '_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("EchoedCookie")
    }
}

impl<'retry, 'client> EchoedCookie<'retry, 'client> {
    /// Borrow the exact cookie parsed from `HelloRetryRequest`.
    #[must_use]
    pub const fn hello_retry_request(&self) -> &Tls13Cookie<'retry> {
        &self.hello_retry_request
    }

    /// Borrow the exact cookie parsed from the retried `ClientHello`.
    #[must_use]
    pub const fn client_hello(&self) -> &Tls13Cookie<'client> {
        &self.client_hello
    }
}

/// Bind one retry cookie to its exact second-ClientHello echo.
///
/// Complete canonical extension data is compared, rather than a reconstructed
/// payload. Success consumes and retains both typed values.
pub fn bind_cookie_echo<'retry, 'client>(
    hello_retry_request: Tls13Cookie<'retry>,
    client_hello: Tls13Cookie<'client>,
) -> Result<EchoedCookie<'retry, 'client>, CookieError> {
    if hello_retry_request.extension_data != client_hello.extension_data {
        return Err(CookieError::EchoMismatch);
    }
    Ok(EchoedCookie {
        hello_retry_request,
        client_hello,
    })
}

fn validate_cookie_length(length: usize, limits: CookieLimits) -> Result<(), CookieError> {
    if length == 0 {
        return Err(CookieError::EmptyCookie);
    }
    if length > limits.maximum_cookie_length() {
        return Err(CookieError::CookieTooLong);
    }
    Ok(())
}
