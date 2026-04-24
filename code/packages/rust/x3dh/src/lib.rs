//! # coding_adventures_x3dh — Extended Triple Diffie-Hellman Key Agreement
//!
//! X3DH is the **asynchronous key agreement protocol** that powers Signal's
//! initial session establishment. Alice can send Bob an encrypted message even
//! when Bob is offline — she just needs his pre-published public key bundle.
//!
//! ## The Sealed Letter Analogy
//!
//! Imagine a mailbox outside Bob's house. Bob puts inside it:
//! - His **identity card** (long-term identity key, signed by him)
//! - A **fresh set of keys** for Alice to use (signed prekey)
//! - A **one-time token** (one-time prekey, optional)
//!
//! Alice picks up the bundle, verifies it's really Bob's (signature check),
//! generates a **temporary key pair** (ephemeral key), and mixes all the key
//! material together into a single shared secret. She then sends Bob her
//! ephemeral public key so he can reproduce the same shared secret.
//!
//! ## The Four DH Operations
//!
//! ```text
//!   Alice                               Bob
//!   ─────                               ───
//!   IK_A (identity key pair)            IK_B (identity key pair)
//!   EK_A (ephemeral, one-time)          SPK_B (signed prekey)
//!                                       OPK_B (one-time prekey, optional)
//!
//!   DH1 = X25519(IK_A_dh, SPK_B)       ←  long-term + medium-term
//!   DH2 = X25519(EK_A,    IK_B_dh)     ←  ephemeral + long-term
//!   DH3 = X25519(EK_A,    SPK_B)       ←  ephemeral + medium-term
//!   DH4 = X25519(EK_A,    OPK_B)       ←  ephemeral + one-time (if present)
//!
//!   KM  = F ‖ DH1 ‖ DH2 ‖ DH3 [‖ DH4]      F = 0xFF × 32
//!   SK  = HKDF(salt=F, ikm=KM, info="WhisperText", L=32)
//! ```
//!
//! Bob mirrors the same four operations (using his private keys) to recover SK.
//!
//! ## Why Four DH Operations?
//!
//! - **DH1** binds Alice's long-term identity to the session.
//! - **DH2** binds Bob's long-term identity — ensures only the real Bob can decrypt.
//! - **DH3** binds the ephemeral key to Bob's signed prekey — adds forward secrecy.
//! - **DH4** (if present) eliminates replays — a one-time prekey can only be used once.
//!
//! ## References
//! - Signal X3DH Specification: <https://signal.org/docs/specifications/x3dh/>
//! - RFC 7748 (X25519): <https://tools.ietf.org/html/rfc7748>
//! - RFC 8032 (Ed25519): <https://tools.ietf.org/html/rfc8032>

use coding_adventures_curve25519::{x25519, x25519_public_key};
use coding_adventures_ed25519::{
    generate_keypair as ed25519_generate_keypair,
    sign as ed25519_sign,
    verify as ed25519_verify,
};
use coding_adventures_hkdf::{hkdf, HashAlgorithm};
use coding_adventures_zeroize::{Zeroize, Zeroizing};

// ═══════════════════════════════════════════════════════════════════════════════
// SECTION 1: KEY TYPES
// ═══════════════════════════════════════════════════════════════════════════════

/// A long-term identity key pair.
///
/// Holds both an **X25519** key (for DH) and an **Ed25519** key (for signing),
/// generated from independent random seeds to prevent cross-algorithm attacks.
pub struct IdentityKeyPair {
    pub x25519_public: [u8; 32],
    pub ed25519_public: [u8; 32],
    x25519_secret: [u8; 32],
    ed25519_secret: [u8; 64],
}

impl Zeroize for IdentityKeyPair {
    fn zeroize(&mut self) {
        self.x25519_secret.zeroize();
        self.ed25519_secret.zeroize();
    }
}

impl Drop for IdentityKeyPair {
    fn drop(&mut self) {
        self.zeroize();
    }
}

impl IdentityKeyPair {
    /// Return the X25519 secret key, used by the Sealed Sender layer for ECDH.
    pub fn x25519_secret(&self) -> [u8; 32] {
        self.x25519_secret
    }
}

/// An X25519 prekey pair (signed or one-time).
///
/// The public key is shared in bundles. The secret key is used for DH on receive.
pub struct PreKeyPair {
    pub public: [u8; 32],
    secret: [u8; 32],
}

