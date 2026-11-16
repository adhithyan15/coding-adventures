---
category: CI & GitHub Actions
---

# A .warrant file added outside warrant-grammar's dependency graph is never parsed by CI until some later PR rebuilds warrant-grammar

**What went wrong.** PR #19463 added `code/specs/arcade-sfx.warrant` with `/` for division (Warrant has no `/`; division is `div(a, b)` returning `Option<Int>`, floor semantics). PR #19461 did the same in `audio-mixer-realtime.warrant`. Neither PR touched `warrant-grammar`, whose test `tests/v0_compat.rs::every_warrant_file_in_the_repository_still_checks_and_the_v0_ones_stay_v0` is the only thing that parses and checks every `.warrant` file under `code/specs`. The build tool only runs a package's tests when that package or one of its dependencies changed, and a spec file depends on nothing, so the breakage merged green and surfaced only when an unrelated PR (#19492, #19369) rebuilt `warrant-grammar`. A sweep of `code/specs` at that point found more invalid documents from the same batch (float `F64` types, other parse errors), so the gap had been open for several PRs.

**Fix.** Rewrite `/` as `unwrap_or(div(a, b), 0)` (the divisor is a non-zero constant, so the default is never used). Rust's `>> 16` on signed integers is an arithmetic shift, which is floor division, so `div` matches the implementation exactly, negatives included; `/` on signed ints would truncate and need a sign correction.

**Do differently.** Any `.warrant` change must be parsed and checked by CI in the PR that makes it. Proposed guard (not implemented here), either of:
1. A build-tool trigger that maps any changed `*.warrant` file to `warrant-grammar`, so `v0_compat` runs on every PR that adds or edits a document.
2. A cheap repo-wide parse-and-check step (a tiny binary calling `parse_document` and `check_warrant` over `code/specs/*.warrant` and package-local `.warrant` files) in the metadata job, which runs on every PR regardless of the affected set.
Option 2 is faster to fail and independent of build-tool graph rules; option 1 is the smaller change. Until one lands, run `cargo test --test v0_compat` in `code/packages/rust/warrant-grammar` before pushing any `.warrant` file.
