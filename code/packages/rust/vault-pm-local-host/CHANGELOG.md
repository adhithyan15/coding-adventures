# Changelog

## Unreleased

- Added `LocalVaultPaths::runtime_root`/`agent_socket_path` and
  `PreparedLocalVault::ensure_runtime_root` (VLT-PM48): a short,
  deterministic, owner-private directory beside the system temporary
  directory — not nested under `data_root`, to stay well under
  `sockaddr_un.sun_path`'s ~100-byte platform ceiling — that the local
  agent's Unix domain socket binds at. Verified or created lazily via a new
  leaf-only Unix check (`ensure_private_runtime_directory`) that trusts the
  system temp directory's own platform-owned ancestry instead of walking it,
  since both `/tmp` and `/var` are themselves symlinks on macOS.

## 0.1.0

- Added platform-standard configuration, local-data, and cache path resolution.
- Added owner-only, no-link local root preparation on Unix and Windows.
- Added a persistent owner-only lock file with non-blocking cross-process
  single-writer exclusion.
- Added exact, bounded configuration loading plus owner-only atomic initial
  creation and compare-and-exchange through the live writer guard.
- Added closed, path-free diagnostics and platform security tests.
