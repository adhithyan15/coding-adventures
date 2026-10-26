---
category: Cryptography & security review
---

# A monotonic version comparison cannot enforce an advisory with per-major patched thresholds

The first regression for GHSA-jr5w-w6x2-84p9 compared every Electron version
against `43.8.1`. That guarded the current 43.x lock but silently accepted
`44.0.0` and `44.9.0`, even though the advisory's 44.x patched threshold is
`44.9.1`. SemVer ordering and advisory safety are different relations when an
advisory publishes one vulnerable range per maintained major.

Encode the advisory's ranges directly, or fail closed to the one major that was
actually reviewed. Add adversarial examples from the next major to the test so
a future upgrade cannot cross the boundary merely because its number sorts
higher. A package-manager audit is useful corroboration, but the checked-in
regression still needs to represent the advisory correctly.
