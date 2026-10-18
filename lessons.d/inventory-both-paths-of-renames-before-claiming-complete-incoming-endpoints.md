---
category: CI & GitHub Actions
---

# Inventory both paths of renames before claiming complete incoming endpoints

The first whole-main endpoint materializer correctly used --no-renames raw
changes, but incorrectly compared them with an earlier rename-detected
--name-only inventory. Its completeness guard refused: raw incoming changes
had 1162 paths, while the earlier list had1110. All52 additional paths were
Spanish changelog rename sources, which must have explicit deleted endpoints;
the previous report had listed only destinations. Owned paths were892 in both.
No source/index state was written and no incoming code was executed. Preserve
the failed raw command receipts and correct the earlier completeness wording.

Close a three-way endpoint inventory with a saved explicit rename map and
--no-renames old/new modes and object IDs. Compare both old and new rename
paths, not just displayed destinations. Save absence as an endpoint, verify
every blob length and Git object hash, and distinguish a path list from a
complete endpoint audit. Keep the full incoming set unaccepted until independent
review and affected validation finish.
