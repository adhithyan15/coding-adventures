---
category: Ruby
---

# Ruby String#split with a single-space string is awk-style and swallows doubled spaces, so a strict tokenizer must split on a regexp

The Ruby TFV validator (VER05h) split a record line with `line.split(" ", -1)`.
In Ruby a single-space STRING pattern means awk-style splitting: runs of
whitespace collapse, leading and trailing blanks vanish and the limit is
ignored for them, so `"a  b"` became two tokens and the "doubled space"
rejection never fired. A negative-control test (`Kv  I1` must be refused)
caught it. Use the regexp `split(/ /, -1)` wherever exactly one separator
character is the grammar, and give every strict tokenizer a doubled-space and a
trailing-space negative control.
