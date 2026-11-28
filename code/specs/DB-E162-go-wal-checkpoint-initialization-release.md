# DB-E162 — Publish Go checkpoint initialization and WAL clearing

Add the operations that establish the first Ready frame and clear an
old WAL under a durable Resetting selection. Publish these operations
before the restart and advancement package composes them.
[Physical checkpoint release](DB-E161-go-wal-checkpoint-publication-release.md) · [Raw checkpoint pages](DB12d3a1-go-wal-checkpoint-raw-pages.md) · [First Ready frame](DB12d3a2-go-wal-first-checkpoint.md) · [Clear](DB12d3b-go-wal-checkpoint-clear.md)

Reconcile with the merged physical checkpoint release. Deliver raw
capture from `c61a1636210`, first Ready publication from `13e4ead0fb9`,
and guarded clearing from `9ff5525da24`. Preserve the corresponding
existing package trees outside their reviewed changes. Place each
specification before its implementation in the release branch.

Run both build lists and exact archived gate checks for the three
packages. Retain raw frame and padding cases, bootstrap cut histories,
write-free lost-acknowledgement retries and repeated cuts during clear.
The clear operation must preserve both checkpoint pages and the data
reservation. Check all manifest metadata and the resolved Go closure.
Verify the relevant native build jobs and both required gates before
auto-merge. Compare the merged scope and file bytes with the reviewed
head.
