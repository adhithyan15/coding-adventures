### Added — HL08 narration export: the drivable course, out loud (HL-C16)

- Add `src/speech.ts`: the shared judgement of **what can be said aloud**. Markdown
  inline → words a voice can pronounce (emphasis, code fences, link destinations and
  the linguist's reconstruction asterisk removed; `→` `←` `·` given spoken readings),
  and Markdown tables → spoken utterances or a *reasoned refusal*. Both `modality.ts`
  and `narration.ts` import it, so "this lesson is drivable" and "the export can
  actually narrate this lesson" are the same question asked once.
- Add `src/narration.ts`: the pure narration builder. From the canonical lesson AST it
  produces typed segments — `speech`, `pause`, `repeat`, `prompt`, `table`,
  `table-skipped`, `activity` — plus the continuous plain-text script rendered from
  them. This is the **audio-script output HL04's one-source pipeline diagram has named
  since it was written and which nothing had ever built**.
- Add `src/narration-cli.ts`: `--write` / `--check`, modelled joint for joint on
  `book-cli.ts`. Writes `<language>/narration/chNN.txt` and `.json` for all 375
  chapters plus a hash manifest at `core/generated-narration-hashes.json`. `--check`
  compares byte for byte and exits 1, so a lesson edited without re-running the
  exporter fails the build instead of leaving a voice assistant confidently teaching a
  lesson that no longer exists.
- **`[PAUSE Ns]`, `[REPEAT xN]` and `[YOU …: …]` are preserved as structured
  directives, not flattened into prose.** Cue parsing is a depth-tracking bracket scan,
  because the corpus nests brackets inside cues for real
  (`[YOU SAY: the pattern — "[nā] [pēru]"]`), and a Markdown link that is not a cue is
  handed back intact rather than mistaken for one.
- **A `[YOU SAY: …]` cue is never treated as an answer key.** Cues become `prompt`
  segments with `scored: false`; only `hl-activity` contracts, compiled through
  `compileLessonActivities`, become `activity` segments carrying `acceptedResponses`.
  This is `activity.ts`'s own rule — runtime consumers use only the typed AST and never
  recover prompts or answers from learner-facing Markdown — and the narration export
  would have been the easiest place in the package to break it.
- **Tables are linearised, never dropped.** A two-column word→gloss table becomes
  *"नमस्ते means hello"*; a three-column table becomes labelled facts. A column with no
  heading is spoken as a bare value rather than refused, because `| Read | | Meaning |`
  — script, romanization, gloss — is the corpus's commonest practice-table shape and
  the blank heading is one a sighted reader does not have either. A run of pipe rows
  with no delimiter row is read as an unlabelled sequence for the same reason.
- **A table that cannot be linearised is spoken, not skipped**: the learner hears its
  size, its column headings, and why it needs eyes, and the lesson is marked `sight` so
  they are told before they start. `sight` and `pen` lessons still export in full,
  opening with a notice naming what they will need and which sections to leave until
  they have stopped.
- Target-script text carries its `romanization` alongside — *"خداحافظ (khodâ hâfez)"* —
  drawn from the **whole chapter's** headwords, so a lesson can pair a word a
  neighbouring lesson introduced. Pairing is whole-word only: the Arabic track teaches
  ا (*alif*) as its own lesson, and a plain substring replace turned سلام into
  `سلا (alif)م`, splicing the pronunciation guide into the middle of the word.
- Report `narration-block-unrenderable` when a lesson carries a table the export cannot
  speak yet claims `voice`, and `narration-activity-invalid` when an authored contract
  will not compile. Both are collected, never thrown — one bad directive must not
  silence a lesson.

