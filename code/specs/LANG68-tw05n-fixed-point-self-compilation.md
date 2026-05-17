# LANG68 — TW05-N: Fixed-Point Self-Compilation

> **LANG67 (TW05-M) is merged.** This spec covers TW05-N: verifying the
> fixed-point property of the self-hosted Twig compiler pipeline.

---

## Background

TW05-M (LANG67) demonstrated that the self-hosted pipeline (`lex-source →
parse-program → emit-program`) can compile all eleven compiler source files
from disk and count 173 emitted function definitions.  At that stage the
test only verified *how many* functions were emitted; it said nothing about
the *content* of the IIR.

TW05-N closes that gap by:

1. Adding an **IIR opcode-summary serialiser** to `main.tw` that converts
   the emitted instruction lists to a canonical string.
2. Adding a **`fixed-point-check`** function that runs the pipeline twice on
   `span.tw` (the smallest source file) and verifies the two runs produce
   byte-for-byte identical opcode summaries.
3. Adding a **`self-compile-all-summary`** function that compiles all eleven
   files and returns the full opcode-structure string — used as a stable
   regression anchor.
4. Updating all tests to the new function counts (main.tw gains six helpers,
   changing its contribution from 2 → 8, bringing the grand total from
   173 → 179).

---

## Fixed-Point Semantics

The TW05 spec (Appendix D) describes a three-stage bootstrap:

```
Stage 0:  Rust twig-ir-compiler compiles compiler.tw → IIR
          IIR runs on twig-vm (interpreted) → self-hosted binary B0

Stage 1:  B0 compiles compiler.tw → IIR               (stage1 IIR)
Stage 2:  B0 compiles compiler.tw again → IIR          (stage2 IIR)

Fixed-point: stage1 IIR == stage2 IIR
```

In the TW05-N implementation, B0 = `twig-module-driver` executing the
self-hosted pipeline via `twig-vm`.  "Stage 1" and "Stage 2" are two
successive calls to `(fixed-point-check dir)` (or equivalently, two calls
to `(fn-list-ops-str (emit-program ...))` on the same input).

Because Twig is **purely functional**, identical inputs always produce
identical outputs — the fixed-point property holds trivially.  The purpose
of `fixed-point-check` is to make this invariant **explicit and testable**:
it becomes a concrete test rather than an assumption.

---

## New Functions in `main.tw`

Six new helper functions are added to `compiler/main`.  All are exported so
integration tests can call them directly.

### `(instr-op-tag instr)`

Extracts the opcode string (`iirinstr-op instr`) from one `IirInstr` record.
Returns a string such as `"const"`, `"call_builtin"`, `"jmp_if_false"`, etc.

### `(instr-list-ops instrs)`

Joins the opcodes of a list of `IirInstr` records with `"|"`.

| Input | Output |
|-------|--------|
| `nil` | `""` |
| one const instr | `"const"` |
| const + call_builtin | `"const\|call_builtin"` |

### `(fn-pair-ops-str pair)`

Serialises one `(fn-name . instr-list)` pair to the canonical string:

```
fn-name N op1|op2|...|opN
```

where `N` is the instruction count (decimal integer).

Examples:
- `make-span 12 const|const|call_builtin|...`
- `dummy-span 4 const|const|const|call_builtin`

### `(fn-list-ops-str fns)`

Newline-joins `fn-pair-ops-str` results for all functions.  Returns `""`
for an empty list (union/record-only modules such as `token.tw`).

### `(self-compile-all-summary dir)`

Reads all eleven compiler source files from `dir` via `host/read_file`,
compiles each through the full `lex-source → parse-program → emit-program`
pipeline, and returns the concatenated opcode-summary string.  Files are
separated by `"\n---\n"`.

Expected content of the summary (selected snippets):
- `"make-span 12 "` — from `span.tw`
- `"dummy-span 4 "` — from `span.tw`
- `"new-builder "` — from `iir-builder.tw`
- `"lex-source "` — from `lexer.tw`

### `(fixed-point-check dir)`

Compiles `span.tw` (the smallest compiler source file at ~365 chars) **twice**
using the same pipeline, then returns `#t` if and only if both runs produce
identical opcode summaries.

