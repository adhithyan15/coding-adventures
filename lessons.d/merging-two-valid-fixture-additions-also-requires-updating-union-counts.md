---
category: Testing & coverage
---

# Merging two valid fixture additions also requires updating union counts

The cache branch added the fixed-point repository-boundary case, and the
upstream branch added the Swift HashMap case. Each side's source-collection
roster pinned 23 cases, so Git merged both additions while retaining stale
count assertions. The merged corpus had 24 source-collection cases, grouped as
7 local, 13 repository, and 4 shared cases; its canonical boundary had 26
rows, 41 inputs, 515 scopes, and 536 authorizations.

After resolving a conflict between independently valid fixture additions,
validate the full merged case-ID union and derive family and canonical summary
counts from that merged authority. Update only count and digest pins that no
longer describe the union; keep each side's fixture and selection guards.
