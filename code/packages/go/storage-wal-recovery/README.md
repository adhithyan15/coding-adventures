# storage-wal-recovery

Validates the disjoint WAL/data layout, captures and classifies every bounded log slot, and replays only an accepted complete record prefix. Damaged tails retain the append package's typed page/cause error; no data writes occur before layout, capture, tail, and redo-plan admission.

Run `sh BUILD` on Unix. `BUILD_windows` lists the direct Go commands for Windows.
