# UI35 — `HostDraggable` / `HostDropTarget` kernel primitive family

**Status:** design (spec-first; committed before implementation)
**Motivated by:** [task-app-super-app](task-app-super-app.md) §5.1 — drag-and-drop is the
single biggest kernel gap, and it gates the Trello-grade board (Phase 6) and calendar
drag-to-reschedule (Phase 7).
**Pattern precedent:** [UI31 — `HostTable` family](UI31-host-table.md), which added a
primitive family that lowers to each platform's *native* widget with accessibility as a
non-negotiable contract. This spec follows that shape.

---

## 1. Why this belongs in the kernel

Today the Mosaic kernel has **no drag-and-drop of any kind**. A full-tree search for
`drag`/`drop`/`draggable`/`onDrop` finds nothing in the kernel primitive list, the
emitters, the layout compiler, or the specs; UI31 §7 explicitly lists "drag-to-reorder
columns or rows" as out of scope. So a board where you drag a card between columns — the
single most recognisable interaction in task software — **cannot be expressed** in a
`.mll` today.

It has to be a *kernel primitive family*, not a per-app trick, for the same reason
`HostTable` did:

- **Every backend already has a native drag system**, and they disagree profoundly (HTML5
  `dragstart`/`drop` vs. pointer events; SwiftUI `.draggable`/`.dropDestination`; Compose
  `dragAndDropSource`/`Target`; Qt `QDrag`; Flutter `Draggable`/`DragTarget`; WinUI
  `CanDrag`/`Drop`). Authoring once and lowering natively is exactly what Mosaic is for.
- **Doing it "in the app" means doing it nine times**, badly — and re-inventing the
  accessibility story nine times, which in practice means skipping it.
- **Drag is a platform-conventions minefield** (touch vs. mouse, drag thresholds, autoscroll,
  drop indicators, cancel-on-Escape). The kernel is where a convention gets decided once.

## 2. The primitive family

Two primitives, because a drag has two ends. A kanban card is typically **both**.

### 2.1 `HostDraggable` — a drag source

| slot | type | meaning |
|---|---|---|
| `drag-key` | text | **Opaque** identity of the thing being dragged. The kernel never interprets it; it comes back verbatim on drop. |
| `drag-kind` | text | A category (e.g. `"task"`, `"column"`). Drop targets accept/reject on this, so a column can't be dropped into a card. |
| `drag-disabled` | bool | Turns dragging off without unmounting (a locked row). |

Emits: `onDragStart { key, kind }`, `onDragEnd { key, kind, dropped }` — `dropped` is
false when the drag was cancelled, so an author can undo an optimistic move.

### 2.2 `HostDropTarget` — a drop sink

