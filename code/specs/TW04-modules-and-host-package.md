# TW04 — Modules and the `host` package

## Why this spec exists

Twig today is monolithic — every program is a single file, every name
lives in one global namespace, and the only way to call into the
runtime (e.g. for stdout I/O) is the compiler-internal `SYSCALL` IR
op.  This worked for proving out the language surface across
JVM / CLR / BEAM (TW02 → TW03) but blocks two strategic next steps:

1. **Self-hosting.**  A self-hosted Twig compiler is dozens of files
   (lexer, parser, AST, IR-emitter, type-checker, optimizer,
   per-backend lowering, driver).  Without modules you'd be
   hand-prefixing every name (`my-compiler-parse`,
   `my-compiler-emit-ir`, …) and `(load "file.tw")`-ing in dependency
   order.  No real Lisp ships without modules for this reason.

2. **Portable I/O.**  `(print x)` needs to work on every backend
   without per-backend conditionals in user code.  The only way to
   make that real is a stable host-call API that the user code
   imports — `(import host)`, `(host/write-byte b)` — with each
   backend lowering the leaf calls to its native facility.

This spec covers both.  The design is deliberately influenced by
**WASM/WASI** — the third major real-runtime target on the roadmap —
because WASM modules + WASI capability-based imports are the
architecturally cleanest expression of the layering we want and the
spec needs to compose with that future without retrofits.

## Acceptance criterion

After TW04 lands, this works on JVM, CLR, BEAM (and lowers cleanly
to WASM when that backend is added):

```
;; ── stdlib/io.tw ─────────────────────────────────────────────
(module io
  (export print-int println)
  (import host))

(define (print-digits n)
  (if (< n 10)
      (host/write-byte (+ n 48))
      (begin
        (print-digits (/ n 10))
        (host/write-byte (+ (- n (* 10 (/ n 10))) 48)))))

(define (print-int n)
  (if (< n 0)
      (begin (host/write-byte 45) (print-digits (- 0 n)))
      (print-digits n)))

(define (println n)
  (print-int n)
  (host/write-byte 10))

;; ── user/hello.tw ────────────────────────────────────────────
(module user/hello
  (import io))

(println 42)
(println (+ 17 25))
```

Output: `42\n42\n` on stdout, on every backend.

## Goals

- **Module-as-namespace.**  Names defined in module `A` don't
  collide with names in module `B`, regardless of file order or
  import order.
- **Explicit imports / exports.**  Modules opt into what they
  expose; importers opt into what they pull in.  Closer to
  Python/Rust/Scheme R7RS than to Common Lisp packages.
- **One file = one module.**  Filesystem layout maps directly to
  module names: `stdlib/io.tw` → module `stdlib/io`.
- **Separate compilation.**  Each module compiles to one host
  artifact (one JVM `.class`, one CLR `TypeDef`, one BEAM `.beam`,
  one WASM module) without needing other modules' source.
- **Stable `host` package.**  The compiler ships a built-in
  `host` module whose exports are the same names on every backend.
  User code calls `(host/write-byte b)`; backends provide the
  implementation.
- **Stdlib written in Twig.**  `io`, `list`, `string`, `print`
  are real Twig source code, not per-backend special cases.

## Non-goals (TW04 v1)

- **Macros.**  Twig stays macro-free per [TW03 Phase 5](TW03-lisp-primitives-and-gc.md).
  Eliminates the need for Racket-style phase separation.
- **Nested modules.**  One module per file; flat namespace per
  module.  Racket / R6RS allow nesting; we don't need it yet.
- **Visibility levels.**  No `private` / `protected` / `internal`
  — everything is either exported or not.
- **Versioning / package manager.**  Module resolution is
  purely path-based against a `--module-search-path` argument.
  A package manager is its own spec.
- **Re-exports / aliasing.**  `(import io as i)` and
  `(re-export print-int)` would be nice; defer to v2.

## Module syntax

```scheme
(module name
  (export name1 name2 ...)
  (import other-module ...))

;; ...top-level forms come after the module declaration...
(define (foo) ...)
(define (bar) ...)
```

- The `module` form must be the first form in the file.  Forms
  before it are a `TwigCompileError`.
- `name` is a slash-separated path matching the file's location
  relative to a search-path root.  `stdlib/io.tw` → module
  `stdlib/io`.  Mismatch is a `TwigCompileError`.
- The module declaration is **flat** — it carries the module's
  name, exports, and imports, but its top-level forms (defines
  and expressions) are siblings AFTER the module form, not
  nested inside it.  Phase 4a chose this layout because it
  retrofits cleanly onto every existing single-file Twig
  program (just prepend a `(module ...)` declaration; no
  re-indentation) and it matches the existing program shape
  the parser already accepts.