impl Zeroize for PreKeyPair {
    fn zeroize(&mut self) {
        self.secret.zeroize();
    }
}

impl Drop for PreKeyPair {
    fn drop(&mut self) {
        self.zeroize();
    }
}

impl PreKeyPair {
    /// Return the secret key for DH operations.
    pub fn secret(&self) -> &[u8; 32] {
        &self.secret
    }
}

/// The information Bob publishes to allow Alice to initiate a session.
#[derive(Clone, Debug)]
pub struct PreKeyBundle {
    /// Bob's X25519 identity public key (for DH1 and DH2).
    pub identity_key: [u8; 32],
    /// Bob's Ed25519 identity public key (for SPK signature verification).
    pub identity_key_sign: [u8; 32],
    /// Signed prekey ID — lets Bob know which SPK was used.
    pub signed_prekey_id: u32,
    /// Bob's X25519 signed prekey public key (for DH1 and DH3).
    pub signed_prekey: [u8; 32],
    /// Ed25519 signature over `signed_prekey` by `identity_key_sign`.
    pub signed_prekey_sig: [u8; 64],
    /// One-time prekey ID (if included).
    pub one_time_prekey_id: Option<u32>,
    /// Bob's X25519 one-time prekey (optional, used for DH4).
    pub one_time_prekey: Option<[u8; 32]>,
}

/// Output of a successful X3DH sender operation.
#[derive(Clone, Debug)]
pub struct X3DHOutput {
    /// 32-byte shared secret — passed to the Double Ratchet.
    pub shared_key: [u8; 32],
    /// Alice's ephemeral X25519 public key — sent to Bob in the initial message.
    pub ephemeral_public: [u8; 32],
}

impl Zeroize for X3DHOutput {
    fn zeroize(&mut self) {
        self.shared_key.zeroize();
    }
}

impl Drop for X3DHOutput {
    fn drop(&mut self) {
        self.zeroize();
    }
}

/// Errors that can occur during X3DH key agreement.
#[derive(Debug, PartialEq)]
pub enum X3DHError {
    InvalidSignature,
    KdfError,
}

impl std::fmt::Display for X3DHError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            X3DHError::InvalidSignature => write!(f, "signed prekey signature is invalid"),
            X3DHError::KdfError => write!(f, "HKDF key derivation failed"),
        }
    }
}

impl std::error::Error for X3DHError {}

// ═══════════════════════════════════════════════════════════════════════════════
// SECTION 2: KEY GENERATION
// ═══════════════════════════════════════════════════════════════════════════════

/// Generate a fresh identity key pair.
///
/// Uses two independent random seeds — one for the X25519 key and one for the
/// Ed25519 key. This prevents any mathematical relationship between the two sub-keys,
/// which could otherwise enable cross-algorithm attacks.
///
/// Seeds are held in `Zeroizing<>` wrappers so they are wiped from stack memory
/// when the function returns, even if it panics.
pub fn generate_identity_keypair() -> IdentityKeyPair {
    let mut seed_x = Zeroizing::new([0u8; 32]);
    let mut seed_e = Zeroizing::new([0u8; 32]);
    getrandom::getrandom(&mut *seed_x).expect("getrandom failed");
    getrandom::getrandom(&mut *seed_e).expect("getrandom failed");

    // RFC 7748 §5 clamping for X25519 scalars.
    let mut x25519_secret = *seed_x;
    x25519_secret[0]  &= 248; // clear bits 0-2: forces cofactor-8 scalar
    x25519_secret[31] &= 127; // clear bit 7: keep scalar < 2^255
    x25519_secret[31] |= 64;  // set bit 6: keep scalar >= 2^254

    let x25519_public = x25519_public_key(&x25519_secret);
    let (ed25519_public, ed25519_secret) = ed25519_generate_keypair(&*seed_e);

    IdentityKeyPair { x25519_public, ed25519_public, x25519_secret, ed25519_secret }
}

/// Generate a fresh X25519 prekey pair with RFC 7748 clamping applied.
pub fn generate_prekey_pair() -> PreKeyPair {
    let mut seed = Zeroizing::new([0u8; 32]);
    getrandom::getrandom(&mut *seed).expect("getrandom failed");

    let mut secret = *seed;
    secret[0]  &= 248;
    secret[31] &= 127;
    secret[31] |= 64;

    let public = x25519_public_key(&secret);
    PreKeyPair { public, secret }
}