| slot | type | meaning |
|---|---|---|
| `drop-key` | text | Opaque identity of *this* target (the card you're dropping onto, or the column). |
| `accepts` | list<text> | The `drag-kind`s this target takes. Empty = accept nothing (an inert region). |
| `drop-disabled` | bool | Temporarily refuse drops (e.g. a WIP-limited column that's full). |

Emits:
- `onDragEnter { key, kind }` / `onDragLeave { … }` — for hover styling.
- `onDropHover { key, kind, position }` — continuous, so the author can render a drop
  indicator *before* the drop lands.
- `onDrop { key, kind, targetKey, position }` — the payload that actually mutates state.

### 2.3 `position` — the one idea that makes this general

Every drop reports **where** relative to the target:

```
before | after | into
```

That small enum is what lets one primitive family express every layout we need:

| interaction | target | position |
|---|---|---|
| reorder a list | the sibling you dropped on | `before` / `after` |
| move a card to another column | the column | `into` |
| drop a card between two cards in another column | the card below | `before` |
| nest a task under another (outline) | the parent task | `into` |
| drag a task onto a calendar day | the day cell | `into` |

Backends compute it from the pointer's position within the target's bounds (leading
third → `before`, trailing third → `after`, middle → `into`), and from the keyboard move
mode below. Authors never do hit-testing.

## 3. Non-negotiable contracts

Same stance as UI31: these are guarantees the *kernel* makes, not suggestions.

1. **Keyboard equivalence.** Every `HostDraggable` is focusable and supports a keyboard
   move mode: **Space/Enter** grabs, **arrow keys** move between valid targets,
   **Space/Enter** drops, **Escape** cancels. It emits the **same events with the same
   payload** as a pointer drag. A board that can only be operated with a mouse is broken,
   and retrofitting this later never happens — so it ships with v1.
2. **Screen-reader announcement.** Grab/move/drop transitions announce via the platform's
   live-region equivalent ("Grabbed Write spec. Moved to In Progress, position 2 of 5.").
   Backends map to `aria-live` / `AccessibilityNotification` / `QAccessible::updateAccessibility`.
3. **Touch works.** On web this means the lowering **cannot** be HTML5 drag-and-drop alone
   (it does not fire on touch); it must be pointer-events based, or HTML5 + a pointer
   fallback. Every other backend's native system already handles touch.
4. **Escape always cancels**, and cancellation emits `onDragEnd { dropped: false }`.
5. **Drops are proposals, never mutations.** The kernel moves nothing — it reports intent.
   The author (and therefore the engine) decides whether the move is legal. This keeps the
   "fat engine, dumb UI" split intact: `onDrop` is the intent event; `move_task` /
   `set_status` is the validated operation.
6. **RTL correctness.** `before`/`after` follow *reading order*, not raw x-coordinates.

## 4. Per-backend lowering

| backend | source | target | notes |
|---|---|---|---|
| react / html / webcomponent | pointer events (`pointerdown`/`move`/`up`) with a drag threshold | hit-test against registered targets | **not** HTML5 DnD — it doesn't fire on touch (contract 3) |
| swiftui | `.draggable(_:)` | `.dropDestination(for:)` | payload as `Transferable` string |
| compose | `dragAndDropSource` | `dragAndDropTarget` | Compose 1.6+; else `detectDragGestures` |
| qt | `QDrag` + `QMimeData` | `dragEnterEvent`/`dropEvent` | |
| flutter | `Draggable<String>` | `DragTarget<String>` | |
| xaml (WinUI) | `CanDrag` + `DragStarting` | `AllowDrop` + `DragOver`/`Drop` | |
| paint | — | — | out of scope: no input model |

## 5. Implementation plan

1. **Register the primitives** in `mosaic-package-resolver`'s `KERNEL_PRIMITIVES` and in
   `mosaic-analyzer`'s `is_primitive_node()` (the step UI31's notes call out as easy to
   miss — a missing entry silently emits a custom element instead of the real widget).
2. **Model/layout/analyzer**: slots + emits per §2, with `.mll` parse tests.
3. **React lowering first** (pointer-events based, with keyboard mode) — it unblocks the
   board and calendar and is where the contracts get proven.
4. **A conformance demo** (`mosaic-pkg-*` or the task-app) exercising list reorder,
   cross-container move, and the keyboard path.
5. **Remaining backends**, one PR each, each re-proving the §3 contracts.

Each step: tests → `/security-review` → PR → `/babysit-pr`.

## 6. Open questions

- **Custom drag preview** (a "ghost" of the card). Platform defaults differ a lot; v1 uses
  the platform default and revisits once the board exists.
- **Autoscroll** while dragging near a scroll container's edge — needed for long boards;
  probably a `HostScroll` collaboration rather than a drag concern.
- **Multi-select drag** (drag 3 cards at once). The `drag-key` slot is single-valued today;
  a `drag-keys` list is the obvious extension if the board wants it.

## 7. Out of scope

- Dragging **files in from the OS**, and dragging **out to other applications**.
- Drag **between windows** or between separate Mosaic app instances.
- Drag-to-**resize** (calendar event edges, Gantt bar ends). That is a distinct gesture —
  worth its own primitive later; the calendar ships drag-to-*move* first.
- Any engine change. This spec is purely the UI kernel; the task-side operations it feeds
  (`move_task`, `set_status`, `reparent`) already exist.
