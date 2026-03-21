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

## Installation

```bash
pip install -e ".[dev]"
```

## Usage

### Flat Mode (Simple)

```python
import sys
from progress_bar import Tracker, Event, EventType

tracker = Tracker(total=21, writer=sys.stderr)
tracker.start()

# From any thread:
tracker.send(Event(type=EventType.STARTED, name="pkg-a"))
tracker.send(Event(type=EventType.FINISHED, name="pkg-a", status="built"))
tracker.send(Event(type=EventType.SKIPPED, name="pkg-b"))

tracker.stop()
```

### Hierarchical Mode (Grouped Progress)

```python
parent = Tracker(total=3, writer=sys.stderr, label="Level")
parent.start()

child = parent.child(total=7, label="Package")
child.send(Event(type=EventType.STARTED, name="pkg-a"))
child.send(Event(type=EventType.FINISHED, name="pkg-a", status="built"))
child.finish()  # advances parent by 1

parent.stop()
# Display: Level 1/3  [████░░░░░░░░░░░░░░░░]  1/7  Building: pkg-a  (2.1s)
```

### Disabling Progress (NullTracker)

When you want to conditionally disable progress display without adding `if` checks everywhere:

```python
from progress_bar import NullTracker

# Pick the tracker based on a flag:
if verbose:
    tracker = Tracker(total=10, writer=sys.stderr)
else:
    tracker = NullTracker()

# The rest of the code doesn't need to care:
tracker.start()
tracker.send(Event(type=EventType.STARTED, name="pkg-a"))
tracker.stop()
```

## Concurrency

`send()` can be called from any thread. Internally, events are serialized through a `queue.Queue` and processed by a single renderer thread. No explicit locking is needed.

## Design

See `code/specs/progress-bar.md` for the full specification.
