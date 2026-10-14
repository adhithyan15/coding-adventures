# CV04a — captured-array downstream failure evidence

The actual full18 run at28ac479b ended with Cargo101/helperEXIT1 on October14:
1857PASS,10FAIL,1existing ignore,59 completed targets. It stopped at LANG-AOT's
matrix; strict lint was not run. Original log SHA256:
5fd907b26eb268c9e7a1689f3dbf92295c0ac6bb91d17c7099f3740d24051f54.
The scoped128-case cluster control passed separately and is included in its
suite, not added. No earlier native receipt becomes acceptance of this source.

Nine failure records name NativeAOT BackendRefused(main), including the repeated
matrix failure: three-dimensional captured boolean/real/string arrays,
four-dimensional captured integer/string arrays, and four-dimensional forwarded
captured boolean/real/string arrays. The JVM proven-columns record reports
VerifyError, main()J Stack size too large, for the captured3D real-array source.
These are actual backend/verifier failures, not demonstrated undefined behavior
or a claim that disk exhaustion caused them. Preserve every original source,
expected Exit42, refusal/verifier message and failed receipt.

Before implementation, reproduce the individual failed cells and trace their
emitted IIR, specialized native instructions/ABI callbacks and JVM slot effects.
Distinguish missing operation/signature support from value-copy/captured-array
ownership or uninitialized memory. Derive any JVM stack fix from category-one/
category-two operand effects and branch merges, rather than increasing an
unproven constant. Inspect production bounds and actual generated artifacts.

A bounded repair must preserve full array shape, element types, lower bounds,
value-formal copies, captures/forwarding, output and intentional exit semantics.
Do not remove matrix rows, alter proven backend declarations, accept a missing
available toolchain, weaken capacity/safety assertions or bypass required CI.
Any applicable undefined-behavior evidence blocks delivery despite Miri deferral.

Add exact operation/signature or verifier boundary controls before production
changes, then run native failed cells, backend suites, downstream suites and
strict lint at the final source. Independent exact-head security review and
all hosted branch-protection gates remain required. Full implementation and
Closure/provenance delivery remain OPEN. Miri DEFERRED/PENDING; frozen497inputs,
run/source/target/receipts untouched. No production repair selected by this note.
