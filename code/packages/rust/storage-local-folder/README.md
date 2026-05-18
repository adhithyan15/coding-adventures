# storage-local-folder

`storage-local-folder` is the D18A-named local persistence backend for Chief of
Staff stores.

The lower-level `coding_adventures_storage_fs` crate already owns the STR-FILE
record format, atomic write/rename behavior, tmp-file cleanup, and restart-safe
revision recovery. This crate gives D18A callers the package name and type they
expect:

- `LocalFolderStorageBackend`
- `local_folder_storage_backend_summary()`
- `LocalFolderStorageBackendSummary`

It implements `storage_core::StorageBackend` by delegating to the STR-FILE
backend, so `ContextStore`, `ArtifactStore`, `SkillStore`, and `MemoryStore`
can all target a real local folder without learning the underlying file format.

The backend stores opaque record bodies and JSON metadata under a caller-supplied
root directory. Encryption, cross-device sync, and cross-process locks remain
layers above or beside this backend.

## Development

```bash
bash BUILD
```
