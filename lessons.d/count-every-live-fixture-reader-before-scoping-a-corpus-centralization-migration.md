---
category: Repo policy / workflow reminders
---

# Count every live fixture reader before scoping a corpus-centralization migration, not only packages whose identity matches the corpus name

The first CommonMark centralization inventory counted fourteen package-local
copies by following the expected CommonMark-family identities. A byte-size and
digest census of the whole checkout found sixteen live readers instead: seven
GFM packages also load the official base CommonMark corpus, and six of those
roots were absent from the migration aggregate. The canonical fixture itself
was correct, but the durable receipt and follow-up backlog were not complete.

For corpus centralization, derive ownership from every production or test read
site for the exact bytes, then reconcile that list with a repository-wide
size/digest census. Do not infer the migration set from package identity names.
Record every live reader in the aggregate before decomposing it, even when a
consumer adds separate behavior on top of the shared base corpus. After fixing
the inventory, search the entire repository for the stale count and reader
description; a normative profile can retain the old scope even when its roadmap
and state receipts are corrected.
