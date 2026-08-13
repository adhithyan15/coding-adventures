# W05 — a real conformance harness for the in-repo WASM engine

> Status: **draft — spec-first sign-off gate, no code**. Phase A of a
> larger, user-directed arc toward making `wasm-execution` "as good as
> wasmtime." Scoped strictly to this phase: build a harness that runs the
> **official** WebAssembly spec testsuite against `wasm-execution` and
> produce a real, git-pinned baseline pass-rate number. Widening opcode
> coverage (SIMD, threads/atomics, exceptions, tail calls, reference-types,
> general GC), a WASM→IIR lowering pass, and a JIT tier built on `jit-core`
> are explicitly out of scope — see §6.

## 1. Why this matters, precisely

`wasm-execution` has never been checked against anything other than its own
hand-written unit tests (148 `#[test]`s in `lib.rs`, 11 in `gc.rs`, no
`tests/` directory). It has never run the official spec testsuite, and has
never been cross-checked against a real engine. Every claim this repo has
made about its WASM coverage — "the complete WASM instruction set", "~182
instruction handlers" — is self-reported by the crate's own doc comments,
not independently verified. Before any further WASM work (widening opcode
coverage, adding a JIT tier) can be trusted, there needs to be a real,
externally-sourced, reproducible measurement of where the interpreter
actually stands today.

## 2. What already exists, and the two gaps that shape this design

- `wasm-module-parser` (0.2.0) parses **binary** `.wasm` bytes only.
  `wasm-module-encoder` (0.2.0) emits binary bytes only. **Neither, nor
  anything else in this repo, parses the WASM *text* format
  (`.wat`/`.wast`)** — confirmed by exhaustive grep for "wast", "wat", and
  the testsuite's own directive keywords (`assert_return`, `assert_trap`,
  etc.) across every WASM-related crate. The official testsuite ships
  almost entirely as `.wast` files (S-expression text, with occasional
  inline `(module binary "...")`/`(module quote "...")` escape hatches into
  raw bytes). **This repo cannot consume the real testsuite today without a
  new text-format parser.**
