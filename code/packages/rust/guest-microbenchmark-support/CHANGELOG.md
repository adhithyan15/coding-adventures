# Changelog

## 0.1.0

- Removed the build-script compiler query; guest host-runtime metadata now
  reports `unknown` unless a future API receives an explicit compiler label.
- Declared the support library's actual monotonic-clock and replay-file reads.
- Kept report output on caller-provided writers; removed the unused helper that
  created manifest files on behalf of library callers.
- Added a strict guest envelope, duplicate-aware raw JSON replay for core
  reports and envelopes, shared CLI parsing, and calibrated raw empty-entry
  diagnostics.
- The external consumer fixture remains explicitly excluded from independent
  package discovery because the Mosaic and Starlark BUILD acceptance steps
  compile and run it to compare core and envelope reports.
