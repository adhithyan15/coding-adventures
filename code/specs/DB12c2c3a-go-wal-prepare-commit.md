# DB12c2c3a — Prepare the records for one redo-only commit

Check the complete transaction before the first WAL write. Give its
begin, page images and commit their exact append ranks and use the
existing redo planner to validate the combined history.
[Log and data layout](DB12c2c1-go-wal-data-layout.md) · [Redo planner](DB12c1-go-wal-redo-plan.md)

Add the pure Go package `storage-wal-prepare-commit`.

```go
type PageImage struct {
    PageID int64
    Image  []byte
}

func Prepare(
    log, data storagepagelayout.Layout,
    prefix []storagewalrecord.Record,
    txID uint64,
    images []PageImage,
) ([]storagewalrecord.Record, error)
```

The caller supplies the stable record prefix from a successful capture
and applies the tail policy before preparing normal appends. Validate the
log and data layouts first. Count the unused log slots and require room
for a begin, every requested page image, and a commit. Use
`ErrInsufficientSlots` for that refusal. Handle the available space
before adding record counts.

The begin rank is the prefix length plus one. Put the proposed
transaction ID in the begin and validate it through the existing WAL
record encoding. The writes follow in request order, including repeated
updates to one page. The last rank belongs to the commit. A transaction
with zero requested images still carries begin and commit records.

Validate the concatenation of the admitted prefix and the proposed
records with the existing redo planner. Preserve its first grammar,
page address or image width error. The existing layout admission
and fixed slot overhead ensure every admitted image fits. All
refusals return nil records.

After success return only the proposed records in their original
append order. The result owns its page images. Keep the prefix,
image arguments and caller-owned buffers unchanged. The package
makes no device calls.

Compare an independent record generator and history model. Exercise
exact fit, shifted ranks, duplicate and incomplete transactions,
zero-image commits, repeated pages, malformed images, reused input
slices and crossed faults. Verify exact slot encoding of every
generated record. Run both build lists, race, vet, guided inputs
and clean controls for dropping the commit reservation or using
an incorrect first rank.

## Local verification

The implementation is in `307ab7f5d78`. An independent record and
transaction oracle matched 8,064 capacity, rank, grammar and page
decisions. It encoded each record in all 512 admitted sequences
with the actual slot codec. It also verified that records adopt
separate copies of overlapping input images. Race and vet passed.

Both command lists passed on Linux with 100% statement coverage,
race and vet. Module tidy, 100,000 guided inputs and both mutation
controls passed. Build discovery added only the preflight package
and its eleven expected edges per platform.
