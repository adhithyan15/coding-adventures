---
category: Testing & coverage
---

# Assessment artifact presence does not imply a per-level mock test owner exists

The assessment-artifact report showed Hindi with every declared artifact built,
but I inferred that a level-owned `tests/mocks/hindi/b2.test.ts` therefore
existed. Hindi still owns mock checks differently, so the assumed path aborted
the comparator-read command. Discover both the artifact and test paths with
`rg --files` first; use an actually level-owned Malayalam or Bengali B2 suite
when designing a new Sanskrit level owner.
