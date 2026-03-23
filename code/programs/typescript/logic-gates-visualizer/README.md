# Logic Gates Visualizer

Interactive digital logic visualizer that shows how logic gates work, from Boolean truth tables down to CMOS transistor implementations.

## What it does

This application provides an interactive exploration of digital logic gates:

- **Basic Gates tab**: Interactive NOT, AND, OR, and XOR gates with clickable input toggles, live truth table highlighting, IEEE standard gate symbols, and expandable CMOS transistor diagrams showing the physical implementation.
- **NAND Universality tab** (coming soon): Demonstrates how every gate can be built from NAND alone.
- **Combinational tab** (coming soon): Multiplexers, decoders, encoders.
- **Sequential tab** (coming soon): Latches, flip-flops, registers, counters.

## Dependencies

- `@coding-adventures/logic-gates` — Gate functions (NOT, AND, OR, XOR, etc.)
- `@coding-adventures/transistors` — CMOS transistor simulation (CMOSInverter, CMOSNand, CMOSNor)
- `@coding-adventures/ui-components` — Shared UI (TabList, i18n, hooks, theme CSS)

## Development

```bash
npm install
npm run dev      # Start dev server
npm test         # Run tests
npm run build    # Production build
```

## How it fits in the stack

This visualizer sits at Layer 1 (logic gates) of the computing stack, bridging Layer 0 (transistors, shown in the transistor-visualizer) with higher layers (arithmetic, ALU, CPU). The CMOS panels connect the gate abstraction back down to the transistor level.

## Architecture

```
src/
  main.tsx                          Entry point (i18n init, React mount)
  App.tsx                           Root component (4 tabs)
  components/
    shared/
      BitToggle.tsx                 Clickable 0/1 toggle button
      GateSymbol.tsx                IEEE standard gate symbol SVGs
      TruthTable.tsx                Interactive truth table with row highlight
      WireLabel.tsx                 Inline wire value indicator
      CmosPanel.tsx                 Expandable CMOS transistor diagrams
    fundamental/
      GateCard.tsx                  Single gate visualization card
      FundamentalGates.tsx          Tab 1: 2x2 grid of basic gates
  i18n/locales/en.json              All visible text (no hardcoded strings)
  styles/
    app.css                         Layout, header, footer
    gates.css                       Gate cards, toggles, truth tables, CMOS panels
```
