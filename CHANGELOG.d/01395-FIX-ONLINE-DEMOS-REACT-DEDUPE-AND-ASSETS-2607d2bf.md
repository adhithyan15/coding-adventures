### Fix duplicate React bundling in online visualizer demos and add missing Busicom icon

- Configured `resolve.preserveSymlinks: true` and `resolve.dedupe: ["react", "react-dom"]` in `transistor-visualizer`, `logic-gates-visualizer`, `arithmetic-visualizer`, `eniac-visualizer`, and `todo-app` Vite configurations to prevent multiple copies of React from being bundled when depending on local monorepo packages like `@coding-adventures/ui-components`, resolving runtime `TypeError: Cannot read properties of null (reading 'useState')` errors on GitHub Pages.
- Added `public/calculator-icon.svg` in `busicom-calculator` so that the favicon asset is properly included in Vite production builds.
