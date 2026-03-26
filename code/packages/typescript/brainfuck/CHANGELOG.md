# Changelog

## 0.1.0 -- 2026-03-20

### Added

- Initial release of the Brainfuck interpreter (TypeScript)
- 9 opcodes: RIGHT, LEFT, INC, DEC, OUTPUT, INPUT, LOOP_START, LOOP_END, HALT
- Translator: Brainfuck source -> CodeObject with bracket matching
- Opcode handlers registered with GenericVM via `registerOpcode()`
- Factory function `createBrainfuckVm()` with tape (30,000 cells), data pointer, input buffer
- Convenience function `executeBrainfuck(source, inputData)` for one-call execution
- `BrainfuckResult` interface with output, tape, dp, traces, steps
- Cell wrapping (255+1=0, 0-1=255)
- EOF-on-input returns 0
- Full test suite: translator tests, handler unit tests, end-to-end programs including Hello World
