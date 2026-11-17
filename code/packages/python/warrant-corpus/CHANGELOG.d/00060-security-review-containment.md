## Unreleased

- **Security review of WD-100 (containment).** Separators are now normalised
  before the device-namespace rule, so `\/?\D:\r`, `/\?/D:/r`, `\\?/D:\r` and
  `//?/D:/r` are refused as the device paths they are (a mixed-separator device
  root and child used to compare equal and come back inside). A POSIX path with
  exactly two leading slashes is refused (it was kept as `//` by `normpath` and
  disagreed with the other lanes); three or more collapse to one.
- `relative_inside` has no variant that skips the device rule: with no `flavor`
  it uses `HOST_FLAVOR` (`win32` on Windows, `posix` elsewhere), so
  `corpus_relative` and the binding's source-root check apply the rules the
  shared fixture drives. A test proves the production call path does.
- The fixture grew to 397 cases (device and `//` prefixes, `\\?\UNC\`, device
  roots, 5,000-digit integers, nesting to 5,000, `List`/`Option` as
  `unsupported-type`); a 5,000-digit integer rejects as `malformed-json` (it was
  already caught as a `ValueError`; now pinned).
- `runner.src_sha256` changed; the `ct-compare` manifest is regenerated.
