# Changelog

## 0.2.0 — 2026-03-28

### Added

- `CalendarView` component — generic read-only monthly calendar grid
  - Accepts any `T extends CalendarItem` (requires `id` + `dueDate: string | null`)
  - `renderItem` render prop for caller-controlled item display inside day cells
  - Month navigation (prev/next buttons, "Today" jump button)
  - Items indexed into `Map<YYYY-MM-DD, T[]>` for O(1) per-cell lookup
  - Today's cell highlighted with `aria-current="date"`
  - Days from adjacent months shown with reduced opacity ("other-month" modifier)
  - Full ARIA: `role="region"`, `role="grid"`, `role="gridcell"`, `role="columnheader"`, `aria-label` per cell
  - `initialYear` / `initialMonth` props for override (default: current date at mount)
  - 17 unit tests covering navigation, item placement, edge cases
- `calendar-view.css` — dark theme stylesheet using `theme.css` CSS custom properties

## 0.1.0 — 2026-03-22

### Added
- `TabList` component — WAI-ARIA tablist with full keyboard navigation (ArrowRight/Left, Home, End)
- `SliderControl` component — accessible range input with label, value display, and ARIA attributes
- `useTabs` hook — generic tab state management with keyboard handler
- `useAnimationFrame` hook — requestAnimationFrame loop with delta timing
- `useAutoStep` hook — rate-limited stepping for simulations
- `useReducedMotion` hook — detects `prefers-reduced-motion` preference
- `useTranslation` hook + `initI18n` — lightweight i18n system with JSON locale files
- `theme.css` — shared dark theme CSS variables for visualization apps
- `accessibility.css` — screen-reader utilities, focus rings, reduced motion, tab/slider styling
