# storage-wal-record

Portable codec for the initial WAL begin, full-page redo and commit records. Records are carried inside the admitted DBRF frame; LSN, transaction ID and page coordinates are validated before encoding, and decoded page images are detached copies.

Run `sh BUILD` from this directory. The Windows command list is in `BUILD_windows`.
