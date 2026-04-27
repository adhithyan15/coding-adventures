# Changelog

## Unreleased

### Changed

- `CanvasTable` now renders through `@coding-adventures/paint-vm-canvas` instead of `@coding-adventures/draw-instructions-canvas`. The table builds a `PaintScene` (paint-instructions IR) which carries the same primitives used by the rest of the codebase. Dropped the draw-instructions dependencies entirely. No API or visual change — identical columns/rows/grid/text render.
- Internal helpers renamed: `toDrawAlign` → `toPaintAlign` (mapping now produces `"start" | "center" | "end"` to match PaintText.text_align and Canvas 2D textAlign spelling; the old "middle" spelling is gone).
- Added a small `makeCanvasFontRef(family, size, weight)` helper that encodes table text as a `canvas:<family>@<size>:<weight>` font_ref per spec TXT03d.

This is the first step in retiring the `draw-instructions` family of packages in favour of `paint-instructions` (PaintText made draw-instructions' only non-overlapping feature — text rendering — redundant).

## 0.3.0 — 2026-03-29

### Added

- `Table` component — unified entry point with `renderer` prop ("html" | "canvas")
- `DataTable` component — HTML `<table>` backend with full semantic markup
  - `<thead>/<tbody>/<th scope="col">/<td>` for native screen reader support
  - BEM class names (`.table__cell--header`, `.table__cell--align-right`, etc.)
  - Scrollable region wrapper (`role="region"`, `tabindex="0"`) for keyboard scrolling
  - Column widths, alignment modifiers, optional caption
- `CanvasTable` component — Canvas 2D rendering backend with ARIA grid overlay
  - DPR-aware rendering for crisp text on retina displays
  - CSS custom property theme bridge via `useCanvasTheme` hook
  - Transparent ARIA grid overlay with `role="grid"`, `role="row"`, `role="gridcell"`
  - `aria-rowcount`, `aria-colcount`, `aria-rowindex`, `aria-colindex` for full position reporting
  - Keyboard navigation via `useGridKeyboard` hook (Arrow keys, Home/End, Ctrl+Home/End)
  - ResizeObserver for responsive canvas sizing
- `useCanvasTheme` hook — reads CSS custom properties from a DOM element for Canvas drawing
- `useGridKeyboard` hook — WAI-ARIA grid keyboard navigation with roving tabindex
- Shared types: `ColumnDef<T>`, `CellAlignment`, `RowKeyFn<T>`, `TableBaseProps<T>`, `TableProps<T>`
- `resolveCellValue` utility — consistent cell value extraction for both backends
- `table.css` — dark theme stylesheet for DataTable and CanvasTable overlay
- Spec: `code/specs/table.md` — full specification covering both backends and accessibility
- Column resizing via drag-to-resize handles on both backends
  - `resizable` prop enables resize handles on header cells
  - `onColumnResize` callback fires with column id and new width
  - Mouse drag: grab the column border and drag to adjust width
  - Keyboard: focus the resize handle, use Arrow keys (10px) or Shift+Arrow (50px)
  - Screen reader: `role="separator"` with `aria-valuenow`, `aria-valuemin`, `aria-label`
  - RTL support: drag delta and arrow keys flip automatically based on text direction
  - `useColumnResize` hook manages drag lifecycle and keyboard resize for both backends
  - Minimum column width: 40px

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
