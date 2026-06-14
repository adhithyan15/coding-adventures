# Changelog

## 0.3.0 — 2026-05-04 — conv.u2 opcode (TW04 Phase 4c)

- Add `CILOpcode.CONV_U2 = 0xD3` to the opcode enum.
- Add `CILBytecodeBuilder.emit_conv_u2()` convenience method.

`conv.u2` truncates/zero-extends the int32 stack top to uint16
(char), required by `System.Console.Write(char)` in Twig's inline
host-call lowering so the correct overload is resolved.

## 0.2.0

- Add `CILOpcode.XOR = 0x61` to the opcode enum (AND=0x5F and OR=0x60 were
  already present but XOR was missing).
- Add `emit_and()`, `emit_or()`, `emit_xor()` convenience methods on
  `CILBytecodeBuilder` matching the existing `emit_add()` / `emit_sub()` style.

## 0.1.0

- Initial release.
- Add compact CIL integer, local, argument, metadata-token, and branch builders.
- Add two-pass label assembly with automatic short-to-long branch promotion.
