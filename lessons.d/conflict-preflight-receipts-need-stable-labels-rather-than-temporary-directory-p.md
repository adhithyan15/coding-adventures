---
category: Testing & coverage
---

# Conflict preflight receipts need stable labels rather than temporary directory paths

An interrupted main-merge preflight preserved partial raw conflicts. Its retry
correctly refused to overwrite those files, but an attempted byte-equality reuse
also failed: default merge-file conflict markers embedded a newly randomized
temporary directory. Neither attempt modified source or accepted a partial merge.

Preserve failed receipts. Give merge-file explicit stable owned/base/incoming
labels and use a new snapshot-specific receipt namespace. Only reuse an existing
immutable artifact after checking its exact captured inputs and bytes; temporary
paths are not semantic identities. Complete blob availability and whole-tree
mode/type/blob reconstruction remain separate required checks.
