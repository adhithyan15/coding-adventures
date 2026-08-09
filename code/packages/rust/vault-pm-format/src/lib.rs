//! Canonical V1 repository formats for the local-first password manager.
//!
//! This crate is deliberately a pure byte-contract layer. It encodes and
//! validates structured values, builds domain-separated signing preimages, and
//! derives content IDs. Key derivation, encryption, signatures, storage, and
//! policy belong to higher layers.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use coding_adventures_canonical_cbor::{decode, encode, CborError, CborValue};
use coding_adventures_sha256::sha256;
use std::collections::BTreeMap;

/// The only structured format version understood by this crate.
pub const FORMAT_VERSION_V1: u64 = 1;
/// V1 crypto-suite registry value.
pub const CRYPTO_SUITE_V1: u16 = 1;
/// Maximum repository payload ciphertext accepted in one object frame.
pub const MAX_OBJECT_CIPHERTEXT: usize = 64 * 1024 * 1024;
/// Maximum ciphertext accepted by a generic structured AEAD envelope.
pub const MAX_ENVELOPE_CIPHERTEXT: usize = 64 * 1024;
/// Maximum recovery wraps in one bootstrap.
pub const MAX_RECOVERY_WRAPS: usize = 16;
/// Maximum parent commits in one commit.
pub const MAX_COMMIT_PARENTS: usize = 32;
/// Maximum newly reachable objects declared by one commit.
pub const MAX_ADDED_OBJECTS: usize = 4096;
/// Maximum device capability codes in one certificate.
pub const MAX_DEVICE_CAPABILITIES: usize = 64;

const MAX_STRUCTURED_BYTES: usize = 1024 * 1024;
const OBJECT_MAGIC: &[u8; 4] = b"VPO1";
const OBJECT_FRAME_FIXED_LEN: usize = 126;

const BOOTSTRAP_SIGN_DOMAIN: &[u8] = b"VPM-BOOTSTRAP-SIGN-v1";
const DEVICE_CERT_SIGN_DOMAIN: &[u8] = b"VPM-DEVICE-CERT-SIGN-v1";
const COMMIT_SIGN_DOMAIN: &[u8] = b"VPM-COMMIT-SIGN-v1";
const ANNOUNCEMENT_SIGN_DOMAIN: &[u8] = b"VPM-ANNOUNCEMENT-SIGN-v1";
const BOOTSTRAP_ID_DOMAIN: &[u8] = b"VPM-BOOTSTRAP-ID-v1";
const OBJECT_ID_DOMAIN: &[u8] = b"VPM-OBJECT-ID-v1";

/// Errors produced by strict V1 parsing and validation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FormatError {
    /// The underlying bytes are not canonical CBOR.
    Cbor(CborError),
    /// A structured input exceeds the global parser bound.
    InputTooLarge,
    /// A closed-schema value has missing, extra, or incorrectly typed fields.
    Schema(&'static str),
    /// The format version is not supported.
    UnsupportedVersion,
    /// The cryptographic suite is not supported.
    UnsupportedSuite,
    /// A named count, length, integer, or KDF field exceeds its V1 bound.
    Bound(&'static str),
    /// A set-like array is not strictly bytewise sorted and unique.
    Ordering(&'static str),
    /// Bootstrap generation and previous-ID fields disagree.
    InvalidGeneration,
    /// A device counter is zero.
    InvalidCounter,
    /// An object frame does not start with the V1 magic.
    InvalidMagic,
    /// A binary object frame ends before all declared fields are present.
    Truncated,
    /// A binary object frame has bytes after the declared payload tag.
    TrailingBytes,
    /// Length arithmetic or conversion overflowed.
    LengthOverflow,
}

impl core::fmt::Display for FormatError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let message = match self {
            Self::Cbor(_) => "vault-pm-format: invalid canonical CBOR",
            Self::InputTooLarge => "vault-pm-format: structured input too large",
            Self::Schema(_) => "vault-pm-format: schema mismatch",
            Self::UnsupportedVersion => "vault-pm-format: unsupported version",
            Self::UnsupportedSuite => "vault-pm-format: unsupported crypto suite",
            Self::Bound(_) => "vault-pm-format: field exceeds V1 bound",
            Self::Ordering(_) => "vault-pm-format: set field is not sorted and unique",
            Self::InvalidGeneration => "vault-pm-format: invalid bootstrap generation link",
            Self::InvalidCounter => "vault-pm-format: device counter must be non-zero",
            Self::InvalidMagic => "vault-pm-format: invalid object magic",
            Self::Truncated => "vault-pm-format: truncated object frame",
            Self::TrailingBytes => "vault-pm-format: trailing object-frame bytes",
            Self::LengthOverflow => "vault-pm-format: object length overflow",
        };
        f.write_str(message)
    }
}

impl std::error::Error for FormatError {}

impl From<CborError> for FormatError {
    fn from(value: CborError) -> Self {
        Self::Cbor(value)
    }
}

macro_rules! fixed_bytes_type {
    ($name:ident, $size:expr, $docs:literal) => {
        #[doc = $docs]
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name([u8; $size]);

        impl $name {
            /// Construct from exact bytes.
            pub const fn new(bytes: [u8; $size]) -> Self {
                Self(bytes)
            }

            /// Borrow the exact bytes.
            pub const fn as_bytes(&self) -> &[u8; $size] {
                &self.0
            }

            /// Consume and return the exact bytes.
            pub const fn into_bytes(self) -> [u8; $size] {
                self.0
            }
        }
    };
}

fixed_bytes_type!(VaultId, 16, "Random 128-bit vault identifier.");
fixed_bytes_type!(DeviceId, 16, "Random 128-bit certified device identifier.");
fixed_bytes_type!(
    ObjectId,
    32,
    "Domain-separated identifier of an encrypted object frame."
);
fixed_bytes_type!(
    BootstrapId,
    32,
    "Domain-separated identifier of a signed bootstrap."
);
fixed_bytes_type!(PublicKey, 32, "Raw 32-byte Ed25519 or X25519 public key.");
fixed_bytes_type!(Signature, 64, "Raw 64-byte Ed25519 signature.");
fixed_bytes_type!(RecipientId, 16, "Opaque recovery-recipient identifier.");

impl ObjectId {
    /// Hash a complete encoded object frame with the V1 object-ID domain.
    pub fn for_frame(frame: &[u8]) -> Self {
        Self(domain_hash(OBJECT_ID_DOMAIN, frame))
    }
}

