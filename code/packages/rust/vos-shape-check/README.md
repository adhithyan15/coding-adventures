# vos-shape-check

The lexical **component shape checker** of the verified-OS program
(backlog VOS-D002, spec `code/specs/VOS03-component-shape-checker.md`).

## What it is

A verified-OS *component* is a small, total, bounded, by-value step function
whose contract is proved on the compiled RV32 image. That proof only works if the
source stays inside a narrow shape: `#![no_std]`, no `unsafe`, no `static`, no
`dyn` or closures, no recursion, one loop form (`for i in 0..K`), no division, no
panic path, fixed-width types, no floats, only `core` and a contract crate as
dependencies, no macros or build scripts the analyzer cannot see. VOS01 section
1.4 names these rules SH1-SH15; VOS02 adds SH17 and SH18 for the *ghost face*, and VOS03 SH19 (the command-type check of a contract crate)
(the evidence-only `mod ghost`). Existing analyzers (VER08a) check little of this.

`vos-shape-check` tokenises a crate's `src/**/*.rs` and `Cargo.toml` and reports
every occurrence a lexer can see, per rule, with file and line. It is **lexical**:
a pass means "no occurrence the lexer can see". What it cannot see (macro
expansion, name resolution, callees in other crates, bare `+ - *`) is listed rule
by rule in VOS03 section 6 and is the job of the image-level companions
(VOS-026, VOS-028). It reads Rust with the repository's **grammar-driven lexer** on the RS01 lexical
pair for the crate's edition, and TOML (`Cargo.toml`, `vos.toml`) with the
repository's TOML lexer and parser: no hand-written lexer or parser. The
structural layer over the tokens (headers, impl types, the call graph, the ghost
shapes) is heuristic and **fails closed**: a scan that gives up is a `VT03` finding,
never a skipped item. See VOS03 section 0 for the architecture and the one-lexer
per run design.

## Where it fits

```text
 contract crate  ->  component crate  ->  vos-shape-check  (this)  ->  vos-link
 (VOS01/VOS02)       (class K, vos.toml)   SH1-SH15, SH17-SH19        profile/lock
```

The crate class (K, C, S, M, P, H; VOS01 section 1.6) and kind come from the
crate's `vos.toml`; a crate with no tag is class K. The central evidence-feature
list defaults to `ghost`, `cov` and is overridden by `vos-link`.

## Usage

```text
vos-shape-check <crate-dir> [--evidence-features ghost,cov]
```

```text
vos-shape-check: crate vos-ready-bitmap, class K (vos.toml), kind production
SH7 src/lib.rs:12 division operator `/`; RV32I has no divide (use shifts and masks)
result: FAIL, 1 findings
```

Exit codes: `0` PASS, `1` FAIL (findings, including unlexable input), `2` no
verdict (usage error, or a symlink, non-regular file, oversize or non-UTF-8 file).
The report is sorted and deterministic.

As a library (pure, no I/O):

```rust
use vos_shape_check::{check_crate, CrateInput, Policy};
# let input = CrateInput::default();
let report = check_crate(&input, &Policy::default());
println!("{}", report.render());
```

## SH19 and the contract crate's attributes (VOS-D112)

For `kind = "contract"` crates SH19 also holds every attribute to a closed list (derive of the bare core traits,
`repr(u8|C|transparent)`, `no_std`, the lint attributes, `doc`, `must_use`, `inline`, `default`, exactly
`cfg(test)`), so a path attribute or a foreign derive cannot rewrite a command enum after it was judged.  The
dependency half is `vos-link check-component` CC10.

## Tests

`cargo test -p vos-shape-check` (run from `code/packages/rust`): a clean and a
violating case per rule, the VOS01 section 5 toy crates (which must pass) and
broken variants (which must fail) under `tests/fixtures/`, CLI exit codes and
filesystem refusals. Fixture manifests are stored as `Cargo.toml.fixture` so no
nested Cargo root exists under the package. Per-item security regression tests live in `tests/security.rs`. Line coverage is reported by llvm-cov (see the changelog).

## The step counter and the tick audit

The linearity tests (`scales_linearly`, n against 4n against 16n) do not read a clock.  They read a
deterministic step counter (`src/steps.rs`): a scanning loop calls `steps::tick()` once per iteration, and a rescan
that turns quadratic shows up as a ratio near 16 instead of 4 for a 4x input.  A loop that does not tick is
invisible to that counter, so VOS03 section 7.1 (VOS-D089) makes it a rule:

```text
every while / loop / for in src/**/*.rs (outside #[cfg(test)]) either
  - calls tick() in its own body, or
  - has the line  // bounded: <reason>  directly above it
```

`tests/tick_audit.rs` enforces the rule on this crate's own source.  It finds loops with the same grammar-driven
lexer and `FileCtx` the checker uses (no second scanner), and a failure names every `file:line` that breaks it.
Negative controls pin what counts: an inner loop's tick does not excuse the outer loop, and `impl X for Y` and
`for<'a>` are not loops.  When you add a scanner loop, tick it; use `bounded:` only when the trip count is a
constant or one step of an already-ticked outer loop.  Iterator chains and recursion are outside the audit.

## Dependencies

Only in-repo crates: `grammar-tools`, `lexer`, `parser`, `coding-adventures-toml-lexer`
and `coding-adventures-toml-parser` (and, through them, `regex`). No dependency on
`ca-capability-analyzer`. The RS01 token grammar is embedded at compile time
(`include_str!` of `code/grammars/rust/rust1.97.0-e{2021,2024}.tokens`).
