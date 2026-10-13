---
category: Repo policy / workflow reminders
---

# Use find or test -e instead of optional unquoted globs in repository audits

An otherwise useful discovery command ended with `ls code/scripts/CHANGELOG*`.
No matching changelog exists, so zsh rejected the unmatched optional glob and
made the complete audit command fail after printing its earlier results. This
repeated an already-recorded glob lesson because the optional check was added
at the end without applying it. Use `find` for discovery or guard an exact path
with `test -e`; never append an unquoted optional glob to a multi-part audit.
