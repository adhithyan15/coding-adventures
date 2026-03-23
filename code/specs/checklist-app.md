# Checklist App — Interactive Decision-Tree Checklist Runner

## Overview

Most checklist tools are flat lists. Real procedures are not. A pre-flight
inspection branches on aircraft type. A deployment runbook branches on
whether smoke tests passed. A troubleshooting guide branches on whether the
service is running.

This app models checklists as **decision trees**: each item is either a
simple checkbox or a branching yes/no question. The answer to a question
determines which items appear next, hiding the irrelevant branch entirely
(not greying it out — hiding it, as a pilot's checklist would).

The design separates **template definition** (the reusable procedure) from
**instance execution** (one run of that procedure). A template is a class;
an instance is an object.

The V0 goal is to explore the full anatomy of a web application: in-memory
state management, event-driven UI, React component trees, TypeScript type
safety, testing with React Testing Library, and the path from browser app
to Electron desktop app.

---

## Core Concepts

### Template

A **Template** is a reusable checklist definition. It is authored once and
run many times. Editing a template does not affect instances already in
progress. A template holds an ordered list of `TemplateItem` nodes.

### TemplateItem

A `TemplateItem` is one node in the template's decision tree. Two variants:

- **check** — A simple step. Completed by ticking it off.
- **decision** — A yes/no question. Completed by answering. The answer
  activates one of two branches (`yesBranch` or `noBranch`), each an
  array of further `TemplateItem` nodes. Nesting is unlimited.

### Instance

An **Instance** is a single execution run of a template. Creating an instance
deep-clones the template's item tree into a parallel tree of `InstanceItem`
nodes, each carrying mutable execution state. Two instances of the same
template are fully independent.

Instance status: `in-progress` | `completed` | `abandoned`.

### InstanceItem

An `InstanceItem` mirrors its template counterpart with added state:

- **check** — tracks `checked: boolean`
- **decision** — tracks `answer: "yes" | "no" | null` and the two branches
  as `InstanceItem[]` arrays

### Visible Items

`flattenVisibleItems` is the core tree-walking algorithm. It walks the
instance item tree and returns only the items currently visible to the user:

1. For each item in the list, include the item itself.
2. If the item is a decision with `answer === null`, stop descending
   (the question must be answered before its branch items are shown).
3. If the item is a decision with an answer, recurse into the chosen branch.
4. Never include items from the unchosen branch.

This gives a linear, ordered list of items to render — the flat projection
of the active path through the decision tree.

### Stats

`computeStats` is a pure function over a completed or in-progress instance.
It counts only the items returned by `flattenVisibleItems` (i.e., only items
the user actually encountered). Stats are computed on demand, not stored.

```
totalItems    = count of non-decision items in the flattened list
                (decision items are navigational, not completable)
checkedItems  = count of those items where checked === true
decisionCount = count of decision items in the flattened list
completionRate = (checkedItems / totalItems) * 100, clamped 0–100
durationMs    = completedAt - createdAt, or null if still in progress
```

---

## TypeScript Type Definitions

```typescript
// ── Template types (immutable, authored once) ─────────────────────────────

interface CheckTemplateItem {
  id: string;
  type: "check";
  label: string;
}

interface DecisionTemplateItem {
  id: string;
  type: "decision";
  label: string;
  yesBranch: TemplateItem[];
  noBranch: TemplateItem[];
}

type TemplateItem = CheckTemplateItem | DecisionTemplateItem;

interface Template {
  id: string;
  name: string;
  description: string;
  createdAt: number;      // Date.now()
  items: TemplateItem[];
}

// ── Instance types (mutable, one per run) ─────────────────────────────────

type DecisionAnswer = "yes" | "no" | null;
type InstanceStatus = "in-progress" | "completed" | "abandoned";

interface CheckInstanceItem {
  templateItemId: string;
  type: "check";
  label: string;
  checked: boolean;
}

interface DecisionInstanceItem {
  templateItemId: string;
  type: "decision";
  label: string;
  answer: DecisionAnswer;
  yesBranch: InstanceItem[];
  noBranch: InstanceItem[];
}

type InstanceItem = CheckInstanceItem | DecisionInstanceItem;

interface Instance {
  id: string;
  templateId: string;
  templateName: string;
  status: InstanceStatus;
  createdAt: number;
  completedAt: number | null;
  items: InstanceItem[];
}

// ── Stats (computed, not stored) ──────────────────────────────────────────

interface InstanceStats {
  totalItems: number;
  checkedItems: number;
  decisionCount: number;
  completionRate: number;   // 0–100
  durationMs: number | null;
}
```

---

## UI/UX Flows

Navigation is hash-based (`#/`, `#/template/new`, `#/template/:id/edit`,
`#/instance/:id`, `#/instance/:id/stats`). The router listens to
`window.hashchange` and renders the matching screen into `<div id="root">`.

### Screen 1 — Template Library (`#/`)

A card grid of all defined templates. Each card shows: name, description,
item count, and a "Run" button.

Actions:
- **New Template** → navigates to Template Editor (new)
- **Run** on a card → calls `createInstance(templateId)`, navigates to Instance Runner
- **Edit** on a card → navigates to Template Editor (edit mode)
- **Delete** on a card → removes template from state (with confirmation)

### Screen 2 — Template Editor (`#/template/new` or `#/template/:id/edit`)

A form for building the item tree. Items can be of type `check` or
`decision`. Decision items expand to reveal two sub-forms: Yes branch and
No branch. The form is recursive — each branch can contain further check
or decision items.

Actions:
- **Add Item** — appends a new check item
- **Change type** — toggle between check/decision for any item
- **Add branch item** — appends a child item inside a decision branch
- **Remove item** — removes an item and all its children
- **Move up / Move down** — reorder items within their list
- **Save** — commits to state, navigates to Library
- **Cancel** — discards changes, navigates to Library

State: draft template is local component state until Save is pressed.

### Screen 3 — Instance Runner (`#/instance/:id`)

Renders the currently visible items via `flattenVisibleItems`. Items above
the current position are complete (checked/answered). The first incomplete
item is highlighted.

- **Check item** — clicking the checkbox marks it checked; checked items
  show a strikethrough
- **Decision item** — shows two buttons (Yes / No). Clicking records the
  answer and reveals the chosen branch below. The answer can be changed by
  clicking the answer badge.
- **Progress bar** — `checkedItems / totalItems` based on flattened visible list
- **Complete Checklist** — enabled when all visible non-decision items are
  checked and all visible decision items are answered. Navigates to Stats.
- **Abandon** — marks instance abandoned, navigates to Stats

### Screen 4 — Stats View (`#/instance/:id/stats`)

Summary of a completed or abandoned instance.

Shows: template name, instance status badge, completion rate (large number),
item counts (total / checked / decisions made), duration, and a read-only
replay of the full item tree with final states visible.

Actions:
- **Run Again** — creates a new instance from the same template
- **Back to Library** — navigates to `#/`

---

## Tech Stack Rationale

### React 19 + TypeScript

React's component model matches the recursive structure of the item tree
naturally. A `DecisionItem` component renders itself recursively with its
branch items. The discriminated union `TemplateItem = Check | Decision`
maps directly to TypeScript's exhaustive narrowing — `switch (item.type)`
with no default case is a compile-time proof that all variants are handled.

React 19 is chosen because it is already in use in the other visualizers in
this repo (`logic-gates-visualizer`, `arithmetic-visualizer`), keeping the
tech stack consistent.

### Vite 6

Vite is the build tool and dev server already used by the other TypeScript
programs in this repo. Hot module replacement makes UI iteration fast. The
production build outputs a self-contained `dist/` folder that Electron can
load directly.

### @coding-adventures/ui-components

This shared package provides: `initI18n` / `useTranslation` for
internationalization, `TabList` for any tabbed navigation needed in future
tabs, `theme.css` for the shared dark color scheme, and `accessibility.css`
for focus ring and screen-reader utilities. Using it keeps the checklist app
visually consistent with the other visualizers.

### Path to Electron

The app is authored as a standard browser web app. Adding Electron requires:
1. `npm install --save-dev electron`
2. `electron/main.ts` — main process that opens `BrowserWindow` loading
   `dist/index.html` with `webSecurity: false` for local file loading
3. `"main": "dist/electron/main.js"` in package.json

The renderer code (all of `src/`) is untouched. This is valid for V0 because
state is already in-memory; V1 would add `contextBridge` IPC to expose
native file system access for persistence.

---

## V0 Scope

**In scope:**
- Template CRUD (create, read, update, delete) — in-memory only
- Decision-tree items with unlimited nesting depth
- Instance creation from template (deep clone)
- Instance execution: check items, answer decisions
- Visible-item flattening: only items on the active decision branch are shown
- Progress bar and "Complete" button
- Stats computation on completion/abandonment
- Hash-based router: five routes, four screens
- Seed data: three pre-loaded example templates
- Unit tests for all state logic; component tests with React Testing Library
- i18n with `@coding-adventures/ui-components`

**Out of scope for V0:**
- Persistent storage (localStorage, IndexedDB, file system)
- Template import / export (JSON)
- Item drag-and-drop reordering
- Undo / redo
- Electron packaging
- Multi-user collaboration
- Dark/light theme toggle (dark only, via theme.css)
- Mobile-specific touch gestures

---

## Component Architecture

```
App
├── TemplateLibrary       (Screen 1)
│   └── TemplateCard      (one per template)
├── TemplateEditor        (Screen 2)
│   └── ItemEditor        (recursive — handles check and decision types)
│       └── ItemEditor    (recursive child for each branch item)
├── InstanceRunner        (Screen 3)
│   ├── ProgressBar       (checked / total)
│   └── VisibleItemList
│       ├── CheckItem     (checkbox + label)
│       └── DecisionItem  (question + Yes/No buttons)
└── StatsView             (Screen 4)
    └── ItemReplay        (read-only tree of final states)
```

State lives in `state.ts` (a module-level singleton). Components import
state functions directly — no React context, no Redux, no prop drilling
beyond what a component renders. On every user action, the handler mutates
state and calls `forceUpdate()` via a `useState` toggle in `App.tsx`, which
re-renders the active screen. This is intentionally simple for V0.

---

## Test Strategy

### Unit tests — `src/__tests__/state.test.ts`

Pure logic in `state.ts` is tested in isolation (Node environment, no DOM).
Target: 95%+ line coverage on `state.ts`.

Required test cases:
- `createTemplate` returns a Template with generated IDs
- `createInstance` deep-clones items, including nested decision branches
- `createInstance` copies labels from template items to instance items
- `flattenVisibleItems` with all check items returns all of them
- `flattenVisibleItems` with unanswered decision returns items up to and
  including the decision question, then stops
- `flattenVisibleItems` after answering yes returns yes-branch items (not no-branch)
- `flattenVisibleItems` after answering no returns no-branch items (not yes-branch)
- `flattenVisibleItems` with nested decisions: inner branch only revealed
  after both outer and inner decisions are answered
- `computeStats` on fresh instance: totalItems = N, checkedItems = 0
- `computeStats` after all checked: completionRate = 100
- `computeStats` with a decision answered no: yes-branch items not counted
- `computeStats` durationMs is null while in-progress, set after completion
- `checkItem` marks the correct item checked
- `checkItem` is idempotent
- `answerDecision` records answer, does not affect other items
- `answerDecision` can be changed (answering yes then no replaces the answer)
- `completeInstance` sets status = "completed" and completedAt
- `abandonInstance` sets status = "abandoned"
- `deleteTemplate` removes template from state
- `getTemplate` returns undefined for unknown ID

### Component tests — co-located `.test.tsx` files

Using React Testing Library + jsdom. Tests verify rendered output and user
interactions, not implementation details.

Key tests per component:
- `ProgressBar` — renders correct percentage, accessible label
- `TemplateLibrary` — renders template cards, Run button creates instance
- `InstanceRunner` — check item toggles strikethrough, decision buttons
  reveal branch items, Complete button disabled until all items done
- `StatsView` — shows correct completion rate, Run Again creates new instance

### Coverage target

95%+ for `state.ts`. 80%+ overall.

---

## Seed Templates

Three templates are pre-loaded on first visit to demonstrate V0 features:

1. **Morning Routine** — flat check list (~8 items). Demonstrates the
   simplest case: no decisions, just a procedural sequence.

2. **Deployment Runbook** — mix of check and decision items. "Did smoke
   tests pass?" branches to either "Monitor dashboard for 10 minutes" (yes)
   or "Run rollback script" + "Alert on-call engineer" (no).

3. **Troubleshooting Guide** — nested decisions. "Is the service running?"
   → if no: "Restart service" → "Did it start successfully?" → further branches.
   Demonstrates two levels of nesting.

---

## Future Extensions

- **V1**: localStorage persistence; JSON import/export; drag-and-drop reorder
- **V2**: Electron packaging; native file open/save dialogs via IPC
- **V3**: Aggregate stats view (compare multiple runs of the same template)
- **V4**: Template sharing via URL (state serialized into hash fragment)
