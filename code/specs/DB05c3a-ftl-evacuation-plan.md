# DB05c3a — Plan an evacuation with an exact spare-page reserve

Use a configured immutable page map and a selected whole block. Plan
copies before changing the map or issuing media commands. Each copy
names the live logical page, its current physical source and one free
physical destination outside the target block.

## First Go profile

Use `code/packages/go/ftl-evacuation-plan`. The Go planning profile accepts `ftlpagemap.Table`, pages per block,
the target block index and an optional exclusion mask with one boolean
per block. The returned plan owns its ordered copy records. The mask
is read only during planning. A zero-length mask allows all blocks.

```go
type Copy struct {
    Logical int64
    SourcePhysical int64
    DestinationPhysical int64
}
type Plan struct {
    Block int64
    FirstPhysical int64
    PageCount int64
    Copies []Copy
}
func PlanEvacuation(table ftlpagemap.Table, pagesPerBlock, block int64,
    excludedBlocks []bool) (Plan, error)
```

Admit the exact DB05c2a page map at `dafcd74db3ba`, with
`page_map.go` SHA-256
`517ea6398a4a96eb9af00b82384a5b0c75bbe3bef3d6bb4d02609499caebc1a5`.
Use its empty snapshot to recognize an unconfigured table.

Validate the page-map geometry and equal-size block divisor, then the
target index, mask size, and absence of a pending write reservation.
Refuse a target that the mask excludes. Require at least one stale
slot in the target before planning. A target of stale and free slots
can give an empty plan. Later policy work can choose a victim using
the measured copy cost and reclaimable count.

Return a zero plan and one stable error on refusal. Reuse
`ftlpagemap.ErrInvalidGeometry` and `ftlpagemap.ErrReservationPending`.
Use `ErrInvalidBlockIndex`, `ErrInvalidBlockMask`, `ErrExcludedBlock`,
`ErrNoStalePage` and `ErrInsufficientCapacity` for the remaining cases.
A successful empty plan names the target block extent and has no copy
records.

Visit live source pages by increasing physical address. In parallel,
visit eligible destination indices by increasing physical address.
The eligible set consists of free pages outside the target and every
excluded block. Pair sources with destinations in those orders.
Refuse insufficient capacity before yielding any partial list. The
caller retains the same map, mask and medium on every outcome.

For a physical block with status `live, stale, live`, two destination
slots must be simultaneously available outside the target. Exactly
two destinations are sufficient. A free slot in the target or in an
excluded block contributes no destination. Any live owner stays visible
at its source until a later executed copy publishes its replacement.

## Evidence

Independently enumerate small reachable maps, all block divisors and
masks, and each target index. Check the exact chosen sources and
destinations, uniqueness and detached values. Test enough and one-short
capacity, all-fenced capacity, mixed user addresses, no-copy blocks,
and pending and signed geometry refusals. Replay a bounded shared
`ftl-evacuation-plan.json` fixture before runtime replay. Cap it at
64 KiB and depth 32 with exact fields and portable integers. Fuzz the
fixture and reachable commands, and demonstrate
deliberate failures for counting the victim's free slots, selecting
a non-free destination and issuing a partial plan before refusal.

DB05c3b will execute the copies against the current device one at a
time. It will recheck source ownership and destination freshness when
it applies each planned copy, especially after an interruption has
spent a destination without publishing a replacement.
