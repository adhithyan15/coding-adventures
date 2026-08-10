# Changelog

## 0.1.0

- Add fixed-prompt, bounded secret collection from the controlling terminal on
  Unix and the attached console on Windows.
- Add echo restoration, oversized-line draining, zeroizing ownership, and
  constant-time new-passphrase confirmation.
- Add a stable, payload-free OS entropy adapter over `csprng`.
