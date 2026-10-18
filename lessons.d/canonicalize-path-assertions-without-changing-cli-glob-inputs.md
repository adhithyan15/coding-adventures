---
category: Cross-platform & Windows BUILD_windows
---

# Canonicalize path assertions without changing CLI glob inputs

CLOC41's corrected native red built and completed4P9F, but every CLI fixture
failed its baseline guard with exit2 before lineage assertions. Canonicalizing
the temporary directory on Windows produced a verbatim path beginning with
the question-mark namespace prefix. The CLI interprets --js as a glob, so the
question mark changed path matching and no file matched. This is setup failure,
not the intended semantic red; the validator withheld the complete receipt.

Pass ordinary absolute fixture paths to the existing CLI, as the retained
integration fixtures do. Canonicalize both sides only when asserting the safe
temporary-parent boundary for cleanup. This also handles macOS temporary path
aliases without introducing Windows glob syntax. Keep the failed baseline and
all exact graph/value/sentinel assertions; do not accept an arbitrary nonzero
CLI status or weaken the expected successful baseline.
