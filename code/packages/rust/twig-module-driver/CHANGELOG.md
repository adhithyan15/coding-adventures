# Changelog — twig-module-driver

## [0.8.0] — 2026-05-16

### Changed (LANG63 — grammar-driven Twig lexer and CST parser)

`"cst-parser"` added to every `copy_all_tw_modules` helper and every explicit
module copy list in `src/lib.rs`.

`compiler/parser` now imports `compiler/cst-parser` (the newly generated CST
parser module).  Without `"cst-parser"` in the copy lists the module driver
cannot resolve that import and all tests that exercise the parser (i.e. every
test that calls `parse-program` or `parse-expr` through the pipeline) would
fail with an `UnresolvedImport` error.

Affected test modules and copy sites:

| Test module         | Updated copy-lists / explicit copies                      |
|---------------------|----------------------------------------------------------|
| `tw05e_tests`       | `copy_all_tw_modules` (9-module list, now includes `"cst-parser"`) |
| `tw05f_tests`       | `copy_all_tw_modules`                                     |
| `tw05g_tests`       | `copy_all_tw_modules`                                     |
| `tw05h_tests`       | `copy_all_tw_modules`                                     |
| `tw05i_tests`       | `copy_all_tw_modules` + two explicit `copy_tw(…, "cst-parser")` calls |

No new tests were added in this release; the change is purely infrastructural
to support the new `compiler/cst-parser` module introduced in LANG63.

---

## [0.7.0] — 2026-05-15

### Added (LANG62 — TW05-I first self-compilation check)

New `#[cfg(test)] mod tw05i_tests` with 6 integration tests that exercise the
full lex → parse → `emit-program` pipeline on `compiler/span.tw` — the first
of the compiler's own modules.

#### Tests added

| Test | What it verifies |
|------|-----------------|
| `self_compile_stripped_span_fn_count` | Stripped span source → 2 emitted functions |
| `self_compile_stripped_span_fn_names` | First emitted function is `"make-span"` |
| `self_compile_dummy_span_instr_count` | `dummy-span` body emits exactly 4 instructions |
| `self_compile_make_span_instr_count` | `make-span` body emits exactly 12 instructions |
| `self_compile_real_span_tw` | Actual `span.tw` file content (read at runtime) → 2 functions |
| `full_lex_parse_emit_self_compile` | All 9 modules + `main.tw` → `(main) = 2` |

#### Key behaviours verified

- Lexer comment-skipping (`; …` lines skipped) and colon-token handling
  (`(source-id : int)` → `TkColon` → `NilLit` fallback in parser)
- Parser fallback `parse-call` path for `(module ...)` and `(record ...)` forms
  (both become `CallExpr`, not `DefExpr`)
- `emit-program` skip-logic: only `DefExpr(LambdaExpr)` nodes are emitted
- `make-span` `IfExpr` body → 12 IIR instructions
- `dummy-span` `CallExpr(Span, ...)` body → 4 IIR instructions

#### `main.tw` updated to TW05-I

`main.tw` now runs the pipeline on a comment-stripped version of `span.tw`
(assembled via `string-append`) and returns `(length funcs)` = 2.  The
`MAX_DISPATCH_DEPTH` bump (256 → 4096, in `twig-vm` 0.16.0) is required for
lexing the ~365-char source.

---

## [0.6.0] — 2026-05-15

### Added (LANG61 — TW05-H self-hosted program emitter)

New `#[cfg(test)] mod tw05h_tests` with 6 tests exercising the updated
`compiler/emit.tw` (added `emit-program`, `emit-top-level-form`,
`emit-program-loop`, `emit-symlit`) and `compiler/main.tw` (now returns 2 —
the count of emitted function definitions from a two-define program).

#### Emitter changes (`emit.tw`)
- Gate 3 extended: tests `StrLit?` first, then `SymLit?`, then falls through
  to gate 4.  `SymLit` previously fell through to nil; now correctly emits a
  `const` instruction via `emit-symlit`.
- New `emit-symlit`: allocates a slot, emits `(IirInstr "const" dest (list val) (TAny) sp)`,
  mirrors `emit-strlit` but retrieves the value via `symlit-value`.
- New `emit-program`: entry point for whole-program emission.  Calls
  `emit-program-loop` with an empty accumulator.
- New `emit-program-loop`: tail-recursive accumulator loop.  For each form calls
  `emit-top-level-form`; skips nil results; reverses accumulator at end.
- New `emit-top-level-form`: processes one top-level `Expr` node.  If the node
  is a `DefExpr` whose body is a `LambdaExpr`, creates a fresh `IirBuilder`,
  calls `emit-lambdaexpr` with an empty env, finalises the builder, and returns
  `(cons fn-name instruction-list)`.  Otherwise returns nil.

#### `main.tw` updated to TW05-H smoke test
Lexes, parses, and emits `"(define (double x) (* x 2)) (define (triple x) (* x 3))"`;
`(length (emit-program forms))` returns 2.

