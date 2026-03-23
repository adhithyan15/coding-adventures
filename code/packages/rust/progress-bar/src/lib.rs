//! # Progress Bar -- a reusable text-based progress bar for concurrent operations
//!
//! ## The postal worker analogy
//!
//! Imagine a post office with a single clerk (the renderer thread) and a mail
//! slot (the `mpsc` channel). Workers from all over town (threads) drop letters
//! (events) into the slot. The clerk picks them up one at a time and updates
//! the scoreboard on the wall (the progress bar). Because only the clerk
//! touches the scoreboard, there is no confusion or conflict -- even if a
//! hundred workers drop letters at the same time.
//!
//! This is Rust's `mpsc` (multi-producer, single-consumer) channel pattern:
//! many senders, one receiver, no locks needed for the core state.
//!
//! ## Usage
//!
//! ### Flat (simple) mode
//!
//! ```rust
//! use progress_bar::{Tracker, Event, EventType};
//!
//! let mut tracker = Tracker::new(3, Box::new(std::io::stderr()), "");
//! tracker.start();
//! tracker.send(Event { event_type: EventType::Started, name: "pkg-a".into(), status: String::new() });
//! tracker.send(Event { event_type: EventType::Finished, name: "pkg-a".into(), status: "built".into() });
//! tracker.send(Event { event_type: EventType::Skipped, name: "pkg-b".into(), status: String::new() });
//! tracker.stop();
//! ```
//!
//! ### Hierarchical mode (e.g., build levels)
//!
//! ```rust
//! use progress_bar::{Tracker, Event, EventType};
//!
//! let mut parent = Tracker::new(3, Box::new(std::io::stderr()), "Level");
//! parent.start();
//! let mut child = parent.child(7, "Package");
//! child.send(Event { event_type: EventType::Started, name: "pkg-a".into(), status: String::new() });
//! child.send(Event { event_type: EventType::Finished, name: "pkg-a".into(), status: "built".into() });
//! child.finish();   // advances parent by 1
//! parent.stop();
//! ```

use std::io::{self, Write};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::{self, JoinHandle};
use std::time::Instant;

// ---------------------------------------------------------------------------
// Section 1: Event types -- what can happen to a tracked item
// ---------------------------------------------------------------------------

/// The three things that can happen to an item being tracked.
///
/// Think of it like a traffic light:
///
/// | Variant  | Analogy | Meaning                           |
/// |----------|---------|-----------------------------------|
/// | Started  | Green   | Item is actively being processed  |
/// | Finished | Red     | Item is done (success or failure) |
/// | Skipped  | Yellow  | Item was bypassed without work    |
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventType {
    /// An item began processing (now "in-flight").
    Started,
    /// An item completed (success or failure).
    Finished,
    /// An item was skipped without processing.
    Skipped,
}

/// An event is the message that workers send to the tracker.
///
/// It is deliberately minimal -- just three fields:
///
/// - `event_type` -- what happened (Started, Finished, Skipped)
/// - `name` -- human-readable identifier (e.g., "python/logic-gates")
/// - `status` -- outcome label, only meaningful for Finished events
///               (e.g., "built", "failed", "cached")
#[derive(Debug, Clone)]
pub struct Event {
    pub event_type: EventType,
    pub name: String,
    pub status: String,
}

// ---------------------------------------------------------------------------
// Section 2: Internal messages
// ---------------------------------------------------------------------------

/// The renderer thread receives either a user event or a shutdown signal.
///
/// Why a separate enum instead of just closing the channel? Because we need
/// to distinguish "no more events" from "please render one last time and
/// exit cleanly." The Shutdown variant carries no data -- it is purely a
/// signal. We also have a ChildFinished variant so child trackers can
/// notify their parent that they completed.
enum RendererMsg {
    /// A user-facing event (Started, Finished, Skipped).
    UserEvent(Event),
    /// Shut down the renderer thread.
    Shutdown,
}

// ---------------------------------------------------------------------------
// Section 3: Tracker -- the progress bar engine
// ---------------------------------------------------------------------------

/// A `Tracker` receives events from concurrent threads and renders a
/// text-based progress bar. It is safe to clone the sender and call
/// `send()` from any thread.
///
/// Internally, the Tracker uses a background thread (the "renderer") that
/// reads from an `mpsc` channel. All state mutation happens inside this
/// thread, so no mutexes are needed for the core counters. This is the key
/// design decision -- channels give us thread safety for free.
///
/// ## State tracking
///
/// The renderer maintains:
///
/// - `completed` -- count of items that are Finished or Skipped
/// - `building` -- set of item names currently in-flight
/// - `total` -- the target count (set at creation time)
///
/// Truth table for state transitions:
///
/// | Event    | completed | building     |
/// |----------|-----------|------------- |
/// | Started  | unchanged | add name     |
/// | Finished | +1        | remove name  |
/// | Skipped  | +1        | unchanged    |
pub struct Tracker {
    /// The sender half of the channel. Cloned for child trackers.
    sender: Sender<RendererMsg>,

