# Changelog — coding_adventures_argon2id

## 0.1.0 — 2026-04-20

### Added
- Initial implementation of Argon2id (RFC 9106) for Elixir.
- `CodingAdventures.Argon2id.argon2id/7` / `argon2id_hex/7`.
- Keyword options: `:key`, `:associated_data`, `:version`.
- Implements Argon2 v1.3 (0x13) only.
- RFC 9106 §5.3 gold-standard vector plus 18 parameter-edge tests (19 total).
