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

**Partially addressed:** the Rust lane runs every corpus record through TF01's
L0 harness (VER26 step 4), with independently checked TFV transport and generated
fixed-width dispatch. The quirk remains open across the other lanes, owned by
the package-parity loop's `ct-compare-existing-lane-conformance-adoption` item.
