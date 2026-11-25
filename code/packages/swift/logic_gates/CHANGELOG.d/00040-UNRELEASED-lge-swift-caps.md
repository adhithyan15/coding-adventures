## [Unreleased]

### Security

- LGE-SWIFT-CAPS: cross-language cap parity with Rust SEC-008.
- New public `maxInputs = 1 << 16`. `andN` and `orN` reject more than
  `maxInputs` inputs with `LogicGateError.invalidWidth(name: "inputs", value:)`
  BEFORE the per-element validation loop or the reduce.
- `shiftRegister` rejects `q.count > maxWidth` with
  `.invalidWidth(name: "q", value:)` before any validation or allocation.
- Already present from the previous change: `maxSelectBits`, `maxWidth`,
  `maxCounterWidth` and their checks in decoder, demux, muxN, encoder,
  priorityEncoder, register and counter. There is no `xorN` in this package.
- Arrays are value types (copy-on-write): the checked count cannot change
  between check and use, so no defensive copy is needed.

BREAKING: the bad-element payload for `andN`/`orN` is now `inputs[i]` (was
`input[i]`).

BREAKING: `andN`/`orN` with more than 65536 inputs and `shiftRegister` with
more than 65536 stages, previously accepted, now throw. Package versioning is
changelog-only (no version field in Package.swift), so no manifest bump.
