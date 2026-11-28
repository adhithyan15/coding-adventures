# DB12d3a1 — Read the physical checkpoint pages during formatting

Expose the admitted physical page read needed before a database
has a published checkpoint. Reuse the existing checkpoint
capture package.
[Capture contract](DB12d2d-go-wal-checkpoint-capture.md)

```go
func ReadPages(device blockdevicecontract.Device, log, data, checkpoint storagepagelayout.Layout) ([2][]byte, error)
```

Apply the same validation, pager open, physical ordinal order
and padding checks already defined for Capture. Preserve exact
errors and return a zero page pair on refusal. Read both pages
before checking padding. Return both complete owned buffers.
This operation admits physically valid padded pages containing
vacant, staged, corrupt or published frame prefixes. The caller
may inspect or select those prefixes through the existing rules.

Make Capture use this read and keep its existing selection
contract, first error and zero snapshot on refusal. Keep its
physical calls and returned ownership identical. Formatting
can now inspect vacant or staged pages before the first
initialization marker is available.

Compare direct physical reads and Capture at every lower error,
including shape before device, read order, padding, detached
copies, vacancy and invalid frame contents. Retain the existing
capture tests. Run both build lists, coverage, race, vet,
guided inputs and clean mutation controls for premature logical
selection or a missing padding check.

## Local verification

The implementation is in `c61a1636210`. It retained the 56
independent capture decisions and matched raw reads of vacant,
staged and malformed frame prefixes across four block geometries.
It checked exact read order, detached pages and padding errors.
Both command lists passed at 100% coverage with race, vet and
tidy. Capture and raw reads each passed 100,000 guided inputs.
All three mutation controls passed, publication and clearing
passed again, and the build graph and recipes matched exactly.
