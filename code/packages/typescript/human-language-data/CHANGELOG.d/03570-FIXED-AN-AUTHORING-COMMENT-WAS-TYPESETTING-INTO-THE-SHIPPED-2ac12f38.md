### Fixed - an authoring comment was typesetting into the shipped Spanish book (HL-C221)

- `renderMarkdown` passed any non-directive `<!-- ... -->` straight into the book as body text. One had been printing inside a coloured culture box in `spanish/book/chapters/ch07-thank-you.tex` — a note from an author to future authors, in the reader's PDF.
- `parse.ts` strips the `hl-knowledge` and `hl-activity` directives because it consumes them; everything else arrived at the renderer untouched. Comments are now stripped whole-string, so a multi-line one goes as a unit.
- Found by the security review of the gate above, which observed that applying **Markdown** comment semantics to **LaTeX** output made the gate blind in exactly the place it exists to watch.

