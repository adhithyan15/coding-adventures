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

## V0.2 — Tree View

### Problem

V0.1 renders the Instance Runner as a flat list. The `flattenVisibleItems`
algorithm produces a one-dimensional array and the Runner maps it into a
vertical stack of cards with zero indentation. This works for shallow trees
(1–2 levels of decisions), but breaks down as depth grows:

- You lose spatial context: "am I in the yes-branch or the no-branch?"
- You cannot see the unchosen branch at all — it vanishes entirely.
- You cannot compare the two paths side by side.
- There is no visual indication of nesting depth.

### Solution

Replace the flat list rendering in both the Runner and Editor with a
**recursive tree view** that draws CSS connectors between parent and child
nodes. Both branches of every decision are always visible: the active branch
is fully interactive, the inactive branch is dimmed and collapsed to a
one-line summary (expandable on click for review).

The tree view components live in `@coding-adventures/ui-components` as
shared, reusable components — not specific to the checklist app.

---

### Shared Component: `TreeView<T>`

**Package:** `@coding-adventures/ui-components`

A generic recursive tree renderer. Not checklist-specific — any app can use
it to render hierarchical data with visual connectors.

```typescript
// The minimal shape every node must satisfy.
interface TreeViewNode {
  id: string;
  children?: TreeViewNode[];
}

interface TreeViewProps<T extends TreeViewNode> {
  /** The root-level nodes to render. */
  nodes: T[];

  /** Render function called for every node. Return the node's visual content.
   *  The TreeView handles layout, indentation, and connectors around it. */
  renderNode: (node: T, depth: number) => React.ReactNode;

  /** Optional: render a label above a child group (e.g., "If yes:", "If no:"). */
  renderBranchLabel?: (node: T, branchIndex: number) => React.ReactNode;

  /** Return true if the node's children should be visible. Default: true. */
  isExpanded?: (node: T) => boolean;

  /** Called when the user toggles a node's expand/collapse state. */
  onToggleExpand?: (node: T) => void;

  /** CSS class on the outermost container. */
  className?: string;

  /** Accessible label for the tree. */
  ariaLabel?: string;
}
```

**Rendering algorithm:**

For each node in `nodes`:
1. Render the node via `renderNode(node, depth)`.
2. If the node has `children` and `isExpanded(node)` returns true, recurse
   into each child group with `depth + 1`.
3. Draw CSS connectors: vertical trunk (border-left on the child list),
   horizontal connector (::before pseudo-element), T-shape for middle items,
   L-shape for the last item.

**CSS connectors (border-based):**

```
├─ Step 1                    T-connector (middle item)
├─ Did it work?              T-connector (middle item)
│  ├─ ✅ YES branch          T-connector (branch start)
│  │  ├─ Monitor 10 min      T-connector
│  │  └─ Post in #deploys    L-connector (last item)
│  └─ ❌ NO branch (dimmed)  L-connector (last branch)
│     └─ 4 steps • expand    Collapsed summary
└─ Step 3                    L-connector (last top-level item)
```

Technique:
- `border-left: var(--tree-connector-width) solid var(--tree-connector-color)`
  on each item creates the vertical trunk.
- `::before` pseudo-element with `border-bottom` draws the horizontal
  connector from the trunk to the node content.
- `:last-child` switches from T-shape (trunk continues below) to L-shape
  (trunk ends).

**ARIA treeview pattern:**

```
role="tree"        → outermost container
role="treeitem"    → each node
  aria-expanded    → true/false for nodes with children
  aria-level       → depth (1-indexed)
  aria-selected    → true for checked items
role="group"       → wrapper around child nodes, labelledby parent
```

Keyboard:
- `↑` / `↓` — move focus between visible nodes
- `←` — collapse current node (if expanded), or move to parent
- `→` — expand current node (if collapsed), or move to first child
- `Space` / `Enter` — activate (check item, answer decision, toggle expand)
- `Home` / `End` — jump to first / last visible node

---

### Shared Component: `BranchGroup`

**Package:** `@coding-adventures/ui-components`

A wrapper for a group of child nodes under a parent decision. Handles the
three visual states of a decision branch.

