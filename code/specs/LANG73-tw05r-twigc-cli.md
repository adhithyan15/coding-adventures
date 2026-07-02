# LANG73 — TW05-R: `twigc` CLI Driver

## Motivation

After LANG72 (TW05-Q), the full Twig compilation pipeline —
multi-file module resolution, Phase 3.5 type-checking, and IIR
compilation — lives entirely inside `compile_module_tree` in
`twig-module-driver`.  However, there is no user-facing CLI tool;
the pipeline is only accessible through Rust integration tests.

TW05-R closes this gap by creating `twigc`: a command-line compiler
driver that exposes `compile_module_tree` + `twig-vm` to end users.
This completes the "Artifact and CLI shape" section of the TW05 spec.

## Solution

New crate `code/packages/rust/twigc/` added to the main Rust
workspace.

### CLI surface

```
twigc [OPTIONS] <file.tw>

Options:
  --check            Type-check only.  Exit 0 on success, 1 on type errors.
  --emit=iir         Compile to IIR and print a human-readable summary to
                     stdout.  Exit 0 on success.
  --search-path=DIR  Add DIR to the module search path (may be repeated).
  -h, --help         Print help.
  -V, --version      Print version.

Default (no flags):  compile and run via twig-vm; print the integer return
value of main() to stdout.
```

### Behaviour

**`--check`** mode:
1. Call `compile_module_tree(path, search_paths)`.
2. If it returns `Err(TypeErrors { … })` print the first error to
   stderr and exit 1.
3. If it returns any other `Err` print the error to stderr and exit 2.
4. On `Ok(_)` exit 0 (type check passed).

**`--emit=iir`** mode:
1. Call `compile_module_tree(path, search_paths)`.
2. On error: same stderr + exit-code policy as `--check`.
3. On `Ok(module)`: iterate `module.functions` and for each function
   print:
   ```
   fn <name>:
     <index>  <op>  <dest>  <srcs…>
   ```
   Exit 0.

**Default (run)** mode:
1. Compile via `compile_module_tree`.
2. Run the resulting `IIRModule` via `twig_vm::TwigVM::new().run_module`.
3. Print the integer return value of `main()` to stdout.
4. Exit 0 on success, non-zero on error.

### Library API (`src/lib.rs`)

```rust
pub fn twigc_check(path: &Path, search_paths: &[PathBuf]) -> Result<(), TwigcError>;
pub fn twigc_emit_iir(path: &Path, search_paths: &[PathBuf]) -> Result<String, TwigcError>;
pub fn twigc_run(path: &Path, search_paths: &[PathBuf]) -> Result<i64, TwigcError>;
```

`TwigcError` wraps `ModuleDriverError` and adds a `Vm` variant for
runtime errors.

## Files changed

| File | Change |
|------|--------|
| `code/specs/LANG73-tw05r-twigc-cli.md` | **new** (this file) |
| `code/packages/rust/twigc/Cargo.toml` | **new** |
| `code/packages/rust/twigc/src/main.rs` | **new** |
| `code/packages/rust/twigc/src/lib.rs` | **new** |
| `code/packages/rust/twigc/BUILD` | **new** |
| `code/packages/rust/twigc/README.md` | **new** |
| `code/packages/rust/twigc/CHANGELOG.md` | **new** |
| `code/packages/rust/Cargo.toml` | Add `"twigc"` to `members` |

## Tests (`twigc_tests`, 6 tests)

| Test | Verifies |
|------|---------|
| `check_clean_strict_program_ok` | `twigc_check` on a valid strict module → `Ok(())` |
| `check_strict_program_with_type_error_fails` | `twigc_check` on bad varref → `Err(TypeErrors)` |
| `check_lenient_bad_varref_passes` | `twigc_check` on lenient bad varref → `Ok(())` |
| `emit_iir_produces_fn_listing` | `twigc_emit_iir` → string contains `fn main:` |
| `run_arithmetic_returns_value` | `twigc_run` on `(define (main) (+ 21 21))` → `Ok(42)` |
| `run_compiler_tree_main_returns_2` | `twigc_run` on `code/packages/twig/compiler/main.tw` → `Ok(2)` (`(main)` returns the span.tw fn count) |

## Version

`twigc`: 0.1.0 (new package)

## Commit sequence

1. `docs(specs)` — `LANG73-tw05r-twigc-cli.md`
2. `feat(twigc)` — new CLI driver package, 6 tests, bump workspace

## Verification

```bash
cargo test -p twigc --lib            # 6 new tests pass
cargo build -p twigc --release       # binary compiles
cargo build --workspace              # clean workspace build
```
