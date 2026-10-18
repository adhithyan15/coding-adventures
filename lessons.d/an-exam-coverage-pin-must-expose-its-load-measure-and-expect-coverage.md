---
category: Testing & coverage
---

# An exam coverage pin must expose its load measure and expect coverage in one test block

The Sanskrit A1 census was refactored to compute coverage once at module scope
and pass it to an assertion helper. Its focused corpus test passed, but
`exam-inventory-census.test.ts` reported Sanskrit as unpinned. That meta-test
intentionally scans each top-level `it(...)` block for all three operations:
`loadExamInventory(...)`, `measureExamCoverage(...)`, and a direct
`expect(coverage...)` or `formatExamCoverage(...)` read. Module-scope work and a
custom helper are invisible because accepting them would let a computed but
never asserted report count as a pin.

Keep expensive corpus parsing at module scope, but bind the inventory and run
the cheap coverage measurement inside the owning test block. Include one direct
`expect(coverage...)` before delegating richer comparisons to a helper. Run
`tests/exam-inventory-census.test.ts` whenever a coverage pin changes shape; a
language-focused suite cannot verify this repository-wide ownership contract.
