### Fixed — a reading lesson needs sight, as a writing lesson needs a pen

- **The rule.** `deriveLessonModality` now derives `sight` for every lesson
  whose realization type is `reading` (frontmatter `type: reading`), at both
  scales: the full modality the book signs and the core a hands-free view
  delivers. It mirrors rule 1 (`type: writing` → `pen`): the lesson type
  decides and the body is not consulted. New `ModalityReasonCode`
  `"reading-type"`, recorded in `reasons` and `coreReasons` and accepted by the
  modality owner validator (`modality-shards.ts`).
- **Why it was needed.** A reading lesson is printed target-language text (a
  connected passage under `## Reading`, or a run of signs, labels or a word
  map) plus the instruction to read it. Every word of that is speakable, so
  the structural sight detectors (script block, anchored cue, unspeakable
  table) could not see it: 59 of the corpus's 74 reading lessons had a `voice`
  core and were offered to the driving edition (58 of them `voice` outright).
- **The type, not the skill.** The module still ignores `skills: [reading]`,
  which some 29,000 lessons list because it is what they develop. `type:
  reading` (74 lessons) describes what the lesson is. The module comment
  records the argument beside the existing one against deriving from
  `skills`.
- **No core to rescue.** The text is the whole lesson, so nothing is
  detachable and the core is `sight` too. `sight`, not `pen`: a reading lesson
  that also carried a writing block would be `pen` in full and `sight` at its
  core. None does today.
- **Authored overrides.** Unchanged rules: an authored `modality: voice` on a
  reading lesson is honoured, contradicts the derivation, and so needs a
  `modality_reason:` or it is reported as `modality-unexplained-override` (the
  corpus test pins zero findings). No reading lesson authors `modality:`.
- **Corpus effect.** 59 lessons stop being drivable; core voice 27,656 →
  27,597 of 29,551, corpus drivable 94% → 93%; full `voice` 27,078 → 27,020,
  `sight` 1,011 → 1,069, `pen` unchanged. By track (core flips): Arabic 3,
  Bengali 2, Chinese 2, French 3, German 3, Gujarati 3, Hindi 2, Italian 2,
  Japanese 4, Kannada 3, Latin 5, Malayalam 3, Marathi 2, Marwadi 2, Persian
  2, Portuguese 3, Punjabi 3, Russian 1, Sanskrit 1, Spanish 3, Tamil 3,
  Telugu 2, Urdu 2. Track percentages move by at most one point (Latin 99 →
  98, Punjabi 77 → 76, Sanskrit 94 → 93, Spanish 94 → 93). Twelve chapters
  lose drivable prefix: Spanish 266 (5 of 5 → 2), Japanese 12 (20 of 20 →
  19), Italian 36 (2 of 3 → 0), Tamil 84 (3 of 7 → 0), Malayalam 69 (3 of 4
  → 0), and Arabic 42, French 45, German 52, Gujarati 44, Kannada 76, Latin 58
  and Portuguese 29 (3 of 3 → 0 each). The Tamil and Malayalam before-counts
  are what the narration header said, which counts a lesson's full modality;
  the modality summary counts the core and had both at 4, because the fourth
  lesson in each (TA-R84-letters-that-look-alike, ML-R69-script-recall-first-
  nine) is `sight` in full but `voice` at its core. The narration header and
  the modality summary measure different things there, before and after this
  change. That split predates this change and is left alone. In the other eleven tracks the reading
  lessons sat behind an earlier blocker, so no prefix moves.
- **Narration.** `noticeNeeds` names the new cause ("your eyes, to read the
  printed text yourself"). The notice follows the full modality, so the 58
  reading lessons that were `voice` now open with the eyes notice, and the 16
  already-`sight` ones (PA-W01-haan-assemble's core was the 59th drivable one)
  add the reading cause to theirs. The
  chapter headers of the affected narration scripts restate their drivable
  prefix. Regenerated: `core/lesson-modality` (74 owners), narration for the
  23 tracks and its hash shards. No lesson file changes; book hashes are
  unchanged.
- **Tests.** Four derivation cases (both scales against a voice control, the
  skill ignored, reading plus a writing block, the override with and without a
  reason), one narration notice case, and `reading-type` added to the known
  causes of a corpus `sight` lesson. HL08 and the README record the rule.
- **Review follow-up: the reading notice.** Three corrections to the notice
  the first pass produced, all in `narration.ts`:
  - *Eyes once.* The 16 reading lessons that also tripped another eye rule
    said "You will want your eyes, to read the printed text yourself and your
    eyes, because the lesson points at something written down" (telugu ch83,
    hindi ch85, italian ch36, punjabi ch45, marwadi ch40 …). For a reading
    lesson `noticeNeeds` now names the eyes once: a sight cue is dropped (it
    points at the text being read) and a script block or unspeakable table is
    folded into the same phrase ("… and for letter shapes on the page", for
    PA-W01-haan-assemble). Lessons of every other type are unchanged.
  - *Save the whole lesson.* The notice offered "You can listen to everything
    else now — leave the section called Reading until you have stopped", which
    is the split the modality rule itself says a reading lesson does not have.
    All 74 reading lessons now say "this one needs your eyes, so it is not a
    driving lesson. … The questions are about what you read, so save the whole
    lesson for when you have stopped." (`waitUntilStopped` is still computed.)
  - *Two template slips, fixed everywhere.* "leave the sections called A and B
    … when we reach it" now says "them" for two or more sections (822 lessons
    across all 23 tracks), and a chapter header with a one-lesson prefix says
    "You can do the first one in the car" instead of "the first 1 of them"
    (179 chapters). The book's "Hands-free start: first 1 of 2 lessons" is a
    different template and is unchanged.
  - Tests: the reading notice's full text, eyes-once (cue dropped, script
    block folded, other types unchanged), sections/them agreement, and the
    one-lesson chapter header. Regenerated narration and its hash shards
    (1,225 files, every one a one-sentence change).