/// Bounded detached AEAD envelope used inside structured values.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AeadEnvelopeV1 {
    /// Registry suite; V1 accepts only [`CRYPTO_SUITE_V1`].
    pub suite: u16,
    /// XChaCha20-Poly1305 nonce.
    pub nonce: [u8; 24],
    /// Encrypted body, bounded by [`MAX_ENVELOPE_CIPHERTEXT`].
    pub ciphertext: Vec<u8>,
    /// Detached Poly1305 tag.
    pub tag: [u8; 16],
}

impl AeadEnvelopeV1 {
    /// Validate the suite and ciphertext length.
    pub fn validate(&self) -> Result<(), FormatError> {
        check_suite(self.suite)?;
        if self.ciphertext.len() > MAX_ENVELOPE_CIPHERTEXT {
            return Err(FormatError::Bound("envelope ciphertext"));
        }
        Ok(())
    }
}

/// Persisted and parser-bounded Argon2id parameters.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Argon2idParametersV1 {
    /// Memory cost in KiB, bounded to 8 MiB through 1 GiB.
    pub memory_kib: u32,
    /// Iteration count, bounded to 1 through 32.
    pub iterations: u32,
    /// Parallel lanes, bounded to 1 through 16.
    pub lanes: u8,
    /// Random KDF salt.
    pub salt: [u8; 16],
}

impl Argon2idParametersV1 {
    /// Validate all defensive parser bounds.
    pub fn validate(&self) -> Result<(), FormatError> {
        if !(8 * 1024..=1024 * 1024).contains(&self.memory_kib) {
            return Err(FormatError::Bound("argon2 memory"));
        }
        if !(1..=32).contains(&self.iterations) {
            return Err(FormatError::Bound("argon2 iterations"));
        }
        if !(1..=16).contains(&self.lanes) {
            return Err(FormatError::Bound("argon2 lanes"));
        }
        Ok(())
    }
}

/// One opaque recovery recipient wrap in a bootstrap.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecoveryWrapV1 {
    /// Non-zero recovery-mechanism registry value.
    pub kind: u16,
    /// Opaque recipient identifier.
    pub recipient_id: RecipientId,
    /// Wrapped vault root key.
    pub root_wrap: AeadEnvelopeV1,
}

impl RecoveryWrapV1 {
    fn validate(&self) -> Result<(), FormatError> {
        if self.kind == 0 {
            return Err(FormatError::Bound("recovery kind"));
        }
        self.root_wrap.validate()?;
        if self.root_wrap.ciphertext.len() != 32 {
            return Err(FormatError::Schema("wrapped root key"));
        }
        Ok(())
    }
}

/// Signed, provider-discoverable bootstrap record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BootstrapV1 {
    /// Vault identifier.
    pub vault_id: VaultId,
    /// Monotonic bootstrap generation.
    pub generation: u64,
    /// Previous signed bootstrap ID, absent only for generation zero.
    pub previous_bootstrap: Option<BootstrapId>,
    /// V1 cryptographic suite registry value.
    pub crypto_suite: u16,
    /// Persisted passphrase KDF parameters.
    pub kdf: Argon2idParametersV1,
    /// Passphrase-derived wrapping of the random vault root key.
    pub passphrase_root_wrap: AeadEnvelopeV1,
    /// Vault authority Ed25519 public key.
    pub authority_public_key: PublicKey,
    /// Optional recovery wraps.
    pub recovery_wraps: Vec<RecoveryWrapV1>,
    /// Authority signature over [`BootstrapV1::signing_preimage`].
    pub signature: Signature,
}

impl BootstrapV1 {
    /// Validate cross-field invariants and defensive bounds.
    pub fn validate(&self) -> Result<(), FormatError> {
        check_suite(self.crypto_suite)?;
        self.kdf.validate()?;
        self.passphrase_root_wrap.validate()?;
        if self.passphrase_root_wrap.ciphertext.len() != 32 {
            return Err(FormatError::Schema("wrapped root key"));
        }
        if self.recovery_wraps.len() > MAX_RECOVERY_WRAPS {
            return Err(FormatError::Bound("recovery wraps"));
        }
        for wrap in &self.recovery_wraps {
            wrap.validate()?;
        }
        if (self.generation == 0) != self.previous_bootstrap.is_none() {
            return Err(FormatError::InvalidGeneration);
        }
        Ok(())
    }

    /// Canonically encode the complete signed bootstrap.
    pub fn encode(&self) -> Result<Vec<u8>, FormatError> {
        self.validate()?;
        Ok(encode(&self.to_cbor(true)))
    }

    /// Strictly decode and validate a complete signed bootstrap.
    pub fn decode(bytes: &[u8]) -> Result<Self, FormatError> {
        let value = decode_structured(bytes)?;
        let mut fields = closed_fields(value, &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10], "bootstrap")?;
        check_version(take_uint(&mut fields, 1, "bootstrap version")?)?;
        let recovery_values = take_array(&mut fields, 9, "recovery wraps")?;
        if recovery_values.len() > MAX_RECOVERY_WRAPS {
            return Err(FormatError::Bound("recovery wraps"));
        }
        let value = Self {
            vault_id: VaultId(take_fixed(&mut fields, 2, "vault id")?),
            generation: take_uint(&mut fields, 3, "generation")?,
            previous_bootstrap: take_optional_fixed(&mut fields, 4, "previous bootstrap")?
                .map(BootstrapId),
            crypto_suite: to_u16(take_uint(&mut fields, 5, "crypto suite")?, "crypto suite")?,
            kdf: kdf_from_cbor(take(&mut fields, 6, "kdf")?)?,
            passphrase_root_wrap: envelope_from_cbor(take(&mut fields, 7, "root wrap")?)?,
            authority_public_key: PublicKey(take_fixed(&mut fields, 8, "authority key")?),
            recovery_wraps: recovery_values
                .into_iter()
                .map(recovery_from_cbor)
                .collect::<Result<_, _>>()?,
            signature: Signature(take_fixed(&mut fields, 10, "signature")?),
        };
        value.validate()?;
        Ok(value)
    }

    /// Domain-separated bytes that the vault authority signs.
    pub fn signing_preimage(&self) -> Result<Vec<u8>, FormatError> {
        self.validate()?;
        Ok(domain_bytes(
            BOOTSTRAP_SIGN_DOMAIN,
            &encode(&self.to_cbor(false)),
        ))
    }

    /// Replace the signature after an external signer signs the preimage.
    pub fn with_signature(mut self, signature: Signature) -> Self {
        self.signature = signature;
        self
    }

    /// Derive the domain-separated ID of the complete signed bootstrap.
    pub fn id(&self) -> Result<BootstrapId, FormatError> {
        Ok(BootstrapId(domain_hash(
            BOOTSTRAP_ID_DOMAIN,
            &self.encode()?,
        )))
    }

    fn to_cbor(&self, signed: bool) -> CborValue {
        let mut fields = vec![
            field(1, CborValue::Unsigned(FORMAT_VERSION_V1)),
            field(2, bytes(self.vault_id.0)),
            field(3, CborValue::Unsigned(self.generation)),
            field(4, optional_bytes(self.previous_bootstrap.map(|id| id.0))),
            field(5, CborValue::Unsigned(self.crypto_suite.into())),
            field(6, kdf_to_cbor(&self.kdf)),
            field(7, envelope_to_cbor(&self.passphrase_root_wrap)),
            field(8, bytes(self.authority_public_key.0)),
            field(
                9,
                CborValue::Array(self.recovery_wraps.iter().map(recovery_to_cbor).collect()),
            ),
        ];
        if signed {
            fields.push(field(10, bytes(self.signature.0)));
        }
        CborValue::Map(fields)
    }
}

