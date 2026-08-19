//! Bounded, hand-rolled wire protocol for the local vault-pm agent (VLT-PM48).
//!
//! # What crosses this boundary, and what does not
//!
//! The agent retains exactly one thing across process boundaries: the master
//! passphrase a person already typed once, so a later one-shot command does
//! not have to ask again. Everything else this product treats as secret —
//! decrypted item fields, the vault root key, search results, TOTP codes —
//! never crosses this wire. See `VLT-PM48-local-agent-ipc.md` §3 for the full
//! argument; the short version is that the agent is a *passphrase* cache, not
//! a session cache, so a compromise of this channel recovers no more than a
//! compromise of `vault-pm shell`'s own in-memory buffer already would
//! (VLT-PM40 §7.1).
//!
//! # Why hand-rolled framing instead of a serialization crate
//!
//! This workspace does not carry `serde` as a general-purpose dependency —
//! `vault-pm-format`, `vault-pm-cli-host`'s `ClipboardClearRequest`, and every
//! other wire format in this product line are exact, hand-checked byte
//! layouts instead. This protocol follows the same convention: every request
//! and response has one fixed tag byte and explicitly bounded fields, so a
//! reviewer can read the exact bytes a caller can send without first
//! auditing a general-purpose decoder.
//!
//! # Frame shape
//!
//! A frame is `[4-byte big-endian length][payload]`. The length prefix is
//! read by the transport (`vault-pm-agent-host`), which enforces
//! [`MAX_FRAME_BYTES`] *before* allocating a buffer for the payload — an
//! oversized length is refused without ever reading the bytes that follow.
//! This crate only encodes and decodes the payload; it performs no I/O and
//! owns no socket.
//!
//! A payload is `[1-byte version][1-byte tag][tag-specific fields]`. The
//! version is [`PROTOCOL_VERSION`]; a mismatched version is refused rather
//! than interpreted leniently, because a future incompatible revision must
//! never be silently misread as this one.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use coding_adventures_zeroize::Zeroizing;
use core::fmt::{self, Debug, Formatter};

/// The wire protocol version this crate encodes and expects.
pub const PROTOCOL_VERSION: u8 = 1;

/// Longest vault name accepted on the wire.
///
/// Equal to `vault-pm-config`'s own `ConfigName` bound. A longer value cannot
/// name a real vault, so it is refused before it is copied into a `String`.
pub const MAX_VAULT_NAME_BYTES: usize = 64;

/// Longest passphrase accepted on the wire.
///
/// Equal to `vault-pm-cli-host::MAX_SECRET_BYTES`, the same ceiling every
/// terminal-collected secret in this product is already bound by. A
/// passphrase that could not have been typed at a prompt is not a passphrase
/// this protocol will carry.
pub const MAX_PASSPHRASE_BYTES: usize = 1_024;

/// Most vaults one [`AgentResponse::Status`] will describe.
///
/// `VLT-PM07` bounds one configuration to at most a small, fixed number of
/// vaults; this is a generous multiple of that, present so a malformed or
/// hostile peer cannot make a status response unboundedly large.
pub const MAX_STATUS_VAULTS: usize = 64;

/// Upper bound on one encoded payload, including its version and tag bytes.
///
/// Sized for the single largest message, [`AgentRequest::Unlock`]: a
/// version byte, a tag byte, one length-prefixed vault name, one
/// length-prefixed passphrase, and one 8-byte idle bound. The transport
/// enforces this as the ceiling on the frame's length prefix, so a peer
/// cannot make it allocate an unbounded buffer merely by claiming one.
pub const MAX_FRAME_BYTES: usize =
    1 + 1 + (1 + MAX_VAULT_NAME_BYTES) + (4 + MAX_PASSPHRASE_BYTES) + 8;

const TAG_PING: u8 = 0x01;
const TAG_UNLOCK: u8 = 0x02;
const TAG_GET_PASSPHRASE: u8 = 0x03;
const TAG_LOCK: u8 = 0x04;
const TAG_STATUS: u8 = 0x05;
const TAG_SHUTDOWN: u8 = 0x06;

const TAG_OK: u8 = 0x81;
const TAG_PASSPHRASE: u8 = 0x82;
const TAG_NOT_RETAINED: u8 = 0x83;
const TAG_STATUS_REPORT: u8 = 0x84;
const TAG_ERROR: u8 = 0xFF;

/// A vault name may be absent (every vault) or present (one vault).
///
/// Encoded as one presence byte (`0` or `1`) followed by the name's own
/// length-prefixed encoding when present. Kept as a free function rather than
/// a shared trait because exactly two call sites need it and a trait would
/// buy nothing a reviewer could not already see in nine lines.
fn encode_optional_name(out: &mut Vec<u8>, name: Option<&str>) -> Result<(), ProtocolError> {
    match name {
        None => out.push(0),
        Some(name) => {
            out.push(1);
            encode_name(out, name)?;
        }
    }
    Ok(())
}

