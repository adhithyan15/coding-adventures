//! # coding_adventures_double_ratchet — Signal Double Ratchet Algorithm
//!
//! The Double Ratchet algorithm is the **heart of Signal's message encryption**.
//! It provides two security properties that are very hard to achieve together:
//!
//! 1. **Forward secrecy** — past messages stay secret even if your long-term key
//!    is later compromised. Each message uses a fresh, one-time key.
//! 2. **Break-in recovery** (post-compromise security) — if an attacker steals
//!    your current session state, they cannot read your *future* messages
//!    indefinitely. The ratchet "heals" with every new DH exchange.
//!
//! ## The Two Interlocking Ratchets
//!
//! ### Ratchet 1 — The Symmetric KDF Chain
//!
//! Imagine a hash chain where you can only move forward, never backward:
//!
//! ```text
//! CK₀ → CK₁ → CK₂ → CK₃ → …
//!         ↓      ↓      ↓
//!        MK₁   MK₂   MK₃   ← per-message encryption keys
//! ```
//!
//! At each step we call `HMAC-SHA256(CKₙ, 0x01)` to get the next chain key,
//! and `HMAC-SHA256(CKₙ, 0x02)` to derive the message key. Deleting `CKₙ`
//! after use gives forward secrecy: an attacker who captures `CKₙ₊₁` cannot
//! reverse the HMAC to recover `CKₙ` or the message keys before it.
//!
//! ### Ratchet 2 — The DH Ratchet
//!
//! Every time one party generates a new X25519 key pair and sends the public
//! half in a message header, both sides advance the root chain:
//!
//! ```text
//! RK₀ ──DH(Alice₁, Bob₀)──► RK₁, CKs₁
//! RK₁ ──DH(Bob₁,  Alice₁)──► RK₂, CKr₁
//! RK₂ ──DH(Alice₂, Bob₁) ──► RK₃, CKs₂
//! …
//! ```
//!
//! Each DH output is mixed with the root key via HKDF ("WhisperRatchet").
//! Because X25519 ephemeral keys are freshly generated, each new DH output is
//! unpredictable even to an attacker who captured the previous root key. This
//! is the break-in recovery mechanism.
//!
//! ## Message Flow (Alice → Bob → Alice)
//!
//! ```text
//!   Alice                               Bob
//!   ─────                               ───
//!   init_alice(SK, bob_ratchet_pub)     init_bob(SK, bob_ratchet_kp)
//!
//!   encrypt(m₀) ──────────────────────► decrypt(m₀)   [DH ratchet triggers]
//!   encrypt(m₁) ──────────────────────► decrypt(m₁)   [symmetric ratchet]
//!              ◄──── encrypt(r₀) ─────  decrypt(r₀)   [DH ratchet triggers Alice]
//! ```
//!
//! ## Out-of-Order Delivery
//!
//! Messages sometimes arrive out of order over the network. The ratchet handles
//! this by **skipping ahead** when it sees a higher message counter, storing the
//! unused message keys in a map indexed by `(DH_ratchet_pub, message_number)`.
//! When the out-of-order message finally arrives, we look up its key and decrypt.
//!
//! We cap this cache at `MAX_SKIP = 1000` to prevent a DoS attack where a
//! malicious message with `n = 1_000_000` would cause us to derive and store a
//! million keys before noticing the message is fake.
//!
//! ## References
//! - Signal Double Ratchet Specification: <https://signal.org/docs/specifications/doubleratchet/>
//! - Trevor Perrin, Moxie Marlinspike — "The Double Ratchet Algorithm" (2016)

use std::collections::HashMap;

use coding_adventures_chacha20_poly1305::{aead_decrypt, aead_encrypt};
use coding_adventures_curve25519::{x25519, x25519_public_key};
use coding_adventures_hkdf::{hkdf, HashAlgorithm};
use coding_adventures_hmac::hmac_sha256;
use coding_adventures_zeroize::{Zeroize, Zeroizing};

// ═══════════════════════════════════════════════════════════════════════════════
// SECTION 1: CONSTANTS
// ═══════════════════════════════════════════════════════════════════════════════

/// Maximum number of future message keys to cache at once per ratchet step.
///
/// If a peer's message counter is more than `MAX_SKIP` ahead of ours, we
/// reject the message. A legitimate peer never sends that far ahead, so
/// seeing it is a sign of a bug or a DoS attempt.
pub const MAX_SKIP: u32 = 1000;

