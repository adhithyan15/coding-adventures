# Haskell CI Cabal HTTPS bootstrap

Status: implementation contract. This change applies only to the Haskell CI setup and package-index sequence. It does not alter package metadata, dependencies, or Cabal's signed repository metadata.

## Goal and security boundary

CI uses canonical `https://hackage.haskell.org/` while retaining Cabal's built-in secure mode, shipped trust roots, and default threshold. Do not rewrite signed index metadata, keys, hashes, package pins, or dependency versions. Do not change a user/global Cabal configuration or expose inherited environment values. Cabal's default cache and store locations remain in effect; this is a repository endpoint choice, not a cache reset.

## Workflow behavior

Before `haskell-actions/setup`, a short Python block in `.github/workflows/ci.yml` creates a unique config file beneath `RUNNER_TEMP`. It resolves the temporary root and newly created file, rejects a path outside that root or containing CR/LF, and writes UTF-8 without BOM with LF newlines:

```
repository hackage.haskell.org
  url: https://hackage.haskell.org/
  secure: True
```

Creation must be exclusive (no overwrite or symlink/reparse escape). It appends one `CABAL_CONFIG=<native absolute path>` assignment to `GITHUB_ENV`, preserving existing bytes and lines. The config file persists for later steps. Failure to validate required runner inputs, create/write the file, or append the environment entry fails the step; do not report success if the environment entry was not written. Diagnostics may identify the failed operation but must not print environment contents.

Set the setup action's `cabal-update` to false. In the following existing `Verify Haskell package index` step, run `cabal update` before the existing `cabal list` verification. Later Cabal commands inherit `CABAL_CONFIG` from `GITHUB_ENV`; do not depend on repeatedly passing `--config-file`.

Keep the truthful Haskell step label and existing `cabal list` check in the changed workflow hunk so the CI diff classifier recognizes the Haskell toolchain. Avoid unsafe/shared workflow controls. Verify the actual plan for the complete workflow diff: Haskell is selected, unrelated toolchains are not selected, and `full_build` remains false for an ordinary pull request. The workflow sentinel's existing required-gate behavior remains unchanged.

## Tests

A focused test extracts and executes the exact inline Python block from the workflow. It covers a temp path containing spaces, existing `GITHUB_ENV` content, exact config bytes and newline/BOM properties, native absolute path export, missing/malformed inputs, write/append failures, and no success after append failure. Test that path CR/LF cannot inject extra environment assignments and that the created config resolves beneath the canonical temp root. Tests must not mutate user Cabal configuration or dump the environment.

For integration, read back the assignment emitted by the exact block and run Cabal with only that `CABAL_CONFIG` value (no `--config-file`). Reuse the isolated signed index where available, but prove a package archive absent before the command is fetched successfully through the HTTPS configuration. Record separately whether the signed index was reused and whether the package archive was cold. Preserve the earlier cold test using explicit `--config-file` as separate supporting evidence; it does not prove the workflow's environment-only handoff.

Use Cabal 3.10.3.0 and GHC 9.4.8 when available. Preserve exact commands, versions, exits, source/input hashes, and sanitized logs. Run focused workflow-classifier tests, emitted-plan checks, and affected source-input/cache checks. At least one actual Cabal command must inherit the config via `CABAL_CONFIG`. No registry upload or rank-statistics source changes are part of this item.