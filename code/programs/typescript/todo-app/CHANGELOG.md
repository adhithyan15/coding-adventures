# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-03-28

### Added

- Initial app scaffolding with Vite + React + TypeScript
- Offline-first IndexedDB persistence via `@coding-adventures/indexeddb`
- Flux state management via `@coding-adventures/store`
- Lattice CSS superset for styling (via `@coding-adventures/vite-plugin-lattice`)
- Full CRUD: create, read, update, delete todos
- Status lifecycle: todo → in-progress → done
- Priority levels: low, medium, high, urgent
- Free-form categories with auto-suggestion
- Due date tracking with overdue/due-today badges
- Search, filter (status/priority/category), and sort
- Clear completed todos
- Seed data for first-time visitors
- Dark theme with glassmorphism and micro-animations
- Electron wrapper for desktop app packaging
- Comprehensive unit tests (reducer, actions, types, persistence)
- Playwright e2e test setup
