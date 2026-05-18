# chief-of-staff-tool-audit-store

`chief-of-staff-tool-audit-store` persists D18D payload-free
`ToolAuditRecord` rows through the D18A `StorageBackend` abstraction.

The crate keeps the boundary narrow:

- `chief-of-staff-tool-api` owns the audit record vocabulary
- this crate serializes those records into storage records
- callers can use in-memory, local-folder, or future storage backends without
  changing audit code

## Development

```bash
bash BUILD
```
