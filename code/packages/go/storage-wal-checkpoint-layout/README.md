# storage-wal-checkpoint-layout

Admits exactly two adjacent checkpoint pages with the minimum whole-block width for a 40-byte frame, after validating the existing log/data pair. It requires matching device geometry and a disjoint half-open interval. The pure validator makes no device calls and allocates no buffers.

Run `sh BUILD` on Unix. `BUILD_windows` lists the direct Go commands for Windows.
