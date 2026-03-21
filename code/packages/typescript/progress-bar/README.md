# @coding-adventures/progress-bar

A reusable, text-based progress bar for tracking concurrent operations in the terminal. Zero runtime dependencies.

## What it does

Renders a Unicode progress bar that updates in-place using carriage returns. Supports both flat (single-level) and hierarchical (parent/child) progress tracking.

```
[████████░░░░░░░░░░░░]  7/21  Building: pkg-a, pkg-b  (12.3s)
```

In hierarchical mode, a parent tracker wraps child trackers to show multi-level progress:

```
Level 2/3  [████░░░░░░░░░░░░░░░░]  5/12  Building: pkg-a  (8.2s)
```

## How it fits in the stack

This package provides progress feedback for the build system. When the build tool compiles packages across dependency levels, the progress bar shows what's happening and how far along we are.

## Usage

### Flat mode (simple)

```typescript
import { Tracker, EventType } from "@coding-adventures/progress-bar";

const tracker = new Tracker(21, { write: (s) => process.stderr.write(s) });
tracker.start();

tracker.send({ type: EventType.Started, name: "pkg-a" });
// ... do work ...
tracker.send({ type: EventType.Finished, name: "pkg-a", status: "built" });

tracker.send({ type: EventType.Skipped, name: "pkg-b" });

tracker.stop();
```

### Hierarchical mode (multi-level)

```typescript
import { Tracker, EventType } from "@coding-adventures/progress-bar";

const writer = { write: (s: string) => process.stderr.write(s) };
const parent = new Tracker(3, writer, "Level");
parent.start();

// Level 1: 7 packages
const child1 = parent.child(7, "Package");
child1.send({ type: EventType.Started, name: "pkg-a" });
child1.send({ type: EventType.Finished, name: "pkg-a", status: "built" });
// ... more events ...
child1.finish(); // advances parent by 1

// Level 2: 12 packages
const child2 = parent.child(12, "Package");
// ... events ...
child2.finish();

parent.stop();
```

### Disabled progress (NullTracker)

```typescript
import { Tracker, NullTracker, EventType } from "@coding-adventures/progress-bar";

// Use NullTracker when progress display is disabled
const tracker = verbose ? new Tracker(10, writer) : new NullTracker();
tracker.start();
tracker.send({ type: EventType.Started, name: "pkg-a" }); // no-op if NullTracker
tracker.stop();
```

## API

### `Tracker`

- `constructor(total: number, writer: Writable, label?: string)` — Create a tracker expecting `total` items.
- `start()` — Begin tracking (records start time).
- `send(event: Event)` — Process an event and redraw the bar.
- `child(total: number, label: string)` — Create a child tracker for hierarchical progress.
- `finish()` — Mark child complete and advance parent.
- `stop()` — Print final newline and stop.

### `NullTracker`

Same interface as Tracker, but all methods are no-ops. Implements the Null Object pattern.

### `EventType`

- `EventType.Started` — Item began processing.
- `EventType.Finished` — Item completed.
- `EventType.Skipped` — Item was skipped.

### `Event`

```typescript
interface Event {
  type: EventType;
  name: string;
  status?: string;
}
```

### `Writable`

```typescript
interface Writable {
  write(s: string): void;
}
```

## Design decisions

- **Synchronous rendering**: Unlike the Go version (channels + goroutines), this is synchronous. Node.js is single-threaded, so `send()` updates state and redraws directly. No timers, no debouncing.
- **Dependency injection**: The `Writable` interface makes the tracker testable without capturing stderr.
- **Null Object pattern**: `NullTracker` eliminates null checks at call sites.
- **Unicode block characters**: `█` (filled) and `░` (empty) work on all modern terminals.
- **Carriage return**: Uses `\r` instead of ANSI escape codes for maximum compatibility.

## Testing

```bash
npm test              # Run tests
npm run test:coverage # Run with coverage report
```
