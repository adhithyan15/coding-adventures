# DB12c1 — Build a redo plan from committed WAL transactions

Use the admitted WAL prefix to find transactions that reached a
commit record. Keep the redo plan independent of I/O so its grammar
and ordering can be checked against a small state model.
[WAL records](DB12a-go-wal-record.md) · [WAL scanner](DB12b2-go-wal-scan.md)

Add the Go package `storage-wal-redo-plan`.

```go
type RedoWrite struct {
    Record    storagewalrecord.Record
    CommitLSN uint64
}

func Plan(
    layout storagepagelayout.Layout,
    records []storagewalrecord.Record,
) ([]RedoWrite, error)
```

The caller supplies the accepted, contiguous record prefix in LSN
order and keeps its bytes stable. Recovery decides which physical
tail is admissible before invoking this operation. Validate the data
page layout first and preserve its error.

Visit records in sequence order and maintain each transaction's
begin, buffered page images and commit state. A begin requires an
unused transaction ID within this prefix. Use `ErrTransactionExists`
for a repeated begin. A write or commit requires an active begin,
using `ErrTransactionInactive` for a refusal. An unknown kind
returns `ErrRecordKind`. For a page image, check its page ID
using the data layout and then require its exact byte width with
`ErrImageWidth`. Buffer the write for that transaction. A commit
selects its buffered writes and closes the transaction. A
transaction can commit with an empty write set.

Order selected transactions by their commit LSN. Within each
transaction, preserve the order of its original page records.
Attach the transaction's commit LSN to every selected write.
An incomplete transaction contributes zero selected writes.
Every refusal returns a nil plan. A successful plan owns its image
bytes. A successful empty plan returns nil.

Compare against a separate transaction table over bounded histories.
Check interleaved transactions, repeated updates to one page,
duplicate transaction starts, inactive writes and commits,
zero-write commits, original LSN provenance and output ownership.
Verify layout and payload error order and include histories with
several simultaneous faults. Run both build lists, coverage, race,
vet, 100,000 guided inputs and controls for applying uncommitted
writes or reversing commit order.
