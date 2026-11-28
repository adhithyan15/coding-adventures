# hdd-barrier-plan (Go)

This package prepares a barrier's ordered sector writes before any device or
cache effects occur. It validates the geometry, ascending dirty-block snapshot,
barrier descriptor, lower media cut, and full timing admission for every
planned sector. Any refusal returns a zero plan.

`Prepare` emits a detached `Steps` slice. Completed steps use the lower
acknowledged cut; an `AtSector` terminal step owns a copy of the supplied
prefix pointer. Its receipt records only completed earlier sectors and the
terminal persisted-byte prefix. `AfterAllMedia` counts all sectors without
acknowledging the barrier; `BarrierComplete` also sets acknowledgement.
`BeforeFirstSector` and empty plans retain a nil `Steps` slice.

Run with Go 1.26 and `GOTOOLCHAIN=local`:

```sh
sh BUILD
go test -run=^$ -fuzz=FuzzBarrierPrepare -fuzztime=100000x -parallel=1
python3 tools/check_mutations.py
go mod tidy -diff
```

The independent stepped-timing model sweeps every dirty mask in an eight-block
geometry and every valid barrier ordinal/prefix. Additional cases cover exact
error order, later-step clock exhaustion, empty barriers, and detached terminal
cuts. The package composes geometry, timing, lower cut description, and the
fixed barrier descriptors; it does not depend on the timed media implementation.
