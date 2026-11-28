# DB12a — Encode the first redo WAL records

A redo log records the order of transactions and the page bytes that
recovery can install after a durable commit. Start with one record
codec. Later slices assemble the append stream, publish its
durability and distinguish committed transactions from incomplete
ones. [Record frame](DB10b-storage-record-frame.md)

Add the Go package `storage-wal-record`. The first codec uses the
checksummed storage record frame and the following payload.

```go
type Kind uint8

const (
    BeginTransaction Kind = 1
    WritePageImage   Kind = 2
    CommitTransaction Kind = 3
)

type Record struct {
    Kind    Kind
    LSN     uint64
    TxID    uint64
    PageID  int64
    Image   []byte
}

func Encode(Record) ([]byte, error)
func Decode([]byte) (Record, error)
```

The payload is 28 bytes plus optional page image bytes. Byte zero
contains the kind. Bytes 1 through 3 are reserved zero bytes.
Bytes 4 through 11 store the LSN, bytes 12 through 19 the transaction
ID and bytes 20 through 27 the page ID. Each numeric field is an
unsigned little-endian integer. The codec accepts LSN and TxID from
1 through 9,007,199,254,740,991. Page IDs range from zero through the
same ceiling. Check the signed input before encoding it.

Begin and commit records use page ID zero and an empty image.
A page record accepts an image of 1 through 65,492 bytes, keeping
the encoded payload within the record frame's limit. The pager
and stream later check the exact page width and target geometry.

Encode checks kind, LSN, TxID, page ID, then the image shape. Use
`ErrKind`, `ErrLSN`, `ErrTxID`, `ErrPageID` and `ErrImageLength`
for those respective refusals. A begin or commit page-ID mismatch
uses `ErrPageID` even when its image is also present.

Decode first preserves the checksummed frame decoder's result.
A decoded payload shorter than 28 bytes returns `ErrHeaderSize`.
Check kind and then reserved bytes (`ErrReserved`) before the
numeric and image checks. Evaluate the decoded page ID before
converting it to the signed API field. Return a zero record on
every refusal. A successful image result owns its bytes, and
begin or commit returns a nil image. Encode owns its result.
Keep inputs stable during either operation.

Compare independent binary vectors through the frame boundary.
Include zero and maximum identifiers, false kind and reserved
values, syntactically valid frames with malformed WAL payloads,
crossed validation order, maximum page image, and changed header
or image bytes. Check image ownership, bounded fuzzing, both
command lists, race, vet and controlled field-order and data-copy
failures. Stream replay will establish LSN order separately.
