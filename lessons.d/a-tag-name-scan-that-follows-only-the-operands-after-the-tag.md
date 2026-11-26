---
category: Python
---

# A tag-name scan that follows only the operands after the tag never sees an unknown tag inside a list operand

The Python lane's IR walker treated every item after a tag as a sub-expression and skipped anything that was not
a node. A `match` keeps its arms in a list of `[pattern, body]` pairs, a `tuple` and a `ctor` keep their operands
in a list, so the walker never descended into them. VER04r section 4.2 rule 7 says an unknown tag is a corpus error
wherever it stands, and tag names win over level scoping; instead the enclosing `match` was refused as
`level-missing` and the unknown tag inside it was never looked at. The independent TypeScript lane disagreed on
exactly this input, which is how it was found.

Fix: give the walker one rule per tag shape (`operands` in `ir.py`), and test a hostile tag in every position a
list can hold one. When a checker is told to look at "every node", enumerate the positions that can hold a node,
not the positions that happen to look like one. A second lane run on crafted inputs is what finds this; a kit
that pins only the shapes its author thought of does not.
