# storage-wal-ready-commit

`Commit` captures the two physical checkpoint pages before attempting the
existing WAL transaction commit. A Resetting checkpoint refuses before epoch
comparison; a Ready checkpoint must match the requested portable epoch. These
preflight refusals return a zero result and perform no log or data writes or
flushes. An admitted Ready epoch returns its selected checkpoint with the exact
underlying WAL receipt and error, including partial or ambiguous progress.

The caller provides a sequential device interval with no concurrent writers or
unrelated pending writes. Run `sh BUILD` on Unix or execute the direct Go command
list in `BUILD_windows` on Windows.
