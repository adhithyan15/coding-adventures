---
category: Repo policy / workflow reminders
---

# A filmstrip caption that wraps to two lines can still run past its panel

`wrapCaption` in script-ductus counts characters, not glyph widths: it
breaks a caption at roughly 23 characters per line for a filmstrip panel.
In the fourth Kannada consonant batch, ಶ's second caption wrapped to two
lines by that count ("2. curve down and sweep" / "to the lower left"), so a
character-count check passed, but the rendered SVG showed the first line
running past the panel's left edge, because "w" and "p" are wider than the
average the wrap assumes. Nothing in the test suites measures rendered
width, so only looking at the figure caught it.

The fix was a shorter label ("sweep down to the lower left"), changed in
the ductus, its tests and the ledger together, then the figures and books
regenerated.

What to do differently: render every new filmstrip SVG to PNG (for example
with cairosvg) and look at each panel, not only the line count. Treat a
caption line of 22 or more characters, numbering included, as a likely
overflow when it holds wide letters (m, w, p, d), and shorten it before the
stroke-ownership hash is re-measured, since every caption edit moves that
hash.
