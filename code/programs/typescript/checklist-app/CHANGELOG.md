# Changelog

## [0.1.0] - Unreleased

### Added

- **Template CRUD** — create, edit, and delete checklist templates in memory
- **Decision-tree items** — two item types: `check` (a simple step) and
  `decision` (a yes/no question with two branches); nesting is unlimited
- **Instance execution** — each "Run" deep-clones a template into an
  independent instance; two runs of the same template do not share state
- **flattenVisibleItems** — tree-walking algorithm that returns only the
  items on the active decision path; hidden branches are never shown
- **Stats computation** — pure function over the final instance state:
  completion rate, total/checked item counts, decision count, elapsed time
- **Four screens** — Template Library, Template Editor, Instance Runner,
  Stats View; navigation via URL hash router (`#/path`)
- **Seed data** — three pre-loaded example templates demonstrating flat,
  branching, and nested-decision checklists
- **i18n** — all UI strings externalised to `src/i18n/locales/en.json` via
  `@coding-adventures/ui-components` i18n singleton
- **Unit test suite** — 30+ tests for `state.ts` (95%+ coverage); component
  tests with React Testing Library for all four screens
- **Tech stack** — React 19, Vite 6, Vitest 3, TypeScript strict mode,
  `@coding-adventures/ui-components` for shared theme and i18n