/// Whether every byte of `bytes` is a character a real vault name could
/// contain.
///
/// Matches `vault-pm-config::ConfigName`'s own rule exactly — ASCII
/// alphanumeric, or `_`/`-` past the first byte — rather than merely UTF-8
/// and a length bound. This is load-bearing, not cosmetic: the peer on the
/// other end of this socket is authenticated only as "the same local user"
/// (`vault-pm-agent-host::peer`), never as "the genuine `vault-pm` binary",
/// so any same-user process can open a raw connection and send a
/// hand-crafted `Unlock` whose name it controls completely. That name is
/// later rendered into `agent status`'s plain-text and `--json` output
/// (`vault-pm-cli::agent::render_agent_status`) without further escaping. A
/// name containing a quote, a backslash, or a raw terminal escape sequence
/// would otherwise reach a person's JSON parser or terminal emulator
/// unescaped. Restricting the *character set* a name can ever decode to is
/// simpler and more robust than trying to escape it correctly at every
/// present and future render site.
fn is_valid_name_bytes(bytes: &[u8]) -> bool {
    !bytes.is_empty()
        && bytes.len() <= MAX_VAULT_NAME_BYTES
        && bytes.iter().enumerate().all(|(index, byte)| {
            byte.is_ascii_alphanumeric() || (index > 0 && matches!(byte, b'_' | b'-'))
        })
}

fn encode_name(out: &mut Vec<u8>, name: &str) -> Result<(), ProtocolError> {
    let bytes = name.as_bytes();
    if !is_valid_name_bytes(bytes) {
        return Err(ProtocolError);
    }
    #[allow(clippy::cast_possible_truncation)]
    out.push(bytes.len() as u8);
    out.extend_from_slice(bytes);
    Ok(())
}

fn decode_name(input: &[u8], offset: &mut usize) -> Result<String, ProtocolError> {
    let length = usize::from(read_u8(input, offset)?);
    if length == 0 || length > MAX_VAULT_NAME_BYTES {
        return Err(ProtocolError);
    }
    let bytes = read_bytes(input, offset, length)?;
    if !is_valid_name_bytes(bytes) {
        return Err(ProtocolError);
    }
    // `is_valid_name_bytes` already restricts every byte to ASCII, so this
    // can never fail — but going through `str::from_utf8` rather than
    // `String::from_utf8_unchecked` costs nothing here and keeps this
    // function free of the one `unsafe` shortcut that would otherwise be
    // tempting.
    String::from_utf8(bytes.to_vec()).map_err(|_| ProtocolError)
}

fn decode_optional_name(input: &[u8], offset: &mut usize) -> Result<Option<String>, ProtocolError> {
    match read_u8(input, offset)? {
        0 => Ok(None),
        1 => decode_name(input, offset).map(Some),
        _ => Err(ProtocolError),
    }
}

fn read_u8(input: &[u8], offset: &mut usize) -> Result<u8, ProtocolError> {
    let byte = *input.get(*offset).ok_or(ProtocolError)?;
    *offset += 1;
    Ok(byte)
}

fn read_u32(input: &[u8], offset: &mut usize) -> Result<u32, ProtocolError> {
    let bytes = read_bytes(input, offset, 4)?;
    Ok(u32::from_be_bytes(
        bytes.try_into().expect("exactly 4 bytes"),
    ))
}

fn read_u64(input: &[u8], offset: &mut usize) -> Result<u64, ProtocolError> {
    let bytes = read_bytes(input, offset, 8)?;
    Ok(u64::from_be_bytes(
        bytes.try_into().expect("exactly 8 bytes"),
    ))
}

fn read_bytes<'a>(
    input: &'a [u8],
    offset: &mut usize,
    length: usize,
) -> Result<&'a [u8], ProtocolError> {
    let end = offset.checked_add(length).ok_or(ProtocolError)?;
    let slice = input.get(*offset..end).ok_or(ProtocolError)?;
    *offset = end;
    Ok(slice)
}

/// A malformed, oversized, or version-mismatched frame.
///
/// Deliberately one variant. A local IPC decoder does not owe a remote peer a
/// diagnosis of exactly what was wrong with its bytes — see
/// `VLT-PM48-local-agent-ipc.md` §5 — and collapsing every failure to one
/// value keeps this crate from becoming an oracle for "which byte did I get
/// wrong."
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProtocolError;

impl fmt::Display for ProtocolError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("vault-pm agent protocol: malformed frame")
    }
}

impl std::error::Error for ProtocolError {}

