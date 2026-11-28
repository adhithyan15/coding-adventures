# nand-page-program-cuts (Go)

`nand-page-program-cuts` is a pure model of power loss during one ideal NAND
page program. A page's bytes and its program marker are separate values, so an
all-ones payload can leave the bytes unchanged while consuming the page's
program opportunity. The result also records whether the caller received an
acknowledgement.

This package implements the shared [DB04b model](../../../specs/DB04b-nand-page-program-cuts.md)
and its [Go profile](../../../specs/DB04b1-nand-page-program-cuts-go.md).

```go
before := nandpageprogramcuts.PageState{Bytes: []byte{0xff, 0xff, 0xff, 0xff}}
payload := []byte{0x12, 0x34, 0x56, 0x78}
out, err := nandpageprogramcuts.Apply(4, before, payload,
	nandpageprogramcuts.Cut{Kind: nandpageprogramcuts.AfterPrefix, PrefixBytes: 2})
if err != nil {
	panic(err)
}
// out.State.Bytes is {0x12, 0x34, 0xff, 0xff}; it is programmed but not acknowledged.
```

The call owns no persistent medium and does not mutate its page or payload
inputs. Every successful result owns a new byte slice. Refusals return a zero
result with one stable typed error. Input checks follow the DB04b order:
geometry, page consistency, payload length, cut shape, and existing marker.

The canonical `nand-page-program-cuts.json` trace is read by tests only. Its
bounded reader checks the schema and every expected result against an
independent prefix oracle before replaying the public API. The runtime has no
dependencies outside the Go standard library.

The fixture reader caps input at 16 KiB, 32 cases, and 16 levels of nesting.
History fuzzing accepts at most 4 KiB and executes at most 32 fixed-size page
commands per input.

Run from this module directory with Go 1.26 and `GOTOOLCHAIN=local`:

```sh
go test -count=1 ./... -cover
go test -race -count=1 ./...
go vet ./...
go test -run=^$ -fuzz=FuzzParseFixture -fuzztime=2s -parallel=2
go test -run=^$ -fuzz=FuzzCommandHistory -fuzztime=2s -parallel=2
GO=/path/to/go python3 tools/check_mutations.py
GOOS=linux GOARCH=386 go test -c -o /tmp/nand-page-program-cuts-linux-386.test .
GOOS=windows GOARCH=amd64 go test -c -o /tmp/nand-page-program-cuts-windows.test.exe .
```

The native suite exhausts every payload and cut for page sizes one through
four, checks retries after all 2,272 cases, and compares 5,000 seeded commands
with an independent transition model. The fixture and history fuzz targets
bound both input bytes and command counts. Three disposable source mutations
verify that a persisted suffix byte, a cleared prefix marker, or a premature
acknowledgement is caught by a named test.

## Validation evidence

With Go 1.26.1 and `GOTOOLCHAIN=local`, the 13 native unit/model tests pass
with 100% statement coverage. The race detector and `go vet` pass. The
small-page suite checks all 2,272 payload/cut combinations and 2,272 retry
outcomes. The seeded run with seed `0xDB04B1` checks 5,000 commands, including
458 accepted transitions and 4,542 typed refusals. The 65,536-byte completion
and independent malformed-input tests pass.

Independent coverage-guided runs passed 30,000 fixture inputs and 120,000
command histories. Reproduce them with the listed targets, using
`-fuzztime=30000x` and `-fuzztime=120000x` respectively, with `-parallel=2`.
A separate API probe checked 12,288 cut outcomes with retry and ownership
checks for each case. All tests passed under emulated Linux/386. The
Windows/amd64 test binary cross-compiled. The clean mutation control passed,
and all three mutants failed their named tests.
