# DB05c1 — Select an erase victim from flash page status

Find a physical block that can be erased without moving a live page.
Use a detached physical-status vector from the
[DB05a page map](DB05a-ftl-page-map.md) and the whole-block erase
granularity in [DB04d](DB04d-nand-block-erase-cuts.md).

## Policy

Partition physical addresses into equal contiguous blocks. A reclaimable
block contains at least one stale page. Every page in that block is stale
or free. Reserved and live pages make the block ineligible. Pick the
reclaimable block with the most stale pages. Resolve ties by lowest block
index. Return its block index, first physical page, number of pages
and stale-page count. The planned extent includes already free pages
as well as the stale slots.

The planner observes the input vector and leaves it unchanged. A later
adapter will repeat eligibility checks at the erase boundary and apply
DB04d's media outcome. That layer will publish reusable slots after a
full erase and quarantine a block during an incomplete erase. Live-page
evacuation will extend the policy with separately admitted relocation
and spare-space rules.

## First Go profile

Use `code/packages/go/ftl-erase-victim` and reuse the `PhysicalStatus`
values from the verified DB05a package at revision `e91580989282`,
whose `page_map.go` SHA-256 is
`0d7c41d99573a6e08732d547132d28b8e6848c95892d102e73c48c95b59d73e6`.
The caller keeps the input slice stable while planning.

- `Select(status []ftlpagemap.PhysicalStatus, pagesPerBlock int64) (Victim, bool, error)`
  accepts 2–128 physical pages. Pages per block must be positive, at most
  the physical page count and divide that count exactly.
- `Victim` has `Block int64`, `FirstPhysical int64`,
  `PageCount int64` and `StalePages int64`. No eligible block returns
  the zero victim with `found == false` and a nil error.
- Return `ErrInvalidGeometry` before checking status values when the
  geometry is invalid. Accept the four DB05a values and return
  `ErrInvalidStatus` for every other raw value, including in an
  ineligible block. Return the zero victim and `found == false` on refusal.

## Manual evidence

- Exhaust all 19,360 status-vector and block-width combinations. Use
  every vector with 2–6 slots and every valid block width.
  Compute all eligible blocks by an independent model and compare the
  chosen range, benefit and deterministic ties.
- Test invalid signed widths, inconsistent geometry, unknown status
  values, maximum geometry and input preservation.
- Model-check the shared `ftl-erase-victim.json` vector before runtime
  replay. Use schema `coding-adventures.database-media.ftl-erase-victim.v1`,
  capped at 64 KiB and nesting depth 32. Exercise free blocks, live and reserved exclusions,
  exact ties and the last block's physical extent. Add bounded fixture
  and status-vector fuzzing.
- Use named controls for admitting a live or reserved block, scoring
  free slots as reclaimable and breaking ties in reverse order. Run
  native tests, race, vet and coverage. Route the fixture independently.
