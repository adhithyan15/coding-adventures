# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-17

### Added

- Portable `JobSpec`, `JobAction`, `JobTrigger`, and supporting policy types
- `InstalledJob`, `JobStatus`, `JobRunReceipt`, and `JobExitStatus` observability
  records for D18C runtime APIs
- Installed-job and job-status query options for filtering read-side runtime
  views by backend, enabled state, action/trigger kind, status, run outcome,
  next-run deadline, sort order, and bounded result count
- Job-status summary counts for D18C/D18D read tools that need a compact
  runtime health rollup
- Repository-owned `InstallPlan`, `InstallFile`, and `InstallCommand` contracts
- Shared validation and error reporting for job identifiers, triggers, outputs,
  retry settings, and environment variables
- A backend trait that native scheduler adapters can implement
- Portability-report and error types for runtime-level cross-platform rejection
- Renamed the package to `os-job-core` to make its OS scheduling role explicit
