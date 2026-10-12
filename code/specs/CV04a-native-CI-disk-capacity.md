# CV04a — bounded native CI disk-capacity repair

Status: implemented after the specification commit; local validation passes.
Final security review, publication and new required native CI remain pending. Follow-up within existing Closure
PR #17288 / foundation #16970 / provenance epic #15830. Full local Miri remains
**DEFERRED/PENDING**, never passed; its frozen checkout/run/inputs are preserved.

## Observed failure

Required pull-request run 38184271841 for published candidate
`f5389812a0469db06f4a7b8d09db48a63f12ce57` failed Linux job 114607733873.
GitHub's check annotation reports `System.IO.IOException: No space left on
device` while writing the hosted runner's diagnostic log. The affected-package
step never reported a terminal conclusion; no completed Closurec aggregate row
is available. No compiler test failure or undefined-behavior result is inferred.
The existing reclaim step reported 72 GiB available before later native setup
and package compilation. That does not identify which artifact consumed it.

A request to rerun just that failed job was rejected while the workflow still
contained running macOS and Windows jobs. Preserve all evidence and accept
neither this failed run nor a future queued retry as passing native validation.

## Bounded change

In both existing build-tool execution steps, configure only Cargo dev/test
artifact storage through step-local environment variables:

- `CARGO_PROFILE_DEV_DEBUG=0` and `CARGO_PROFILE_TEST_DEBUG=0`: omit debug
  information from those CI artifacts.
- `CARGO_INCREMENTAL=0`: omit incremental compilation caches on disposable
  runners.
- `CARGO_PROFILE_DEV_CODEGEN_UNITS=256` and
  `CARGO_PROFILE_TEST_CODEGEN_UNITS=256`: retain the normal development-profile
  codegen partition count when disabling incremental compilation.

Cargo documents debug information, debug assertions, overflow checking and
incremental compilation as separate profile settings. Only the listed settings
are changed. Do not set release mode, optimization, panic strategy,
debug-assertions, overflow-checks, RUSTFLAGS, test filters, test skips or weakened
limits. Keep required native/unsafe checks, package selection, step admission,
lint flags, build-plan authority, containment and timeout ceilings unchanged.
The variables apply to native jobs' package build steps, not local developer
defaults or the separately frozen Miri run. No crate dependency is added.

This mitigates artifact amplification; hosted-runner capacity acceptance still
requires a new exact-head Linux job to complete every selected package and
Closurec command. Do not claim a measured disk saving before measuring it.

## Validation and delivery

1. Commit this specification before workflow changes.
2. Parse the real workflow; prove only the five specified environment values
   were added to the two package steps. Compare all commands, conditions,
   permissions, timeouts, setup and security-gate steps to the prior workflow.
3. Exercise actual Cargo dev/test compilation with these settings in a separate
   target; verify debug assertions and overflow checks remain enabled and pin
   representative captured-range/provenance/compiler safety tests without skips.
   Record actual rustc arguments and artifact sizes; do not label them full CI.
4. Run affected workflow contract tests, build-tool tests/vet and unchanged
   production planning. Confirm the actual classifier is Rust-scoped rather
   than bypassing conservative classification or package selection.
5. Obtain independent security review on the final exact repair head before
   updating the same PR. Preserve earlier source validation at its actual heads
   with complete source/payload equivalence proofs; run affected checks again
   when source changes invalidate them.
6. Require new exact-head terminal acceptable CI on Linux/macOS/Windows, clean
   reviews/conflicts, full applicable integration and actual merge reachability
   plus payload identities before squash auto-merge. Unrelated TaskApp helper
   defect #17313 stays visible; a narrow security review does not close it.

Full Closure parity, transformations/output-byte lineage and CV04b remain open.
After implementation delivery, obtain genuine final-source Miri acceptance.

References: [Cargo profiles](https://doc.rust-lang.org/cargo/reference/profiles.html)
and [Cargo environment variables](https://doc.rust-lang.org/cargo/reference/environment-variables.html).
