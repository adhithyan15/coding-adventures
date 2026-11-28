# DB12b3 — Append one WAL record and acknowledge its durability

Use the verified page I/O and fixed-reservation scan to append a
single WAL record. Write its complete page, then request the shared
block-device barrier. The first sequential profile reserves at most
1,024 pages, which bounds the full-reservation scan.
[WAL scanner](DB12b2-go-wal-scan.md) · [Page I/O](DB11b-go-page-io.md)

Add the Go package `storage-wal-append`.

```go
const MaxLogPages int64 = 1_024

type Receipt struct {
    Attempted     bool
    Page          int64
    WrittenBlocks int64
    Acknowledged  bool
}

func Append(
    device blockdevicecontract.Device,
    layout storagepagelayout.Layout,
    record storagewalrecord.Record,
) (Receipt, error)
```

Open the pager first. Preserve layout, device and geometry refusals.
Then require the reserved page count at most `MaxLogPages`, using
`ErrLogCapacity` for a refusal. Derive the exact log page width from
the layout and require the WAL slot's admitted width, preserving its
`ErrPageSize` identity.

Read pages in ascending reservation order and preserve the first
device or pager error. Pass the full captured reservation through
the admitted scanner. Preserve its hard refusals. A full log returns
`ErrLogFull`. A damaged tail returns a typed failure naming its
page and decoder cause and wrapping `ErrDamagedTail`. The first
vacant page is the append destination. Return `ErrAppendSequence`
unless the requested LSN is the next contiguous rank. Encode the
complete fixed-width page and preserve its first error. Every
failure through this preflight returns a zero receipt and leaves
the device's state intact.

Write the encoded page once through the pager. Return its successful
block prefix and error on an incomplete write. Call the barrier
only after the whole page write succeeds. Forward the barrier error
and set `Acknowledged` only when the barrier returns success. A
receipt after a write attempt reports its reservation-relative page,
even when the error arrived at the first block.

A successful barrier can make all earlier writes on the same device
durable, including writes from other callers. The transaction-level
write-ahead rule will order data-page updates against acknowledged
log records. The durability contract allows persisted bytes after
a lost acknowledgement. Recovery uses the resulting scan and a
separately admitted tail policy.

Compare scripted calls and exact lower faults at each position,
full/vacant/damaged reservations, signed and width boundaries, and
a real restart trace across memory and HDD. Inject blocked writes,
every bounded HDD barrier cut and lost response. Check untouched
neighbors and earlier acknowledged log pages. Run both command
lists, race, vet, guided inputs and clean controls for flushing
after a failed page write or placing the record after the wrong gap.

## Local verification

The implementation is in `30c1d5819cd`. An independent caller checked
all 66 modeled HDD barrier cuts across restart, including a complete
durable record with a lost acknowledgement. It compared receipts,
exact recovered bytes, neighboring blocks, and previously acknowledged
records. It also verified the first write refusal and typed tail and
sequence errors. Replay passed race and vet.

Both command lists passed at 100% statement coverage with race and
vet. Module tidy, 100,000 guided inputs and both mutation controls
passed. The native suite checked every read position and every
incomplete write prefix in its fixed reservation. Build discovery
added only this package, with twelve expected edges per platform,
and preserved all 6,014 existing graph rows and 6,013 recipes.
