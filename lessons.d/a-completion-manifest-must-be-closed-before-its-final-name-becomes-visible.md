---
category: Testing & coverage
---

# A completion manifest must be closed before its final name becomes visible

Independent review found that opening `manifest.json` with exclusive creation
exposed the completion marker before its write and close succeeded. A failed
close could leave complete-looking JSON despite a failed run, and a polling
reader could see a partial file while it was still being written.

Write to a unique temporary file in the private output directory, check both
write and close, then publish its final name atomically without overwriting an
existing marker. This program uses a hard link and fails on unsupported file
systems before publication. Test write/close failures, existing-name refusal,
temporary-file cleanup and complete-marker contents separately from timed work.
