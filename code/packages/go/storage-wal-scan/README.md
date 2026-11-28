# storage-wal-scan

Scans an ordered reservation of fixed-width WAL slots, accepts contiguous LSNs from one, and classifies the first vacant or malformed tail. Later occupied or malformed slots invalidate that tail; typed frame and record errors are retained as the damaged tail cause.

Run `sh BUILD` from this directory. The Windows command list is in `BUILD_windows`.
