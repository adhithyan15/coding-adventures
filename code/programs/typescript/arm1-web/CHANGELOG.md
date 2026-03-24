# Changelog — arm1-web

## [0.1.0] — 2026-03-23

### Added

- Initial release: full browser-based ARM1 processor simulator.
- **6 visualization tabs:**
  - **Registers** — 16-register file with CPSR/R15 breakdown, N/Z/C/V flags,
    mode indicator (USR/FIQ/IRQ/SVC), I/F interrupt-disable bits
  - **Decode** — 32-bit instruction word broken into colour-coded bit fields;
    full condition code table showing all 16 ARM condition codes
  - **Pipeline** — 3-stage Fetch → Decode → Execute diagram with timing table
    explaining PC+8 behaviour and branch penalty
  - **Barrel Shifter** — Bit-level visualization of LSL/LSR/ASR/ROR/RRX with
    highlighted moving bits and 5-level MUX2 tree diagram
  - **Memory** — 4 KiB hex dump with PC/SP/read/write highlights and
    quick-navigation buttons
  - **Trace** — Full execution history with register deltas, flag changes,
    and memory accesses per instruction
- **4 pre-loaded demo programs:**
  - Fibonacci (fib(10) = 55) — demonstrates CMP, BEQ, loop structure
  - Sum 1..10 = 55 — demonstrates SUBS + BNE single-instruction decrement-and-test
  - Array Max — demonstrates post-index LDR and MOVGT conditional execution
  - Barrel Shifter Demo — steps through LSL #8, LSR #4, ASR #4, ROR #8
- **Sidebar assembly listing** — shows the source with the active line highlighted
- **Controls** — Step, Run ×10, Run to End, Reset, program selector
- **Keyboard-accessible tabs** — WAI-ARIA tablist pattern, Arrow/Home/End navigation
- **Responsive layout** — sidebar collapses below main panel on small screens
- Imports `@coding-adventures/arm1-simulator` for the behavioral CPU engine
- Tests verify all 4 programs produce correct register outputs
