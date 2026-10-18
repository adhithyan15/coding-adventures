---
category: Testing & coverage
---

# Distinguish an available node field from a token in its ancestry

The first CLOC41 full-green attempt passed the complete native folder tests,
then three new CLI tests failed. Two expected a direct property-token parent
even though the actual dot-member property field is None. The real property
token is present through the observed member's exact physical prefix. The
implementation correctly joined only available node fields; adding a made-up
property identity to satisfy the test would violate the chosen contract.

Clarify that boundary before correcting the oracle: require exactly three
immediate CLI parents and separately require the real property token through
the member ancestry. Keep native five-slot/mask tests for actual populated
property fields. The third failure was an incorrect diagnostic prefix: the
existing CLI stage says optimization pass. Preserve normal exit1, empty stdout,
the precise merge/derive checked cause and all four artifact sentinels.

The failed full receipt remains unaccepted. Run the corrected new CLI target
before the remaining full target list so a local assertion error fails early;
retain complete affected suites and strict checks for actual acceptance.
