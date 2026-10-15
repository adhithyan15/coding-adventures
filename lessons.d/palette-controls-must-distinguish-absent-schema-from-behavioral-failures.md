---
category: Testing & coverage
---

# Palette controls must distinguish absent schema from behavioral failures

Tests committed before importing the Venn palette referenced the not-yet-present
theme_colors field. The actual compiler failures establish absent schema, not a
behavioral red. The same baseline also found a test fixture passing a recording
resolver to a helper specialized for FakeResolver; construct the generic options
explicitly, retaining the complete resolver trace. Preserve both failed logs and
obtain real assertions after type migration. Never count compile failures as
palette semantics, hosted acceptance or final-source Miri acceptance.
