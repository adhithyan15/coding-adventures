# storage-wal-checkpoint-capture

Reads both checkpoint pages from a caller-supplied device in physical order, rejects nonzero reserved padding, then selects from the exact frame prefixes. Successful snapshots own both full pages; capture performs no writes or flushes.

Run `sh BUILD` on Unix. `BUILD_windows` lists the direct Go commands for Windows.
