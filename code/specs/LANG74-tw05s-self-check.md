# LANG74 — TW05-S: `twigc --self-check`

## Motivation

The TW05 "Definition of done" states:

> `twigc --self-check` reaches a stage1/stage2 fixed point

LANG73 (TW05-R) created the `twigc` CLI but only exposed `--check`,
`--emit=iir`, and the default run mode.  LANG74 adds `--self-check <dir>`:
the final required CLI mode, which exercises the fixed-point self-compilation
property that was proven algebraically in LANG68 (TW05-N).

## Background

LANG68 added `fixed-point-check` to `compiler/main.tw`:

```scheme
(define (fixed-point-check dir)
  ; Compile span.tw twice; verify identical opcode summaries.
  (let* ((src    (host/read_file (string-append dir "/span.tw")))
         (stage1 (fn-list-ops-str (emit-program (parse-program (lex-source src)))))
         (stage2 (fn-list-ops-str (emit-program (parse-program (lex-source src))))))
    (string=? stage1 stage2)))
```

The function always returns `#t` because Twig is purely functional.
Its value is making the determinism invariant **explicit and testable** via
the CLI, completing TW05's bootstrap story.

## Solution

### New CLI flag

```
twigc --self-check <DIR>
```

`<DIR>` is the path to the Twig compiler source directory (the directory
that contains `span.tw`, `lexer.tw`, `main.tw`, etc.).

**Behaviour:**
1. Discover the compiler's entry point: `<DIR>/main.tw`.
2. The parent of `<DIR>` is added to the module search path so that
   `(import compiler/main)` can resolve.
3. Write an ephemeral wrapper `.tw` file to a temp directory:
   ```scheme
   (module twigc/self-check-runner
     (typed lenient)
     (import compiler/main))
   (define (main)
     (if (fixed-point-check "<DIR-as-string>") 1 0))
   ```
4. Compile and run the wrapper via `twigc_run`.
5. If the result is `1` (`#t` from `fixed-point-check`): print
   `"twigc: self-check passed (fixed point reached)"` and exit 0.
6. If the result is `0` (`#f`): print
   `"twigc: self-check FAILED (fixed point not reached)"` to stderr and exit 5.

### New exit code

| Code | Meaning |
|------|---------|
| 5    | Self-check failed — fixed-point not reached |

(Codes 0–4 are unchanged from TW05-R / LANG73.)

### New library API (`src/lib.rs`)

```rust
pub fn twigc_self_check(
    compiler_dir: &Path,
    extra_search_paths: &[PathBuf],
) -> Result<bool, TwigcError>;
```

Returns `Ok(true)` when `fixed-point-check` returns `#t`, `Ok(false)`
otherwise.  `TwigcError` is returned if compilation or execution fails.

**Implementation sketch:**
1. Canonicalize `compiler_dir`.
2. Derive `search_root = parent(compiler_dir)` and prepend it to the
   search paths (so `(import compiler/main)` resolves).
3. Generate a temp directory and write the wrapper `.tw` source.
4. Call `twigc_run(&wrapper_path, &all_paths)` and map `1` → `true`,
   anything else → `false`.

## Stack requirements

`fixed-point-check` only processes `span.tw` (~365 chars, 2 functions).
The IIR compilation of all 11 modules happens at load time (before `main`
runs) but is not recursive — that phase uses the Rust-based compiler.
The runtime call stack for `fixed-point-check` is shallow enough to stay
within the 8 MiB default thread stack.

## Files changed

| File | Change |
|------|--------|
| `code/specs/LANG74-tw05s-self-check.md` | **new** (this file) |
| `code/packages/rust/twigc/src/lib.rs` | Add `twigc_self_check` + 1 test |
| `code/packages/rust/twigc/src/main.rs` | Add `--self-check` mode |
| `code/packages/rust/twigc/CHANGELOG.md` | Prepend `[0.2.0]` |
| `code/packages/rust/twigc/Cargo.toml` | 0.1.0 → 0.2.0 |

## Tests (`twigc_tests`, +1 test)

| Test | Verifies |
|------|---------|
| `self_check_compiler_tree_fixed_point` | `twigc_self_check` on the real 11-module compiler → `Ok(true)` |

## Version

`twigc`: 0.1.0 → 0.2.0 (minor — new public API function + new CLI flag)

## Commit sequence

1. `docs(specs)` — `LANG74-tw05s-self-check.md`
2. `feat(twigc)` — `twigc_self_check` + `--self-check` CLI, bump 0.2.0

## Verification

```bash
cargo test -p twigc --lib                                      # all 7 tests pass
cargo build -p twigc --release                                 # binary compiles
twigc --self-check code/packages/twig/compiler                          # exit 0
```
