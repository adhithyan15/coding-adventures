# Changelog

## 0.1.0

- Add strict IPP/1.1 `Get-Printer-Attributes` request framing for one fixed
  status allowlist.
- Add bounded response decoding with request-id, status, group, attribute,
  value-type, duplicate, count, and completeness checks.
