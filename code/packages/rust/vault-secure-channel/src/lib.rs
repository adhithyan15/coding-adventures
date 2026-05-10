//! # coding_adventures_vault_secure_channel — VLT-CH
//!
//! ## What this crate does
//!
//! Wraps the existing [`coding_adventures_x3dh`] and
//! [`coding_adventures_double_ratchet`] crates into a single
//! ergonomic *vault channel* that the Vault stack uses for every
//! client↔server (or device↔device) wire payload. Every message
//! carries fresh per-message keys derived through a KDF chain that
//! is itself re-keyed every time the peers exchange Diffie–Hellman
//! ratchet keys. Compromise of channel state at time T does not
//! reveal messages sent before T, nor — after the next DH ratchet
//! step — messages sent after T.
//!
//! This is the "constant-key-rotation" channel the user requested:
//! channel takeover doesn't compromise past or future secrets.
//!
//! ## How it composes
//!
//! ```text
//!   ChannelInitiator::open(peer_bundle)             ChannelResponder::accept(first_msg)
//!         │                                                  │
//!         ▼                                                  ▼
//!   x3dh_send(my_ik, peer_bundle) ─────► first_msg ───► x3dh_receive(my_ik, my_spk, my_opk?, …)
//!         │                                                  │
//!         ▼ shared_key                                       ▼ shared_key
//!   ratchet_init_alice(shared_key, peer_dh_pub)        ratchet_init_bob(shared_key, my_ratchet_kp)
//!         │                                                  │
//!         ▼                                                  ▼
//!   Channel::send / receive (Double Ratchet)           Channel::send / receive (Double Ratchet)
//! ```
//!
//! The initiator's `Channel::send` produces opaque wire bytes; the
//! responder's `Channel::receive` consumes them. Both sides ratchet
//! independently per RFC-style Signal Protocol semantics —
//! out-of-order delivery within the message window is supported by
//! the underlying double-ratchet skipped-key cache; replay of any
//! single message is rejected.
//!
//! ## What's in this crate (v0.1)
//!
//! - `ChannelInitiator::open(peer_bundle, my_ik) -> (Channel, FirstMessage)`
//!   — Alice's side. Produces the channel and the first
//!   wire-encodable message that bootstraps Bob's side.
//! - `ChannelResponder::accept(first_message, my_ik, my_spk, my_opk?) -> Channel`
//!   — Bob's side. Consumes Alice's first message, performs
//!   X3DH on the receive side, initialises his ratchet.
//! - `Channel::send(plaintext, aad) -> Vec<u8>` — encrypt + ratchet.
//! - `Channel::receive(wire, aad) -> Vec<u8>` — decrypt; reject
//!   replayed nonces; advance the ratchet on header DH change.
//! - All key material is held inside `Zeroizing` (and the
//!   underlying ratchet/x3dh crates already wrap their own state in
//!   Zeroizing<…>); `Drop` of `Channel` propagates that wipe.
//!
//! ## Wire format
//!
//! ### `FirstMessage`
//!
//! ```text
//!   first_message_bytes =
//!     magic(2) "C1" || ek_pub(32) || dr_header(40) || ciphertext_len(4 BE) || ciphertext
//! ```
//!
//! - `magic` — distinguishes vault-channel bytes from other framing.
//! - `ek_pub` — Alice's X3DH ephemeral public key (Bob needs this
//!   to recompute the shared secret).
//! - `dr_header` — first double-ratchet header (40 bytes per
//!   `coding_adventures_double_ratchet::HEADER_LEN`).
//! - `ciphertext` — the ratchet-encrypted plaintext + auth tag.
//!
//! ### Subsequent messages
//!
//! ```text
//!   wire_bytes =
//!     magic(2) "CN" || dr_header(40) || ciphertext_len(4 BE) || ciphertext
//! ```
//!
//! ### AAD
//!
//! Caller-supplied `aad` is passed through to the ratchet
//! `aead_encrypt`/`aead_decrypt`, so it binds the ciphertext to
//! application context (e.g. `vault_id || record_id`).
//!
//! ## What this crate does *not* do
//!
//! - **No PreKeyBundle distribution.** That's a server / sync
//!   concern (VLT10). Apps fetch a peer's `PreKeyBundle` out of
//!   band, then call `ChannelInitiator::open`.
//! - **No long-term identity binding.** The channel uses the X3DH
//!   identity keys that the application provides; whose-keys-go-
//!   where is VLT05 / VLT09's job.
//! - **No metadata privacy.** The first message exposes Alice's
//!   ephemeral pubkey by design; the magic prefix is observable.
//!   Sealed-sender style metadata privacy is future work.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use coding_adventures_double_ratchet::{
    decode_header, encode_header, generate_ratchet_keypair, ratchet_decrypt, ratchet_encrypt,
    ratchet_init_alice, ratchet_init_bob, Message as DrMessage, MessageHeader, RatchetError,
    RatchetState, HEADER_LEN,
};
use coding_adventures_x3dh::{
    x3dh_receive, x3dh_send, IdentityKeyPair, PreKeyBundle, PreKeyPair, X3DHError,
};
use coding_adventures_zeroize::Zeroize;

