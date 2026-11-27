## [0.01] - Capability manifest

### Fixed

- `required_capabilities.json` now carries the v1 `$schema`, `version`, `package` and `justification` fields (still no capabilities), so it validates against the v1 schema. Its scope is the runtime library (`lib/`); the unshipped test suite (`t/`) reads fixtures and sources from the checkout, which the justification states.
