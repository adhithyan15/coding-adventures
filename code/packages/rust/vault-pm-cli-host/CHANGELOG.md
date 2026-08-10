# Changelog

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
