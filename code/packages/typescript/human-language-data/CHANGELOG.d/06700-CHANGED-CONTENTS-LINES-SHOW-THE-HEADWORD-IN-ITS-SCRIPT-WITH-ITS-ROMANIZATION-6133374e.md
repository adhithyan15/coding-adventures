### Changed — contents lines show the headword in its own script, with its romanization

Each lesson's contents line and running head used to show only the romanization in every non-Latin book (`kaṇ`). That hid the script the book teaches. They now show both, as every lesson heading already does: **கண் (kaṇ)**.

- **Width.** When the pair is wider than the 40-column budget, each half is cut to half the budget at a word boundary. A long weekday list keeps the start of both forms instead of all of one and none of the other.
- **Japanese lists.** Lists written without spaces, such as て・みみ・くち・…, are now cut at their middle dots. Before, they were never cut.
- **Headword only.** These lessons still show one form:
  - a practice lesson;
  - a track with no target script;
  - a lesson with no romanization, or one identical to its headword;
  - a bracketed placeholder such as `(X, continued)`.
- **PDF bookmarks.** The short title is also the PDF bookmark, so it is now wrapped in `\texorpdfstring` with plain text for the bookmark. Before, the Persian, Russian and Urdu preambles let their script commands reach the bookmark, and hyperref warned about 700 times per book.

**Glyph-coverage gate.** The gate now drops the bookmark argument of `\texorpdfstring` before measuring, through the new exported `stripPdfStrings`. That text is never typeset, and left in it read as about 28,000 Latin Modern gaps. The typeset argument is still measured exactly as body text is.

All 23 books are regenerated. The 22 that compile here (every book but Chinese, whose font download is blocked in this environment) compile with no overfull, missing-character or hyperref warnings.
