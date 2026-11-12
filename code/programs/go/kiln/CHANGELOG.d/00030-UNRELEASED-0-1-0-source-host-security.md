## [0.1.0] - UNRELEASED

### Security

- `Host.Trusted()` no longer trusts `KILN_DEFAULT_BRANCH`: the default branch
  is read from the GitHub-generated event JSON (size-capped, regular file,
  fail closed) and the env var can only narrow it.
- `Local.Trusted()` is false; new `WritesLocalLedger()`; `Detect` returns a
  new `Untrusted` host whenever any CI indicator is present but the
  environment is not GitHub Actions.
- Reports strip C0/C1 control characters and defang any `::` / `##[`.
- Glob matching is polynomial (dynamic programming); `.kilnignore` is capped
  (64 KiB, 1000 patterns, 64 segments) and opened without following symlinks.
- Source opens go through `os.Root`; paths with `\`, `:` or Windows device
  names are refused in the Source layer.
- `git` runs with a scrubbed environment (no credentials, no `GIT_*`) and
  hooks, fsmonitor and file protocol disabled.
- `ScrubSecrets` (now `internal/envscrub`) covers more names and suffixes.
- `WorkDir()` returns an error instead of a predictable fallback; `ImageID`
  components are sanitised.
