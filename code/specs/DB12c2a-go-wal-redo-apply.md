# DB12c2a — Replay full committed images through page I/O

Replay the admitted WAL prefix into data pages while recovery has exclusive
access to a restarted device. Finish each image through its durability
barrier before beginning the next image. A caller can start again from the
same prefix after a crash and reach the same final page contents.
[Redo plan](DB12c1-go-wal-redo-plan.md) · [Page I/O](DB11b-go-page-io.md)

Add the Go package `storage-wal-redo-apply`.

```go
type Receipt struct {
    AcknowledgedWrites int
    Interrupted        bool
    PageID             int64
    RecordLSN          uint64
    WrittenBlocks      int64
}

func Replay(
    device blockdevicecontract.Device,
    layout storagepagelayout.Layout,
    records []storagewalrecord.Record,
) (Receipt, error)
```

The caller supplies the stable contiguous record prefix and owns the
decision to admit the WAL tail. The device starts with no pending writes.
The data layout covers only data pages. The log stays available and its
captured records remain stable throughout replay. Clients observe data
pages after a successful recovery barrier sequence.

Open the data pager first and preserve its error. Build the entire redo
plan before writing data and preserve its error. Either refusal returns
a zero receipt. An empty plan makes no writes or barrier requests and
returns the zero receipt successfully.

Visit each planned write in order. Pass its complete image to
`WritePage` once. At the first write error, preserve the reported block
prefix and stop immediately. Issue a flush after every complete page
write. At the first flush error, preserve that error and stop. In either
case set `Interrupted` and report the current page, its original write
LSN, the successful block prefix, and the count of earlier image
barriers that acknowledged. A barrier error can arrive after the full
image became durable. Increment `AcknowledgedWrites` after each
successful barrier. After complete replay the interruption fields are zero.

Recovery repeats the full plan on a restarted device. Earlier images may
temporarily replace later images during that replay. The caller keeps these
intermediate states private. With a complete successful replay, every
updated page contains the final image in commit order and every other
page retains its previous contents.

Use a scripted device for exact command order and every data write and
barrier failure. Compare memory and real HDD backends against a detached
page oracle. Include competing transactions, repeated page images,
uncommitted writes and nonzero bytes in neighboring blocks. Cut each
modeled barrier for a bounded workload, restart and interrupt recovery
again. Verify convergence and exact receipts for both crashes, including
a lost acknowledgement. Verify plan errors precede writes, both build
lists, race, vet, guided inputs and clean controls for issuing a flush
after a failed write and reordering the committed images.
