# Changelog

## 0.1.0

- Parse the D18 Chief TOML schema through the repository-owned fallible parser.
- Reject duplicate, missing, unknown, ill-typed, and unsafe configuration.
- Require loopback-only binding and validate all timeout and path invariants.
- Resolve explicit home-relative paths without consulting process environment.
- Require explicit daemon port, state root, credential path, and host executable
  composition settings.
- Add bounded, canonical, duplicate-free agent, channel, package, and model tier
  assignments to the closed privilege schema.
- Accept an optional normalized Tier 1 notification-helper path for shell-free
  production approval composition.
- Accept an independently optional normalized Tier 2 biometric-helper path for
  reviewed native-authenticator composition.
- Accept an independently optional normalized Tier 3 hardware-key-helper path
  for reviewed physical-authenticator composition.
- Require privilege deadline declarations to equal the Trust Checker-owned
  canonical 5/30/60-second policy instead of accepting ignored alternatives.
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
- Accept an all-or-none Axis pairing tuple for one exact bridge, owner-only KEK,
  and exact-length owner-only username/password files only with explicit
  in-process Vault custody.
- Accept an all-or-none ZoneMinder pairing tuple for one exact NVR bridge,
  owner-only KEK, and exact-length owner-only username/password files only with
  explicit in-process Vault custody.
- Accept an all-or-none Reolink pairing tuple for one exact bridge and pinned
  canonical network target, owner-only KEK, and exact-length owner-only
  username/password files only with explicit in-process Vault custody.
- Accept optional `restart_window` (milliseconds, at most one day) and
  `max_restarts_per_window` keys under `[hosts.defaults]`. Omitting them keeps
  the reconciler's defaults, so configs written before D18R R2 load unchanged.
  A window past the ceiling is refused rather than saturating into one that
  never elapses.
