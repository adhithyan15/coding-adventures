# Changelog — tetrad-compiler

## [0.1.0] — 2026-04-20

### Added

- `tetrad_compiler.bytecode` module: `Instruction`, `CodeObject`, `Op` opcode namespace
- Full ISA opcode constants: loads (LDA_IMM, LDA_ZERO, LDA_REG, LDA_VAR), stores (STA_REG, STA_VAR), arithmetic (ADD/SUB/MUL/DIV/MOD with ADD_IMM/SUB_IMM fast paths), bitwise (AND/OR/XOR/NOT/SHL/SHR/AND_IMM), comparisons (EQ/NEQ/LT/LTE/GT/GTE), logical helpers (LOGICAL_NOT/AND/OR), control flow (JMP/JZ/JNZ/JMP_LOOP), calls (CALL/RET), I/O (IO_IN/IO_OUT), HALT
- Two-path compilation: typed binary ops emit `[r]` (no slot); untyped emit `[r, slot]`
- ADD_IMM/SUB_IMM optimisation for `n + literal` and `n - literal` patterns
- Short-circuit `&&` and `||` via JZ/JNZ jump sequences
- Unary: `~` → NOT, `!` → LOGICAL_NOT, `-` → LDA_ZERO + SUB (wrapping negation)
- Function compilation with parameter preamble (copy R0..argc-1 to var_names)
- CALL instruction with `[func_idx, argc, slot]` operands
- Forward-declaration pass so all functions are visible to all callers
- Jump patching for if/else and while loops using signed offset operands
- `immediate_jit_eligible` flag set on FULLY_TYPED functions
- Source map: list of (instruction_index, line, column) triples
- `CompilerError` with message, line, column attributes
- `compile_checked(TypeCheckResult) -> CodeObject` and `compile_program(source) -> CodeObject` entry points
- 77 unit tests, 100% line coverage
