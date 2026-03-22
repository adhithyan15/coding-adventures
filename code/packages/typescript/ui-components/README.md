# @coding-adventures/ui-components

Shared React UI components for interactive visualizations in the coding-adventures monorepo. Extracted from the Busicom calculator app so that multiple visualization apps can share accessible tabs, i18n, animation hooks, and a consistent dark theme.

## What's Inside

| Export | Kind | Purpose |
|--------|------|---------|
| `TabList` | Component | WAI-ARIA tablist with full keyboard navigation |
| `SliderControl` | Component | Accessible range input with label, value display, and units |
| `useTabs` | Hook | Generic tab state + keyboard handler (used internally by TabList) |
| `useAnimationFrame` | Hook | requestAnimationFrame loop with delta timing |
| `useAutoStep` | Hook | Rate-limited stepping for simulations (N iterations/sec) |
| `useReducedMotion` | Hook | Detects `prefers-reduced-motion` preference |
| `useTranslation` | Hook | React hook for the i18n system |
| `initI18n` | Function | Initialize locale data at app startup |
| `translate` | Function | Look up a translation key (non-hook version) |
| `theme.css` | CSS | Shared dark theme variables |
| `accessibility.css` | CSS | Screen-reader utilities, focus rings, reduced motion |

## Installation

From another package in the monorepo:

```json
{
  "dependencies": {
    "@coding-adventures/ui-components": "file:../../packages/typescript/ui-components"
  }
}
```

Then install:

```bash
npm install
```

## Usage

### TabList

The `TabList` component implements the full WAI-ARIA Tabs pattern. It handles keyboard navigation (ArrowRight/Left/Up/Down, Home, End) and roving tabindex automatically.

```tsx
import { TabList } from "@coding-adventures/ui-components";
import { useState } from "react";

type Era = "vacuum-tube" | "bjt" | "mosfet";

function App() {
  const [activeTab, setActiveTab] = useState<Era>("vacuum-tube");

  const tabs = [
    { id: "vacuum-tube" as const, label: "Vacuum Tube" },
    { id: "bjt" as const, label: "BJT" },
    { id: "mosfet" as const, label: "MOSFET" },
  ];

  return (
    <>
      <TabList
        items={tabs}
        activeTab={activeTab}
        onActiveChange={setActiveTab}
        ariaLabel="Technology eras"
      />
      <div role="tabpanel" id={`panel-${activeTab}`}>
        {/* Panel content here */}
      </div>
    </>
  );
}
```

### SliderControl

An accessible slider with a visible label, current value display, and optional unit suffix.

```tsx
import { SliderControl } from "@coding-adventures/ui-components";
import { useState } from "react";

function VoltageControl() {
  const [voltage, setVoltage] = useState(1.5);

  return (
    <SliderControl
      label="Gate Voltage"
      value={voltage}
      min={0}
      max={3.3}
      step={0.1}
      onChange={setVoltage}
      unit="V"
    />
  );
}
```

### i18n (Internationalization)

The i18n system uses flat JSON locale files. Initialize once at startup, then use the `useTranslation` hook in components.

#### Step 1: Create locale files

```json
// src/i18n/locales/en.json
{
  "app.title": "Transistor Visualizer",
  "tabs.vacuumTube": "Vacuum Tube",
  "tabs.bjt": "BJT",
  "controls.voltage": "Gate Voltage"
}
```

#### Step 2: Initialize at startup

```typescript
import { initI18n } from "@coding-adventures/ui-components";
import en from "./i18n/locales/en.json";

initI18n({ en });
```

#### Step 3: Use in components

```tsx
import { useTranslation } from "@coding-adventures/ui-components";

function Header() {
  const { t } = useTranslation();
  return <h1>{t("app.title")}</h1>;
}
```

#### Adding a new language

1. Copy `en.json` to `ja.json`
2. Translate all values (keys stay the same)
3. Import and register: `initI18n({ en, ja })`
4. The language picker appears automatically when 2+ locales exist

### Animation Hooks

#### useAnimationFrame

Low-level hook for running code on every animation frame with delta timing.

```tsx
import { useAnimationFrame } from "@coding-adventures/ui-components";

function ParticleSystem({ running }: { running: boolean }) {
  useAnimationFrame((deltaMs) => {
    // Move particles based on elapsed time
    particles.forEach(p => p.update(deltaMs));
  }, running);

  return <canvas ref={canvasRef} />;
}
```

#### useAutoStep

Rate-limited stepping, useful for simulations that need a fixed number of iterations per second.

```tsx
import { useAutoStep } from "@coding-adventures/ui-components";

function Simulation({ speed }: { speed: number }) {
  useAutoStep(
    () => simulator.step(),  // Called N times per second
    speed,                    // e.g., 100 steps/sec
    true,                     // active
  );
}
```

#### useReducedMotion

Detects the user's `prefers-reduced-motion` preference. Use this to disable particle animations and show static alternatives instead.

```tsx
import { useReducedMotion } from "@coding-adventures/ui-components";

function Visualization() {
  const reducedMotion = useReducedMotion();

  return reducedMotion
    ? <StaticDiagram />
    : <AnimatedVisualization />;
}
```

### CSS Styles

Import the shared styles in your app's main CSS file:

```css
@import "@coding-adventures/ui-components/src/styles/theme.css";
@import "@coding-adventures/ui-components/src/styles/accessibility.css";
```

The theme provides CSS custom properties for backgrounds, accents, wire colors, tab styling, and typography. Apps can override any variable for their own needs.

## Development

```bash
npm install        # Install dev dependencies
npm run build      # Type-check with tsc
npm test           # Run tests
npm run test:coverage  # Run tests with coverage
```

## Architecture

This package is a peer dependency consumer -- it expects `react` and `react-dom` to be provided by the consuming app. This avoids duplicate React instances and keeps bundle sizes small.

The i18n system uses a module-level singleton pattern (not React context) so that `translate()` can be called outside of React components. The `useTranslation` hook subscribes to locale changes via a listener set for automatic re-renders.
