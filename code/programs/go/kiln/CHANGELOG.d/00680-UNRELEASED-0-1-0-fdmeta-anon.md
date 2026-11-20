## [0.1.0] - UNRELEASED

### Changed (trace: metadata writes on anonymous descriptors, KILN04 D41)

- `fchmod`, `fchown`, `fsetxattr` and `fremovexattr` on a descriptor whose -y
  annotation starts with `pipe:[`, `socket:[` or `anon_inode:` no longer give
  `trace-incomplete:fd-path` (dotnet's `fchmod(<socket>, mode) = 0` refused csharp
  actions in the real-build census). They produce no event, whatever the result.
  A bare descriptor number, any other non-absolute annotation and AT_FDCWD stay
  `fd-path`; an absolute path is unchanged.
- The prefix test is `trace.AnonymousFD`, now the single definition shared by the
  trace model and the classifier (`traced.anonymousFD` delegates to it).
- Pass 2 is shared, so the old scanner and the grammar reader change together;
  the cross-check stays equal.
- `RecorderVersion` 8 to 9: the accept set grows, so every weak key changes.

### Added

- `internal/trace/fdmeta_anon_test.go`; FuzzLineDiff and FuzzParseDiff seeds.
