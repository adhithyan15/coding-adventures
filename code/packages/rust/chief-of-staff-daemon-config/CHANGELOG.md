# Changelog

## 0.1.0

- Parse the D18 Chief TOML schema through the repository-owned fallible parser.
- Reject duplicate, missing, unknown, ill-typed, and unsafe configuration.
- Require loopback-only binding and validate all timeout and path invariants.
- Resolve explicit home-relative paths without consulting process environment.
- Require explicit daemon port, state root, credential path, and host executable
  composition settings.
- Bound secure-bootstrap and graceful-stop deadlines to five minutes.
- Add optional, closed, typed data-plane declarations for exact directional
  channel-key files and exact Ollama model endpoints.
- Add optional bounded smart-home tool-grant declarations for exact Chief host
  principals, stable grant identities, issuance/expiry times, and explicit
  pending, active, or revoked lifecycle state.
- Add an optional closed Home Assistant-compatible loopback listener with a
  distinct endpoint and bounded instance name.
- Accept an optional bounded Hue mDNS interface in the smart-home table for
  Chief-owned supervised discovery.
- Accept an optional `hue_pairing_kek_path` only with explicit in-process Vault
  custody, preserving strict owner-only secret-file handling at composition.
- Accept an all-or-none ONVIF pairing tuple for one exact bridge, owner-only KEK,
  and exact-length owner-only username/password files only with explicit
  in-process Vault custody.
