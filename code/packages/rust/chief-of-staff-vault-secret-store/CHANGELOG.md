# Changelog

## 0.1.0 — Unreleased

- **New crate**: the D18U sealed secret record. It is the format that lets a
  Chief of Staff vault secret, and the VLT06 admission policy it carries, live
  on disk in `vault-sealed-store`. Before this crate nothing serialized between
  `ChiefVaultRuntime` and the sealed store, so the vault could be populated only
  by test code. That gap blocked backlog item P1.4 (#13980).
- `encode_record` and `decode_record` implement the version-1 envelope:
  `CHIEFSEC` magic, version byte, tier, mode, rotation time, a canonical
  allow-list, then the payload. Every field is bounded. Decoding is total and
  closed, and rejects unknown tags, truncation, trailing bytes, out-of-order or
  duplicate agent ids, empty allow-lists and empty payloads.
- `SecretName` restricts names to `[a-z0-9][a-z0-9._-]{0,119}` with no `..`,
  because the name is used verbatim as a storage key. `storage-fs`
  hex-encodes keys into file names, so 120 bytes becomes 240 characters,
  under `NAME_MAX`.
- `ChiefSecretStore` over an already-unsealed `SealedStore`: `put` (overwrite is
  rotation), `delete`, `names`, `load_all` (all-or-nothing, bounded at 1024
  records) and `register_all`. `register_all` decodes everything before it
  registers anything, so a corrupt record leaves the runtime untouched.
- Listing follows the backend's `next_cursor` through the new
  `SealedStore::list_page`, and never treats a short page as the end.
  `storage-fs` can return a short page with more records still to come.
- Documents that the format does not stop rollback. A writer of the storage
  directory can restore an older valid record, so the directory must be
  owner-only until freshness binding (P1.20) lands.
