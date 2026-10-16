---
category: Testing & coverage
---

# Resolve the complete commit hash before invoking a frozen-source runner

The D3b successor red runner was invoked with an invented suffix after the known
a522cb43 prefix. Its exact-source guard correctly refused before creating any
receipt or starting Cargo. Resolve git rev-parse HEAD directly and pass that
complete value; never reconstruct a commit identity from a remembered prefix.
The refusal provides no compile or semantic-test evidence.

The same run also guessed helper/spec filenames while reading evidence; those
reads failed without changing source or executing tests. Enumerate filenames
with rg --files or the already returned directory listing, then read the exact
returned path. An assumed identifier is not verified input, even for a read.

A later inline PowerShell replacement embedded Python quotes with backslash
escaping; PowerShell rejected the entire cell at parse time. Use apply_patch or
a saved Python helper for nested syntax instead of shell-escaped replacements.
No part of that refused cell executed and it supplies no validation evidence.
