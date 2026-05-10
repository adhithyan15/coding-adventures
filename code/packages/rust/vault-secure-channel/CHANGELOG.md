# Changelog

All notable changes to this package are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] — 2026-05-04

### Added

- Initial implementation of VLT-CH
  (`code/specs/VLT-CH-vault-secure-channel.md`).
- Composes `coding_adventures_x3dh` (initial key agreement)
  and `coding_adventures_double_ratchet` (per-message DH ratchet
  + KDF chain) into a single ergonomic vault channel — the
  Signal-protocol-style "constant key rotation" wrapper the user
  asked for, so a snapshot of channel state at time T cannot
  decrypt messages sent before T (forward secrecy) and after one
  ratchet step cannot decrypt messages sent after T either
  (post-compromise security).
- `ChannelInitiator::open(my_identity, peer_bundle, plaintext, aad)`
  → `(Channel, FirstMessage)` — Alice's side: X3DH send + ratchet
  init + first encrypted payload, all in one call.
- `ChannelResponder::accept(first_msg, my_identity, my_spk,
  my_opk?, sender_ik_pub, aad)` → `(Channel, plaintext)` — Bob's
  side: X3DH receive + ratchet init + first plaintext, all in
  one call.
- `Channel::send(plaintext, aad)` → wire bytes; `Channel::receive(
  wire, aad)` → plaintext. Out-of-order delivery within the
  underlying double-ratchet window is supported via the cached
  skipped-message-keys mechanism upstream.
- Wire format:
  - First message: `magic(2) "C1" || ek_pub(32) || dr_header(40)
    || ct_len(4 BE) || ct` — the magic prefix lets the
    responder fail-fast on a wrong-protocol blob.
  - Subsequent: `magic(2) "CN" || dr_header(40) || ct_len(4 BE)
    || ct`.
- Caller-supplied AAD is passed through to the ratchet AEAD so
  the ciphertext is bound to application context (e.g.
  `vault_id || record_id`).
- `ChannelError` typed enum: `MalformedWire`, `X3dh`, `Ratchet`,
  `PlaintextTooLarge`. `Display` strings sourced exclusively from
  this crate's literals.
- `Channel`'s ratchet state is held in the upstream `RatchetState`
  which itself wipes on drop via `Zeroize`.
- 13 unit tests covering: open/accept first-message round-trip,
  50-step alternating-sender exchange, Alice-burst + in-order
  delivery, out-of-order-within-window recovery, replay rejection
  on subsequent messages, body tamper on first and subsequent
  messages → AEAD failure, magic tamper on both message kinds →
  `MalformedWire`, truncated-wire rejection, AAD-mismatch
  rejection, forward-secrecy property test (state after delivery
  cannot re-decrypt the same wire), and the
  Display-strings-from-literals invariant.

### Out of scope (future PRs)

- PreKeyBundle distribution (server / sync layer; VLT10 territory).
- Sealed-sender / metadata-private envelope (the magic prefix is
  observable; ephemeral pubkey on first message is observable).
- Multi-device fan-out (one channel per device pair; orchestration
  lives at a higher layer).
