# DB11b — Read and write pages over logical blocks

Read and write a fixed-size page through the same Go interface on the
memory and HDD backends. Keep the block sequence and durability
barrier visible to the caller.
[Page addressing](DB11a-go-page-addressing.md) · [Block contract](DB06b2a-block-device-contract.md)

Add the Go package `storage-page-io`.

```go
func Open(device blockdevicecontract.Device, layout storagepagelayout.Layout) (Pager, error)
func (pager Pager) ReadPage(page int64) ([]byte, error)
func (pager Pager) WritePage(page int64, data []byte) (completedBlocks int64, err error)
func (pager Pager) Flush() error
```

`Open` validates the layout first. A nil interface returns
`ErrInvalidDevice`. Otherwise require `device.Geometry()` to equal
the validated layout geometry, or return `ErrGeometryMismatch`.
A refused open returns the zero pager. A successful pager holds the
fixed layout and the same sequential device. It requires the
device geometry to stay fixed for its lifetime. All operations on
the zero pager return `ErrInvalidDevice` without device calls.

`ReadPage` resolves the page span before reading. Read blocks in
ascending order and copy their bytes into one result of the page
width. Stop at the first lower error and preserve its identity.
If a lower read succeeds with a byte count different from the
block width, return `ErrBlockWidthMismatch`. A refused read returns
nil bytes. A successful read owns its result.

`WritePage` resolves the span, then requires exactly the page width.
Use `ErrInvalidPageData` for a wrong width and perform no block
writes. For a valid page, issue one device write per block in
ascending order, passing the corresponding bytes. The caller keeps
its input stable during the call. Count each successful device write.
Stop at the first error and return the successful prefix count with
that error. The failing block's behavior follows the device contract.
After success return the complete block count. A successful page
write updates visible bytes and leaves the durability decision to
`Flush`. Flush forwards the device barrier and its result directly.

A multi-block page can be partially visible after a lower write
failure. A crash can also leave a mix of old and new page bytes,
including a torn sector in the HDD model. A caller that requires
atomic recovery uses the later WAL contract.

Use real memory and HDD clients over the same reservation and a
scripted device that exposes exact read, write and flush calls. Check
reserved offsets, alternate geometries, wrong widths, detached reads,
prefix counts and every fault position across small pages. Retain
snapshots across writes, flushes and both backends' restarts. Check
every HDD cut on one page, including a torn constituent block and
completed media with lost acknowledgement. Run both build lists,
coverage, race, vet, 100,000 guided inputs and clean controls that
skip an earlier error or write a block at the wrong offset. The
runtime depends on the layout and block contract. Backend modules
participate in tests.
