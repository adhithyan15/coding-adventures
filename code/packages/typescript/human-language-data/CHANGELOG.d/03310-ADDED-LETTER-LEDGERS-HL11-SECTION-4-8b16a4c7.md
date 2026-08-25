### Added — Letter ledgers (HL11 section 4)

- `loadLetterLedgers()` reads `data/scripts/<script>-ledger.json`: the order a
  reader meets each script's letters, ordered by the words they make writable
  rather than by the traditional recitation order.
- `validateLetterLedger()` checks a ledger against the corpus that justifies it —
  contiguous positions, glyphs belonging to the named script, no vowel sign
  before a base letter, families kept together, every claimed unlock naming a
  lesson that exists, and unspent letters. Report-only.
- `summarizeLetterLedger()` publishes `firstWritableWord` and the writable-word
  curve. A word, not a letter count: twenty taught letters is not something a
  reader can feel, and writing *thank you* is.
- Each ledger row carries its **code point** beside its Unicode name. A rendered
  glyph is not an audit surface — it can be a lookalike, and it can carry code
  points that render as nothing — so the validator rejects a multi-code-point
  glyph, a code point disagreeing with the glyph, and a name from the wrong
  script. Without the first of those the two Unicode checks are satisfiable by
  different parts of one string: the script test is unanchored and the combining
  test is anchored.
- A ledger whose `tracks` match no loaded lesson now reports
  `ledger-unlocks-unverified` rather than passing silently. One mistyped track
  name would otherwise make the only check for fictional unlock claims vanish
  while the report still read zero.
- `loadLetterLedgers()` shape-checks each ROW, not just the two top-level
  arrays. The validator reads `glyph`, `codePoint`, `unicodeName` and `unlocks`
  off every row before it checks anything, so guarding only the arrays moved the
  unhandled TypeError down a level instead of removing it.
- Two positions sharing a `unicodeName` is an error. The code point pins a row to
  a character; this pins a name to one row, so a row duplicated and half-edited
  cannot leave two positions claiming to be the same letter.
- `loadScripts()` now skips `*-ledger.json`, case-insensitively. Both files sit in `data/scripts` and
  carry the same `script` key, so reading both into one map would have had one
  silently overwrite the other — decided by filename sort order.

