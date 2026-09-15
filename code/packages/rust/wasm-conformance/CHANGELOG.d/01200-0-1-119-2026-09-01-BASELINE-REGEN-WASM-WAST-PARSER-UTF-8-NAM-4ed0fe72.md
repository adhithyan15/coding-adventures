## 0.1.119 — 2026-09-01 — baseline regen: `wasm-wast-parser` UTF-8 name validation closes `utf8-invalid-encoding.wast` (0/176 → 176/176)

Regenerated `tests/fixtures/testsuite-status.json` (`--write-baseline`)
after `wasm-wast-parser` 0.1.98 fixed its `String::from_utf8_lossy` name-
string bug — see that crate's own CHANGELOG for the full root-cause
writeup and security rationale. No code changes in this crate itself;
`grade_assert_malformed`'s `ModuleSource::Quote` arm already had exactly
the right shape for this (`wasm_wast_parser::parse_module` returning
`Err` on a malformed quote payload was always graded `Pass`) — it was
purely waiting on the parser it calls to actually reject the input.

Programmatic per-file diff against the pre-fix baseline (Python,
comparing the `files` dict keyed by filename), across all 257 corpus
files: exactly 1 file changed —

| File | Directive kind | Before | After |
|---|---|---|---|
| `utf8-invalid-encoding.wast` | `assert_malformed` | 0/176 (176 not-yet-supported) | 176/176, 0 fail, 0 NYS |

Aggregate: `assert_malformed` 1313→1489 pass, not-yet-supported 627→451
(both deltas exactly 176). Every other directive kind and every other of
the 257 files — including the other three `utf8-*.wast` siblings
(`utf8-custom-section-id.wast`, `utf8-import-field.wast`, `utf8-import-
module.wast`, already 176/176 before this change via a different code
path) — is byte-for-byte unchanged from the 0.1.118 baseline, confirming
the fix is exactly as scoped as intended: stricter UTF-8 validation in
the WAT text front-end did not newly reject any currently-passing module
anywhere in the vendored testsuite.

