# Changelog

## 0.1.0 - 2026-08-03

- Add the concrete `chief-of-staff` executable over the declarative CLI core.
- Compose strict local configuration, owner-only credential loading,
  authenticated loopback WebSocket dispatch, and deterministic output.
- Add `install-daemon` composition for launchd, systemd user services, and
  Windows Task Scheduler.
- Expose the CLI core's typed authenticated pipeline `wire` and `unwire`
  commands without adding credentials or endpoints to argv.
