# DB05e2b2e2a — Separate erase candidates from copy destination capacity

Reuse the Go copy-aware victim score with independently supplied
block fences. One fence controls which blocks can be selected for
reclamation. The other controls which free pages can count as copy
destinations. A block can have no remaining erase budget and still
contain a usable free page. [Victim policy](DB05c3c-ftl-copy-aware-selection.md) · [Erase budget](DB05d1-ftl-erase-budget.md)

```go
func SelectCopyAwareWithFences(status []ftlpagemap.PhysicalStatus,
    pagesPerBlock int64, victimExcluded, destinationExcluded []bool,
) (CopyAwareVictim, bool, error)
```

Keep the existing geometry, mask length, physical-status and pending
reservation validation in that order. Each mask may be empty or have
exactly one bit per block. Exercise both mask lengths independently.
Accepted calls leave all three input slices unchanged. Keep
`SelectCopyAware` as the existing entry point with its supplied mask
applied to both roles.

Consider only allowed victim blocks containing at least one stale
page. For each candidate, count free pages in destination-allowed
blocks other than the candidate itself. Refuse that candidate when
the external capacity is below its live page count. Rank eligible
candidates by the existing exact ratio `stale / (live + 1)`, then by
larger stale count, then by lower block ordinal. Expose the same
victim fields with the independently computed free destination count.

As a witness, use width two and a first block containing stale and
live pages. A second block contains one free and one live page, and
a third contains two live pages. Exclude the second block only as a
victim. Its spare page makes the first block eligible. A shared
exclusion for both roles removes that spare and yields no victim.

Use an independent model that counts destinations per candidate.
Cover distinct, equal and overlapping masks, an excluded destination
candidate's own free pages, all missing capacity, stale-only victims,
ties, bad statuses, pending reservations, signed bounds and retained
inputs. Compare the existing entry point under its shared mask.
Run both command lists, exhaustive and seeded checks, fixed fuzz,
clean-controlled mutations, race, vet, architecture and emitted builds.

## Go verification

The independent evaluator counted destinations per candidate and
ranked exact rational scores. It matched 2,304,575 decisions, including
1,443,625 selections and 860,190 layouts with no eligible victim.
It also matched 187,457 exact shared-mask calls and checked input
retention. The runtime SHA-256 is
`e250c5e9996f42d8c2af8b94c7f423ef2ed3dc4a8ea8b40e7c2bcd67bc080a39`.

Both command lists passed on Linux with 100% statement coverage,
race and vet. The split and shared-mask guided targets each completed
100,000 inputs. The clean control and all seven copy-aware mutations
passed. The Linux/386 suite ran in QEMU and the Windows/amd64 binary
compiled. Native Windows execution remains pending.

The emitted recipe passed from a path with spaces. All 5,978 package
records and 5,977 Linux recipes matched the earlier export.
