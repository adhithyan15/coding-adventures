# Changelog

All notable changes to this package will be documented in this file.

## [0.2.0] - 2026-03-28

### Added

- **Calendar view** (`#/calendar`) — read-only monthly grid showing todos by due date
  - Each todo appears as a compact priority-coloured chip in its due-date cell
  - Completed todos shown with strikethrough and reduced opacity
  - Month navigation (‹ ›) and "Today" jump button
  - Today's cell highlighted with accent colour
  - Overflow days from adjacent months shown at reduced opacity
  - Powered by the new generic `CalendarView` component from `@coding-adventures/ui-components`
- **App navigation bar** — "☰ List" and "📅 Calendar" buttons in the header
  - Active route highlighted with accent colour and `aria-current="page"`
  - `/`, `/new`, `/edit/:id` all map to "List" as the active nav item
- **`TodoCalendar` component** — wires `CalendarView` to the todo store with `useStore`
- **Calendar and chip styles** in `app.lattice` — priority-coloured chips matching the app's dark theme

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
