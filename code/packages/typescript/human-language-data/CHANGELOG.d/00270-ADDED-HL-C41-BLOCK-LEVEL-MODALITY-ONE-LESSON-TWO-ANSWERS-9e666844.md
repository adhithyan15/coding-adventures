### Added — HL-C41 block-level modality: one lesson, two answers

- Add the `writing` lesson-body block type (`## Writing: …`), for a section that
  teaches the **hand** to form a letter — as against `script`, which teaches the
  **eye** to recognise one. It is the first and so far only **detachable** block type:
  nothing later in a lesson depends on it, so a renderer that cannot use a hand may
  set it aside and still deliver a coherent lesson.
- Derive modality at two scales. `LessonModality.modality` is unchanged and still
  describes the whole lesson — what the **book** signs. New `coreModality` describes
  the lesson minus its detachable blocks — what a hands-free view can deliver. New
  `coreDerived`, `coreReasons`, `blocks` (per-block `BlockModality`) and
  `writingSegments` expose the derivation. New `deriveBlockModality`,
  `lessonCoreText`, `isDetachableBlock`, `DETACHABLE_BLOCK_TYPES`,
  `strongerModality`, `weakerModality`.
- **This is why it exists, and it is not what an earlier framing assumed.** The
  project owner's ruling is that the book is a standalone artifact and keeps all
  writing content in full; a dictation-friendly edition is a *separate output view*
  over the same canonical source, exactly as the narration export is. `coreModality`
  is the metadata that view reads. It is a strict improvement for that view: today a
  lesson with any pen content is lost to a commuter wholesale, whereas block marking
  lets them take the voice core and defer only the segment.
- Sight cues and tables are now attributed to the block they occur in, so a cue inside
  a writing segment does not follow it out into the core, while a cue in ordinary prose
  still does.
- An authored `modality:` override **caps** the core, giving the invariant a hands-free
  view relies on: `coreModality` is never stronger than `modality`.
- `drivablePrefix` and `drivablePercent` now count the core; `coreVoice` and
  `lessonsWithWritingSegments` are published beside the unchanged `voice`/`sight`/`pen`
  counts so the book's numbers and the hands-free numbers reconcile in the gap report.
- New report-only finding `modality-writing-segment-not-separable`: a lesson that is
  not `type: writing` may carry one writing segment; several means it should be split
  or declared a writing lesson. `type: writing` lessons are exempt.
- **Measured no-op.** No track has authored an interspersed writing segment yet, so
  every lesson's core equals its full modality and no published number moves — the
  regenerated `core/lesson-modality.json` is byte-identical in its summary (1,133
  lessons, 725 `voice`, 64% drivable). Pinned as `coreVoice === voice` alongside
  `lessonsWithWritingSegments === 0`, so the first interspersed lesson has to break the
  equality deliberately. Deliberately *not* pinned as an absolute literal here: the
  corpus totals live in one place, `modality-manifest.test.ts`, against the generated
  manifest.
- `features.blockModality` stays **false**: this change derives block modality but the
  manifest does not yet emit block rows, and the flag exists precisely so a consumer can
  tell those two states apart.
- Amends [`HL08`](../../../specs/HL08-modality-gentle-ramp-and-the-drivable-course.md),
  which had assumed one modality per lesson.

