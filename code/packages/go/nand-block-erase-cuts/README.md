# nand-block-erase-cuts (Go)

This package models one deterministic SLC NAND block erase with observable
power-loss cuts. A block contains page bytes and independent program markers,
a completed-erase count, and an incomplete-erase flag. It is a pure transition:
it has no device, clock, wear pulse count, or persistent storage.

```go
before := nandblockerasecuts.Block{
    Pages: []nandblockerasecuts.Page{
        {Bytes: []byte{0x12, 0x34}, Programmed: true},
        {Bytes: []byte{0xff, 0xff}, Programmed: false},
    },
}
out, err := nandblockerasecuts.Apply(
    2, 3, before,
    nandblockerasecuts.Cut{Kind: nandblockerasecuts.AfterPrefix, PrefixPages: 1},
)
if err != nil {
    panic(err)
}
// Page 0 is erased, page 1 remains unchanged, the erase is incomplete,
// and no acknowledgement was returned.
```

`Apply` validates page size, the complete block invariant, count ceiling, cut
shape, and finally erase exhaustion. It returns one sentinel error and a zero
outcome on refusal. Successful outcomes own every page byte slice. A partial
cut preserves the suffix and count; a full media erase increments the count,
while `AfterMedia` still withholds acknowledgement. `Complete` performs the
same media transition and returns acknowledgement.

Runtime code uses only the Go standard library. Tests read the exact
`nand-block-erase-cuts.json` vector through a bounded strict JSON adapter and
validate every expected step against a separate page/counter model before
calling the runtime. The adapter caps input at 64 KiB and JSON nesting at 32.
The history fuzzer accepts at most 4 KiB and executes at most 64 commands.

Run from this module directory with Go 1.26 and `GOTOOLCHAIN=local`:

```sh
go test -count=1 ./... -cover
go test -race -count=1 ./...
go vet ./...
go test -run=^$ -fuzz=FuzzParseFixture -fuzztime=2s -parallel=2
go test -run=^$ -fuzz=FuzzBlockHistory -fuzztime=2s -parallel=2
GO=/path/to/go python3 tools/check_mutations.py
```

On Go 1.26.1 Linux/amd64, the native suite reports 100% runtime statement
coverage; `go test -race` and `go vet` pass. The deterministic suite enumerates
every cut and prefix on blocks of one through four pages, checks 156 short
histories, validates width and count extrema, and compares 5,000 seeded
transitions. Two fixed 2-second fuzz campaigns completed 1,046 parser inputs
and 8,233 block-history inputs in the recorded run. Three disposable mutants
were killed by their named assertions: suffix-marker preservation, the
completed-count rule for partial erase, and acknowledgement before protocol
completion. A 386 test binary cross-compiles, but cannot execute on this host.

An independent public client compared 3,144 finite transitions and
ordered refusals. It also erased a 16 MiB logical block and checked that
returned pages remained detached from the input and their siblings.
A separate page-prefix oracle matched all 11 fixture transitions.
The review repeated 45,000 fixture fuzz inputs, 110,000 history inputs,
and all three named mutation controls from a path containing spaces.
The native build passed with 100% statement coverage. Linux 386 tests
passed under QEMU, the Windows amd64 test binary cross-compiled, and
Ruff passed on the mutation runner.