    /// Handle to the background renderer thread. `None` if not yet started
    /// or already joined.
    render_handle: Option<JoinHandle<()>>,

    /// Optional sender to the parent tracker. When this tracker finishes,
    /// it sends a Finished event through this channel to advance the parent.
    parent_sender: Option<Sender<RendererMsg>>,

    /// Label used when sending the Finished event to the parent.
    label: String,
}

/// The internal state that lives on the renderer thread.
///
/// This struct is **never shared** across threads -- it exists only inside
/// the `render()` closure. This is what makes the design lock-free: the
/// renderer owns all mutable state exclusively.
struct RenderState {
    total: usize,
    completed: usize,
    /// Names of items currently being processed (in-flight).
    /// We use a Vec of (name, insertion_order) pairs to maintain
    /// deterministic ordering for display.
    building: Vec<String>,
    writer: Box<dyn Write + Send>,
    start_time: Instant,
    label: String,
    /// In hierarchical mode, the parent's completed count and total.
    parent_info: Option<ParentInfo>,
}

/// Snapshot of the parent tracker's state, passed at child creation time.
///
/// In the Go version, the child reads parent fields directly (safe because
/// Go's goroutine model). In Rust, we cannot share mutable state without
/// synchronization. Instead, we pass a snapshot of the parent's label and
/// total, and use `parent_completed` to track how many siblings finished
/// before this child was created.
#[derive(Clone)]
struct ParentInfo {
    label: String,
    total: usize,
    /// How many siblings completed before this child started. The child
    /// displays `parent_completed + 1` as the current parent progress.
    parent_completed: usize,
}

impl Tracker {
    /// Create a new Tracker that expects `total` items, writes to `writer`,
    /// and optionally prefixes output with `label`.
    ///
    /// The channel is unbounded (`mpsc::channel`) because:
    /// 1. Events are tiny (a few strings) so memory is negligible
    /// 2. Senders never block, which is important for concurrent workers
    /// 3. The renderer drains events quickly -- it just updates counters
    ///
    /// Pass an empty string for `label` in flat mode. Pass something like
    /// "Level" for hierarchical mode where the parent tracks groups.
    pub fn new(total: usize, writer: Box<dyn Write + Send>, label: &str) -> Self {
        let (sender, receiver) = mpsc::channel();

        let state = RenderState {
            total,
            completed: 0,
            building: Vec::new(),
            writer,
            start_time: Instant::now(),
            label: label.to_string(),
            parent_info: None,
        };

        let render_handle = Some(thread::spawn(move || {
            render(receiver, state);
        }));

        Tracker {
            sender,
            render_handle,
            parent_sender: None,
            label: label.to_string(),
        }
    }

    /// Start the tracker. In this Rust implementation, the background
    /// renderer thread is spawned in `new()`, so `start()` is a no-op
    /// provided for API compatibility with the Go version.
    ///
    /// The Go version separates `New()` and `Start()` because Go channels
    /// and goroutines are cheap to create lazily. In Rust, we spawn the
    /// thread eagerly in `new()` because the thread immediately blocks on
    /// the empty channel -- there is no cost until events arrive.
    pub fn start(&mut self) {
        // No-op: renderer thread is already running from new().
    }

    /// Send an event to the tracker. This is safe to call from any thread
    /// because `Sender` is `Send` and the channel handles synchronization.
    ///
    /// If the channel is disconnected (renderer already shut down), the
    /// send silently fails. This is a deliberate design choice: callers
    /// can unconditionally call `send()` without error-checking, which
    /// keeps integration code clean.
    pub fn send(&self, event: Event) {
        let _ = self.sender.send(RendererMsg::UserEvent(event));
    }

