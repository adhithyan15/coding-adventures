# Progress Bar — Reusable Text-Based Progress Tracker

## Overview

A reusable, dependency-free progress bar library for tracking the progress of
concurrent operations in the terminal. Designed as a general-purpose building
block — any program that processes a known number of items can use it.

The primary motivation is the build tool, which goes silent during package
execution. But the package is deliberately generic: it knows nothing about
builds, packages, or dependency graphs. It only needs a total count and events.

## Layer

Utility — no dependencies on other coding-adventures packages.

## API Surface

### Core Types

#### EventType

An enumeration of what can happen to a tracked item:

| Event     | Meaning                                      |
|-----------|----------------------------------------------|
| `Started` | An item began processing (now "in-flight")   |
| `Finished`| An item completed processing (success or failure) |
| `Skipped` | An item was skipped without processing       |

#### Event

A struct/record carrying:

| Field   | Type       | Description                              |
|---------|------------|------------------------------------------|
| `type`  | EventType  | What happened                            |
| `name`  | string     | Human-readable item identifier           |
| `status`| string     | Outcome label (e.g., "built", "failed", "cached") — only meaningful for `Finished` events |

#### Tracker

The main object. Created with:

| Parameter | Type      | Description                                       |
|-----------|-----------|---------------------------------------------------|
| `total`   | integer   | Total number of items that will be processed       |
| `writer`  | io.Writer | Output destination (typically stderr)              |
| `label`   | string    | Optional prefix label (e.g., "Level", "Package")  |

### Methods

| Method                        | Description                                                    |
|-------------------------------|----------------------------------------------------------------|
| `Start()`                     | Launch the background renderer                                 |
| `Send(event)`                 | Submit an event — safe to call from any thread/goroutine/task  |
| `Child(total, label) → Tracker` | Create a nested sub-tracker for hierarchical progress        |
| `Finish()`                    | Mark this child tracker as complete, advancing the parent by 1 |
| `Stop()`                      | Shut down the renderer, print final newline                    |

### Lifecycle

```
tracker = New(total=21, writer=stderr)
tracker.Start()

// From concurrent workers:
tracker.Send(Event{Started, "pkg-a", ""})
tracker.Send(Event{Finished, "pkg-a", "built"})
tracker.Send(Event{Skipped, "pkg-b", "cached"})

tracker.Stop()
```

### Hierarchical Usage

For operations that have natural groupings (e.g., build levels):

```
parent = New(total=3, writer=stderr, label="Level")
parent.Start()

child = parent.Child(total=7, label="Package")
child.Send(Event{Started, "pkg-a", ""})
child.Send(Event{Finished, "pkg-a", "built"})
child.Finish()   // advances parent by 1

child2 = parent.Child(total=5, label="Package")
// ... process level 2 ...
child2.Finish()

parent.Stop()
```

## Rendering

### Flat Mode

When no label is set or no hierarchy is active:

```
[████████░░░░░░░░░░░░]  7/21  Building: pkg-a, pkg-b, pkg-c  (12.3s)
```

### Hierarchical Mode

When a parent label is set and a child is active:

```
Level 2/3  [████░░░░░░░░░░░░░░░░]  5/12  Building: pkg-a, pkg-b  (8.2s)
```

### Bar Specification

| Element            | Detail                                           |
|--------------------|--------------------------------------------------|
| Bar width          | 20 characters                                    |
| Filled character   | `█` (U+2588 FULL BLOCK)                         |
| Empty character    | `░` (U+2591 LIGHT SHADE)                        |
| Counter            | `completed/total`                                |
| In-flight names    | Up to 3 names, then `+N more`                   |
| Elapsed time       | Wall-clock seconds since `Start()`, format `%.1fs` |
| Idle state         | When nothing in-flight: display `waiting...`     |
| Complete state     | When all items done: display `done`              |

### Output Target

All progress output goes to **stderr** so that stdout remains clean for
piping and scripting. The `writer` parameter allows tests to capture output
with a buffer instead.

