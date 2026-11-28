# Go storage page I/O

`storage-page-io` binds a validated `storage-page-layout.Layout` to a
`blockdevicecontract.Device`. `Open` validates the layout before checking for a
nil device and exact geometry equality. A `Pager` reads one page by issuing
ascending block reads and assembling the exact declared width. Writes split
one page into ascending block writes and return the successful prefix count if
a lower write fails. `Flush` forwards the device barrier unchanged.

Lower read and write errors retain their identities. A successful short or
long block read returns `ErrBlockWidthMismatch`; a page payload of the wrong
width returns `ErrInvalidPageData` before any block writes. Successful reads
own their result. The caller keeps input bytes stable during `WritePage` while
the device copies each block according to its contract.

The tests use an exact-call scripted device, then replay page traces on both
memory and HDD adapters. The HDD suite compares all barrier cut families over
a 64-byte page spanning eight blocks, including torn sectors and lost
acknowledgement. Run the Unix command list with `sh BUILD` and each Windows
command one line at a time. Run source mutations with
`python3 tools/check_mutations.py`.
