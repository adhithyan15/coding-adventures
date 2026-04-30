# Changelog — twig

All notable changes to this package will be documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

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
