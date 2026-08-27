## 0.12.0 — introduces a script the first time you meet it (HL03 phase 7a)

- **New writing systems are now introduced as-needed in the Learn sweep.** When
  the walk first reaches a non-Latin script — Arabic, then Devanagari (Hindi),
  then Tamil — that step's card carries a compact **"New script"** note: the
  script's name, its system (abjad / abugida), and *how to recognise it*, pulled
  straight from the script data's `signature` field (e.g. Devanagari's "a
  horizontal head-line runs across the top; letters hang beneath it like laundry
  on a line"). It appears **once**, at the earliest concept in book order that
  teaches the script, and never again.
- New pure `src/scriptintro.ts`: `LANGUAGE_SCRIPT`/`scriptOf` map each chain
  language to its writing system, `firstIntroductionByScript` computes the intro
  concept per script from the spine + lessons, and `scriptIntroFor` returns the
  note for a step or null. **Grounded, never invented:** a script with no JSON
  data (Kannada / Telugu / Malayalam today) gets no note — the mapping still
  knows the language's script, but the note is gated on having real data.
- 13 new tests (208 total) with controls that bite: the *second* appearance of a
  script must not re-introduce it; a script absent from the available-data set
  yields no note. Verified in a real browser — concept #1 (COURTESY-THANKS)
  shows the note on its Arabic / Devanagari / Tamil stops and nothing on the
  data-less Dravidian stops. Grammar-intro is the next slice (7b).

