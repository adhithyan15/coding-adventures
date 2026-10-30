//! Exact borrowed TLS 1.3 `CertificateVerify` message-body framing.
//!
//! This crate owns only the two-byte signature-scheme field and the following
//! two-byte-length-prefixed signature vector from RFC 8446 section 4.4.3. It
//! does not interpret policy, construct signed content, or verify signatures.

#![forbid(unsafe_code)]

use core::fmt;

/// Two-byte algorithm field plus two-byte signature length.
pub const HEADER_LENGTH: usize = 4;
/// Largest signature vector permitted by the RFC's 16-bit length field.
pub const MAXIMUM_SIGNATURE_LENGTH: usize = u16::MAX as usize;
/// Largest complete `CertificateVerify` body accepted by this package.
pub const MAXIMUM_MESSAGE_LENGTH: usize = HEADER_LENGTH + MAXIMUM_SIGNATURE_LENGTH;

/// Stable, redacted message framing failures.
///
/// Errors retain no algorithm identifier, signature bytes, or caller lengths.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum MessageError {
    /// Fewer than four bytes were available for the fixed fields.
    TruncatedHeader,
    /// The declared signature length did not consume the exact remaining body.
    SignatureLengthMismatch,
    /// An outbound signature exceeded the 16-bit vector limit.
    SignatureTooLong,
    /// Caller-owned output could not hold the complete body.
    OutputTooShort,
}

impl fmt::Display for MessageError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TruncatedHeader => formatter.write_str("truncated CertificateVerify header"),
            Self::SignatureLengthMismatch => {
                formatter.write_str("CertificateVerify signature length mismatch")
            }
            Self::SignatureTooLong => formatter.write_str("CertificateVerify signature too long"),
            Self::OutputTooShort => formatter.write_str("output buffer is too short"),
        }
    }
}

impl std::error::Error for MessageError {}

/// One structurally valid borrowed TLS 1.3 `CertificateVerify` message body.
///
/// The algorithm remains uninterpreted wire data. Debug output deliberately
/// omits both it and the signature.
#[derive(Eq, PartialEq)]
pub struct CertificateVerifyMessage<'a> {
    algorithm_wire_id: u16,
    signature: &'a [u8],
}

impl<'a> CertificateVerifyMessage<'a> {
    /// Validate outbound fields without interpreting signature policy.
    pub fn new(algorithm_wire_id: u16, signature: &'a [u8]) -> Result<Self, MessageError> {
        if signature.len() > MAXIMUM_SIGNATURE_LENGTH {
            return Err(MessageError::SignatureTooLong);
        }
        Ok(Self {
            algorithm_wire_id,
            signature,
        })
    }

    /// Decode one exact message body, rejecting truncation and trailing bytes.
    pub fn decode(body: &'a [u8]) -> Result<Self, MessageError> {
        if body.len() < HEADER_LENGTH {
            return Err(MessageError::TruncatedHeader);
        }

        let algorithm_wire_id = u16::from_be_bytes([body[0], body[1]]);
        let declared_length = u16::from_be_bytes([body[2], body[3]]) as usize;
        if declared_length != body.len() - HEADER_LENGTH {
            return Err(MessageError::SignatureLengthMismatch);
        }

        Ok(Self {
            algorithm_wire_id,
            signature: &body[HEADER_LENGTH..],
        })
    }

    /// Return the uninterpreted signature-scheme wire identifier.
    #[must_use]
    pub const fn algorithm_wire_id(&self) -> u16 {
        self.algorithm_wire_id
    }

    /// Borrow the structurally framed signature bytes.
    #[must_use]
    pub const fn signature(&self) -> &[u8] {
        self.signature
    }

    /// Return the complete encoded body length.
    #[must_use]
    pub const fn encoded_length(&self) -> usize {
        HEADER_LENGTH + self.signature.len()
    }

    /// Encode this exact body into caller-owned output.
    ///
    /// Capacity is validated before `output` changes. Extra capacity is left
    /// untouched.
    pub fn encode<'output>(
        &self,
        output: &'output mut [u8],
    ) -> Result<&'output [u8], MessageError> {
        let encoded_length = self.encoded_length();
        if output.len() < encoded_length {
            return Err(MessageError::OutputTooShort);
        }

        output[..2].copy_from_slice(&self.algorithm_wire_id.to_be_bytes());
        output[2..4].copy_from_slice(&(self.signature.len() as u16).to_be_bytes());
        output[HEADER_LENGTH..encoded_length].copy_from_slice(self.signature);
        Ok(&output[..encoded_length])
    }
}

