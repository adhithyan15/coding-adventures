# Changelog

## 0.1.0

- Parse the D18 Chief TOML schema through the repository-owned fallible parser.
- Reject duplicate, missing, unknown, ill-typed, and unsafe configuration.
- Require loopback-only binding and validate all timeout and path invariants.
- Resolve explicit home-relative paths without consulting process environment.
