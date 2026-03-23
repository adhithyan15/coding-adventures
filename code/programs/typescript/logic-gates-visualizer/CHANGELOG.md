# Changelog

## 0.1.0 — Initial scaffold + Basic Gates tab

### Added

- Project scaffold: package.json, vite.config.ts, tsconfig.json, vitest.config.ts, BUILD
- App shell with 4 tabs (Basic Gates, NAND Universality, Combinational, Sequential)
- Only "Basic Gates" tab is implemented; others show "Coming soon" placeholder
- Shared components:
  - `BitToggle` — clickable 0/1 toggle button with keyboard accessibility
  - `GateSymbol` — IEEE standard gate symbol SVGs (NOT, AND, OR, XOR, NAND, NOR)
  - `TruthTable` — interactive truth table with active row highlighting
  - `WireLabel` — inline wire value indicator (green=1, gray=0)
  - `CmosPanel` — expandable CMOS transistor implementation diagrams
- Fundamental gates tab:
  - `GateCard` — self-contained gate visualization (symbol + toggles + truth table + CMOS)
  - `FundamentalGates` — 2x2 responsive grid of NOT, AND, OR, XOR
- Full i18n: all visible text externalized to en.json
- Accessibility: aria-labels, aria-expanded, aria-current, keyboard navigation
- CSS: dark theme via ui-components, responsive grid, gate-specific styling
- Tests: BitToggle, TruthTable, CmosPanel, GateCard