/// Authority-signed device public-key certificate.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeviceCertificateV1 {
    /// Vault to which this device belongs.
    pub vault_id: VaultId,
    /// Device identifier.
    pub device_id: DeviceId,
    /// Device Ed25519 verification key.
    pub signing_public_key: PublicKey,
    /// Device X25519 recipient/wrapping key.
    pub wrapping_public_key: PublicKey,
    /// Advisory creation time in Unix milliseconds.
    pub created_at_ms: u64,
    /// Strictly sorted unique capability registry codes.
    pub capabilities: Vec<u16>,
    /// Vault authority signature.
    pub signature: Signature,
}

impl DeviceCertificateV1 {
    /// Validate capability bounds and canonical ordering.
    pub fn validate(&self) -> Result<(), FormatError> {
        if self.capabilities.len() > MAX_DEVICE_CAPABILITIES {
            return Err(FormatError::Bound("device capabilities"));
        }
        if !strictly_sorted(&self.capabilities) {
            return Err(FormatError::Ordering("device capabilities"));
        }
        Ok(())
    }

    /// Encode the complete signed certificate.
    pub fn encode(&self) -> Result<Vec<u8>, FormatError> {
        self.validate()?;
        Ok(encode(&self.to_cbor(true)))
    }

    /// Decode a complete signed certificate.
    pub fn decode(bytes: &[u8]) -> Result<Self, FormatError> {
        let mut fields = closed_fields(
            decode_structured(bytes)?,
            &[1, 2, 3, 4, 5, 6, 7, 8],
            "device certificate",
        )?;
        check_version(take_uint(&mut fields, 1, "certificate version")?)?;
        let raw_capabilities = take_array(&mut fields, 7, "capabilities")?;
        if raw_capabilities.len() > MAX_DEVICE_CAPABILITIES {
            return Err(FormatError::Bound("device capabilities"));
        }
        let capabilities = raw_capabilities
            .into_iter()
            .map(|v| match v {
                CborValue::Unsigned(n) => to_u16(n, "device capability"),
                _ => Err(FormatError::Schema("device capability")),
            })
            .collect::<Result<Vec<_>, _>>()?;
        let value = Self {
            vault_id: VaultId(take_fixed(&mut fields, 2, "vault id")?),
            device_id: DeviceId(take_fixed(&mut fields, 3, "device id")?),
            signing_public_key: PublicKey(take_fixed(&mut fields, 4, "signing key")?),
            wrapping_public_key: PublicKey(take_fixed(&mut fields, 5, "wrapping key")?),
            created_at_ms: take_uint(&mut fields, 6, "creation time")?,
            capabilities,
            signature: Signature(take_fixed(&mut fields, 8, "signature")?),
        };
        value.validate()?;
        Ok(value)
    }

    /// Domain-separated bytes signed by the vault authority.
    pub fn signing_preimage(&self) -> Result<Vec<u8>, FormatError> {
        self.validate()?;
        Ok(domain_bytes(
            DEVICE_CERT_SIGN_DOMAIN,
            &encode(&self.to_cbor(false)),
        ))
    }

    /// Replace the authority signature.
    pub fn with_signature(mut self, signature: Signature) -> Self {
        self.signature = signature;
        self
    }

    fn to_cbor(&self, signed: bool) -> CborValue {
        let mut fields = vec![
            field(1, CborValue::Unsigned(FORMAT_VERSION_V1)),
            field(2, bytes(self.vault_id.0)),
            field(3, bytes(self.device_id.0)),
            field(4, bytes(self.signing_public_key.0)),
            field(5, bytes(self.wrapping_public_key.0)),
            field(6, CborValue::Unsigned(self.created_at_ms)),
            field(
                7,
                CborValue::Array(
                    self.capabilities
                        .iter()
                        .map(|v| CborValue::Unsigned((*v).into()))
                        .collect(),
                ),
            ),
        ];
        if signed {
            fields.push(field(8, bytes(self.signature.0)));
        }
        CborValue::Map(fields)
    }
}

/// Device-signed immutable repository commit.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommitV1 {
    /// Vault identifier.
    pub vault_id: VaultId,
    /// Certified writer device.
    pub device_id: DeviceId,
    /// Non-zero monotonic counter for this device.
    pub device_counter: u64,
    /// Strictly sorted unique parent commit object IDs.
    pub parents: Vec<ObjectId>,
    /// Root of the encrypted catalog structure.
    pub catalog_root: ObjectId,
    /// Strictly sorted unique objects made reachable by this commit.
    pub added_objects: Vec<ObjectId>,
    /// Optional encrypted tombstone-set root.
    pub tombstone_root: Option<ObjectId>,
    /// Advisory wall time in Unix milliseconds.
    pub wall_time_ms: u64,
    /// Object ID of the writer's encrypted device certificate.
    pub device_certificate: ObjectId,
    /// Device signature over [`CommitV1::signing_preimage`].
    pub signature: Signature,
}

impl CommitV1 {
    /// Validate counters, bounds, and set ordering.
    pub fn validate(&self) -> Result<(), FormatError> {
        if self.device_counter == 0 {
            return Err(FormatError::InvalidCounter);
        }
        validate_ids(&self.parents, MAX_COMMIT_PARENTS, "commit parents")?;
        validate_ids(&self.added_objects, MAX_ADDED_OBJECTS, "added objects")?;
        Ok(())
    }

    /// Encode the complete signed commit.
    pub fn encode(&self) -> Result<Vec<u8>, FormatError> {
        self.validate()?;
        Ok(encode(&self.to_cbor(true)))
    }

