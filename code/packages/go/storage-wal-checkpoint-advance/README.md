# storage-wal-checkpoint-advance

Advances one explicit checkpoint epoch or resumes a selected Resetting epoch. A Ready checkpoint returns immediately when it already has the requested target; otherwise only its immediate successor is allowed. The operation recovers the WAL and its last admitted boundary, publishes Resetting to authorize reclamation, clears the WAL, and publishes Ready with the same epoch and boundary.

A selected Resetting checkpoint resumes directly at clearing without replaying the log. Each completed or partially attempted phase retains its receipt on lower errors. Reopen after any write or barrier error and retry with the same target. Concurrent writers and unrelated pending device changes are outside the sequential operation contract.

Run `sh BUILD` on Unix. `BUILD_windows` lists the direct Go commands for Windows.