/// One request a client sends to the agent.
pub enum AgentRequest {
    /// Confirm the agent is listening and answering. Carries no state.
    Ping,
    /// Retain one vault's passphrase for up to `idle_bound_ms` of inactivity.
    ///
    /// The agent never verifies the passphrase itself — see
    /// `VLT-PM48-local-agent-ipc.md` §4.2. A caller sends `Unlock` only after
    /// it has already confirmed the passphrase against the real vault through
    /// the ordinary application unlock path, so the agent's job is retention
    /// and expiry, never authentication.
    Unlock {
        /// The vault this passphrase belongs to.
        vault_name: String,
        /// The passphrase itself, exactly as collected from the terminal.
        passphrase: Zeroizing<Vec<u8>>,
        /// How long the agent may retain it before forgetting it on its own,
        /// mirrored from `vaults.<name>.auto_lock_seconds` (VLT-PM07).
        idle_bound_ms: u64,
    },
    /// Ask for one vault's retained passphrase, if any and not yet expired.
    GetPassphrase {
        /// The vault whose passphrase is being requested.
        vault_name: String,
    },
    /// Forget one vault's retained passphrase, or every vault's when absent.
    Lock {
        /// `Some` forgets one vault; `None` forgets all of them.
        vault_name: Option<String>,
    },
    /// Ask which vaults currently have a retained, unexpired passphrase.
    Status,
    /// Forget everything and stop listening.
    Shutdown,
}

impl Debug for AgentRequest {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Ping => formatter.write_str("AgentRequest::Ping"),
            Self::Unlock { vault_name, .. } => formatter
                .debug_struct("AgentRequest::Unlock")
                .field("vault_name", vault_name)
                .field("passphrase", &"<redacted>")
                .finish_non_exhaustive(),
            Self::GetPassphrase { vault_name } => formatter
                .debug_struct("AgentRequest::GetPassphrase")
                .field("vault_name", vault_name)
                .finish(),
            Self::Lock { vault_name } => formatter
                .debug_struct("AgentRequest::Lock")
                .field("vault_name", vault_name)
                .finish(),
            Self::Status => formatter.write_str("AgentRequest::Status"),
            Self::Shutdown => formatter.write_str("AgentRequest::Shutdown"),
        }
    }
}

impl AgentRequest {
    /// Encode this request's payload (version and tag included).
    ///
    /// # Errors
    ///
    /// Returns [`ProtocolError`] if a carried vault name or passphrase
    /// exceeds this protocol's bounds. Every constructor of a real request in
    /// this codebase validates its own inputs first (a `ConfigName` cannot
    /// exceed [`MAX_VAULT_NAME_BYTES`], a terminal-collected secret cannot
    /// exceed [`MAX_PASSPHRASE_BYTES`]), so this is defense in depth rather
    /// than an expected path.
    ///
    /// The returned buffer is [`Zeroizing`]: an `Unlock` request's encoding
    /// contains the passphrase in plaintext (that is the whole point of the
    /// wire — the agent must actually receive it), so the buffer that holds
    /// it is wiped on drop the same way every other passphrase-bearing
    /// buffer in this product is, rather than left for the allocator to hand
    /// back unscrubbed.
    pub fn encode(&self) -> Result<Zeroizing<Vec<u8>>, ProtocolError> {
        // Capacity is reserved for the *exact* worst-case size of this
        // specific message before a single byte is written, and never
        // grown after. This is load-bearing, not an optimization: wrapping
        // the buffer in `Zeroizing` only wipes whatever allocation it holds
        // at drop time — it does nothing about `Vec`'s ordinary
        // incremental-growth reallocation, which `memcpy`s the *old*
        // allocation (already containing the passphrase, for `Unlock`) into
        // a new one and frees the old one through the global allocator
        // without scrubbing it first. A buffer built with `push`/
        // `extend_from_slice` from a length-1 start reallocates for
        // essentially any real passphrase, leaving plaintext behind in freed
        // heap `Zeroizing` never sees. Exact upfront capacity is what
        // actually closes that.
        let capacity = self.encoded_capacity();
        let mut out = Zeroizing::new(Vec::with_capacity(capacity));
        out.push(PROTOCOL_VERSION);
        match self {
            Self::Ping => out.push(TAG_PING),
            Self::Unlock {
                vault_name,
                passphrase,
                idle_bound_ms,
            } => {
                out.push(TAG_UNLOCK);
                encode_name(&mut out, vault_name)?;
                if passphrase.len() > MAX_PASSPHRASE_BYTES {
                    return Err(ProtocolError);
                }
                #[allow(clippy::cast_possible_truncation)]
                out.extend_from_slice(&(passphrase.len() as u32).to_be_bytes());
                out.extend_from_slice(passphrase);
                out.extend_from_slice(&idle_bound_ms.to_be_bytes());
            }
            Self::GetPassphrase { vault_name } => {
                out.push(TAG_GET_PASSPHRASE);
                encode_name(&mut out, vault_name)?;
            }
            Self::Lock { vault_name } => {
                out.push(TAG_LOCK);
                encode_optional_name(&mut out, vault_name.as_deref())?;
            }
            Self::Status => out.push(TAG_STATUS),
            Self::Shutdown => out.push(TAG_SHUTDOWN),
        }
        if out.len() > MAX_FRAME_BYTES {
            return Err(ProtocolError);
        }
        // If capacity was computed correctly, nothing after the first push
        // ever grew the allocation. This is the property the whole function
        // exists to guarantee, so it is asserted rather than only hoped for:
        // a future field added to a variant without updating
        // `encoded_capacity` would otherwise silently reopen exactly the bug
        // this rewrite closes.
        debug_assert_eq!(
            out.capacity(),
            capacity,
            "encode reallocated: a secret-bearing buffer may have left unscrubbed plaintext behind"
        );
        Ok(out)
    }

