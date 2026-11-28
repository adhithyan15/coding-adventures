# hdd-sparse-block-device (Go)

This package presents a fixed logical block device over a sparse volatile write-back overlay and an eight-byte timed HDD medium. Writes copy complete caller blocks into the overlay. Reads return owned bytes from that overlay without touching the medium; cache misses read the durable sparse medium and advance its head and clock.

`FlushWithCut` captures the dirty IDs in ascending order, obtains the medium position, and asks `hdd-barrier-plan` to validate the entire sequence before applying a write. A refusal returns the planner's exact error and zero receipt with pending and durable state unchanged. An accepted cut applies its exact media prefix and clears the volatile overlay, including after before-first, partial-prefix, and lost-response cuts. `Flush` uses an acknowledged barrier. `Restart` drops pending writes and returns an independent device with the durable sparse image and reset position.

Callers serialize operations and keep write input stable for each call. `Geometry` returns zero for nil or unconfigured devices. Nil and zero-value operations return the block contract's geometry error.

Run with Go 1.26 and `GOTOOLCHAIN=local`:

```sh
sh BUILD
go test -run=^$ -fuzz=FuzzSparseBlockDeviceHistory -fuzztime=100000x -parallel=1
python3 tools/check_mutations.py
go mod tidy -diff
```

Tests use an independent visible/durable block model, both 512-block profiles, every byte-prefix barrier cut at every terminal ordinal, a full-profile read/write barrier, a portable-ceiling later-step preflight refusal, detached reads, and restart behavior.
