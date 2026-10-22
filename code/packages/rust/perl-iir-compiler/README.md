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
// None = DEFAULT_RELEASE: the inventory release 5.38.0, grounded in tag v5.38.0.
let module = compile_source_at("print(1 + 2);", None, "demo", &budgets)?;
assert_eq!(module.release.as_deref(), Some("5.38.0")); // recorded provenance
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
IIR instructions. The unversioned LANG81 `perl.tokens`/`perl.grammar` pair
is no longer reachable from this crate (LANG84 C4.5); it remains only behind
`perl-lexer::tokenize_perl` and `perl-parser::parse_perl`.

## Contract tests (LANG84 C5, C8)

- `cargo test -p perl-iir-compiler cst_contract` checks, for every checked
  release, that its compiled grammar has every rule (`CST_RULES`) and token
  type (`CST_TOKEN_TYPES`) the compiler dispatches on, lexes every spelling
  it compares (`CST_TOKEN_SPELLINGS`) as one token, and parses the corpus
  into exactly the shapes the compiler destructures.
- `hostile_inputs_end_in_typed_errors_in_a_child_process` re-runs this test
  binary as a child that feeds deep nesting, long operator chains,
  unterminated constructs, floods and oversized input through three
  releases. Every probe must end in a typed error or a bounded result; a
  stack overflow or abort shows up as the child's signal.

Run the bounded example with `cargo run --manifest-path
code/packages/rust/Cargo.toml -p perl-iir-compiler --bin plvm --
code/packages/rust/perl-iir-compiler/examples/arithmetic.pl` from the repository root.
It prints `3-46` without a trailing newline, matching Perl 5.38 for this input.
Add `--release <inventory-id>` (before the path) to compile against a
specific checked release, for example `--release 1.000`.
