# Changelog

All notable changes to `coding_adventures_correlation_vector` are documented here.

Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).
This project uses [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [0.1.0] — 2026-04-06

### Added

- **CVLog** — central log class with `enabled` flag for zero-cost disabling
- **create** — born a new root CV from an origin string or as synthetic (`00000000` base)
- **contribute** — append a stage contribution to an entity's history
- **derive** — create a child entity; ID extends parent with `.M` suffix
- **merge** — combine multiple entities; ID uses SHA-256 of sorted parent IDs
- **delete** — mark entity deleted with `DeletionRecord` (entry stays in log)
- **passthrough** — record that a stage saw an entity but made no changes
- **get** — look up a CVEntry by ID
- **ancestors** — BFS walk of parent chain, nearest-first
- **descendants** — scan log for all entities descended from a given ID
- **history** — return contributions array for an entity
- **lineage** — ancestors + self, oldest-first (complete provenance chain)
- **serialize** — convert log to compact JSON via `CodingAdventures::JsonSerializer`
- **deserialize** — reconstruct log from JSON via Ruby stdlib `JSON.parse`
- **CVEntry** — struct with `cv_id`, `origin`, `parent_cv_id`, `merged_from`,
  `contributions`, `deleted`, `pass_order` fields
- **Origin** — struct with `string`, `synthetic` fields
- **Contribution** — struct with `source`, `tag`, `meta`, `timestamp` fields
- **DeletionRecord** — struct with `by`, `at` fields
- 72 unit tests, 100% line coverage
- Literate programming style throughout (Knuth-style explanations)

### Dependencies

- `coding_adventures_sha256` — ID base generation via `sha256_hex`
- `coding_adventures_json_serializer` — CVLog serialization (dogfoods the gem)

### Implementation Notes

- Global monotonic counter ensures ID uniqueness even with hash base collisions
- `pass_order` is deduplicated at both entry level and log level
- `enabled: false` skips all storage but still returns valid cv_ids
- Serialization uses `CodingAdventures::JsonValue.from_native` + `JsonSerializer.serialize`
- Deserialization uses stdlib `JSON.parse` for speed
