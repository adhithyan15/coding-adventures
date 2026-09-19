---
category: Testing & coverage
---

# Parser error tests must keep every earlier field valid

When a parser intentionally reports the first invalid field, a fixture aimed at
a later error must keep every preceding field valid. Otherwise the test either
asserts the wrong precedence or changes meaning when validation becomes
correctly sequential. Construct malformed inputs from a known-valid value and
alter only the byte belonging to the error under test.
