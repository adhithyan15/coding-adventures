# Changelog

- Reject duplicate JSON object members in raw report ingress, including escaped aliases.
- Use the shared `arch` metadata field and omit private checksum metadata so
  native CLI reports replay in the other language implementations.

## 0.1.0

- Add paired batch collection, ratio analysis, report replay and the indexed-array CLI.
- Preserve signed-64-bit integer fields in memory and through JSON.
