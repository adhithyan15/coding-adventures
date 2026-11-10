## 0.1.0

- Initial Dart lane of CT01: `ctEq`, `ctEqFixed`, `ctSelectBytes`, `ctEqU64` and the
  bit-returning `ctEqBit`, `ctEqFixedBit`, `ctEqU64Bit` over `Uint8List` and `BigInt`.
- Integer selection bit (0 or 1) validated with `choice * choice != choice`, which is
  correct on the VM and under JavaScript truncation; u64 reduced with 32-bit arithmetic only.
- `CtCompareError` carries CT_SELECT_LENGTH_MISMATCH, CT_INVALID_CHOICE or CT_INVALID_U64.
- Tests replay the complete CT01 corpus and add exhaustive position, byte-value,
  length-mismatch, 64-bit-flip and invalid-choice checks (100% line coverage).
