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
- Security review, before first release: `decode_name` (and, symmetrically,
  `encode_name`) now enforce the exact character set `vault-pm-config::
  ConfigName` already enforces — ASCII alphanumeric, or `_`/`-` past the
  first byte — rather than merely a length bound and valid UTF-8. The socket
  this protocol runs over authenticates a peer only as "the same local
  user," never as "the genuine `vault-pm` binary," so any same-user process
  could otherwise send a hand-crafted `Unlock` naming a vault containing a
  quote, a backslash, or a raw terminal escape sequence — later rendered
  unescaped into `agent status`'s plain-text and `--json` output.
