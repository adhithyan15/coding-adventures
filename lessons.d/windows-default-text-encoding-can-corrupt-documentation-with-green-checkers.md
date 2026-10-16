---
category: Python
---

# Windows default text encoding can corrupt documentation with green checkers

A timeline documentation helper used default read_text/write_text on Windows.
The new shard encoded an em dash as raw CP1252 byte97, while reading an existing
UTF-8 changelog as CP1252 and re-encoding UTF-8 corrupted two historical dashes.
The doc-shard/lessons/diff checkers all returned0, but independent raw-byte review
correctly withheld the carrier. Preserve those receipts, use explicit UTF-8 for
all authored text, reconstruct the entire historical suffix from its original
Git blob, and require strict UTF-8 decoding alongside structural doc checks.
Do not transfer checker success into an encoding or untouched-history claim.

The first repair restored the temporal suffix, then stopped when the new shard's
raw CRLF checkout differed from its LF Git blob. Preserve that failed helper;
the successor admits only the exact already-restored temporal bytes or exact
CRLF-to-LF equivalence to the frozen shard before writing corrected UTF-8/LF.
Check every guard before assuming a compound repair completed all paths.