/// Maximum total number of skipped message keys across all ratchet epochs.
///
/// Without this bound an adversary could accumulate up to `MAX_SKIP` entries
/// per DH ratchet step, indefinitely, causing unbounded memory growth.
pub const MAX_SKIPPED_KEYS_TOTAL: usize = 5_000;

/// Byte length of an encoded `MessageHeader`.
///
/// Layout: DH_ratchet_pub (32) ‖ previous_chain_count (4 LE) ‖ message_n (4 LE)
pub const HEADER_LEN: usize = 40;

// ═══════════════════════════════════════════════════════════════════════════════
// SECTION 2: KEY PAIR
// ═══════════════════════════════════════════════════════════════════════════════

/// An X25519 Diffie-Hellman ratchet key pair.
///
/// The secret key is **wiped from memory when the pair is dropped**, so it
/// is safe to keep ratchet states in memory without worrying about a
/// compromised memory dump exposing old ratchet keys.
pub struct KeyPair {
    /// Public half — safe to send in message headers.
    pub public: [u8; 32],
    /// Secret half — never leaves this struct except via `x25519`.
    secret: [u8; 32],
}

impl Zeroize for KeyPair {
    fn zeroize(&mut self) {
        self.secret.zeroize();
    }
}

impl Drop for KeyPair {
    fn drop(&mut self) {
        self.zeroize();
    }
}

