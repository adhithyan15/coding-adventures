---
category: Testing & coverage
---

# A mutation control must differ from its fixture baseline

CLOC35's first real private run passed seven controls but its range-forgery test
accepted the fixture. The mutation wrote start.byte_offset="0" for a.b, whose
real source already starts at0. No malformed range had been introduced.

Assert the replacement field differs from the actual fixture baseline, then
write the genuinely different value and keep the original rejection expectation.
Preserve the failed receipt and distinguish this test correction from a source
validation repair. Copying a mutant from a later-offset array fixture is unsafe
when the new member fixture starts at byte zero.