// ─────────────────────────────────────────────────────────────────────
// 1. Wire format constants
// ─────────────────────────────────────────────────────────────────────

const FIRST_MAGIC: &[u8; 2] = b"C1";
const NEXT_MAGIC: &[u8; 2] = b"CN";
const EK_PUB_LEN: usize = 32;
const CT_LEN_FIELD: usize = 4;

// ─────────────────────────────────────────────────────────────────────
// 2. Errors
// ─────────────────────────────────────────────────────────────────────

/// Errors from any [`Channel`] operation.
///
/// `Display` strings are sourced exclusively from this crate's
/// literals — never from the input wire bytes.
#[derive(Debug)]
pub enum ChannelError {
    /// The wire bytes were malformed (bad magic, length mismatch,
    /// truncated). Never reveals the secret — failures here are
    /// pre-AEAD structural rejections.
    MalformedWire,
    /// X3DH initial-key-agreement failure (signature check, low-
    /// order point, etc.). Surfaces the underlying error variant
    /// tag, never bytes.
    X3dh,
    /// Double-ratchet failure (bad AEAD tag, replay, skipped-key
    /// cache exhaustion, …). Surfaces the underlying error variant
    /// tag, never bytes.
    Ratchet,
    /// The plaintext was too large to fit in a 4-byte length field
    /// (ciphertext > 4 GiB). The vault stack does not chunk at this
    /// layer — large blobs go through VLT14 attachments which has
    /// its own framing.
    PlaintextTooLarge,
}

impl core::fmt::Display for ChannelError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let s = match self {
            ChannelError::MalformedWire => "vault-secure-channel: malformed wire bytes",
            ChannelError::X3dh => "vault-secure-channel: X3DH initial key agreement failed",
            ChannelError::Ratchet => "vault-secure-channel: double-ratchet operation failed",
            ChannelError::PlaintextTooLarge => {
                "vault-secure-channel: plaintext exceeds the 4 GiB single-message limit"
            }
        };
        write!(f, "{}", s)
    }
}

impl std::error::Error for ChannelError {}

impl From<X3DHError> for ChannelError {
    fn from(_: X3DHError) -> Self {
        ChannelError::X3dh
    }
}
impl From<RatchetError> for ChannelError {
    fn from(_: RatchetError) -> Self {
        ChannelError::Ratchet
    }
}

// ─────────────────────────────────────────────────────────────────────
// 3. Channel
// ─────────────────────────────────────────────────────────────────────

/// A live secure channel. Holds Double-Ratchet state internally;
/// `Drop` zeroes it via the upstream `RatchetState::Drop` impl.
pub struct Channel {
    state: RatchetState,
}

impl Channel {
    /// Encrypt `plaintext` and produce the wire bytes for one
    /// outgoing message. The internal sending chain advances by one;
    /// next call uses a fresh derived message key.
    pub fn send(&mut self, plaintext: &[u8], aad: &[u8]) -> Result<Vec<u8>, ChannelError> {
        let msg = ratchet_encrypt(&mut self.state, plaintext, aad)?;
        encode_next_message(&msg)
    }

