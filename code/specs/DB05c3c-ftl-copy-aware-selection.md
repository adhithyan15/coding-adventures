# DB05c3c — Rank flash victims by recovered pages and copy cost

Add a second pure selection policy to the admitted Go `ftl-erase-victim`
package. Retain the stale-only policy, its typed refusals and its tie order.
The new policy receives a current physical-page snapshot and a list of
excluded blocks from the caller. The caller can exclude an incomplete
erase, an active evacuation or a block at its wear ceiling.

Use the admitted stale-only selector at `0872a35535ea`. Its
`victim.go` SHA-256 is
`d05fa7151bce7077254007b82ff02a19ab874ed2dfdcdbe4063d4d8bc170f091`.
Compose through the planner at `a9a6616bbb9f` and the Go relocation
controller at `e733d2dbb5fd`.

```go
func SelectCopyAware(status []ftlpagemap.PhysicalStatus, pagesPerBlock int64,
    excludedBlocks []bool) (CopyAwareVictim, bool, error)

type CopyAwareVictim struct {
    Block            int64
    FirstPhysical    int64
    PageCount        int64
    StalePages       int64
    LivePages        int64
    FreeDestinations int64
}
```

Match the existing aligned geometry and enum contracts for 2–128 physical
pages. Check geometry, then check that a nonempty mask has one entry per
block, then check physical status kinds. Use `ErrInvalidBlockMask`
for the mask and the existing `ErrInvalidStatus` for an unknown role.
When all roles are valid, a pending reservation in any
physical slot refuses copy-aware selection with
`ftlpagemap.ErrReservationPending`. Empty mask excludes nothing. Refusals
preserve both input slices and return a zero victim and false.

For every nonexcluded block with at least one stale page, count currently
eligible free destinations outside it and all excluded blocks. Include the
block only if free destinations can cover every live page in the block.
Return a zero victim and false when no block qualifies.

## Integer comparison

A successful evacuation reclaims S net free slots when its block has S
stale pages. It consumes L spare pages to move L live values then gains
S+L free slots when the old block is released. Count both metrics.

Select the block with the largest ratio `S / (L + 1)`. Compare cross
products using the bounded integers instead of rounding. Resolve equal
ratios by larger S, then lowest block index. An equal positive ratio
and equal S imply the same L. The two
policies agree on an input where every candidate is stale-only, no block is excluded, and the snapshot has
no reservation. Keep the score in the policy rather than storing it
in the map or durable state.

## Manual verification

- Compare with an independent enumerator over small role arrays and
  masks, including full, exact-spare, one-short, all excluded, reserved,
  invalid role and every tie branch. Record the explored domain.
- Compose the choice with the exact admitted DB05c3a planner and the
  DB05c3b2 controller on reachable traces. Put composition tests in
  the relocation package, which already imports both planning packages
  in tests. Confirm the returned copy
  and spare counts, earliest eligible destinations, eventual net free
  slots and unchanged logical values after successful reclamation.
- Compare the two policies on a fixed workload with mixed live/stale
  blocks. Record refused copies, spent destinations, completed copies
  and recovered slots by policy. Keep wear ranking as a separate item.
- Use the portable `ftl-copy-aware-victim.json` vector with schema
  `coding-adventures.database-media.ftl-copy-aware-victim.v1`. Check
  64 KiB, depth 32, exact token kinds and portable integers. Compare
  every declaration independently
  before runtime replay and make the accepted fuzz path exercise the
  runtime. Keep all existing selector checks and exact inputs.
- Run bounded exhaustive and seeded checks. Exercise 45,000 fixture and
  110,000 role-array fuzz inputs and distinct mutation controls for excluding the wrong block, counting target
  free pages as external spares, reversing the ratio and changing ties.
  Record native, race/vet, cross-target and exact dependency evidence.

Keep the existing selector runtime and fixture contract. The relocation
runtime and its four shared vectors stay at the admitted versions.
