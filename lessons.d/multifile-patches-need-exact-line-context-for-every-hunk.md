---
category: Repo policy / workflow reminders
---

# Multifile patches need exact line context for every hunk

A CV02 documentation patch assumed a sentence began on a new line, but the
actual file wrapped it onto the preceding line. One mismatched hunk rejected
the entire multifile patch. Read the exact nearby lines for each target, keep
hunks small, then inspect the resulting diff rather than inferring success.

The follow-up lesson command also used an invented category. Use a category
listed by `lessons.py` or `_meta.md`; an intuitive label is not necessarily
part of the closed category contract. Correct the command before filling the
exact path it prints.
