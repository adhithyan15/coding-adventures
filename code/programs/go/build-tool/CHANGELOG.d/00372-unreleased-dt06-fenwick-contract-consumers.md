## Unreleased - DT06 Fenwick contract inputs select native adapters (Go)

- Edits to the shared DT06 Fenwick spec or exact `fenwick-tree-v1` fixture now
  select both `dart/fenwick-tree` and `python/fenwick-tree` during repository
  input change detection, subject to the normal language filter.
- The exact spec filename and slash-delimited fixture prefix exclude backups,
  adjacent fixture versions, and noncanonical traversal paths. A selected
  consumer missing from discovery remains an error for the selected language.
