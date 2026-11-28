# DB05a — Flash translation page map and reservations

Model the metadata transition the flash controller uses when it relocates
a host page. Start with [DB01](DB01-nand-flash.md) and the distinction
between media completion and acknowledgement in
[DB04b](DB04b-nand-page-program-cuts.md) and
[DB04d](DB04d-nand-block-erase-cuts.md). This slice takes an explicit
media outcome and computes a new mapping. DB05b supplies those outcomes
through an admitted device adapter.

## State

Use 1–64 logical pages and 2–128 physical pages with at least one extra
physical page. Addresses are zero-based. Each physical page is free,
reserved, live or stale. The table maps each previously completed
logical page to one live physical page. At most one reservation can be
pending. A reserved page is excluded from reads and cannot be reserved
again. Every live page has exactly one logical owner. Stale pages stay
unavailable until a later reclamation operation.

A new table has no mappings and every physical page is free.

## Transitions

- **Look up a logical page.** Return its active physical address, or
  indicate that it is absent. An active reservation leaves lookups
  directed to the previous address.
- **Begin a write.** Reserve the lowest available physical page for the
  specified logical address. Refuse a second outstanding reservation
  or a reservation when no free page remains.
- **Finish a write.** Use one explicit disposition from the table below.
  Every accepted finish clears the outstanding reservation.

| Disposition | Destination | Previous location and mapping |
|---|---|---|
| `media_ready` | Mark live | Point the logical page to the destination and mark an existing previous page stale |
| `abort_fresh` | Mark free | Preserve both |
| `abort_spent` | Mark stale | Preserve both |

For a logical page with three physical pages, begin and finish a write
at page 0. A second reservation at page 1 leaves page 0 visible. Marking
page 1 spent preserves that mapping. A completed write at page 2 then
makes page 0 stale. The next reservation refuses until a later slice
reclaims a page.

The caller supplies evidence that a destination is fresh, spent or
ready. A spent destination may contain a partial program. A ready
destination may have completed at the media before the host received
acknowledgement. Ownership and acknowledgement are separate concerns
in the later controller adapter.

## First Go profile

Use `code/packages/go/ftl-page-map` and the standard library. Use private
value-owned state. `New(logicalCount, physicalCount int64) (Table, error)`
constructs a configured table. The zero table is unconfigured.

- `Lookup(logical int64) (physical int64, present bool, err error)`
  gives the currently live physical address. Absent results return zero
  with `present == false`.
- `Begin(logical int64) (Table, Reservation, error)` returns the next
  table and the selected logical and physical addresses.
- `Finish(disposition Disposition) (Table, error)` resolves the
  active reservation.
- `Snapshot() Snapshot` returns a detached copy of the logical map,
  physical status, and optional reservation.

Use stable refusal classes `invalid_geometry`, `invalid_logical_page`,
`reservation_pending`, `no_free_page`, `no_reservation`, and
`invalid_disposition`. Validate configuration first. Begin validates
logical address, then reservation state, then free capacity. Finish
validates reservation state before disposition. Refusals return the
original table and a zero additional result.

## Manual evidence

- Enumerate reachable states to a fixed point for logical and physical
  geometries 1/2 and 2/3 using an independent map and slot model. Check
  every valid command and representative invalid addresses and outcomes
  from each reached state.
- At every state check live ownership, exclusion of reserved pages from
  lookup, reservation count, and exact partitioning of physical pages.
  Check refusal atomicity, lowest free selection and detached state.
- Validate a bounded shared `ftl-page-map.json` vector through the
  independent model before runtime. Include rewrite, abort with spent
  media, abort with fresh media and exact exhaustion.
- Add seeded stateful comparisons and fixture and history fuzzing.
  Require named failures for early mapping publication, reuse of a
  spent page, and failure to invalidate an old live slot. Run native
  tests, race, vet and coverage. Route the exact fixture separately.
