## [Unreleased]

### Fixed

- The two `TestRepoRootDetection` tests failed on Windows with
  `PermissionError: [WinError 32]`. They changed the working directory into a
  `tempfile.TemporaryDirectory`, whose `with` block deletes the directory
  before `monkeypatch` restores the old working directory at teardown, and
  Windows refuses to delete a process's current directory. They now use
  pytest's `tmp_path`, which is cleaned up after teardown.
