# storage-wal-commit

Publishes a complete transaction to the WAL and, only after the commit record's barrier is acknowledged, replays that transaction's detached page images. Log and data receipts are retained independently so a committed transaction can be recovered after interrupted data replay.

Run `sh BUILD` on Unix. `BUILD_windows` lists the direct Go commands for Windows.
