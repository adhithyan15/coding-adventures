# storage-wal-data-layout

Validates that a fixed WAL reservation and a fixed data reservation are valid, disjoint, on the same logical device, and sized so each full data page image fits in one WAL slot. Validation is pure and performs no device calls or allocations.

Run `sh BUILD` on Unix. `BUILD_windows` lists the direct Go commands for Windows.