```typescript
interface BranchGroupProps {
  /** Label rendered above the group (e.g., "If yes:", "If no:"). */
  label: React.ReactNode;

  /** When true, children are hidden and summary is shown. */
  collapsed: boolean;

  /** When true, the branch is dimmed (40% opacity, pointer-events: none). */
  inactive: boolean;

  /** Text shown when collapsed (e.g., "3 steps • click to expand"). */
  summary?: string;

  /** Called when the user clicks the summary to toggle collapse. */
  onToggleCollapse?: () => void;

  /** The child tree nodes to render inside this group. */
  children: React.ReactNode;

  /** CSS class on the outermost wrapper. */
  className?: string;
}
```

**Three branch states:**

| State | Trigger | Opacity | Interaction | Height |
|-------|---------|---------|-------------|--------|
| **Pending** | Decision unanswered | 100% | Buttons visible | 0 (no branch items shown) |
| **Active** | Chosen branch | 100% | Fully interactive | Auto |
| **Inactive** | Unchosen branch | 40% | Collapsed to summary; click to expand read-only | Collapsed: 1 line; Expanded: auto |

**Collapse/expand animation:**
- `max-height` transition (0 → generous fallback) with `overflow: hidden`
- `opacity` transition (0.4 ↔ 1.0) with 200ms ease
- `pointer-events: none` on inactive collapsed branches

**Inactive summary format:**
- `"3 steps • click to expand"` — for branches with only check items
- `"3 steps, 1 decision • click to expand"` — if it contains sub-decisions

---

### Shared CSS: `tree.css`

**Package:** `@coding-adventures/ui-components/src/styles/tree.css`

```css
:root {
  --tree-indent: 24px;
  --tree-connector-color: var(--panel-border, #30363d);
  --tree-connector-width: 1px;
  --tree-branch-active-opacity: 1;
  --tree-branch-inactive-opacity: 0.4;
  --tree-transition-duration: 200ms;
}
```

BEM classes:
- `.tree` — outermost container
- `.tree__node` — one tree node (content + connector)
- `.tree__connector` — the vertical + horizontal line drawing
- `.tree__children` — wrapper around child nodes
- `.branch-group` — the BranchGroup wrapper
- `.branch-group--active` / `.branch-group--inactive` / `.branch-group--collapsed`
- `.branch-group__label` — the "If yes:" / "If no:" header
- `.branch-group__summary` — the collapsed summary line

---

### Changes to Checklist App

**InstanceRunner.tsx:**

Replace the current flat rendering:
```tsx
// V0.1 (flat)
flattenVisibleItems(instance.items).map(item => ...)

// V0.2 (tree)
<TreeView
  nodes={instance.items}
  renderNode={(item, depth) =>
    item.type === "check"
      ? <CheckItemRow item={item} ... />
      : <DecisionItemRow item={item} ... />
  }
  isExpanded={(item) => item.type === "decision" && item.answer !== null}
  ariaLabel="Checklist"
/>
```

Decision nodes render two `<BranchGroup>` children inside their `renderNode`.
Active/inactive state is driven by `decision.answer`. The `CheckItemRow`
and `DecisionItemRow` sub-components remain largely unchanged — only their
layout wrapper changes.

**TemplateEditor.tsx:**

Replace the current `ItemList` / `ItemEditor` recursion with `<TreeView>` +
`<BranchGroup>`. The `renderNode` function returns the editing UI: label
input, type toggle (check/decision), move up/down, remove. "Add step"
buttons appear at the end of each branch group.

**state.ts:**

Add one new helper (the rest stays unchanged):
```typescript
function countBranchItems(items: InstanceItem[]): {
  checks: number;
  decisions: number;
}
```

Recursively counts items in a branch for the collapsed summary text.
`flattenVisibleItems` remains for stats computation.

**en.json additions:**
```json
"branch.summary": "{checks} steps",
"branch.summaryWithDecisions": "{checks} steps, {decisions} decisions",
"branch.clickToExpand": "click to expand",
"branch.yes": "If yes:",
"branch.no": "If no:"
```

---

### Test Strategy for V0.2

**TreeView tests (ui-components):**
- Renders flat list of nodes (no children) — no connectors
- Renders nested nodes — correct indentation at each depth
- `isExpanded` false → children hidden
- `onToggleExpand` called on click
- ARIA attributes: role=tree, role=treeitem, aria-expanded, aria-level
- Keyboard: arrow up/down moves focus, left/right expand/collapse