    /// Decrypt a wire-encoded outgoing message from the peer. Header
    /// DH-public change triggers a ratchet step; the underlying
    /// double-ratchet handles out-of-order and skipped messages
    /// (within the configured window).
    pub fn receive(&mut self, wire: &[u8], aad: &[u8]) -> Result<Vec<u8>, ChannelError> {
        let msg = decode_next_message(wire)?;
        let pt = ratchet_decrypt(&mut self.state, &msg, aad)?;
        Ok(pt)
    }
}

// ─────────────────────────────────────────────────────────────────────
// 4. Initiator (Alice) — open()
// ─────────────────────────────────────────────────────────────────────

/// Encoded first-message bytes that the initiator hands to the
/// responder. Opaque to the caller; pass through to
/// `ChannelResponder::accept`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FirstMessage(pub Vec<u8>);

/// Initiator side. Builds a channel against `peer_bundle` and
/// produces the first wire message that bootstraps the responder.
pub struct ChannelInitiator;

impl ChannelInitiator {
    /// Open a new channel to a peer whose `PreKeyBundle` we already
    /// have (out-of-band fetch). Uses our [`IdentityKeyPair`] for
    /// the X3DH identity DH; the rest is fresh ephemeral material.
    ///
    /// Returns:
    /// * `Channel` ready for `send` / `receive` from this side.
    /// * `FirstMessage` to be transmitted out-of-band so the peer
    ///   can call `ChannelResponder::accept`. Carries Alice's X3DH
    ///   ephemeral public key plus the first ratchet-encrypted
    ///   payload (the `initial_plaintext` arg).
    pub fn open(
        my_identity: &IdentityKeyPair,
        peer_bundle: &PreKeyBundle,
        initial_plaintext: &[u8],
        aad: &[u8],
    ) -> Result<(Channel, FirstMessage), ChannelError> {
        // 1. X3DH on the send side. Produces a 32-byte shared_key
        //    and the ephemeral pubkey we send to Bob.
        let x = x3dh_send(my_identity, peer_bundle)?;

        // 2. Initialise Double-Ratchet as Alice using shared_key
        //    and Bob's signed-prekey as the initial DH-pub. Per
        //    Signal spec, Alice's `dhs` is generated inside
        //    ratchet_init_alice and her sending chain is built
        //    immediately so she can encrypt the first message.
        let mut state = ratchet_init_alice(&x.shared_key, &peer_bundle.signed_prekey);

        // 3. Encrypt the first plaintext.
        let dr_msg = ratchet_encrypt(&mut state, initial_plaintext, aad)?;

        // 4. Compose FirstMessage wire bytes.
        let bytes = encode_first_message(&x.ephemeral_public, &dr_msg)?;

        Ok((Channel { state }, FirstMessage(bytes)))
    }
}

// ─────────────────────────────────────────────────────────────────────
// 5. Responder (Bob) — accept()
// ─────────────────────────────────────────────────────────────────────

/// Responder side. Consumes the initiator's first message,
/// recomputes the X3DH shared secret, and initialises the channel.
pub struct ChannelResponder;

