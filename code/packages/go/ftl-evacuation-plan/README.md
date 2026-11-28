# ftl-evacuation-plan (Go)

`ftl-evacuation-plan` builds a deterministic list of live-page copies that
would evacuate one physical block. It reads an immutable `ftl-page-map.Table`
and returns ordered records. The execution step can recheck each source and
destination against the current device before copying.

Sources are visited in ascending physical-page order. Destinations are the
lowest free pages outside the target and every excluded block. A short reserve
returns a zero plan and one stable error. A block of stale and free slots
returns a successful empty plan.

```go
import (
    "fmt"

    ftlevacuationplan "github.com/adhithyan15/coding-adventures/code/packages/go/ftl-evacuation-plan"
    ftlpagemap "github.com/adhithyan15/coding-adventures/code/packages/go/ftl-page-map"
)

table, err := ftlpagemap.New(4, 9)
if err != nil {
    panic(err)
}
for _, logical := range []int64{0, 1, 2, 1} {
    table, _, err = table.Begin(logical)
    if err != nil {
        panic(err)
    }
    table, err = table.Finish(ftlpagemap.MediaReady)
    if err != nil {
        panic(err)
    }
}
plan, err := ftlevacuationplan.PlanEvacuation(
    table,
    3,                         // pages per block
    0,                         // target block
    []bool{false, false, true}, // third block is unavailable
)
if err != nil {
    panic(err)
}
fmt.Println(plan.Copies)
// [{0 0 4} {2 2 5}]
```

Readable retirement uses the same pure enumeration while allowing the source
block itself to be fenced by the allocation ledger. It accepts all-live and
empty targets and still places every copy into the lowest unexcluded free
pages. The caller owns erase eligibility and any subsequent erase.

```go
drain, err := ftlevacuationplan.PlanDrain(table, 3, 0, excludedBlocks)
if err != nil {
    panic(err)
}
fmt.Println(drain.Copies)
```

The runtime and its module manifest depend only on the local `ftl-page-map`
module. A downstream integration test in `ftl-allocation-retirement` composes
the planner with that ledger and `ftl-erase-budget`. The package graph
orders the planner before both consumers. The bounded test adapter reads the canonical `ftl-evacuation-plan.json`, rejects duplicate or
case-aliased keys, wrong token kinds, invalid UTF-8, trailing JSON, depth above
32 and input above 64 KiB. It validates all expected plans and map states with
an independent model before runtime replay.

These commands run from the module directory with Go 1.26.1 and
`GOTOOLCHAIN=local`.

```sh
go test -count=1 ./... -cover
go test -race -count=1 ./...
go vet ./...
go test -run=^$ -fuzz=FuzzParseEvacuationFixture -fuzztime=2s -fuzzminimizetime=512x -parallel=2
go test -run=^$ -fuzz=FuzzPlanHistory -fuzztime=2s -parallel=2
GO=/path/to/go python3 tools/check_mutations.py
```

Native tests compare 20,032 reachable map, block-layout, mask and target
combinations across five small geometries. They also verify exact reserve
boundaries, pending reservation and signed-address precedence, detached empty
and non-empty plans, and the full 64/128 page-map geometry. Three named
mutations prove the tests reject counting target/excluded free pages, choosing
a non-free destination, and returning a partial plan on shortage.

Go 1.26.1 on Linux/amd64 passed native tests, race and vet with 100% runtime
statement coverage. Qualification ran 45,000 fixture fuzz inputs with
`-fuzzminimizetime=512x`, 110,000 reachable-command fuzz inputs, and all three
named mutation controls. The test binary also passed under Linux/386 QEMU and
cross-compiled for Windows/amd64.

The drain extension adds model-first replay of
`ftl-retirement-drain-plan.json`, including an all-live target whose draining,
retired and factory-bad neighbors are fenced. Its composition check marks the
erase-budget assessment exhausted at a zero ceiling and still obtains a valid
copy plan in the downstream ledger integration suite. Exhaustive reachable
map/layout/mask comparisons check 20,032 cases per API. The additional fixture
and command-state fuzz runs completed 45,000 and 110,027 inputs. Three more
disposable source faults check stale-page requirements, destination fencing
and source ordering.

The independent review checked 219,314 calls over 1,478 reachable maps,
all five fixture cases, and a 128-page layout with exactly 15 free spares.
Removing one of those spares produced an atomic refusal. The review reran
155,000 fuzz inputs, all three mutation controls and the lower page-map
suite. The example above ran as a Go program.

The independent drain review enumerated 26,670 reachable maps, then
compared 2,887,954 queries per API across widths, addresses and masks.
The oracle derived copy pairs from a separate model of page ownership
and matched exact refusals, ordered copies and predecessor observations.
Both command lists ran for each package on Linux. Both final test
binaries passed under Linux/386 QEMU and cross-compiled for Windows/amd64.
A real Go resolver check topologically sorted all 11 media manifests
with the composition test in the downstream ledger package.
