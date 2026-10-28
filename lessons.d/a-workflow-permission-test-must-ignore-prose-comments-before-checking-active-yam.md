---
category: Testing & coverage
---

# A workflow permission test must ignore prose comments before checking active YAML

The replay-safe book publication test initially searched the preflight job's
entire text for `contents: write`. A comment *between* `publication_preflight:`
and `publish:` explained that the latter job holds `contents: write`, so the
test failed even though the preflight job's active permission was only
`contents: read`. The fix was to exclude YAML comment lines before asserting
on the job block. For future workflow contract tests, distinguish active YAML
from explanatory prose; ideally parse the relevant job mapping when a parser
is already available, and keep a negative test that proves a real permission
change is still caught.
