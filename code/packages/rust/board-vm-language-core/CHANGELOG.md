# Changelog

## Unreleased

- **Windows build.** `collect_matching_dir_paths` is only called by the Unix
  serial-device scan, so on Windows it was dead code and `-D warnings` failed the
  crate. It now carries the same `#[cfg(unix)]` gate as its only caller.
