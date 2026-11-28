# DB12c2c2a — Return the recovered log's final admitted rank

Give checkpoint publication the exact tail rank from the
same captured log that successfully completed data replay.
[Existing recovery](DB12c2c2-go-wal-recovery.md)

Extend the Go `storage-wal-recovery` package.

```go
type Result struct {
    Replay  storagewalredoapply.Receipt
    LastLSN uint64
}

func RecoverWithBoundary(device blockdevicecontract.Device, log, data storagepagelayout.Layout) (Result, error)
```

Use the existing paired layout, log capture, tail admission
and redo application in the same order. A successful call
returns the exact replay receipt and the LSN of the last
admitted record. Use zero for an empty log. The last admitted
record may be a begin or uncommitted image whose data was
excluded from replay.

On refusal or partial replay, preserve the existing error and
partial replay receipt. Leave the rank zero. Reuse this shared
operation in Recover so its public receipt, errors and device
effects remain identical. Capture the log only once.

Check complete and unfinished transactions, empty and damaged
tails, full reservations, exact read counts and interrupted
data replay. Compare old and new APIs across physical and
scripted device histories. Run both build lists, coverage,
race, vet, guided inputs and clean mutation controls for using
the last committed rank or exposing a boundary after failed
replay.

## Local verification

The implementation is in `6bddc3ff7dd`. The independent caller
compared 160 normal and legacy histories with exact media and a
single 28-block capture. Another 704 lower-layer cuts preserved
the original replay receipt. The caller observed a last rank
only after replay completed. Both command lists, two guided
campaigns and four clean mutation controls passed. The build
graph and recipes matched the earlier package exactly.
