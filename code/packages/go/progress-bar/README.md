# Progress Bar

A reusable, text-based progress bar for tracking concurrent operations in the terminal. Part of the [coding-adventures](https://github.com/adhithyan15/coding-adventures) project.

## What It Does

Renders a live-updating progress bar to stderr that shows:
- A visual bar with completed/total count
- Names of currently in-flight items
- Elapsed wall-clock time

```
[████████░░░░░░░░░░░░]  7/21  Building: pkg-a, pkg-b, pkg-c  (12.3s)
```

## How It Fits in the Stack

This is a utility package at the bottom of the dependency tree — it depends on nothing else in the monorepo. It's used by the build tool to show progress during parallel package builds, but it's generic enough for any program that processes a known number of items concurrently.

## Usage

### Flat Mode (Simple)

```go
import progress "github.com/adhithyan15/coding-adventures/code/packages/go/progress-bar"

tracker := progress.New(21, os.Stderr, "")
tracker.Start()

// From any goroutine:
tracker.Send(progress.Event{Type: progress.Started, Name: "pkg-a"})
tracker.Send(progress.Event{Type: progress.Finished, Name: "pkg-a", Status: "built"})
tracker.Send(progress.Event{Type: progress.Skipped, Name: "pkg-b"})

tracker.Stop()
```

### Hierarchical Mode (Grouped Progress)

```go
parent := progress.New(3, os.Stderr, "Level")
parent.Start()

child := parent.Child(7, "Package")
child.Send(progress.Event{Type: progress.Started, Name: "pkg-a"})
child.Send(progress.Event{Type: progress.Finished, Name: "pkg-a", Status: "built"})
child.Finish()  // advances parent by 1

parent.Stop()
// Display: Level 1/3  [████░░░░░░░░░░░░░░░░]  1/7  Building: pkg-a  (2.1s)
```

### Nil Safety

The tracker is safe to use when nil — all methods are no-ops:

```go
var tracker *progress.Tracker  // nil
tracker.Send(progress.Event{Type: progress.Started, Name: "test"})  // no-op, no panic
```

## Concurrency

Send can be called from any goroutine. Internally, events are serialized through a buffered channel and processed by a single renderer goroutine. No explicit locking is needed.

## Design

See `code/specs/progress-bar.md` for the full specification.
