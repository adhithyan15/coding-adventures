## [0.4.0] - 2026-11-24 (security review of PIF-002)

### Fixed

- Denial of service through one huge legal name. `Name` hashed and compared by walking its whole chain and every byte on each call, and the expression store, the environment and every `HashMap<Name, _>` do that per lookup; 32 components of 64 KiB (about 2 MiB, within the per-component limit) referenced by millions of `econst` records cost hours of CPU in a file under 64 MiB. `Name` now caches a structural hash in every node at construction (prefix hash plus the new component); `Hash` writes it, `Eq` is pointer equality, then the hash, and only then an iterative walk; `Ord` is the same order as before, iterative. Unit tests (`src/name_tests.rs`) cover equal-implies-equal-hash, collisions forced with a test-only constructor, ordering, and constant-time hashing.
- The PIF reader bounds the total decoded bytes of a whole name: new `PifLimits::name_bytes` (default 4,096, numeric components count 8), enforced at `nstr`/`nnum` insert as `LimitExceeded("name bytes")`. VER32 section 7 needs a matching "total name bytes" row (cross-spec follow-up).
- Tests: the hostile file is refused at insert; with the limit raised, 100,000 references to a 2 MiB name finish in well under a second (the old code did not finish in 40 s).

### Changed

- `tests/ceiling.rs` lists `name_tests.rs` as unbudgeted test code.
