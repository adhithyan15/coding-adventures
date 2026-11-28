# storage-wal-publish-commit

Validates paired layouts and the captured WAL tail, prepares a complete transaction, then appends its records in order. The result records each acknowledged append and exposes the detached transaction only after the commit barrier is acknowledged.

Run `sh BUILD` on Unix. `BUILD_windows` lists the direct Go commands for Windows.
