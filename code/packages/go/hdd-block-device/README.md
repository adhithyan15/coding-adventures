# Go HDD block device

This adapter implements `blockdevicecontract.Device` for the fixed 32-block,
eight-byte HDD profile. It delegates volatile and durable bytes to the admitted
`hdd-writeback-cache` value. Reads and writes use the shared block geometry and
refusal order. Writes are copied, and every read returns detached bytes.

`Flush` runs an acknowledged full dirty-set barrier. `FlushWithCut` exposes the
cache's exact barrier cut and receipt; successful unacknowledged cuts still
install their media and power-loss transition, while refusals leave the HDD
unchanged and return a zero receipt. `Restart` uses the admitted cache
`PowerLoss` transition to return a separate value from durable bytes only and
preserves the input HDD.

The zero value is a fresh HDD; a nil receiver reports zero geometry and
`ErrInvalidGeometry` for operations. Run the Unix list with `sh BUILD` and the
Windows list one command at a time from this directory.

The native tests compare common client traces with the memory device and use an
independent byte-array model for every barrier cut over three dirty blocks.
Run named disposable semantic faults with `python3 tools/check_mutations.py`.
