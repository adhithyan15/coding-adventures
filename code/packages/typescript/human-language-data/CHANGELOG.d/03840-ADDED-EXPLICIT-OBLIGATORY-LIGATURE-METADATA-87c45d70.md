### Added - explicit obligatory ligature metadata

- Represent a ligature's editable letter sequence separately from its
  presentation-form glyph, initially for Arabic lam-alif.
- Keep ligatures outside the base-letter inventory while retaining sourced
  forms, components, writing order, and lift count.
- Close Arabic's sourced shape audit after its base letters, ending forms,
  carrier compositions, and obligatory ligature pass their provenance gates;
  keep the separate corpus-closure flag fail-closed.

