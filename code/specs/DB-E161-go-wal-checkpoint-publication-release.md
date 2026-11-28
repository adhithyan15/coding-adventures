# DB-E161 — Publish physical Go WAL checkpoints

Publish reservation validation, ordered capture and three-step
checkpoint publication after the checkpoint codecs and selection
have merged. These packages establish the placement and publication
guarantees that checkpoint initialization and reset will use.
[Placement](DB12d2c-go-wal-checkpoint-layout.md) · [Capture](DB12d2d-go-wal-checkpoint-capture.md) · [Publication](DB12d2e-go-wal-checkpoint-publication.md)

Use the exact first admitted package bytes from `fbfbeb5c193`,
`2820934555a` and `f49ddef76ec`. Place each specification before its
implementation in the release branch. Include one package at each
boundary. Capture reads and selects the two physical pages. The
publisher preserves the current frame while advancing the other
page through three acknowledged barriers. Bootstrap and reset have
separate acceptance contracts and release slices.

Reconcile each declared dependency with fetched `origin/main`. Run
both standalone build command lists in a fresh release archive,
plus race, vet, tidy and the full manifest metadata suite. Recheck
the exact archived models and named mutation controls for geometry
shape, physical read order, padding, barrier sequencing and receipt
confirmation. Verify that native Linux and Windows jobs build all
three packages. Confirm both required gates before auto-merge and
compare the merged scope with the reviewed tree.