**BranchGroup tests (ui-components):**
- Active state: children visible, full opacity
- Inactive state: children hidden, summary shown
- Collapsed + click summary → calls onToggleCollapse
- Inactive has pointer-events: none (via class check)

**InstanceRunner tests (checklist-app):**
- Tree renders with connectors for nested decision template
- Answering Yes → yes-branch active, no-branch dimmed with summary
- Clicking inactive summary → expands to show items
- All existing Runner tests still pass (check, uncheck, complete, abandon)

**TemplateEditor tests (checklist-app):**
- Editor renders tree structure for decision template
- Add item inside a branch → appears under correct branch group
- Move up/down within a branch works

---

## V0.3 — IndexedDB Persistence + Store Pattern

### Problem

V0.2 stores all state in memory. Page reload loses everything. React
components mutate a global `appState` singleton directly, mixing state
management with the UI layer.

### Solution

Three changes:

1. **`@coding-adventures/indexeddb`** — a standalone Promise-based wrapper
   around the raw browser IndexedDB API. Defines a `KVStorage` interface
   with two implementations: `IndexedDBStorage` (browser) and
   `MemoryStorage` (tests). No external dependencies.

2. **`@coding-adventures/store`** — a standalone Flux-like event-driven
   state store. Components dispatch actions; a reducer processes them; the
   store emits change events; React components subscribe and re-render.

3. **Checklist app refactor** — components dispatch actions instead of
   calling mutation functions. A persistence middleware writes to IndexedDB
   after each dispatch (fire-and-forget).

---

### Package: `@coding-adventures/indexeddb`

```typescript
interface KVStorage {
  open(): Promise<void>;
  get<T>(storeName: string, key: string): Promise<T | undefined>;
  getAll<T>(storeName: string): Promise<T[]>;
  put<T>(storeName: string, record: T): Promise<void>;
  delete(storeName: string, key: string): Promise<void>;
  close(): void;
}
```

**IndexedDBStorage** wraps the raw browser API:
- Constructor takes `dbName`, `version`, `stores: StoreSchema[]`
- `open()` calls `indexedDB.open()`, handles `onupgradeneeded`
- Each method opens a transaction, performs the operation, returns a Promise

**MemoryStorage** is an in-memory Map-of-Maps for tests. Same interface,
zero browser APIs.

**IndexedDB schema for checklist-app:**
- Database: `"checklist-app"`, version 1
- Store `"templates"`: keyPath `"id"`
- Store `"instances"`: keyPath `"id"`, index on `"templateId"`

---

### Package: `@coding-adventures/store`

```typescript
class Store<S> {
  constructor(initialState: S, reducer: Reducer<S>);
  getState(): S;
  dispatch(action: Action): void;
  subscribe(listener: () => void): () => void;
  use(middleware: Middleware<S>): void;
}

function useStore<S>(store: Store<S>): S;
// React hook: subscribes on mount, triggers re-render on store change.
```

**Middleware** intercepts every dispatch. The persistence layer is a
middleware that writes to KVStorage after the reducer runs.

**Data flow:**
```
User action → dispatch(Action) → middleware chain → reducer(state, action) → new state
                                      ↓
                              persistence middleware writes to IndexedDB (async)
                                      ↓
                              store emits "change" → useStore re-renders
```

---

### Checklist App Changes

**New files:**
- `actions.ts` — action types + creators (TEMPLATE_CREATE, INSTANCE_CHECK, etc.)
- `reducer.ts` — pure `(AppState, Action) → AppState` function
- `persistence.ts` — middleware mapping action types to DB writes

**Modified files:**
- `state.ts` — simplified to just store creation
- `main.tsx` — async init: open DB → load data → create store → seed if empty
- All components — `useStore(store)` + `store.dispatch(action)` instead of
  direct `appState` mutation

---

### Startup Flow

```
1. initI18n(en)
2. storage = new IndexedDBStorage(schema)
3. await storage.open()
4. templates = await storage.getAll("templates")
5. instances = await storage.getAll("instances")
6. store = new Store({ templates, instances }, reducer)
7. store.use(persistenceMiddleware(storage))
8. if (templates.length === 0) {
     dispatch seed actions → middleware persists them
   }
9. createRoot(root).render(<App />)
```

---

## V0.5 — Due Dates

### Problem

Todo items have no timeline. You can't see what's due today, what's
overdue, or plan ahead.

