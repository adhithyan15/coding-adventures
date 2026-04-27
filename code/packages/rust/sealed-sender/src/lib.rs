//! # coding_adventures_sealed_sender — Signal Sealed Sender
//!
//! Sealed Sender is the final layer of the Signal protocol stack. It solves
//! one remaining problem: even though message *contents* are encrypted, the
//! server still sees **who is sending to whom**. A server that logs metadata
//! can build a social graph of all communication patterns, which is sensitive
//! information in its own right.
//!
//! ## The Core Idea
//!
//! Normally a message envelope looks like:
//! ```text
//!  From: Alice
//!  To: Bob
//!  Body: <encrypted>
//! ```
//!
//! Sealed Sender wraps the entire message — including Alice's identity — in
//! an additional encryption layer keyed to Bob's identity key:
//! ```text
//!  From: ???
//!  To: <token derived from Bob's key>
//!  Envelope: ECDH(ephemeral_key, Bob_IK) → encrypt(Alice_cert ‖ Double_Ratchet_msg)
//! ```
//!
//! The server cannot open the envelope (it doesn't have Bob's private key).
//! It can only see the routing token (which is a hash of Bob's key, not his
//! identity) and the size of the encrypted payload.
//!
//! ## The Two-Layer Design
//!
//! ```text
//! ┌────────────────────────────────────────────────────────┐
//! │  Outer layer: Ephemeral-ECDH sealed envelope           │
//! │  ┌──────────────────────────────────────────────────┐  │
//! │  │  SenderCertificate  (server-signed)              │  │
//! │  │  ┌─────────────────────────────────────────────┐ │  │
//! │  │  │  Double Ratchet encrypted message           │ │  │
//! │  │  │  (protected by Signal's forward-secret algo)│ │  │
//! │  │  └─────────────────────────────────────────────┘ │  │
//! │  └──────────────────────────────────────────────────┘  │
//! └────────────────────────────────────────────────────────┘
//! ```
//!
//! 1. **Double Ratchet** encrypts the actual message with per-message keys.
//! 2. **SenderCertificate** proves who sent it (signed by a trusted server,
//!    so the recipient knows the sender is a real registered user).
//! 3. **Sealed envelope** hides both the certificate and the ratchet message
//!    from anyone who isn't the intended recipient.
//!
//! ## Wire Format
//!
//! ### SenderCertificate (124 bytes)
//! ```text
//! uuid[0..16]          sender's opaque identity (UUID v4)
//! device_id[16..20]    u32 LE — sender's device number
//! ik_public[20..52]    sender's X25519 identity public key
//! expires_at[52..60]   u64 LE — Unix ms expiry timestamp
//! server_sig[60..124]  Ed25519 sig over bytes[0..60]
//! ```
//!
//! ### Envelope
//! ```text
//! eph_pub[0..32]        ephemeral X25519 public key
//! ciphertext[32..]      ChaCha20-Poly1305 encryption of the inner payload
//!                        (last 16 bytes of ciphertext are the Poly1305 tag)
//! ```
//!
//! ### Inner payload (inside the envelope)
//! ```text
//! cert_len[0..4]        u32 LE (always CERT_LEN = 124)
//! cert_bytes[4..128]    SenderCertificate bytes
//! header_bytes[128..168] MessageHeader (HEADER_LEN = 40)
//! ct_len[168..172]      u32 LE — length of the ratchet ciphertext
//! ct[172..]             Double Ratchet ciphertext (+ 16-byte Poly1305 tag)
//! ```
//!
//! ## Key Derivation
//!
//! ```text
//! DH_out    = X25519(eph_secret, recipient_IK_x25519)
//! okm       = HKDF(salt=0×32, ikm=DH_out, info="sealed-sender-v1", len=44)
//! enc_key   = okm[0..32]
//! nonce     = okm[32..44]
//! ```
//!
//! AAD for the envelope AEAD is `eph_pub ‖ recipient_ik_x25519_pub`, which
//! binds the ciphertext to both the key exchange and the intended recipient.
//!
//! ## References
//! - Signal Sealed Sender Blog Post: <https://signal.org/blog/sealed-sender/>
//! - Trevor Perrin — "Sealed Sender for Signal" (2018)

