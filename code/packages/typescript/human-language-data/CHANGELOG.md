# Changelog

All notable changes to `@coding-adventures/human-language-data` are documented here.

## [0.1.0] - 2026-07-17

### Added
- Initial release — the HL01 data layer over the Human Languages curriculum.
- **Types** (`types.ts`): `Concept`, `Realization`, `Dataset`, `Taxonomy`,
  `ScriptData`/`Glyph`/`VowelSign`, `Issue`.
- **Frontmatter reader** (`frontmatter.ts`): a tiny zero-dependency parser for the
  `key: value` / `[list]` frontmatter shape the lesson schema uses (BOM- and
  CRLF-tolerant, quote-stripping, comment-skipping).
- **Parser** (`parse.ts`): `parseLesson` derives a `Realization` from lesson
  frontmatter (romanization defaults to headword for Latin scripts; gender sniffed
  from the gloss when unfielded); `buildDataset` joins content lessons through the
  taxonomy into concepts + per-language indexes.
- **Validator** (`validate.ts`): the round-trip consistency gate — resolves every
  concept tag, forbids duplicate realizations per language, checks required fields
  and field shapes, script-glyph coverage, and core-concept coverage. Errors fail
  CI; warnings/info are tolerated.
- **Queries** (`queries.ts`): `allConcepts`, `conceptsByLanguage`,
  `languagesForConcept`, `coverageByLanguage`.
- **Loader + CLI** (`loader.ts`, `cli.ts`): the filesystem boundary — reads the
  curriculum and runs `validate`. Declared `fs:read`/`fs:list` capabilities.
- Tests for the pure core (frontmatter, parse, validate, queries) plus an
  integration test that validates the **real** curriculum in CI and asserts the
  cross-language joins (e.g. `GREETING-HELLO` across all 16 tracks).

### Notes
- `data/scripts/*.json` character-breakdown data is authored incrementally in
  follow-up work; the package degrades gracefully when it is absent.
