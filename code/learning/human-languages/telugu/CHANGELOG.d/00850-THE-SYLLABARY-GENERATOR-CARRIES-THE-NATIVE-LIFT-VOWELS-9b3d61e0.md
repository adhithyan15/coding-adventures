## Unreleased — the syllabary generator carries the native-lift vowels

- `generate_syllabary.py` declares అ and ఎ explicitly in
  `VERIFIED_INDEPENDENT_VOWELS`, and an explicit declaration wins over the
  committed row. The table still held the old tracing-guide rows (one pen
  lift each), so regenerating `telugu.json` would have put the lifts back,
  and `test_committed_outputs_are_regeneration_stable` failed. The two
  entries now match the committed native-lift rows field for field: one
  pen-down run each, with the HP Labs India counts cited in the note and
  variation. A regeneration is a no-op again.
