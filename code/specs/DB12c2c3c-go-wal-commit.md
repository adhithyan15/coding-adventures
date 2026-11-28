# DB12c2c3c — Apply data only after the log commit acknowledges

Compose durable log publication with the existing page replay
boundary. Apply only the newly committed transaction's images,
in their prepared order. An interrupted data application leaves
the committed log available for recovery.
[Log publication](DB12c2c3b-go-wal-publish-commit.md) · [Replay](DB12c2a-go-wal-redo-apply.md)

Add the Go package `storage-wal-commit`.

```go
type Result struct {
    Log  storagewalpublishcommit.Result
    Data storagewalredoapply.Receipt
}

func Commit(
    device blockdevicecontract.Device,
    log, data storagepagelayout.Layout,
    txID uint64,
    images []storagewalpreparecommit.PageImage,
) (Result, error)
```

The caller owns the device exclusively. Prior data writes have
completed, or the caller has recovered the current log. The caller
keeps the supplied records stable and holds foreground readers
until the data application succeeds.

Call log publication and preserve its exact receipt and error.
On a log refusal, leave the data receipt zero and perform no data
writes. A complete log commit with a lost acknowledgement can
become visible through later recovery from the durable bytes.

After an acknowledged log commit, apply its detached transaction
records through the admitted page replay. Preserve the replay
receipt and error. Report the acknowledged log result even if a
data-page write or barrier fails. A zero-image transaction has
no data effects. Earlier data images remain as previously applied
unless this new transaction updates those same pages.

Check write and barrier order with a scripted device, and compare
memory and HDD with restart after both log and data interruptions.
Include a complete lost-acknowledgement log commit, data-page
failure after acknowledged commit, repeated writes to one page,
several transactions at different original log ranks and a
zero-image commit. Preserve neighboring data and the committed
log. Run both build lists, coverage, race, vet, guided inputs and
clean controls for applying data before log success or replaying
the previous transaction's images.

## Original-rank admission

An independent caller passed four standalone committed sequences
through the existing redo planner and replay on memory, at starting
LSNs 1, 3, 19 and 1,000. It checked original write and commit ranks,
a fully durable page with a lost response, a complete replay, and
untouched neighbors. Race and vet passed.
