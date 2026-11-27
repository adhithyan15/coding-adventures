# Go HDD write-back cache

`hdd-writeback-cache` models a 32-sector, eight-byte-per-sector disk with a
volatile write overlay. The zero value is a fresh disk. `WriteSector` copies a
sector into the pending overlay, and reads prefer pending bytes while a sector
is dirty. `Snapshot` returns fixed arrays containing both the durable and
visible images.

`FlushSector` applies the selected pending sector to the durable image through
the local `hdd-write-crash-cuts` package. An acknowledged completion clears only
that sector's pending entry. Every valid cut without acknowledgement represents
power loss during the flush and drops every pending entry while retaining the
exact persisted byte prefix. `PowerLoss` performs the same volatile-cache drop
between commands. Refusals return the original state and a zero result.

All states and observations are value-owned. A caller must keep `data` stable
until `WriteSector` returns; the cache copies it before returning. Returned
sector arrays and snapshots cannot mutate the state.

## Verification

With Go 1.26.1, `GOTOOLCHAIN=local`, and the sibling crash-cut package available
at `../hdd-write-crash-cuts`:

```sh
sh BUILD
go test -run=^$ -fuzz=FuzzCacheHistories -fuzztime=100000x -parallel=2
GO=/path/to/go python3 tools/check_mutations.py
GOARCH=386 go test -exec=/path/to/qemu-i386 ./...
GOOS=windows GOARCH=amd64 go test -c -o /tmp/hdd-writeback-cache.test.exe
```

The tests compare each transition against a separate array model, enumerate all
ten cut classes/prefix choices, check 3,262 bounded short-history transitions,
and retain 102,400 seeded commands (seed `0xDB06A`). They check write
visibility, overwrites, acknowledgement, crash loss, typed validation order,
predecessor retention, and detached snapshots. The mutation runner verifies
three source faults against their named failing assertions.

## Validation evidence

With Go 1.26.1 and `GOTOOLCHAIN=local`, both command lists passed. Native
statement coverage was 100%; race detection and `go vet` passed. The guided
history target completed 100,000 inputs. All three clean-controlled mutations
failed at their intended assertions. Linux/386 tests passed under QEMU and a
Windows/amd64 test binary cross-compiled; native Windows execution was not run.
