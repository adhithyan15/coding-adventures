# Progress Bar (Elixir)

A reusable, text-based progress bar for tracking concurrent operations in the terminal. Part of the [coding-adventures](https://github.com/adhithyan15/coding-adventures) project.

## What It Does

Renders a live-updating progress bar to an IO device (typically stderr) that shows:
- A visual bar with completed/total count
- Names of currently in-flight items (up to 3, then "+N more")
- Elapsed wall-clock time

```
[████████░░░░░░░░░░░░]  7/21  Building: pkg-a, pkg-b, pkg-c  (12.3s)
```

## How It Fits in the Stack

This is a utility package at the bottom of the dependency tree — it depends on nothing else in the monorepo. It is the Elixir port of the Go `progress-bar` package, using OTP GenServer instead of Go channels. It is generic enough for any program that processes a known number of items concurrently.

## Design: The GenServer as Postal Clerk

The Go version uses a buffered channel and a goroutine. The Elixir version uses a GenServer process and the BEAM process mailbox — which is conceptually the same pattern, but built into the language:

| Go concept            | Elixir equivalent            |
|-----------------------|------------------------------|
| buffered channel      | process mailbox (unbounded)  |
| goroutine             | GenServer process            |
| channel send (`ch <-`)| `GenServer.cast` (async)     |
| channel close + done  | `GenServer.stop` (sync)      |

## Usage

### Flat Mode (Simple)

```elixir
alias CodingAdventures.ProgressBar

{:ok, tracker} = ProgressBar.start_link(total: 21, writer: :stderr)

# From any process (Task, GenServer, etc.):
ProgressBar.send_event(tracker, :started, "pkg-a")
ProgressBar.send_event(tracker, :finished, "pkg-a", "built")
ProgressBar.send_event(tracker, :skipped, "pkg-b")

ProgressBar.stop(tracker)
```

### Hierarchical Mode (Grouped Progress)

For multi-level progress (e.g., a build system with dependency levels):

```elixir
alias CodingAdventures.ProgressBar

{:ok, parent} = ProgressBar.start_link(total: 3, writer: :stderr, label: "Level")

{:ok, child} = ProgressBar.child(parent, 7, "Package")
ProgressBar.send_event(child, :started, "pkg-a")
ProgressBar.send_event(child, :finished, "pkg-a", "built")
ProgressBar.finish(child)   # advances parent by 1

ProgressBar.stop(parent)
# Display: Level 1/3  [████░░░░░░░░░░░░░░░░]  1/7  Building: pkg-a  (2.1s)
```

### Nil Safety

All public functions accept `nil` as the pid argument and return `:ok` (a no-op). This lets callers unconditionally call functions without nil-checking:

```elixir
tracker = nil
ProgressBar.send_event(tracker, :started, "test")  # no-op, no crash
ProgressBar.child(tracker, 5, "test")               # returns nil
```

### Concurrent Usage with Tasks

```elixir
alias CodingAdventures.ProgressBar

{:ok, tracker} = ProgressBar.start_link(total: 100, writer: :stderr)

tasks =
  for i <- 1..100 do
    Task.async(fn ->
      name = "item-#{i}"
      ProgressBar.send_event(tracker, :started, name)
      # ... do work ...
      ProgressBar.send_event(tracker, :finished, name, "ok")
    end)
  end

Task.await_many(tasks)
ProgressBar.stop(tracker)
```

## Event Types

| Event      | Effect on completed | Effect on building set |
|------------|--------------------|-----------------------|
| `:started` | unchanged          | add name              |
| `:finished`| +1                 | remove name           |
| `:skipped` | +1                 | unchanged             |

## OTP Supervision

The tracker is a standard GenServer and can be placed under a supervisor:

```elixir
children = [
  {CodingAdventures.ProgressBar.Tracker, total: 10, writer: :stderr}
]

Supervisor.start_link(children, strategy: :one_for_one)
```
