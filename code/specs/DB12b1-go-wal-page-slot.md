# DB12b1 — Store one WAL record in a fixed page

Give the first append-only WAL one record per page. This makes the
boundary between a checksummed record and unreadable or partially
written storage visible in experiments. A later layout can pack
multiple records into a page.
[WAL records](DB12a-go-wal-record.md) · [Page I/O](DB11b-go-page-io.md)

Add the Go package `storage-wal-page-slot`.

```go
func EncodePage(pageBytes int64, record storagewalrecord.Record) ([]byte, error)
func DecodePage(page []byte) (record storagewalrecord.Record, present bool, err error)
```

Support page widths from 48 through 65,536 bytes. Encode a valid
WAL record and put its full framed length in the first four bytes
as an unsigned little-endian integer. Copy the frame immediately
after those bytes and fill the remaining page with zeros. The
encoded frame must fit the supplied width. The entire page is
caller-owned after success.

An all-zero page is vacant. A page with zero in its length field
and a nonzero byte anywhere else returns `ErrVacantPageContents`.
For a nonzero length, require a full WAL frame of at least 44 bytes
that fits within the page after the length field. Use
`ErrSlotFrameLength` for that refusal. Pass exactly the selected
frame bytes to the record decoder and preserve its typed errors.
After a successful record decode, require each suffix byte to be
zero, using `ErrPadding` for a refusal.

Check page size first with `ErrPageSize`. For encode, preserve the
first record error before checking whether the frame fits.
All encode refusals return nil bytes. A vacant page returns a zero
record, false and no error. A corrupt page returns zero, false and
the first error. An occupied page returns an owned record, true
and no error. The caller keeps all input bytes stable during the call.

Log and data page widths are separate. The first HDD experiment can
reserve four 56-byte log slots and four 8-byte data pages in its
32-sector image. A 56-byte slot fits an 8-byte redo payload plus
its fixed header, frame and length word.

Compare independent portable vectors for vacancy, begin, redo
and commit, frame-size bounds, crossed errors and padding. Cut
writes at each byte boundary for short pages, including cuts where
new and previous bytes happen to agree. Check code paths for exact
width, detached results and bounded allocation. Run both command
lists, race, vet, 100,000 guided inputs and controlled failures for
accepting nonzero vacant content or bypassing the record decoder.