- `(export name1 ...)` declares the module's public surface.
  Names not in the export list are file-private.
- `(import other-module)` brings every exported name from
  `other-module` into the current module's namespace, prefixed
  by the imported module's path: `(import host)` → use as
  `host/write-byte`, not bare `write-byte`.
- Multiple `(import ...)` forms are allowed; order doesn't
  matter.  Multiple `(export ...)` clauses are similarly
  concatenated.  Duplicate names within or across clauses
  raise a `TwigParseError` at extraction time.

The path prefix is mandatory — it eliminates ambiguity.  Want
shorter names?  Use `let`-binding:

```scheme
(module my/program
  (import io))

(let ((p io/println))
  (p 42))
```

## File layout & resolution

```
src/
├── stdlib/
│   ├── io.tw           ; module stdlib/io
│   ├── list.tw         ; module stdlib/list
│   └── print.tw        ; module stdlib/print
└── user/
    ├── hello.tw        ; module user/hello
    └── compiler/
        ├── lexer.tw    ; module user/compiler/lexer
        └── parser.tw   ; module user/compiler/parser
```

The driver (e.g. `twig-jvm-compiler`) takes:

- An **entry module** — usually the one with top-level expressions
  to execute.
- A **module search path** — list of directories to look in.

It computes the import closure transitively and compiles each
discovered module.  Cycles between modules are a `TwigCompileError`
in v1 (no mutual recursion across modules).

The built-in `host` module is provided by the compiler itself
and doesn't live on disk.

## The `host` package

`host` is the cross-backend host-call surface.  Its v1 export list
is intentionally small — three primitives covering "byte to
stdout, byte from stdin, exit" — and grows only when concrete
need arises (file I/O, time, etc.).

```scheme
(module host
  (export write-byte read-byte exit))

;; (host/write-byte b: int) -> int   — returns 0 (success)
;;   Writes one byte to stdout.  ``b`` is masked to its low 8 bits.
;;
;; (host/read-byte) -> int           — returns the byte read, or
;;                                     -1 on EOF.
;;
;; (host/exit code: int) -> never
;;   Exits the process with the given code.
```

Per-backend lowering (today and future):

| Primitive | JVM | CLR | BEAM | WASM (future, WASI) |
|---|---|---|---|---|
| `write-byte b` | `System.out.write(b)` | `Console.OpenStandardOutput().WriteByte((byte)b)` | `io:put_chars([b])` | `wasi_snapshot_preview1::fd_write(1, ...)` |
| `read-byte` | `System.in.read()` | `Console.In.Read()` | `io:get_chars/2` | `wasi_snapshot_preview1::fd_read(0, ...)` |
| `exit code` | `System.exit(code)` | `Environment.Exit(code)` | `erlang:halt(code)` | `wasi_snapshot_preview1::proc_exit(code)` |

All three primitives carry small fixed signatures so they map
cleanly to WASM's strictly-typed function imports.  If a v2 surface
adds `(host/open-file path mode) -> int` we'd want to settle the
WASM/WASI shape first (capability-based file descriptors with
specific return-error encoding) so the cross-backend signature
matches what WASI gives us.

## Stdlib design

The standard library is real Twig source, shipped with the
compiler in `code/stdlib-twig/` (or similar).  Initial contents:

- `stdlib/io` — `print-int`, `print-bool`, `println`, `newline`
- `stdlib/list` — `length`, `reverse`, `map`, `filter`, `fold`
- `stdlib/print` — `print` (dispatches on type via `null?`,
  `pair?`, `symbol?`, `number?`)

These get compiled alongside user code.  No backend-specific
implementations.

The driver implicitly includes `code/stdlib-twig/` in the module
search path so `(import io)` Just Works out of the box.

## Per-backend lowering

Each module compiles to ONE host artifact.  The mapping:

### JVM

- One module → one Java class.  Class name = module name with
  `/` → `.` (e.g. `stdlib/io` → `stdlib.io`).  Package name and
  class layout match Java conventions.
- Exported functions become PUBLIC STATIC methods on the class.
- File-private functions become PRIVATE STATIC methods.
- Cross-module calls lower to `invokestatic <ImportedModule>.<name>(...)`.
- The existing `__ca_regs` / `__ca_objregs` static arrays move to
  per-module (each module gets its own pair) — alternatively, a
  shared "runtime" class hosts them and every module references it.

### CLR

- One module → one CLR `TypeDef`.  Type name = module name with
  `/` → `_` (e.g. `stdlib/io` → `stdlib_io`).  Namespace
  `CodingAdventures.Twig`.
- Exports = public static methods on the type.
- Cross-module calls = `call <type>::<method>` with MethodDef tokens.
- Multi-module assemblies are natively supported by PE/CLI metadata
  (closure types already prove this works — see CLR02 Phase 2c).

