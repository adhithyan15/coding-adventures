# hdd-timed-media (Go)

This package combines the configurable HDD geometry, pure seek planner, and
single-sector crash-cut descriptor into a sparse durable medium. It stores
only nonzero eight-byte sectors. An absent sector reads as zero. A read moves
the head and clock but does not change durable bytes; a successful write
advances time and applies only the described byte prefix.

`New` validates geometry and starts at cylinder and tick zero. The medium is
caller-serialized. `Snapshot` returns a detached sparse map, and `Restart`
preserves that durable map while resetting head and tick to zero. Snapshots and
reads return fixed byte arrays, so their contents do not alias internal media.
`Position` returns the current head and tick by value without copying the sparse
map or changing either value.

Write validation is ordered as geometry, block, exact payload width, cut
descriptor, and service timing. No bytes or state change until every check
passes. A before-media cut reports seek and rotation time but no transfer tick
after full-service admission succeeds; the write is refused if that service
would overflow. Every cut that reaches media consumes one transfer tick. The
persisted prefix counts bytes physically written within that transfer.

Run the package from this module with Go 1.26 and `GOTOOLCHAIN=local`:

```sh
sh BUILD
go test -run=^$ -fuzz=FuzzTimedMediaHistory -fuzztime=100000x -parallel=1
python3 tools/check_mutations.py
go test -race -count=1 ./...
go vet ./...
```

Tests compare both 512-sector geometries against an independent stepped
byte/tick model, exercise detached snapshots/restarts and sparse zero removal,
and drive a maximum-sized logical geometry without allocating the absent
medium. The canonical DB04a fixture is not reread by this composition package;
the exact lower crash-cut package remains an unmodified dependency.
