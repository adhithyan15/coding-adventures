# nand-flash (Go)

A dependency-free logical model of full-page, single-program SLC NAND media,
following [DB01](../../../specs/DB01-nand-flash.md) and its [Go profile](../../../specs/DB01b-nand-flash-go.md).
It demonstrates why a page cannot be rewritten in place and why erasure happens
at block granularity. It is an in-memory teaching model, not a controller, FTL,
wear-leveler, ECC implementation, filesystem, or database.

```go
package main

import (
	"fmt"

	nandflash "github.com/adhithyan15/coding-adventures/code/packages/go/nand-flash"
)

func main() {
	device, err := nandflash.New(nandflash.Config{
		Geometry: nandflash.Geometry{Blocks: 4, PagesPerBlock: 16, PageSizeBytes: 512},
	})
	if err != nil {
		panic(err)
	}
	page := make([]byte, 512)
	if err := device.Program(0, 0, page); err != nil {
		panic(err)
	}
	copyOfPage, err := device.Read(0, 0)
	if err != nil {
		panic(err)
	}
	fmt.Printf("read %d bytes\n", len(copyOfPage))
}
```

The device copies the optional `MaxEraseCycles` value and each program payload.
Reads and state snapshots are detached copies, so later caller mutation cannot
change stored pages. A nil `MaxEraseCycles` means no configured ceiling; the
portable DB01 safe-counter ceiling still applies. Operations on one device must
be serialized by the caller. Independent devices own independent state.

Returned errors are stable sentinels and can be checked with `errors.Is`, for
example `errors.Is(err, nandflash.ErrProgrammed)`. Their messages match the
shared DB01 trace. `Snapshot` returns lowercase-hex pages, per-block erase
counts, and program-once markers as a detached JSON-compatible value.

## Validation

Run these commands from this module directory with Go 1.26.1:

```sh
GOTOOLCHAIN=local go test -count=1 ./...
GOTOOLCHAIN=local go test -race -count=1 ./...
GOTOOLCHAIN=local go test -cover ./...
GOTOOLCHAIN=local go vet ./...
GOTOOLCHAIN=local go test -run=^$ -fuzz=FuzzCommandStream -fuzztime=2s
```

The normal test suite replays the canonical JSON trace with a strict bounded
test-only reader, checks an independent deterministic model, enumerates short
histories, and runs the saved fuzz seeds. Runtime code does not read files or
parse JSON.