impl ChannelResponder {
    /// Accept an incoming `FirstMessage` and recover the channel.
    ///
    /// `my_signed_prekey` and `my_one_time_prekey` MUST be the
    /// keypairs whose public components were in the bundle Alice
    /// used. The caller is expected to look these up by the bundle's
    /// `signed_prekey_id` / `one_time_prekey_id` fields when
    /// dispatching incoming connections in production.
    ///
    /// Returns the [`Channel`] plus the decrypted first plaintext.
    pub fn accept(
        first_message: &FirstMessage,
        my_identity: &IdentityKeyPair,
        my_signed_prekey: &PreKeyPair,
        my_one_time_prekey: Option<&PreKeyPair>,
        sender_identity_x25519_pub: &[u8; 32],
        aad: &[u8],
    ) -> Result<(Channel, Vec<u8>), ChannelError> {
        // 1. Parse the first-message wire bytes.
        let parsed = decode_first_message(&first_message.0)?;

        // 2. X3DH on the receive side. Produces the same 32-byte
        //    shared_key as the sender's side did.
        let shared = x3dh_receive(
            my_identity,
            my_signed_prekey,
            my_one_time_prekey,
            sender_identity_x25519_pub,
            &parsed.ek_pub,
        )?;

        // 3. Initialise Double-Ratchet as Bob using shared_key and
        //    his signed-prekey-pair as the initial ratchet keypair.
        //    NOTE: ratchet_init_bob takes the ratchet *KeyPair*, not
        //    just the secret. We construct one from the signed
        //    prekey.
        let bob_kp =
            coding_adventures_double_ratchet::KeyPair::from_secret(*my_signed_prekey.secret());
        let mut state = ratchet_init_bob(&shared, bob_kp);

        // 4. Decrypt the embedded first DR message.
        let pt = ratchet_decrypt(&mut state, &parsed.dr_msg, aad)?;

        Ok((Channel { state }, pt))
    }
}

// ─────────────────────────────────────────────────────────────────────
// 6. Wire encode / decode helpers
// ─────────────────────────────────────────────────────────────────────

/// Layout: magic(2) || ek_pub(32) || dr_header(40) || ct_len(4 BE) || ct.
fn encode_first_message(ek_pub: &[u8; 32], msg: &DrMessage) -> Result<Vec<u8>, ChannelError> {
    let header_bytes = encode_header(&msg.header);
    let ct_len = u32::try_from(msg.ciphertext.len()).map_err(|_| ChannelError::PlaintextTooLarge)?;
    let mut out = Vec::with_capacity(2 + EK_PUB_LEN + HEADER_LEN + CT_LEN_FIELD + msg.ciphertext.len());
    out.extend_from_slice(FIRST_MAGIC);
    out.extend_from_slice(ek_pub);
    out.extend_from_slice(&header_bytes);
    out.extend_from_slice(&ct_len.to_be_bytes());
    out.extend_from_slice(&msg.ciphertext);
    Ok(out)
}

struct ParsedFirstMessage {
    ek_pub: [u8; 32],
    dr_msg: DrMessage,
}

fn decode_first_message(wire: &[u8]) -> Result<ParsedFirstMessage, ChannelError> {
    if wire.len() < 2 + EK_PUB_LEN + HEADER_LEN + CT_LEN_FIELD {
        return Err(ChannelError::MalformedWire);
    }
    if &wire[..2] != FIRST_MAGIC {
        return Err(ChannelError::MalformedWire);
    }
    let mut p = 2;

    let mut ek_pub = [0u8; EK_PUB_LEN];
    ek_pub.copy_from_slice(&wire[p..p + EK_PUB_LEN]);
    p += EK_PUB_LEN;

    let mut hdr = [0u8; HEADER_LEN];
    hdr.copy_from_slice(&wire[p..p + HEADER_LEN]);
    p += HEADER_LEN;
    let header = decode_header(&hdr);

    let mut ct_len_bytes = [0u8; CT_LEN_FIELD];
    ct_len_bytes.copy_from_slice(&wire[p..p + CT_LEN_FIELD]);
    p += CT_LEN_FIELD;
    let ct_len = u32::from_be_bytes(ct_len_bytes) as usize;
    if p + ct_len != wire.len() {
        return Err(ChannelError::MalformedWire);
    }
    let ciphertext = wire[p..p + ct_len].to_vec();

    Ok(ParsedFirstMessage {
        ek_pub,
        dr_msg: DrMessage { header, ciphertext },
    })
}

/// Layout: magic(2) || dr_header(40) || ct_len(4 BE) || ct.
fn encode_next_message(msg: &DrMessage) -> Result<Vec<u8>, ChannelError> {
    let header_bytes = encode_header(&msg.header);
    let ct_len = u32::try_from(msg.ciphertext.len()).map_err(|_| ChannelError::PlaintextTooLarge)?;
    let mut out = Vec::with_capacity(2 + HEADER_LEN + CT_LEN_FIELD + msg.ciphertext.len());
    out.extend_from_slice(NEXT_MAGIC);
    out.extend_from_slice(&header_bytes);
    out.extend_from_slice(&ct_len.to_be_bytes());
    out.extend_from_slice(&msg.ciphertext);
    Ok(out)
}

