---
package: */ct-compare
lane: *
rung: E1
status: open
---

# No ct-compare lane ran the shared CT01 corpus

`code/specs/fixtures/ct-compare-v1/` (21 cases, with an independent
validator) was written as the normative oracle for every `ct-compare` lane,
but no lane's tests reference it. Each lane tests itself with its own
hand-written cases, so the lanes drifted apart (error types, `eq_fixed`
meaning) without any test noticing.

**Rust: done** (VER26 step 4). `rust/ct-compare` runs all 21 cases on the L0
harness from the generated `cases.tfv`, and corpus edits select it through the
build tool's repository inputs. Adopting the corpus also forced CT01's
`CT_SELECT_LENGTH_MISMATCH` into the Rust API, which used to panic.

**Open for every other lane:** the package-parity loop's
`ct-compare-existing-lane-conformance-adoption` item. Lanes with a JSON reader
may read `cases.json`; the rest can read `cases.tfv`.
