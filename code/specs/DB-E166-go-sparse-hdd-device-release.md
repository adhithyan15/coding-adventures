# DB-E166 — Publish the sparse timed Go HDD block device

Compose exact seek timing and torn-sector effects with a volatile
write-back overlay. The common block contract exposes reads, writes
and durability barriers to the page and WAL stack.
[Media](DB06b2d3b-go-sparse-timed-hdd-media.md) · [Position](DB06b2d3b1-go-hdd-media-position.md) · [Barrier plan](DB06b2d3c1-go-hdd-barrier-plan.md) · [Device](DB06b2d3c2-go-sparse-hdd-write-back.md)

After DB-E165, publish the sparse timed medium at `dcc4113ea89`,
its position extension at `46bab963a69`, the pure barrier planner
at `514875143d3`, and the block adapter at `d945343ead1`. Carry
its own specification before each implementation change. Keep all
fixture paths inherited from the merged lower disk packages.

Reconcile the exact dependency closure with fetched main. Run both
native lists, race, vet, tidy, guided inputs, named mutation controls,
and the exact metadata suite on the release tree. Check that a later
clock refusal leaves the complete volatile and durable state intact.
Verify real 512-sector page I/O, restart, exact cut receipts and their
surviving image. Confirm both named gates and the emitted recipes
before conditional auto-merge and compare the verified merged scope.
