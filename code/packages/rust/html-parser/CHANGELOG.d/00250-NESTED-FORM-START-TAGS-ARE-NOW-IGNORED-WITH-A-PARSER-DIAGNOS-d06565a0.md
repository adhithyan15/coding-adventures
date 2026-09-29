- Nested `form` start tags are now ignored with a parser diagnostic while an
  outer form remains open, keeping form-associated content in the existing form
  instead of creating nested form DOMs.
