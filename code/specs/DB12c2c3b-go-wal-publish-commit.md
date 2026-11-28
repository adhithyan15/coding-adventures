# DB12c2c3b — Publish one fully prepared WAL transaction

Append the begin, page images and commit in their prepared order.
Preserve the exact acknowledgement boundary after each append.
Return the prepared transaction to its data-page caller once the
commit's barrier acknowledges.
[Transaction preparation](DB12c2c3a-go-wal-prepare-commit.md) · [WAL append](DB12b3-go-wal-append.md)

Add the Go package `storage-wal-publish-commit`.

```go
type Result struct {
    AcknowledgedRecords int
    LastLSN             uint64
    LastAppend          storagewalappend.Receipt
    Committed           []storagewalrecord.Record
}

func Publish(
    device blockdevicecontract.Device,
    log, data storagepagelayout.Layout,
    txID uint64,
    images []storagewalpreparecommit.PageImage,
) (Result, error)
```

The caller holds exclusive access to the device. Preceding operations
have finished or the device has restarted, and there are no other
pending writes for a barrier to publish. The caller keeps its inputs
stable throughout the operation.

Validate the paired layouts first. Capture the bounded log and preserve
its first refusal. Apply the initial tail policy: return the existing
typed damaged-tail error with the exact page and decoder cause.
Pass the full or vacant accepted prefix to transaction preparation.
Preserve its first refusal. A failure in this preflight returns a zero
result and performs no block writes or barriers.

Call the admitted append operation once per prepared record in order.
After each call, record that entry's LSN and the exact append receipt.
Increment the acknowledged-record count after a successful append.
At the first error, return it with the accumulated result. Any
attempted append can leave durable bytes after a lost acknowledgement.
`Committed` is nil on every refusal. After all entries acknowledge,
return the detached prepared transaction in `Committed` in append
order. The data interval is reserved for the later data-page caller.

Compare the call order and bytes with a scripted block device and
the admitted memory and HDD backends. Check preflight refusals,
sparse and exact capacity, every failed log write and barrier position,
lost commit acknowledgement, and restart classification. Preserve
the data interval and neighboring blocks. Run both build lists,
coverage, race, vet, guided inputs and clean controls for continuing
after a failed append or acknowledging the transaction early.
