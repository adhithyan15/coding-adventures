# hdd-simulator (Go)

A dependency-free, in-memory implementation of DB02's fixed deterministic HDD
profile. One disk has 32 eight-byte sectors, four track coordinates, and an
integer clock. `ReadSector` and `WriteSector` synchronously apply exact seek,
rotation, and transfer costs.

```go
package main

import (
	"fmt"

	hddsimulator "github.com/adhithyan15/coding-adventures/code/packages/go/hdd-simulator"
)

func main() {
	disk := hddsimulator.New()
	result, err := disk.ReadSector(0)
	if err != nil {
		panic(err)
	}
	fmt.Printf("read %d bytes in %d ticks\n", len(result.Data), result.Timing.TotalTicks)
}
```

Read results and snapshots are detached copies. Writes copy an exact eight-byte
slice. Errors are stable sentinels and can be checked with `errors.Is`. The
portable clock ceiling is `2^53-1`. Calls on one device must be serialized by
the caller; this package provides no internal lock or concurrent-operation
contract.

See [DB02](../../../specs/DB02-hdd-simulator.md) and the [Go profile](../../../specs/DB02b-hdd-simulator-go.md)
for geometry, timing, refusal order, ownership, and test requirements. The
runtime has no JSON or other third-party dependency.

## Validation

Run from this module directory with Go 1.26:

```sh
GOTOOLCHAIN=local go test -count=1 ./...
GOTOOLCHAIN=local go test -race -count=1 ./...
GOTOOLCHAIN=local go test -cover ./...
GOTOOLCHAIN=local go vet ./...
GOTOOLCHAIN=local go test -run=^$ -fuzz=FuzzMixedCommandStream -fuzztime=2s
GOTOOLCHAIN=local go test -run=^$ -fuzz=FuzzFixtureReader -fuzztime=2s
```

The ordinary tests replay the canonical trace and run saved fuzz seeds. A
tick-by-tick reference model checks 512 timing states, 3,584 near-limit read
and write outcomes, 4,680 short histories, and 5,000 seeded commands. The
fixture reader runs inside the test package.
