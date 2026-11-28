# storage-wal-redo-plan

Builds a pure redo plan from an already accepted contiguous WAL record prefix. It buffers full-page images by active transaction, selects only committed write sets, orders transactions by commit LSN, and preserves each transaction's original write order. Returned images are owned by the plan.

Run `sh BUILD` on Unix. `BUILD_windows` lists the direct Go commands for Windows.