impl fmt::Debug for CertificateVerifyMessage<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CertificateVerifyMessage")
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_and_reencodes_exact_body() {
        let body = [0x08, 0x04, 0x00, 0x03, 0xaa, 0xbb, 0xcc];
        let message = CertificateVerifyMessage::decode(&body).unwrap();

        assert_eq!(message.algorithm_wire_id(), 0x0804);
        assert_eq!(message.signature(), &[0xaa, 0xbb, 0xcc]);
        assert_eq!(message.encoded_length(), body.len());

        let mut output = [0xa5; 10];
        assert_eq!(message.encode(&mut output).unwrap(), &body);
        assert_eq!(&output[body.len()..], &[0xa5; 3]);
    }

    #[test]
    fn accepts_zero_length_vector_as_syntax_only() {
        let body = [0x08, 0x04, 0x00, 0x00];
        let message = CertificateVerifyMessage::decode(&body).unwrap();
        assert!(message.signature().is_empty());

        let constructed = CertificateVerifyMessage::new(0x0804, &[]).unwrap();
        let mut output = [0xa5; 4];
        assert_eq!(constructed.encode(&mut output).unwrap(), &body);
    }

    #[test]
    fn accepts_maximum_signature_vector() {
        let signature = vec![0x42; MAXIMUM_SIGNATURE_LENGTH];
        let message = CertificateVerifyMessage::new(0x0806, &signature).unwrap();
        let mut output = vec![0u8; MAXIMUM_MESSAGE_LENGTH];

        let encoded = message.encode(&mut output).unwrap();
        assert_eq!(&encoded[..4], &[0x08, 0x06, 0xff, 0xff]);
        assert_eq!(&encoded[4..], signature);
        assert_eq!(CertificateVerifyMessage::decode(encoded), Ok(message));
    }

    #[test]
    fn rejects_every_truncated_header() {
        for length in 0..HEADER_LENGTH {
            assert_eq!(
                CertificateVerifyMessage::decode(&[0u8; HEADER_LENGTH][..length]),
                Err(MessageError::TruncatedHeader)
            );
        }
    }

    #[test]
    fn rejects_short_long_and_trailing_signature_lengths() {
        for body in [
            &[0x08, 0x04, 0x00, 0x02, 0xaa][..],
            &[0x08, 0x04, 0x00, 0x01, 0xaa, 0xbb][..],
            &[0x08, 0x04, 0xff, 0xff][..],
        ] {
            assert_eq!(
                CertificateVerifyMessage::decode(body),
                Err(MessageError::SignatureLengthMismatch)
            );
        }
    }

    #[test]
    fn rejects_oversized_signature() {
        let signature = vec![0x42; MAXIMUM_SIGNATURE_LENGTH + 1];
        assert_eq!(
            CertificateVerifyMessage::new(0x0804, &signature),
            Err(MessageError::SignatureTooLong)
        );
    }

    #[test]
    fn short_output_is_failure_atomic() {
        let message = CertificateVerifyMessage::new(0x0804, &[0xaa, 0xbb]).unwrap();
        let mut output = [0xa5; 5];
        assert_eq!(
            message.encode(&mut output),
            Err(MessageError::OutputTooShort)
        );
        assert_eq!(output, [0xa5; 5]);
    }

    #[test]
    fn diagnostics_are_stable_and_redacted() {
        let message = CertificateVerifyMessage::new(0x0804, &[0xaa, 0xbb]).unwrap();
        assert_eq!(format!("{message:?}"), "CertificateVerifyMessage { .. }");
        assert_eq!(
            MessageError::TruncatedHeader.to_string(),
            "truncated CertificateVerify header"
        );
        assert_eq!(
            MessageError::SignatureLengthMismatch.to_string(),
            "CertificateVerify signature length mismatch"
        );
        assert_eq!(
            MessageError::SignatureTooLong.to_string(),
            "CertificateVerify signature too long"
        );
        assert_eq!(
            MessageError::OutputTooShort.to_string(),
            "output buffer is too short"
        );
    }
}
