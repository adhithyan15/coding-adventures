# Changelog

## 0.1.0 - 2026-10-29

- Add exact allocation-free TLS 1.3 alert decoding and caller-buffer encoding.
- Preserve untrusted legacy levels and unknown description values while
  deriving fail-closed dispositions only from the description.
- Require standard error alerts to use the fatal level when encoding.
- Add payload-free stable failures, capability metadata, documentation, and
  exhaustive alert-registry and framing tests.
