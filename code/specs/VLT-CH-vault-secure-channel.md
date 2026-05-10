# VLT-CH — Vault Secure Channel

## Overview

The **secure-channel** layer of the Vault stack. Wraps the
already-shipped `coding_adventures_x3dh` (Signal-style initial key
agreement) and `coding_adventures_double_ratchet` (per-message DH
ratchet + KDF chain) crates into a single ergonomic vault channel.
This is the layer that gives the Vault stack the **continuous key
rotation** property the user asked for: channel takeover at time T
does not compromise messages sent before T (forward secrecy), and
after the next ratchet step does not compromise messages sent after
T either (post-compromise security).

Implementation lives at `code/packages/rust/vault-secure-channel/`.

## Why this layer exists

Storage-layer encryption (VLT01) protects records *at rest*. But
in a multi-device or client/server vault, records also flow over
the wire — and that wire needs an authenticated, encrypted, *and*
forward-secret channel. The Signal Protocol is the gold standard
for this; the Vault stack already has its primitives, but apps
shouldn't have to wire them together themselves.

## API

```rust
pub struct Channel { /* opaque */ }
pub struct FirstMessage(pub Vec<u8>);

pub struct ChannelInitiator;
impl ChannelInitiator {
    pub fn open(
        my_identity: &IdentityKeyPair,
        peer_bundle: &PreKeyBundle,
        initial_plaintext: &[u8],
        aad: &[u8],
    ) -> Result<(Channel, FirstMessage), ChannelError>;
}

pub struct ChannelResponder;
impl ChannelResponder {
    pub fn accept(
        first_message: &FirstMessage,
        my_identity: &IdentityKeyPair,
        my_signed_prekey: &PreKeyPair,
        my_one_time_prekey: Option<&PreKeyPair>,
        sender_identity_x25519_pub: &[u8; 32],
        aad: &[u8],
    ) -> Result<(Channel, Vec<u8>), ChannelError>;
}

impl Channel {
    pub fn send(&mut self, plaintext: &[u8], aad: &[u8]) -> Result<Vec<u8>, ChannelError>;
    pub fn receive(&mut self, wire: &[u8], aad: &[u8]) -> Result<Vec<u8>, ChannelError>;
}
```

## Composition

```text
         INITIATOR (Alice)                      RESPONDER (Bob)

  open(my_ik, peer_bundle, pt, aad)        accept(first, my_ik, my_spk, my_opk?,
         │                                          sender_ik_pub, aad)
         ▼                                          │
   x3dh_send(my_ik, peer_bundle)                    ▼
         │                                   x3dh_receive(my_ik, my_spk,
         ▼ shared_key                                my_opk?, sender_ik_pub,
                                                    sender_ek_pub)
         │                                          │
         ▼                                          ▼ shared_key
   ratchet_init_alice(shared_key,
                      peer_signed_prekey)     ratchet_init_bob(shared_key,
         │                                                     KeyPair::from_secret(my_spk.secret))
         ▼                                          │
   ratchet_encrypt(initial_pt, aad) → DrMessage     ▼
         │                                   ratchet_decrypt(parsed_dr_msg, aad) → plaintext
         ▼                                          │
   FirstMessage{ "C1" || ek_pub || dr_header        ▼
                 || ct_len || ct }            (Channel, plaintext)
         │
         ▼
     (Channel, FirstMessage)
```

After the first exchange, every `Channel::send` advances the
sending chain; whenever the sender changes, the underlying
double-ratchet performs a DH ratchet step (re-keying both chains
under fresh ECDH output).

## Wire format

### `FirstMessage` (94 bytes + ct)

```text
   first_msg = magic(2) "C1" || ek_pub(32) || dr_header(40) || ct_len(4 BE) || ct
```

- `magic` lets the responder fail-fast on a wrong-protocol blob.
- `ek_pub` is Alice's X3DH ephemeral public key — Bob needs it to
  compute his side of the shared secret.
- `dr_header` is the upstream double-ratchet's first header
  (40 bytes per `coding_adventures_double_ratchet::HEADER_LEN`).
- `ct_len` is the big-endian byte length of the ciphertext.
- `ct` is the ratchet-encrypted plaintext + 16-byte tag.

### Subsequent messages (62 bytes + ct)

```text
   wire = magic(2) "CN" || dr_header(40) || ct_len(4 BE) || ct
```

