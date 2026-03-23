# Changelog

## 0.2.0 — NAND Universality tab

### Added

- **Tab 2: NAND Universality** — interactive visualizations proving NAND is functionally complete
- `NandDerivation` component with 4 derivation types:
  - NAND → NOT: 1 NAND gate (4T) — tie both inputs together
  - NAND → AND: 2 NAND gates (8T) — NAND then invert
  - NAND → OR: 3 NAND gates (12T) — De Morgan's Law in action
  - NAND → XOR: 4 NAND gates (16T) — most complex, with shared intermediate wire
- `NandUniversality` container with intro, NAND gate card, derivations, and tradeoff note
- Interactive SVG wiring diagrams with labeled intermediate wire values
- Transistor cost comparison on each derivation (NAND-only vs native implementation)
- 20 new i18n strings covering all NAND universality content
- NAND-specific CSS (derivation cards, formula badges, cost indicators)
- 17 tests covering all 4 derivations with input toggling and output verification

### Changed

- `App.tsx` now renders NandUniversality when NAND tab is active (was placeholder)

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
