# Perl IIR compiler (Rust)

Compiles LANG81's bounded Perl 5.38 `print` arithmetic subset directly from
the Rust Perl parser tree to `interpreter-ir`. The `plvm` binary runs it on
Rust `vm-core`. It does not use Semantic IR or a host Perl process at runtime.
The pilot accepts plain decimal integer literals within signed 32-bit range;
legacy leading-zero octal spellings are rejected. Direct AST input is bounded
by item count, depth, token and rule-name text, aggregate text, and module
name size before lowering.

`print` lowers to the LANG83 standard builtin `write_i64` (decimal, no
newline), so the same module runs on every lang-aot backend through
`Language::Perl`: native x86-64/AArch64, LLVM, WASM, JVM, CLR, BEAM, vm-core
and the JIT.

## Release selection (LANG84)

```rust
use perl_iir_compiler::{compile_source_at, FrontendBudgets, FrontendError};

let budgets = FrontendBudgets::default();
// None = DEFAULT_RELEASE (the unversioned LANG81 pair for now).
let module = compile_source_at("print(1 + 2);", None, "demo", &budgets)?;
// Any checked inventory release, by exact ID.
let module = compile_source_at("print(1 + 2);", Some("5.004_72"), "demo", &budgets)?;
// Pending and unknown IDs are typed errors; nothing falls back.
assert!(matches!(
    compile_source_at("print(1);", Some("1.001"), "demo", &budgets),
    Err(FrontendError::ReleaseNotAvailable { .. })
));
```

Every error after resolution names the release; lexer and parser errors
carry line and column, and `Unsupported` carries the CST rule and span.
`FrontendBudgets` bounds source bytes (64 KiB by default), parser depth and
IIR instructions.

Run the bounded example with `cargo run --manifest-path
code/packages/rust/Cargo.toml -p perl-iir-compiler --bin plvm --
code/packages/rust/perl-iir-compiler/examples/arithmetic.pl` from the repository root.
It prints `3-46` without a trailing newline, matching Perl 5.38 for this input.
Add `--release <inventory-id>` (before the path) to compile against a
specific checked release, for example `--release 1.000`.
