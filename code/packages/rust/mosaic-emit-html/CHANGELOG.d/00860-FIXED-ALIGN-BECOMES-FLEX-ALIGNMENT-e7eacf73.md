- **Fixed: mosstyle `align` becomes flex alignment (#16932).** `align` is
  mosstyle's own vocabulary, not a CSS property. It was written into the
  inline style verbatim (`align: center-vertical`), and browsers discarded it,
  so nothing using it was centred. `translate_layout_alias` now maps it as
  `mosaic-emit-react` does:
  - `center-vertical` → `align-items: center`;
  - `center-horizontal` → `justify-content: center`;
  - `center` → both;
  - `start` / `end` → `align-items: flex-start` / `flex-end`;
  - `space-between` → `justify-content: space-between`.

  Any other value passes through unchanged.
