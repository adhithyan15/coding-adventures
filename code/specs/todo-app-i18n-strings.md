# Todo App — i18n Strings & Component Rename

**Status:** Proposed
**Scope:** App-wide string extraction + component naming cleanup

---

## 1. Problem

### 1a. Raw strings in code

User-visible text is scattered across component files as raw string literals:

```tsx
<h2>New Task</h2>
<option value="done">Done</option>
return "Just now";
```

This makes it impossible to ship a translated version of the app by swapping
a single file. Every translation requires hunting through every component.

### 1b. Todo-prefixed component names

When the action/type layer was renamed from `Todo` → `Task`, the component
files were not. We now have a semantic mismatch:

| File                           | Should be                     |
|--------------------------------|-------------------------------|
| `components/TodoEditor.tsx`    | `components/TaskEditor.tsx`   |
| `components/TodoCard.tsx`      | `components/TaskCard.tsx`     |
| `components/TodoList.tsx`      | `components/TaskList.tsx`     |
| `components/TodoCalendar.tsx`  | `components/TaskCalendar.tsx` |

`CalendarViewWrapper.tsx`, `EmptyState.tsx`, `FilterBar.tsx`, `KanbanView.tsx`,
and `ViewRenderer.tsx` don't have `Todo`-prefixed names and stay as-is.

---

## 2. Design

### 2a. String catalog: `src/strings.en.json`

A single JSON file holds every user-visible string in the app, keyed by a
namespaced dot-path. Namespaces follow the component/feature hierarchy:

```
app.*         — top-level app chrome (title, nav)
task.form.*   — TaskEditor form labels and buttons
task.form.history.*  — activity history section in TaskEditor
task.card.*   — TaskCard labels and tooltips
task.status.* — status values (displayed in filters, card badges)
task.priority.* — priority values (displayed in filters, card badges)
task.list.*   — TaskList stats and group headers
task.empty.*  — empty state messages
filter.*      — FilterBar labels and option text
calendar.*    — TaskCalendar headings
board.*       — KanbanView (Board) placeholder text
view.error.*  — ViewRenderer error messages
```

**File:** `src/strings.en.json`

Naming convention: `strings.LOCALE.json` — today only `en`, but a `fr.json`
drop-in replaces every string without touching component code.

#### Complete key inventory

```json
{
  "app": {
    "nav": {
      "list":     "☰ List",
      "calendar": "📅 Calendar"
    }
  },
  "task": {
    "form": {
      "headingNew":   "New Task",
      "headingEdit":  "Edit Task",
      "submitNew":    "Create Task",
      "submitEdit":   "Save Changes",
      "cancel":       "Cancel",
      "titleLabel":   "Title",
      "titleRequired":"*",
      "titleError":   "Title is required",
      "titlePlaceholder": "What needs to be done?",
      "descLabel":    "Description",
      "descPlaceholder": "Add details, notes, or context...",
      "priorityLabel":"Priority",
      "categoryLabel":"Category",
      "categoryPlaceholder": "e.g. work, personal",
      "dueDateLabel": "Due Date",
      "history": {
        "heading":   "Activity",
        "loading":   "Loading…",
        "eventCreated":  "Task created",
        "eventUpdated":  "Task updated",
        "eventUpdatedFields": "Updated: {fields}",
        "eventStatusToggled":  "Status toggled",
        "eventStatusSet":      "Status set to {status}",
        "eventDeleted":        "Task deleted",
        "timeJustNow":   "Just now",
        "timeMinutes":   "{n}m ago",
        "timeHours":     "{n}h ago"
      },
      "notFound": {
        "heading": "Task not found",
        "body":    "The task you're looking for doesn't exist or was deleted.",
        "back":    "Back to list"
      }
    },
    "card": {
      "editAriaLabel":   "Edit",
      "deleteAriaLabel": "Delete",
      "overdueLabel":    "⚠ Overdue",
      "dueTodayLabel":   "📅 Due today"
    },
    "status": {
      "todo":       "Todo",
      "inProgress": "In Progress",
      "done":       "Done"
    },
    "priority": {
      "low":    "Low",
      "medium": "Medium",
      "high":   "High",
      "urgent": "Urgent"
    },
    "list": {
      "statTotal":      "Total",
      "statTodo":       "Todo",
      "statInProgress": "In Progress",
      "statDone":       "Done"
    },
    "empty": {
      "noTasksHeading":   "No tasks yet",
      "noTasksBody":      "Create your first task to get started.",
      "noMatchesHeading": "No matches",
      "noMatchesBody":    "Try adjusting your filters."
    }
  },
  "filter": {
    "statusAll":      "All Status",
    "statusTodo":     "Todo",
    "statusProgress": "In Progress",
    "statusDone":     "Done",
    "priorityAll":    "All Priority",
    "priorityLow":    "Low",
    "priorityMedium": "Medium",
    "priorityHigh":   "High",
    "priorityUrgent": "Urgent",
    "categoryAll":    "All Categories",
    "sortCreated":    "Created",
    "sortUpdated":    "Updated",
    "sortDueDate":    "Due Date",
    "sortPriority":   "Priority",
    "sortTitle":      "Title",
    "sortAsc":        "Ascending",
    "sortDesc":       "Descending"
  },
  "calendar": {
    "heading":  "Calendar",
    "subtitle": "Tasks due each day"
  },
  "board": {
    "comingSoonHeading": "Board view coming soon",
    "comingSoonBody":    "Your tasks are safe. Switch to \"All Tasks\" to see them now."
  },
  "view": {
    "error": {
      "notFoundHeading": "View not found",
      "notFoundBody":    "This view doesn't exist or has been removed.",
      "unknownTypeHeading": "Unknown view type",
      "unknownTypeBody":    "This view type is not supported in this version of the app."
    }
  }
}
```