impl KeyPair {
    /// Construct a `KeyPair` from a known secret (e.g. Bob's pre-shared ratchet key).
    ///
    /// The public key is derived via `scalar × basepoint`. The scalar is **not**
    /// re-clamped here — callers must ensure the secret is already RFC 7748–clamped.
    pub fn from_secret(secret: [u8; 32]) -> Self {
        let public = x25519_public_key(&secret);
        KeyPair { public, secret }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// SECTION 3: MESSAGE TYPES
// ═══════════════════════════════════════════════════════════════════════════════

/// Metadata prepended to every encrypted message.
///
/// The header travels in plaintext so the recipient can perform the DH
/// ratchet step and locate any skipped message keys before decrypting.
#[derive(Clone, Debug, PartialEq)]
pub struct MessageHeader {
    /// Sender's current DH ratchet public key.
    pub dh: [u8; 32],
    /// Number of messages sent in the **previous** sending chain.
    ///
    /// When a new DH ratchet step occurs, this counter tells the recipient
    /// how many messages to skip in the old chain before switching.
    pub pn: u32,
    /// This message's index in the current sending chain (0-based).
    pub n: u32,
}

/// A complete encrypted Double Ratchet message.
///
/// The `ciphertext` field contains the encrypted payload with the 16-byte
/// Poly1305 authentication tag appended at the end:
///
/// ```text
/// ciphertext = ChaCha20_Encrypt(plaintext) ‖ Poly1305_Tag(…)
/// ```
#[derive(Debug, PartialEq)]
pub struct Message {
    /// Decoded message header (sender's DH ratchet pub, chain counters).
    pub header: MessageHeader,
    /// Encrypted payload bytes followed by the 16-byte authentication tag.
    pub ciphertext: Vec<u8>,
}

// ═══════════════════════════════════════════════════════════════════════════════
// SECTION 4: RATCHET STATE
// ═══════════════════════════════════════════════════════════════════════════════

/// The full mutable state of one side of a Double Ratchet session.
///
/// Fields follow the naming convention from the Signal spec:
/// - `dhs` — DH **s**ending ratchet key pair
/// - `dhr` — DH **r**eceiving ratchet public key (the remote peer's current key)
/// - `rk`  — **r**oot **k**ey; shared by both parties, updated each DH ratchet step
/// - `cks` — **c**hain **k**ey for **s**ending
/// - `ckr` — **c**hain **k**ey for **r**eceiving
/// - `ns`  — next send counter
/// - `nr`  — next receive counter
/// - `pn`  — previous chain message count
/// - `mk_skipped` — skipped message keys, indexed by (ratchet_pub, message_n)
pub struct RatchetState {
    dhs: KeyPair,
    dhr: Option<[u8; 32]>,
    rk: [u8; 32],
    cks: Option<[u8; 32]>,
    ckr: Option<[u8; 32]>,
    ns: u32,
    nr: u32,
    pn: u32,
    mk_skipped: HashMap<([u8; 32], u32), [u8; 32]>,
}

impl Drop for RatchetState {
    fn drop(&mut self) {
        // Zeroize all key material. Rust will drop `dhs` (which has its own
        // Zeroize/Drop impl) automatically after this function returns.
        self.rk.zeroize();
        self.cks.zeroize(); // Option<[u8;32]> implements Zeroize
        self.ckr.zeroize();
        for v in self.mk_skipped.values_mut() {
            v.zeroize();
        }
        self.mk_skipped.clear();
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// SECTION 5: ERROR TYPE
// ═══════════════════════════════════════════════════════════════════════════════

/// Errors that can occur during ratchet encryption or decryption.
#[derive(Debug, PartialEq)]
pub enum RatchetError {
    /// Authentication or decryption failed (wrong key, tampered ciphertext).
    DecryptionFailed,
    /// A message's counter is more than `MAX_SKIP` ahead. Likely an attack.
    TooManySkippedMessages,
    /// No receiving chain key exists yet; we haven't received the first message.
    NoReceivingChain,
    /// No sending chain key exists yet; `ratchet_init_alice` wasn't called.
    NoSendingChain,
    /// HKDF derivation returned an error (shouldn't happen for valid inputs).
    KdfError,
    /// HMAC-SHA256 returned an error (shouldn't happen for valid inputs).
    HmacError,
}

impl std::fmt::Display for RatchetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RatchetError::DecryptionFailed => write!(f, "decryption failed or authentication tag invalid"),
            RatchetError::TooManySkippedMessages => write!(f, "message counter too far ahead (possible DoS)"),
            RatchetError::NoReceivingChain => write!(f, "no receiving chain key — not yet initialized"),
            RatchetError::NoSendingChain => write!(f, "no sending chain key — not yet initialized"),
            RatchetError::KdfError => write!(f, "HKDF key derivation failed"),
            RatchetError::HmacError => write!(f, "HMAC-SHA256 failed"),
        }
    }
}

impl std::error::Error for RatchetError {}

// ═══════════════════════════════════════════════════════════════════════════════
// SECTION 6: INTERNAL KDF HELPERS
// ═══════════════════════════════════════════════════════════════════════════════

/// KDF_RK — advance the root chain using a DH output.
///
/// The Signal spec defines this as a 2-output KDF:
/// ```text
/// RK', CK = KDF_RK(RK, DH_out)
///         = HKDF(salt=RK, ikm=DH_out, info="WhisperRatchet", len=64)
/// ```
///
/// We split the 64-byte output: first 32 bytes → new root key, last 32 → chain key.
/// The HKDF output is held in a `Zeroizing<>` wrapper so it's wiped even if we
/// return an error mid-way.
fn kdf_rk(rk: &[u8; 32], dh_out: &[u8]) -> Result<([u8; 32], [u8; 32]), RatchetError> {
    let okm = Zeroizing::new(
        hkdf(rk, dh_out, b"WhisperRatchet", 64, HashAlgorithm::Sha256)
            .map_err(|_| RatchetError::KdfError)?,
    );
    let mut new_rk = [0u8; 32];
    let mut new_ck = [0u8; 32];
    new_rk.copy_from_slice(&okm[..32]);
    new_ck.copy_from_slice(&okm[32..]);
    Ok((new_rk, new_ck))
}

/// KDF_CK — advance a symmetric chain key to derive a message key.
///
/// Uses two HMAC-SHA256 calls with different input constants:
/// ```text
/// MK      = HMAC-SHA256(CK, 0x02)   ← message encryption key
/// new_CK  = HMAC-SHA256(CK, 0x01)   ← next chain key
/// ```
///
/// The constant 0x01/0x02 domain-separates the two derivations, preventing
/// the message key from leaking the next chain key (or vice versa).
fn kdf_ck(ck: &[u8; 32]) -> Result<([u8; 32], [u8; 32]), RatchetError> {
    let mk = hmac_sha256(ck, &[0x02]).map_err(|_| RatchetError::HmacError)?;
    let new_ck_arr = hmac_sha256(ck, &[0x01]).map_err(|_| RatchetError::HmacError)?;
    Ok((new_ck_arr, mk))
}

/// Expand a 32-byte message key into an encryption key (32 bytes) and a nonce (12 bytes).
///
/// The HKDF output is wiped after use via `Zeroizing<>`.
///
/// Layout of the 44-byte expansion:
/// ```text
/// okm[0..32]  = ChaCha20 encryption key
/// okm[32..44] = ChaCha20-Poly1305 nonce
/// ```
fn expand_message_key(mk: &[u8; 32]) -> Result<([u8; 32], [u8; 12]), RatchetError> {
    let okm = Zeroizing::new(
        hkdf(&[0u8; 32], mk, b"WhisperMessageKeys", 44, HashAlgorithm::Sha256)
            .map_err(|_| RatchetError::KdfError)?,
    );
    let mut enc_key = [0u8; 32];
    let mut iv = [0u8; 12];
    enc_key.copy_from_slice(&okm[..32]);
    iv.copy_from_slice(&okm[32..44]);
    Ok((enc_key, iv))
}

// ═══════════════════════════════════════════════════════════════════════════════
// SECTION 7: HEADER ENCODING
// ═══════════════════════════════════════════════════════════════════════════════

/// Serialize a `MessageHeader` to the 40-byte on-wire format.
///
/// ```text
/// [0..32]  = DH ratchet public key (32 bytes)
/// [32..36] = previous chain count (u32 little-endian)
/// [36..40] = message index (u32 little-endian)
/// ```
pub fn encode_header(h: &MessageHeader) -> [u8; HEADER_LEN] {
    let mut out = [0u8; HEADER_LEN];
    out[..32].copy_from_slice(&h.dh);
    out[32..36].copy_from_slice(&h.pn.to_le_bytes());
    out[36..40].copy_from_slice(&h.n.to_le_bytes());
    out
}

/// Deserialize a 40-byte buffer into a `MessageHeader`.
pub fn decode_header(bytes: &[u8; HEADER_LEN]) -> MessageHeader {
    let mut dh = [0u8; 32];
    dh.copy_from_slice(&bytes[..32]);
    let pn = u32::from_le_bytes(bytes[32..36].try_into().unwrap());
    let n = u32::from_le_bytes(bytes[36..40].try_into().unwrap());
    MessageHeader { dh, pn, n }
}

// ═══════════════════════════════════════════════════════════════════════════════
// SECTION 8: KEY PAIR GENERATION
// ═══════════════════════════════════════════════════════════════════════════════

/// Generate a fresh X25519 ratchet key pair with RFC 7748 clamping.
pub fn generate_ratchet_keypair() -> KeyPair {
    let mut secret = Zeroizing::new([0u8; 32]);
    getrandom::getrandom(&mut *secret).expect("getrandom failed");
    // RFC 7748 §5 scalar clamping: ensures the scalar is a valid X25519 scalar
    // and the result point lies in the prime-order subgroup.
    secret[0] &= 248; // clear bits 0-2
    secret[31] &= 127; // clear bit 7
    secret[31] |= 64; // set bit 6
    KeyPair::from_secret(*secret) // copy out; Zeroizing drops and wipes the seed
}

// ═══════════════════════════════════════════════════════════════════════════════
// SECTION 9: SESSION INITIALIZATION
// ═══════════════════════════════════════════════════════════════════════════════

/// Initialize the ratchet state for Alice (the message initiator).
///
/// Alice already knows Bob's ratchet public key from the X3DH key agreement.
/// She immediately performs the first DH ratchet step to establish her sending
/// chain, so she can send messages right away without waiting for a reply.
///
/// ```text
///   Alice_DHS  ← fresh ratchet key pair (generated here)
///   Alice_DHR  ← Bob's ratchet public key (from X3DH output)
///   RK, CKs    ← KDF_RK(shared_key, DH(Alice_DHS.secret, Alice_DHR))
/// ```
pub fn ratchet_init_alice(shared_key: &[u8; 32], bob_ratchet_pub: &[u8; 32]) -> RatchetState {
    let dhs = generate_ratchet_keypair();
    let dh_out = Zeroizing::new(x25519(&dhs.secret, bob_ratchet_pub));
    let (rk, cks) = kdf_rk(shared_key, &*dh_out).expect("kdf_rk failed on init");
    RatchetState {
        dhs,
        dhr: Some(*bob_ratchet_pub),
        rk,
        cks: Some(cks),
        ckr: None,
        ns: 0,
        nr: 0,
        pn: 0,
        mk_skipped: HashMap::new(),
    }
}

/// Initialize the ratchet state for Bob (the message responder).
///
/// Bob waits for Alice's first message. He doesn't have a sending chain yet —
/// that will be established once he performs a DH ratchet step on receipt.
///
/// ```text
///   Bob_DHS  ← his pre-published ratchet key pair (known to Alice via X3DH)
///   RK       ← the X3DH shared secret (no DH ratchet step yet)
///   CKs/CKr  ← None (no chains until first message arrives)
/// ```
pub fn ratchet_init_bob(shared_key: &[u8; 32], ratchet_keypair: KeyPair) -> RatchetState {
    RatchetState {
        dhs: ratchet_keypair,
        dhr: None,
        rk: *shared_key,
        cks: None,
        ckr: None,
        ns: 0,
        nr: 0,
        pn: 0,
        mk_skipped: HashMap::new(),
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// SECTION 10: ENCRYPTION
// ═══════════════════════════════════════════════════════════════════════════════

/// Encrypt a plaintext using the current sending chain.
///
/// Steps:
/// 1. Advance the symmetric KDF chain → get `(new_CKs, MK)`
/// 2. Build the message header with the current DH ratchet public key
/// 3. Expand `MK` → `(enc_key, nonce)` via `expand_message_key`
/// 4. AEAD-encrypt with `AAD = outer_aad ‖ header_bytes`
/// 5. Return `Message { header, ciphertext: encrypted_bytes ‖ tag }`
///
/// # Errors
///
/// Returns `RatchetError::NoSendingChain` if called on a freshly-initialized
/// Bob state before receiving any message (which triggers the DH ratchet).
pub fn ratchet_encrypt(
    state: &mut RatchetState,
    plaintext: &[u8],
    aad: &[u8],
) -> Result<Message, RatchetError> {
    let cks = state.cks.as_ref().ok_or(RatchetError::NoSendingChain)?;
    let (new_cks, mk) = kdf_ck(cks)?;
    state.cks = Some(new_cks);

    let header = MessageHeader { dh: state.dhs.public, pn: state.pn, n: state.ns };
    state.ns += 1;

    let (enc_key, iv) = expand_message_key(&mk)?;
    let header_bytes = encode_header(&header);

    // AAD binds the header to the ciphertext: tampering with the header
    // will cause the Poly1305 tag verification to fail.
    let mut full_aad = Vec::with_capacity(aad.len() + HEADER_LEN);
    full_aad.extend_from_slice(aad);
    full_aad.extend_from_slice(&header_bytes);

    let (ct, tag) = aead_encrypt(plaintext, &enc_key, &iv, &full_aad);
    let mut ciphertext = ct;
    ciphertext.extend_from_slice(&tag);

    Ok(Message { header, ciphertext })
}

// ═══════════════════════════════════════════════════════════════════════════════
// SECTION 11: DECRYPTION HELPERS
// ═══════════════════════════════════════════════════════════════════════════════

/// Try to decrypt using a previously stored (skipped) message key.
///
/// If the `(header.dh, header.n)` key exists in `mk_skipped`, removes it from
/// the map and attempts decryption. Returns `None` if the key isn't cached or
/// if decryption fails.
fn try_skipped_message_key(
    mk_skipped: &mut HashMap<([u8; 32], u32), [u8; 32]>,
    header: &MessageHeader,
    ciphertext: &[u8],
    aad: &[u8],
) -> Option<Vec<u8>> {
    let key = (header.dh, header.n);
    let mut mk = mk_skipped.remove(&key)?;

    let header_bytes = encode_header(header);
    let mut full_aad = Vec::with_capacity(aad.len() + HEADER_LEN);
    full_aad.extend_from_slice(aad);
    full_aad.extend_from_slice(&header_bytes);

    let result = if let Ok((enc_key, iv)) = expand_message_key(&mk) {
        if ciphertext.len() >= 16 {
            let ct = &ciphertext[..ciphertext.len() - 16];
            let tag: [u8; 16] = ciphertext[ciphertext.len() - 16..].try_into().ok()?;
            aead_decrypt(ct, &enc_key, &iv, &full_aad, &tag)
        } else {
            None
        }
    } else {
        None
    };

    // Zeroize the message key whether decryption succeeded or failed.
    mk.zeroize();
    result
}

/// Store message keys for future out-of-order messages up to `until`.
///
/// Advances the receiving chain key from `state.nr` to `until`, caching each
/// derived message key in `mk_skipped`. Enforces `MAX_SKIP` to prevent DoS.
fn skip_message_keys(state: &mut RatchetState, until: u32) -> Result<(), RatchetError> {
    // Per-step limit: reject if the jump is too large.
    let keys_to_add = until.saturating_sub(state.nr) as usize;
    if keys_to_add > MAX_SKIP as usize {
        return Err(RatchetError::TooManySkippedMessages);
    }

    // Global limit: prevent unbounded cache growth across all DH ratchet epochs.
    if state.mk_skipped.len().saturating_add(keys_to_add) > MAX_SKIPPED_KEYS_TOTAL {
        return Err(RatchetError::TooManySkippedMessages);
    }

    if let Some(mut ck) = state.ckr {
        // dhr is always Some when ckr is Some (invariant maintained by dh_ratchet_step).
        let dhr = state.dhr.ok_or(RatchetError::NoReceivingChain)?;
        while state.nr < until {
            let (new_ck, mk) = kdf_ck(&ck)?;
            state.mk_skipped.insert((dhr, state.nr), mk);
            ck = new_ck;
            state.nr += 1;
        }
        state.ckr = Some(ck);
    }
    Ok(())
}

/// Perform a DH ratchet step on the receiving side.
///
/// Called when the incoming message header contains a new DH ratchet public key.
/// Two DH operations are performed in sequence:
///
/// 1. **Receiving ratchet**: mix the peer's new key with our current DHS secret
///    → derive a receiving chain key.
/// 2. **Sending ratchet**: generate a new DHS key pair; mix with the peer's new key
///    → derive a sending chain key.
///
/// After this step, the old `dhs` key pair is dropped and its secret is
/// zeroized. The two Zeroizing wrappers ensure the DH outputs are wiped.
fn dh_ratchet_step(state: &mut RatchetState, header_dh: &[u8; 32]) -> Result<(), RatchetError> {
    state.pn = state.ns;
    state.ns = 0;
    state.nr = 0;
    state.dhr = Some(*header_dh);

    // First DH: establish the receiving chain with the peer's new key.
    let dh_out = Zeroizing::new(x25519(&state.dhs.secret, header_dh));
    let (new_rk, new_ckr) = kdf_rk(&state.rk, &*dh_out)?;
    state.rk = new_rk;
    state.ckr = Some(new_ckr);

    // Second DH: generate our new ratchet key pair and establish the sending chain.
    // The old `dhs` is replaced here; its Drop impl zeroizes the old secret.
    let new_dhs = generate_ratchet_keypair();
    let dh_out2 = Zeroizing::new(x25519(&new_dhs.secret, header_dh));
    let (new_rk2, new_cks) = kdf_rk(&state.rk, &*dh_out2)?;
    state.rk = new_rk2;
    state.cks = Some(new_cks);
    state.dhs = new_dhs; // old dhs dropped (and thus zeroized) here

    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════════
// SECTION 12: DECRYPTION
// ═══════════════════════════════════════════════════════════════════════════════

/// Decrypt a `Message`, advancing the ratchet as needed.
///
/// Decision tree:
/// 1. If `(header.dh, header.n)` is in `mk_skipped`: use that key, skip the ratchet.
/// 2. If `header.dh` is a new public key: perform a DH ratchet step.
/// 3. If `header.n > state.nr`: skip ahead, caching the missing message keys.
/// 4. Derive the current message key and decrypt.
///
/// # Errors
///
/// - `DecryptionFailed` — AEAD tag mismatch or malformed ciphertext
/// - `TooManySkippedMessages` — message counter too far ahead
/// - `NoReceivingChain` — we have no receiving chain key (shouldn't happen after first message)
pub fn ratchet_decrypt(
    state: &mut RatchetState,
    msg: &Message,
    aad: &[u8],
) -> Result<Vec<u8>, RatchetError> {
    let header = &msg.header;
    let ciphertext = &msg.ciphertext;

    // Fast path: this might be a previously-skipped message whose key we stored.
    if let Some(pt) = try_skipped_message_key(&mut state.mk_skipped, header, ciphertext, aad) {
        return Ok(pt);
    }

    // Slow path: check if we need a DH ratchet step.
    let need_dh_ratchet = state.dhr.map_or(true, |dhr| dhr != header.dh);
    if need_dh_ratchet {
        // Save any skipped keys from the current receiving chain before switching.
        skip_message_keys(state, header.pn)?;
        dh_ratchet_step(state, &header.dh)?;
    }

    // Skip any messages in the new receiving chain that came before this one.
    skip_message_keys(state, header.n)?;

    // Derive the current message key.
    let ckr = state.ckr.as_ref().ok_or(RatchetError::NoReceivingChain)?;
    let (new_ckr, mk) = kdf_ck(ckr)?;
    state.ckr = Some(new_ckr);
    state.nr += 1;

    // Build the same AAD as the sender did.
    let header_bytes = encode_header(header);
    let mut full_aad = Vec::with_capacity(aad.len() + HEADER_LEN);
    full_aad.extend_from_slice(aad);
    full_aad.extend_from_slice(&header_bytes);

    let (enc_key, iv) = expand_message_key(&mk)?;

    if ciphertext.len() < 16 {
        return Err(RatchetError::DecryptionFailed);
    }
    let ct = &ciphertext[..ciphertext.len() - 16];
    let tag: [u8; 16] = ciphertext[ciphertext.len() - 16..]
        .try_into()
        .map_err(|_| RatchetError::DecryptionFailed)?;

    aead_decrypt(ct, &enc_key, &iv, &full_aad, &tag).ok_or(RatchetError::DecryptionFailed)
}

// ═══════════════════════════════════════════════════════════════════════════════
// SECTION 13: TESTS
// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    fn make_shared_key() -> [u8; 32] {
        let mut sk = [0u8; 32];
        getrandom::getrandom(&mut sk).unwrap();
        sk
    }

    fn make_session_pair() -> (RatchetState, RatchetState) {
        let sk = make_shared_key();
        let bob_kp = generate_ratchet_keypair();
        let bob_pub = bob_kp.public;
        let alice = ratchet_init_alice(&sk, &bob_pub);
        let bob = ratchet_init_bob(&sk, bob_kp);
        (alice, bob)
    }

    #[test]
    fn single_message_alice_to_bob() {
        let (mut alice, mut bob) = make_session_pair();
        let pt = b"hello, bob!";
        let msg = ratchet_encrypt(&mut alice, pt, b"").unwrap();
        let got = ratchet_decrypt(&mut bob, &msg, b"").unwrap();
        assert_eq!(got, pt);
    }

    #[test]
    fn multiple_messages_alice_to_bob() {
        let (mut alice, mut bob) = make_session_pair();
        for i in 0u8..10 {
            let plaintext = [i; 32];
            let msg = ratchet_encrypt(&mut alice, &plaintext, b"").unwrap();
            let got = ratchet_decrypt(&mut bob, &msg, b"").unwrap();
            assert_eq!(got, &plaintext[..]);
        }
    }

    #[test]
    fn bidirectional_exchange() {
        let (mut alice, mut bob) = make_session_pair();

        // Alice → Bob
        let m1 = ratchet_encrypt(&mut alice, b"hi bob", b"").unwrap();
        let r1 = ratchet_decrypt(&mut bob, &m1, b"").unwrap();
        assert_eq!(r1, b"hi bob");

        // Bob → Alice
        let m2 = ratchet_encrypt(&mut bob, b"hi alice", b"").unwrap();
        let r2 = ratchet_decrypt(&mut alice, &m2, b"").unwrap();
        assert_eq!(r2, b"hi alice");

        // Alice → Bob again
        let m3 = ratchet_encrypt(&mut alice, b"how are you?", b"").unwrap();
        let r3 = ratchet_decrypt(&mut bob, &m3, b"").unwrap();
        assert_eq!(r3, b"how are you?");
    }

    #[test]
    fn out_of_order_delivery() {
        let (mut alice, mut bob) = make_session_pair();

        // Alice sends three messages
        let m0 = ratchet_encrypt(&mut alice, b"msg 0", b"").unwrap();
        let m1 = ratchet_encrypt(&mut alice, b"msg 1", b"").unwrap();
        let m2 = ratchet_encrypt(&mut alice, b"msg 2", b"").unwrap();

        // Bob receives them out of order: 2, 0, 1
        let r2 = ratchet_decrypt(&mut bob, &m2, b"").unwrap();
        let r0 = ratchet_decrypt(&mut bob, &m0, b"").unwrap();
        let r1 = ratchet_decrypt(&mut bob, &m1, b"").unwrap();

        assert_eq!(r0, b"msg 0");
        assert_eq!(r1, b"msg 1");
        assert_eq!(r2, b"msg 2");
    }

    #[test]
    fn aad_mismatch_fails_decryption() {
        let (mut alice, mut bob) = make_session_pair();
        let msg = ratchet_encrypt(&mut alice, b"secret", b"correct-aad").unwrap();
        let result = ratchet_decrypt(&mut bob, &msg, b"wrong-aad");
        assert_eq!(result, Err(RatchetError::DecryptionFailed));
    }

    #[test]
    fn tampered_ciphertext_rejected() {
        let (mut alice, mut bob) = make_session_pair();
        let mut msg = ratchet_encrypt(&mut alice, b"secret", b"").unwrap();
        msg.ciphertext[0] ^= 0xFF;
        let result = ratchet_decrypt(&mut bob, &msg, b"");
        assert_eq!(result, Err(RatchetError::DecryptionFailed));
    }

    #[test]
    fn tampered_header_rejected() {
        let (mut alice, mut bob) = make_session_pair();
        let mut msg = ratchet_encrypt(&mut alice, b"secret", b"").unwrap();
        msg.header.n ^= 0xFF;
        let result = ratchet_decrypt(&mut bob, &msg, b"");
        assert!(result.is_err());
    }

    #[test]
    fn header_encoding_roundtrip() {
        let h = MessageHeader { dh: [0xABu8; 32], pn: 42, n: 7 };
        let encoded = encode_header(&h);
        let decoded = decode_header(&encoded);
        assert_eq!(decoded.dh, h.dh);
        assert_eq!(decoded.pn, h.pn);
        assert_eq!(decoded.n, h.n);
    }

    #[test]
    fn keypair_from_secret_derives_correct_public() {
        let mut secret = [0u8; 32];
        getrandom::getrandom(&mut secret).unwrap();
        secret[0] &= 248;
        secret[31] &= 127;
        secret[31] |= 64;
        let kp = KeyPair::from_secret(secret);
        let expected_pub = x25519_public_key(&secret);
        assert_eq!(kp.public, expected_pub);
    }

    #[test]
    fn two_ratchet_keypairs_differ() {
        let kp1 = generate_ratchet_keypair();
        let kp2 = generate_ratchet_keypair();
        assert_ne!(kp1.public, kp2.public);
    }

    #[test]
    fn empty_plaintext_works() {
        let (mut alice, mut bob) = make_session_pair();
        let msg = ratchet_encrypt(&mut alice, b"", b"").unwrap();
        let got = ratchet_decrypt(&mut bob, &msg, b"").unwrap();
        assert!(got.is_empty());
    }

    #[test]
    fn large_plaintext_works() {
        let (mut alice, mut bob) = make_session_pair();
        let plaintext = vec![0x42u8; 65536];
        let msg = ratchet_encrypt(&mut alice, &plaintext, b"").unwrap();
        let got = ratchet_decrypt(&mut bob, &msg, b"").unwrap();
        assert_eq!(got, plaintext);
    }

    #[test]
    fn many_back_and_forth_exchanges() {
        let (mut alice, mut bob) = make_session_pair();
        for i in 0u8..20 {
            let pt_a = [i; 16];
            let m = ratchet_encrypt(&mut alice, &pt_a, b"aad").unwrap();
            let r = ratchet_decrypt(&mut bob, &m, b"aad").unwrap();
            assert_eq!(r, &pt_a[..]);

            let pt_b = [i.wrapping_add(100); 16];
            let m = ratchet_encrypt(&mut bob, &pt_b, b"aad").unwrap();
            let r = ratchet_decrypt(&mut alice, &m, b"aad").unwrap();
            assert_eq!(r, &pt_b[..]);
        }
    }

    #[test]
    fn max_skip_exceeded_returns_error() {
        let (mut alice, mut bob) = make_session_pair();
        // Encrypt MAX_SKIP + 2 messages but only deliver the last one
        let mut last_msg = None;
        for _ in 0..=(MAX_SKIP + 1) {
            last_msg = Some(ratchet_encrypt(&mut alice, b"skip", b"").unwrap());
        }
        let result = ratchet_decrypt(&mut bob, last_msg.as_ref().unwrap(), b"");
        assert_eq!(result, Err(RatchetError::TooManySkippedMessages));
    }

    #[test]
    fn no_sending_chain_on_fresh_bob_returns_error() {
        let sk = make_shared_key();
        let bob_kp = generate_ratchet_keypair();
        let mut bob = ratchet_init_bob(&sk, bob_kp);
        let result = ratchet_encrypt(&mut bob, b"hi", b"");
        assert_eq!(result, Err(RatchetError::NoSendingChain));
    }
}