    /// The exact byte length [`Self::encode`] will write for this message,
    /// computed from unvalidated field lengths so it can be reserved
    /// *before* any field is validated or written. An over-estimate here
    /// would be safe (merely wasteful); an under-estimate would silently
    /// reopen the vulnerability [`Self::encode`]'s own comment describes, so
    /// this mirrors that function's field order exactly.
    fn encoded_capacity(&self) -> usize {
        const HEADER: usize = 1 + 1; // version + tag
        const NAME_PREFIX: usize = 1; // one length byte, per `encode_name`
        match self {
            Self::Ping | Self::Status | Self::Shutdown => HEADER,
            Self::Unlock {
                vault_name,
                passphrase,
                ..
            } => {
                HEADER
                    + NAME_PREFIX
                    + vault_name.len()
                    + 4 // passphrase length prefix
                    + passphrase.len()
                    + 8 // idle_bound_ms
            }
            Self::GetPassphrase { vault_name } => HEADER + NAME_PREFIX + vault_name.len(),
            Self::Lock { vault_name } => {
                let presence = 1;
                HEADER
                    + presence
                    + vault_name
                        .as_ref()
                        .map_or(0, |name| NAME_PREFIX + name.len())
            }
        }
    }

    /// Decode one payload previously produced by [`Self::encode`].
    ///
    /// # Errors
    ///
    /// Returns [`ProtocolError`] on a version mismatch, an unknown tag, a
    /// truncated field, or trailing bytes after a complete message. There is
    /// no tolerant arm: this wire has exactly one producer per version, this
    /// same crate, so a byte sequence that isn't exactly what
    /// [`Self::encode`] would have written is refused rather than guessed at.
    pub fn decode(input: &[u8]) -> Result<Self, ProtocolError> {
        if input.len() > MAX_FRAME_BYTES {
            return Err(ProtocolError);
        }
        let mut offset = 0;
        if read_u8(input, &mut offset)? != PROTOCOL_VERSION {
            return Err(ProtocolError);
        }
        let tag = read_u8(input, &mut offset)?;
        let request = match tag {
            TAG_PING => Self::Ping,
            TAG_UNLOCK => {
                let vault_name = decode_name(input, &mut offset)?;
                let length = read_u32(input, &mut offset)? as usize;
                if length > MAX_PASSPHRASE_BYTES {
                    return Err(ProtocolError);
                }
                let passphrase = Zeroizing::new(read_bytes(input, &mut offset, length)?.to_vec());
                let idle_bound_ms = read_u64(input, &mut offset)?;
                Self::Unlock {
                    vault_name,
                    passphrase,
                    idle_bound_ms,
                }
            }
            TAG_GET_PASSPHRASE => Self::GetPassphrase {
                vault_name: decode_name(input, &mut offset)?,
            },
            TAG_LOCK => Self::Lock {
                vault_name: decode_optional_name(input, &mut offset)?,
            },
            TAG_STATUS => Self::Status,
            TAG_SHUTDOWN => Self::Shutdown,
            _ => return Err(ProtocolError),
        };
        if offset != input.len() {
            return Err(ProtocolError);
        }
        Ok(request)
    }
}

/// Stable, closed reasons the agent refuses a request or reports a fault.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum AgentErrorCode {
    /// The request could not be decoded.
    Malformed = 1,
    /// The named vault has no retained passphrase, or it already expired.
    ///
    /// [`AgentResponse::NotRetained`] carries this same meaning for
    /// `GetPassphrase` specifically; this variant exists for symmetry on
    /// requests where "not retained" is an error rather than an ordinary
    /// answer, such as `Lock` naming a vault the agent never unlocked.
    NotRetained = 2,
    /// The agent could not complete the request for an internal reason.
    Internal = 3,
}

impl AgentErrorCode {
    const fn from_u8(value: u8) -> Option<Self> {
        match value {
            1 => Some(Self::Malformed),
            2 => Some(Self::NotRetained),
            3 => Some(Self::Internal),
            _ => None,
        }
    }
}

/// One vault's retention status, as reported by [`AgentResponse::Status`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VaultStatusEntry {
    /// The vault this entry describes.
    pub vault_name: String,
    /// Milliseconds remaining before the agent forgets this passphrase on
    /// its own, as of when the agent answered.
    pub remaining_ms: u64,
}

