# storage-wal-checkpoint-clear

Clears every physical block in the validated WAL log reservation only after the selected checkpoint record is Resetting. It writes one zero block at a time in ascending order, then acknowledges one durability barrier. A receipt preserves the selected checkpoint, successful write prefix, and barrier acknowledgement separately.

After a write or barrier error, reopen from the durable device image before retrying. An interrupted clear leaves the Resetting checkpoint available for restart; repeating a complete clear is safe.

Run `sh BUILD` on Unix. `BUILD_windows` lists the direct Go commands for Windows.