| Test | Verifies |
|------|----------|
| `emit_program_single_fn` | `emit-program` on `"(define (f x) x)"` → 1 entry |
| `emit_program_two_fns` | two defines → 2 entries |
| `emit_program_fn_name` | first entry's `car` = `"answer"` |
| `emit_program_fn_instruction_count` | `"(define (double x) (* x 2))"` → 2 instructions |
| `emit_symlit_one_instruction` | `(SymLit "foo" sp)` → 1 instruction |
| `full_lex_parse_emit_program` | All 9 modules + main.tw → `(main) = 2` |

## [0.5.0] — 2026-05-15

### Added (LANG60 — TW05-G lambda expressions + function definitions)

New `#[cfg(test)] mod tw05g_tests` with 6 tests exercising the updated
`compiler/ast.tw`, `compiler/parser.tw`, `compiler/emit.tw`, and `main.tw`
that together implement lambda expressions and the `(define (name args) body)`
function-definition shorthand.

#### AST changes (`ast.tw`)
`LambdaExpr (params : any) (body : any) (span : any)` added as tag 11.
Exports `LambdaExpr`, `LambdaExpr?`, `lambdaexpr-params`, `lambdaexpr-body`,
`lambdaexpr-span`.

#### Parser changes (`parser.tw`)
- New gate `parse-list-6` dispatches on `"lambda"` keyword.
- `parse-define` now dispatches: `TkLParen` first token → `parse-define-fn`
  (function shorthand); otherwise → `parse-define-simple` (original path).
- New helpers: `parse-define-fn`, `parse-define-simple`, `parse-lambda`,
  `parse-param-list`.
- `(define (name params) body)` parses to
  `DefExpr name (LambdaExpr params body sp)`.

#### Emitter changes (`emit.tw`)
- Gate 9 (new): `DefExpr?` → `emit-defexpr`.
- Gate 10 (new): `LambdaExpr?` → `emit-lambdaexpr`.
- Gate 11 (was 9): `CallExpr?` / fallback.
- New functions: `emit-defexpr`, `emit-lambdaexpr`, `emit-lambda-params`.

#### `main.tw` updated to TW05-G smoke test
Lexes and emits `"(define (answer) 42)"` through the full pipeline; still
returns 42.

| Test | Verifies |
|------|----------|
| `parser_lambda_expr` | `(parse-program "(lambda (x) x)")` → `LambdaExpr?` = 1 |
| `parser_define_fn_form` | `(define (f x) x)` → `defexpr-expr` is a `LambdaExpr` |
| `emit_lambda_no_params` | emit `(LambdaExpr [] (IntLit 99))` → 1 instruction |
| `emit_lambda_with_param` | emit `(LambdaExpr ["x"] (+ x 1))` → 2 instructions |
| `emit_defexpr_answer_42` | emit `(DefExpr "answer" (LambdaExpr [] (IntLit 42)))` → 1 instruction |
| `full_lex_parse_emit_defexpr` | All 9 modules + main.tw → `(main) = 42` |

## [0.4.0] — 2026-05-15

### Added (LANG59 — TW05-F self-hosted IIR emitter integration tests)

New `#[cfg(test)] mod tw05f_tests` with 6 tests exercising `compiler/emit.tw`
(the self-hosted IIR emitter) through the full module driver pipeline.
`copy_all_tw_modules` updated to include `"emit"`.

| Test | Verifies |
|------|----------|
| `emit_intlit_one_instruction` | emit `(IntLit 42 sp)` → 1 instruction |
| `emit_call_plus_1_2` | emit `(CallExpr (VarRef "+") [IntLit 1, IntLit 2])` → 3 instructions |
| `emit_if_expr_count` | emit `(IfExpr (BoolLit #t) (IntLit 1) (IntLit 2))` → 9 instructions |
| `emit_let_binding_count` | emit `(LetExpr [("x", IntLit 1)] (VarRef "x"))` → 1 instruction |
| `emit_begin_sequence_count` | emit `(BeginExpr [IntLit 1, IntLit 2, IntLit 3])` → 3 instructions |
| `full_lex_parse_emit_roundtrip` | All 9 modules + main.tw → `(main) = 42` |

`full_lex_parse_roundtrip` (tw05e_tests) updated: comment clarified to reflect
that main.tw now runs the full lex → parse → emit pipeline (still returns 42).

## [0.3.0] — 2026-05-15

### Added (LANG58 — TW05-E self-hosted lexer + parser integration tests)

New `#[cfg(test)] mod tw05e_tests` with 7 tests exercising `compiler/lexer.tw`
and `compiler/parser.tw` through the full module driver pipeline:

| Test | Verifies |
|------|----------|
| `lexer_single_integer_token` | `(lex-source "42")` first token lexeme = `"42"` |
| `lexer_parens_and_identifier` | `(lex-source "(foo)")` → 4 tokens |
| `lexer_skips_whitespace` | `"  42  "` → 2 tokens (TkInteger + TkEOF) |
| `lexer_skips_comment` | `"; comment\n42"` → 2 tokens |
| `parser_integer_literal` | parse `"99"` → `(intlit-value expr) = 99` |
| `parser_nested_call` | parse `"(+ 1 2)"` → `(length (callexpr-args expr)) = 2` |
| `full_lex_parse_roundtrip` | All 8 modules + main.tw → `(main) = 42` |

