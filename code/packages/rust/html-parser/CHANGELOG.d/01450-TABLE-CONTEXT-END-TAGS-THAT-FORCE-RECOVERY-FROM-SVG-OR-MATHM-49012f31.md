- Table-context end tags that force recovery from SVG or MathML foreign
  content now report the foreign-content parse error, covering 4 previously
  silent malformed corpus cases without changing DOM recovery or
  undeclared-diagnostic coverage.
