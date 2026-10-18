---
category: Repo policy / workflow reminders
---

# Decode resume text with universal newlines before matching anchors

The green checkpoint helper correctly wrote the consumed native result to
the external checkpoint, then decoded raw Windows CRLF resume bytes and
searched for LF-only anchors. It exited1 before writing any of the three
resumes. Native source, test results and Cargo targets were unchanged.
The recovery reads text with universal newlines, prepares and verifies all
three complete before/after bodies before any write, then updates only the
external resumes. The original helper, checkpoint backup and recovery receipt
remain saved. Prepare every related output before committing a metadata
transaction, and preserve physical preimages separately from parsed text.
