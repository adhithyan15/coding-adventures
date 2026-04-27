# Changelog — @coding-adventures/brainfuck-ir-compiler

## [0.1.0] — 2026-04-11

### Added

- `BuildConfig` interface with `insertBoundsChecks`, `insertDebugLocs`, `maskByteArithmetic`, `tapeSize` fields
- `debugConfig()` factory: all checks enabled, 30,000-cell tape
- `releaseConfig()` factory: bounds checks off, debug locs off, masking on, 30,000-cell tape
- `CompileResult` interface with `program: IrProgram` and `sourceMap: SourceMapChain`
- `compile(ast, filename, config)` — main entry point
  - Validates AST root is "program" node
  - Validates tapeSize > 0
  - Emits prologue (_start label, LOAD_ADDR v0, LOAD_IMM v1)
  - Emits debug prologue registers (v5, v6) when bounds checks enabled
  - Compiles all 6 BF commands to IR
  - Compiles loops with unique numbered labels (loop_N_start / loop_N_end)
  - Emits epilogue (HALT, optional __trap_oob handler)
  - Populates SourceToAst and AstToIr segments
- Comprehensive test suite with >95% coverage

### Implementation notes

- Uses `isASTNode` from `@coding-adventures/parser` for AST traversal
- Token extraction does depth-first search through AST children
- Loop source mapping uses `node.startLine` / `node.startColumn` from ASTNode
- Output command now uses ADD_IMM 0 to copy into the syscall arg register without depending on the debug-only zero register
