# Go block-device contract

This leaf package defines the first sequential logical block-device contract. `Geometry` accepts a positive block count through 9,007,199,254,740,991 and a positive block width through 65,536 bytes. `CheckBlock` validates geometry before the signed block address; `CheckWrite` then validates exact payload width. The stable errors are `ErrInvalidGeometry`, `ErrInvalidBlock`, and `ErrInvalidBlockData`.

`Device` exposes geometry, block reads, block writes and a flush barrier. The interface defines operation shape only; concrete devices define persistence and recovery behavior. This package owns no media bytes and does not allocate during validation.

Run the Unix command list with `sh BUILD`. Run the Windows command list one line at a time from the package directory. `device_test.go` contains an independent validation model and a fuzz target for signed bounds and validation precedence.

Run the named disposable source mutations with `python3 tools/check_mutations.py` from this directory.