    /// Decode a complete signed commit.
    pub fn decode(bytes: &[u8]) -> Result<Self, FormatError> {
        let mut fields = closed_fields(
            decode_structured(bytes)?,
            &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11],
            "commit",
        )?;
        check_version(take_uint(&mut fields, 1, "commit version")?)?;
        let value = Self {
            vault_id: VaultId(take_fixed(&mut fields, 2, "vault id")?),
            device_id: DeviceId(take_fixed(&mut fields, 3, "device id")?),
            device_counter: take_uint(&mut fields, 4, "device counter")?,
            parents: take_ids(&mut fields, 5, MAX_COMMIT_PARENTS, "commit parents")?,
            catalog_root: ObjectId(take_fixed(&mut fields, 6, "catalog root")?),
            added_objects: take_ids(&mut fields, 7, MAX_ADDED_OBJECTS, "added objects")?,
            tombstone_root: take_optional_fixed(&mut fields, 8, "tombstone root")?.map(ObjectId),
            wall_time_ms: take_uint(&mut fields, 9, "wall time")?,
            device_certificate: ObjectId(take_fixed(&mut fields, 10, "device certificate")?),
            signature: Signature(take_fixed(&mut fields, 11, "signature")?),
        };
        value.validate()?;
        Ok(value)
    }

    /// Domain-separated bytes signed by the device.
    pub fn signing_preimage(&self) -> Result<Vec<u8>, FormatError> {
        self.validate()?;
        Ok(domain_bytes(
            COMMIT_SIGN_DOMAIN,
            &encode(&self.to_cbor(false)),
        ))
    }

    /// Replace the device signature.
    pub fn with_signature(mut self, signature: Signature) -> Self {
        self.signature = signature;
        self
    }

    fn to_cbor(&self, signed: bool) -> CborValue {
        let mut fields = vec![
            field(1, CborValue::Unsigned(FORMAT_VERSION_V1)),
            field(2, bytes(self.vault_id.0)),
            field(3, bytes(self.device_id.0)),
            field(4, CborValue::Unsigned(self.device_counter)),
            field(5, ids_to_cbor(&self.parents)),
            field(6, bytes(self.catalog_root.0)),
            field(7, ids_to_cbor(&self.added_objects)),
            field(8, optional_bytes(self.tombstone_root.map(|id| id.0))),
            field(9, CborValue::Unsigned(self.wall_time_ms)),
            field(10, bytes(self.device_certificate.0)),
        ];
        if signed {
            fields.push(field(11, bytes(self.signature.0)));
        }
        CborValue::Map(fields)
    }
}

/// Signed discovery pointer to an encrypted commit object.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AnnouncementV1 {
    /// Vault identifier.
    pub vault_id: VaultId,
    /// Certified announcing device.
    pub device_id: DeviceId,
    /// Non-zero counter matching the referenced commit.
    pub device_counter: u64,
    /// Object ID of the encrypted commit frame.
    pub commit_id: ObjectId,
    /// Object ID of the encrypted authority-signed device certificate.
    pub device_certificate: ObjectId,
    /// Device signature.
    pub signature: Signature,
}

impl AnnouncementV1 {
    /// Validate the non-zero device counter.
    pub fn validate(&self) -> Result<(), FormatError> {
        if self.device_counter == 0 {
            return Err(FormatError::InvalidCounter);
        }
        Ok(())
    }

    /// Encode the complete signed announcement.
    pub fn encode(&self) -> Result<Vec<u8>, FormatError> {
        self.validate()?;
        Ok(encode(&self.to_cbor(true)))
    }

    /// Decode a complete signed announcement.
    pub fn decode(bytes: &[u8]) -> Result<Self, FormatError> {
        let mut fields = closed_fields(
            decode_structured(bytes)?,
            &[1, 2, 3, 4, 5, 6, 7],
            "announcement",
        )?;
        check_version(take_uint(&mut fields, 1, "announcement version")?)?;
        let value = Self {
            vault_id: VaultId(take_fixed(&mut fields, 2, "vault id")?),
            device_id: DeviceId(take_fixed(&mut fields, 3, "device id")?),
            device_counter: take_uint(&mut fields, 4, "device counter")?,
            commit_id: ObjectId(take_fixed(&mut fields, 5, "commit id")?),
            device_certificate: ObjectId(take_fixed(&mut fields, 6, "device certificate")?),
            signature: Signature(take_fixed(&mut fields, 7, "signature")?),
        };
        value.validate()?;
        Ok(value)
    }

    /// Domain-separated bytes signed by the announcing device.
    pub fn signing_preimage(&self) -> Result<Vec<u8>, FormatError> {
        self.validate()?;
        Ok(domain_bytes(
            ANNOUNCEMENT_SIGN_DOMAIN,
            &encode(&self.to_cbor(false)),
        ))
    }

    /// Replace the device signature.
    pub fn with_signature(mut self, signature: Signature) -> Self {
        self.signature = signature;
        self
    }

    fn to_cbor(&self, signed: bool) -> CborValue {
        let mut fields = vec![
            field(1, CborValue::Unsigned(FORMAT_VERSION_V1)),
            field(2, bytes(self.vault_id.0)),
            field(3, bytes(self.device_id.0)),
            field(4, CborValue::Unsigned(self.device_counter)),
            field(5, bytes(self.commit_id.0)),
            field(6, bytes(self.device_certificate.0)),
        ];
        if signed {
            fields.push(field(7, bytes(self.signature.0)));
        }
        CborValue::Map(fields)
    }
}

/// Exact encrypted binary object frame described by VLT-PM01 §7.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObjectFrameV1 {
    /// V1 cryptographic suite registry value.
    pub suite: u16,
    /// Nonce for wrapping the random object DEK.
    pub wrap_nonce: [u8; 24],
    /// Wrapped 256-bit object DEK ciphertext.
    pub wrapped_dek: [u8; 32],
    /// Detached tag for the wrapped object DEK.
    pub wrap_tag: [u8; 16],
    /// Independently random payload nonce.
    pub payload_nonce: [u8; 24],
    /// Encrypted canonical repository-object payload.
    pub ciphertext: Vec<u8>,
    /// Detached tag for the payload.
    pub payload_tag: [u8; 16],
}

impl ObjectFrameV1 {
    /// Validate suite and payload bound.
    pub fn validate(&self) -> Result<(), FormatError> {
        check_suite(self.suite)?;
        if self.ciphertext.len() > MAX_OBJECT_CIPHERTEXT {
            return Err(FormatError::Bound("object ciphertext"));
        }
        Ok(())
    }

