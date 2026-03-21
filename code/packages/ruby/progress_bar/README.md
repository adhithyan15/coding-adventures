# CodingAdventures::ProgressBar

A reusable, thread-safe, text-based progress bar for tracking concurrent operations in the terminal.

## What It Does

This gem renders a live-updating progress bar that shows:

- A 20-character Unicode bar (`[████████████░░░░░░░░]`)
- Completed / total count
- Names of items currently being processed (up to 3, sorted alphabetically)
- Elapsed time in seconds

It is the Ruby port of the Go `progress-bar` package and follows the same architecture: a `Thread::Queue` for event delivery and a background thread for rendering.

## How It Fits in the Stack

The progress bar is used by the build tool to display build progress. Each package being built sends events (started, finished, skipped) to the tracker, which renders the progress bar to stderr. The hierarchical mode allows showing both the current dependency level and the packages within that level.

## Installation

```ruby
gem "coding_adventures_progress_bar", path: "code/packages/ruby/progress_bar"
```

## Usage

### Flat Mode (Single Level)

```ruby
require "coding_adventures_progress_bar"

include CodingAdventures::ProgressBar

tracker = Tracker.new(21, $stderr)
tracker.start

tracker.send_event(Event.new(type: EventType::STARTED, name: "logic-gates"))
# ... do work ...
tracker.send_event(Event.new(type: EventType::FINISHED, name: "logic-gates", status: "built"))

tracker.send_event(Event.new(type: EventType::SKIPPED, name: "matrix"))

tracker.stop
```

Output:

```
[████████░░░░░░░░░░░░]  7/21  Building: logic-gates, parser  (12.3s)
```

### Hierarchical Mode (Parent / Child)

For multi-level builds where you want to show both the level and the packages within it:

```ruby
parent = Tracker.new(3, $stderr, "Level")
parent.start

# Level 1: 7 packages
child = parent.child(7, "Package")
child.send_event(Event.new(type: EventType::STARTED, name: "logic-gates"))
child.send_event(Event.new(type: EventType::FINISHED, name: "logic-gates", status: "built"))
# ... more events ...
child.finish  # advances parent by 1

# Level 2: 5 packages
child2 = parent.child(5, "Package")
# ... events ...
child2.finish

parent.stop
```

Output:

```
Level 2/3  [████████████░░░░░░░░]  3/5  Building: parser, vm  (8.2s)
```

### NullTracker (Disable Progress)

When you want to disable progress display without changing calling code:

```ruby
tracker = verbose ? Tracker.new(10, $stderr) : NullTracker.new
tracker.start
tracker.send_event(event)  # works either way, no nil checks needed
tracker.stop
```

## Thread Safety

The tracker is fully thread-safe. Multiple threads can call `send_event` simultaneously -- events are delivered via `Thread::Queue` and processed by a single background thread.

```ruby
threads = packages.map do |pkg|
  Thread.new do
    tracker.send_event(Event.new(type: EventType::STARTED, name: pkg.name))
    pkg.build!
    tracker.send_event(Event.new(type: EventType::FINISHED, name: pkg.name, status: "built"))
  end
end
threads.each(&:join)
```

## Running Tests

```bash
bundle install
bundle exec rake test
```