/// One response the agent sends back to a client.
pub enum AgentResponse {
    /// The request succeeded and carries no further data.
    Ok,
    /// The requested vault's retained passphrase.
    Passphrase(Zeroizing<Vec<u8>>),
    /// The requested vault has no retained, unexpired passphrase.
    NotRetained,
    /// Every vault the agent currently retains a passphrase for.
    Status(Vec<VaultStatusEntry>),
    /// The request was refused; see [`AgentErrorCode`].
    Err(AgentErrorCode),
}

impl Debug for AgentResponse {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Ok => formatter.write_str("AgentResponse::Ok"),
            Self::Passphrase(_) => formatter.write_str("AgentResponse::Passphrase(<redacted>)"),
            Self::NotRetained => formatter.write_str("AgentResponse::NotRetained"),
            Self::Status(entries) => formatter
                .debug_tuple("AgentResponse::Status")
                .field(entries)
                .finish(),
            Self::Err(code) => formatter
                .debug_tuple("AgentResponse::Err")
                .field(code)
                .finish(),
        }
    }
}

impl AgentResponse {
    /// Encode this response's payload (version and tag included).
    ///
    /// # Errors
    ///
    /// Returns [`ProtocolError`] if a carried passphrase or status list
    /// exceeds this protocol's bounds — see [`AgentRequest::encode`] for why
    /// this is defense in depth rather than an expected path.
    ///
    /// The returned buffer is [`Zeroizing`], for the same reason
    /// [`AgentRequest::encode`]'s is: a `Passphrase` response's encoding
    /// contains the retained passphrase in plaintext.
    pub fn encode(&self) -> Result<Zeroizing<Vec<u8>>, ProtocolError> {
        // See `AgentRequest::encode`'s comment: capacity is reserved exactly,
        // up front, so the allocation backing `out` never grows once a
        // `Passphrase` response's plaintext bytes are written to it.
        let capacity = self.encoded_capacity();
        let mut out = Zeroizing::new(Vec::with_capacity(capacity));
        out.push(PROTOCOL_VERSION);
        match self {
            Self::Ok => out.push(TAG_OK),
            Self::Passphrase(passphrase) => {
                out.push(TAG_PASSPHRASE);
                if passphrase.len() > MAX_PASSPHRASE_BYTES {
                    return Err(ProtocolError);
                }
                #[allow(clippy::cast_possible_truncation)]
                out.extend_from_slice(&(passphrase.len() as u32).to_be_bytes());
                out.extend_from_slice(passphrase);
            }
            Self::NotRetained => out.push(TAG_NOT_RETAINED),
            Self::Status(entries) => {
                out.push(TAG_STATUS_REPORT);
                if entries.len() > MAX_STATUS_VAULTS {
                    return Err(ProtocolError);
                }
                #[allow(clippy::cast_possible_truncation)]
                out.push(entries.len() as u8);
                for entry in entries {
                    encode_name(&mut out, &entry.vault_name)?;
                    out.extend_from_slice(&entry.remaining_ms.to_be_bytes());
                }
            }
            Self::Err(code) => {
                out.push(TAG_ERROR);
                out.push(*code as u8);
            }
        }
        // See `AgentRequest::encode`'s identical assertion.
        debug_assert_eq!(
            out.capacity(),
            capacity,
            "encode reallocated: a secret-bearing buffer may have left unscrubbed plaintext behind"
        );
        Ok(out)
    }

    /// The exact byte length [`Self::encode`] will write, mirroring
    /// [`AgentRequest::encoded_capacity`]'s contract and reasoning exactly.
    fn encoded_capacity(&self) -> usize {
        const HEADER: usize = 1 + 1; // version + tag
        const NAME_PREFIX: usize = 1;
        match self {
            Self::Ok | Self::NotRetained => HEADER,
            Self::Passphrase(passphrase) => HEADER + 4 + passphrase.len(),
            Self::Status(entries) => {
                let count_byte = 1;
                let entries_len: usize = entries
                    .iter()
                    .map(|entry| NAME_PREFIX + entry.vault_name.len() + 8)
                    .sum();
                HEADER + count_byte + entries_len
            }
            Self::Err(_) => HEADER + 1,
        }
    }