### Solution

Add an optional `dueDate` field to `TodoItem` and a shared `DatePicker`
component in `@coding-adventures/ui-components`.

### Data Model Change

```typescript
interface TodoItem {
  // ... existing fields ...
  dueDate: string | null;  // ISO 8601 date: "YYYY-MM-DD" or null
}
```

`dueDate` is a **string, not a Date object or timestamp**, because:
- JSON-serializable without conversion (IndexedDB, REST, SQL all handle it)
- A due date is a calendar date, not a point in time — "2026-03-25" means
  the same thing in every timezone
- The HTML `<input type="date">` returns YYYY-MM-DD natively
- String comparison works for sorting: `"2026-03-25" < "2026-04-01"`

### Shared Component: `DatePicker`

**Package:** `@coding-adventures/ui-components`

A thin, accessible wrapper around `<input type="date">` that integrates
with the shared dark theme and provides a consistent API.

```typescript
interface DatePickerProps {
  /** Current value as YYYY-MM-DD string, or empty string for no date. */
  value: string;
  /** Called with the new YYYY-MM-DD string on change. */
  onChange: (value: string) => void;
  /** Accessible label text. */
  label: string;
  /** HTML id for the input (for htmlFor on external labels). */
  id?: string;
  /** Placeholder text when no date is selected. */
  placeholder?: string;
  /** Additional CSS class. */
  className?: string;
}
```

The component renders a `<div>` containing:
- The `<input type="date">` styled to match the dark theme
- A "clear" button (✕) that resets the value to empty string
- Proper `aria-label` and focus styling

### UI Changes

**TodoEditor**: date picker field between description and status selector.
**TodoList**: due date shown on each item card, with overdue highlighting
(red text if `dueDate < today` and status is not "done").

### i18n

```json
"todos.dueDate": "Due Date",
"todos.dueDateNone": "No due date",
"todos.overdue": "Overdue"
```

---

## V0.6 — Electron Desktop App

### Problem

The app only runs in a browser tab. Users want a native desktop experience:
a dedicated window, a dock/taskbar icon, and offline use without a web server.

### Solution

Wrap the existing React app in Electron. The web code is unchanged — Electron's
main process creates a BrowserWindow that loads the Vite-built `dist/index.html`.

### Architecture

```
┌──────────────────────────────────────────────┐
│  Electron App                                │
│                                              │
│  ┌──────────────┐    ┌────────────────────┐  │
│  │ Main Process  │    │ Renderer Process   │  │
│  │ (Node.js)     │    │ (Chromium)         │  │
│  │               │    │                    │  │
│  │ • Window mgmt │◄──►│ • React app        │  │
│  │ • OS access   │ IPC│ • IndexedDB        │  │
│  │ • Menu bar    │    │ • Same code as     │  │
│  │ • Auto-update │    │   the browser app  │  │
│  └──────────────┘    └────────────────────┘  │
│                                              │
│  Chromium engine         Node.js runtime     │
└──────────────────────────────────────────────┘
```

**Main process** (`electron/main.ts`): Node.js process that creates windows,
handles OS integration. Security: `nodeIntegration: false`, `contextIsolation: true`.

**Renderer process**: Your React app running inside Chromium. Loaded from
`dist/index.html` (production) or Vite dev server URL (development).

### Build Pipeline

```
Vite builds React app → dist/index.html + assets
tsc compiles main.ts  → dist-electron/main.js
electron-builder      → platform-specific installer (dmg/nsis/AppImage)
```

### GitHub Release Workflow

A GitHub Actions workflow triggers on `checklist-v*` tags. Three parallel
jobs build for macOS (dmg+zip), Windows (nsis+portable), and Linux (AppImage).
All binaries are attached to the GitHub Release automatically.

### Security Model

- `nodeIntegration: false` — renderer cannot access Node.js APIs
- `contextIsolation: true` — renderer runs in a separate JavaScript context
- Future: `preload.ts` + `contextBridge` for safe OS access (file system, keychain)

---

## Future Extensions

- **V1**: Web Crypto encryption layer on top of IndexedDB (PBKDF2 + AES-GCM)
- **V2**: OAuth client package + calendar integration
- **V3**: Aggregate stats view (compare multiple runs of the same template)
- **V4**: Template sharing via URL (state serialized into hash fragment)