### BEAM

- One module → one `.beam` file.  This is the natural BEAM model
  (Erlang already says "module = file = atom").
- Exports = the BEAM ExpT (export) table.
- Cross-module calls = `call_ext` with the importer's ImpT row.
- Implementation almost free since BEAM is module-shaped already.

### WASM (future)

- One module → one `.wasm` module.  Imports declared in the
  WASM binary's import section.
- Exports declared in the export section.
- Cross-module calls = WASM `call` indirected through imports.
- The `host` module isn't a `.wasm` at all — it's the WASI
  capability surface the runtime provides, declared as imports
  on every module that needs them.

This is the moment WASM compatibility pays off.  WASM's import /
export model is THE most strict of the four backends.  If our
spec compiles cleanly to WASM imports/exports, it'll compile
cleanly to anything.

## Implementation phases

1. **Phase 4a — module spec + parser.**  Add `(module ...)`
   form to `twig.parser` and `twig.ast_extract`.  No code
   generation yet.  Programs without `(module ...)` get an
   implicit "default module" so existing single-file programs
   keep working.  **Status: shipped — `twig` v0.2.0.**  The
   `Module` AST node attaches to `Program.module` (defaults to
   `None`) and downstream backends ignore it for now.

2. **Phase 4b — module resolution.**  New `twig.module_resolver`
   that takes (entry module name, search paths) and returns a
   topologically-sorted list of `(module name, AST)` pairs.
   Detects cycles, missing imports, name conflicts.
   **Status: shipped — `twig` v0.3.0.**  Returns
   `list[ResolvedModule]` (each carrying `name`, `program`,
   `source_path`).  The synthetic `host` module is auto-resolved
   without consulting the search path.  Cycle errors include
   the full path (`a -> b -> c -> a`) for diagnosability.

3. **Phase 4c — `host` module + cross-module IR ops.**
   **Status: shipped — `twig` v0.4.0.**

   **Platform-independent syscall convention** rather than
   module-qualified CALL labels.  Every call to a `host/*`
   export in user code lowers to a `SYSCALL` IR op with a
   small fixed numeric code shared across all backends:

   | Num | Host export | Interpreter IIR | Compiler IR (JVM/CLR) |
   |-----|-------------|-----------------|------------------------|
   | 1 | `host/write-byte` | `call_builtin "syscall" 1 arg` | `IrOp.SYSCALL IrImmediate(1) IrRegister(0)` |
   | 2 | `host/read-byte` | `call_builtin "syscall" 2` | `IrOp.SYSCALL IrImmediate(2) IrRegister(dest)` |
   | 10 | `host/exit` | `call_builtin "syscall" 10 arg` | `IrOp.SYSCALL IrImmediate(10) IrRegister(0)` |

   The simpler "extend CALL's label" approach was considered but
   rejected — `IrOp.SYSCALL` was already wired end-to-end in the
   JVM and CLR backends (and Brainfuck uses it with the same
   numbers) so emitting SYSCALL directly avoids introducing a
   new naming convention that each backend would need to decode.

   **`twig.compiler`** — `_compile_apply` detects module-qualified
   names (slash with non-empty prefix and suffix) and looks up the
   syscall number in `_HOST_SYSCALLS`.  Unknown `host/*` names
   raise `TwigCompileError`.

   **`TwigVM`** — single `"syscall"` builtin dispatcher replaces
   the three individual `host/write-byte` / `host/read-byte` /
   `host/exit` builtins.

   **`twig.free_vars`** — module-qualified names are never
   captured as closure free-variables.

   **Module resolver** — refactored from a hand-rolled DFS to
   `topological_sort` + `strongly_connected_components` from the
   `coding-adventures-directed-graph` package.  Public API
   unchanged.

4. **Phase 4d — JVM module lowering.**  Each module → one
   `.class`.  Cross-module CALL → `invokestatic`.  Bundle the
   stdlib classes alongside the user JAR.

5. **Phase 4e — CLR module lowering.**  Each module → one
   `TypeDef` in a multi-type assembly.

6. **Phase 4f — BEAM module lowering.**  Each Twig module →
   one `.beam` file.  Almost free given existing BEAM plumbing.

7. **Phase 4g — Stdlib in Twig.**  Write `stdlib/io.tw`,
   `stdlib/list.tw`, `stdlib/print.tw` etc. as real Twig
   source.  Driver ships them alongside the compiler.

After 4g you can write `(module hello (import io))
(io/println 42)` and it compiles + runs on three real
runtimes from a single source tree.

WASM (Phase 4h or its own spec) lands when the WASM backend
itself does — but the design above ensures no retrofit work.

## WASM compatibility notes

