# Changelog

## 0.1.0

- Added the VLT-PM48 local-agent wire format: `AgentRequest` (`Ping`,
  `Unlock`, `GetPassphrase`, `Lock`, `Status`, `Shutdown`) and
  `AgentResponse` (`Ok`, `Passphrase`, `NotRetained`, `Status`, `Err`), with
  bounded, hand-rolled binary encode/decode and no I/O of any kind.
- Added closed, single-value `ProtocolError` decoding: a version mismatch, an
  unknown tag, a truncated field, or trailing bytes are all refused rather
  than tolerated.
- Added redacted `Debug` implementations for `AgentRequest` and
  `AgentResponse` so a carried passphrase can never reach a log or a panic
  message.
- Security review, before first release: `encode` on both `AgentRequest` and
  `AgentResponse` now returns `Zeroizing<Vec<u8>>` instead of a plain `Vec<u8>`
  — an encoded `Unlock` or `Passphrase` message contains a passphrase in
  plaintext, and the buffer holding it is now wiped on drop like every other
  passphrase buffer in this product, rather than left for the allocator to
  hand back unscrubbed.
- Security review, before first release (a regression of the finding above,
  caught on re-review): `Zeroizing` alone did not actually stop the leak.
  Both `encode` functions built their buffer incrementally from a length-1
  start via `push`/`extend_from_slice`, so `Vec`'s ordinary growth
  reallocation copied the already-written plaintext passphrase into a new
  allocation and freed the old one, unscrubbed, through the global
  allocator — reproducible for roughly three-quarters of realistic
  passphrase lengths. Both functions now compute the exact byte length of
  the specific message being encoded (`encoded_capacity`) and reserve it
  once, before writing anything, so the buffer that ends up holding a
  passphrase is the only allocation that ever holds it. A `debug_assert_eq!`
  inside `encode` catches a future field added without updating the matching
  capacity computation in every debug build and test run, and
  `encode_never_reallocates_a_buffer_already_holding_a_secret` sweeps
  passphrase lengths 0..=200 to prove it empirically.
- Security review, before first release: `decode_name` (and, symmetrically,
  `encode_name`) now enforce the exact character set `vault-pm-config::
  ConfigName` already enforces — ASCII alphanumeric, or `_`/`-` past the
  first byte — rather than merely a length bound and valid UTF-8. The socket
  this protocol runs over authenticates a peer only as "the same local
  user," never as "the genuine `vault-pm` binary," so any same-user process
  could otherwise send a hand-crafted `Unlock` naming a vault containing a
  quote, a backslash, or a raw terminal escape sequence — later rendered
  unescaped into `agent status`'s plain-text and `--json` output.