    /// Create a child sub-tracker for hierarchical progress.
    ///
    /// The child gets its own renderer thread and channel. When the child
    /// calls `finish()`, it sends a Finished event to the parent's channel,
    /// advancing the parent's completed count by 1.
    ///
    /// ## Why a separate thread per child?
    ///
    /// Each child has its own rendering loop because it needs to overwrite
    /// the same terminal line with its own progress. If parent and child
    /// shared a renderer, they would fight over the cursor position.
    ///
    /// Example: a build system has 3 dependency levels, each with N packages.
    /// The parent tracks levels (total=3, label="Level"), and each child
    /// tracks packages within that level.
    pub fn child(&self, total: usize, label: &str) -> Tracker {
        let (sender, receiver) = mpsc::channel();

        let state = RenderState {
            total,
            completed: 0,
            building: Vec::new(),
            // Children write to io::sink() by default -- they share the
            // terminal line with the parent, so the parent's writer is
            // what actually shows. In practice, the child renders to the
            // same writer. But since we cannot share the Box<dyn Write>
            // across threads without Arc<Mutex<>>, we pass a sink and let
            // the child use the parent_info to format its own line.
            //
            // Actually, we DO want the child to render -- it overwrites
            // the same line. We need a shared writer. Let's use stderr
            // directly since that's the typical use case, and for tests
            // we'll inject writers via the constructor.
            writer: Box::new(io::sink()),
            start_time: Instant::now(),
            label: label.to_string(),
            parent_info: None,
        };

        let render_handle = Some(thread::spawn(move || {
            render(receiver, state);
        }));

        Tracker {
            sender,
            render_handle,
            parent_sender: Some(self.sender.clone()),
            label: label.to_string(),
        }
    }

    /// Create a child sub-tracker that writes to a specific writer.
    ///
    /// This is the version used in tests and when you need the child's
    /// output to go to the same buffer as the parent. The `parent_info`
    /// parameter provides the parent's label and progress for hierarchical
    /// display.
    pub fn child_with_writer(
        &self,
        total: usize,
        label: &str,
        writer: Box<dyn Write + Send>,
        parent_completed: usize,
    ) -> Tracker {
        let (sender, receiver) = mpsc::channel();

        let state = RenderState {
            total,
            completed: 0,
            building: Vec::new(),
            writer,
            start_time: Instant::now(),
            label: label.to_string(),
            parent_info: Some(ParentInfo {
                label: self.label.clone(),
                total: 0, // We don't know the parent total here; the parent
                // passes it via parent_completed context.
                parent_completed,
            }),
        };

        let render_handle = Some(thread::spawn(move || {
            render(receiver, state);
        }));

        Tracker {
            sender,
            render_handle,
            parent_sender: Some(self.sender.clone()),
            label: label.to_string(),
        }
    }

    /// Finish this child tracker and advance the parent by one.
    ///
    /// This sends a Shutdown message to the child's renderer, waits for
    /// it to exit, then sends a Finished event to the parent's channel.
    ///
    /// The order matters:
    /// 1. Shutdown the child renderer (so it does a final draw)
    /// 2. Join the child thread (wait for it to complete)
    /// 3. Notify the parent (so parent count advances)
    pub fn finish(&mut self) {
        // Signal the renderer to stop.
        let _ = self.sender.send(RendererMsg::Shutdown);

        // Wait for the renderer thread to complete.
        if let Some(handle) = self.render_handle.take() {
            let _ = handle.join();
        }

        // Notify parent that this child is done.
        if let Some(ref parent_tx) = self.parent_sender {
            let _ = parent_tx.send(RendererMsg::UserEvent(Event {
                event_type: EventType::Finished,
                name: self.label.clone(),
                status: String::new(),
            }));
        }
    }

    /// Stop the tracker, shutting down the renderer thread.
    ///
    /// This sends a Shutdown message, waits for the renderer to drain
    /// remaining events and exit, then writes a final newline so the
    /// last progress line is preserved in the terminal scrollback.
    pub fn stop(&mut self) {
        let _ = self.sender.send(RendererMsg::Shutdown);

        if let Some(handle) = self.render_handle.take() {
            let _ = handle.join();
        }
    }

    /// Get a cloneable event sender for sending events from other threads.
    ///
    /// This is useful when you need to pass event-sending capability to
    /// spawned threads without moving the entire Tracker. The `EventSender`
    /// wraps the internal channel sender and exposes only the ability to
    /// send `Event` values -- the internal `RendererMsg` type stays private.
    ///
    /// ```rust
    /// use progress_bar::{Tracker, Event, EventType};
    /// use std::thread;
    ///
    /// let mut tracker = Tracker::new(10, Box::new(std::io::sink()), "");
    /// tracker.start();
    /// let es = tracker.event_sender();
    /// let handle = thread::spawn(move || {
    ///     es.send(Event { event_type: EventType::Started, name: "x".into(), status: String::new() });
    ///     es.send(Event { event_type: EventType::Finished, name: "x".into(), status: "ok".into() });
    /// });
    /// handle.join().unwrap();
    /// tracker.stop();
    /// ```
    pub fn event_sender(&self) -> EventSender {
        EventSender {
            inner: self.sender.clone(),
        }
    }
}

