# beam-opcode-metadata

Version-aware BEAM opcode metadata for Erlang/OTP tooling.

This package exposes:

- The external generic BEAM opcode catalog
- Release-family profiles such as `otp24`, `otp28`, and `otp29`
- Profile-aware opcode lookups by number or name

It is intentionally small and reusable so decoders, disassemblers, and future
runtime implementations can all share the same opcode table.
