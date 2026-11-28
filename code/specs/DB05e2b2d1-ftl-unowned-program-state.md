# DB05e2b2d1 — Record an unowned programmed page in the translation table

Extend the Go page map with one pure observation of consumed media.
The later metadata writer needs to keep a programmed physical page
out of allocation while every logical mapping stays in place. The
relocation device already represents consumed pages with no owner as
`Stale`. It rejects a programmed page whose table entry is `Free`.
[Page-map contract](DB05a-ftl-page-map.md) · [Relocation device](DB05b-ftl-nand-relocation.md)

```go
func (table Table) RecordUnownedProgram(physical int64) (Table, error)
```

Validate table geometry, then the signed physical address, then the
absence of any pending allocation. Use `ErrInvalidGeometry`,
`ErrInvalidPhysicalPage` and `ErrReservationPending` in that order.
Require a `Free` target or return `ErrPhysicalPageUnavailable`.
Each refusal returns the original table. An accepted transition marks
the target `Stale` and preserves all other physical entries and logical
owners. The next allocation skips that page. Applying the existing
`ReleaseErased` transition with completed-media evidence makes it
available again. The caller supplies evidence that the program consumed
the physical page. This primitive models page-map state only.

Check all physical states, pending collisions, signed address extremes,
and preservation of both old and new tables. Compare with a separate
finite state model. For every reachable released table and free target
in the small domain, compare the outcome with an exact-position
reservation finished using `AbortSpent`. Exercise refusal and retry,
allocation skipping and completed erase reuse in seeded histories.
Use fixed fuzz budgets, named mutation controls,
both command lists, race, vet and architecture checks.

## Go verification

Independent breadth-first replay matched 223,940 decisions. The
small-state domain covered one through three logical pages, up to six
physical pages, and reachable tables through six commands. It accepted
10,130 unowned program observations and matched 213,810 typed refusals.
Every accepted table matched an exact-position reservation finished as
spent. The observer checked retained tables, skipped allocation and
reuse after a completed erase, including the largest geometry.
The runtime SHA-256 is
`bf203bd24ea478255a16d4b13d1b93e581493501777ccab8067c67777d565d95`.

Both command lists passed on Linux with 100% statement coverage, race
and vet. The history fuzzer passed 100,000 inputs and all 12 mutation
controls reached their intended assertions. Linux/386 ran in QEMU and
the Windows/amd64 test binary compiled. Native Windows execution is
pending. The emitted recipe passed in a clean path with spaces, and
all 5,974 graph records and 5,973 Linux recipes matched the prior export.
