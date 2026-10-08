---
category: Repo policy / workflow reminders
---

# Anchor source-edit paths at the workspace root when changing a command working directory

A combined source-edit and cargo command ran from the compiler package directory, but the Python edit still used repository-relative `code/programs/...` paths. The edit failed before changing files; cargo then ran against the earlier source. Keep source edits rooted at an explicit workspace path, or separate root edits from package-scoped commands. Verify working directories before combining dependent operations, and do not attribute test results to edits that never ran.
