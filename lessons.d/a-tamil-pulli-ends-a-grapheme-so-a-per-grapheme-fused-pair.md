---
category: Testing & coverage
---

# A Tamil pulli ends a grapheme, so a per-grapheme fused-pair check cannot see a ligature formed across it

**Context:** giving the Tamil pulli ் a ductus and a `WRITTEN_SIGN_SIDES`
row, so words such as வணக்கம் compose as consonant, then dot.

**What happened:** the composer's guard against drawing a shape the page
does not show is `FUSED_SIGN_PAIRS`, checked inside `writtenPiecesOf` for
one grapheme at a time (consonant + its own signs). Tamil's virama is not
an extended-grapheme linker (Unicode's InCB=Linker covers Devanagari,
Bengali, Gujarati, Oriya, Telugu and Malayalam only), so க்ஷ segments as two
graphemes, க் and ஷ. Dumping every GSUB lookup of Noto Sans Tamil that
mentions the virama glyph showed that the font joins க + ் + ஷ into one
glyph (`akhn`) and ஸ/ஶ + ் + ர + ீ into another (`abvs`). With the pulli
row added, a word holding either would have been drawn as separate parts
that the printed page never shows, and the per-grapheme check could not
have noticed.

**Fix:** a word-level table, `FUSED_LETTER_SEQUENCE_SOURCES`, with its GSUB
citation and a test that pins the bundled font's version, refuses any word
containing such a run. The same dump confirmed that every other consonant
+ pulli glyph is a composite of the unchanged consonant and the unchanged
dot, so those are honest to draw in parts.

**Do differently:** when a sign gains a written-order row, dump the font's
lookups for that sign's glyph and check whether any ligature spans a
grapheme boundary, not only whether the sign fuses with its own consonant.
Viramas are the usual culprits.
