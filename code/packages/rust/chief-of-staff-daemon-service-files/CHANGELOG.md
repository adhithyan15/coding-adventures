# Changelog

## 0.1.0 - 2026-08-03

- Add deterministic launchd LaunchAgent, systemd user-service, and Windows Task
  Scheduler definitions for the Chief daemon.
- Validate normalized absolute executable and configuration paths before
  rendering platform files.
- Encode login startup, least-privilege user scope, cooperative Unix shutdown,
  single-instance execution, and crash restart policy without shell wrappers.
