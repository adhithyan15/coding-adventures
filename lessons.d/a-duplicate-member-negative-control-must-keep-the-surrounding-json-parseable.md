---
category: Testing & coverage
---

# A duplicate-member negative control must keep the surrounding JSON parseable

When testing a parser's duplicate-member rejection, insert the repeated key
before the object's closing brace. Appending a second key after the closing
brace only tests malformed JSON, so the duplicate-key guard is never exercised.

TODO: what went wrong, what the fix was, and what to do differently. Replace this line.