```scheme
(define (fixed-point-check dir)
  (let* ((src    (host/read_file (string-append dir "/span.tw")))
         (stage1 (fn-list-ops-str (emit-program (parse-program (lex-source src)))))
         (stage2 (fn-list-ops-str (emit-program (parse-program (lex-source src))))))
    (string=? stage1 stage2)))
```

`span.tw` is chosen for speed: its pipeline runs in milliseconds even in
debug mode, so `fixed-point-check` adds negligible overhead to the test suite.

---

## Updated Function Counts

| Module | LANG67 count | LANG68 count | Delta |
|--------|-------------|-------------|-------|
| `span.tw` | 2 | 2 | — |
| `token.tw` | 0 | 0 | — |
| `diagnostic.tw` | 3 | 3 | — |
| `ast.tw` | 0 | 0 | — |
| `iir-types.tw` | 0 | 0 | — |
| `iir-builder.tw` | 8 | 8 | — |
| `lexer.tw` | 25 | 25 | — |
| `cst-parser.tw` | 69 | 69 | — |
| `parser.tw` | 29 | 29 | — |
| `emit.tw` | 35 | 35 | — |
| `main.tw` | **2** | **8** | +6 |
| **Total** | **173** | **179** | **+6** |

The six new functions in `main.tw` are the serialisation helpers and the
fixed-point entry points listed above.

---

## Acceptance Criteria

1. `(fixed-point-check dir)` returns `#t`.
2. `(self-compile-all dir)` returns `179`.
3. `(length (emit-program (parse-program (lex-source (host/read_file (string-append dir "/main.tw"))))))` returns `8`.
4. `(self-compile-all-summary dir)` contains the substring `"make-span 12 "`.
5. `(self-compile-all-summary dir)` contains the substring `"dummy-span 4 "`.
6. All existing TW05-L / TW05-M regression tests continue to pass (7-file
   set still returns 171; `(main)` still returns 2).

---

## VM and Stack Limits

No changes to `MAX_DISPATCH_DEPTH` or `MAX_INSTRUCTIONS_PER_RUN` are needed.

### Instruction budget

The new helper functions add instructions only for calls to
`self-compile-all-summary` and `fixed-point-check`:

- `self-compile-all-summary` in main.tw: large `let*` body, approximately
  100–150 instructions.
- `fixed-point-check` in main.tw: small function, approximately 20–30
  instructions.
- Other helpers (`instr-op-tag`, `instr-list-ops`, `fn-pair-ops-str`,
  `fn-list-ops-str`): each approximately 5–15 instructions.

Additional contribution: ~6 functions × ~60 instructions avg ≈ 360 new
instructions in `main.tw`; grand total still far below `MAX_INSTRUCTIONS_PER_RUN`.

### Recursion depth

`instr-list-ops` recurses over instruction lists.  The deepest function in
the compiler corpus has at most ~200 instructions, adding ≤ 200 extra frames
on top of the lexing stack.  The existing 3 GiB `run_in_xlarge_stack` thread
provides more than adequate headroom.

---

## Files Changed

| File | Change |
|------|--------|
| `code/specs/LANG68-tw05n-fixed-point-self-compilation.md` | **new** |
| `code/twig/compiler/main.tw` | add 6 functions, update exports + header |
| `code/packages/rust/twig-module-driver/src/lib.rs` | update 2 test expectations (173→179, 2→8); add `tw05n_tests` (7 tests) |
| `code/packages/rust/twig-module-driver/Cargo.toml` | `0.12.0 → 0.13.0` |
| `code/packages/rust/twig-module-driver/CHANGELOG.md` | prepend `[0.13.0]` |

---

## Commit Sequence

1. `docs(specs)` — `LANG68-tw05n-fixed-point-self-compilation.md`
2. `feat(twig)` — add IIR serialisers + fixed-point check to `main.tw`
3. `test(twig-module-driver)` — update counts + `tw05n_tests`, bump 0.13.0

---

## Verification

```bash
cargo test -p twig-module-driver -- tw05n      # 7 new tests pass
cargo test -p twig-module-driver -- tw05m      # existing tests still pass (updated counts)
cargo test -p twig-module-driver -- tw05l      # regression: 171 unchanged
cargo build --workspace                         # clean build
```
