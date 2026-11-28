# DB05d2c6b2a — Invalidate one exact current page mapping

An explicit loss disposition needs a settled map transition that removes
one logical owner and retains its physical page as stale. Add the
transition to the existing Go page map. A higher layer supplies the
validated identity and loss record before using this transition.
[Loss claim contract](DB05d2c6b1-ftl-loss-claim.md)

```go
func (table Table) InvalidateCurrent(logical, physical int64) (Table, error)
```

Check configured geometry, signed logical address, signed physical
address, pending reservation and exact current mapping in that order.
Use the existing geometry, logical and pending errors. Add
`ErrInvalidPhysicalPage` for invalid physical coordinates and
`ErrCurrentOwnerChanged` for an absent or different current owner.
Every refusal retains an equivalent predecessor with its reservation.

On success remove only the specified logical mapping. Change its old
physical slot from live to stale. Keep all other mappings and slot
states. The result owns its modified metadata and the predecessor stays
observable. The removed slot remains unavailable to `Begin` until a
caller supplies a completed erase to the existing `ReleaseErased`.
A new write may publish a different owner for the same logical address.

Enumerate bounded reachable maps with reservations, media-ready and
aborted writes, explicit invalidation and completed erase. Compare
exact mappings and physical state with a separate model. Cover signed
boundaries and compound refusals, initial absence, stale coordinates,
retained tables and independent snapshot storage. Run both command
lists, race, vet, coverage, fixed fuzz and clean mutation controls.
Run native Linux/386 and compile the Windows test target when available.
Check the existing relocation and retirement callers.

A logical and physical pair may repeat after an erase. The relocation
and loss adapter compose this lower transition with the exact owner
observation before removal. Durable loss records and recovery belong
to the mapping publication protocol in DB05e.

## Local verification

Independent closed-state checks covered 2,298 distinct states in five
geometries and 83,556 commands. They checked 21,088 direct invalidations,
including 704 successes, and 18,384 signed-address invalidation probes.
Every state was rechecked after altering its detached observed map
and reservation. In a fully occupied 64-logical, 128-physical map,
removing the final owner changed only its mapping and slot. The
allocator accepted that slot after explicit complete-erase evidence.

The native suite retained the prior history decoder and exercised
invalidation in its two finite closures and a 4,000-step seeded run.
Both command lists passed on Linux with 100% statement coverage, race
and vet. The new history target covered 100,000 fuzz inputs, and all
nine mutation controls reached their declared witnesses. Linux/386 ran
under emulation and the Windows amd64 binary compiled. The relocation
and retirement-controller builds remained green.

The integrated page-map, relocation, fault-report and retirement
builds passed. All 5,962 package records and 5,961 Linux command lists
matched the prior graph export.
