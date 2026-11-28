# storage-wal-checkpoint-capture

`ReadPages` validates the layout, reads both complete checkpoint pages in physical order, checks only their reserved padding, and returns detached buffers. It admits vacant, staged, corrupt, and published frame prefixes for formatter use before a checkpoint is logically selectable.

`Capture` uses the same physical read path and then selects from the exact frame prefixes. It preserves the selector's errors and successful snapshots own both full pages. Both operations are read-only.

Run `sh BUILD` on Unix. `BUILD_windows` lists the direct Go commands for Windows.
