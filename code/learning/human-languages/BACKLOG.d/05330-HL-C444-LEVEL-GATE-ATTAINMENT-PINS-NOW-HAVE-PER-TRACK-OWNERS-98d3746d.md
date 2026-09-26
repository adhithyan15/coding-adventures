## HL-C444-c12f6d6d — Level-gate attainment pins now have per-track owners

**Status: CLOSED — implemented by #16082.** The fresh post-#15743 contention
audit counted only paths still tracked on current main and only commits since
2026-09-25. `tests/level-gate.test.ts` led the remaining cross-track authoring
surfaces at 20 touches: every track that climbed a rung edited the same `HELD`
map, global attained count, and rendered roster assertion.

The exact verdict now lives in one strict JSON owner per track, including a
`null` owner for a track that has not closed pre-A1. Missing, extra, unsafe,
case-fold-colliding, and symlink owners fail. Global level counts and rendered
rosters are derived from those owners, so a track climbing a rung changes one
small file and keeps the exact anti-spurious-level gate.

The reprioritized structural queue before broad #13295 content fan-out is:

1. split `letter-anchoring.test.ts`'s per-track ratchets (15 recent touches);
2. remove the four-file filmstrip aggregate edit set (10 touches each):
   `generated-figure-hashes.json`, `filmstrip-geometry.json`,
   `figure-targets.test.ts`, and `integration.test.ts`.

Older apparent leaders were excluded after checking present ownership:
exam-inventory pins were sharded by #15662, gentle-ramp snapshots were retired
by #15671, and curriculum membership digests were split per track by HL-C442.
