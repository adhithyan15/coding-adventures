- HTML `p` and `br` start tags recovered from seeded foreign fragment contexts
  now report the foreign-content breakout parse error. This closes the final
  malformed tree-construction case that previously emitted no diagnostic.