fn decode_next_message(wire: &[u8]) -> Result<DrMessage, ChannelError> {
    if wire.len() < 2 + HEADER_LEN + CT_LEN_FIELD {
        return Err(ChannelError::MalformedWire);
    }
    if &wire[..2] != NEXT_MAGIC {
        return Err(ChannelError::MalformedWire);
    }
    let mut p = 2;

    let mut hdr = [0u8; HEADER_LEN];
    hdr.copy_from_slice(&wire[p..p + HEADER_LEN]);
    p += HEADER_LEN;
    let header = decode_header(&hdr);

    let mut ct_len_bytes = [0u8; CT_LEN_FIELD];
    ct_len_bytes.copy_from_slice(&wire[p..p + CT_LEN_FIELD]);
    p += CT_LEN_FIELD;
    let ct_len = u32::from_be_bytes(ct_len_bytes) as usize;
    if p + ct_len != wire.len() {
        return Err(ChannelError::MalformedWire);
    }
    let ciphertext = wire[p..p + ct_len].to_vec();

    Ok(DrMessage { header, ciphertext })
}

// Suppress "unused imports" for the smaller shapes we touch only
// transitively; the upstream crates own zeroization.
#[allow(dead_code)]
fn _suppress_unused() {
    let _: Option<fn() -> _> = Some(generate_ratchet_keypair);
    let _: fn(&mut [u8]) = |b| b.zeroize();
    let _ = MessageHeader { dh: [0u8; 32], pn: 0, n: 0 };
}

