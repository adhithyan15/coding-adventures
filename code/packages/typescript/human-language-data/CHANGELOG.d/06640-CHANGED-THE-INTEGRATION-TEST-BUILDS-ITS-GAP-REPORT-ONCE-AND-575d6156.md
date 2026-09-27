### Changed — the integration test builds its gap report once and shares it with the track evidence

`tests/integration.test.ts` now builds the gap report over `{ registry, lessons, books }` once, at import. It passes the report to every track evidence module in their shared context, and the migration-baseline test reads the same report. The field is `curriculumGapReport`: #16102 landed the same shared-report contract on main under that name, and the merge kept main's naming.

Before this change the Japanese, Persian–Urdu and Spanish evidence modules each built that report themselves, and all three ran inside the single "keeps track-specific integration evidence independently owned" test. That test timed out at 30s on CI (Persian A1 PR). No assertion or timeout changes. The test now runs in well under a second locally.
