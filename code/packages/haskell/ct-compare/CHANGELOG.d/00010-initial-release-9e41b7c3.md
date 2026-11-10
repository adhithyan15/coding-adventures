## 0.1.0

- Initial Haskell lane of CT01: `ctEq`, `ctEqFixed`, `ctSelectBytes`, `ctEqU64` and the
  bit-returning `ctEqBit`, `ctEqFixedBit`, `ctEqU64Bit`, over strict `ByteString` and `Word64`.
- `ctSelectBytes` returns `Either CtError ByteString` (`SelectLengthMismatch`, `InvalidChoice`);
  the selection bit is a `Word8` that must be 0 or 1.
- Tests replay the complete CT01 corpus and add exhaustive position, byte-value,
  length-mismatch, 64-bit-flip and invalid-choice checks (hpc: 100% of expressions).