    /// Decode one payload previously produced by [`Self::encode`].
    ///
    /// # Errors
    ///
    /// Returns [`ProtocolError`] under the same closed conditions as
    /// [`AgentRequest::decode`].
    pub fn decode(input: &[u8]) -> Result<Self, ProtocolError> {
        if input.len()
            > MAX_FRAME_BYTES.max(1 + 1 + MAX_STATUS_VAULTS * (1 + MAX_VAULT_NAME_BYTES + 8))
        {
            return Err(ProtocolError);
        }
        let mut offset = 0;
        if read_u8(input, &mut offset)? != PROTOCOL_VERSION {
            return Err(ProtocolError);
        }
        let tag = read_u8(input, &mut offset)?;
        let response = match tag {
            TAG_OK => Self::Ok,
            TAG_PASSPHRASE => {
                let length = read_u32(input, &mut offset)? as usize;
                if length > MAX_PASSPHRASE_BYTES {
                    return Err(ProtocolError);
                }
                Self::Passphrase(Zeroizing::new(
                    read_bytes(input, &mut offset, length)?.to_vec(),
                ))
            }
            TAG_NOT_RETAINED => Self::NotRetained,
            TAG_STATUS_REPORT => {
                let count = usize::from(read_u8(input, &mut offset)?);
                if count > MAX_STATUS_VAULTS {
                    return Err(ProtocolError);
                }
                let mut entries = Vec::with_capacity(count);
                for _ in 0..count {
                    let vault_name = decode_name(input, &mut offset)?;
                    let remaining_ms = read_u64(input, &mut offset)?;
                    entries.push(VaultStatusEntry {
                        vault_name,
                        remaining_ms,
                    });
                }
                Self::Status(entries)
            }
            TAG_ERROR => {
                let code =
                    AgentErrorCode::from_u8(read_u8(input, &mut offset)?).ok_or(ProtocolError)?;
                Self::Err(code)
            }
            _ => return Err(ProtocolError),
        };
        if offset != input.len() {
            return Err(ProtocolError);
        }
        Ok(response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_formatting_covers_every_variant_and_redacts_secrets() {
        assert_eq!(format!("{:?}", AgentRequest::Ping), "AgentRequest::Ping");
        assert_eq!(
            format!(
                "{:?}",
                AgentRequest::GetPassphrase {
                    vault_name: "personal".to_owned()
                }
            ),
            "AgentRequest::GetPassphrase { vault_name: \"personal\" }"
        );
        assert_eq!(
            format!(
                "{:?}",
                AgentRequest::Lock {
                    vault_name: Some("personal".to_owned())
                }
            ),
            "AgentRequest::Lock { vault_name: Some(\"personal\") }"
        );
        assert_eq!(
            format!("{:?}", AgentRequest::Status),
            "AgentRequest::Status"
        );
        assert_eq!(
            format!("{:?}", AgentRequest::Shutdown),
            "AgentRequest::Shutdown"
        );

        assert_eq!(format!("{:?}", AgentResponse::Ok), "AgentResponse::Ok");
        assert_eq!(
            format!(
                "{:?}",
                AgentResponse::Passphrase(Zeroizing::new(b"secret".to_vec()))
            ),
            "AgentResponse::Passphrase(<redacted>)"
        );
        assert_eq!(
            format!("{:?}", AgentResponse::NotRetained),
            "AgentResponse::NotRetained"
        );
        assert_eq!(
            format!(
                "{:?}",
                AgentResponse::Status(vec![VaultStatusEntry {
                    vault_name: "personal".to_owned(),
                    remaining_ms: 5_000,
                }])
            ),
            "AgentResponse::Status([VaultStatusEntry { vault_name: \"personal\", remaining_ms: 5000 }])"
        );
        assert_eq!(
            format!("{:?}", AgentResponse::Err(AgentErrorCode::Internal)),
            "AgentResponse::Err(Internal)"
        );
    }

    #[test]
    fn ping_round_trips() {
        let encoded = AgentRequest::Ping.encode().unwrap();
        assert!(matches!(
            AgentRequest::decode(&encoded).unwrap(),
            AgentRequest::Ping
        ));
    }

    #[test]
    fn unlock_round_trips_and_redacts_debug() {
        let request = AgentRequest::Unlock {
            vault_name: "personal".to_owned(),
            passphrase: Zeroizing::new(b"correct horse battery staple".to_vec()),
            idle_bound_ms: 300_000,
        };
        assert_eq!(
            format!("{request:?}"),
            "AgentRequest::Unlock { vault_name: \"personal\", passphrase: \"<redacted>\", .. }"
        );
        let encoded = request.encode().unwrap();
        match AgentRequest::decode(&encoded).unwrap() {
            AgentRequest::Unlock {
                vault_name,
                passphrase,
                idle_bound_ms,
            } => {
                assert_eq!(vault_name, "personal");
                assert_eq!(&*passphrase, b"correct horse battery staple");
                assert_eq!(idle_bound_ms, 300_000);
            }
            other => panic!("wrong variant: {other:?}"),
        }
    }

    #[test]
    fn get_passphrase_round_trips() {
        let encoded = AgentRequest::GetPassphrase {
            vault_name: "work".to_owned(),
        }
        .encode()
        .unwrap();
        match AgentRequest::decode(&encoded).unwrap() {
            AgentRequest::GetPassphrase { vault_name } => assert_eq!(vault_name, "work"),
            other => panic!("wrong variant: {other:?}"),
        }
    }

    #[test]
    fn lock_round_trips_both_forms() {
        for name in [None, Some("personal".to_owned())] {
            let encoded = AgentRequest::Lock {
                vault_name: name.clone(),
            }
            .encode()
            .unwrap();
            match AgentRequest::decode(&encoded).unwrap() {
                AgentRequest::Lock { vault_name } => assert_eq!(vault_name, name),
                other => panic!("wrong variant: {other:?}"),
            }
        }
    }

    #[test]
    fn status_and_shutdown_round_trip() {
        assert!(matches!(
            AgentRequest::decode(&AgentRequest::Status.encode().unwrap()).unwrap(),
            AgentRequest::Status
        ));
        assert!(matches!(
            AgentRequest::decode(&AgentRequest::Shutdown.encode().unwrap()).unwrap(),
            AgentRequest::Shutdown
        ));
    }

    #[test]
    fn responses_round_trip() {
        assert!(matches!(
            AgentResponse::decode(&AgentResponse::Ok.encode().unwrap()).unwrap(),
            AgentResponse::Ok
        ));
        assert!(matches!(
            AgentResponse::decode(&AgentResponse::NotRetained.encode().unwrap()).unwrap(),
            AgentResponse::NotRetained
        ));
        let passphrase = AgentResponse::Passphrase(Zeroizing::new(b"a passphrase value".to_vec()));
        match AgentResponse::decode(&passphrase.encode().unwrap()).unwrap() {
            AgentResponse::Passphrase(value) => assert_eq!(&*value, b"a passphrase value"),
            other => panic!("wrong variant: {other:?}"),
        }
        let status = AgentResponse::Status(vec![
            VaultStatusEntry {
                vault_name: "personal".to_owned(),
                remaining_ms: 12_000,
            },
            VaultStatusEntry {
                vault_name: "work".to_owned(),
                remaining_ms: 0,
            },
        ]);
        match AgentResponse::decode(&status.encode().unwrap()).unwrap() {
            AgentResponse::Status(entries) => {
                assert_eq!(entries.len(), 2);
                assert_eq!(entries[0].vault_name, "personal");
                assert_eq!(entries[0].remaining_ms, 12_000);
                assert_eq!(entries[1].vault_name, "work");
                assert_eq!(entries[1].remaining_ms, 0);
            }
            other => panic!("wrong variant: {other:?}"),
        }
        for code in [
            AgentErrorCode::Malformed,
            AgentErrorCode::NotRetained,
            AgentErrorCode::Internal,
        ] {
            let response = AgentResponse::Err(code);
            match AgentResponse::decode(&response.encode().unwrap()).unwrap() {
                AgentResponse::Err(decoded) => assert_eq!(decoded, code),
                other => panic!("wrong variant: {other:?}"),
            }
        }
    }

    #[test]
    fn malformed_bytes_are_refused_not_guessed_at() {
        assert_eq!(AgentRequest::decode(&[]).unwrap_err(), ProtocolError);
        assert_eq!(
            AgentRequest::decode(&[PROTOCOL_VERSION + 1, TAG_PING]).unwrap_err(),
            ProtocolError
        );
        assert_eq!(
            AgentRequest::decode(&[PROTOCOL_VERSION, 0xEE]).unwrap_err(),
            ProtocolError
        );
        // Trailing garbage after a complete Ping is refused, not ignored.
        assert_eq!(
            AgentRequest::decode(&[PROTOCOL_VERSION, TAG_PING, 0]).unwrap_err(),
            ProtocolError
        );
        // A truncated Unlock (name present, passphrase length missing).
        let mut truncated = vec![PROTOCOL_VERSION, TAG_UNLOCK, 4];
        truncated.extend_from_slice(b"work");
        assert_eq!(AgentRequest::decode(&truncated).unwrap_err(), ProtocolError);
        assert_eq!(
            AgentResponse::decode(&[PROTOCOL_VERSION, TAG_ERROR, 0xEE]).unwrap_err(),
            ProtocolError
        );
    }

    /// A hand-crafted frame — never produced by [`AgentRequest::encode`] —
    /// naming a vault outside the allowed character set is refused at
    /// decode time.
    ///
    /// This is the attack this test exists to close: the socket's peer check
    /// (`vault-pm-agent-host::peer`) authenticates only "the same local
    /// user," never "the genuine `vault-pm` binary," so any same-user
    /// process can write raw bytes to the socket. Without this check, a
    /// crafted `Unlock` naming a vault such as `personal","evil":true` (or
    /// containing a raw terminal escape sequence) would be retained
    /// verbatim and later rendered, unescaped, into `agent status`'s
    /// plain-text and `--json` output.
    #[test]
    fn a_vault_name_outside_the_allowed_character_set_is_refused_at_decode_time() {
        for hostile in [
            "personal\",\"evil\":true".as_bytes(),
            b"personal\"".as_slice(),
            b"personal\\",
            b"personal\n",
            b"personal\x1b[31m",
            b" personal",
            b"-leading-dash",
            b"_leading-underscore",
        ] {
            let mut frame = vec![PROTOCOL_VERSION, TAG_GET_PASSPHRASE];
            #[allow(clippy::cast_possible_truncation)]
            frame.push(hostile.len() as u8);
            frame.extend_from_slice(hostile);
            assert!(
                matches!(AgentRequest::decode(&frame), Err(ProtocolError)),
                "{hostile:?} must be refused"
            );
        }
        // The one legitimate case adjacent to the refused ones above: a
        // non-leading `-`/`_` is fine, matching `ConfigName`'s own rule.
        let mut legitimate = vec![PROTOCOL_VERSION, TAG_GET_PASSPHRASE, 12];
        legitimate.extend_from_slice(b"work-laptop_");
        assert!(AgentRequest::decode(&legitimate).is_ok());
    }

    #[test]
    fn oversized_fields_are_refused_at_encode_time() {
        let long_name = "a".repeat(MAX_VAULT_NAME_BYTES + 1);
        assert!(matches!(
            AgentRequest::GetPassphrase {
                vault_name: long_name
            }
            .encode(),
            Err(ProtocolError)
        ));
        let long_passphrase = vec![b'x'; MAX_PASSPHRASE_BYTES + 1];
        assert!(matches!(
            AgentRequest::Unlock {
                vault_name: "personal".to_owned(),
                passphrase: Zeroizing::new(long_passphrase),
                idle_bound_ms: 1_000,
            }
            .encode(),
            Err(ProtocolError)
        ));
        let empty_name = String::new();
        assert!(matches!(
            AgentRequest::GetPassphrase {
                vault_name: empty_name
            }
            .encode(),
            Err(ProtocolError)
        ));
    }

    #[test]
    fn oversized_status_report_is_refused_at_encode_time() {
        let entries: Vec<_> = (0..=MAX_STATUS_VAULTS)
            .map(|index| VaultStatusEntry {
                vault_name: format!("v{index}"),
                remaining_ms: 0,
            })
            .collect();
        assert!(matches!(
            AgentResponse::Status(entries).encode(),
            Err(ProtocolError)
        ));
    }

    #[test]
    fn maximum_size_frames_encode_and_round_trip() {
        let request = AgentRequest::Unlock {
            vault_name: "a".repeat(MAX_VAULT_NAME_BYTES),
            passphrase: Zeroizing::new(vec![b'p'; MAX_PASSPHRASE_BYTES]),
            idle_bound_ms: u64::MAX,
        };
        let encoded = request.encode().unwrap();
        assert!(encoded.len() <= MAX_FRAME_BYTES);
        assert!(AgentRequest::decode(&encoded).is_ok());
    }

    /// `encode` never reallocates while a secret is already resident in the
    /// buffer, across a sweep of realistic passphrase and vault-name
    /// lengths.
    ///
    /// This is the direct, empirical form of the security-review finding
    /// this test closes: a buffer that starts at length 1 and grows via
    /// `push`/`extend_from_slice` reallocates for the overwhelming majority
    /// of real passphrase lengths, and `Zeroizing` only wipes the
    /// allocation it *currently* holds at drop — never an allocation the
    /// buffer already grew out of and the global allocator already freed
    /// unscrubbed. `Vec::capacity()` staying exactly equal to the reserved
    /// upfront capacity, for every length in this sweep, is what proves no
    /// such reallocation happened.
    #[test]
    fn encode_never_reallocates_a_buffer_already_holding_a_secret() {
        for vault_name_len in [1_usize, 8, 32, MAX_VAULT_NAME_BYTES] {
            for passphrase_len in 0..=200_usize {
                let request = AgentRequest::Unlock {
                    vault_name: "a".repeat(vault_name_len),
                    passphrase: Zeroizing::new(vec![b'p'; passphrase_len]),
                    idle_bound_ms: 300_000,
                };
                let capacity_before = request.encoded_capacity();
                let encoded = request.encode().unwrap();
                assert_eq!(
                    encoded.len(),
                    capacity_before,
                    "vault_name_len={vault_name_len} passphrase_len={passphrase_len}"
                );
                assert_eq!(
                    encoded.capacity(),
                    capacity_before,
                    "encode reallocated for vault_name_len={vault_name_len} \
                     passphrase_len={passphrase_len}: a secret-bearing buffer left \
                     unscrubbed plaintext behind"
                );
            }
        }

        // Same property for the response side's `Passphrase` arm.
        for passphrase_len in 0..=200_usize {
            let response = AgentResponse::Passphrase(Zeroizing::new(vec![b'p'; passphrase_len]));
            let capacity_before = response.encoded_capacity();
            let encoded = response.encode().unwrap();
            assert_eq!(
                encoded.len(),
                capacity_before,
                "passphrase_len={passphrase_len}"
            );
            assert_eq!(
                encoded.capacity(),
                capacity_before,
                "encode reallocated for passphrase_len={passphrase_len}"
            );
        }
    }
}
