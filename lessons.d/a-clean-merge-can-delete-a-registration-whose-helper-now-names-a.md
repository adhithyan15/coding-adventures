---
category: Testing & coverage
---

# A clean merge can delete a registration whose helper now names a different package

Main removed a historical-Unix fixture row because its selector derived the
Ruby package name with hyphens, while discovery uses the underscore name. A
feature branch corrected the selector to match the discovered package. Git
merged the row deletion cleanly, even
though the new helper made that registration valid and necessary.

The existing selection tests and the discovered-package identity regression
caught the missing reader. Restore its row with the corrected helper, retain
the negative assertion against the obsolete hyphenated alias, and rerun the
routing suite.
Review both merge conflicts and automatically merged registrations: textual
success does not establish that the combined behavior preserves each side.
