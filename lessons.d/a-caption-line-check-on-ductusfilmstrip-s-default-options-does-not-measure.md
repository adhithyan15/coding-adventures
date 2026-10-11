---
category: Testing & coverage
---

# A caption-line check on ductusFilmstrip's default options does not measure the printed strip

**Context:** `script-ductus`, adding the Jayasree-cited Malayalam glyphs (്,
ഠ, ൧-൯) and wanting a test that no caption needs more than the two lines a
panel holds.

**What happened:** the first try called `wrapCaption(segment.label)` with one
argument. `wrapCaption(text, width, captionSize)` then computed
`Math.max(6, NaN)`, which is `NaN`, so no comparison ever wrapped and every
caption came back as one line: the check passed and measured nothing. The
second try counted the `tspan`s in `ductusFilmstrip(letter, outline).frames`.
That failed for seven glyphs with three to five lines, although the rendered
book SVGs showed every caption in at most two lines.

**Why:** the printed strip is built by the filmstrip ledger, which sizes the
caption per letter (`captionSizeFor`) and the panel per strip; a bare
`ductusFilmstrip` call uses the defaults, whose panel and type size differ. So
neither call measures what the book prints.

**Fix:** dropped the check. The captions were verified the way the earlier
lesson on wide letters recommends: regenerate the ledger and figures, render
each new `*-filmstrip.svg` to PNG (`rsvg-convert`) and look at every panel.

**Do differently:** to test caption fit, read the ledger entry or the generated
figure, not a default-option strip; and never call a helper with fewer
arguments than its signature, since a `NaN` threshold silently passes every
comparison.