### Terminal Technique

Use carriage return (`\r`) to overwrite the current line. No ANSI escape
codes required — `\r` works on all platforms (Windows cmd, PowerShell, Git
Bash, Unix terminals).

Pad each line to a fixed width (or to the previous line's length) to ensure
shorter lines fully overwrite longer ones.

`Stop()` prints a final newline to preserve the last progress line in the
terminal scrollback.

## Concurrency Model

Events arrive from multiple concurrent sources (goroutines, threads, tasks).
The tracker must handle this safely **without requiring callers to synchronize**.

### Recommended Patterns by Language

| Language    | Event transport          | Renderer                    |
|-------------|--------------------------|-----------------------------|
| Go          | Buffered channel         | Goroutine reading channel   |
| Python      | `queue.Queue`            | `threading.Thread`          |
| Ruby        | `Thread::Queue`          | `Thread.new`                |
| Rust        | `std::sync::mpsc`        | `std::thread::spawn`        |
| TypeScript  | Synchronous (single-threaded) | `send()` triggers redraw directly (debounced) |
| Elixir      | `GenServer` cast         | GenServer process           |

The key invariant: **all tracker state mutation happens in a single
renderer context** (goroutine/thread/process). Senders only write to the
transport. This eliminates the need for explicit locks on tracker state.

## Nil/Null Safety

When the tracker is nil/null/None, `Send()` must be a no-op. This lets
callers unconditionally call `Send()` without nil-checking, which keeps
integration code clean:

```go
// Go example — tracker may be nil
tracker.Send(progress.Event{Type: progress.Started, Name: "pkg-a"})
```

For Go, this means `Send` is a method on `*Tracker` that checks for nil
receiver. For Python/Ruby/TypeScript, a `NullTracker` or conditional check
in the calling code. For Elixir, pattern-match on `nil` pid.

## Testing Strategy

All implementations must achieve >80% test coverage (target 95%+).

### Required Test Cases

1. **Event counting** — Started increments in-flight, Finished decrements
   in-flight and increments completed, Skipped increments completed
2. **Bar rendering** — Known sequence of events produces expected output
   string (verify bar characters, counter, names, elapsed format)
3. **Name truncation** — More than 3 in-flight items shows first 3 + "+N more"
4. **Hierarchical progress** — Parent advances when child finishes, display
   shows parent label and child state
5. **Concurrent sends** — Multiple threads/goroutines send events
   simultaneously without races or panics
6. **Stop cleanup** — Stop closes resources, final newline is written
7. **Nil safety** — Sending to nil tracker does not panic

### Testing Output

Use an in-memory buffer (e.g., `bytes.Buffer`, `io.StringIO`, `StringIO`)
as the writer to capture and assert on rendered output without touching
the real terminal.

For concurrency tests, use language-specific race detectors where available
(Go's `-race`, Rust's `--release` + TSAN, Python's `threading` stress tests).

## Cross-Language Implementations

Each language follows its ecosystem's conventions:

| Language   | Package location                          | Build system     |
|------------|-------------------------------------------|------------------|
| Go         | `code/packages/go/progress-bar/`          | `go.mod`         |
| Python     | `code/packages/python/progress-bar/`      | `pyproject.toml` |
| Ruby       | `code/packages/ruby/progress_bar/`        | `.gemspec`       |
| Rust       | `code/packages/rust/progress-bar/`        | `Cargo.toml`     |
| TypeScript | `code/packages/typescript/progress-bar/`  | `package.json`   |
| Elixir     | `code/packages/elixir/progress_bar/`      | `mix.exs`        |

Each package includes: BUILD, README.md, CHANGELOG.md, source, and tests.

## Literate Programming

All implementations must follow Knuth-style literate programming:

- Explain **why** before each non-obvious section
- Include truth tables, state diagrams, or timing diagrams where helpful
- Use analogies to make concurrency patterns accessible to newcomers
- Document the channel/queue pattern with a "postal worker" or "mailbox" analogy
