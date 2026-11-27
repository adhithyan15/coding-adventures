---
category: Testing & coverage
---

# A duplicate-member negative control must keep the surrounding JSON parseable

When testing a parser's duplicate-member rejection, insert the repeated key
before the object's closing brace. Appending a second key after the closing
brace only tests malformed JSON, so the duplicate-key guard is never exercised.

When testing duplicate JSON member rejection, add the repeated member before
the object's closing brace so the document remains valid JSON apart from the
duplicate. Appending another member after the closing brace only tests the
parser's malformed-document path and never reaches the duplicate-key guard.
