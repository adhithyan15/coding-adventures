# storage-wal-checkpoint-selection

Selects the governing published WAL checkpoint from two independently persisted slots. Slot inspection errors take precedence in physical order; epoch, prior LSN agreement, and phase precedence determine the winner only after both slots are valid.

Run `sh BUILD` on Unix. `BUILD_windows` lists the direct Go commands for Windows.