Because the user explicitly wants WASM portability:

- **Strictly typed function signatures.**  WASM imports/exports
  are typed `(func (param i32 i32) (result i32))`-style.  Our
  module exports already have known parameter types per
  region (TW03 follow-up classifier work).  As long as we don't
  introduce variadic functions or untyped params, lowering to
  WASM is mechanical.

- **No exceptions in MVP.**  WASM MVP doesn't have exceptions;
  the exception-handling proposal is post-MVP.  Our `TwigCompileError`
  is a *compile-time* concern (Python exception in the compiler),
  not a runtime one — Twig programs don't have try/catch today
  and shouldn't until WASM stabilizes its proposal.

- **Linear memory + GC.**  WASM MVP only has linear memory (an
  untyped byte array).  Heap primitives (cons / symbol / nil)
  would need either:
  - **Custom GC in linear memory** (Cheney's two-space copying
    collector — ~600 LoC of WASM emission).  Most portable.
  - **WASM GC proposal** (`struct.new`, `ref` types, `i31ref`).
    Cleaner, supported in modern V8 / SpiderMonkey, pinned to
    a specific WASM dialect.

  This is TW04's only real risk: we should validate that
  cons / symbol / nil can fit in EITHER scheme.  The current
  cross-backend value model (object refs on JVM/CLR/BEAM, atoms
  on BEAM) maps to:
  - WASM-MVP custom GC: tagged 64-bit pointers in linear memory
    (matching the original TW03 design that the JVM/CLR
    backends ended up bypassing in favor of native object refs).
  - WASM GC proposal: `(ref $cons)`, `(ref $symbol)`,
    `i31ref` for tagged ints.

  Either works.  Decision punted to whichever WASM backend
  spec lands first.

- **No heap-allocated strings in v1.**  Match WASM's "everything
  is bytes in linear memory" model.  Strings stay byte-list
  cons cells until we have a String runtime type.

- **Capability-based I/O.**  WASI is capability-based —
  `host/open-file` returns a file descriptor that the runtime
  granted access to.  Our v1 surface is just stdin/stdout/exit,
  which WASI grants by default.  When file I/O lands, the WASI
  shape (open / fd_read / fd_write / fd_close with explicit
  capability checks) drives our `host` API design.

## Risk register

- **Cross-module name collisions.**  Two modules each export
  `length`.  An importer importing both gets `list/length` and
  `string/length` via the import-prefix convention — no
  collision.  But intra-module shadowing (a local `length`
  shadowing an imported `list/length`) is a footgun.  Document
  + warn.

- **Module-resolution as part of the build.**  Currently
  `compile_to_ir(source: str)` is a single-string API.  Adding
  modules means the API takes (entry, search_paths) → IrProgram.
  Backwards compat: keep single-string `compile_to_ir` for tests
  that don't need modules.

- **Static-field placement under multi-class JVM.**  The
  existing `__ca_regs` / `__ca_objregs` arrays live on the main
  user class.  With many modules, EITHER each gets its own
  pair (more allocation) OR one shared "runtime" class hosts
  them.  Latter is cleaner; pick it now to avoid migration.

- **WASM MVP vs WASM GC choice.**  Both are real targets and
  have different tradeoffs.  We don't decide here; we make
  sure the design doesn't accidentally lock us out of either.

- **Cycle detection.**  Module cycles must be diagnosed at
  resolution time, not compile time.  The resolver does
  topo-sort and rejects cycles.  Test coverage: at least one
  cycle test.

## Out of scope

- **Macros.**  TW03 Phase 5 explicitly punted; TW04 doesn't
  bring them back.
- **Re-exports / aliasing / qualified imports.**  v2.
- **Conditional / platform-specific imports.**  No `#+jvm` /
  `#+clr` reader macros (we don't have reader macros).
- **A package manager.**  Module resolution is path-based.
  Packaging up a stdlib for distribution is a separate concern.
- **Dynamic loading.**  `(load "file.tw")` at runtime — no.
  Modules are compile-time.
- **C FFI.**  Calling arbitrary native code (`dlopen`,
  `JNI`, `P/Invoke`).  When a native VM lands, FFI is its
  own spec.

## Sister specs

| Spec | Relation to TW04 |
|---|---|
| [TW00](TW00-twig-language.md) | Defines the Twig language; TW04 adds modules to it |
| [TW02](TW02-twig-jvm-compiler.md) | First per-backend frontend; modules extend it |
| [TW03](TW03-lisp-primitives-and-gc.md) | Heap primitives; TW04 builds on the heap so `print` can dispatch on type |
| TW05 (future) | Macros — would extend TW04 with phase-separated imports |
| TW06 (future) | Native VM + custom GC — would add FFI on top of `host` |
