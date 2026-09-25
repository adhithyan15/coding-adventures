### Added — filmstrips derived from single-letter writing lessons, and a gate that every filmstrip is printed (HL-C443)

Before this change no book printed a stroke-order filmstrip. Three were
generated as proofs and no lesson referenced them.

- **`src/figure-targets.ts` (new).** A `type: writing` lesson with a
  one-grapheme headword and a `## Writing:` block, on a track listed in
  `DERIVED_FILMSTRIP_SCRIPTS`, is a filmstrip candidate. It becomes a figure
  only when the generated filmstrip ledger holds a cited ductus for the letter
  (HL11 §5.2: no citation, no figure). A declared target still wins for its
  lesson.
- **`figure-cli.ts`.** `resolvedFigureTargets(root, lessons)` merges the
  declared targets with the derived ones.
- **`book-cli.ts`.** The book's view of each lesson places the filmstrip at the
  top of its Writing block, above the numbered strokes. Narration keeps the
  authored lesson, since a listener cannot see a figure. A curriculum with no
  figure config prints none.
- **`tests/figure-targets.test.ts` (new).** Covers candidate selection, the
  merge and image placement. It pins Tamil at 20 lessons and 19 letters. It
  also gates that every resolved filmstrip appears in its chapter's `.tex`,
  which caught the Persian proof figure being printed nowhere.

First rollout is Tamil (19 letters), plus the two existing proofs: Hindi आ, and
Persian چ, which is now placed by hand in `FA-C03-chist`.
