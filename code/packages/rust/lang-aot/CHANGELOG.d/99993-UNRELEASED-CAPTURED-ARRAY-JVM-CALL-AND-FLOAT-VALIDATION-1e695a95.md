## [Unreleased] — captured-array JVM call and float validation

- The existing eight captured/forwarded array rows execute on the installed real JVM with Exit42. A new float-array allocation/store/call/load/return control computes42 and its descriptor-only [F to [I negative control is rejected by the verifier. A real JVM table checks84 finite/signed-zero/NaN comparisons across both widths and all six relations. Original rows, expected results and backend rosters are retained.

Actual c5b00f83 validation completed: full JVM200 tests/docs across3 targets
(69 library,124 integration,7 docs), the original8 real JVM rows, positive/
negative float-array verification, real IEEE84 checks and strictJVM/LANG/Twig
alltargets passed. Focused controls are included in200, not additive. Earlier
source da45489b unfiltered LANG validation actually failed:463PASS9 native ABI
refusals. Latest full/native/hosted/merge acceptance remains open, along with
general-hop32/32 work failures. Miri **DEFERRED/PENDING**, never passed; frozen
run/source/inputs/target/receipts preserved and unpolled. No new crates.io deps.
