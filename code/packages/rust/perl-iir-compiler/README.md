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

## Release profiles (LANG84 4c)

Releases whose trees are identical can still mean different things. The
one difference this subset meets is the negative zero: Perl 1.000 through
5.004_05 compute `+`, `-`, `*` and unary minus in C doubles, so `-0` and
`0 * -3` are `-0.0`, which those releases print as `0` or `-0` depending on
constant folding and the platform's `printf`. `src/profiles.rs` names two
profiles and maps every checked release to one by exact ID:

| Profile | Releases | Arithmetic | A printed negative zero |
|---|---|---|---|
| `perl_nv` | 1.000 – 5.004_05 (114 releases) | `Arithmetic::Double` | refused, `Unsupported` |
| `perl_iv` | 5.38.0, 5.38.2, 5.44.0, 5.45.3 | `Arithmetic::Integer` | prints `0` |

The compiler tracks every value's zero sign by IEEE rules whatever the
profile, so the IIR never depends on it; the profile only decides whether a
`print` of a negative zero is accepted. The pinned upstream sources (tag,
commit, file and line) are in the module docs of `profiles.rs`.

```rust
use perl_iir_compiler::{compile_source_at, FrontendBudgets, FrontendError};

let budgets = FrontendBudgets::default();
assert!(compile_source_at("print(-0);", Some("5.38.0"), "z", &budgets).is_ok());
assert!(matches!(
    compile_source_at("print(-0);", Some("1.000"), "z", &budgets),
    Err(FrontendError::Unsupported { .. })
));
```

## Stage I and the release sweep (LANG84 4c)

The `pl-int-print` corpus lives in `code/grammars/perl/corpus/pl-int-print/`
(one `.pl` per case, plus `.expect` or `.reject`, a `.cite` oracle record,
and `.reject@perl_nv` where that profile refuses a case).

- `tests/release_subsets.rs` proves stage I for every release whose
  `iir_status` lists `pl-int-print`: each case compiles, validates, passes
  LANG83 C7 and prints its `.expect` on vm-core, or is refused with the
  category it names. It also checks that `releases.csv` and `profiles.rs`
  agree, and (with a host `perl` v5.38.x) that the host prints every
  `.expect`.
- `tests/release_sweep.rs` compiles every case under all 118 checked
  releases, groups the releases into IIR-identity classes (same IIR digest
  or rejection for every case), and compares the result with
  `code/grammars/perl/release_classes.csv`. There are three classes, whose
  representatives `1.000`, `5.003_26` and `5.38.0` are the releases
  `lang-aot`'s `lang_matrix` runs on every backend. After a deliberate change,
  regenerate with
  `PERL_RELEASE_SWEEP_WRITE=1 cargo test -p perl-iir-compiler --test release_sweep`.
  That run writes the file and then fails on purpose (and refuses to run
  under `CI` or `GITHUB_ACTIONS`); review the diff and run the test again
  without the variable.

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