### AAD

Caller-supplied `aad` is passed through to the ratchet's AEAD on
both encrypt and decrypt, so the ciphertext is bound to
application context (`vault_id || record_id`, etc.). Mismatch =
AEAD failure on receive.

## Security properties

- **Confidentiality + integrity per message.** Every ciphertext
  carries a Poly1305 tag (via the upstream double-ratchet's AEAD).
- **Forward secrecy.** Each message uses a freshly-derived key
  from a one-way KDF chain. A snapshot of state immediately after
  successful decrypt of message `n` cannot recover the plaintext
  of message `n` or earlier.
- **Post-compromise security.** Each DH ratchet step re-keys the
  symmetric chain under fresh ECDH output. Once a clean ratchet
  step completes, even an attacker who briefly held the prior
  state cannot decrypt new messages.
- **Out-of-order delivery within a window.** The upstream
  double-ratchet caches up to `MAX_SKIP` (1000) skipped message
  keys per chain so reordered messages decrypt successfully.
- **Replay rejection.** Once a message key is consumed, the
  upstream skipped-keys map drops it; replay returns
  `ChannelError::Ratchet`.
- **Magic-prefix rejection.** Wrong-protocol blobs fail-fast as
  `MalformedWire` before the AEAD path runs.

## Threat model & test coverage

| Threat                                                                | Defence                                                               | Test                                                          |
|-----------------------------------------------------------------------|-----------------------------------------------------------------------|---------------------------------------------------------------|
| Adversary observes wire and tries to decrypt                          | AEAD per message; keys never on the wire                              | All round-trip tests (13/13)                                  |
| Adversary takes a snapshot of state at time T                         | Forward secrecy via KDF chain; PCS via DH ratchet                     | `forward_secrecy_state_after_send_cannot_re_decrypt_old`      |
| Adversary replays a captured ciphertext                               | Skipped-keys window drops consumed keys                               | `replay_of_previously_received_message_fails`                 |
| Adversary tampers the body                                            | AEAD tag fails                                                        | `body_tamper_in_subsequent_message_fails`, `…_in_first_message_fails` |
| Adversary tampers the magic prefix                                    | Pre-AEAD `MalformedWire` rejection                                    | `first_message_magic_tamper_rejected`, `next_message_magic_tamper_rejected` |
| Adversary truncates the wire                                          | `MalformedWire`                                                       | `truncated_wire_is_malformed`                                 |
| Adversary sends with mismatched AAD                                   | AEAD verify fails on receive                                          | `wrong_aad_on_receive_fails`                                  |
| Out-of-order delivery within window                                   | Skipped-keys cache                                                    | `out_of_order_within_window_is_recovered`, `alice_burst_messages_received_in_order` |
| Burst sends without reply                                             | Independent sender chain; works                                       | `alice_burst_messages_received_in_order`                      |
| 50-step alternating exchange (DH ratchet stress)                      | Each sender flip triggers ratchet step                                | `many_send_receive_steps`                                     |
| Attacker-controlled bytes in error logs                               | All `Display` strings are static literals                             | `error_messages_are_static_literals`                          |

## Out of scope (future PRs)

- **PreKeyBundle distribution.** That's a server / sync concern
  (VLT10 + VLT11). Apps fetch the bundle out-of-band and call
  `ChannelInitiator::open`.
- **Sealed sender / metadata privacy.** The first message's `C1`
  magic and `ek_pub` are observable; an on-path adversary learns
  the channel kind and the ephemeral pubkey. Sealed-sender style
  metadata privacy is a separate primitive layered on top.
- **Multi-device fan-out.** One channel per device pair; the
  orchestration of "which device gets which message" lives at a
  higher layer.

## Citations

- Signal Protocol — X3DH ([Marlinspike & Perrin, 2016](https://signal.org/docs/specifications/x3dh/))
  and Double Ratchet ([Marlinspike & Perrin, 2016](https://signal.org/docs/specifications/doubleratchet/)).
- VLT00-vault-roadmap.md — VLT-CH layer purpose.
- `coding_adventures_x3dh` — the upstream X3DH primitive.
- `coding_adventures_double_ratchet` — the upstream Double Ratchet
  primitive (KDF chain, DH ratchet, MAX_SKIP=1000 message-key
  cache, MAX_SKIPPED_KEYS_TOTAL=5000).
