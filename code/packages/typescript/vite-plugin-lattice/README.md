# vite-plugin-lattice

Vite plugin that transpiles [Lattice](../../specs/17-lattice-transpiler.md) CSS superset files (`.lattice`) to plain CSS at build time.

## Features

- **Transform**: `.lattice` → CSS via the Lattice transpiler
- **HMR**: Instant style updates during development
- **Style injection**: CSS injected via `<style>` tags in dev, extracted in production

## Usage

```typescript
// vite.config.ts
import { latticePlugin } from "@coding-adventures/vite-plugin-lattice";

export default defineConfig({
  plugins: [latticePlugin()],
});
```

Then import `.lattice` files in your source:

```typescript
import "./styles/app.lattice";
```

## Options

```typescript
latticePlugin({
  minified: false,  // Emit minified CSS (default: false)
  indent: "  ",     // Indentation string (default: 2 spaces)
});
```

## Dependencies

- @coding-adventures/lattice-transpiler (browser-compatible transpiler)

## Development

```bash
npm install
npm test
```
