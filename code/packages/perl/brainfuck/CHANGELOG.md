# Changelog — CodingAdventures::Brainfuck (Perl)

## [0.01] — 2026-03-29

### Added
- `validate($program)` — bracket balance checker.
- `compile_to_opcodes($program)` — two-pass compiler: opcodes + stack-based jump target resolution.
- `run_opcodes($opcodes, $input)` — eval loop with correct cell wrapping (0→255, 255→0), EOF-as-zero, and bounds checking.
- `interpret($program, $input)` — high-level one-call interface.
- Opcode constants: `OP_RIGHT`, `OP_LEFT`, `OP_INC`, `OP_DEC`, `OP_OUTPUT`, `OP_INPUT`, `OP_LOOP_START`, `OP_LOOP_END`, `OP_HALT`.
- Test suite: validation, compilation, all 8 commands, cell wrapping, loops (skip/execute/copy), input/EOF, Hello World multiplication pattern.
