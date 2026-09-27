### Known limitations carried to PR-6

- **CLI integration** (`mosaic-compile --backend xaml --package-mode`)
  still pending. The CLI needs to read each dependency's
  `mosaic-package.toml`, parse it via `mosaic-package-manifest`, and
  populate the `ComponentRegistry` before invoking `from_pipeline`.
  Same status as the swiftui/qt backends.
- **Emit-ref props on component references** are surfaced as a
  comment but not wired. The host-side handler stubs and the
  package's own `Dispatch` event subscription are PR-5+ work that
  lands either at the tail end of the xaml series or in a generic
  cross-backend PR.
- **`--use-community-datagrid` flag** still inert (PR-4 carryover).

## [Unreleased] — PR-4 — HostTable + section sub-tags

