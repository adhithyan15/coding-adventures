# Changelog — nib-clr-compiler

## [0.1.0] — 2026-04-28

### Added

- Initial release: end-to-end Nib → CLR CIL compiler in Rust.

#### Pipeline

Five-stage compilation pipeline:

1. **Parse** — `nib_parser::parse_nib` lexes and parses the Nib source into a grammar AST.
2. **Type-check** — `nib_type_checker::check` runs type inference and constraint checking.
3. **IR compile** — `nib_ir_compiler::compile_nib` emits a target-independent `IrProgram`.
4. **Optimise** — `ir_optimizer::optimize_program` runs constant folding and dead-code elimination.
5. **CIL lower** — `ir_to_cil_bytecode::lower_ir_to_cil_bytecode` translates the `IrProgram`
   into a `CILProgramArtifact` (entry label + method bodies + local variable types).

#### Public API

- `NibClrCompiler` — configurable compiler struct with `compile_source`, `write_cil_file`.
- `compile_source(source)` — free-function shorthand (default settings).
- `pack_source(source)` — alias for `compile_source`, mirrors the Python `pack_source` API.
- `write_cil_file(source, path)` — compile and persist raw CIL bytes to disk.
- `PackageResult` — structured result: source, AST, typed AST, raw IR, optimised IR, CIL artifact, CIL bytes.
- `PackageError { stage, message }` — labelled error identifying which pipeline stage failed.

#### Design notes

- `PackageResult` implements `Debug` manually because `CILProgramArtifact` contains a
  `Box<dyn CILTokenProvider>` which does not implement `Debug`.
- `CILBackendConfig::syscall_arg_reg = 4` matches the Python CLR backend default.
- The CLR simulator (`clr-simulator`) supports a limited opcode subset (no `call`);
  Nib programs emit `call` instructions for some operations, so integration is tested
  via `validate_for_clr` rather than direct simulator execution.  The simulator test
  verifies the `clr-simulator` integration using hand-assembled arithmetic bytecode.

#### Error stages

| `stage`        | Cause                                           |
|----------------|-------------------------------------------------|
| `"parse"`      | Malformed Nib syntax                            |
| `"type-check"` | Type errors in the Nib source                   |
| `"ir-compile"` | IR emission error (should be rare)              |
| `"lower-cil"`  | CIL lowering error                              |
| `"write"`      | File I/O error writing the output CIL bytes     |

#### Tests

24 tests (19 unit + 5 doc-tests), all passing:

- Compilation produces non-empty CIL bytes.
- IR stages (raw and optimised) are both populated.
- Entry label is `"_start"`.
- Default assembly name is `"NibProgram"`.
- Default type name is `"NibProgram"`.
- CIL artifact has at least one method.
- CIL body ends with `0x2A` (`ret` opcode).
- `pack_source` alias produces identical output to `compile_source`.
- Custom assembly/type names propagate correctly.
- `optimize_ir = false` leaves raw and optimised IR identical.
- Multi-variable programs compile successfully.
- Source field is captured verbatim.
- `typed_ast` is populated.
- `write_cil_file` creates the output file with correct bytes.
- Parse errors → `PackageError { stage: "parse" }`.
- Type errors → `PackageError { stage: "type-check" }`.
- `PackageError::Display` includes stage and message.
- `validate_for_clr` returns no errors for compiled Nib IR.
- CLR simulator executes hand-assembled arithmetic bytecode (10 - 5 = 5).
