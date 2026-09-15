# A character no book has rendered before is invisible to every local gate (human-language books)

The round-two Hindi tranche passed tsc, 723 data-package tests, 725 app tests and all
five `check:` targets locally, then failed CI with:

    hindi missing_character rose to 1 against a baseline of 0

The books all COMPILED — `latexmk` exited 0 and wrote every PDF. What failed was
`scan_latex_log_warnings.py`, reading the `book.log` afterwards. The cause was one
character: **U+0325 COMBINING RING BELOW**, in the reconstructed PIE form *séh₂wl̥*
cited in the sun lesson. `\setmainfont{Latin Modern Roman}` has no glyph for it, so
XeLaTeX dropped it silently and logged one `Missing character` line.

**Why no local gate could catch it.** There is no LaTeX toolchain in the container, so
the only check that reads a `book.log` cannot run at all. Every gate that CAN run reads
the Markdown or the AST, and at that level the character is perfectly valid UTF-8 in a
perfectly valid sentence. The defect exists only in the font.

**The cheap pre-push check, which is also how it was found.** Diff the character set of
the .tex you just generated against the union of every .tex already in the repo:

```python
novel = chars(new_tex_files) - chars(all_existing_tex_files)
```

Anything in `novel` is a character no book has ever successfully rendered, so it is
exactly the candidate set for a missing glyph — two characters here, and the rare one
was the culprit. Run it against the LESSON MARKDOWN and you get the wrong answer: that
comparison missed U+0325 entirely, because the frontmatter was stripped and because
seven lessons in other tracks already use the character in prose **that no book
renders**. Compare the artifact the engine actually reads, not the source it came from.

**Corollary — "used elsewhere in the corpus" is not evidence a glyph renders.** I
cleared `ṓ` on exactly that reasoning (11 lessons, 8 tracks, all with baseline 0) and it
was sound for `ṓ`, which really is in the font. It would have been unsound for U+0325,
which appears in seven lesson files and zero rendered books. The question is never
whether the corpus contains the character; it is whether a BOOK has ever printed it.

**Fix in prose, not in the baseline.** The citation became "the Proto-Indo-European word
for the sun", keeping every cousin claim (*sōl*, *hḗlios*, *sun*) and dropping only the
reconstructed spelling. Raising `missing_character` to 1 would have blessed a character
the reader never sees — the glyph is dropped from the page, so the book silently prints
a different word than the source says.
