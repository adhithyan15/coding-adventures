## Changed — no lesson shows more than three new letters at once

The script-ramp report (`ramp.script`, policy `maxNewGlyphsPerLesson: 3`)
counts every target-script shape the first time a lesson body shows it.
It listed TE-C01-namaskaram and TE-C16-nelalu (8 each) and
TE-C01-dhanyavadamulu (6); it now lists no Telugu lesson. Script closure and
the exposure-exempted glyph count are unchanged.

- **TE-C01-namaskaram** (8 → 3) reads న మ ర and traces న;
  **TE-C01-dhanyavadamulu** (6 → 3) reads ధ beside plain ద, and వ. Both
  headings are romanized and both say the whole word appears near the
  chapter's end.
- The chapter's letter lessons carry the rest, at most three each:
  TE-S150-letter-ma reads the tail ములు (ు, ల), TE-S03-letter-ka the stacked
  స్క (క, స, ్), TE-S07-vowel-sign-aa కా and వా, and TE-S120-vowel-sign-ee
  shows both long words whole for the first time (ం, య). TE-S150 and TE-S151
  now point at the shapes already met instead of words not yet shown.
- **TE-C16-nelalu** (8 → 3) writes seven month names (new ణ, భ, జ); the
  other five are spelled in the short letter lessons that follow (ఖ, ఘ, ఠ,
  ఫ, ఢ), each beside its one rare shape. Its heading is romanized.
- Regenerated book chapters 1 and 16, narration, modality and hashes.
