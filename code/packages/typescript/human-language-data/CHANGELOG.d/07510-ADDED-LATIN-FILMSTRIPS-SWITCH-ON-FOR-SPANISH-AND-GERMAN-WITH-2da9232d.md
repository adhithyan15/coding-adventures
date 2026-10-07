### Added Latin filmstrips switch on for Spanish and German with a Latin inventory

- `DERIVED_FILMSTRIP_SCRIPTS.spanish` and `.german` are `"latin"`, and
  `latin` joins `SEPARATE_LETTER_SCRIPTS`: print letters stand apart, so a
  word of cited letters is drawn letter by letter. Latin has no
  `WRITTEN_SIGN_SIDES` table, so a precomposed ñ is one letter and ñ typed as
  n + U+0303 is refused; a word with punctuation inside it (¿cómo?) is
  refused; a list of marks ("¿ ¡") is drawn.
- Four lessons print a strip: ES-W02-enye (ñ), ES-W03-inverted (¿ ¡),
  GE-W01-eszett (ß) and GE-W04-vier-zeilen (weil). Every other Latin writing lesson holds an a
  (Noto's two-storey a has no source) or an uncited mark, so French, Italian,
  Portuguese and Latin stay switched off.
- New `data/scripts/latin.json`: every Latin character the six Latin-script
  tracks' headwords use, decomposed (26 small letters, 24 capitals, ß, œ, ñ,
  the ordinal indicators ª and º, ¿, ¡ and seven combining marks),
  `complete` false, 18 cited rows. The validator's glyph closure never
  measures Latin (the script matchers leave Latin out), and letter anchoring
  and script closure skip Latin tracks, so no ceiling moves;
  `latin.evidence.ts` is the inventory's exact closure gate instead.
- Tests: a new figure-targets case for Latin print letters; the real-corpus
  case pins the four Latin strips and adds weil to the words pin; the switched-off example moves from Spanish to French; new
  `filmstrip-target-counts/spanish.json` (2) and `german.json` (2);
  `script-inventories/latin.evidence.ts`.
- `MAX_SEQUENCE_PIECES` (10): `filmstripCandidates` drops a sequence of more
  than ten written pieces. The 14-letter Großschreibung strip measured about
  2,380 units tall, against about 1,800 for the tallest earlier strip, and
  would print too small to read, so GE-W03-capitalization stays undrawn. No
  earlier strip has more than nine pieces, so nothing else changes.
