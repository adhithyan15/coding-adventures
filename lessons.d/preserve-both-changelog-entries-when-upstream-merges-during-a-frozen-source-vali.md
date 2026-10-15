---
category: Repo policy / workflow reminders
---

# Preserve both changelog entries when upstream merges during a frozen source validation

The CV04a main refresh met an ordinary top-of-file changelog conflict: upstream
PREP01 documented bounded shifts while the candidate documented Windows emitted-C
linking. Retain both entries and their descriptions; neither source feature
supersedes the other. Check the automatically merged test helper still retains
the Windows libm condition, and rerun the changed C consumer after the merge.

Before touching a checkout running long Miri validation, traverse the conservative
local dependency closure and compare every upstream changed path, including the
workspace manifests, lockfile and script. Refuse a merge that touches that cone.
The PREP01 C delta was outside it, so final lexical/Miri source bytes stayed frozen.
Record the merge and exact validation heads instead of calling old-head evidence
current. Security review must cover the refreshed candidate before publication.

The first lesson CLI attempt used an invented category and was rejected. Use an
exact category from the returned catalogue. Inspect large one-line JSON with a
parser and print selected fields: a line-count limit does not bound its output.
