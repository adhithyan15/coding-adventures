---
category: Repo policy / workflow reminders
---

# Update algorithm prose when the implementation strategy changes before review

The RSA public-operation spec still described fixed-width output conversion by
repeated division after the implementation deliberately switched to the
repository integer substrate's reviewed hexadecimal formatter. The public
contract was unchanged, so tests could not expose the stale implementation
prose. Before committing a spec-first slice, compare every low-level algorithm
description against the final source, especially after replacing a hand-rolled
primitive with a reviewed repository-owned helper.
