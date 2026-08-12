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
