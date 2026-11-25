# Changelog

## 0.1.0

- Added a strict guest envelope, duplicate-aware raw JSON replay for core
  reports and envelopes, shared CLI parsing, and calibrated raw empty-entry
  diagnostics.
- The external consumer fixture remains explicitly excluded from independent
  package discovery because the Mosaic and Starlark BUILD acceptance steps
  compile and run it to compare core and envelope reports.
