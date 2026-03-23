# Changelog

## 0.1.0 — 2026-03-20

### Added
- `translate()` — Brainfuck source-to-bytecode translator with bracket matching
- `BrainfuckVM` — specialized VM with 30,000-cell tape, data pointer, and input buffer
- All 8 Brainfuck opcodes: RIGHT, LEFT, INC, DEC, OUTPUT, INPUT, LOOP_START, LOOP_END
- Wrapping arithmetic (255+1=0, 0-1=255) matching standard Brainfuck semantics
- Tape pointer wrapping at boundaries
- EOF convention: input reads return 0 when exhausted
- `execute_brainfuck()` — one-shot high-level API returning `BrainfuckResult`
- `BrainfuckResult` — output, tape state, traces, and step count
- Comprehensive error handling for unmatched brackets and invalid opcodes
- Execution tracing with human-readable descriptions for every step
