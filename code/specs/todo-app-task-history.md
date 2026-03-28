# Todo App — Task History & UX Defaults

**Status:** Proposed
**Relates to:** `todo-app-audit-log.md` (audit log infrastructure)

---

## 1. Problem

Two UX gaps exist in the current Views Engine V1:

### 1a. Blank due date on new tasks

The "New Todo" form initialises the due date field to empty. Users who create
a task intending to do it today must remember to manually set the date,
otherwise the task never appears in the "Today" view. This is the primary
reason tasks go missing from filtered views.

### 1b. No visible benefit from the audit log

We built an audit log in v0.4.0. It records every action as an `AuditEvent`
in IndexedDB. But no part of the UI surfaces this data yet. The audit log is
invisible to the user — it looks like dead infrastructure.

---

## 2. Proposed Changes

### 2a. Default due date = today

**Rule:** When the user opens the "New Task" form, pre-fill the due date field
with today's date in the user's *local* timezone.

**Why local, not UTC?**
`new Date().toISOString()` returns UTC. A user in UTC-5 at 11 PM on March 28
would see "2026-03-27" — yesterday — because UTC is already March 29. The
pre-fill must use local calendar date to match what the date picker shows.

**Implementation:**
```
function todayLocal(): string
  yyyy = new Date().getFullYear()
  mm   = (new Date().getMonth() + 1).toString().padStart(2, "0")
  dd   = new Date().getDate().toString().padStart(2, "0")
  return "${yyyy}-${mm}-${dd}"
```

**Edit mode:** When editing an existing task, the due date field is pre-filled
from the stored task data (unchanged). The today-default only applies to new tasks.

**User override:** The user can clear the date or pick a different date as
normal — the default is just a convenience, not a lock.

---

### 2b. Task activity history panel

**Goal:** Surface the audit log to the user as a timeline in the task edit
view. This:
- Gives the user visibility into what changed and when.
- Proves the audit log is working (developer confidence).
- Lays the foundation for streak detection and activity feeds.

#### 2b-i. Placement

The history panel appears **below the edit form** in `TodoEditor`, visible
only when editing an existing task (not on the "New Task" form — a brand-new
task has no history yet).

#### 2b-ii. Data source

`getActivitiesForEntity(storage, taskId)` from `audit.ts`. Returns
`AuditEvent[]` sorted oldest-first (ascending vector clock order).

#### 2b-iii. Async boundary

The IDB query is async. The form must **not** block on the history load.
The history section transitions through explicit states:

```
idle ──(todoId set)──▶ loading ──(IDB resolves)──▶ ready
```

- **idle:** component is in "New Todo" mode; no query issued; no section rendered.
- **loading:** query in flight; render a "Loading…" placeholder in the history section.
- **ready:** query resolved; render the event list (or a quiet empty state if no events).

A `cancelled` flag guards against calling `setState` on an unmounted component
(i.e., user navigates away before the IDB promise resolves).

#### 2b-iv. Event descriptions

Raw action type strings (`TASK_UPDATE`) are translated to friendly prose:

| `actionType`        | Display text                              |
|---------------------|-------------------------------------------|
| `TASK_CREATE`       | "Task created"                            |
| `TASK_UPDATE`       | "Updated: {field, field, …}"             |
| `TASK_TOGGLE_STATUS`| "Status toggled"                          |
| `TASK_SET_STATUS`   | "Status set to {status}"                 |
| `TASK_DELETE`       | "Task deleted"                            |
| (other)             | lowercased, underscores → spaces          |

For `TASK_UPDATE`, the `patch` field in the action payload lists exactly which
fields changed. We enumerate them with human-readable labels:

| Patch key     | Display label |
|---------------|---------------|
| `title`       | title         |
| `description` | description   |
| `priority`    | priority      |
| `category`    | category      |
| `dueDate`     | due date      |
| `dueTime`     | due time      |

#### 2b-v. Timestamp formatting

Recent events use relative labels; older events use absolute date+time:

| Age            | Format                   | Example              |
|----------------|--------------------------|----------------------|
| < 60 seconds   | "Just now"               | "Just now"           |
| < 1 hour       | "{n}m ago"               | "5m ago"             |
| < 24 hours     | "{n}h ago"               | "3h ago"             |
| ≥ 24 hours     | locale date+time         | "Mar 28, 2:45 PM"    |

The `<time>` element carries a machine-readable `dateTime` attribute for
accessibility and SEO, with a human-readable `title` tooltip showing the full
ISO string.

#### 2b-vi. Visual design

A vertical timeline:
- A thin connector line runs down the left edge.
- Each event has a filled dot (accent colour) on the connector.
- Event description on the left, relative timestamp right-aligned.
- Section heading "Activity" in small-caps muted text.
- Loading state: inline "Loading…" text (no spinner — avoids layout shift).
- Empty state (ready, no events): no section rendered — avoids visual noise
  on tasks created before the audit log was deployed.

#### 2b-vii. Storage access

Components cannot receive `storage` as a prop today — the store singleton
pattern is used everywhere. We follow the same pattern with a storage
singleton (`src/storage.ts`):

```
initStorage(s: KVStorage): void   — called by main.tsx before React mounts
getStorage(): KVStorage           — called by components; throws if not init'd
```

`main.tsx` calls `initStorage` immediately after opening IDB, before any
middleware registration or React mounting. This guarantees `getStorage()` is
always available on the first render.

---

## 3. Out of Scope

- **React Suspense integration** — Using `use(promise)` + `<Suspense>` is a
  valid future upgrade but requires a data-fetching library or `cache()`. For
  now, explicit `useState` + `useEffect` + status enum is simpler and
  sufficient.
- **Real-time history updates** — The history list does not re-query on every
  dispatch. It loads once when the edit form opens. A future improvement could
  subscribe to new events via a store middleware.
- **Pagination** — The history list shows all events. For tasks with very long
  histories, a "show more" button could be added. Not needed until the audit
  log has been live long enough to accumulate significant per-task history.
- **History on the list view** — Showing inline activity on `TodoCard` is out
  of scope for this iteration.

---

## 4. Files Changed

| File | Change |
|------|--------|
| `src/storage.ts` | New — settable storage singleton |
| `src/main.tsx` | Call `initStorage(storage)` before React mounts |
| `src/components/TodoEditor.tsx` | Default dueDate to today; add history panel |
| `src/styles/app.lattice` | Add `.history` block styles |
| `CHANGELOG.md` | Add v0.5.0 entry |

---

## 5. Tests

No new test files are required for this change:

- `todayLocal()` is a pure function returning a deterministic string given a
  fixed clock — it is trivially correct and tested implicitly by the existing
  `views.test.ts` date handling tests.
- The history panel is a React component — component tests would require
  mocking `getStorage()` and `getActivitiesForEntity`. These interactions are
  already covered by `audit.test.ts` (the query layer) and
  `ViewRenderer.test.tsx` (the rendering layer). Adding a TodoEditor-specific
  test file is planned as a follow-up but is not blocking.
- The `storage.ts` singleton is two functions with no logic — not tested in
  isolation.
