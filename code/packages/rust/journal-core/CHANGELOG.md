# Changelog

All notable changes to `journal-core` are documented here.

## [0.1.0] - Unreleased

### Added

- **The model** (J2a of #14416, spec `code/specs/journal-core.md`):
  `JournalState` holds several named `Journal`s (never zero) and their
  `Entry`s. An entry is dated by the civil `Date` it is *about* (wire form
  `YYYY-MM-DD`, identical to the TypeScript Journal's `createdAt`), carries
  created/updated instants, case-insensitive `Tag`s, and a star.
- **`Command` + `apply`** — create/rename/delete journals (moving or dropping
  their entries, refusing the last one), create/edit/redate/move/tag/star/
  delete entries. Every command validates before it writes, so a rejected
  command leaves the state untouched; tests assert this for every rejection.
- **Limits enforced in the core** so no host can store what another refuses
  to load: journal names ≤ 128 chars on one line and unique case-insensitively,
  titles ≤ 512 chars, bodies ≤ 1 MiB, ≤ 64 tags of ≤ 64 chars each.
  `JournalState::validate` re-checks all of them for state loaded from outside.
- **Projections**: `timeline` (day groups, newest first, total order with an
  id tiebreak), `on_this_day` (earlier years, with 29 February recalled on
  28 February in common years), `search` (all terms must match; title > tag >
  body ranking; ≤ 160-char one-line snippets cut on character boundaries via a
  folded→original offset map, with bounded work per hit), `tag_counts`, and
  `month_activity`. All take one `EntryFilter` (journal, tag, starred-only).
- Optional `serde` feature: camelCase JSON, bare-string ids, ISO dates, tags as
  their display string, commands tagged by `type`.
