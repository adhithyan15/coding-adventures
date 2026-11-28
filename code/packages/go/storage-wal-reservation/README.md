# storage-wal-reservation

Captures a bounded fixed-page WAL reservation through the admitted page-I/O layer and classifies the full record prefix. Capture preserves the first validation, read, or scanner error; a successful result includes the checked pager and complete scan, including a damaged tail for caller policy. The operation only reads blocks.

Run `sh BUILD` on Unix. `BUILD_windows` lists the direct Go commands for Windows.
