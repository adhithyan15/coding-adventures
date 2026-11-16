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
1.4 names these rules SH1-SH15; VOS02 adds SH17 and SH18 for the *ghost face*
(the evidence-only `mod ghost`). Existing analyzers (VER08a) check little of this.

`vos-shape-check` tokenises a crate's `src/**/*.rs` and `Cargo.toml` and reports
every occurrence a lexer can see, per rule, with file and line. It is **lexical**:
a pass means "no occurrence the lexer can see". What it cannot see (macro
expansion, name resolution, callees in other crates, bare `+ - *`) is listed rule
by rule in VOS03 section 6 and is the job of the image-level companions
(VOS-026, VOS-028). It carries its own lexer because VER08a exposes no token API
and `ca-capability-analyzer` must not be modified (VOS03 section 0).

## Where it fits

```text
 contract crate  ->  component crate  ->  vos-shape-check  (this)  ->  vos-link
 (VOS01/VOS02)       (class K, vos.toml)   SH1-SH15, SH17, SH18        profile/lock
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

## Tests

`cargo test -p vos-shape-check` (run from `code/packages/rust`): a clean and a
violating case per rule, the VOS01 section 5 toy crates (which must pass) and
broken variants (which must fail) under `tests/fixtures/`, CLI exit codes and
filesystem refusals. Fixture manifests are stored as `Cargo.toml.fixture` so no
nested Cargo root exists under the package. Per-item security regression tests live in `tests/security.rs`. Line coverage 98% (llvm-cov).

## Zero dependencies

No third-party crates and no dependency on `ca-capability-analyzer`.
