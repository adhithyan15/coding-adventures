## Unreleased — Windows native Perl bridge

- Use a shared Windows recipe helper to select native Strawberry even when
  Git Bash puts MSYS Perl first, then build, stage and test with that interpreter.
- Delegate Windows shim compilation and CORE import-library linking to
  `perl-bridge`, replacing interpreter-library directory guessing.
- Use the bridge's matched callback-error deallocator so a dying handler returns
  its `on_error` response and subsequent HTTP requests still work on Windows.
- Launch and clean up the real server OS process in Windows E2E tests instead
  of an emulated fork whose exec child survives the pseudo-process's cleanup.