- `wasm-validator` (0.1.0) does *module-level structural validation only* —
  its own doc comment says so explicitly (index bounds, unique exports,
  memory/table cardinality, segment validity). It does **not** do
  instruction-level type-checking (the abstract-stack-machine pass that
  catches, e.g., `i32.add` fed an `f64`). `code/specs/W02-wasm-validator.md`
  §"Phase 2: Type Checking" (line 394 onward) already fully designs this —
  abstract value-type stack, control-frame stack, per-instruction type
  rules, the "unreachable" polymorphic-stack state — it was simply never
  implemented. **The official testsuite's `assert_invalid` directives
  (modules that must be *rejected* for a type violation) cannot be graded
  correctly without this pass.** This spec does not implement it (that's
  `W02`'s own job); it designs the harness to degrade gracefully around the
  gap instead (§4.3).

A third, smaller but load-bearing gap: `wasm-runtime::call()`
(`wasm-runtime/src/lib.rs:1275`) is the crate's only public execution entry
point, and it is **lossy for floats** — its result-conversion arm does
`WasmValue::F32(v) => *v as i64` / `F64(v) => *v as i64` (lines 1385-1386),
a numeric *truncation* (Rust's `as` cast), not a bit reinterpretation. The
testsuite's `assert_return` on `f32.wast`/`f64.wast`/`float_exprs.wast`/etc.
requires bit-exact result comparison (including NaN payload/sign
distinctions) — `call()` as it stands cannot support this. §4.4 adds a
narrow, additive `call_typed` entry point instead of touching `call()`.

## 3. New crate: `wasm-wast-parser`

Sibling to `wasm-module-parser`, in `code/packages/rust/wasm-wast-parser`.
Depends on `wasm-types` (produces `WasmModule`/`FuncType`/etc. structures
directly — no round-trip through `wasm-module-encoder`) and `wasm-opcodes`
(reuses the existing mnemonic→opcode table rather than duplicating it). Owns
its own error type (`WastParseError`), distinct from
`wasm_module_parser::WasmParseError`, so a harness can tell "our text parser
rejected this" apart from "our binary parser rejected this" when grading
`assert_malformed`.

### 3.1 Grammar scope

Driven by what the actual testsuite files use, not a full from-scratch WAT
spec reading:

- **Tokenizer**: atoms, parens, line comments (`;;`), and **nestable**
  block comments (`(; ... ;)` — a real WAT feature; a non-nesting
  implementation breaks on the first file that comments out a block
  containing another comment).
- **Module forms**: `type`, `func` (named and positional params/locals),
  `import`, `export`, `memory`, `table`, `global`, `elem`, `data`, `start`.
- **Folded instruction syntax**: `(i32.add (i32.const 1) (local.get 0))`
  must flatten recursively into the linear postfix instruction sequence the
  interpreter actually walks. This is the single most pervasive construct
  in real `.wast` files — nearly every instruction in every file uses it —
  so it is the first thing implemented and tested, not an afterthought.
- **Symbolic identifiers** (`$name`): every WASM index space (types,
  functions, locals, globals, labels, tables, memories) can be named instead
  of numbered. Needs a scoped symbol table — locals/labels are
  function-scoped, everything else module-scoped — resolved to real indices
  before handing the module to `wasm-execution`.
- **Implicit type deduplication**: `(func (param i32) (result i32) ...)`
  implicitly creates-or-reuses a module-level `(type (func ...))` entry by
  structural match; downstream `call_indirect`/explicit `type` references
  depend on this resolving to the same index.
- **Numeric literals**: decimal/hex integers, decimal and **hex floats**
  (`0x1.8p3`, parsed IEEE-754-exact — an approximate `f64::from_str`-style
  parse is not good enough here), `inf`, `nan`, `nan:0x<payload>`, and
  digit-separator underscores (`1_000_000`).
- **String escapes**: `\n \t \\ \" \'`, `\u{XXXX}`, and raw `\XX` hex-byte
  escapes (used to embed intentionally-invalid byte sequences for
  `assert_malformed` text-variant cases).
- **Script directives**: `module` (plus `(module binary "...")` and
  `(module quote "...")` variants), `register`, `invoke`, `assert_return`
  (including `nan:canonical`/`nan:arithmetic` result literals, which
  compare by NaN *class*, not bit-exact value), `assert_trap`,
  `assert_exhaustion`, `assert_invalid`, `assert_malformed`,
  `assert_unlinkable`.

## 4. New crate: `wasm-conformance`

In `code/packages/rust/wasm-conformance`, shaped like `sir-conformance`
(corpus + oracle + run + report), single-target here since there is one
interpreter, not several backends.

### 4.1 Directive executor

`src/lib.rs` parses a `.wast` file via `wasm-wast-parser`, then walks its
directives **in file order**, maintaining a module registry
(`HashMap<Option<String>, WasmInstance>`, keyed by a `register` name or
`None` for "the current module") so cross-module `invoke`/`register`/
`assert_unlinkable` resolve correctly. It owns the bit-exact float/NaN
comparison logic `assert_return` needs.

### 4.2 Report shape

`src/report.rs` defines:

```rust
enum DirectiveOutcome { Pass, Fail(String), Trap(String), NotYetSupported }
```

with per-file and aggregate tallies, broken down **by directive kind**
(`assert_return`, `assert_trap`, `assert_invalid`, ...) so the report
distinguishes "the interpreter is wrong" from "we haven't built the
type-checker yet" at a glance.

### 4.3 Handling `assert_invalid` / `assert_malformed` without a type-checker

- `assert_return` / `assert_trap` / `assert_exhaustion` run *valid* modules
  — the WASM spec guarantees this by construction — so today's
  structural-only `wasm-validator` accepts them fine and the interpreter
  can just execute them. **No blocker; these are graded as real
  pass/fail from day one.**
- `assert_invalid` needs the type-checker `W02` §"Phase 2" already designs
  but doesn't implement. The harness calls `wasm_validator::validate()`
  regardless; when it (wrongly, for now) accepts a module the spec says
  should be rejected, that's recorded as `NotYetSupported`, not `Fail`.
  Once `W02` Phase 2 ships, these flip to real graded outcomes with **zero
  harness changes** — the dispatch already routes through
  `wasm_validator::validate()`.
- `assert_malformed` on the **binary** module variant is *not* blanket
  deferred — `wasm-module-parser` already has real, working error paths for
  LEB128 overflow, bad section ordering, truncation, and bad magic, so many
  binary `assert_malformed` cases are gradeable today. Only the **text**
  (`quote`) variant and specific cases needing type-checking knowledge are
  `NotYetSupported`.

### 4.4 `wasm-runtime::call_typed` — additive, not a `call()` rewrite

```rust
pub fn call_typed(
    &mut self,
    instance: &mut WasmInstance,
    name: &str,
    args: &[WasmValue],
) -> Result<Vec<WasmValue>, TrapError>
```

Thin-wraps the already-typed `WasmExecutionEngine::call_function` directly,
skipping the lossy i64 round-trip `call()` does. `call()` itself and its
existing callers/tests are untouched — this is a pure addition.

### 4.5 The baseline mechanism

`src/bin/wasm_conformance_report.rs` is the actual day-to-day deliverable: a
CLI that walks the vendored `.wast` files and prints a per-file table plus
an aggregate line per directive kind
(`assert_return: 812/900 (90.2%), assert_invalid: 0/140 (not yet
supported), ...`) — what a maintainer runs to see "where do we actually
stand" and "what should I fix next."

`tests/testsuite_conformance.rs` is **one** data-driven test (not one per
file, matching this repo's `html-lexer` fixture-driven pattern rather than
40 near-duplicate test functions) that runs every vendored file and diffs
the result against a checked-in golden manifest
(`tests/fixtures/testsuite-status.json`). It fails on **any** change from
the committed baseline, improvement or regression — the number can never
silently drift; every change to it is a deliberate, reviewed commit. The
report binary gets a `--write-baseline` flag to regenerate the manifest
after an intentional change.

## 5. Vendoring the real testsuite

Fetched from `WebAssembly/testsuite` at a **pinned commit SHA** (never
`main` — confirmed live: `main` already interleaves GC/exceptions/tail-call/
SIMD proposal files with MVP ones, so an unpinned vendor script produces a
non-reproducible baseline on every re-run). A Python fetch script under
`wasm-conformance/tests/fixtures/` follows this repo's existing
`html-lexer`-style vendoring pattern (a `generate_*_fixture.py`-equivalent
script with an explicit `curl`/fetch provenance comment). Files are vendored
**verbatim**, alongside the upstream `LICENSE` (Apache-2.0) and a `NOTICE`
recording the pinned SHA and fetch date.

Initial slice — MVP core only, ~38 files, deliberately excluding anything
needing the `spectest` host-import module or heavier module-linking
semantics (its own later increment, once `register` is proven on simpler
files):

- **Numerics/literals**: `i32.wast`, `i64.wast`, `f32.wast`, `f64.wast`,
  `f32_bitwise.wast`, `f64_bitwise.wast`, `f32_cmp.wast`, `f64_cmp.wast`,
  `int_exprs.wast`, `int_literals.wast`, `float_literals.wast`,
  `float_exprs.wast`, `float_misc.wast`, `conversions.wast`, `const.wast`
- **Control flow**: `block.wast`, `loop.wast`, `if.wast`, `br.wast`,
  `br_if.wast`, `br_table.wast`, `return.wast`, `labels.wast`, `nop.wast`,
  `unreachable.wast`, `switch.wast`, `forward.wast`
- **Calls**: `call.wast`, `call_indirect.wast`, `func.wast`,
  `func_ptrs.wast`, `fac.wast`
- **Variables**: `local_get.wast`, `local_set.wast`, `local_tee.wast`,
  `global.wast`
- **Memory**: `memory.wast`, `address.wast`, `align.wast`,
  `endianness.wast`, `load.wast`, `store.wast`, `memory_size.wast`,
  `memory_grow.wast`, `memory_trap.wast`, `traps.wast`
- **Parser self-test**: `select.wast`, `comments.wast`

## 6. Explicitly out of scope

- **Any opcode/proposal coverage widening** (SIMD, threads/atomics,
  exceptions, tail calls, general reference-types/GC beyond
  `wasm-execution`'s existing narrow `i31ref` + one-struct-shape slice, the
  component model). This phase only *measures*; the corresponding
  testsuite files (`simd_*.wast`, `array*.wast`/`struct.wast`/`i31.wast`/
  `ref_*.wast`/`br_on_*.wast`/`call_ref.wast`, `tag.wast`/`throw*.wast`/
  `try_table.wast`, `return_call*.wast`, memory64 variants) are deliberately
  not vendored yet — each becomes its own future phase's vendored slice,
  reusing this same harness unchanged.
- **`W02`'s type-checker implementation.** Its design already exists; this
  spec only wires the harness to route around its absence cleanly (§4.3),
  so implementing it later is a pure win with no harness rework.
- **A WASM→IIR/CIR lowering pass and a `jit-core`-based JIT tier for
  `wasm-execution`.** Investigated as a longer-term direction this session;
  `jit-core` only accepts IIR as input, and `wasm-execution` has zero
  present connection to IIR/`vm-core`/`aot-core`/`jit-core` — building that
  bridge is a substantial separate effort with no dependency on this
  harness existing first (though this harness would become that future
  lowering pass's own conformance gate once it exists).
- **Linking/imports/exports files needing the `spectest` host module**
  (`imports*.wast`, `linking*.wast`, `exports*.wast`, `instance.wast`,
  `inline-module.wast`) and **numbered duplicate/edge-variant files**
  (`address0/1`, `align0/64`, and similar) — deferred as a mechanical
  follow-up once the core harness is proven on the simpler slice, not
  because they are individually hard.

## 7. Staged commits

1. This spec (sign-off only).
2. `wasm-wast-parser` — tests first (hand-written `.wat` snippets covering
   every hard grammar corner from §3.1), then implementation, then
   BUILD/README/CHANGELOG.
3. Vendoring — fetch script + the pinned-SHA file list from §5 +
   LICENSE/NOTICE. No Rust code in this PR; a reviewable, inspectable diff
   of exactly what upstream text landed.
4. `wasm-conformance` harness + `wasm-runtime::call_typed` — directive
   executor, bit-exact comparison, module registry, report CLI, golden
   baseline manifest + data-driven test, README documenting the maintainer
   workflow. This PR delivers the actual baseline number this phase exists
   to produce.
