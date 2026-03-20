# Changelog

## 0.1.0 — 2026-03-20

### Added
- Initial release
- Brainfuck interpreter built on the pluggable GenericVM
- Translator: source -> bytecode with bracket matching
- 9 opcode handlers using GenericVM's extra state
- Factory function and convenience executor
- BrainfuckResult struct with output, tape, dp, traces, steps
- Full Hello World support
- Input handling with EOF producing 0
- Cell wrapping (255+1=0, 0-1=255)
- Pointer bounds checking with clear error messages
- 78 tests at 96.84% coverage
