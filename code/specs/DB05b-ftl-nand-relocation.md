# DB05b — NAND-backed page relocation

Compose the flash translation map in [DB05a](DB05a-ftl-page-map.md)
with the deterministic page-program transition in
[DB04b1](DB04b1-nand-page-program-cuts-go.md). Store one fixed-width host
page in one physical NAND page. Resolve the reserved mapping using
the media progress modeled by each program cut.

## Device state

Use the DB05a geometry of 1–64 logical pages and 2–128 physical pages,
with at least one extra physical page. Page width is 1–65,536 bytes.
Every physical page starts erased with a clear program marker. A live
mapping references a programmed page. A free page has a clear marker
and all bytes erased. A stale page retains its bytes and set program
marker until a later reclamation step.

Use value-owned pages and a DB05a table. Each write takes the current
device value and returns a successor. Earlier device values, the caller's
payload and detached read or snapshot bytes remain usable. The caller
keeps payload bytes stable for the duration of each write. Lookups read
only the live page. An unwritten logical page is absent.

## Write outcomes

Reserve the lowest free physical address for the logical page. Apply
the requested DB04b1 transition to that physical page. The deterministic
simulator supplies the progress point explicitly. An all-ones payload
and a zero-byte prefix can both leave the bytes equal to the erased
value, so use explicit completion progress as evidence. In particular, AfterPrefix and
AfterMedia can have identical bytes, marker and acknowledgement.

| Program cut | Page state | Map disposition | Host acknowledgement |
|---|---|---|---|
| BeforeAccept | Erased bytes and clear marker | AbortFresh | false |
| AfterPrefix | Exact programmed prefix and set marker | AbortSpent | false |
| AfterMedia | Complete payload and set marker | MediaReady | false |
| Complete | Complete payload and set marker | MediaReady | true |

The two ready outcomes publish the destination and mark an older live
mapping stale in the same successor. Publish an unacknowledged AfterMedia
outcome using the simulation's explicit completion evidence. DB05e will
model what a recovering controller can learn from persistent records
when it lacks that evidence. Garbage collection and block erase
reclamation enter in DB05c. DB04d provides the erase-side evidence
for that later operation.

For width two and three physical pages, complete a write of
`[0x10, 0x20]`, then attempt a replacement of `[0xff, 0xff]` with
AfterPrefix at zero bytes. The old value stays visible and the
replacement page is spent with a set marker. AfterMedia on a following
write at the third page publishes the new value and returns no host
acknowledgement.

## First Go profile

Use `code/packages/go/ftl-nand-relocation`. Admit only the exact reviewed
Go sources for DB05a and DB04b1. Reuse the DB04b1 cut type and error
sentinels. Reuse the DB05a geometry, logical-address and exhaustion
sentinels. The admitted inputs are:

| Package | Verified revision | Runtime source SHA-256 |
|---|---|---|
| `go/ftl-page-map` | `e91580989282` | `0d7c41d99573a6e08732d547132d28b8e6848c95892d102e73c48c95b59d73e6` |
| `go/nand-page-program-cuts` | `11dbaeb25b3` | `6b50bea50d6c28f40452d5316c419f2fa1e2886c15e56780aa49e1f4a8221a4d` |

- `New(logicalCount, physicalCount int64, pageSize int) (Device, error)`
  validates bounded counts first, then page width. The zero device is
  unconfigured.
- `Read(logical int64) (data []byte, present bool, err error)` validates
  geometry and logical address. Absence returns a nil slice and
  `present == false`.
- `Write(logical int64, payload []byte, cut Cut) (Device, Receipt, error)`
  validates device and logical address, then reserves a physical page.
  The page-program transition validates payload length before the cut.
  A refusal returns the original device and a zero receipt, including
  when validation refuses a proposal after an internal reservation.
- `Receipt` has `Physical int64`, `Published bool` and
  `Acknowledged bool`.
- `Snapshot() Snapshot` returns a detached `Map ftlpagemap.Snapshot`
  and `Pages []nandpageprogramcuts.PageState` in physical address order.
  An unconfigured device gives the zero snapshot.

Each successful write clears its internal reservation. A
program refusal after an internal reserve therefore preserves the
original free page and the original live mapping. A refusal from lack
of capacity occurs before checking the payload or cut.

## Manual evidence

- Compare all four cut classes and valid prefix lengths to an independent
  model using distinct logical-to-page ownership and media bytes. Pin
  the clear marker on BeforeAccept and the set marker on both partial
  and complete all-ones writes. Distinguish publication from acknowledgement.
- Test read continuations after each write outcome, retry and
  exhaustion. Check that mapping, slot status, byte contents, markers,
  and refusal precedence agree at every transition. Check detached
  inputs, reads, snapshots and earlier device values.
- Explore a bounded graph of small histories and run seeded multi-address
  traces. Include both rewrite and first-write cases and model
  after-refusal capacity. Use the shared `ftl-nand-relocation.json` vector
  with schema `coding-adventures.database-media.ftl-nand-relocation.v1`,
  capped at 64 KiB and depth 32. Validate the complete independent model
  before runtime replay. Fuzz fixtures and stateful commands.
- Require named mutation witnesses for publishing a partial destination,
  freeing a spent slot, conflating publication with acknowledgement,
  and returning aliased read bytes. Run native tests, race, vet and
  coverage, then add exact fixture routing in a separate item.
