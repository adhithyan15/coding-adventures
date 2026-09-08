### Added — a reading passage can now reach a book at all

- `comprehension` was a declared-but-dead member of `LessonBlockType`: nothing
  produced it and nothing rendered it. `## Reading` now classifies to it, and
  the book renderer sets it as an inline `tcolorbox` titled *Read this*.
- Inline rather than a named environment on purpose. `culture` and `grammarlens`
  are defined in per-track `preamble.tex` files — 23 of them, all hand-written,
  and exactly the files this programme is retiring. An inline box means any
  track can print a passage the day it authors one, with no preamble edit.
- The index gains a `reading` facet, so a passage is findable rather than
  silently filed under nothing.
- Before this, the only way past the parser was to label a passage
  `You'll want to know` and call it an input block. That is why the corpus had
  three `type: reading` lessons and no reading. See HL-C369 in the backlog for
  the twenty-two tracks still without one, and for the exam inventories that
  cannot yet see reading at all.
