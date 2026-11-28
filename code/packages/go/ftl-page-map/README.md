# ftl-page-map (Go)

This package models one pure flash-translation table. It assigns a logical
page to a physical page only after the caller reports that the reserved media
is ready. A fresh abort releases a reservation; a spent abort leaves a stale
slot unavailable for reuse. NAND operations and reclamation are subsequent
controller layers.

```go
table, err := ftlpagemap.New(1, 3)
if err != nil {
    panic(err)
}
table, reservation, err := table.Begin(0) // lowest free page: physical 0
if err != nil {
    panic(err)
}
_ = reservation
table, err = table.Finish(ftlpagemap.MediaReady)
if err != nil {
    panic(err)
}
physical, present, err := table.Lookup(0)
// physical == 0, present == true
```

The table uses private value-owned slices. Methods that advance it return a
new table. A refusal returns the prior value. `Snapshot` returns detached map,
slot-status and reservation values. A pending destination is not visible to
`Lookup`; a ready rewrite publishes the new slot and stales the previous one
in the same transition.

`BeginExcluding` adds a per-call allocation fence. A nil or empty mask keeps
`Begin` behavior. A nonempty mask must match the physical page count, and the
lowest free page whose bit is false is reserved. A wrong-sized nonempty mask
returns `ErrInvalidFence` after the logical-address and pending-reservation
checks. `ReleaseErased(first, count)`
releases a nonempty in-bounds half-open extent only when all of its slots are
free or stale. Invalid signed bounds return `ErrInvalidExtent`; a live or
reserved page anywhere in a valid extent returns `ErrIneligibleExtent` without
changing any page. It preserves mappings, live pages, and reservations outside
the extent. The media adapter must establish full erase completion and block
alignment before it calls this metadata-only operation.

Runtime imports use only the Go standard library. Tests read the exact
`ftl-page-map.json` vector through an exact-key, exact-token-type,
duplicate-key-rejecting JSON adapter capped at 64 KiB and depth 32. This keeps
null array values, missing reservation fields, and case-insensitive JSON field
aliases from becoming scalar zero values. The fixture's complete declared
trace is checked against a separate map-and-slot model before any runtime
replay.

Run in this module directory with Go 1.26.1 and `GOTOOLCHAIN=local`:

```sh
go test -count=1 ./... -cover
go test -race -count=1 ./...
go vet ./...
go test -run=^$ -fuzz=FuzzParseFixture -fuzztime=2s -parallel=2
go test -run=^$ -fuzz=FuzzPageMapHistory -fuzztime=2s -parallel=2
go test -run=^$ -fuzz=FuzzParseReclamationFixture -fuzztime=2s -parallel=2
go test -run=^$ -fuzz=FuzzReclamationHistory -fuzztime=2s -parallel=2
GO=/path/to/go python3 tools/check_mutations.py
```

The deterministic suite closes the combined state space for geometries 1/2
and 2/3, checking every allocation mask and every in-bounds erased extent at
each reachable state. It also checks signed invalid endpoints, all-fenced
capacity, validation order, complete refusal atomicity and pending-reservation
preservation. The original page-map fixture and the new
`ftl-page-reclamation.json` vector are independently model-checked before
runtime replay. The new 23-step vector covers fenced selection, stale-prefix
release, release during an outside reservation, lowest-page reuse and
all-fenced, invalid-mask and stale-before-live refusals. Seeded comparisons
cover 8,000 commands across fresh page-map histories.

Six disposable mutations cover the original mapping transition and all three
reclamation guarantees: honor a fence, refuse an ineligible extent atomically
after an eligible prefix, and preserve an outside reservation.

Go 1.26.1 on Linux/amd64 passed native tests, race and vet with 100%
runtime statement coverage. The independent review used a package path
with spaces, ran Linux/386 under QEMU and cross-compiled a Windows/amd64
test binary. It ran 45,000 fixture fuzz inputs and 110,000 history fuzz
inputs and reproduced all three mutation controls.

An external allocation-set model matched 65,536 transitions, including
4,038 published writes and all refusal classes. It checked the maximum
geometry and verified detached snapshots and preservation of earlier
table values. A separate model matched all 12 fixture steps and reached
the same 9 and 46 states for the two small geometries.

The reclamation extension closes 14 states for geometry 1/2 and 128 states
for 2/3, evaluating every mask and every in-bounds extent at each state. On
Go 1.26.1/Linux amd64, the expanded suite passed with 100% statement coverage,
race and vet. Linux/386 tests passed under QEMU and the Windows/amd64 test
binary cross-compiled. The new fixture fuzzer completed 45,000 inputs and the
history fuzzer completed 110,000 inputs. All six disposable mutations passed
their clean control and failed at their named witnesses. The fixture digest
and exact run logs are recorded with the DB05c2a review receipt.

The independent review used reverse physical ownership to compare
171,986 commands over geometries 1/2, 2/3 and 3/4. It replayed all
23 fixture transitions and tested 128 concurrent branches from one
64/128 predecessor under the race detector. Native checks covered
100 percent of runtime statements. The Linux/386 suite ran under
QEMU and the Windows/amd64 suite compiled. The existing relocation
and erase-victim suites both passed against this page map.
