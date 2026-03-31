# Changelog — coding-adventures-brainfuck (Lua)

## [0.1.0] — 2026-03-29

### Added
- `bf.validate(program)` — bracket balance checker.
- `bf.compile_to_opcodes(program)` — translates source to opcode list with pre-computed jump targets using stack-based bracket matching.
- `bf.run_opcodes(opcodes, input_str)` — eval loop executing all 8 Brainfuck commands with correct cell wrapping (0→255, 255→0) and EOF-as-zero convention.
- `bf.interpret(program, input_str)` — high-level one-call interface.
- Opcode constants: `OP_RIGHT`, `OP_LEFT`, `OP_INC`, `OP_DEC`, `OP_OUTPUT`, `OP_INPUT`, `OP_LOOP_START`, `OP_LOOP_END`, `OP_HALT`.
- Test suite: validation, compilation, all 8 commands, cell wrapping, loops, input/EOF, Hello World 'H' multiplication pattern.
