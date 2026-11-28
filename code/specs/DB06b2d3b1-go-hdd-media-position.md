# DB06b2d3b1 — Observe the HDD head and clock

A cache barrier needs the current head and clock before it
plans any media writes. Reading that position leaves the disk
and sector image untouched.
[Timed medium](DB06b2d3b-go-sparse-timed-hdd-media.md) · [Barrier plan](DB06b2d3c1-go-hdd-barrier-plan.md)

Extend the existing Go package `hdd-timed-media`.

```go
func (medium *Medium) Position() (hddseektiming.State, error)
```

Use the same receiver admission as Snapshot. A nil or zero
receiver returns zero State and the exact geometry error.
For a valid medium, return the current head and tick by value.
Position allocates zero bytes and preserves media and timing.
Snapshot continues to produce its detached sector map.

Compare Position with Snapshot after each read, every admitted
write cut, a refused request and a restart. Exercise empty and
populated sparse media and the portable geometry ceiling. Check
zero allocations, both build lists, race, vet, guided inputs and
a clean control for returning the initial clock after motion.

## Local validation

Commit `46bab963a69` matched 48,960 independent position checks over
reads, all physical write cuts, refused requests and separate restart
instances on both supported test geometries. The original 5,440 sparse
sector histories also passed. Position allocated zero bytes with 97
populated sectors on the maximum portable geometry.

Both build command lists reached 100% statement coverage on Linux.
Race, vet, tidy, Linux/386 and 100,000 guided inputs passed. The clean
controls passed and all three mutations hit their intended witnesses.
The build graph and recipes matched the previous export byte for byte.
