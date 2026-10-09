# Changelog

## Unreleased

- `vault put --destination HOST:PORT` (repeatable, lowercased, validated as
  D18U U-E8). It is required for `--mode leased|both` and refused for
  `--mode direct` (U-C4a). `VaultPut` carries `allowed_destinations` into the
  stored policy.
- Add `vault put|delete|list` as typed local actions (`CliAction::Vault`,
  `VaultCommand`, `VaultPut`) for D18U provisioning.
- `--mode`, `--tier`, and exactly one of `--allow-agent`/`--any-agent` are
  required, so no policy field has a default (U-C4). Each rule is checked again
  in the typed boundary, so it does not rely only on the parser spec.
- `--allow-agent` values use the service-registry host-name grammar (U-C5).
- The allow-list is checked against the sealed record's bounds before any
  secret is read.
- `VaultPut::secret_bytes` strips exactly one trailing `\n` or `\r\n` unless
  `--raw` is given (U-C3).
- Add `CliError::SecretName`.

## 0.1.0

- Add a declarative host-lifecycle command surface over an authenticated Chief daemon client.
- Validate host identity and package hashes before dispatch.
- Keep credentials, endpoints, terminal access, and connection setup outside argv parsing.
- Add deterministic pretty-JSON result rendering.
- Add a typed local `install-daemon` action without introducing filesystem or
  process authority into the command parser.
- Add typed `wire` and `unwire` commands over the authenticated daemon client,
  including repeatable exact channel bindings and all-or-none Level 1 model
  settings.
