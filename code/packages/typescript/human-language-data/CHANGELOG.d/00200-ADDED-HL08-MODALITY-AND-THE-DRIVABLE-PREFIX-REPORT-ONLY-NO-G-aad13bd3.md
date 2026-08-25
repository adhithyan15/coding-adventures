### Added — HL08 modality and the drivable prefix (report only, no gates)

- Add `src/modality.ts`: a pure module deriving each lesson's required channel
  (`voice` / `sight` / `pen`) and each chapter's **drivable prefix** — how many of
  its lessons, in authored `sequence` order, are learnable by ear before the first
  that is not. Implements the first migration step of
  [`HL08`](../../../specs/HL08-modality-gentle-ramp-and-the-drivable-course.md).
- **Modality is derived from lesson type and block structure, never from `skills:`.**
  `skills` records what a lesson *develops*, not what it *requires*: 501 of the 531
  schema-v2 lessons declare `[listening, speaking, reading]`, yet *hola* is
  perfectly learnable by ear. Deriving from `skills` would have stamped roughly 95%
  of the corpus "needs eyes" and made the drivable course an empty promise. The
  rules are: `type: writing` → `pen`; otherwise a `script` block, a sight cue, or a
  table wider than the configured linearisable width → `sight`; otherwise `voice`.
- Modality is monotonic — `pen` implies `sight` — exposed as `requiredChannels()`
  and `unionModalities()`, and a chapter's modality is the union of its lessons'.
- `maxLinearisableTableColumns` defaulted to **0** in this slice: until HL08's
  narration exporter could linearise a two-column table into speech, no table was
  speakable, and claiming otherwise would let a learner silently miss content they
  were never told they had missed. (Superseded above: HL-C16 built the lineariser and
  the default is now 3.)
- Support an authored `modality:` frontmatter override. An override that
  *contradicts* the derivation requires a `modality_reason:`; unexplained overrides
  (`modality-unexplained-override`) and unrecognised values (`modality-unknown-value`,
  which falls back to the derivation) are collected across the whole corpus and
  reported once. Nothing throws, and nothing gates — the HL-V01 precedent.
- Add a modality section to `buildCurriculumGapReport()` and its text renderer:
  per-track `voice`/`sight`/`pen` counts, each chapter's drivable prefix, the
  chapters that cannot be started by ear at all, and the corpus-wide drivable
  percentage. New summary fields: `drivableLessons`, `drivablePercent`,
  `chaptersWithoutDrivablePrefix`, `unexplainedModalityOverrides`.
- Measured over all 1,096 lessons: **51 `pen`**, **7** carrying a `script` block,
  and among the remaining 1,038, **322 carry a Markdown table** — the single largest
  obstacle to a hands-free course, and far more tractable than the script.
  **694 lessons (63%) are drivable exactly as authored.** Track extremes: Bengali
  and Persian at 90%, Russian at 9%.
- `tests/modality.test.ts` covers every derivation branch, monotonicity, the
  override-plus-reason rule, drivable-prefix computation (including a chapter whose
  prefix is 0), and pins the corpus-wide drivable count as a regression. The pin
  exists because a parser change that renamed a block's `markdown` field would make
  every lesson scan clean and silently report a 100%-drivable curriculum.
- Divergence from HL08's recorded baseline, stated rather than tuned away: the spec
  reports 56 cue-bearing lessons and 695 drivable. The published `SIGHT_CUES` list
  matches 61 lessons and lands on 694. Every structural count reproduces exactly
  (51 / 7 / 1,038 / 322), so the gap is entirely in the cue list, whose exact
  contents the spec never recorded. The detector was left alone.

