//! Reviewed Mozilla public-Web-PKI trust-anchor snapshot data.
//!
//! This crate parses only its repository-pinned canonical anchor snapshot. It
//! is not a certificate parser, path validator, signature verifier, trust
//! decision, TLS stack, or network transport.

#![deny(unsafe_code)]

use std::error::Error;
use std::fmt;

const MAGIC: &[u8] = b"webpki-roots-canonical-v1\0";
const SNAPSHOT_BYTES: &[u8] = include_bytes!("../data/webpki-roots-1.0.9.bin");
pub const SNAPSHOT_METADATA_JSON: &str = include_str!("../snapshot.json");

pub const UPSTREAM_RELEASE: &str = "1.0.9";
pub const UPSTREAM_TAG: &str = "v/1.0.9";
pub const UPSTREAM_COMMIT: &str = "0a553dbc8b3f18ea05c4f881cffa3f2d005d0d30";
pub const CRATES_IO_ARCHIVE_SHA256: &str =
    "7dcd9d09a39985f5344844e66b0c530a33843579125f23e21e9f0f220850f22a";
pub const REVIEWED_ROOT_COUNT: usize = 121;
pub const ROOT_SET_SHA256: &str =
    "0aa729fbb339061e8aad2e49fefd83674cd6b2e45eeb586c0d456cf33b70dc01";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TrustAnchor<'a> {
    subject: &'a [u8],
    subject_public_key_info: &'a [u8],
    name_constraints: Option<&'a [u8]>,
}

impl<'a> TrustAnchor<'a> {
    pub const fn subject(self) -> &'a [u8] {
        self.subject
    }

    pub const fn subject_public_key_info(self) -> &'a [u8] {
        self.subject_public_key_info
    }

    pub const fn name_constraints(self) -> Option<&'a [u8]> {
        self.name_constraints
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrustAnchorSnapshotErrorKind {
    InvalidMagic,
    Truncated,
    LengthOverflow,
    EmptySubject,
    EmptySubjectPublicKeyInfo,
    InvalidNameConstraintsFlag,
    EmptyNameConstraints,
    RootCountMismatch,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TrustAnchorSnapshotError {
    kind: TrustAnchorSnapshotErrorKind,
    offset: usize,
}

impl TrustAnchorSnapshotError {
    const fn new(kind: TrustAnchorSnapshotErrorKind, offset: usize) -> Self {
        Self { kind, offset }
    }

    pub const fn kind(self) -> TrustAnchorSnapshotErrorKind {
        self.kind
    }

    pub const fn offset(self) -> usize {
        self.offset
    }
}

impl fmt::Display for TrustAnchorSnapshotError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "trust-anchor snapshot error {:?} at byte {}",
            self.kind, self.offset
        )
    }
}

impl Error for TrustAnchorSnapshotError {}

#[derive(Clone, Copy, Debug)]
pub struct TrustAnchorSnapshot<'a> {
    encoded: &'a [u8],
    expected_count: usize,
}

impl<'a> TrustAnchorSnapshot<'a> {
    /// Parse a canonical snapshot with an independently reviewed root count.
    ///
    /// This constructor parses data; it does not grant trust to caller input.
    pub const fn from_bytes(encoded: &'a [u8], expected_count: usize) -> Self {
        Self {
            encoded,
            expected_count,
        }
    }

    pub const fn encoded(self) -> &'a [u8] {
        self.encoded
    }

    pub const fn expected_count(self) -> usize {
        self.expected_count
    }

    pub fn iter(self) -> TrustAnchorIter<'a> {
        TrustAnchorIter::new(self.encoded)
    }

    pub fn validate(self) -> Result<(), TrustAnchorSnapshotError> {
        let mut count = 0usize;
        for anchor in self.iter() {
            anchor?;
            // Every entry consumes at least 17 bytes, so a slice cannot hold
            // enough entries to overflow usize.
            count += 1;
        }
        if count != self.expected_count {
            return Err(TrustAnchorSnapshotError::new(
                TrustAnchorSnapshotErrorKind::RootCountMismatch,
                self.encoded.len(),
            ));
        }
        Ok(())
    }
}

/// Return the exact reviewed `webpki-roots` 1.0.9 snapshot.
pub const fn reviewed_webpki_roots() -> TrustAnchorSnapshot<'static> {
    TrustAnchorSnapshot::from_bytes(SNAPSHOT_BYTES, REVIEWED_ROOT_COUNT)
}