// ═══════════════════════════════════════════════════════════════════════════════
// SECTION 3: BUNDLE CONSTRUCTION
// ═══════════════════════════════════════════════════════════════════════════════

/// Sign a prekey's public key with the identity key's Ed25519 signing key.
pub fn sign_prekey(ik: &IdentityKeyPair, spk_pub: &[u8; 32]) -> [u8; 64] {
    ed25519_sign(spk_pub, &ik.ed25519_secret)
}

/// Build a `PreKeyBundle` from Bob's key material.
pub fn create_prekey_bundle(
    ik: &IdentityKeyPair,
    spk: &PreKeyPair,
    spk_id: u32,
    opk: Option<(&PreKeyPair, u32)>,
) -> PreKeyBundle {
    let sig = sign_prekey(ik, &spk.public);
    let (opk_id, opk_pub) = match opk {
        Some((kp, id)) => (Some(id), Some(kp.public)),
        None => (None, None),
    };
    PreKeyBundle {
        identity_key: ik.x25519_public,
        identity_key_sign: ik.ed25519_public,
        signed_prekey_id: spk_id,
        signed_prekey: spk.public,
        signed_prekey_sig: sig,
        one_time_prekey_id: opk_id,
        one_time_prekey: opk_pub,
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// SECTION 4: KEY AGREEMENT
// ═══════════════════════════════════════════════════════════════════════════════

/// Perform the X3DH sender operation (Alice's side).
///
/// All intermediate DH outputs and the IKM buffer are held in `Zeroizing<>`
/// wrappers so they are wiped from memory when the function returns, regardless
/// of whether it succeeded or failed. This limits the window during which these
/// values are live in process memory.
pub fn x3dh_send(
    sender_ik: &IdentityKeyPair,
    bundle: &PreKeyBundle,
) -> Result<X3DHOutput, X3DHError> {
    // Verify the signed prekey before using any key material from the bundle.
    if !ed25519_verify(&bundle.signed_prekey, &bundle.signed_prekey_sig, &bundle.identity_key_sign) {
        return Err(X3DHError::InvalidSignature);
    }

    // Generate Alice's ephemeral key pair. Single-use; wiped on return.
    let mut ek_secret = Zeroizing::new([0u8; 32]);
    getrandom::getrandom(&mut *ek_secret).expect("getrandom failed");
    ek_secret[0]  &= 248;
    ek_secret[31] &= 127;
    ek_secret[31] |= 64;
    let ek_pub = x25519_public_key(&*ek_secret);

    // Four DH operations — all outputs wiped on return.
    let dh1 = Zeroizing::new(x25519(&sender_ik.x25519_secret, &bundle.signed_prekey));
    let dh2 = Zeroizing::new(x25519(&*ek_secret, &bundle.identity_key));
    let dh3 = Zeroizing::new(x25519(&*ek_secret, &bundle.signed_prekey));

    // KM = F ‖ DH1 ‖ DH2 ‖ DH3 [‖ DH4]
    let salt = [0xFFu8; 32];
    let mut ikm = Zeroizing::new(Vec::with_capacity(32 * 5));
    ikm.extend_from_slice(&[0xFFu8; 32]); // F prefix
    ikm.extend_from_slice(&*dh1);
    ikm.extend_from_slice(&*dh2);
    ikm.extend_from_slice(&*dh3);

    if let Some(opk) = bundle.one_time_prekey {
        let dh4 = Zeroizing::new(x25519(&*ek_secret, &opk));
        ikm.extend_from_slice(&*dh4);
    }

    let okm = Zeroizing::new(
        hkdf(&salt, &*ikm, b"WhisperText", 32, HashAlgorithm::Sha256)
            .map_err(|_| X3DHError::KdfError)?,
    );

    let mut shared_key = [0u8; 32];
    shared_key.copy_from_slice(&*okm);

    Ok(X3DHOutput { shared_key, ephemeral_public: ek_pub })
}

/// Perform the X3DH receiver operation (Bob's side).
///
/// See `x3dh_send` for the protocol description. Bob mirrors the four DH
/// operations using his private keys; X25519 commutativity ensures both
/// sides arrive at the same shared key.
pub fn x3dh_receive(
    receiver_ik: &IdentityKeyPair,
    receiver_spk: &PreKeyPair,
    receiver_opk: Option<&PreKeyPair>,
    sender_ik_pub: &[u8; 32],
    sender_ek_pub: &[u8; 32],
) -> Result<[u8; 32], X3DHError> {
    let dh1 = Zeroizing::new(x25519(&receiver_spk.secret, sender_ik_pub));
    let dh2 = Zeroizing::new(x25519(&receiver_ik.x25519_secret, sender_ek_pub));
    let dh3 = Zeroizing::new(x25519(&receiver_spk.secret, sender_ek_pub));

    let salt = [0xFFu8; 32];
    let mut ikm = Zeroizing::new(Vec::with_capacity(32 * 5));
    ikm.extend_from_slice(&[0xFFu8; 32]);
    ikm.extend_from_slice(&*dh1);
    ikm.extend_from_slice(&*dh2);
    ikm.extend_from_slice(&*dh3);

    if let Some(opk) = receiver_opk {
        let dh4 = Zeroizing::new(x25519(&opk.secret, sender_ek_pub));
        ikm.extend_from_slice(&*dh4);
    }

    let okm = Zeroizing::new(
        hkdf(&salt, &*ikm, b"WhisperText", 32, HashAlgorithm::Sha256)
            .map_err(|_| X3DHError::KdfError)?,
    );

    let mut shared_key = [0u8; 32];
    shared_key.copy_from_slice(&*okm);
    Ok(shared_key)
}

// ═══════════════════════════════════════════════════════════════════════════════
// SECTION 5: TESTS
// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_keypair_has_distinct_keys() {
        let ik = generate_identity_keypair();
        assert_ne!(ik.x25519_public, ik.ed25519_public);
    }

    #[test]
    fn prekey_public_matches_secret() {
        let pk = generate_prekey_pair();
        let derived_pub = x25519_public_key(pk.secret());
        assert_eq!(pk.public, derived_pub);
    }

    #[test]
    fn two_identity_keypairs_differ() {
        let ik1 = generate_identity_keypair();
        let ik2 = generate_identity_keypair();
        assert_ne!(ik1.x25519_public, ik2.x25519_public);
        assert_ne!(ik1.ed25519_public, ik2.ed25519_public);
    }

    #[test]
    fn prekey_secret_clamped_correctly() {
        for _ in 0..20 {
            let pk = generate_prekey_pair();
            let s = pk.secret();
            assert_eq!(s[0] & 7, 0);
            assert_eq!(s[31] & 128, 0);
            assert_ne!(s[31] & 64, 0);
        }
    }

    #[test]
    fn bundle_spk_signature_verifies() {
        let ik  = generate_identity_keypair();
        let spk = generate_prekey_pair();
        let bundle = create_prekey_bundle(&ik, &spk, 1, None);
        assert!(ed25519_verify(&bundle.signed_prekey, &bundle.signed_prekey_sig, &bundle.identity_key_sign));
    }

    #[test]
    fn bundle_with_opk_includes_opk() {
        let ik  = generate_identity_keypair();
        let spk = generate_prekey_pair();
        let opk = generate_prekey_pair();
        let bundle = create_prekey_bundle(&ik, &spk, 1, Some((&opk, 42)));
        assert_eq!(bundle.one_time_prekey_id, Some(42));
        assert_eq!(bundle.one_time_prekey, Some(opk.public));
    }

    #[test]
    fn bundle_without_opk_has_none() {
        let ik  = generate_identity_keypair();
        let spk = generate_prekey_pair();
        let bundle = create_prekey_bundle(&ik, &spk, 1, None);
        assert!(bundle.one_time_prekey.is_none());
    }

    #[test]
    fn x3dh_without_opk_produces_matching_keys() {
        let alice_ik = generate_identity_keypair();
        let bob_ik   = generate_identity_keypair();
        let bob_spk  = generate_prekey_pair();
        let bundle   = create_prekey_bundle(&bob_ik, &bob_spk, 1, None);

        let alice_out = x3dh_send(&alice_ik, &bundle).unwrap();
        let bob_sk    = x3dh_receive(
            &bob_ik, &bob_spk, None,
            &alice_ik.x25519_public, &alice_out.ephemeral_public,
        ).unwrap();

        assert_eq!(alice_out.shared_key, bob_sk);
    }

    #[test]
    fn x3dh_with_opk_produces_matching_keys() {
        let alice_ik = generate_identity_keypair();
        let bob_ik   = generate_identity_keypair();
        let bob_spk  = generate_prekey_pair();
        let bob_opk  = generate_prekey_pair();
        let bundle   = create_prekey_bundle(&bob_ik, &bob_spk, 1, Some((&bob_opk, 1)));

        let alice_out = x3dh_send(&alice_ik, &bundle).unwrap();
        let bob_sk    = x3dh_receive(
            &bob_ik, &bob_spk, Some(&bob_opk),
            &alice_ik.x25519_public, &alice_out.ephemeral_public,
        ).unwrap();

        assert_eq!(alice_out.shared_key, bob_sk);
    }

    #[test]
    fn x3dh_with_opk_differs_from_without() {
        let alice_ik    = generate_identity_keypair();
        let bob_ik      = generate_identity_keypair();
        let bob_spk     = generate_prekey_pair();
        let bob_opk     = generate_prekey_pair();
        let bundle_no   = create_prekey_bundle(&bob_ik, &bob_spk, 1, None);
        let bundle_with = create_prekey_bundle(&bob_ik, &bob_spk, 1, Some((&bob_opk, 1)));

        let out_no   = x3dh_send(&alice_ik, &bundle_no).unwrap();
        let out_with = x3dh_send(&alice_ik, &bundle_with).unwrap();

        let sk_no   = x3dh_receive(&bob_ik, &bob_spk, None,
            &alice_ik.x25519_public, &out_no.ephemeral_public).unwrap();
        let sk_with = x3dh_receive(&bob_ik, &bob_spk, Some(&bob_opk),
            &alice_ik.x25519_public, &out_with.ephemeral_public).unwrap();

        assert_eq!(out_no.shared_key, sk_no);
        assert_eq!(out_with.shared_key, sk_with);
        assert_ne!(sk_no, sk_with);
    }

    #[test]
    fn invalid_spk_signature_rejected() {
        let alice_ik = generate_identity_keypair();
        let bob_ik   = generate_identity_keypair();
        let bob_spk  = generate_prekey_pair();
        let mut bundle = create_prekey_bundle(&bob_ik, &bob_spk, 1, None);
        bundle.signed_prekey_sig[0] ^= 0xFF;
        assert_eq!(x3dh_send(&alice_ik, &bundle).unwrap_err(), X3DHError::InvalidSignature);
    }

    #[test]
    fn wrong_identity_key_produces_different_shared_key() {
        let alice_ik = generate_identity_keypair();
        let bob_ik   = generate_identity_keypair();
        let eve_ik   = generate_identity_keypair();
        let bob_spk  = generate_prekey_pair();
        let bundle   = create_prekey_bundle(&bob_ik, &bob_spk, 1, None);

        let alice_out = x3dh_send(&alice_ik, &bundle).unwrap();
        let eve_sk    = x3dh_receive(
            &eve_ik, &bob_spk, None,
            &alice_ik.x25519_public, &alice_out.ephemeral_public,
        ).unwrap();

        assert_ne!(alice_out.shared_key, eve_sk);
    }

    #[test]
    fn two_sessions_produce_different_keys() {
        let alice_ik = generate_identity_keypair();
        let bob_ik   = generate_identity_keypair();
        let bob_spk  = generate_prekey_pair();
        let bundle   = create_prekey_bundle(&bob_ik, &bob_spk, 1, None);

        let out1 = x3dh_send(&alice_ik, &bundle).unwrap();
        let out2 = x3dh_send(&alice_ik, &bundle).unwrap();
        assert_ne!(out1.shared_key, out2.shared_key);
    }

    #[test]
    fn secret_accessor_returns_correct_value() {
        let pk = generate_prekey_pair();
        let pub_from_accessor = x25519_public_key(pk.secret());
        assert_eq!(pk.public, pub_from_accessor);
    }

    #[test]
    fn x25519_secret_accessor_roundtrips() {
        let ik = generate_identity_keypair();
        let derived_pub = x25519_public_key(&ik.x25519_secret());
        assert_eq!(ik.x25519_public, derived_pub);
    }
}
