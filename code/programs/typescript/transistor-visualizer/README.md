# Transistor Visualizer

Interactive history of the transistor — from vacuum tubes to modern CMOS. Four tabbed visualizations teach how each generation of electronic amplification works, driven by real simulation models from the `@coding-adventures/transistors` package.

## Eras

| Tab | Year | Technology | Key Concept |
|-----|------|-----------|-------------|
| Vacuum Tube | 1906 | Triode | Child-Langmuir law, thermionic emission |
| BJT | 1947 | NPN transistor | Current amplification (beta), Ebers-Moll model |
| MOSFET | 1959 | NMOS transistor | Voltage-controlled, inversion channel, threshold voltage |
| CMOS | 1963 | Complementary MOS | Zero static power, VTC curve, Moore's Law scaling |

## Running

```bash
# Install dependencies (including local packages)
cd code/packages/typescript/ui-components && npm install
cd code/packages/typescript/transistors && npm install
cd code/programs/typescript/transistor-visualizer && npm install

# Development server
npm run dev

# Run tests
npm test

# Production build
npm run build
```

## Architecture

- **Simulation models**: Pure TypeScript in `src/lib/` (vacuum tube model, particle system)
- **React hooks**: `src/hooks/` wraps the transistors package and particle system for React
- **Components**: `src/components/` organized by era, with shared components at the top level
- **Styles**: `src/styles/` with per-era CSS files importing shared theme from ui-components
- **i18n**: All visible text in `src/i18n/locales/en.json`

## Dependencies

- `@coding-adventures/transistors` — MOSFET, BJT, and CMOS simulation models
- `@coding-adventures/ui-components` — TabList, SliderControl, i18n, animation hooks, theme CSS

## Adding a New Language

1. Copy `src/i18n/locales/en.json` to a new file (e.g., `ja.json`)
2. Translate all values (keys stay the same)
3. Import the new locale in `src/main.tsx` and pass it to `initI18n`:
   ```typescript
   import ja from "./i18n/locales/ja.json";
   initI18n({ en, ja });
   ```
4. The language picker appears automatically when 2+ locales are loaded

## License

MIT
