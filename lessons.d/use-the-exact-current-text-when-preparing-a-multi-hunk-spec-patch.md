---
category: Repo policy / workflow reminders
---

# Use the exact current text when preparing a multi-hunk spec patch

A multi-hunk spec edit failed because one expected line said independently
where the actual file said independent. The entire patch was refused: diff
showed no tracked changes. Read the exact current text, or use a saved
replacement that verifies the complete original before writing. Preserve the
rejection separately from semantic red or implementation validation.
