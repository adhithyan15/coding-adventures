# Changelog — twig

All notable changes to this package will be documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [0.4.0] — 2026-05-04 (TW04 Phase 4c — host module + cross-module IR ops)

### Added

- **Platform-independent syscall convention** — host-module calls
  (`host/write-byte`, `host/read-byte`, `host/exit`) are now lowered
  uniformly to `call_builtin "syscall" <num> arg` in the interpreter
  IR and to `IrOp.SYSCALL IrImmediate(num) IrRegister(arg)` in the
  compiler IR.  Syscall numbers match the CLR and JVM backend
  conventions established by the Brainfuck compiler:

  | Export          | Syscall num |
  |-----------------|-------------|
  | `host/write-byte` | 1         |
  | `host/read-byte`  | 2         |
  | `host/exit`       | 10        |

- **`_is_module_qualified(name)`** helper in `twig.compiler` —
  detects `module/export` patterns while correctly excluding the
  bare `/` division operator.

- **`"syscall"` builtin in `TwigVM`** — single dispatcher replacing
  the old per-name `host/write-byte` / `host/read-byte` / `host/exit`
  builtins.  Dispatches by syscall number and implements all three
  host operations.

- **`free_vars.py` guard updated** — module-qualified names (slash
  with non-empty prefix and suffix) are never treated as free
  variables, and the check now correctly excludes the bare `/`
  arithmetic operator.

- **Module-resolver refactored to use `directed-graph`** — the
  hand-rolled DFS topological sort is replaced by
  `topological_sort` from the `coding-adventures-directed-graph`
  package.  Cycle detection uses `strongly_connected_components`
  for accurate path reconstruction; `_cycle_path` reconstructs the
  exact `a -> b -> c -> a` string.
  `coding-adventures-directed-graph` is now a formal dependency in
  `pyproject.toml`.

### Changed

- `twig/compiler.py` — `_compile_apply` now emits
  `IIRInstr("call_builtin", dest, ["syscall", num, *arg_regs])`
  for module-qualified calls, replacing the former
  `IIRInstr("call", dest, [qualified_name, ...])` approach.
- `twig/vm.py` — removed `_inject_host_stubs` (the function that
  patched synthetic IIR stubs for `call "host/…"` targets) and the
  individual `host/write-byte` / `host/read-byte` / `host/exit`
  builtins; replaced by the single `"syscall"` builtin.
- `pyproject.toml` — added `coding-adventures-directed-graph` as an
  explicit runtime dependency.

### Tests

- `tests/test_host_calls.py` (new, 26 tests):
  - `TestIsModuleQualified` — 9 cases covering the slash helper.
  - `TestHostSyscallNumbers` — asserts the exact syscall mapping.
  - `TestCompilerSyscallEmission` — IR-level checks that the
    interpreter compiler emits `call_builtin "syscall" <num> …`.
  - `TestVMSyscall` — execution-level checks:
    - `write-byte` writes the correct byte to stdout.
    - `write-byte` masks values > 255.
    - `read-byte` returns the byte from stdin.
    - `read-byte` returns −1 on EOF.
    - Unknown syscall number raises `TwigRuntimeError`.
  - `TestFreeVarsHostCalls` — ensures host names are not captured
    as free variables and the `/` division operator is unaffected.

### Test metrics

  169 tests, 95.63% coverage (≥ 95% required).

## [0.3.0] — 2026-04-29 (TW04 Phase 4b — module resolver)

### Added

- **`twig.module_resolver`** — new module that walks the import
  graph from a named entry module and returns every reachable
  module's parsed AST in topological order.

  ```python
  from pathlib import Path
  from twig import resolve_modules

  modules = resolve_modules(
      "user/hello",
      search_paths=[Path("src/")],
  )
  for m in modules:
      # ``m.name``, ``m.program``, ``m.source_path``
      compile_one(m)  # backends iterate; deps come first
  ```

- **`ResolvedModule`** dataclass — `name`, `program`,
  `source_path` (None for the synthetic `host` module).

- **Synthetic `host` module** — ``(import host)`` works without
  a `host.tw` file; the resolver materialises a `Module` with
  the v1 export surface (`write-byte`, `read-byte`, `exit`) on
  demand.  Per-backend lowering (Phase 4d–4f) will intercept
  these names and emit the runtime-specific implementations.

- **Cycle detection** with full path messages —
  ``cycle: a -> b -> c -> a`` — using a three-colour DFS so
  the user sees exactly which edge to break.

- **Path / name validation** — a file reached via the resolver
  MUST declare `(module name)` matching the import path.
  Single-file programs without a module declaration still work
  through the existing direct compile-source API; the resolver
  only governs explicit-module imports.

### Tests

- 17 new tests in `tests/test_module_resolver.py`:
  happy-path single / chain / diamond / nested-path / multiple
  search paths / re-imports; cycle (2-, 3-, self-); missing
  entry / transitive / no-search-paths; path-name mismatches.
