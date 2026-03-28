# todo-app

Offline-first todo list app — beautiful dark UI with IndexedDB persistence, styled with Lattice.

## Features

- **Offline-first** — all data stored in browser IndexedDB, no internet required
- **Lattice styling** — CSS superset with variables, mixins, functions
- **Priority levels** — Low, Medium, High, Urgent with color-coded cards
- **Status lifecycle** — Todo → In Progress → Done with visual indicators
- **Search & filters** — Filter by status, priority, category; sort by any field
- **Due dates** — Calendar date picker with overdue/due-today alerts
- **Free-form categories** — Tag todos with any category, auto-suggestions
- **Electron support** — Ship as desktop app on macOS, Windows, Linux

## Architecture

```
Flux pattern:  Action → Reducer → State → UI
Persistence:   Store middleware → IndexedDB (fire-and-forget)
Styling:       Lattice → CSS (via vite-plugin-lattice)
```

## Dependencies

- @coding-adventures/store (Flux state management)
- @coding-adventures/indexeddb (IndexedDB storage)
- @coding-adventures/ui-components (shared styles)
- @coding-adventures/vite-plugin-lattice (Lattice → CSS in Vite)

## Development

```bash
# Install dependencies
npm install

# Run dev server
npm run dev

# Run tests
npm test

# Run tests with coverage
npm run test:coverage

# Build for production
npm run build
```

## Electron

```bash
# Run in Electron dev mode (start Vite first with npm run dev)
npm run electron:dev

# Build desktop app
npm run electron:build        # current platform
npm run electron:build:mac    # macOS
npm run electron:build:win    # Windows
npm run electron:build:linux  # Linux
```

## E2E Testing

```bash
# Run Playwright e2e tests
npx playwright test

# Run with UI mode
npx playwright test --ui
```
