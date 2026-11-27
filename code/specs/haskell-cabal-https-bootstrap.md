# Haskell CI Cabal HTTPS bootstrap

Status: implementation contract. This change applies only to the Haskell CI setup and package-index sequence. It does not alter package metadata, dependencies, or Cabal's signed repository metadata.

## Goal and security boundary

CI uses canonical `https://hackage.haskell.org/` while retaining Cabal's built-in secure mode, shipped trust roots, and default threshold. Do not rewrite signed index metadata, keys, hashes, package pins, or dependency versions. Do not change a user/global Cabal configuration or expose inherited environment values. Cabal's default cache and store locations remain in effect; this is a repository endpoint choice, not a cache reset.

## Workflow behavior

The pinned `haskell-actions/setup` step sets `cabal-update` to false so it does not contact the old mirror. Immediately after setup, a short Bash block in `.github/workflows/ci.yml` creates a unique directory with `mktemp -d` beneath `RUNNER_TEMP` and writes a new private config. It receives the action's `cabal-store` output as `CABAL_STORE_DIR`; it does not read or modify the user/global config. The base config at that directory's `config` path has these exact UTF-8/LF bytes:

```
repository hackage.haskell.org
  url: https://hackage.haskell.org/
  secure: True
```

After the base stanza it appends `store-dir: ${CABAL_STORE_DIR}` on every platform. On Windows it also appends `install-method: copy` and `overwrite-policy: always`, matching the pinned setup action's platform settings. Tests compare complete platform-specific config bytes, including LF newlines. The block requires nonempty `RUNNER_TEMP`, `GITHUB_ENV`, `CABAL_STORE_DIR`, and `RUNNER_OS`, rejects CR or LF in every path/value it writes, and runs with the workflow's fail-fast Bash settings. `mktemp` must create a new directory rather than reuse or overwrite one. The runner's native Windows temp path must remain usable from Git Bash and by later Cabal processes. Append one `CABAL_CONFIG=<native absolute path>` assignment to `GITHUB_ENV`; never replace its existing contents. The config persists for later steps. Any validation, directory creation, config write, or environment append failure fails the step; do not continue with Cabal's prior global config. Diagnostics must not print environment contents.

The setup action cannot contact the old mirror because its `cabal-update` input is false. After setup, the existing `Verify Haskell package index` step runs `cabal update` first and then retains its current `cabal list` verification. Later Cabal commands inherit the same config through `GITHUB_ENV`; do not depend on repeatedly passing `--config-file`.

Keep the truthful Haskell step label and existing `cabal list` check in the changed workflow hunk so the CI diff classifier recognizes the Haskell toolchain. Avoid unsafe/shared workflow controls. Verify the actual plan for the complete workflow diff: Haskell is selected, unrelated toolchains are not selected, and `full_build` remains false for an ordinary pull request. The workflow sentinel's existing required-gate behavior remains unchanged.

## Tests

A focused test extracts and executes the exact Bash block from the workflow with the same fail-fast options used by GitHub. It covers a temp path containing spaces, existing `GITHUB_ENV` content, exact config bytes/newline/no-BOM, native absolute config path, missing/malformed inputs, CR/LF injection rejection, config-write and environment-append failures, and failure propagation before Cabal runs. It verifies setup disables its update, bootstrap follows setup, and the new `cabal update` follows bootstrap and precedes the existing index query. Tests must not mutate user Cabal configuration or dump environment values.

For integration, run the extracted workflow block, read back its emitted `CABAL_CONFIG` assignment, and invoke Cabal using only that environment value (no `--config-file`). Reuse the isolated signed index where available, but prove a package archive absent before the command is fetched through the HTTPS configuration. Record separately whether the signed index was reused and whether the package archive was cold. Preserve the earlier cold test using explicit `--config-file` as separate supporting evidence; it does not prove the workflow's environment-only handoff.

Use Cabal 3.10.3.0 and GHC 9.4.8 when available. Preserve exact commands, versions, exits, source/input hashes, and sanitized logs. Run focused workflow-classifier tests, emitted-plan checks, and affected source-input/cache checks. At least one actual Cabal command must inherit the config via `CABAL_CONFIG`. No registry upload or rank-statistics source changes are part of this item.