    /// Encode the exact V1 binary frame.
    pub fn encode(&self) -> Result<Vec<u8>, FormatError> {
        self.validate()?;
        let total = OBJECT_FRAME_FIXED_LEN
            .checked_add(self.ciphertext.len())
            .ok_or(FormatError::LengthOverflow)?;
        let payload_len =
            u64::try_from(self.ciphertext.len()).map_err(|_| FormatError::LengthOverflow)?;
        let mut out = Vec::with_capacity(total);
        out.extend_from_slice(OBJECT_MAGIC);
        out.extend_from_slice(&self.suite.to_be_bytes());
        out.extend_from_slice(&self.wrap_nonce);
        out.extend_from_slice(&self.wrapped_dek);
        out.extend_from_slice(&self.wrap_tag);
        out.extend_from_slice(&self.payload_nonce);
        out.extend_from_slice(&payload_len.to_be_bytes());
        out.extend_from_slice(&self.ciphertext);
        out.extend_from_slice(&self.payload_tag);
        Ok(out)
    }

    /// Strictly decode an exact V1 binary frame.
    pub fn decode(bytes: &[u8]) -> Result<Self, FormatError> {
        if bytes.len() < OBJECT_FRAME_FIXED_LEN {
            return Err(FormatError::Truncated);
        }
        if &bytes[0..4] != OBJECT_MAGIC {
            return Err(FormatError::InvalidMagic);
        }
        let suite = u16::from_be_bytes([bytes[4], bytes[5]]);
        check_suite(suite)?;
        let payload_len_u64 = u64::from_be_bytes(
            bytes[102..110]
                .try_into()
                .map_err(|_| FormatError::Truncated)?,
        );
        let payload_len =
            usize::try_from(payload_len_u64).map_err(|_| FormatError::LengthOverflow)?;
        if payload_len > MAX_OBJECT_CIPHERTEXT {
            return Err(FormatError::Bound("object ciphertext"));
        }
        let expected = OBJECT_FRAME_FIXED_LEN
            .checked_add(payload_len)
            .ok_or(FormatError::LengthOverflow)?;
        if bytes.len() < expected {
            return Err(FormatError::Truncated);
        }
        if bytes.len() > expected {
            return Err(FormatError::TrailingBytes);
        }
        let payload_end = 110 + payload_len;
        let value = Self {
            suite,
            wrap_nonce: bytes[6..30]
                .try_into()
                .map_err(|_| FormatError::Truncated)?,
            wrapped_dek: bytes[30..62]
                .try_into()
                .map_err(|_| FormatError::Truncated)?,
            wrap_tag: bytes[62..78]
                .try_into()
                .map_err(|_| FormatError::Truncated)?,
            payload_nonce: bytes[78..102]
                .try_into()
                .map_err(|_| FormatError::Truncated)?,
            ciphertext: bytes[110..payload_end].to_vec(),
            payload_tag: bytes[payload_end..expected]
                .try_into()
                .map_err(|_| FormatError::Truncated)?,
        };
        value.validate()?;
        Ok(value)
    }

    /// Derive the domain-separated object ID from the exact encoded frame.
    pub fn id(&self) -> Result<ObjectId, FormatError> {
        Ok(ObjectId::for_frame(&self.encode()?))
    }
}

fn domain_bytes(domain: &[u8], body: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(domain.len() + body.len());
    out.extend_from_slice(domain);
    out.extend_from_slice(body);
    out
}

fn domain_hash(domain: &[u8], body: &[u8]) -> [u8; 32] {
    sha256(&domain_bytes(domain, body))
}

fn check_suite(suite: u16) -> Result<(), FormatError> {
    if suite == CRYPTO_SUITE_V1 {
        Ok(())
    } else {
        Err(FormatError::UnsupportedSuite)
    }
}

fn check_version(version: u64) -> Result<(), FormatError> {
    if version == FORMAT_VERSION_V1 {
        Ok(())
    } else {
        Err(FormatError::UnsupportedVersion)
    }
}

fn field(key: u64, value: CborValue) -> (CborValue, CborValue) {
    (CborValue::Unsigned(key), value)
}

fn bytes<const N: usize>(value: [u8; N]) -> CborValue {
    CborValue::Bytes(value.to_vec())
}

fn optional_bytes<const N: usize>(value: Option<[u8; N]>) -> CborValue {
    value.map(bytes).unwrap_or(CborValue::Null)
}

fn envelope_to_cbor(value: &AeadEnvelopeV1) -> CborValue {
    CborValue::Map(vec![
        field(1, CborValue::Unsigned(value.suite.into())),
        field(2, bytes(value.nonce)),
        field(3, CborValue::Bytes(value.ciphertext.clone())),
        field(4, bytes(value.tag)),
    ])
}

fn envelope_from_cbor(value: CborValue) -> Result<AeadEnvelopeV1, FormatError> {
    let mut fields = closed_fields(value, &[1, 2, 3, 4], "AEAD envelope")?;
    let ciphertext = take_bytes(&mut fields, 3, "envelope ciphertext")?;
    let value = AeadEnvelopeV1 {
        suite: to_u16(
            take_uint(&mut fields, 1, "envelope suite")?,
            "envelope suite",
        )?,
        nonce: take_fixed(&mut fields, 2, "envelope nonce")?,
        ciphertext,
        tag: take_fixed(&mut fields, 4, "envelope tag")?,
    };
    value.validate()?;
    Ok(value)
}

fn kdf_to_cbor(value: &Argon2idParametersV1) -> CborValue {
    CborValue::Map(vec![
        field(1, CborValue::Unsigned(value.memory_kib.into())),
        field(2, CborValue::Unsigned(value.iterations.into())),
        field(3, CborValue::Unsigned(value.lanes.into())),
        field(4, bytes(value.salt)),
    ])
}

fn kdf_from_cbor(value: CborValue) -> Result<Argon2idParametersV1, FormatError> {
    let mut fields = closed_fields(value, &[1, 2, 3, 4], "Argon2id parameters")?;
    let value = Argon2idParametersV1 {
        memory_kib: to_u32(take_uint(&mut fields, 1, "argon2 memory")?, "argon2 memory")?,
        iterations: to_u32(
            take_uint(&mut fields, 2, "argon2 iterations")?,
            "argon2 iterations",
        )?,
        lanes: to_u8(take_uint(&mut fields, 3, "argon2 lanes")?, "argon2 lanes")?,
        salt: take_fixed(&mut fields, 4, "argon2 salt")?,
    };
    value.validate()?;
    Ok(value)
}

fn recovery_to_cbor(value: &RecoveryWrapV1) -> CborValue {
    CborValue::Map(vec![
        field(1, CborValue::Unsigned(value.kind.into())),
        field(2, bytes(value.recipient_id.0)),
        field(3, envelope_to_cbor(&value.root_wrap)),
    ])
}

