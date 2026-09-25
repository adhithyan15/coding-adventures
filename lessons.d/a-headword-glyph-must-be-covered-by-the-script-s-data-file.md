---
category: Repo policy / workflow reminders
---

# A headword glyph must be covered by the script's data file, not only taught by a script lesson

Sanskrit's pre-A1 vocabulary tranche (chapters 66-89) was vetted with a
checker that compared each headword's glyphs with the glyphs the track's
writing and script lessons teach. Sanskrit teaches ङ (`SA-W65-letter-nga`),
so अङ्गुली, प्राङ्गणम् and मङ्गलवासरः all passed.

`validate.ts` checks something else. Rule (4) resolves every headword glyph
against `data/scripts/<script>.json`. When that file declares itself
`complete`, a gap is an **error**. `devanagari.json` is complete and has no
entry for ङ (a new entry needs a sourced stroke order), so the integration
test and `runValidate` both failed with "characters not yet in
devanagari.json: ङ".

Fix: two headwords were replaced (finger -> ललाटम्, courtyard -> छदिः), and
Tuesday takes the anusvara spelling मंगलवासरः.

Do differently: a headword is admissible only when **both** hold. A lesson
must teach the glyph (the gentle-ramp rule), **and** the script data file must
cover it (the validator rule). Check both before writing a spec. Also run
`tests/integration.test.ts` early; it is quick and catches this class
directly.
