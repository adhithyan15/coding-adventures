## Unreleased — voice mode defers hands-on cues the way narration does

- `buildVoiceScript` no longer turns every `[YOU …: …]` prompt into a
  `respond` step. A prompt the narration export marks `spoken: false` — a
  `WRITE`, `TRACE`, `POINT`, `GESTURE`, `LABEL` or `FEEL` cue, which a driver
  cannot do — is now spoken as the narration's own deferral, "Once you have
  stopped driving — write: …", with no answer gap after it. Before, voice mode
  read the instruction as "Your turn" and waited eight seconds for a driver to
  pick up a pen; across the corpus that was 1,355 cues (836 `WRITE`, 476
  `TRACE`, 41 `POINT`, one `LABEL`, one `FEEL`).
- Every deferred cue is queued and recapped once at the end of the lesson
  ("Saved for when you have stopped driving, 2 things. Write: … Trace: …"),
  which is HL10 §10.2's "skip it and queue it".
- A `sight` or `pen` lesson's spoken notice ("Before we start: this one needs
  your hands…") is now read straight after the lesson title, where the
  narration's plain-text script puts it. Voice mode used to drop it.
- The verb and qualifier are joined with human-language-data's own
  `joinQualifier`, so `[YOU WRITE (m.): …]` says "write (m.): …" exactly as the
  plain-text narration does.
- Single source of truth: the app keeps no list of manual verbs. The
  generator's `spoken` flag decides (CI checks the narration export
  byte-for-byte, so it cannot go stale); a segment without the flag asks
  human-language-data's `MANUAL_CUE_ACTIONS` directly, counting a multi-word
  verb as manual if any of its words is. When that set grows, voice mode
  follows. New export: `isHandsOnCue(segment)`.
- Tests: a WRITE cue is deferred and recapped with no `respond` step and no
  answer silence; a SAY cue is still asked with its eight seconds; a mixed
  block keeps its order; qualifiers; multi-word verbs (`WRITE OUT`,
  `RETURN TO`); every member of the shared set is deferred; the generator's
  flag wins over the verb; the notice follows the title; and against Arabic
  chapter one's real narration, no manual cue is ever asked.
- Eager chunk grows by 1,348 bytes (465,593 → 466,941); only the shared set,
  not the rest of `narration.ts`, reaches the bundle.
