## [0.2.1] - 2026-11-24

### Security

- PIF-003b security review: the report says which sign-off parts were compared (`signoff.goals`, `signoff.aggregate`) and prints `goals_sha256_verified` (true only when a sign-off supplied an aggregate to compare with); the goals list and the sign-off comparison use hash lookups (linear time) instead of scans per claim; a goals list is capped at 100,000 lines (`too many goal lines`).