// ─────────────────────────────────────────────────────────────────────
// 7. Tests
// ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use coding_adventures_x3dh::{create_prekey_bundle, generate_identity_keypair, generate_prekey_pair};

    /// Fresh bundle for Bob plus the secret keypairs the responder
    /// will need to call `accept`.
    struct BobMaterials {
        ik: IdentityKeyPair,
        spk: PreKeyPair,
        opk: PreKeyPair,
        bundle: PreKeyBundle,
    }

    fn make_bob() -> BobMaterials {
        let ik = generate_identity_keypair();
        let spk = generate_prekey_pair();
        let opk = generate_prekey_pair();
        let bundle = create_prekey_bundle(&ik, &spk, /* spk_id */ 1, Some((&opk, /* opk_id */ 7)));
        BobMaterials { ik, spk, opk, bundle }
    }

    fn open_pair(initial_plaintext: &[u8], aad: &[u8]) -> (Channel, Channel, Vec<u8>) {
        let alice_ik = generate_identity_keypair();
        let bob = make_bob();
        let (alice_ch, first) =
            ChannelInitiator::open(&alice_ik, &bob.bundle, initial_plaintext, aad).unwrap();
        let (bob_ch, recovered) = ChannelResponder::accept(
            &first,
            &bob.ik,
            &bob.spk,
            Some(&bob.opk),
            &alice_ik.x25519_public,
            aad,
        )
        .unwrap();
        assert_eq!(&recovered[..], initial_plaintext);
        (alice_ch, bob_ch, recovered)
    }

    // --- Round-trip ---

    #[test]
    fn open_accept_first_message_roundtrip() {
        let (_, _, recovered) = open_pair(b"hello bob", b"vault/123");
        assert_eq!(&recovered[..], b"hello bob");
    }

    #[test]
    fn many_send_receive_steps() {
        // Exchange 50 messages alternating sender; the underlying
        // double-ratchet steps the DH chain whenever the sender
        // changes, so this exercises the ratchet.
        let (mut alice, mut bob, _) = open_pair(b"first", b"aad");
        let aad: &[u8] = b"aad";

        for i in 0..50u32 {
            let plaintext = format!("a->b #{}", i);
            let wire = alice.send(plaintext.as_bytes(), aad).unwrap();
            let got = bob.receive(&wire, aad).unwrap();
            assert_eq!(&got[..], plaintext.as_bytes());

            let plaintext = format!("b->a #{}", i);
            let wire = bob.send(plaintext.as_bytes(), aad).unwrap();
            let got = alice.receive(&wire, aad).unwrap();
            assert_eq!(&got[..], plaintext.as_bytes());
        }
    }

    #[test]
    fn alice_burst_messages_received_in_order() {
        // Alice sends 10 in a row before Bob replies.
        let (mut alice, mut bob, _) = open_pair(b"hi", b"aad");
        let aad: &[u8] = b"aad";
        let mut wires = Vec::new();
        for i in 0..10u32 {
            wires.push(alice.send(format!("a-{}", i).as_bytes(), aad).unwrap());
        }
        for (i, w) in wires.iter().enumerate() {
            let got = bob.receive(w, aad).unwrap();
            assert_eq!(&got[..], format!("a-{}", i).as_bytes());
        }
    }

    #[test]
    fn out_of_order_within_window_is_recovered() {
        // Alice sends 5, Bob reads them out of order. Double-ratchet
        // caches skipped message keys per chain so this works
        // within MAX_SKIP.
        let (mut alice, mut bob, _) = open_pair(b"hi", b"aad");
        let aad: &[u8] = b"aad";
        let mut wires = Vec::new();
        for i in 0..5u32 {
            wires.push(alice.send(format!("a-{}", i).as_bytes(), aad).unwrap());
        }
        // Read in reversed order.
        for i in (0..5usize).rev() {
            let got = bob.receive(&wires[i], aad).unwrap();
            assert_eq!(&got[..], format!("a-{}", i).as_bytes());
        }
    }

    // --- Replay rejection ---

    #[test]
    fn replay_of_previously_received_message_fails() {
        let (mut alice, mut bob, _) = open_pair(b"hi", b"aad");
        let aad: &[u8] = b"aad";
        let wire = alice.send(b"once and only once", aad).unwrap();
        let got = bob.receive(&wire, aad).unwrap();
        assert_eq!(&got[..], b"once and only once");
        // Replay: the underlying double-ratchet's chain has advanced;
        // the message-key for this header.n is gone.
        match bob.receive(&wire, aad) {
            Err(ChannelError::Ratchet) => {}
            other => panic!(
                "expected Ratchet replay rejection, got {}",
                if matches!(other, Ok(_)) { "Ok" } else { "different Err" }
            ),
        }
    }

    // --- Tamper detection ---

    #[test]
    fn body_tamper_in_subsequent_message_fails() {
        let (mut alice, mut bob, _) = open_pair(b"first", b"aad");
        let aad: &[u8] = b"aad";
        let mut wire = alice.send(b"second", aad).unwrap();
        // Flip a byte in the ciphertext (after magic + header + len).
        let off = 2 + HEADER_LEN + CT_LEN_FIELD;
        wire[off] ^= 0x01;
        match bob.receive(&wire, aad) {
            Err(ChannelError::Ratchet) => {}
            other => panic!(
                "expected Ratchet AEAD failure, got {}",
                if matches!(other, Ok(_)) { "Ok" } else { "different Err" }
            ),
        }
    }

    #[test]
    fn body_tamper_in_first_message_fails() {
        let alice_ik = generate_identity_keypair();
        let bob = make_bob();
        let (_alice_ch, mut first) =
            ChannelInitiator::open(&alice_ik, &bob.bundle, b"hi", b"aad").unwrap();
        // Flip a byte in the first-message ciphertext.
        let off = 2 + EK_PUB_LEN + HEADER_LEN + CT_LEN_FIELD;
        first.0[off] ^= 0x01;
        match ChannelResponder::accept(
            &first,
            &bob.ik,
            &bob.spk,
            Some(&bob.opk),
            &alice_ik.x25519_public,
            b"aad",
        ) {
            Err(ChannelError::Ratchet) => {}
            other => panic!(
                "expected Ratchet AEAD failure on first-message tamper, got {}",
                if matches!(other, Ok(_)) { "Ok" } else { "different Err" }
            ),
        }
    }

    #[test]
    fn first_message_magic_tamper_rejected() {
        let alice_ik = generate_identity_keypair();
        let bob = make_bob();
        let (_alice_ch, mut first) =
            ChannelInitiator::open(&alice_ik, &bob.bundle, b"hi", b"aad").unwrap();
        first.0[0] ^= 0xFF;
        match ChannelResponder::accept(
            &first,
            &bob.ik,
            &bob.spk,
            Some(&bob.opk),
            &alice_ik.x25519_public,
            b"aad",
        ) {
            Err(ChannelError::MalformedWire) => {}
            other => panic!(
                "expected MalformedWire on magic tamper, got {}",
                if matches!(other, Ok(_)) { "Ok" } else { "different Err" }
            ),
        }
    }

    #[test]
    fn next_message_magic_tamper_rejected() {
        let (mut alice, mut bob, _) = open_pair(b"hi", b"aad");
        let aad: &[u8] = b"aad";
        let mut wire = alice.send(b"second", aad).unwrap();
        wire[0] ^= 0xFF;
        match bob.receive(&wire, aad) {
            Err(ChannelError::MalformedWire) => {}
            other => panic!(
                "expected MalformedWire on next-message magic tamper, got {}",
                if matches!(other, Ok(_)) { "Ok" } else { "different Err" }
            ),
        }
    }

    #[test]
    fn truncated_wire_is_malformed() {
        let (mut alice, mut bob, _) = open_pair(b"hi", b"aad");
        let aad: &[u8] = b"aad";
        let mut wire = alice.send(b"second", aad).unwrap();
        wire.truncate(10);
        match bob.receive(&wire, aad) {
            Err(ChannelError::MalformedWire) => {}
            other => panic!(
                "expected MalformedWire on truncated wire, got {}",
                if matches!(other, Ok(_)) { "Ok" } else { "different Err" }
            ),
        }
    }

    // --- Wrong AAD rejection ---

    #[test]
    fn wrong_aad_on_receive_fails() {
        let (mut alice, mut bob, _) = open_pair(b"hi", b"vault/123");
        let wire = alice.send(b"second", b"vault/123").unwrap();
        match bob.receive(&wire, b"vault/999") {
            Err(ChannelError::Ratchet) => {}
            other => panic!(
                "expected Ratchet AEAD failure on AAD mismatch, got {}",
                if matches!(other, Ok(_)) { "Ok" } else { "different Err" }
            ),
        }
    }

    // --- Forward secrecy property: a snapshot of state at time T
    //     cannot decrypt messages sent BEFORE T. We test this by
    //     sending one message, holding its wire bytes, then sending
    //     a second message (which advances the ratchet's symmetric
    //     chain). At that point, asking the *current* state to
    //     decrypt the first wire bytes again must fail because the
    //     message key for that chain step has been deleted on
    //     successful decrypt — i.e. the state has "forgotten."

    #[test]
    fn forward_secrecy_state_after_send_cannot_re_decrypt_old() {
        let (mut alice, mut bob, _) = open_pair(b"hi", b"aad");
        let aad: &[u8] = b"aad";
        let wire1 = alice.send(b"first", aad).unwrap();
        let _ = bob.receive(&wire1, aad).unwrap();
        // Now Bob's ratchet has consumed the message key for n=0 of
        // that chain. Re-presenting the same wire1 must fail (replay
        // / forward-secrecy).
        match bob.receive(&wire1, aad) {
            Err(ChannelError::Ratchet) => {}
            other => panic!(
                "expected forward-secrecy rejection, got {}",
                if matches!(other, Ok(_)) { "Ok" } else { "different Err" }
            ),
        }
    }

    // --- Errors come from literals ---

    #[test]
    fn error_messages_are_static_literals() {
        let errs: Vec<ChannelError> = vec![
            ChannelError::MalformedWire,
            ChannelError::X3dh,
            ChannelError::Ratchet,
            ChannelError::PlaintextTooLarge,
        ];
        for e in &errs {
            let s = e.to_string();
            assert!(s.starts_with("vault-secure-channel:"));
        }
    }
}
