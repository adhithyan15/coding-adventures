## Joining the cross-language verb corpus

Hindi already taught five verbs, but every one of them sat under a
**namespaced** `concept_tag` (`HI-VERB-BOLNA`, `HI-VERB-HONA`, …). Namespaced
ids are language-local, so Hindi's *bolnā* and Bengali's *bôlā* were unrelated
concepts and the track contributed **zero** verbs to the cross-language join
while eighteen other tracks were already realizing the canonical `VERB-*` set.
This retags four of them and rewires the realization path that follows.

- **Retagged**, metadata only — no lesson prose touched:
  `HI-C03-hun` (हूँ) `HI-VERB-HONA` → **`VERB-BE`**;
  `HI-C05-bolna` (बोलना) `HI-VERB-BOLNA` → **`VERB-SPEAK`**;
  `HI-C05-karna` (करना) `HI-VERB-KARNA` → **`VERB-DO-MAKE`**;
  `HI-C05-rahna` (रहना) `HI-VERB-RAHNA` → **`VERB-LIVE`**.
- **`HI-C04-milenge` keeps `HI-VERB-MILNA` deliberately.** `VERB-MEET` exists
  and the tag looks like an easy fifth, but the lesson does not teach "to
  meet" — it teaches the *future* **मिलेंगे** (*milenge*, "we will meet") as
  the second half of the farewell *phir milenge*. Filing it under `VERB-MEET`
  would have raised the coverage number by describing the lesson falsely, so
  it stays namespaced and the gap stays visible.
- **Chapter 5 now sits in the path at all.** `HI-C05-bolna`, `HI-C05-rahna`
  and `HI-C05-karna` were absent from `curriculum.json` entirely; they are now
  `HI-PATH-028` under `SPINE-SAY-WHAT-I-DO`, inserted between the Chapter 4
  segment and the Chapter 6 segment, in book order. No lesson moved relative
  to another and no chapter was reordered.
- **`HI-PATH-008` was split in place, not moved.** A canonical tag is owned by
  a spine node, so `VERB-BE` obliges `HI-C03-hun` to sit in a
  `SPINE-SAY-WHAT-I-DO` segment. Rather than relocate the lesson — which would
  have changed where a learner meets हूँ — the existing two-lesson segment was
  split at its seam: `HI-PATH-008` keeps `HI-C03-hun` (now
  `SPINE-SAY-WHAT-I-DO`, still carrying `HI-EXT-008-LANGUAGE-SPECIFIC`), and
  the new `HI-PATH-027` holds `HI-C03-thik` under `SPINE-CHECK-WELLBEING` at
  the very next position. The walked order of the path is byte-identical.
  This also matches every sibling track: all seventeen others that realize
  `VERB-BE` place that lesson directly in `SPINE-SAY-WHAT-I-DO`.
- **Ledgers rewired to match**: `SPINE-SAY-WHAT-I-DO.segments` `[]` →
  `["HI-PATH-008", "HI-PATH-028"]`; `VERB-BE`, `VERB-SPEAK`, `VERB-DO-MAKE`
  and `VERB-LIVE` dropped from that node's `omits` (42 → 38 concepts omitted);
  `SPINE-CHECK-WELLBEING.segments` retargeted `HI-PATH-008` → `HI-PATH-027`.
  `relocates` stays empty — no lesson carries a `spine_node` pin that
  disagrees with its placement.
- **Corpus effect**: Hindi covers 4 of the core 40 verbs (0% → 10%);
  `tracksWithNoCoreVerb` 4 → 3; `meanCoveredPercent` 13 → 14. Because
  `SPINE-SAY-WHAT-I-DO` is an A2 node, the Hindi track's `reach` becomes
  **A2** and four Hindi lessons are now levelled A2 (`pre-A1` 654 → 653
  corpus-wide, A2 122 → 126, ramp-to-A1 951 → 950). The corpus snapshot pins
  in `tests/levels.test.ts` and `tests/verbs.test.ts` are deliberately left
  failing rather than re-pinned here.

