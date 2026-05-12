# Changelog

## [0.1.0] — 2026-05-12

### Added
- Initial implementation of the PowerPC 601 (1992) behavioral simulator (Layer 07u)
- `PowerPC601State` — frozen dataclass with `cia`, `gpr` (32 × 32-bit), `lr`, `ctr`, `xer`, `cr`, `memory` (64 KiB), `halted`
- `PowerPC601Simulator` implementing `Simulator[PowerPC601State]` (SIM00 protocol):
  - `reset()`, `load()`, `step()`, `execute()`, `get_state()`
- Instruction set (integer subset):
  - **Arithmetic**: `add`, `addc`, `adde`, `subf`, `subfic`, `neg`, `mullw`, `divw`, `divwu`
  - **Immediate arithmetic**: `addi`, `addis`
  - **Logical**: `and`, `or`, `xor`, `nand`, `nor`, `cntlzw`
  - **Logical immediate**: `andi.`, `andis.`, `ori`, `oris`, `xori`
  - **Shift**: `slw`, `srw`, `sraw`, `srawi`
  - **Compare**: `cmpw`, `cmplw`, `cmpwi`, `cmplwi`
  - **Load**: `lwz`, `lwzu`, `lbz`, `lbzu`, `lhz`, `lhzu`, `lha`
  - **Store**: `stw`, `stwu`, `stb`, `stbu`, `sth`
  - **Branch**: `b`, `bl`, `bc` (all BO combinations), `blr`, `bctr`, `bctrl`
  - **Special registers**: `mfspr`/`mtspr` (LR=8, CTR=9, XER=1), `mfcr`, `mtcrf`
  - **HALT**: `0x00000000`
- Encoding helpers exported: `i_form`, `b_form`, `d_form`, `x_form`, `xo_form`, `xfx_form`, `xl_form`
- Constants exported: `HALT`, `BO_ALWAYS`, `BO_TRUE`, `BO_FALSE`, `BO_BDNZ`, `BO_BDZ`, `BI_LT`, `BI_GT`, `BI_EQ`, `BI_SO`, `SPR_LR`, `SPR_CTR`, `SPR_XER`, `MASK32`, `MEM_SIZE`, `XER_CA`, `XER_OV`, `XER_SO`
- Test suite: 109 tests across 4 test files (protocol, instructions, coverage, programs)
- End-to-end program tests: sum 1–10, factorial 5!, Fibonacci F(9), subroutine call/return, array sum, memory copy, XOR swap, max of two values, countdown loop, 64-bit add
- Spec: `code/specs/07u-powerpc601-simulator.md`
