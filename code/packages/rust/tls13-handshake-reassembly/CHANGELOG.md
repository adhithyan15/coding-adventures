# Changelog

## 0.1.0 - 2026-11-17

- Add allocation-free reassembly of one TLS 1.3 handshake message across
  already authenticated record-content fragments in caller-owned storage.
- Stop at the first complete message and report the exact consumed input count
  so the caller retains every coalesced byte in order.
- Validate framing policy as soon as the header is complete, reject declarations
  beyond storage capacity, clear buffered bytes on reset, reuse, failure, and
  drop, and expose only payload-free stable failures.
- Declare an empty capability set because the package performs no external or
  credential effect.
