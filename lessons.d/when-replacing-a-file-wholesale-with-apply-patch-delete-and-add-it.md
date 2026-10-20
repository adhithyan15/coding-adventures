---
category: Repo policy / workflow reminders
---

# When replacing a file wholesale with apply_patch, delete and add it in separate patch operations because one patch cannot target the same path twice.

A wholesale test rewrite attempted `Delete File` and `Add File` for the same path in one patch. The patch tool rejected the duplicate target before changing the file. Apply the deletion first and the addition in a second patch, or use a single update hunk, so every path appears only once per patch operation.