fn recovery_from_cbor(value: CborValue) -> Result<RecoveryWrapV1, FormatError> {
    let mut fields = closed_fields(value, &[1, 2, 3], "recovery wrap")?;
    let value = RecoveryWrapV1 {
        kind: to_u16(take_uint(&mut fields, 1, "recovery kind")?, "recovery kind")?,
        recipient_id: RecipientId(take_fixed(&mut fields, 2, "recipient id")?),
        root_wrap: envelope_from_cbor(take(&mut fields, 3, "recovery root wrap")?)?,
    };
    value.validate()?;
    Ok(value)
}

fn ids_to_cbor(ids: &[ObjectId]) -> CborValue {
    CborValue::Array(ids.iter().map(|id| bytes(id.0)).collect())
}

fn validate_ids(ids: &[ObjectId], max: usize, field_name: &'static str) -> Result<(), FormatError> {
    if ids.len() > max {
        return Err(FormatError::Bound(field_name));
    }
    if !strictly_sorted(ids) {
        return Err(FormatError::Ordering(field_name));
    }
    Ok(())
}

fn strictly_sorted<T: Ord>(values: &[T]) -> bool {
    values.windows(2).all(|pair| pair[0] < pair[1])
}

fn decode_structured(bytes: &[u8]) -> Result<CborValue, FormatError> {
    if bytes.len() > MAX_STRUCTURED_BYTES {
        return Err(FormatError::InputTooLarge);
    }
    Ok(decode(bytes)?)
}

fn closed_fields(
    value: CborValue,
    expected: &[u64],
    name: &'static str,
) -> Result<BTreeMap<u64, CborValue>, FormatError> {
    let entries = match value {
        CborValue::Map(entries) => entries,
        _ => return Err(FormatError::Schema(name)),
    };
    if entries.len() != expected.len() {
        return Err(FormatError::Schema(name));
    }
    let mut fields = BTreeMap::new();
    for (key, value) in entries {
        let key = match key {
            CborValue::Unsigned(key) => key,
            _ => return Err(FormatError::Schema(name)),
        };
        if !expected.contains(&key) || fields.insert(key, value).is_some() {
            return Err(FormatError::Schema(name));
        }
    }
    Ok(fields)
}

fn take(
    fields: &mut BTreeMap<u64, CborValue>,
    key: u64,
    name: &'static str,
) -> Result<CborValue, FormatError> {
    fields.remove(&key).ok_or(FormatError::Schema(name))
}

fn take_uint(
    fields: &mut BTreeMap<u64, CborValue>,
    key: u64,
    name: &'static str,
) -> Result<u64, FormatError> {
    match take(fields, key, name)? {
        CborValue::Unsigned(value) => Ok(value),
        _ => Err(FormatError::Schema(name)),
    }
}

fn take_bytes(
    fields: &mut BTreeMap<u64, CborValue>,
    key: u64,
    name: &'static str,
) -> Result<Vec<u8>, FormatError> {
    match take(fields, key, name)? {
        CborValue::Bytes(value) => Ok(value),
        _ => Err(FormatError::Schema(name)),
    }
}

fn take_array(
    fields: &mut BTreeMap<u64, CborValue>,
    key: u64,
    name: &'static str,
) -> Result<Vec<CborValue>, FormatError> {
    match take(fields, key, name)? {
        CborValue::Array(value) => Ok(value),
        _ => Err(FormatError::Schema(name)),
    }
}

fn take_fixed<const N: usize>(
    fields: &mut BTreeMap<u64, CborValue>,
    key: u64,
    name: &'static str,
) -> Result<[u8; N], FormatError> {
    take_bytes(fields, key, name)?
        .try_into()
        .map_err(|_| FormatError::Schema(name))
}

fn take_optional_fixed<const N: usize>(
    fields: &mut BTreeMap<u64, CborValue>,
    key: u64,
    name: &'static str,
) -> Result<Option<[u8; N]>, FormatError> {
    match take(fields, key, name)? {
        CborValue::Null => Ok(None),
        CborValue::Bytes(value) => value
            .try_into()
            .map(Some)
            .map_err(|_| FormatError::Schema(name)),
        _ => Err(FormatError::Schema(name)),
    }
}

fn take_ids(
    fields: &mut BTreeMap<u64, CborValue>,
    key: u64,
    max: usize,
    name: &'static str,
) -> Result<Vec<ObjectId>, FormatError> {
    let values = take_array(fields, key, name)?;
    if values.len() > max {
        return Err(FormatError::Bound(name));
    }
    let ids = values
        .into_iter()
        .map(|value| match value {
            CborValue::Bytes(value) => value
                .try_into()
                .map(ObjectId)
                .map_err(|_| FormatError::Schema(name)),
            _ => Err(FormatError::Schema(name)),
        })
        .collect::<Result<Vec<_>, _>>()?;
    validate_ids(&ids, max, name)?;
    Ok(ids)
}

fn to_u16(value: u64, name: &'static str) -> Result<u16, FormatError> {
    u16::try_from(value).map_err(|_| FormatError::Bound(name))
}

fn to_u32(value: u64, name: &'static str) -> Result<u32, FormatError> {
    u32::try_from(value).map_err(|_| FormatError::Bound(name))
}

