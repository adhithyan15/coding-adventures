---
category: Repo policy / workflow reminders
---

# Inspect each evidence manifest schema before reusing a validator

The native18 union preflight assumed both independently authored manifests
used the same nested blobs field. Raw18 instead used before/after records with
relative paths and explicit absence objects. A KeyError stopped before any
repository or preflight-directory write. Preserve the failing helper/receipt,
inspect each schema and validate actual absences/Git identities independently;
do not normalize away absent slots or silently skip a complete frame.
