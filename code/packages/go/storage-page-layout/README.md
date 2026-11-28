# Go storage page layout

`storage-page-layout` maps zero-based pages to contiguous blocks in one logical
block-device geometry. `Layout.Validate` validates the shared geometry first,
then the reservation start, dimensions, portable page width and exact-fit
capacity. `Layout.Span` returns the exact first block, block count and page
byte width, with signed page validation after the layout.

The implementation checks page width and reservation capacity with division
before multiplying signed values. It performs no device access or buffer
allocation. `ErrInvalidLayout` marks reservation failures; invalid page IDs
return `ErrInvalidPage`. An invalid underlying geometry preserves the shared
`blockdevicecontract.ErrInvalidGeometry` identity.

`testdata/vectors.json` is the small portable exact input/result table for other
ports. The native tests also compare small geometries against an oracle that
enumerates occupied block intervals, exercise signed portable ceilings, and
measure zero allocations.

Run the Unix list with `sh BUILD`, or the Windows list one command at a time.
Run the two disposable source mutations with
`python3 tools/check_mutations.py`.