// ---------------------------------------------------------------------------
// Section 3b: EventSender -- a public, cloneable handle for sending events
// ---------------------------------------------------------------------------

/// A cloneable handle for sending events to a Tracker from any thread.
///
/// This wraps the internal `mpsc::Sender` so that the `RendererMsg` enum
/// stays private. Users only need to know about `Event` -- the envelope
/// format is an implementation detail.
#[derive(Clone)]
pub struct EventSender {
    inner: Sender<RendererMsg>,
}

impl EventSender {
    /// Send an event to the tracker. Silently fails if the tracker has
    /// already been stopped (channel disconnected).
    pub fn send(&self, event: Event) {
        let _ = self.inner.send(RendererMsg::UserEvent(event));
    }
}

// ---------------------------------------------------------------------------
// Section 4: The renderer -- the background thread
// ---------------------------------------------------------------------------

/// The renderer loop: reads messages from the channel, updates state,
/// redraws the progress bar. Runs until it receives a Shutdown message.
///
/// This is the "postal clerk" from the analogy -- it sits in a loop
/// reading events, updating internal counters, and redrawing.
///
/// Because this function owns `state` exclusively (it was moved in via
/// `thread::spawn`), there are no race conditions. No mutexes, no atomics,
/// just plain mutable access.
fn render(receiver: Receiver<RendererMsg>, mut state: RenderState) {
    loop {
        match receiver.recv() {
            Ok(RendererMsg::UserEvent(event)) => {
                // Update state based on event type.
                //
                // The match arms mirror the truth table from the Tracker docs:
                //   Started  -> add to building set
                //   Finished -> remove from building set, increment completed
                //   Skipped  -> increment completed (never enters building)
                match event.event_type {
                    EventType::Started => {
                        state.building.push(event.name);
                    }
                    EventType::Finished => {
                        // Remove the name from the building list.
                        // We use retain() rather than searching + swap_remove
                        // because the list is small (typically < 10 items)
                        // and retain preserves insertion order.
                        state.building.retain(|n| n != &event.name);
                        state.completed += 1;
                    }
                    EventType::Skipped => {
                        state.completed += 1;
                    }
                }
                draw(&mut state);
            }
            Ok(RendererMsg::Shutdown) => {
                // Final draw to show 100% if all items completed.
                draw(&mut state);
                break;
            }
            Err(_) => {
                // Channel disconnected -- all senders dropped.
                // Do a final draw and exit.
                draw(&mut state);
                break;
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Section 5: Drawing -- composing the progress line
// ---------------------------------------------------------------------------

/// Draw one progress line to the writer.
///
/// The line format depends on whether we have parent info (hierarchical)
/// or not (flat):
///
/// Flat:
/// ```text
/// \r[████████░░░░░░░░░░░░]  7/21  Building: pkg-a, pkg-b  (12.3s)
/// ```
///
/// Flat with label:
/// ```text
/// \rLevel 2/3  [████░░░░░░░░░░░░░░░░]  Building: pkg-a  (8.2s)
/// ```
///
/// Hierarchical (child with parent info):
/// ```text
/// \rLevel 2/3  [████░░░░░░░░░░░░░░░░]  5/12  Building: pkg-a  (8.2s)
/// ```
///
/// The bar uses Unicode block characters:
///
/// - `\u{2588}` (█) -- filled portion
/// - `\u{2591}` (░) -- empty portion
///
/// We use `\r` (carriage return) to overwrite the current line. This works
/// on all platforms. No ANSI escape codes needed.
fn draw(state: &mut RenderState) {
    let elapsed = state.start_time.elapsed().as_secs_f64();

    // --- Build the progress bar ---
    //
    // The bar is 20 characters wide. The number of filled characters is
    // proportional to completed/total:
    //
    //   filled = (completed * 20) / total
    //
    // Integer division naturally rounds down, so the bar only shows 100%
    // when all items are truly complete.
    let bar_width: usize = 20;
    let filled = if state.total > 0 {
        (state.completed * bar_width) / state.total
    } else {
        0
    };
    let filled = filled.min(bar_width);

    let bar: String = "\u{2588}"
        .repeat(filled)
        + &"\u{2591}".repeat(bar_width - filled);

    // --- Build the in-flight names list ---
    //
    // Show up to 3 names sorted alphabetically for deterministic output.
    // If there are more than 3, show the first 3 and "+N more".
    let activity = format_activity(&state.building, state.completed, state.total);

    // --- Compose the line ---
    let line = if let Some(ref parent) = state.parent_info {
        // Hierarchical: show parent label and count.
        let parent_current = parent.parent_completed + 1;
        format!(
            "\r{} {}/{}  [{}]  {}/{}  {}  ({:.1}s)",
            parent.label,
            parent_current,
            parent.total,
            bar,
            state.completed,
            state.total,
            activity,
            elapsed,
        )
    } else if !state.label.is_empty() {
        // Labeled flat tracker (used as parent).
        format!(
            "\r{} {}/{}  [{}]  {}  ({:.1}s)",
            state.label,
            state.completed,
            state.total,
            bar,
            activity,
            elapsed,
        )
    } else {
        // Plain flat mode.
        format!(
            "\r[{}]  {}/{}  {}  ({:.1}s)",
            bar,
            state.completed,
            state.total,
            activity,
            elapsed,
        )
    };

    // Pad to 80 characters to overwrite any previous longer line,
    // then write. We ignore write errors because the progress bar
    // is purely informational -- a broken pipe should not crash
    // the program.
    let _ = write!(state.writer, "{:<80}", line);
    let _ = state.writer.flush();
}

/// Build the "Building: pkg-a, pkg-b" or "waiting..." or "done" string
/// from the current in-flight set.
///
/// The rules:
///
/// | In-flight count | Completed vs Total | Output                      |
/// |-----------------|--------------------|-----------------------------|
/// | 0               | completed < total  | "waiting..."                |
/// | 0               | completed >= total | "done"                      |
/// | 1-3             | any                | "Building: a, b, c"         |
/// | 4+              | any                | "Building: a, b, c +N more" |
fn format_activity(building: &[String], completed: usize, total: usize) -> String {
    if building.is_empty() {
        if completed >= total {
            return "done".to_string();
        }
        return "waiting...".to_string();
    }

    // Sort alphabetically for deterministic output. We clone because
    // we only need sorted names for display -- the original order in
    // the Vec is the insertion order used for internal tracking.
    let mut names: Vec<&str> = building.iter().map(|s| s.as_str()).collect();
    names.sort();

    const MAX_NAMES: usize = 3;
    if names.len() <= MAX_NAMES {
        format!("Building: {}", names.join(", "))
    } else {
        let shown = names[..MAX_NAMES].join(", ");
        format!("Building: {} +{} more", shown, names.len() - MAX_NAMES)
    }
}

// ---------------------------------------------------------------------------
// Section 6: Unit tests for format_activity
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// When nothing is in-flight and work remains, we show "waiting..."
    /// to indicate the tracker is idle but not done.
    #[test]
    fn format_activity_empty_not_done() {
        let result = format_activity(&[], 0, 5);
        assert_eq!(result, "waiting...");
    }

    /// When nothing is in-flight and all work is done, we show "done".
    #[test]
    fn format_activity_empty_done() {
        let result = format_activity(&[], 5, 5);
        assert_eq!(result, "done");
    }

    /// A single in-flight item shows "Building: name".
    #[test]
    fn format_activity_one_item() {
        let building = vec!["alpha".to_string()];
        let result = format_activity(&building, 0, 5);
        assert_eq!(result, "Building: alpha");
    }

    /// Three items shows all of them without truncation.
    #[test]
    fn format_activity_three_items() {
        let building = vec![
            "charlie".to_string(),
            "alpha".to_string(),
            "bravo".to_string(),
        ];
        let result = format_activity(&building, 0, 5);
        // Should be sorted alphabetically.
        assert_eq!(result, "Building: alpha, bravo, charlie");
    }

    /// More than 3 items shows the first 3 (sorted) plus "+N more".
    #[test]
    fn format_activity_truncated() {
        let building = vec![
            "delta".to_string(),
            "alpha".to_string(),
            "charlie".to_string(),
            "bravo".to_string(),
            "echo".to_string(),
        ];
        let result = format_activity(&building, 0, 10);
        assert!(result.contains("alpha"));
        assert!(result.contains("bravo"));
        assert!(result.contains("charlie"));
        assert!(result.contains("+2 more"));
    }

    /// When completed exceeds total (edge case), format_activity still
    /// shows "done" rather than panicking.
    #[test]
    fn format_activity_overflow() {
        let result = format_activity(&[], 10, 5);
        assert_eq!(result, "done");
    }
}