pub use coding_adventures_double_ratchet::{
    generate_ratchet_keypair, ratchet_decrypt, ratchet_encrypt, ratchet_init_alice,
    ratchet_init_bob, KeyPair, Message, MessageHeader, RatchetError, RatchetState, HEADER_LEN,
};
pub use coding_adventures_x3dh::{
    create_prekey_bundle, generate_identity_keypair, generate_prekey_pair, x3dh_receive,
    x3dh_send, IdentityKeyPair, PreKeyBundle, PreKeyPair, X3DHError, X3DHOutput,
};

use coding_adventures_chacha20_poly1305::{aead_decrypt, aead_encrypt};
use coding_adventures_double_ratchet::{decode_header, encode_header};
use coding_adventures_ed25519::{sign as ed25519_sign, verify as ed25519_verify};
use coding_adventures_hkdf::{hkdf, HashAlgorithm};
use coding_adventures_zeroize::Zeroizing;

// ═══════════════════════════════════════════════════════════════════════════════
// SECTION 1: CONSTANTS
// ═══════════════════════════════════════════════════════════════════════════════

/// Byte length of a serialized `SenderCertificate`.
pub const CERT_LEN: usize = 124;

/// Byte offset up to which the certificate is covered by the server's signature.
///
/// The signature covers: uuid (16) + device_id (4) + ik_public (32) + expires_at (8) = 60 bytes.
pub const CERT_SIGNED_LEN: usize = 60;

// ═══════════════════════════════════════════════════════════════════════════════
// SECTION 2: SENDER CERTIFICATE
// ═══════════════════════════════════════════════════════════════════════════════

/// A server-issued certificate asserting that a given X25519 key belongs to a
/// registered user with a specific UUID and device ID.
///
/// The certificate is signed by the server's Ed25519 key. Before trusting any
/// sealed message, the recipient verifies this signature and checks expiry.
///
/// Design note: the certificate does **not** contain the server signature over
/// the full 124 bytes — only over the first 60 (`CERT_SIGNED_LEN`). The last
/// 64 bytes *are* the signature. This avoids a chicken-and-egg: you can't sign
/// a buffer that includes the signature itself.
#[derive(Clone, Debug, PartialEq)]
pub struct SenderCertificate {
    /// Sender's opaque UUID (typically UUID v4, treated as raw bytes here).
    pub uuid: [u8; 16],
    /// Sender's device ID (Signal supports multiple devices per account).
    pub device_id: u32,
    /// Sender's X25519 identity public key (not the Ed25519 signing key).
    pub ik_public: [u8; 32],
    /// Unix timestamp in milliseconds after which this certificate is invalid.
    pub expires_at: u64,
    /// Ed25519 signature over `bytes[0..CERT_SIGNED_LEN]` by the server's key.
    pub server_sig: [u8; 64],
}

/// The sealed message envelope produced by `sealed_send`.
pub struct SealedMessage {
    /// Server routing token: `HKDF(salt=0, ikm=recipient_IK_pub, info="sealed-sender-token")`.
    ///
    /// The server can route the message to the correct recipient without
    /// learning who the sender is — the token is a function of the *recipient*
    /// only, not the sender.
    pub recipient_token: [u8; 32],
    /// Encrypted inner payload: `eph_pub[0..32] ‖ aead_ciphertext[32..]`.
    pub envelope: Vec<u8>,
}

// ═══════════════════════════════════════════════════════════════════════════════
// SECTION 3: ERROR TYPE
// ═══════════════════════════════════════════════════════════════════════════════

/// Errors that can occur during sealed send or receive.
#[derive(Debug, PartialEq)]
pub enum SealedError {
    /// `getrandom` or HKDF failed — should not happen on a working system.
    CryptoError,
    /// The server's signature on the `SenderCertificate` did not verify.
    CertificateInvalid,
    /// The certificate's `expires_at` is ≤ `now_ms`: the certificate has expired.
    CertificateExpired,
    /// AEAD tag verification failed — the envelope was tampered with.
    DecryptionFailed,
    /// The payload wire format was malformed (wrong lengths, truncated).
    DecodeError,
    /// An error from the inner Double Ratchet layer.
    RatchetError(RatchetError),
}

impl std::fmt::Display for SealedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SealedError::CryptoError => write!(f, "cryptographic operation failed"),
            SealedError::CertificateInvalid => write!(f, "sender certificate signature is invalid"),
            SealedError::CertificateExpired => write!(f, "sender certificate has expired"),
            SealedError::DecryptionFailed => write!(f, "envelope decryption failed (tampered?)"),
            SealedError::DecodeError => write!(f, "malformed payload: unexpected length or format"),
            SealedError::RatchetError(e) => write!(f, "double ratchet error: {}", e),
        }
    }
}