fn to_u8(value: u64, name: &'static str) -> Result<u8, FormatError> {
    u8::try_from(value).map_err(|_| FormatError::Bound(name))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn envelope(seed: u8) -> AeadEnvelopeV1 {
        AeadEnvelopeV1 {
            suite: 1,
            nonce: [seed; 24],
            ciphertext: vec![seed; 32],
            tag: [seed + 3; 16],
        }
    }

    fn bootstrap() -> BootstrapV1 {
        BootstrapV1 {
            vault_id: VaultId::new([1; 16]),
            generation: 0,
            previous_bootstrap: None,
            crypto_suite: 1,
            kdf: Argon2idParametersV1 {
                memory_kib: 65_536,
                iterations: 3,
                lanes: 2,
                salt: [2; 16],
            },
            passphrase_root_wrap: envelope(3),
            authority_public_key: PublicKey::new([4; 32]),
            recovery_wraps: vec![RecoveryWrapV1 {
                kind: 1,
                recipient_id: RecipientId::new([5; 16]),
                root_wrap: envelope(6),
            }],
            signature: Signature::new([7; 64]),
        }
    }

    fn certificate() -> DeviceCertificateV1 {
        DeviceCertificateV1 {
            vault_id: VaultId::new([1; 16]),
            device_id: DeviceId::new([8; 16]),
            signing_public_key: PublicKey::new([9; 32]),
            wrapping_public_key: PublicKey::new([10; 32]),
            created_at_ms: 1_700_000_000_000,
            capabilities: vec![1, 4, 9],
            signature: Signature::new([11; 64]),
        }
    }

    fn commit() -> CommitV1 {
        CommitV1 {
            vault_id: VaultId::new([1; 16]),
            device_id: DeviceId::new([8; 16]),
            device_counter: 1,
            parents: vec![ObjectId::new([1; 32]), ObjectId::new([2; 32])],
            catalog_root: ObjectId::new([3; 32]),
            added_objects: vec![ObjectId::new([4; 32]), ObjectId::new([5; 32])],
            tombstone_root: Some(ObjectId::new([6; 32])),
            wall_time_ms: 1_700_000_000_001,
            device_certificate: ObjectId::new([7; 32]),
            signature: Signature::new([12; 64]),
        }
    }

    fn announcement() -> AnnouncementV1 {
        AnnouncementV1 {
            vault_id: VaultId::new([1; 16]),
            device_id: DeviceId::new([8; 16]),
            device_counter: 1,
            commit_id: ObjectId::new([13; 32]),
            device_certificate: ObjectId::new([7; 32]),
            signature: Signature::new([14; 64]),
        }
    }

    fn frame() -> ObjectFrameV1 {
        ObjectFrameV1 {
            suite: 1,
            wrap_nonce: [1; 24],
            wrapped_dek: [2; 32],
            wrap_tag: [3; 16],
            payload_nonce: [4; 24],
            ciphertext: vec![5, 6, 7, 8],
            payload_tag: [9; 16],
        }
    }

    fn object_id(index: usize) -> ObjectId {
        let mut bytes = [0; 32];
        bytes[..8].copy_from_slice(&(index as u64).to_be_bytes());
        ObjectId::new(bytes)
    }

    #[test]
    fn structured_round_trips() {
        let b = bootstrap();
        assert_eq!(BootstrapV1::decode(&b.encode().unwrap()).unwrap(), b);
        let c = certificate();
        assert_eq!(
            DeviceCertificateV1::decode(&c.encode().unwrap()).unwrap(),
            c
        );
        let m = commit();
        assert_eq!(CommitV1::decode(&m.encode().unwrap()).unwrap(), m);
        let a = announcement();
        assert_eq!(AnnouncementV1::decode(&a.encode().unwrap()).unwrap(), a);
    }

    #[test]
    fn signing_preimages_are_domain_separated_and_ignore_signature() {
        let b = bootstrap();
        assert!(b
            .signing_preimage()
            .unwrap()
            .starts_with(BOOTSTRAP_SIGN_DOMAIN));
        assert_eq!(
            b.signing_preimage().unwrap(),
            b.clone()
                .with_signature(Signature::new([99; 64]))
                .signing_preimage()
                .unwrap()
        );
        assert_ne!(
            b.encode().unwrap(),
            b.clone()
                .with_signature(Signature::new([99; 64]))
                .encode()
                .unwrap()
        );
        assert!(certificate()
            .signing_preimage()
            .unwrap()
            .starts_with(DEVICE_CERT_SIGN_DOMAIN));
        assert!(commit()
            .signing_preimage()
            .unwrap()
            .starts_with(COMMIT_SIGN_DOMAIN));
        assert!(announcement()
            .signing_preimage()
            .unwrap()
            .starts_with(ANNOUNCEMENT_SIGN_DOMAIN));
    }

    #[test]
    fn ids_are_domain_separated_and_signature_sensitive() {
        let b = bootstrap();
        assert_ne!(
            b.id().unwrap(),
            b.clone()
                .with_signature(Signature::new([99; 64]))
                .id()
                .unwrap()
        );
        let f = frame();
        assert_eq!(f.id().unwrap(), ObjectId::for_frame(&f.encode().unwrap()));
        assert_ne!(f.id().unwrap().into_bytes(), sha256(&f.encode().unwrap()));
    }

    #[test]
    fn frame_round_trip_and_layout() {
        let f = frame();
        let bytes = f.encode().unwrap();
        assert_eq!(bytes.len(), 130);
        assert_eq!(&bytes[..4], b"VPO1");
        assert_eq!(&bytes[102..110], &4u64.to_be_bytes());
        assert_eq!(ObjectFrameV1::decode(&bytes).unwrap(), f);
    }

    #[test]
    fn frame_rejects_truncation_trailing_magic_suite_and_bound() {
        let bytes = frame().encode().unwrap();
        for end in [0, 4, 5, 109, 125, 129] {
            assert_eq!(
                ObjectFrameV1::decode(&bytes[..end]),
                Err(FormatError::Truncated)
            );
        }
        let mut bad = bytes.clone();
        bad[0] = b'X';
        assert_eq!(ObjectFrameV1::decode(&bad), Err(FormatError::InvalidMagic));
        let mut bad = bytes.clone();
        bad[5] = 2;
        assert_eq!(
            ObjectFrameV1::decode(&bad),
            Err(FormatError::UnsupportedSuite)
        );
        let mut bad = bytes.clone();
        bad.extend_from_slice(&[0]);
        assert_eq!(ObjectFrameV1::decode(&bad), Err(FormatError::TrailingBytes));
        let mut bad = bytes;
        bad[102..110].copy_from_slice(&((MAX_OBJECT_CIPHERTEXT as u64) + 1).to_be_bytes());
        assert_eq!(
            ObjectFrameV1::decode(&bad),
            Err(FormatError::Bound("object ciphertext"))
        );
    }

    #[test]
    fn bootstrap_generation_rules_are_strict() {
        let mut b = bootstrap();
        b.generation = 1;
        assert_eq!(b.validate(), Err(FormatError::InvalidGeneration));
        let mut b = bootstrap();
        b.previous_bootstrap = Some(BootstrapId::new([1; 32]));
        assert_eq!(b.validate(), Err(FormatError::InvalidGeneration));
        let mut b = bootstrap();
        b.generation = 1;
        b.previous_bootstrap = Some(BootstrapId::new([1; 32]));
        assert!(b.validate().is_ok());
    }

    #[test]
    fn kdf_and_envelope_bounds_are_enforced() {
        let mut b = bootstrap();
        b.kdf.memory_kib = 8191;
        assert_eq!(b.validate(), Err(FormatError::Bound("argon2 memory")));
        let mut b = bootstrap();
        b.kdf.iterations = 33;
        assert_eq!(b.validate(), Err(FormatError::Bound("argon2 iterations")));
        let mut b = bootstrap();
        b.kdf.lanes = 0;
        assert_eq!(b.validate(), Err(FormatError::Bound("argon2 lanes")));
        let mut b = bootstrap();
        b.passphrase_root_wrap.suite = 2;
        assert_eq!(b.validate(), Err(FormatError::UnsupportedSuite));
        let mut b = bootstrap();
        b.passphrase_root_wrap.ciphertext = vec![0; MAX_ENVELOPE_CIPHERTEXT + 1];
        assert_eq!(b.validate(), Err(FormatError::Bound("envelope ciphertext")));
        let mut b = bootstrap();
        b.passphrase_root_wrap.ciphertext = vec![0; 31];
        assert_eq!(b.validate(), Err(FormatError::Schema("wrapped root key")));
    }

    #[test]
    fn counters_ordering_and_count_bounds_are_enforced() {
        let mut c = commit();
        c.device_counter = 0;
        assert_eq!(c.validate(), Err(FormatError::InvalidCounter));
        let mut c = commit();
        c.parents.reverse();
        assert_eq!(c.validate(), Err(FormatError::Ordering("commit parents")));
        let mut c = commit();
        c.added_objects.push(ObjectId::new([5; 32]));
        assert_eq!(c.validate(), Err(FormatError::Ordering("added objects")));
        let mut cert = certificate();
        cert.capabilities = vec![4, 1];
        assert_eq!(
            cert.validate(),
            Err(FormatError::Ordering("device capabilities"))
        );
        let mut a = announcement();
        a.device_counter = 0;
        assert_eq!(a.validate(), Err(FormatError::InvalidCounter));

        let mut c = commit();
        c.parents = (0..=MAX_COMMIT_PARENTS).map(object_id).collect();
        assert_eq!(c.validate(), Err(FormatError::Bound("commit parents")));
        let mut c = commit();
        c.added_objects = (0..=MAX_ADDED_OBJECTS).map(object_id).collect();
        assert_eq!(c.validate(), Err(FormatError::Bound("added objects")));
        let mut cert = certificate();
        cert.capabilities = (0..=MAX_DEVICE_CAPABILITIES as u16).collect();
        assert_eq!(
            cert.validate(),
            Err(FormatError::Bound("device capabilities"))
        );
        let mut b = bootstrap();
        b.recovery_wraps = vec![b.recovery_wraps[0].clone(); MAX_RECOVERY_WRAPS + 1];
        assert_eq!(b.validate(), Err(FormatError::Bound("recovery wraps")));
    }

    #[test]
    fn closed_schemas_reject_unknown_missing_and_wrong_type_fields() {
        let encoded = bootstrap().encode().unwrap();
        let CborValue::Map(mut fields) = decode(&encoded).unwrap() else {
            panic!()
        };
        fields.push(field(99, CborValue::Unsigned(1)));
        assert!(matches!(
            BootstrapV1::decode(&encode(&CborValue::Map(fields))),
            Err(FormatError::Schema(_))
        ));
        let CborValue::Map(mut fields) = decode(&encoded).unwrap() else {
            panic!()
        };
        fields.retain(|(k, _)| k != &CborValue::Unsigned(2));
        assert!(matches!(
            BootstrapV1::decode(&encode(&CborValue::Map(fields))),
            Err(FormatError::Schema(_))
        ));
        let CborValue::Map(mut fields) = decode(&encoded).unwrap() else {
            panic!()
        };
        for (k, v) in &mut fields {
            if k == &CborValue::Unsigned(2) {
                *v = CborValue::Unsigned(1);
            }
        }
        assert!(matches!(
            BootstrapV1::decode(&encode(&CborValue::Map(fields))),
            Err(FormatError::Schema(_))
        ));
    }

    #[test]
    fn noncanonical_and_oversized_structured_input_are_rejected() {
        assert!(matches!(
            AnnouncementV1::decode(&[0xb8, 0x00]),
            Err(FormatError::Cbor(_))
        ));
        assert_eq!(
            AnnouncementV1::decode(&vec![0; MAX_STRUCTURED_BYTES + 1]),
            Err(FormatError::InputTooLarge)
        );
    }

    #[test]
    fn unsupported_version_and_fixed_width_are_rejected() {
        let encoded = announcement().encode().unwrap();
        let CborValue::Map(mut fields) = decode(&encoded).unwrap() else {
            panic!()
        };
        for (k, v) in &mut fields {
            if k == &CborValue::Unsigned(1) {
                *v = CborValue::Unsigned(2);
            }
        }
        assert_eq!(
            AnnouncementV1::decode(&encode(&CborValue::Map(fields))),
            Err(FormatError::UnsupportedVersion)
        );
        let CborValue::Map(mut fields) = decode(&encoded).unwrap() else {
            panic!()
        };
        for (k, v) in &mut fields {
            if k == &CborValue::Unsigned(2) {
                *v = CborValue::Bytes(vec![0; 15]);
            }
        }
        assert!(matches!(
            AnnouncementV1::decode(&encode(&CborValue::Map(fields))),
            Err(FormatError::Schema(_))
        ));
    }

    #[test]
    fn error_display_does_not_echo_field_names_or_bytes() {
        assert_eq!(
            FormatError::Schema("attacker-controlled").to_string(),
            "vault-pm-format: schema mismatch"
        );
        assert_eq!(
            FormatError::Bound("secret title").to_string(),
            "vault-pm-format: field exceeds V1 bound"
        );
    }

    #[test]
    fn golden_vectors_match_checked_in_fixture() {
        fn hex(bytes: &[u8]) -> String {
            bytes.iter().map(|b| format!("{b:02x}")).collect()
        }
        let fixture = include_str!("../../../../specs/fixtures/vault-pm-format-v1.hex");
        let entries: BTreeMap<&str, &str> = fixture
            .lines()
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .map(|line| line.split_once('=').expect("fixture line must contain '='"))
            .collect();

        assert_eq!(entries["bootstrap"], hex(&bootstrap().encode().unwrap()));
        assert_eq!(
            entries["bootstrap_id"],
            hex(bootstrap().id().unwrap().as_bytes())
        );
        assert_eq!(
            entries["device_certificate"],
            hex(&certificate().encode().unwrap())
        );
        assert_eq!(entries["commit"], hex(&commit().encode().unwrap()));
        assert_eq!(
            entries["announcement"],
            hex(&announcement().encode().unwrap())
        );
        assert_eq!(entries["object_frame"], hex(&frame().encode().unwrap()));
        assert_eq!(entries["object_id"], hex(frame().id().unwrap().as_bytes()));
        assert_eq!(entries.len(), 7);
    }
}
