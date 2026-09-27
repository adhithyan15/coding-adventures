- Self-closing flags on non-void HTML start tags are now ignored with a parser
  diagnostic, keeping elements such as `div`, `script`, `textarea`, and table
  cells open for their real content.
