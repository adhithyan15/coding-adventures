---
category: Repo policy / workflow reminders
---

# Use discovered paths instead of inferred test filenames

An inspection command listed the exact integration-test filenames and then
immediately tried to open a shorter guessed filename, causing the combined
read to stop before the remaining source and fixture files. When `rg --files`
or another discovery step has already produced authoritative paths, copy those
paths into subsequent commands instead of reconstructing names from memory.
