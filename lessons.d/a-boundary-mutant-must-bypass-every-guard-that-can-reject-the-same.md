---
category: Testing & coverage
---

# A boundary mutant must bypass every guard that can reject the same input

The first over-limit key mutant changed the public `setting` size check, but
`fromEntries` and the private builder still rejected that key. The public
boundary test therefore passed even though the mutation had removed no
observable protection. A mutation is useful only when the changed input can
reach different public behavior. For a repeated guard, remove or bypass every
copy needed to exercise the intended defect, or choose an input and entry point
that makes the guard under test decisive. Confirm the mutated source differs
and the named public assertion fails.
