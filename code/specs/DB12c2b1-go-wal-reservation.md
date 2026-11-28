# DB12c2b1 — Capture a fixed WAL reservation through page I/O

Capture and classify a fixed WAL reservation for append and recovery.
Both callers need the same geometry, page width, ordered block reads and
scan results. Keep the decision to use a classified damaged tail in the
individual caller's recovery policy.
[WAL append](DB12b3-go-wal-append.md) · [WAL scan](DB12b2-go-wal-scan.md)

Add the Go package `storage-wal-reservation`.

```go
const MaxLogPages int64 = 1_024

func Capture(
    device blockdevicecontract.Device,
    layout storagepagelayout.Layout,
) (storagepageio.Pager, storagewalscan.Scan, error)
```

Open the pager and preserve its first refusal. Reject a reservation beyond
`MaxLogPages` with `ErrLogCapacity`. Reject a WAL slot width outside the
admitted slot domain with its exact page-size error. Read the reservation
through the pager in ascending page order, stopping on its first error.
Scan the captured pages and preserve its first hard refusal. Every error
returns a zero pager and scan. Successful capture returns the checked
pager and the classified scan. Its record images are detached.
`Capture` reads blocks and performs no writes or barriers.

Route the existing `Append` preflight through the shared capture.
Keep the append package's public capacity constant, error identity,
predicate order, typed damaged-tail error, target position and receipts.
After capture, append refuses full and damaged tails and checks the next
record LSN as before. Its single page write and barrier still use the
pager that captured the reservation. The device remains sequential and
exclusive to the caller for the operation.

Compare against a scripted reader over nonzero block offsets and several
page widths. Check all read failures, unexpected block widths, scope,
success at the reservation boundary, broken record ranks, insufficient
capacity and damaged tails with a blank suffix. Retain the exact append
tests and rerun its independent HDD cuts. Check detached images, coverage,
both build lists, race, vet, guided inputs and clean controls for stopping
the capture at the first vacant page or reading outside the reservation.

## Local verification

The implementation and append refactor are in `62ae519af87`. An
independent scripted reader matched 360 read and short-block refusals
across all block positions at five log widths. It checked bounds before
reads, failure order after an early damaged page, detached record images,
and the distinction between a classified damage and a hard safety
refusal. An independent caller replayed the 66 existing HDD append cuts
and checked shared capacity error identity. Race and vet passed.

Both command lists passed on Linux for both packages at 100% statement
coverage with race and vet. Module tidy passed. Each package completed
100,000 guided inputs and rejected both of its mutation controls.
Build discovery added only the reservation, its eight platform edges
and the incoming append edge. All existing recipes matched.
