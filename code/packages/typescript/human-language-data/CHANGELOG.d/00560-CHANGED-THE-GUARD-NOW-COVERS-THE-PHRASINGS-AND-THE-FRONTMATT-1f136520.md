### Changed — the guard now covers the phrasings, and the frontmatter

- Extended from `.tex` alone to **chapters plus lesson sources**, so frontmatter is
  held to the same rule. Six patterns replace one: a memory verb aimed at another
  language, `the <lang> <material>`, `<lang>'s … <material>`, and the original
  "in this course". Each proven to fire on a reintroduced defect.
- Two false-positive classes it must **not** flag, both found by running it:
  *"seen Hindi borrow umr **from Arabic**"* — a borrowing source, not a locator, so
  bare `from <language>` no longer counts; and *"unlike how closely Kannada and
  Telugu **track** Tamil"* — `track` is a verb there, so the material nouns require
  an article or possessive.
- **The lesson surface needed un-escaping first.** `canonicalLessonSource` returns
  JSON, where a line break is the two-character escape `\n` and not a newline. So
  `\s+` could not cross a wrap — a pointer split as `"the Spanish\ntrack"` was
  invisible, and one real defect (`FR-C03-de-rien`) hid there — and `[^.?!\n]`
  bounded nothing, leaving the 60-char window free to jump paragraphs on exactly the
  surface that had just been added. One `.replace(/\\n/g, "\n")` fixes both.

