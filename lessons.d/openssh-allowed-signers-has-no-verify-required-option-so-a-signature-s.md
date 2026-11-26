---
category: Security boundaries
---

# OpenSSH allowed_signers has no verify-required option, so a signature's PIN flag must be checked by parsing the SSHSIG blob

The VER31 design said a release signer's line in `allowed_signers` should
carry `verify-required`, so that every signature needed the security key's PIN
as well as a touch. That option exists in `authorized_keys` (sshd) but **not**
in `allowed_signers`. OpenSSH 10.3's `ssh-keygen(1)` documents only
`cert-authority`, `namespaces`, `valid-after` and `valid-before` there. A file
written to the spec would either be rejected or have the option silently ignored.

Key creation (`ssh-keygen -t ed25519-sk -O verify-required`) only tells the
*signing client* to ask for a PIN; `ssh-keygen -Y verify` never checks the
resulting flags. So `release_gate.py` parses the SSHSIG blob itself (magic,
version, public key, namespace, reserved, hash, then for `sk-*` keys the
signature string's flags byte) and refuses a signature without user presence
(`0x01`) or user verification (`0x04`). It also refuses any `allowed_signers`
option other than `namespaces=`.

Before writing a security option into a spec, check the man page for the
*exact* file format it goes in. Similar-looking OpenSSH files (`authorized_keys`,
`allowed_signers`, `known_hosts`) take different option sets.
