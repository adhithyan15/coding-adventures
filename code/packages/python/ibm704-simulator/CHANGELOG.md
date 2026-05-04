# Changelog

All notable changes to `coding-adventures-ibm704-simulator` are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] — 2026-04-28

### Added

- Initial release of the IBM 704 simulator (the first mass-produced computer
  with hardware floating-point, host of FORTRAN I and LISP 1).
- 36-bit sign-magnitude word model with distinct +0 and −0 representations.
- 38-bit accumulator (sign + Q + P + 35-bit magnitude) with overflow detection
  via the Q and P bits.
- 36-bit MQ register, 3 × 15-bit index registers (IRA, IRB, IRC), 32K word
  core memory.
- Type B and Type A instruction decoders (Type A = TIX/TXI/TXH/TXL with
  decrement field).
- Effective-address computation including OR-of-multiple-index-registers
  semantics for tag values 3, 5, 6, and 7.
- Core instruction set (v1 scope): HTR, HPR, NOP, CLA, CAL, ADD, SUB, ADM,
  STO, STZ, STQ, LDQ, XCA, MPY, DVP, DVH, TRA, TZE, TNZ, TPL, TMI, TOV, TNO,
  LXA, LXD, SXA, SXD, PAX, PDX, PXA, TIX, TXI, TXH, TXL.
- Floating-point instructions: FAD, FSB, FMP, FDP (sign + 8-bit excess-128
  exponent + 27-bit fraction).
- `IBM704State` frozen dataclass conforming to the SIM00 simulator protocol.
- `IBM704Simulator` exposing `load`, `step`, `execute`, `get_state`, `reset`.
- Packed-big-endian word transport for the byte-oriented `Simulator` protocol
  (5 bytes per 36-bit word, high 4 bits zero).
- Unit tests covering word-format round-trips, sign-magnitude addition, every
  v1 instruction in isolation, end-to-end programs (sum 1..N, factorial,
  cons-cell CAR/CDR extraction), and protocol conformance.
