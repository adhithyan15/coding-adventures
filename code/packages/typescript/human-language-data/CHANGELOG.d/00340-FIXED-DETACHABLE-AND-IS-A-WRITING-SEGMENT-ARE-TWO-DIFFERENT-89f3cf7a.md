### Fixed — "detachable" and "is a writing segment" are two different things

- `DETACHABLE_BLOCK_TYPES` gains `script`, so a hands-free renderer may set aside the
  inline-letters section. HL00 makes it optional scaffolding by design — "a reader who
  already knows the script skims that section" — and nothing later in the lesson depends
  on having read it.
- **This required separating two ideas the model had merged.** `writingSegments` was
  computed as `blocks.filter((block) => block.detachable)` — named for writing, filtered on
  detachability. That was harmless only while `writing` was the sole detachable type. The
  moment a second type joined, every inline-letters section counted as a writing segment,
  which set `hasWritingBlock` and dragged the lesson to `pen`: **`pen` 53 → 309, and 276
  reported "writing segments" that teach no writing at all.** Detachability is about what a
  renderer may skip; pen-ness is about what the learner's hand must do.
- `writingSegments` now filters on `block.type === "writing"`, and a new
  `detachableSegments` carries what a hands-free view sets aside — a superset.
- **Result: the book stays honest and the driver gets more.** Whole-lesson modality is
  unchanged (`voice` 726, `sight` 355, `pen` 53) because the printed book really does show
  glyphs; the core — what the driving edition reads — is **972 lessons, 86%**, above even
  the 84% that stood before the inline-letters section was classified honestly.
- `drivablePercent` is derived from `coreVoice` and now legitimately differs from
  `voice / totalLessons`. The invariant test was updated to assert the correct relationship
  rather than the coincidence that held while core and whole were always equal, and gained
  two more: the whole-lesson partition still closes, and `coreVoice >= voice` always
  (detaching can only help).
- A chapter whose only obstacle was a script section is no longer blocked; the gap
  report's blocked-chapter fixture was moved to a four-column paradigm, which the
  lineariser genuinely refuses, so the test still proves a real blocker gets named.
- **Next slice:** the manifest still publishes the conservative whole-lesson figure (64%)
  while the gap report publishes the core (86%). `coreModality` is the additive key HL-C44
  reserved for exactly this; emitting it and flipping `features.blockModality` closes the
  gap.

