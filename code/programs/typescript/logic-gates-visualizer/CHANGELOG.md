# Changelog

## 0.4.0 — Sequential Logic tab (all 4 tabs complete!)

### Added

- **Tab 4: Sequential Logic** — the leap from combinational to memory
- `SrLatchDiagram` — interactive SR latch with cross-coupled NOR gate SVG
  - Toggle Set/Reset inputs, see Q/Q̄ hold state via feedback
  - Forbidden state (S=R=1) warning with red indicator
  - Truth table with Set/Reset/Hold/Forbidden actions
- `DFlipFlopDiagram` — master-slave D flip-flop with clock pulse
  - Set data input, pulse clock to capture (rising edge behavior)
  - SVG shows master/slave latch configuration
  - Displays last capture event
- `CounterView` — 4-bit binary counter with auto-step mode
  - Manual step, auto-step (500ms interval), and reset buttons
  - Visual bit cells (B3-B0) with decimal display and max value
  - Counter wraps from 1111 → 0000
- `SequentialLogic` container stacking all three circuits
- 18 new i18n strings, 20 new tests (94 total)
- Sequential-specific CSS (state indicators, clock pulse button, counter display, counter controls)

### Changed

- `App.tsx` now renders SequentialLogic when Sequential tab is active
- All 4 tabs are now fully implemented — no more placeholders!

## 0.3.0 — Combinational Logic tab

### Added

- **Tab 3: Combinational Logic** — three interactive circuit visualizations
- `MuxDiagram` — 2:1 multiplexer with D0/D1 data inputs + select line
  - Trapezoid MUX symbol with dashed selection path showing which input is routed
  - Truth table highlighting which data input is selected
- `DecoderDiagram` — 2-to-4 decoder (binary → one-hot conversion)
  - SVG with 4 output lines, active output highlighted with green dot
  - Full truth table with current input combination highlighted
- `EncoderDiagram` — 4-to-2 priority encoder (highest active input wins)
  - Star marker on winning input, binary output + valid flag display
  - Handles multiple simultaneous active inputs (priority arbitration)
- `CombinationalLogic` container stacking all three circuits vertically
- 11 new i18n strings covering all combinational circuit content
- 19 new tests (74 total), all passing

### Changed

- `App.tsx` now renders CombinationalLogic when Combinational tab is active

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
