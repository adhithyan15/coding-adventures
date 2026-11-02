---
category: Testing & coverage
---

# Read-only diff checks can fail after all tests pass because of trailing blank lines

The TaskApp package suite passed all twelve tests, but the following diff check
rejected an extra blank line at the README end. Remove the trailing blank line
and rerun the diff check; a passing test suite does not establish whitespace
validity. Inspect each command result when running verification sequences.
