# storage-wal-page-slot

Stores one complete checksummed WAL record at the start of a fixed-width page, with a little-endian frame length and zero-filled suffix. An all-zero page is vacant. The package delegates record validation and preserves the typed error identities from the record decoder.

Run `sh BUILD` from this directory. The Windows command list is in `BUILD_windows`.
