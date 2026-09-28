- Non-whitespace character data rejected after a template-owned `col` now
  reports the required column-group parse error while remaining ignored.
  ASCII whitespace is retained, while ordinary content, real column groups and
  cells, nested templates, foreign template-named elements, and synthetic
  template fragment contexts retain their existing diagnostic behavior.
