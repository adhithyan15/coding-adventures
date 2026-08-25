### Fixed — the reconstruction asterisk was being deleted, turning reconstructions into attested forms

- `renderInlineMarkdown` reads a bare `*` as an italic opener, so `PIE *ne` printed as
  `PIE ` with the rest of the sentence italicised. In five chapters across German,
  Hindi and Telugu that **silently converted a reconstructed form into an attested
  one** — a false etymological claim, in the part of the book that exists for
  etymology. Lesson authors already wrote `\*`; the ledger authors did not, and
  nothing warned them. Escaped at source, with a test.

