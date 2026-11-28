# storage-wal-append

Appends one record to a bounded fixed-page WAL reservation. The package validates and scans the full reservation before writing the first vacant page, then acknowledges only after the device barrier succeeds. A receipt after a failed write reports the successful block prefix; a lost barrier response may leave a complete record durable without acknowledgement.

Run `sh BUILD` on Unix. `BUILD_windows` lists the direct Go commands for Windows.
