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
