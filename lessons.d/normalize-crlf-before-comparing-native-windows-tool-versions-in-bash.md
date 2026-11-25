---
category: CI & GitHub Actions
---

# Normalize CRLF before comparing native Windows tool versions in Bash

When Bash runs a native Windows executable, captured version output can retain
carriage returns even though Bash separates lines on line feed. A strict exact
version comparison then fails on the correct version string. Remove only the
terminal carriage return from the first line before comparing it; keep the
comparison exact and keep the executable exit status required. This surfaced
when the qpdf 12.4.2 Windows binary printed CRLF output in the pinned-oracle
workflow step.
