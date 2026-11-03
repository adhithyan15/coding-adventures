## Unreleased - 2026-11-02

### Fixed

- Every `compile_file_*` entry point read its source with
  `std::fs::read_to_string`, so a huge file was read whole, `/dev/zero`
  never returned and a FIFO blocked forever (#18474 security review). The
  21 sites now read through `read_source_file` (`bounded-source-file`):
  only a regular file, never blocking, size checked first, at most
  `MAX_SOURCE_FILE_BYTES` (16 MiB, LANG84 C5's trusted ceiling) + 1 bytes
  read. Each frontend still applies its own smaller budget. Errors stay
  `LangAotError::Io` (`InvalidInput` / `FileTooLarge`).
- Test `source_files_are_read_within_the_budget`.
