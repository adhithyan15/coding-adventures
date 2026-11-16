---
category: Cryptography & security review
---

# Security composition consumes stronger proof before adding orthogonal evidence

When one proof establishes a signed reviewed path and another establishes
complete revocation, do not place both behind one broad marker merely because
they expose the same destination fields. The revocation type may be generic
over an older or weaker path source, so relabeling it as signed-path evidence
silently weakens the boundary.

Consume the already-constructed signed-path composition first, then add the
revocation proof under its own sealed exact-type marker. The resulting type
retains both complete inputs, and callers cannot substitute orthogonal
revocation evidence for the stronger path proof.

Do not infer covered path depth from a generic wrapper named “complete.” A
nominal direct or nested revocation branch can retain an accumulated server
path of arbitrary depth even when its revocation checks cover only the target
and one intermediate. Project the processed-intermediate count from the exact
retained signed path and require it to match the concrete branch's coverage
before composing the proofs.