impl std::error::Error for SealedError {}

impl From<RatchetError> for SealedError {
    fn from(e: RatchetError) -> Self {
        SealedError::RatchetError(e)
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// SECTION 4: CERTIFICATE ENCODING
// ═══════════════════════════════════════════════════════════════════════════════

/// Serialize a `SenderCertificate` to the 124-byte wire format.
pub fn encode_cert(cert: &SenderCertificate) -> [u8; CERT_LEN] {
    let mut out = [0u8; CERT_LEN];
    out[0..16].copy_from_slice(&cert.uuid);
    out[16..20].copy_from_slice(&cert.device_id.to_le_bytes());
    out[20..52].copy_from_slice(&cert.ik_public);
    out[52..60].copy_from_slice(&cert.expires_at.to_le_bytes());
    out[60..124].copy_from_slice(&cert.server_sig);
    out
}

/// Deserialize a 124-byte buffer into a `SenderCertificate`.
pub fn decode_cert(bytes: &[u8; CERT_LEN]) -> SenderCertificate {
    let mut uuid = [0u8; 16];
    uuid.copy_from_slice(&bytes[0..16]);
    let device_id = u32::from_le_bytes(bytes[16..20].try_into().unwrap());
    let mut ik_public = [0u8; 32];
    ik_public.copy_from_slice(&bytes[20..52]);
    let expires_at = u64::from_le_bytes(bytes[52..60].try_into().unwrap());
    let mut server_sig = [0u8; 64];
    server_sig.copy_from_slice(&bytes[60..124]);
    SenderCertificate { uuid, device_id, ik_public, expires_at, server_sig }
}

// ═══════════════════════════════════════════════════════════════════════════════
// SECTION 5: CERTIFICATE ISSUANCE AND VERIFICATION
// ═══════════════════════════════════════════════════════════════════════════════

/// Issue a `SenderCertificate` for a sender, signed with the server's Ed25519 key.
///
/// The server's `secret_key` is the 64-byte Ed25519 secret key (seed + public concatenated,
/// as produced by the ed25519 crate). The signature covers bytes[0..CERT_SIGNED_LEN].
pub fn issue_sender_certificate(
    uuid: [u8; 16],
    device_id: u32,
    sender_ik_public: [u8; 32],
    expires_at: u64,
    server_secret_key: &[u8; 64],
) -> SenderCertificate {
    // Build the to-be-signed bytes without the signature field.
    let mut tbs = [0u8; CERT_SIGNED_LEN];
    tbs[0..16].copy_from_slice(&uuid);
    tbs[16..20].copy_from_slice(&device_id.to_le_bytes());
    tbs[20..52].copy_from_slice(&sender_ik_public);
    tbs[52..60].copy_from_slice(&expires_at.to_le_bytes());

    let server_sig = ed25519_sign(&tbs, server_secret_key);
    SenderCertificate { uuid, device_id, ik_public: sender_ik_public, expires_at, server_sig }
}

/// Verify a `SenderCertificate`'s server signature.
///
/// Returns `true` if the signature over bytes[0..CERT_SIGNED_LEN] is valid for
/// `server_verify_key`.
pub fn verify_cert_signature(cert: &SenderCertificate, server_verify_key: &[u8; 32]) -> bool {
    let mut tbs = [0u8; CERT_SIGNED_LEN];
    tbs[0..16].copy_from_slice(&cert.uuid);
    tbs[16..20].copy_from_slice(&cert.device_id.to_le_bytes());
    tbs[20..52].copy_from_slice(&cert.ik_public);
    tbs[52..60].copy_from_slice(&cert.expires_at.to_le_bytes());
    ed25519_verify(&tbs, &cert.server_sig, server_verify_key)
}

// ═══════════════════════════════════════════════════════════════════════════════
// SECTION 6: PAYLOAD ENCODING
// ═══════════════════════════════════════════════════════════════════════════════

/// Encode a sender certificate and a Double Ratchet message into the inner payload.
///
/// Format:
/// ```text
/// cert_len (4 LE) ‖ cert_bytes (CERT_LEN) ‖ header_bytes (HEADER_LEN) ‖
/// ct_len (4 LE)   ‖ ciphertext
/// ```
fn encode_payload(cert: &SenderCertificate, msg: &Message) -> Vec<u8> {
    let cert_bytes = encode_cert(cert);
    let header_bytes = encode_header(&msg.header);
    let mut payload = Vec::with_capacity(4 + CERT_LEN + HEADER_LEN + 4 + msg.ciphertext.len());
    payload.extend_from_slice(&(CERT_LEN as u32).to_le_bytes());
    payload.extend_from_slice(&cert_bytes);
    payload.extend_from_slice(&header_bytes);
    payload.extend_from_slice(&(msg.ciphertext.len() as u32).to_le_bytes());
    payload.extend_from_slice(&msg.ciphertext);
    payload
}

/// Decode the inner payload back into a certificate and a Double Ratchet message.
///
/// All offset arithmetic uses checked operations to prevent integer overflow
/// on adversarially crafted payloads. Returns `None` if the payload is
/// malformed, truncated, or the lengths are inconsistent.
fn decode_payload(payload: &[u8]) -> Option<(SenderCertificate, Message)> {
    let mut offset = 0usize;

    // Read cert_len
    let cert_len_end = offset.checked_add(4)?;
    if payload.len() < cert_len_end {
        return None;
    }
    let cert_len = u32::from_le_bytes(payload[offset..cert_len_end].try_into().ok()?) as usize;
    offset = cert_len_end;

    // Validate and read cert bytes
    if cert_len != CERT_LEN {
        return None;
    }
    let cert_end = offset.checked_add(cert_len)?;
    if payload.len() < cert_end {
        return None;
    }
    let cert_bytes: &[u8; CERT_LEN] = payload[offset..cert_end].try_into().ok()?;
    let cert = decode_cert(cert_bytes);
    offset = cert_end;

    // Read header bytes
    let header_end = offset.checked_add(HEADER_LEN)?;
    if payload.len() < header_end {
        return None;
    }
    let header_bytes: &[u8; HEADER_LEN] = payload[offset..header_end].try_into().ok()?;
    let header = decode_header(header_bytes);
    offset = header_end;

    // Read ct_len
    let ct_len_field_end = offset.checked_add(4)?;
    if payload.len() < ct_len_field_end {
        return None;
    }
    let ct_len = u32::from_le_bytes(payload[offset..ct_len_field_end].try_into().ok()?) as usize;
    offset = ct_len_field_end;

    // Read ciphertext
    let ct_end = offset.checked_add(ct_len)?;
    if payload.len() < ct_end {
        return None;
    }
    let ciphertext = payload[offset..ct_end].to_vec();

    Some((cert, Message { header, ciphertext }))
}

// ═══════════════════════════════════════════════════════════════════════════════
// SECTION 7: KEY DERIVATION
// ═══════════════════════════════════════════════════════════════════════════════

/// Derive the encryption key and nonce from a DH output.
///
/// Uses HKDF with info string `"sealed-sender-v1"` to produce 44 bytes:
/// - bytes `[0..32]` → ChaCha20 encryption key
/// - bytes `[32..44]` → 12-byte nonce
///
/// The DH output and the HKDF OKM are both held in `Zeroizing<>` wrappers.
fn derive_enc_key(dh_out: &[u8; 32]) -> Result<([u8; 32], [u8; 12]), SealedError> {
    let okm = Zeroizing::new(
        hkdf(
            &[0u8; 32],
            dh_out,
            b"sealed-sender-v1",
            44,
            HashAlgorithm::Sha256,
        )
        .map_err(|_| SealedError::CryptoError)?,
    );
    let mut enc_key = [0u8; 32];
    let mut nonce = [0u8; 12];
    enc_key.copy_from_slice(&okm[..32]);
    nonce.copy_from_slice(&okm[32..44]);
    Ok((enc_key, nonce))
}

/// Derive the server routing token from a recipient's X25519 identity public key.
///
/// The token is a 32-byte value that lets the server route sealed messages to
/// the correct recipient, without revealing the recipient's identity to an
/// observer who doesn't already know it.
pub fn derive_recipient_token(recipient_ik_x25519_pub: &[u8; 32]) -> [u8; 32] {
    let okm = hkdf(
        &[0u8; 32],
        recipient_ik_x25519_pub,
        b"sealed-sender-token",
        32,
        HashAlgorithm::Sha256,
    )
    .expect("HKDF for recipient token should never fail");
    let mut token = [0u8; 32];
    token.copy_from_slice(&okm);
    token
}

// ═══════════════════════════════════════════════════════════════════════════════
// SECTION 8: SEALED SEND
// ═══════════════════════════════════════════════════════════════════════════════

/// Encrypt and seal a message from `sender_ik` to `recipient_ik_x25519_pub`.
///
/// The returned `SealedMessage` can be forwarded through an untrusted server.
/// The server can verify the `recipient_token` for routing, but cannot decrypt
/// the `envelope` without the recipient's private identity key.
///
/// # Arguments
///
/// * `sender_cert` — a server-issued certificate binding `sender_ik` to a UUID
/// * `session` — the Double Ratchet session (shared with the recipient)
/// * `recipient_ik_x25519_pub` — recipient's X25519 identity public key
/// * `plaintext` — the message bytes to encrypt
/// * `aad` — additional associated data (authenticated but not encrypted)
pub fn sealed_send(
    sender_cert: &SenderCertificate,
    session: &mut RatchetState,
    recipient_ik_x25519_pub: &[u8; 32],
    plaintext: &[u8],
    aad: &[u8],
) -> Result<SealedMessage, SealedError> {
    // Step 1: Encrypt plaintext with the Double Ratchet.
    let inner_msg = ratchet_encrypt(session, plaintext, aad).map_err(SealedError::RatchetError)?;

    // Step 2: Serialize the inner payload.
    let payload = encode_payload(sender_cert, &inner_msg);

    // Step 3: Generate a fresh ephemeral X25519 key pair.
    let mut eph_secret = Zeroizing::new([0u8; 32]);
    getrandom::getrandom(&mut *eph_secret).map_err(|_| SealedError::CryptoError)?;
    eph_secret[0] &= 248;
    eph_secret[31] &= 127;
    eph_secret[31] |= 64;

    use coding_adventures_curve25519::x25519_public_key;
    let eph_pub = x25519_public_key(&*eph_secret);

    // Step 4: ECDH with the recipient's identity key.
    use coding_adventures_curve25519::x25519;
    let dh_out = Zeroizing::new(x25519(&*eph_secret, recipient_ik_x25519_pub));

    // Step 5: Derive the encryption key and nonce.
    let (enc_key, nonce) = derive_enc_key(&*dh_out)?;

    // Step 6: AEAD-encrypt the payload.
    // AAD = eph_pub ‖ recipient_ik_x25519_pub — this binds the ciphertext to:
    //   a) the specific ephemeral key exchange (prevents ciphertext transplant)
    //   b) the intended recipient's identity (prevents misdirected delivery)
    let outer_aad: Vec<u8> =
        [eph_pub.as_slice(), recipient_ik_x25519_pub.as_slice()].concat();
    let (ct, tag) = aead_encrypt(&payload, &enc_key, &nonce, &outer_aad);

    // Step 7: Build the envelope: eph_pub ‖ ct ‖ tag.
    let mut envelope = Vec::with_capacity(32 + ct.len() + 16);
    envelope.extend_from_slice(&eph_pub);
    envelope.extend_from_slice(&ct);
    envelope.extend_from_slice(&tag);

    // Step 8: Compute the routing token for the server.
    let recipient_token = derive_recipient_token(recipient_ik_x25519_pub);

    Ok(SealedMessage { recipient_token, envelope })
}

// ═══════════════════════════════════════════════════════════════════════════════
// SECTION 9: SEALED RECEIVE
// ═══════════════════════════════════════════════════════════════════════════════

/// Decrypt a sealed message and verify the sender's certificate.
///
/// # Arguments
///
/// * `recipient_ik` — the recipient's identity key pair (X25519 secret used for ECDH)
/// * `sealed` — the `SealedMessage` received from the server
/// * `session` — the Double Ratchet session (shared with the sender)
/// * `server_verify_key` — the server's Ed25519 public key (used to verify `SenderCertificate`)
/// * `now_ms` — current Unix timestamp in milliseconds (for certificate expiry check)
/// * `aad` — must match the `aad` passed to `sealed_send`
///
/// # Returns
///
/// `(plaintext, sender_certificate)` on success.
///
/// # Security
///
/// * The certificate expiry check uses `now_ms >= cert.expires_at` to correctly
///   reject expired certificates. Using `<` instead (a common off-by-one) would
///   allow a certificate at exactly `expires_at` to be accepted, and using
///   `cert.expires_at < now_ms` would allow a certificate with `expires_at = 0`
///   to bypass the check entirely (since `0 < 0` is false).
pub fn sealed_receive(
    recipient_ik: &IdentityKeyPair,
    sealed: &SealedMessage,
    session: &mut RatchetState,
    server_verify_key: &[u8; 32],
    now_ms: u64,
    aad: &[u8],
) -> Result<(Vec<u8>, SenderCertificate), SealedError> {
    let envelope = &sealed.envelope;

    // Step 1: Extract the ephemeral public key.
    if envelope.len() < 32 + 16 {
        return Err(SealedError::DecryptionFailed);
    }
    let eph_pub: &[u8; 32] = envelope[0..32].try_into().unwrap();

    // Step 2: ECDH with our identity key.
    use coding_adventures_curve25519::x25519;
    let secret = recipient_ik.x25519_secret(); // Zeroizing<[u8;32]>
    let dh_out = Zeroizing::new(x25519(&*secret, eph_pub));

    // Step 3: Derive the encryption key and nonce.
    let (enc_key, nonce) = derive_enc_key(&*dh_out)?;

    // Step 4: AEAD-decrypt the payload.
    // AAD = eph_pub ‖ recipient_ik.x25519_public — must match sealed_send exactly.
    // This binds the ciphertext to both the key exchange and the intended recipient.
    let ct_and_tag = &envelope[32..];
    if ct_and_tag.len() < 16 {
        return Err(SealedError::DecryptionFailed);
    }
    let ct = &ct_and_tag[..ct_and_tag.len() - 16];
    let tag: [u8; 16] = ct_and_tag[ct_and_tag.len() - 16..].try_into().unwrap();
    let outer_aad: Vec<u8> =
        [eph_pub.as_slice(), recipient_ik.x25519_public.as_slice()].concat();
    let payload =
        aead_decrypt(ct, &enc_key, &nonce, &outer_aad, &tag).ok_or(SealedError::DecryptionFailed)?;

    // Step 5: Decode the inner payload.
    let (cert, inner_msg) = decode_payload(&payload).ok_or(SealedError::DecodeError)?;

    // Step 6: Verify the sender certificate's server signature.
    if !verify_cert_signature(&cert, server_verify_key) {
        return Err(SealedError::CertificateInvalid);
    }

    // Step 7: Check certificate expiry.
    // SECURITY: Use `>=` not `<` — `cert.expires_at < now_ms` would pass when
    // `now_ms = 0`, silently accepting expired-at-epoch certificates.
    if now_ms >= cert.expires_at {
        return Err(SealedError::CertificateExpired);
    }

    // Step 8: Decrypt the inner Double Ratchet message.
    let plaintext =
        ratchet_decrypt(session, &inner_msg, aad).map_err(SealedError::RatchetError)?;

    Ok((plaintext, cert))
}

// ═══════════════════════════════════════════════════════════════════════════════
// SECTION 10: TESTS
// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use coding_adventures_ed25519::generate_keypair as ed25519_generate_keypair;

    /// Build an Alice+Bob session via X3DH, return (alice_ratchet, bob_ratchet, alice_cert).
    fn setup_session(
        alice_ik: &IdentityKeyPair,
        bob_ik: &IdentityKeyPair,
        server_sk: &[u8; 64],
        expires_at: u64,
    ) -> (RatchetState, RatchetState, SenderCertificate) {
        // Bob publishes a prekey bundle
        let bob_spk = generate_prekey_pair();
        let bundle = create_prekey_bundle(bob_ik, &bob_spk, 1, None);

        // Alice runs X3DH to get the shared key
        let alice_x3dh = x3dh_send(alice_ik, &bundle).unwrap();

        // Bob runs X3DH to get the same shared key
        let bob_sk = x3dh_receive(
            bob_ik,
            &bob_spk,
            None,
            &alice_ik.x25519_public,
            &alice_x3dh.ephemeral_public,
        )
        .unwrap();

        assert_eq!(alice_x3dh.shared_key, bob_sk, "X3DH shared keys must match");

        // Initialize ratchets: Alice uses Bob's SPK as the initial DH ratchet key
        let alice_ratchet = ratchet_init_alice(&alice_x3dh.shared_key, &bob_spk.public);
        let bob_ratchet =
            ratchet_init_bob(&bob_sk, KeyPair::from_secret(*bob_spk.secret()));

        // Server issues a certificate for Alice
        let alice_uuid = [0xAAu8; 16];
        let cert = issue_sender_certificate(
            alice_uuid,
            1,
            alice_ik.x25519_public,
            expires_at,
            server_sk,
        );

        (alice_ratchet, bob_ratchet, cert)
    }

    #[test]
    fn full_stack_single_message() {
        let alice_ik = generate_identity_keypair();
        let bob_ik = generate_identity_keypair();

        // Server key pair
        let mut server_seed = [0u8; 32];
        getrandom::getrandom(&mut server_seed).unwrap();
        let (server_vk, server_sk) = ed25519_generate_keypair(&server_seed);

        let now_ms = 1_000_000u64;
        let expires_at = now_ms + 86_400_000; // +1 day

        let (mut alice_r, mut bob_r, alice_cert) =
            setup_session(&alice_ik, &bob_ik, &server_sk, expires_at);

        let plaintext = b"hello, sealed world!";
        let sealed =
            sealed_send(&alice_cert, &mut alice_r, &bob_ik.x25519_public, plaintext, b"")
                .unwrap();

        let (got, got_cert) =
            sealed_receive(&bob_ik, &sealed, &mut bob_r, &server_vk, now_ms, b"").unwrap();

        assert_eq!(got, plaintext);
        assert_eq!(got_cert.ik_public, alice_ik.x25519_public);
    }

    #[test]
    fn multiple_sealed_messages() {
        let alice_ik = generate_identity_keypair();
        let bob_ik = generate_identity_keypair();
        let mut seed = [0u8; 32];
        getrandom::getrandom(&mut seed).unwrap();
        let (server_vk, server_sk) = ed25519_generate_keypair(&seed);
        let now_ms = 1_000u64;
        let expires_at = now_ms + 86_400_000;

        let (mut alice_r, mut bob_r, alice_cert) =
            setup_session(&alice_ik, &bob_ik, &server_sk, expires_at);

        for i in 0u8..5 {
            let pt = [i; 32];
            let sealed = sealed_send(&alice_cert, &mut alice_r, &bob_ik.x25519_public, &pt, b"aad")
                .unwrap();
            let (got, _cert) =
                sealed_receive(&bob_ik, &sealed, &mut bob_r, &server_vk, now_ms, b"aad").unwrap();
            assert_eq!(got, &pt[..]);
        }
    }

    #[test]
    fn expired_certificate_rejected() {
        let alice_ik = generate_identity_keypair();
        let bob_ik = generate_identity_keypair();
        let mut seed = [0u8; 32];
        getrandom::getrandom(&mut seed).unwrap();
        let (server_vk, server_sk) = ed25519_generate_keypair(&seed);

        // Certificate expired 1 ms ago
        let now_ms = 1_000_000u64;
        let expires_at = now_ms - 1; // already expired

        let (mut alice_r, mut bob_r, alice_cert) =
            setup_session(&alice_ik, &bob_ik, &server_sk, expires_at);

        let sealed =
            sealed_send(&alice_cert, &mut alice_r, &bob_ik.x25519_public, b"hi", b"").unwrap();

        let result = sealed_receive(&bob_ik, &sealed, &mut bob_r, &server_vk, now_ms, b"");
        assert_eq!(result, Err(SealedError::CertificateExpired));
    }

    #[test]
    fn tampered_envelope_rejected() {
        let alice_ik = generate_identity_keypair();
        let bob_ik = generate_identity_keypair();
        let mut seed = [0u8; 32];
        getrandom::getrandom(&mut seed).unwrap();
        let (server_vk, server_sk) = ed25519_generate_keypair(&seed);
        let now_ms = 1_000u64;
        let expires_at = now_ms + 86_400_000;

        let (mut alice_r, mut bob_r, alice_cert) =
            setup_session(&alice_ik, &bob_ik, &server_sk, expires_at);

        let mut sealed =
            sealed_send(&alice_cert, &mut alice_r, &bob_ik.x25519_public, b"secret", b"")
                .unwrap();
        // Flip a bit in the ciphertext portion of the envelope
        let last = sealed.envelope.len() - 1;
        sealed.envelope[last] ^= 0xFF;

        let result = sealed_receive(&bob_ik, &sealed, &mut bob_r, &server_vk, now_ms, b"");
        assert_eq!(result, Err(SealedError::DecryptionFailed));
    }

    #[test]
    fn invalid_server_signature_rejected() {
        let alice_ik = generate_identity_keypair();
        let bob_ik = generate_identity_keypair();
        let mut seed = [0u8; 32];
        getrandom::getrandom(&mut seed).unwrap();
        let (server_vk, server_sk) = ed25519_generate_keypair(&seed);
        let now_ms = 1_000u64;
        let expires_at = now_ms + 86_400_000;

        let (mut alice_r, mut bob_r, mut alice_cert) =
            setup_session(&alice_ik, &bob_ik, &server_sk, expires_at);

        // Corrupt the server signature
        alice_cert.server_sig[0] ^= 0xFF;

        let sealed =
            sealed_send(&alice_cert, &mut alice_r, &bob_ik.x25519_public, b"hi", b"").unwrap();

        let result = sealed_receive(&bob_ik, &sealed, &mut bob_r, &server_vk, now_ms, b"");
        assert_eq!(result, Err(SealedError::CertificateInvalid));
    }

    #[test]
    fn wrong_recipient_ik_cannot_decrypt() {
        let alice_ik = generate_identity_keypair();
        let bob_ik = generate_identity_keypair();
        let eve_ik = generate_identity_keypair();
        let mut seed = [0u8; 32];
        getrandom::getrandom(&mut seed).unwrap();
        let (server_vk, server_sk) = ed25519_generate_keypair(&seed);
        let now_ms = 1_000u64;
        let expires_at = now_ms + 86_400_000;

        let (mut alice_r, mut bob_r, alice_cert) =
            setup_session(&alice_ik, &bob_ik, &server_sk, expires_at);

        // Alice seals to Bob, but Eve tries to decrypt with her own key
        let sealed =
            sealed_send(&alice_cert, &mut alice_r, &bob_ik.x25519_public, b"secret", b"")
                .unwrap();

        let result = sealed_receive(&eve_ik, &sealed, &mut bob_r, &server_vk, now_ms, b"");
        assert!(result.is_err(), "Eve should not be able to decrypt Bob's message");
    }

    #[test]
    fn recipient_token_is_deterministic() {
        let bob_ik = generate_identity_keypair();
        let t1 = derive_recipient_token(&bob_ik.x25519_public);
        let t2 = derive_recipient_token(&bob_ik.x25519_public);
        assert_eq!(t1, t2);
    }

    #[test]
    fn recipient_tokens_differ_for_different_keys() {
        let bob1 = generate_identity_keypair();
        let bob2 = generate_identity_keypair();
        let t1 = derive_recipient_token(&bob1.x25519_public);
        let t2 = derive_recipient_token(&bob2.x25519_public);
        assert_ne!(t1, t2);
    }

    #[test]
    fn cert_encode_decode_roundtrip() {
        let mut seed = [0u8; 32];
        getrandom::getrandom(&mut seed).unwrap();
        let (server_vk, server_sk) = ed25519_generate_keypair(&seed);
        let ik = generate_identity_keypair();
        let cert = issue_sender_certificate([0x01; 16], 42, ik.x25519_public, 9999, &server_sk);
        let encoded = encode_cert(&cert);
        let decoded = decode_cert(&encoded);
        assert_eq!(decoded.uuid, cert.uuid);
        assert_eq!(decoded.device_id, cert.device_id);
        assert_eq!(decoded.ik_public, cert.ik_public);
        assert_eq!(decoded.expires_at, cert.expires_at);
        assert_eq!(decoded.server_sig, cert.server_sig);
        assert!(verify_cert_signature(&decoded, &server_vk));
    }

    #[test]
    fn aad_mismatch_causes_ratchet_failure() {
        let alice_ik = generate_identity_keypair();
        let bob_ik = generate_identity_keypair();
        let mut seed = [0u8; 32];
        getrandom::getrandom(&mut seed).unwrap();
        let (server_vk, server_sk) = ed25519_generate_keypair(&seed);
        let now_ms = 1_000u64;
        let expires_at = now_ms + 86_400_000;

        let (mut alice_r, mut bob_r, alice_cert) =
            setup_session(&alice_ik, &bob_ik, &server_sk, expires_at);

        let sealed = sealed_send(
            &alice_cert,
            &mut alice_r,
            &bob_ik.x25519_public,
            b"secret",
            b"correct-aad",
        )
        .unwrap();

        let result =
            sealed_receive(&bob_ik, &sealed, &mut bob_r, &server_vk, now_ms, b"wrong-aad");
        assert!(
            matches!(result, Err(SealedError::RatchetError(RatchetError::DecryptionFailed))),
            "expected DecryptionFailed, got {:?}",
            result
        );
    }
}
