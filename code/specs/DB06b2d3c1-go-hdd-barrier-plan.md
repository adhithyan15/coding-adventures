# DB06b2d3c1 — Plan a sparse HDD barrier before applying effects

A barrier drains the dirty blocks captured at command entry.
Plan its ordered sector writes and their completion receipt
before the cache changes durable data.
[Fixed barrier](DB06b1-hdd-ordered-flush-barrier.md) · [Timed medium](DB06b2d3b-go-sparse-timed-hdd-media.md)

Add the Go package `hdd-barrier-plan`.

```go
type Step struct {
    Block int64
    Cut   hddwritecrashcuts.Cut
}

type Plan struct {
    Steps   []Step
    Receipt hddwritebackcache.BarrierReceipt
}

func Prepare(geometry hddgeometry.Geometry, current hddseektiming.State, dirty []int64, cut hddwritebackcache.BarrierCut) (Plan, error)
```

Admit the geometry first. In input order, validate each dirty block
against the logical geometry and require strictly ascending IDs.
Use `ErrDirtyOrder` for a repeated or descending block.

Reuse the fixed barrier descriptors and their errors.
BeforeFirstSector, AfterAllMedia and BarrierComplete require a zero
sector index and an empty media cut. AtSector requires an ordinal in
the dirty set and a media cut that has no acknowledgement. Preserve
the exact lower Describe error for an invalid selected media cut.
A refusal always returns zero Plan.

BeforeFirstSector returns a successful empty plan with zero receipt.
AfterAllMedia and BarrierComplete plan acknowledged writes for all
dirty blocks in order. Their receipts report that count.
BarrierComplete also acknowledges the barrier. AtSector plans
acknowledged writes for the preceding blocks and the selected
media cut for the terminal block. Its receipt counts only preceding
blocks and preserves the exact terminal prefix. This path does
not acknowledge the barrier. Empty plans use nil Steps.

Before returning a nonempty plan, check the service timing of each
planned block in order, carrying the full service end state forward
from earlier completed sectors. The final step can be a terminal
cut and uses the medium's full-service admission rule. Preserve
any timing error. Timing and cut validation produce no device
effects. Copy any written-prefix pointer retained in the returned
plan. The later cache applies the plan within the same exclusive
device interval.

Compare dirty-set orders, empty barriers and every cut in finite
geometries. Check every offset, every prefix, timing exhaustion
at a later planned block, exact lower errors and detached output.
Run both build lists, race, vet, guided inputs and clean controls
for visiting one block after a terminal cut and acknowledging
the barrier after all media with a lost response.

## Local validation

Commit `514875143d3` matched 19,968 independently stepped plans on
32- and 512-sector geometries. The model enumerated every subset of
eight addresses, every terminal byte-prefix cut, and complete and
lost-ack barriers. It compared exact step order, detached cut prefixes,
receipts and the first refusal if later service timing overflows.

The Linux and Windows command lists reached 98.2% statement coverage
on Linux. The race detector, vet, Linux/386 replay and 100,000 guided
inputs passed. Both mutation controls failed at their named witnesses
after the clean model passed. Build discovery added exactly five edges
and preserved all prior recipes.
