# Changelog

## 0.1.0 — Complete ENIAC Visualizer

### Added

- **Tab 1: The Triode Switch** — vacuum tube triode as a digital on/off switch
  - SVG triode diagram with cathode, grid, plate
  - Grid voltage slider (-15V to +5V)
  - Live plate current readout and conducting/cutoff state
  - Comparison callout: triode vs MOSFET
- **Tab 2: Decade Ring Counter** — 10 vacuum tubes = one decimal digit
  - 10 TubeIndicator components in a row, one glowing amber when "on"
  - +1 Pulse button, Set Digit dropdown, Auto Pulse mode
  - Carry detection on 9→0 wraparound with visual flash
- **Tab 3: ENIAC Accumulator** — multi-digit decimal addition
  - 4 decades (ones, tens, hundreds, thousands) with 40 tubes total
  - Number input + Add button with carry propagation
  - Per-digit trace table showing pulses, before/after, carry, step sequence
  - Overflow detection
- **Tab 4: ENIAC vs Binary** — side-by-side comparison
  - Left panel: ENIAC decimal with ring counter carry chain
  - Right panel: modern binary with 14-bit ripple-carry adder
  - Comparison table: representation, method, tube count
  - Adjustable operands
- Full i18n (58 strings), dark theme CSS, responsive layout
- 32 tests across all 4 tabs
