### Added — Bengali filmstrips switch on, and script digits count as inventory

- `figure-targets.ts`: `DERIVED_FILMSTRIP_SCRIPTS` gains `bengali`, so the
  Bengali letter lessons whose letter has a cited ductus print a filmstrip.
  Nine do: BN-W01-ra, BN-W02-chandrabindu, BN-W03-ba, BN-W03-kha,
  BN-W04-e-indep, BN-W04-o-indep, BN-W05-tha, BN-W06-nya and
  BN-W41-bisarga (`tests/filmstrip-target-counts/bengali.json`: 9). The
  candidate test that used Bengali as its switched-off track now uses
  Punjabi.
- `validate.ts`: `uncoveredGlyphs` counts a script's `digits` rows as
  covered, the way it counts letters. Bengali is the first track with digit
  headwords (৭, ৪ ৭ …); no other track's result moves.
- New `tests/script-inventories/bengali.evidence.ts`: the new
  `data/scripts/bengali.json` covers every Bengali code point a Bengali
  headword uses (the corpus glyph-gap queue stays empty), and exactly nine
  rows carry a cited order, lift count and LipiTk source.
- Regenerated: nine Bengali SVGs, the Bengali figure-hash owner, and eight
  Bengali book chapters (nine `\hlblockfigure` lines; chapter 8 carries
  two). Narration, modality and lesson prose do not change; no lesson
  duration moves.
