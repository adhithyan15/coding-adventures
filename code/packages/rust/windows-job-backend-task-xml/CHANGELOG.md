# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-17

### Added

- Task Scheduler XML rendering for one-shot, recurring, calendar, login, and
  boot triggers
- `schtasks` install-plan generation for registering tasks from repository-owned
  XML
- Concurrency policy mapping to Windows multiple-instance behavior
- Explicit rejection of environment injection, sub-minute interval schedules,
  and stdin payloads in the pure XML backend
- Renamed the package to `windows-job-backend-task-xml` for clearer OS scoping
