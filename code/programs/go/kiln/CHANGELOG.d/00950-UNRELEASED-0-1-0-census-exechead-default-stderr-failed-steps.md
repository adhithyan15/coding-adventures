## [0.1.0] - UNRELEASED

### Changed (the census workflow's `exec_head` dispatch default is now true)

- `.github/workflows/kiln-census.yml`: the `workflow_dispatch` input `exec_head` defaults to `true`
  (was `false`). Census runs A' (45443745064/45443753967), B (45466205573/45466210497) and the
  classification pair (45557016876/45557181111) were all clean on the hard criteria. The scheduled
  Monday run stays `exec_head=false`: the validate step forces it explicitly, so the PR8 evidence
  does not change configuration. Input description and header comment updated; KILN04 D48 step 6
  notes updated.

### Added (a census counter for stderr-coded steps whose build step also failed)

- `explain.sidecar.strace_stderr_failed_steps`: code -> number of steps that carry that strace stderr
  code AND whose build step failed (started, non-zero root status: the same signal as
  `root_status_nonzero` and `builtOK`'s sidecar fallback). Keys come from the closed
  `StraceStderrCodes` list plus `unrecorded`, so the map is bounded. Rendered beside the other counts
  in the Markdown. Informational: in no gate; `census_not_clean.py` unchanged.
- Purpose: tell a strace race that costs one lost record from one that also takes the build step
  down and cascades into DEP-SKIPs.
- `kiln:recorder` is unchanged (census and reporting only).
