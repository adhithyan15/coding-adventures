---
category: Repo policy / workflow reminders
---

# Patch exact surrounding lines after inspecting wrapped prose

An OAuth specification patch failed because its context assumed a sentence
began on a new line even though the existing prose wrapped it after the prior
sentence. The first attempt to record the failure also guessed a lesson
category that is not in the repository taxonomy. Inspect the exact nearby
lines before composing prose patches, and use `lessons.py index` or the
category list reported by the tool rather than inventing category names.
