# Changelog

## 0.2.0

- Add `CILOpcode.XOR = 0x61` to the opcode enum (AND=0x5F and OR=0x60 were
  already present but XOR was missing).
- Add `emit_and()`, `emit_or()`, `emit_xor()` convenience methods on
  `CILBytecodeBuilder` matching the existing `emit_add()` / `emit_sub()` style.

## 0.1.0

- Initial release.
- Add compact CIL integer, local, argument, metadata-token, and branch builders.
- Add two-pass label assembly with automatic short-to-long branch promotion.
