# storage-wal-prepare-commit

Builds and validates one complete begin/page-image/commit record sequence without device I/O. The existing redo planner validates the combined prefix and proposed transaction; successful output images are detached and remain in request order.

Run `sh BUILD` on Unix. `BUILD_windows` lists the direct Go commands for Windows.
