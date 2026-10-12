---
category: Compiler / VM / language pipeline
---

# An assignment proof must survive the alias state produced by lowering

The ALGOL runtime-real assignment proof was computed before its RHS lowered.
A conditional selector could then pass a branch-root local to a default
by-name formal, promoting the local to shared storage. The cached proof still
marked the destination safe for real output, violating the unaliased-root
contract. Ordinary suites passed while a direct native-library probe admitted
the prohibited case. The exact expect-refusal regression failed before repair;
its value-formal control passed.

Retain the original proof and require it to remain valid in the post-lowering
alias state before publishing destination provenance. This also prevents a
newly qualifying post-state from laundering an originally unproven expression.
Test both root positions, both value-selector arms, actual shared-storage
absence and exact backend output. Admission alone does not demonstrate wrong
numeric output or undefined behavior. Preserve original rejected receipts and
separate ordinary green runs from independent final acceptance.
