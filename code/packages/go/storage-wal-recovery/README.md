# storage-wal-recovery

Validates the disjoint WAL/data layout, captures and classifies every bounded log slot, and replays only an accepted complete record prefix. Damaged tails retain the append package's typed page/cause error; no data writes occur before layout, capture, tail, and redo-plan admission.

`RecoverWithBoundary` performs the same single capture and replay, returning the exact last admitted log LSN only after replay succeeds. The boundary includes unfinished transactions that were admitted but excluded from redo; an empty log returns zero. Refusals and interrupted replay preserve the original error and replay receipt while leaving the boundary zero. `Recover` delegates to this operation and continues to return its original receipt and error.

Run `sh BUILD` on Unix. `BUILD_windows` lists the direct Go commands for Windows.
