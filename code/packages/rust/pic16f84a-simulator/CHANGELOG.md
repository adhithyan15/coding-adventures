# Changelog

## 0.1.0

- Add the source-pinned PIC16F84A classic mid-range functional CPU model.
- Implement all 35 documented instructions from their real 14-bit encodings,
  plus a reserved-word deterministic halt sentinel.
- Model complete Harvard transport, banked and indirect file access, PCL and
  STATUS special writes, circular return stack, and typed atomic lifecycle.
- Add exhaustive independent decoder, arithmetic, addressing, control-flow,
  stack, trace, and rollback tests.
- Validate 14 release unit tests and one doctest at 98.67% package-source line
  coverage (670/679 lines) with strict Clippy and rustdoc.