---

### 2b. Accessor: `src/strings.ts`

```ts
import en from "./strings.en.json";

/**
 * The active locale's string catalog. Today always `en`; in the future,
 * swap this reference based on the user's navigator.language or a settings
 * preference.
 */
const catalog: typeof en = en;

/**
 * t — type-safe string accessor with optional string interpolation.
 *
 * @param key    — dot-path into the catalog, e.g. "task.form.headingNew"
 * @param params — optional substitution map, e.g. { n: 5 }
 *
 * String templates use {key} syntax, e.g.:
 *   t("task.form.history.timeMinutes", { n: 5 })  →  "5m ago"
 *   t("task.form.history.eventUpdatedFields", { fields: "title, priority" })
 *     →  "Updated: title, priority"
 */
export function t(key: NestedKeys<typeof catalog>, params?: Record<string, string | number>): string {
  const raw = getNestedValue(catalog, key);
  if (!params) return raw;
  return raw.replace(/\{(\w+)\}/g, (_, k) => String(params[k] ?? `{${k}}`));
}
```

`NestedKeys<T>` is a recursive type that produces a union of all valid
dot-path strings for a nested object — TypeScript will error at compile-time
if a component references a key that doesn't exist in the catalog.

`getNestedValue(obj, dotPath)` splits the path on `.` and traverses the
object, returning the leaf string.

---

### 2c. Component renames

File rename and export rename happen together:

| Old filename                   | New filename                  | Old export    | New export    |
|--------------------------------|-------------------------------|---------------|---------------|
| `components/TodoEditor.tsx`    | `components/TaskEditor.tsx`   | `TodoEditor`  | `TaskEditor`  |
| `components/TodoCard.tsx`      | `components/TaskCard.tsx`     | `TodoCard`    | `TaskCard`    |
| `components/TodoList.tsx`      | `components/TaskList.tsx`     | `TodoList`    | `TaskList`    |
| `components/TodoCalendar.tsx`  | `components/TaskCalendar.tsx` | `TodoCalendar`| `TaskCalendar`|

All imports in `App.tsx`, `ViewRenderer.tsx`, and any test files update to
the new names.

---

### 2d. CSS class names

CSS class names (`.todo-list__*`, `.todo-card__*`, etc.) are **not** renamed
in this iteration. They are implementation details, not user-visible, and
renaming them would risk breaking styles with no user-facing benefit. This
can be done as a separate cleanup PR if desired.

---

## 3. Interpolation Design

String values that embed dynamic content use `{key}` placeholders:

```json
"timeMinutes":         "{n}m ago"
"eventStatusSet":      "Status set to {status}"
"eventUpdatedFields":  "Updated: {fields}"
```

Callers pass a `params` map:
```ts
t("task.form.history.timeMinutes", { n: 3 })
// → "3m ago"

t("task.form.history.eventUpdatedFields", { fields: "priority, due date" })
// → "Updated: priority, due date"
```

This is intentionally simple — no plural rules, no gender, no HTML
injection. If the app later needs ICU MessageFormat syntax (plurals, selects),
the `t()` function can be upgraded while all call sites remain identical.

---

## 4. Locale Switching (Future)

The active catalog is selected by one line in `strings.ts`:

```ts
const catalog: typeof en = en;   // ← swap for fr, de, ja, etc.
```

A future settings screen can store the user's preferred locale in
localStorage and the app can dynamically import the right JSON file at
startup.

---

## 5. Files Changed

| File | Change |
|------|--------|
| `src/strings.en.json` | New — complete English string catalog |
| `src/strings.ts` | New — `t()` accessor + `NestedKeys` type |
| `src/components/TodoEditor.tsx` → `TaskEditor.tsx` | Rename + use `t()` |
| `src/components/TodoCard.tsx` → `TaskCard.tsx` | Rename + use `t()` |
| `src/components/TodoList.tsx` → `TaskList.tsx` | Rename + use `t()` |
| `src/components/TodoCalendar.tsx` → `TaskCalendar.tsx` | Rename + use `t()` |
| `src/components/EmptyState.tsx` | Use `t()` (no rename) |
| `src/components/FilterBar.tsx` | Use `t()` (no rename) |
| `src/components/KanbanView.tsx` | Use `t()` (no rename) |
| `src/components/ViewRenderer.tsx` | Update imports + use `t()` |
| `src/App.tsx` | Update imports |
| `src/__tests__/ViewRenderer.test.tsx` | Update import name |

---

## 6. Tests

- `src/__tests__/strings.test.ts` — unit tests for `t()`:
  - Key resolution: valid keys return correct strings
  - Interpolation: `{n}` and `{fields}` substitution
  - Missing param: `{key}` left as-is (no crash)
  - TypeScript: invalid key causes compile error (tested via `tsc --noEmit`)
- Existing component tests continue to work; they mock strings at the
  component level, not the `t()` level, so no test changes needed.
