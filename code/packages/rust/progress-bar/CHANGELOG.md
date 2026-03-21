# Changelog

## 0.1.0 -- 2026-03-21

### Added
- `EventType` enum: Started, Finished, Skipped for tracking item lifecycle
- `Event` struct: minimal message type with event_type, name, and status fields
- `Tracker` struct with channel-based concurrency (mpsc sender/receiver pattern)
- `Tracker::new()` -- creates tracker with configurable total, writer, and label
- `Tracker::start()` -- API compatibility entry point (renderer spawned in new())
- `Tracker::send()` -- thread-safe event submission via channel
- `Tracker::child()` -- hierarchical sub-tracker creation
- `Tracker::child_with_writer()` -- child with custom writer for testing
- `Tracker::finish()` -- child completion with parent notification
- `Tracker::stop()` -- graceful shutdown with final render
- `Tracker::event_sender()` -- cloneable `EventSender` handle for cross-thread use
- `EventSender` -- public wrapper for sending events from spawned threads
- 20-character Unicode progress bar with filled/empty block characters
- In-flight name display (up to 3 sorted alphabetically, "+N more" truncation)
- Elapsed time display in seconds
- Flat mode, labeled mode, and hierarchical parent/child mode
- Generic writer via `Box<dyn Write + Send>` for testability
- Zero external dependencies -- stdlib only