## [0.2.0] — 2026-05-15

### Added (LANG57 — TW05-D compiler data model integration tests)

#### Phase 3 extern-name collection extended to record/union definitions

`compile_module_tree` Phase 3 now collects function names generated by
`Form::RecordDef` and `Form::UnionDef` in addition to `Form::Define(lambda)`.

For `(record Name (f0 : T) …)` the following names are pre-registered:
- Constructor `Name`
- Predicate `<lowercase(Name)>?`
- Accessors `<lowercase(Name)>-<fi>` for each field

For `(union Name (V0 (g0 : T) …) …)` per variant:
- Constructor `V0`
- Predicate `V0?`  (keeps original case — mirrors `emit_union_def` in the compiler)
- Accessors `<lowercase(V0)>-<gj>` for each field

Without this fix, calling record accessors or union predicates across module
boundaries produced "unbound name" compile errors.

#### `twig-vm` added as dev-dependency

Added `twig-vm = { path = "../twig-vm" }` as a `[dev-dependencies]` entry to
support the new integration tests that execute compiled module trees.

#### `tw05d_tests` — 6 integration tests for `code/twig/compiler/`

Six new tests in `#[cfg(test)] mod tw05d_tests` that exercise the real
`.tw` source files from `code/twig/compiler/`:

| Test | Assertion |
|------|-----------|
| `span_make_span_valid_invariant` | `(span-start (make-span 0 3 7))` → 3 |
| `span_make_span_bad_invariant_returns_nil` | `(if (make-span 0 7 3) 1 0)` → 0 |
| `token_tkinteger_predicate` | `(if (TkInteger? (TkInteger)) 1 0)` → 1 |
| `ast_intlit_accessor_extracts_value` | `(intlit-value (IntLit 99 nil))` → 99 |
| `iir_builder_alloc_slot_increments_reg_count` | `new-builder` + `alloc-slot` → reg-count 1 |
| `full_module_tree_smoke_test` | Compile all 7 modules, run `(main)` → 1 |

---

## [0.1.0] — 2026-05-14

### New crate (LANG56 — Multi-File Module Driver)

First release.  Implements the file-level driver that turns a root `.tw` source
file (and all its transitive imports) into a single, fully-linked `IIRModule`
ready for `twig-vm::run`.

#### `compile_module_tree(root_path, search_roots) -> Result<IIRModule, ModuleDriverError>`

Four-phase pipeline:

1. **Discovery** — BFS from `root_path`, reading and parsing each `.tw` file
   encountered.  Import names (e.g. `"compiler/lexer"`) are resolved to absolute
   paths by scanning the requesting file's directory and then `search_roots` in
   order.  Each module is parsed exactly once (canonical-path dedup).

2. **Cycle detection** — Iterative DFS with three-colour marking (White / Grey /
   Black) on the adjacency graph built during discovery.  A back-edge to a Grey
   node (module currently on the DFS stack) triggers `CircularImport`.  This is
   separate from discovery so that shared dependencies (two modules both importing
   the same library) do not produce false positives.

3. **Compilation with externs** — Every top-level function name from every
   discovered module is collected and injected into the compiler as "externs" via
   the new `twig_ir_compiler::compile_program_with_externs`.  This allows
   cross-module calls to compile to `call` instructions rather than failing with
   "unbound name".

4. **Linking** — `iir_linker::link(&[IIRModule])` merges all compiled modules into
   one self-contained `IIRModule`.  The root module keeps `entry_point = Some("main")`; all library modules have their `entry_point` cleared before linking.

#### `resolve_import(import_name, search_roots, requesting_file) -> Option<PathBuf>`

Converts a slash-separated module name (e.g. `"stdlib/io"`) to a canonical file
path by searching the requesting file's directory first, then each search root.
The `.tw` extension is always appended.

#### `ModuleDriverError` variants

- `Io { path, error }` — could not read a source file
- `Parse { path, error }` — source file contained a syntax error
- `Compile { path, error }` — source file failed the IR compiler
- `UnresolvedImport { import_name, searched }` — no `.tw` file found in any root
- `CircularImport { cycle_member }` — import graph contains a cycle
- `Link(Vec<LinkError>)` — `iir_linker::link` failed (usually a name collision)

#### Tests

13 unit tests covering:

- `resolve_import_finds_file_in_sibling_dir`
- `resolve_import_uses_explicit_search_root`
- `resolve_import_returns_none_for_missing`
- `single_file_no_imports` — backward-compat: plain Twig programs work unchanged
- `single_file_with_module_decl_exports_populates_iirexport` — compiler-level export check
- `two_file_import_library_function_callable` — root calls lib function post-link
- `three_file_chain_transitive_import` — root → lib-a → lib-b transitivity
- `shared_dependency_compiled_once` — shared lib compiled exactly once (no dup-fn error)
- `unresolved_import_returns_error`
- `circular_import_returns_error`
- `export_only_lists_declared_names` — compiler-level export filter check
- `empty_module_no_panic`
- `library_module_has_no_entry_point`