pub struct TrustAnchorIter<'a> {
    encoded: &'a [u8],
    offset: usize,
    failed: bool,
}

impl<'a> TrustAnchorIter<'a> {
    fn new(encoded: &'a [u8]) -> Self {
        let valid_magic = encoded.get(..MAGIC.len()) == Some(MAGIC);
        Self {
            encoded,
            offset: if valid_magic { MAGIC.len() } else { 0 },
            failed: !valid_magic,
        }
    }

    fn fail(
        &mut self,
        kind: TrustAnchorSnapshotErrorKind,
        offset: usize,
    ) -> Option<Result<TrustAnchor<'a>, TrustAnchorSnapshotError>> {
        self.failed = true;
        Some(Err(TrustAnchorSnapshotError::new(kind, offset)))
    }

    fn take_u64(&mut self) -> Result<u64, TrustAnchorSnapshotError> {
        let offset = self.offset;
        // offset is always within encoded, whose length cannot exceed isize::MAX.
        let end = offset + 8;
        let bytes = self.encoded.get(offset..end).ok_or_else(|| {
            TrustAnchorSnapshotError::new(TrustAnchorSnapshotErrorKind::Truncated, offset)
        })?;
        self.offset = end;
        Ok(u64::from_be_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ]))
    }

    fn take_bytes(&mut self, length: u64) -> Result<&'a [u8], TrustAnchorSnapshotError> {
        let length = usize::try_from(length).map_err(|_| {
            TrustAnchorSnapshotError::new(TrustAnchorSnapshotErrorKind::LengthOverflow, self.offset)
        })?;
        let offset = self.offset;
        let end = offset.checked_add(length).ok_or_else(|| {
            TrustAnchorSnapshotError::new(TrustAnchorSnapshotErrorKind::LengthOverflow, offset)
        })?;
        let bytes = self.encoded.get(offset..end).ok_or_else(|| {
            TrustAnchorSnapshotError::new(TrustAnchorSnapshotErrorKind::Truncated, offset)
        })?;
        self.offset = end;
        Ok(bytes)
    }

    fn parse_anchor(&mut self) -> Result<TrustAnchor<'a>, TrustAnchorSnapshotError> {
        let subject_offset = self.offset;
        let subject_length = self.take_u64()?;
        let subject = self.take_bytes(subject_length)?;
        if subject.is_empty() {
            return Err(TrustAnchorSnapshotError::new(
                TrustAnchorSnapshotErrorKind::EmptySubject,
                subject_offset,
            ));
        }

        let key_offset = self.offset;
        let key_length = self.take_u64()?;
        let subject_public_key_info = self.take_bytes(key_length)?;
        if subject_public_key_info.is_empty() {
            return Err(TrustAnchorSnapshotError::new(
                TrustAnchorSnapshotErrorKind::EmptySubjectPublicKeyInfo,
                key_offset,
            ));
        }

        let flag_offset = self.offset;
        let flag = *self.encoded.get(flag_offset).ok_or_else(|| {
            TrustAnchorSnapshotError::new(TrustAnchorSnapshotErrorKind::Truncated, flag_offset)
        })?;
        self.offset = flag_offset + 1;
        let name_constraints = match flag {
            0 => None,
            1 => {
                let constraints_offset = self.offset;
                let constraints_length = self.take_u64()?;
                let constraints = self.take_bytes(constraints_length)?;
                if constraints.is_empty() {
                    return Err(TrustAnchorSnapshotError::new(
                        TrustAnchorSnapshotErrorKind::EmptyNameConstraints,
                        constraints_offset,
                    ));
                }
                Some(constraints)
            }
            _ => {
                return Err(TrustAnchorSnapshotError::new(
                    TrustAnchorSnapshotErrorKind::InvalidNameConstraintsFlag,
                    flag_offset,
                ));
            }
        };

        Ok(TrustAnchor {
            subject,
            subject_public_key_info,
            name_constraints,
        })
    }
}

impl<'a> Iterator for TrustAnchorIter<'a> {
    type Item = Result<TrustAnchor<'a>, TrustAnchorSnapshotError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.failed {
            if self.offset == 0 {
                self.offset = 1;
                return self.fail(TrustAnchorSnapshotErrorKind::InvalidMagic, 0);
            }
            return None;
        }
        if self.offset == self.encoded.len() {
            return None;
        }
        match self.parse_anchor() {
            Ok(anchor) => Some(Ok(anchor)),
            Err(error) => {
                self.failed = true;
                Some(Err(error))
            }
        }
    }
}
