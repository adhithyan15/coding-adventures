- **Style strategy:** mosstyle parts flatten to inline `style="..."`
  attributes on the matching element. Built-in primitive styles (e.g. the
  flexbox defaults for `Row`/`Column`) merge with the author's part style;
  author wins on collisions (last-property-wins, matching CSS specificity).
