---
category: Testing & coverage
---

# Derive refusal tables from the actual guarded result

The first CLOC41 native tests compiled against unchanged production and ran
3 passed/9 failed. Eight failures exposed the intended lineage gap; the ninth
was a wrong blanket nonfinite-input refusal oracle. The existing Math handler
checks result.is_finite(): sign of positive/negative Infinity yields one/minus
one and is valid existing admission. The validator correctly withheld red
classification on the unexpected count and did not execute the CLI stage.

Keep that actual failed receipt and the static-review prediction separate.
Clarify the specification before changing the test table, retain both complete
Infinity cases as exact positive bits/raw/lineage assertions, and keep every
genuine declined case. Do not change production admission to satisfy a mistaken
negative oracle or silently delete its inputs. Compiled tests distinguish a
test-plan prediction from observed behavior.
