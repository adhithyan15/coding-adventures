# Changelog

## Unreleased

- Add a bounded regular-file portable-artifact reader and reuse the fixed
  hidden import-passphrase prompt through CLI composition.
- Add fixed hidden portable-export passphrase and confirmation prompts with
  constant-time comparison.
- Add create-new durable encrypted-artifact persistence with Unix mode `0600`,
  no final-path replacement, and best-effort incomplete-file cleanup.
- Add fixed secure-note title and hidden body prompts, reusing the bounded,
  echo-restoring, wipe-on-drop controlling-terminal input boundary.

## 0.1.0

- Add fixed-prompt echoed UTF-8 login metadata collection with per-field
  bounds, control rejection, and optional username/URL handling.
- Add a fixed hidden login-password prompt and byte-accurate Windows UTF-8
  bounds.
- Add fixed-prompt, bounded secret collection from the controlling terminal on
  Unix and the attached console on Windows.
- Add echo restoration, oversized-line draining, zeroizing ownership, and
  constant-time new-passphrase confirmation.
- Add a stable, payload-free OS entropy adapter over `csprng`.
