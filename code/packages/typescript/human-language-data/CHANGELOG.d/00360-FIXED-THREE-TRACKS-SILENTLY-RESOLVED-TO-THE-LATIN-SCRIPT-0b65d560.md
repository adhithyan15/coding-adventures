### Fixed — three tracks silently resolved to the Latin script

- `LANGUAGE_SCRIPT` had no entry for **Gujarati** — which was the worked example in its
  own doc comment — so all 39 Gujarati lessons resolved to `latin`. Glyph-coverage
  validation looked Gujarati headwords up in the *Latin* inventory, and `romanization`
  fell back to the Gujarati headword itself, so the narration export published
  `"romanization": "આભાર"` — **Gujarati script in the field a speech engine reads as
  Latin.** Regenerating `lesson-modality.json` and the seven Gujarati narration chapters
  is the whole blast radius; no lesson content changed.
- **Chinese** and **Japanese** were missing from the same map and were saved only by
  shipping a `track.json` the loader prefers. A fallback that is wrong for some tracks
  fails only in the paths that skip the loader — which is exactly where a unit test lives.
  Completing the map removes the trap.

