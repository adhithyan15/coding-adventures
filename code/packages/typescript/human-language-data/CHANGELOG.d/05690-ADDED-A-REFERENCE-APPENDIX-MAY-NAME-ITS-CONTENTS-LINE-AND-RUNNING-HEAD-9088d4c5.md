### Added — a reference appendix may name its contents line and running head

- `renderReferenceAppendix` took one `title` and used it for the chapter head, the
  table-of-contents line and, via a hardcoded literal, the running head. Those are
  three jobs. Optional `shortTitle` and `runningHead` fields now separate them, so
  a hand-set reference being retired into the generator keeps the strings its book
  already prints: a full title over the page, a shorter one in the contents, and a
  running head that names the subject rather than saying "Pronunciation" over a
  chapter about a writing system. Both default to today's behaviour, so every
  reference already generated is unchanged byte for byte.
- The italian and portuguese pronunciation references are generated from
  `<track>/pronunciation-reference.md` instead of hand-written `.tex`. Prose that
  existed only in the LaTeX was carried into the Markdown first; the per-track
  changelogs list it claim by claim.
- `reference-appendices.d`'s owner count is a floor with a registry-derived
  ceiling rather than a literal, for the reason the handwritten-chapter ratchet
  beside it already gives: seventeen retirements land as several PRs, and a
  literal is the one line every one of them would edit.
