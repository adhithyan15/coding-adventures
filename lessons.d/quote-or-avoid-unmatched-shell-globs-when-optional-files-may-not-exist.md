---
category: Repo policy / workflow reminders
---

# Quote or avoid unmatched shell globs when optional files may not exist, and use an exact allowed lesson category

An audit command passed `HL21*` and `HL22*` as unquoted zsh arguments even though
those optional paths did not exist, so zsh aborted the whole read-only command
before the useful searches ran. The first attempt to record the failure also used
an invented lesson category instead of one of the exact categories printed by the
repository tool. Retry with `find` or quoted patterns, and copy an allowed category
verbatim from `lessons.d/_meta.md` (or the tool's error output) when recording it.
