### Fixed — lesson-budget backfills own their track expectations

- Replaced the corpus-wide exact-zero annotation snapshot with stable accounting
  invariants, so independent language backfills no longer serialize on one test.
- Added a reusable track-owned assertion that counts only schema-v2 lessons,
  requires all three declarations, pins declared-unit totals, rejects every
  over-budget lesson, and verifies track-prefixed ids.

