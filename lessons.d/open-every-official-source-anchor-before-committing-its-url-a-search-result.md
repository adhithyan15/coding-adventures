---
category: Repo policy / workflow reminders
---

# Open every official source anchor before committing its URL; a search-result subdomain can be wrong

The first Telugu B2 inventory copied an LDC-IL text-corpus URL from a search
result whose hostname contained an extra `t`. The page description was genuine,
but the committed anchor itself would not resolve at the official hostname.
Independent review caught the defect before push.

For every external anchor, open the exact URL that will be committed and check
that the responding official page supports the nearby claim. Do not treat a
search result's displayed title or cached excerpt as validation of its link.
