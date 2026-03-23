# Changelog

## 0.1.0 — App scaffold + Binary Adders tab

### Added

- **Project scaffold**: package.json, vite.config.ts, tsconfig.json, vitest.config.ts, BUILD, index.html
- **App shell** with 4 tabs (Binary Adders, Everything is Addition, The ALU, CPU Step-Through)
  - Only "Binary Adders" tab is implemented; others show "Coming soon" placeholder
- **Shared components**:
  - `BitToggle` — clickable 0/1 toggle button with keyboard accessibility
  - `WireLabel` — inline wire value indicator (green=1, gray=0)
  - `TruthTable` — interactive truth table with active row highlighting (supports multiple output columns)
  - `BitGroup` — multi-bit input with MSB-first display and decimal conversion
- **Tab 1: Binary Adders** — three visualizations building from simple to complex:
  - `HalfAdderDiagram` — XOR + AND SVG with 2 input toggles, truth table (4 rows)
  - `FullAdderDiagram` — two half adders + OR gate SVG with 3 inputs, intermediate values, truth table (8 rows)
  - `RippleCarryDiagram` — 4 chained full adders using `rippleCarryAdderTraced()` for per-bit snapshots
    - Two 4-bit BitGroup inputs with decimal display
    - Equation display (e.g., "5 + 3 = 8")
    - SVG chain of full adder boxes with carry arrows
    - Per-adder snapshot table showing a, b, cIn, sum, cOut for each bit
    - Overflow indicator when carry-out = 1
- Full i18n: all visible text externalized to en.json
- Accessibility: aria-labels, aria-current, aria-live, keyboard navigation
- CSS: dark theme via ui-components, responsive layout
- 20 tests covering all three adder diagrams
