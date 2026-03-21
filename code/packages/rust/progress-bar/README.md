# progress-bar

A reusable text-based progress bar for tracking concurrent operations in the terminal.

## What is this?

This crate provides a `Tracker` that receives events from concurrent threads and renders a live-updating progress bar. It uses Rust's `mpsc` channels for thread safety without locks -- many threads can send events simultaneously while a single background renderer thread draws the bar.

## Where does it fit in the stack?

The progress bar is a utility crate used by build tools and other programs that need to display progress for parallel operations.

```
Worker Thread 1 ──┐
Worker Thread 2 ──┤── mpsc channel ──► Renderer Thread ──► stderr
Worker Thread N ──┘
```

## Design

The core idea is the **postal worker pattern**: workers drop events into a channel (the mail slot), and a single renderer (the clerk) reads them and updates the display. Because only the renderer touches the state, no mutexes are needed.

## Usage

### Flat mode (simple)

```rust
use progress_bar::{Tracker, Event, EventType};

let mut tracker = Tracker::new(3, Box::new(std::io::stderr()), "");
tracker.start();
tracker.send(Event { event_type: EventType::Started, name: "pkg-a".into(), status: String::new() });
tracker.send(Event { event_type: EventType::Finished, name: "pkg-a".into(), status: "built".into() });
tracker.send(Event { event_type: EventType::Skipped, name: "pkg-b".into(), status: String::new() });
tracker.stop();
```

Output:
```
[██████████████░░░░░░]  2/3  Building: pkg-a  (1.2s)
```

### Sending from multiple threads

```rust
use progress_bar::{Tracker, Event, EventType};
use std::thread;

let mut tracker = Tracker::new(100, Box::new(std::io::stderr()), "");
tracker.start();

let mut handles = vec![];
for i in 0..100 {
    let es = tracker.event_sender();
    handles.push(thread::spawn(move || {
        let name = format!("item-{}", i);
        es.send(Event { event_type: EventType::Started, name: name.clone(), status: String::new() });
        // ... do work ...
        es.send(Event { event_type: EventType::Finished, name, status: "ok".into() });
    }));
}

for h in handles { h.join().unwrap(); }
tracker.stop();
```

### Hierarchical mode (parent/child)

```rust
use progress_bar::{Tracker, Event, EventType};

let mut parent = Tracker::new(3, Box::new(std::io::stderr()), "Level");
parent.start();

let mut child = parent.child(7, "Package");
child.send(Event { event_type: EventType::Started, name: "pkg-a".into(), status: String::new() });
child.send(Event { event_type: EventType::Finished, name: "pkg-a".into(), status: "built".into() });
child.finish();   // advances parent by 1

parent.stop();
```

Output:
```
Level 1/3  [████░░░░░░░░░░░░░░░░]  Building: pkg-a  (2.1s)
```

## Key properties

- **Thread-safe**: `send()` and `event_sender()` work from any thread
- **No external dependencies**: stdlib only (`std::sync::mpsc`, `std::thread`, `std::io`)
- **Generic writer**: accepts any `Box<dyn Write + Send>` for testability
- **Unicode bar**: uses block characters for crisp rendering on modern terminals