- 145/145 twig tests pass; 100% coverage on
  `module_resolver.py`; package-wide 95.58%.

## [0.2.0] — 2026-04-29 (TW04 Phase 4a — module form in parser/AST)

### Added

- **`(module name (export ...) (import ...))` form** at the top
  of every Twig file.  Parser-only landing — no module resolution,
  no cross-module IR, no per-backend lowering yet.
- **`module`, `export`, `import` keywords** added to
  `code/grammars/twig.tokens`.  These names are reserved as a
  side effect.
- **Grammar updates** (`code/grammars/twig.grammar`):
  - `program = [ module_form ] { form } ;`
  - `module_form = LPAREN "module" NAME { module_clause } RPAREN ;`
  - `module_clause = export_clause | import_clause ;`
  - `export_clause = LPAREN "export" { NAME } RPAREN ;`
  - `import_clause = LPAREN "import" { NAME } RPAREN ;`
  - The flat layout (module declaration as a self-contained
    sibling of the file's defines) matches the spec example
    in `code/specs/TW04-modules-and-host-package.md`.
- **`Module` AST node** with `name`, `exports`, `imports`,
  source position; **`Program.module: Module | None`** field
  (defaults to `None` — implicit "default module" for
  backward compatibility).
- **Module-path NAME tokens** like `stdlib/io` and
  `user/compiler/lexer` lex as a single NAME because the
  Twig NAME regex already permits `/` inside identifiers.
- **Duplicate-export and duplicate-import detection** at
  AST-extraction time with positioned error messages.

### Changed

- `extract_program` now returns a `Program` with a populated
  `module` when the file starts with `(module ...)`.  Existing
  code that ignores `module` (every backend prior to TW04 Phase
  4d/e/f) keeps working unchanged — it just reads `forms` as
  before.

### Notes

- This phase intentionally stops at parser/AST.  TW04 Phase 4b
  adds the resolver; 4c adds cross-module IR ops; 4d–4f wire
  per-backend lowering; 4g writes the stdlib in Twig.

## [0.1.0] — 2026-04-28 (TW00 initial release)

### Added

- **Lexer + parser** built on the repo's grammar-driven pipeline
  (``grammar-tools`` + ``lexer`` + ``parser``).  Source-of-truth
  grammar files live in ``code/grammars/twig.tokens`` and
  ``code/grammars/twig.grammar``.  The package's ``lexer.py`` and
  ``parser.py`` are thin shims that load those files — same shape
  as Brainfuck, Dartmouth BASIC, ALGOL, etc.
- **Typed AST** — ``ast_extract.extract_program`` lifts the generic
  ``ASTNode`` tree into a small set of typed dataclasses
  (``Define`` / ``If`` / ``Let`` / ``Lambda`` / ``Apply`` / …) so
  the compiler walks an exhaustive set of cases rather than a
  ``rule_name`` switch.
- **Heap** — refcounted host-side heap supporting cons cells,
  interned symbols, and closures.  Symbols are not refcounted; cons
  cells and closures own any nested handles via ``incref`` /
  ``decref``.  Cycles leak in v1 (no ``letrec`` to construct them in
  the surface, but possible via top-level defines pointing at each
  other) — TW01's mark-sweep handles them.
- **Free-variable analysis** — ``free_vars.free_vars(lam, globals_)``
  returns the ordered list of names a lambda captures, used by the
  compiler to emit ``call_builtin "make_closure"`` with the right
  capture list.
- **Compiler** — typed AST → ``IIRModule``.  Top-level functions
  become direct-call IIR functions; anonymous lambdas become
  gensym'd top-level IIR functions whose leading parameters hold
  captured values; apply-site dispatch chooses direct ``call`` vs.
  indirect ``call_builtin "apply_closure"`` at compile time.
- **TwigVM** — wraps ``vm-core``, registers builtins, owns a per-run
  heap and globals table, overrides ``jmp_if_true`` /
  ``jmp_if_false`` so ``if`` uses Scheme truthiness (``0`` is
  truthy; only ``#f`` and ``nil`` are falsy).
- **Builtins**: ``+``, ``-``, ``*``, ``/``, ``=``, ``<``, ``>``,
  ``cons``, ``car``, ``cdr``, ``null?``, ``pair?``, ``number?``,
  ``symbol?``, ``print``.
- TW00 spec: ``code/specs/TW00-twig-language.md``.

### Notes

- TW01 will replace refcounting with mark-sweep, add ``letrec``,
  and promote the heap operations from ``call_builtin`` into
  native IIR ops via a new ``gc-core`` package.
- TW02 will compile Twig to BEAM bytecode directly (no Erlang
  source intermediary) — analogous to how ``wasm-backend`` produces
  WASM bytes natively.
