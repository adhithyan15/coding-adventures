### Added — tone in the script data model, and a `pronunciation` lesson type (HL-C39)

Driven entirely by the Mandarin Chinese track, which was added as a scale test for
whether the curriculum model generalises outside Indo-European and Dravidian.

- `ScriptData` gains `tones?: Tone[]` and `toneSandhi?: ToneSandhiRule[]`.
  `Letter.tone` already existed and labels the tone a *character* carries, which is
  enough to tag a glyph and nothing more. It cannot say what tone 3 *is* (contour
  214, low and creaky), and it cannot express **sandhi** — a rule that changes a
  syllable's pitch because of the syllable *after* it while the characters and the
  printed pinyin stay identical. Every previously modelled script encodes
  pronunciation segmentally, and a segment always attaches to a glyph; tone is
  suprasegmental, so the existing shape did not stretch. `data/scripts/chinese.json`
  populates both fields.
- `EXEMPT_TYPES` gains `pronunciation`. No earlier track ever needed a lesson
  *about* sound, because segmental facts belong to letters and therefore live inside
  the word lesson that first uses that letter (HL00, "Pronunciation & Script:
  Inline, Never a Gate"). Folding Mandarin's tone system into its first character
  lesson pushed that lesson to 352 effective seconds, past the five-minute contract,
  and HL08's rule is to split rather than waive. `grammar` would have misfiled a
  sound rule as morphology; an unrecognised type would have produced a permanent
  validator warning. Like `grammar` and `etymology`, `pronunciation` is exempt from
  the cross-language concept join because its progression lives in knowledge atoms.

