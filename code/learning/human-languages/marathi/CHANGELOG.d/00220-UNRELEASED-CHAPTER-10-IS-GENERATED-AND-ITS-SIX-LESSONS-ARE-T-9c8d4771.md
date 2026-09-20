## Unreleased — chapter 10 is generated, and its six lessons are typed (HL-C287)

`book/chapters/ch10-responding.tex` now generates from its six lessons. Two of
Marathi's four hand-written chapters remain (11 and 12).

- **Six schema-v1 lessons migrated**, eleven atoms between them, no lesson over
  the per-lesson budget of three. Four `unknown` headings re-pointed by
  prefixing: `The phrase, assembled` → `You'll want to know — …` (×3), plus the
  recap's `The whole exchange` → `The exchange` and `Atoms banked this chapter`
  → `What you've built — …`. Per-character non-ASCII census: **zero**
  Devanagari or IAST characters changed.
- **The spine node has to be the node of the segment that holds the lesson.**
  `MR-C03-mi` sits in `MR-PATH-007` (SPINE-EXCHANGE-NAMES) and
  `MR-C03-kaahi-harkat-nahi` in `MR-PATH-009` (SPINE-COURTESY-THANK), not the
  chapter's headline SPINE-CHECK-WELLBEING. Declaring the chapter's node for
  all six would have been wrong for two of them.
- **`MR-C03-practice` joins `MR-PATH-009` rather than getting a segment of its
  own.** A new path shard would have forced the canonical re-shard to renumber
  every following segment by ten — a 25-file rename for no reader-visible gain
  — and `curriculum-shards.test.ts` catches exactly that.
- **`MR-C03-mi` required `MR-LEX-BARAM-01` without declaring the
  prerequisite.** Its Grammar Lens reaches back to *baraṁ* by name and uses its
  gendered form, so the dependency was real and only the declaration was
  missing; `MR-C01-baram` is now a prerequisite.
- **Two stale chapter pointers, both HL-C102's own failure mode.**
  `MR-C03-mi-bara-aahe` said "the Chapter-1 *baraṁ*" in its recall while saying
  "Chapter 4" twice elsewhere in the same lesson — the lesson id `MR-C01-` had
  leaked into reader prose after a renumber. Named the thing instead. The
  `.tex` carried the same error.
- **One thing carried back from the `.tex`**: it dated the Perso-Arabic loans
  to the **Deccan sultanates**; the lesson said only "Persian-speaking rulers",
  which is true of several centuries. The anchor makes the claim checkable.

Counters, re-measured: lesson-content budget **188 → 194** (six lessons became
measurable, none added; idioms/senses/culture unchanged at 5 / 4 / 7);
`atomsTaught` **186 → 197**; `atomMeasurementBlindLessons` **19 → 13**;
`reinforcementWindowMisses` **267 → 293**, the usual legitimate rise as newly
declared atoms become newly measured ones. `atomChapterSpikes` does NOT move —
eleven atoms is inside the per-chapter budget of twelve. `atomsNeverRevisited`
does not move either: no atom this chapter mints is a dead end.

Verified: 124 test files / 1730 passing; all eleven `check:*` gates; the whole
Marathi book compiles under XeLaTeX with zero overfull boxes, and chapter 10's
pages were read as rendered PDF.

