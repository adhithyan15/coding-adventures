---
category: Testing & coverage
---

# A combining sign's own glyph can carry ink its handwriting source never draws; measure the fit before planning around a sign

**Context:** giving Devanagari vowel signs a ductus for the lessons that teach
a sign by itself. The source (native writers' pen traces in HP Labs India's
LipiTk recognizer) has every sign written alone, without a headline, and the
plan listed all twelve common signs as covered.

**What happened:** Noto Sans Devanagari draws ा, ि, ी, ो and ः with a short
piece of headline (x 0 to 273 font units at the top of the stem, or above the
visarga's dots) so that the sign joins a word's headline. The traces never
draw that piece. Fitted to the ink, the paths that follow the traces left
9.7% (ा), 4.0% (ि), 3.5% (ी), 4.5% (ो) and 37% (ः) of the printed sign
untraced, over the 2% the stroke-honesty check allows at default tolerance.
Five of the planned signs, including the most taught one (ा), could not be
drawn honestly without an override or an invented headline stroke.

**Fix:** those five were left out and the reason recorded in the spec, the
tests and the changelogs; the eight signs whose glyph is only the sign
(ु ू े ं ़ ् ृ ँ) were drawn.

**Do differently:** before promising coverage for a mark, render the font's
glyph for the bare code point and run the honesty metrics (on-ink fraction and
untraced fraction) on a rough path from the source. A glyph can include
joining ink (a headline stub, a connector, a carrier) that no isolated-sign
source draws.
