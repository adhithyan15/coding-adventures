# Checklist App

An interactive decision-tree checklist runner for the browser.

Most checklist tools are flat lists. Real procedures branch. A deployment
runbook asks "did smoke tests pass?" — if no, you roll back. A troubleshooting
guide asks "is the service running?" — if no, you restart it first. This app
models checklists as **decision trees** where yes/no answers reveal the
relevant path and hide everything else.

## Architecture

Two layers:

- **Templates** — reusable procedure definitions, authored once. A template
  holds an ordered list of items, where each item is either a `check` (a
  step to tick off) or a `decision` (a yes/no question whose answer determines
  which branch of items follows).

- **Instances** — one execution run of a template. Created by deep-cloning
  the template's item tree. Two instances of the same template are fully
  independent.

The key algorithm is `flattenVisibleItems` in `src/state.ts`: it walks the
instance item tree and returns only the items currently visible to the user.
When a decision is unanswered, only the question itself is shown. Once
answered, the items in the chosen branch appear below it. Items in the
unchosen branch are never rendered.

Stats are a pure function over the final instance state (`computeStats`),
computed on demand — never stored.

## How it fits the project

This is the first interactive web app in the coding-adventures stack. It
uses the same React 19 + Vite + Vitest setup as `logic-gates-visualizer`
and `arithmetic-visualizer`, and shares the `@coding-adventures/ui-components`
package for i18n and the dark theme.

The path to Electron is intentional: add `electron` as a dev dependency,
create `electron/main.ts` that opens a `BrowserWindow` loading `dist/index.html`,
and the renderer code is untouched.

## Running

```bash
# From this directory
npm install
npm run dev
# Open http://localhost:5173/coding-adventures/checklist/
```

## Testing

```bash
npm run test           # run tests once
npm run test:coverage  # with coverage report
```

Target: 95%+ line coverage on `src/state.ts`, 80%+ overall.

## Building

```bash
npm run build
# Output: dist/
```

## Screens

| Route | Screen |
|---|---|
| `#/` | Template Library — browse and manage templates |
| `#/template/new` | Template Editor — create a new template |
| `#/template/:id/edit` | Template Editor — edit an existing template |
| `#/instance/:id` | Instance Runner — execute a checklist |
| `#/instance/:id/stats` | Stats View — completion summary |

## Spec

See [`/code/specs/checklist-app.md`](/code/specs/checklist-app.md) for the
full specification including type definitions, UI/UX flows, and the path
to Electron.
