# Changelog — brainfuck-clr-compiler

## [0.1.0] — 2026-04-28

### Added

- Initial release: end-to-end Brainfuck → CLR CIL compiler in Rust.

#### Pipeline

Four-stage compilation pipeline:

1. **Parse** — `brainfuck::parse_brainfuck` lexes and parses the Brainfuck source.
2. **IR compile** — `brainfuck_ir_compiler::compile` emits a target-independent `IrProgram`.
3. **Optimise** — `ir_optimizer::optimize_program` runs constant folding and dead-code elimination.
4. **CIL lower** — `ir_to_cil_bytecode::lower_ir_to_cil_bytecode` translates the `IrProgram`
   into a `CILProgramArtifact` (entry label + method bodies + local variable types).

#### Public API

- `BrainfuckClrCompiler` — configurable compiler struct with `compile_source`, `write_cil_file`.
- `compile_source(source)` — free-function shorthand (default settings).
- `pack_source(source)` — alias for `compile_source`, mirrors the Python `pack_source` API.
- `write_cil_file(source, path)` — compile and persist raw CIL bytes to disk.
- `PackageResult` — structured result: source, AST, raw IR, optimised IR, CIL artifact, CIL bytes.
- `PackageError { stage, message }` — labelled error identifying which pipeline stage failed.

#### Design notes

- `CILBackendConfig::syscall_arg_reg = 4` matches the Python CLR backend default.
- `PackageResult` implements `Debug` manually because `CILProgramArtifact` contains a
  `Box<dyn CILTokenProvider>` which does not implement `Debug`.
- The CLR simulator (`clr-simulator`) supports a limited opcode subset (no `call`);
  full Brainfuck programs use tape memory operations that emit `call` instructions and
  therefore cannot be executed on the simulator directly.  Compilation is verified via
  `validate_for_clr` instead.

#### Tests

25 tests (20 unit + 5 doc-tests), all passing:

- Compilation produces non-empty CIL bytes.
- IR stages (raw and optimised) are both populated.
- Entry label is `"_start"`.
- Default assembly name is `"BrainfuckProgram"`.
- Default filename is `"program.bf"`.
- CIL artifact has at least one method.
- CIL body ends with `0x2A` (`ret` opcode).
- `pack_source` is an alias for `compile_source` (identical output).
- Custom assembly/type names propagate correctly.
- `optimize_ir = false` leaves raw and optimised IR identical.
- Loop programs (`[+]`) compile without error.
- Larger programs (65 increments) compile correctly.
- Source field is captured verbatim.
- `write_cil_file` creates the output file with correct bytes.
- Unmatched `[` → `PackageError { stage: "parse" }`.
- Unmatched `]` → `PackageError { stage: "parse" }`.
- `PackageError::Display` includes stage and message.
- `validate_for_clr` returns no errors for compiled Brainfuck IR.
- CLR simulator executes hand-assembled arithmetic bytecode (3 + 4 = 7).
- Empty program compiles to minimal CIL (at least a `ret`).
