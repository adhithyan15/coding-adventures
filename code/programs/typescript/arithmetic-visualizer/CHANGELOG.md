# Changelog

## 0.3.0 — The ALU tab

### Added

- **Tab 3: The ALU** — 8-bit Arithmetic Logic Unit with all 6 operations
- `OperationSelector` — radio button group with Arithmetic (ADD, SUB) and Logic (AND, OR, XOR, NOT) grouping
- `ResultDisplay` — 8-bit result as bit cells + decimal + hex + four condition flags
- `FlagIndicator` — visual condition flag display (dot + abbreviation + description)
  - Zero (Z), Carry (C), Negative (N), Overflow (V)
- `ALUView` container with operation selector, operand inputs, result display
  - B input hidden for NOT (unary operation)
  - Auto-computes on input/operation change
- 14 new i18n strings covering ALU operations and flags
- ALU-specific CSS (operation selector, result bits, flag indicators)
- 14 new tests (51 total)

### Changed

- `App.tsx` now renders ALUView when ALU tab is active

## 0.2.0 — Everything is Addition tab

### Added

- **Tab 2: Everything is Addition** — the central insight of computer arithmetic
- `SubtractionView` — interactive 3-step two's complement transformation
  - Step 1: Show A − B as the problem
  - Step 2: Negate B → NOT(B) + 1 = two's complement of B, with bit-by-bit display
  - Step 3: Add A + (−B) through the SAME ripple-carry adder, with per-bit trace table
  - Educational callout explaining why `NOT(x) + 1 = −x`
- `MultiplicationView` — shift-and-add algorithm with long multiplication grid
  - 4-bit inputs with MSB-first grid display (like pencil-and-paper multiplication)
  - Per-step partial products: shows shifted multiplicand when bit=1, skip when bit=0
  - Step trace table with running totals
  - Active rows highlighted, skipped rows dimmed
  - Educational callout about AND gates and conditional additions
- `EverythingIsAddition` container stacking both views
- 17 new i18n strings covering subtraction and multiplication content
- Addition-specific CSS (transformation steps, long multiplication grid, callouts)
- 17 new tests (37 total)

### Changed

- `App.tsx` now renders EverythingIsAddition when addition tab is active

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
