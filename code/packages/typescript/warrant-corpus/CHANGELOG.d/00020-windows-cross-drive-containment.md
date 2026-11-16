## Unreleased

- **Windows containment fix.** The "corpus must lie inside the repository root"
  check used `relative()` plus `startsWith("..")`. Across Windows drives
  (repository on `D:`, corpus on `C:`) `path.relative` returns an absolute path
  with no leading `..`, so an outside corpus was accepted. The check now lives in
  `src/containment.ts` (`relativeInside`, taking a `path` module) and also rejects
  absolute results; it tests `..` or `..<sep>` so a `..foo` directory is not
  mistaken for an escape. Used by `prepare`, `validate-manifest` and the
  package-source check in `binding.ts`.
- `tests/containment.test.ts` drives `path.win32` (`D:\repo` vs `C:\tmp\x`) so the
  Windows case is covered on Linux.
- `runner.src_sha256` changed with the source; the `ct-compare` manifests were
  regenerated. No rows, counts or digests other than the runner digest changed.
