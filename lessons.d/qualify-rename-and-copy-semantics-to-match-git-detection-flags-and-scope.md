---
category: Repo policy / workflow reminders
---

# Qualify rename and copy semantics to match Git detection flags and scope

HL42 described copy records and “every rename,” while the implementation used
`diff-tree -M` after a pathspec. That does not enable copy detection, and a
rename crossing the measured scope can collapse to an add or delete because
one endpoint is filtered out. A report contract must match its Git invocation:
either enable and test the promised detection on the unfiltered diff before
applying literal scope membership, or explicitly narrow the schema/spec so it
does not imply evidence the command cannot observe.
