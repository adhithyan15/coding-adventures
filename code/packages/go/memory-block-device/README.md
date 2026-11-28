# Go memory block device

This package implements the sequential `blockdevicecontract.Device` interface with sparse pending and durable maps. A new valid device reads as zeroes without allocating media contents. Writes copy complete blocks into the pending image; `Flush` moves all pending entries to the durable image. `Restart` returns a new device containing only durable entries and leaves the original device unchanged.

Reads return detached byte slices. Private block slices are immutable after insertion, so restart lineages can share them safely while keeping independent maps. The zero value and a nil receiver report zero geometry; operations refuse them with the contract's `ErrInvalidGeometry` identity.

Run the Unix command list with `sh BUILD`. Run `BUILD_windows` one line at a time from the package directory. `device_test.go` compares stateful operation histories against a separate sparse byte-map model, including caller and read-result mutation.

Run the two source mutation controls with `python3 tools/check_mutations.py` from this directory.
