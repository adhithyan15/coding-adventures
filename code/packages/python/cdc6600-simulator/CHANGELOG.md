# Changelog

## [0.1.0] — 2026-05-12

### Added

- Initial release: CDC 6600 (1964) behavioral simulator — Layer 07t
- `CDC6600State` frozen dataclass: `p` (parcel pointer), `x[8]` (60-bit), `a[8]` (18-bit), `b[8]` (18-bit), `memory[4096]` (60-bit words)
- `CDC6600Simulator` implementing the SIM00 `Simulator[CDC6600State]` protocol
- **Format 1 (15-bit) instructions**: TXB, TBX, TAX, TXA, IXPB, IXMB, IXXP, IXXM, BXND, BXOR, BXXR, BXMR, LSHL, LSHR, IBBP, IBBM, IAAP, IAAM, CMPEQ, CMPLT, CMPGT, IXMUL
- **Format 2 (30-bit) instructions**: LDXI, LDBI, LDAI, LDX, STX, LDB, STB, JEQ, JNE, JXZ, JXN, JMP, JSR, RET
- HALT on all-zeros 15-bit parcel
- B0 hardwired to zero (writes silently discarded)
- Instruction packing: 4 parcels per 60-bit word (big-endian, MSB = parcel 0)
- `short_instr(f, i, j, k)` and `long_instr(f, i, j, K)` encoding helpers
- `HALT` constant (`b"\x00\x00"`)
- Memory bounds checking with `ValueError` on out-of-bounds access
- `max_steps` guard to terminate infinite loops

### Design Decisions

- **Two's-complement integers** used instead of one's-complement (the real CDC 6600).
  For programs that avoid the ±0 edge case, behaviour is identical.
- **60-bit word memory**: 4096 words instead of the real 131,072. Sufficient for
  all test programs; easily extended by changing `MEMORY_WORDS`.
- **B7 as link register** for JSR/RET: the real CDC 6600 had no dedicated link register;
  this simulator uses B7 by convention (matching common CDC 6600 programming practice).
- **No floating-point**: X registers hold 60-bit integers only; FP instructions omitted.
- **No scoreboarding**: sequential execution only; functional units invisible to programs.
- **No peripheral processors (PPs)**: I/O simulation omitted.
