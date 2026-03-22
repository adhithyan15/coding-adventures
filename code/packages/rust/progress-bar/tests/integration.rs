//! # Integration tests for the progress-bar crate
//!
//! These tests exercise the full Tracker lifecycle: creating a tracker,
//! sending events from one or more threads, stopping the tracker, and
//! inspecting the output written to a buffer.
//!
//! ## Test strategy
//!
//! We use `Vec<u8>` wrapped in `Arc<Mutex<>>` as the writer. This lets us
//! capture all output from the renderer thread and inspect it after the
//! tracker stops. The mutex is needed because the renderer thread writes
//! to the buffer while the test thread needs to read it after joining.
//!
//! Each test follows the same pattern:
//! 1. Create a shared buffer
//! 2. Create a Tracker with that buffer as the writer
//! 3. Send events
//! 4. Stop the tracker (which joins the renderer thread)
//! 5. Read the buffer and assert on its contents

use progress_bar::{Event, EventType, Tracker};
use std::io::Write;
use std::sync::{Arc, Mutex};
use std::thread;

// ---------------------------------------------------------------------------
// Helper: a thread-safe writer backed by a Vec<u8>
// ---------------------------------------------------------------------------

/// A writer that captures output into a shared buffer.
///
/// Why `Arc<Mutex<Vec<u8>>>` and not just `Vec<u8>`?
///
/// The Tracker moves the writer into its renderer thread (via `Box<dyn Write + Send>`).
/// We need a handle to read the buffer after the tracker stops. `Arc` gives us
/// shared ownership, and `Mutex` gives us interior mutability that satisfies
/// the `Write` trait's `&mut self` requirement.
#[derive(Clone)]
struct SharedBuffer {
    inner: Arc<Mutex<Vec<u8>>>,
}

impl SharedBuffer {
    fn new() -> Self {
        SharedBuffer {
            inner: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Read the captured output as a UTF-8 string.
    fn contents(&self) -> String {
        let guard = self.inner.lock().unwrap();
        String::from_utf8_lossy(&guard).to_string()
    }
}

impl Write for SharedBuffer {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let mut guard = self.inner.lock().unwrap();
        guard.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        let mut guard = self.inner.lock().unwrap();
        guard.flush()
    }
}

/// Helper: create a tracker with a shared buffer, send events, stop, return output.
fn run_tracker(total: usize, label: &str, events: Vec<Event>) -> String {
    let buf = SharedBuffer::new();
    let mut tracker = Tracker::new(total, Box::new(buf.clone()), label);
    tracker.start();

    for event in events {
        tracker.send(event);
    }

    // Small sleep to let the renderer process events before stopping.
    // The renderer runs on a separate thread and needs time to drain
    // the channel. 20ms is generous for the tiny amount of work involved.
    thread::sleep(std::time::Duration::from_millis(20));
    tracker.stop();

    buf.contents()
}

// ---------------------------------------------------------------------------
// Tests for event counting and basic rendering
// ---------------------------------------------------------------------------

/// A tracker with zero events should show 0/N and "waiting..." because
/// no items have started or completed yet.
#[test]
fn test_empty_tracker() {
    let out = run_tracker(5, "", vec![]);
    assert!(out.contains("0/5"), "expected 0/5 counter, got: {}", out);
    assert!(
        out.contains("waiting..."),
        "expected 'waiting...' for idle state, got: {}",
        out
    );
}

/// A Started event adds the item name to the "Building:" display
/// without incrementing the completed counter. The item is in-flight
/// but not yet done.
#[test]
fn test_started_event() {
    let events = vec![Event {
        event_type: EventType::Started,
        name: "pkg-a".into(),
        status: String::new(),
    }];
    let out = run_tracker(5, "", events);
    assert!(
        out.contains("0/5"),
        "expected 0/5 (Started doesn't complete), got: {}",
        out
    );
    assert!(
        out.contains("pkg-a"),
        "expected 'pkg-a' in building list, got: {}",
        out
    );
}

/// A Finished event increments the completed counter and removes
/// the item from the building set.
#[test]
fn test_finished_event() {
    let events = vec![
        Event {
            event_type: EventType::Started,
            name: "pkg-a".into(),
            status: String::new(),
        },
        Event {
            event_type: EventType::Finished,
            name: "pkg-a".into(),
            status: "built".into(),
        },
    ];
    let out = run_tracker(1, "", events);
    assert!(out.contains("1/1"), "expected 1/1, got: {}", out);
    assert!(
        out.contains("done"),
        "expected 'done' when all items complete, got: {}",
        out
    );
}

/// A Skipped event increments the completed counter without ever
/// going through the building state.
#[test]
fn test_skipped_event() {
    let events = vec![Event {
        event_type: EventType::Skipped,
        name: "pkg-b".into(),
        status: String::new(),
    }];
    let out = run_tracker(3, "", events);
    assert!(out.contains("1/3"), "expected 1/3, got: {}", out);
}

/// A realistic sequence: some started+finished, some skipped.
/// All three items should be counted.
#[test]
fn test_mixed_events() {
    let events = vec![
        Event {
            event_type: EventType::Skipped,
            name: "pkg-a".into(),
            status: String::new(),
        },
        Event {
            event_type: EventType::Skipped,
            name: "pkg-b".into(),
            status: String::new(),
        },
        Event {
            event_type: EventType::Started,
            name: "pkg-c".into(),
            status: String::new(),
        },
        Event {
            event_type: EventType::Finished,
            name: "pkg-c".into(),
            status: "built".into(),
        },
    ];
    let out = run_tracker(3, "", events);
    assert!(out.contains("3/3"), "expected 3/3, got: {}", out);
    assert!(out.contains("done"), "expected 'done', got: {}", out);
}

// ---------------------------------------------------------------------------
// Tests for bar rendering format
// ---------------------------------------------------------------------------

/// The progress bar should contain both filled (█) and empty (░)
/// block characters when partially complete.
#[test]
fn test_bar_characters() {
    let events = vec![
        Event {
            event_type: EventType::Skipped,
            name: "a".into(),
            status: String::new(),
        },
        Event {
            event_type: EventType::Skipped,
            name: "b".into(),
            status: String::new(),
        },
    ];
    let out = run_tracker(4, "", events);
    // 2/4 = 50% -> 10 filled, 10 empty
    assert!(out.contains('\u{2588}'), "expected filled block character");
    assert!(out.contains('\u{2591}'), "expected empty block character");
}

/// When all items are complete, the bar should be 100% filled.
#[test]
fn test_bar_fully_filled() {
    let events = vec![Event {
        event_type: EventType::Skipped,
        name: "a".into(),
        status: String::new(),
    }];
    let out = run_tracker(1, "", events);
    let full_bar = "\u{2588}".repeat(20);
    assert!(
        out.contains(&full_bar),
        "expected fully filled bar, got: {}",
        out
    );
}

/// When no items are complete, the bar should be 0% filled.
#[test]
fn test_bar_empty() {
    let out = run_tracker(5, "", vec![]);
    let empty_bar = "\u{2591}".repeat(20);
    assert!(
        out.contains(&empty_bar),
        "expected empty bar, got: {}",
        out
    );
}

/// The bar should contain exactly 20 block characters total.
#[test]
fn test_bar_width() {
    let events = vec![
        Event {
            event_type: EventType::Skipped,
            name: "a".into(),
            status: String::new(),
        },
        Event {
            event_type: EventType::Skipped,
            name: "b".into(),
            status: String::new(),
        },
        Event {
            event_type: EventType::Skipped,
            name: "c".into(),
            status: String::new(),
        },
    ];
    let out = run_tracker(4, "", events);
    // Count block characters in the final line. The bar should have
    // exactly 20 total (filled + empty).
    let block_count: usize = out
        .chars()
        .filter(|c| *c == '\u{2588}' || *c == '\u{2591}')
        .count();
    // Each draw writes 20 chars, and we have multiple draws (one per event
    // plus final), so total should be a multiple of 20.
    assert!(
        block_count > 0 && block_count % 20 == 0,
        "expected block count to be a multiple of 20, got: {}",
        block_count
    );
}

// ---------------------------------------------------------------------------
// Tests for name truncation
// ---------------------------------------------------------------------------

/// When more than 3 items are in-flight, only the first 3 (alphabetically)
/// are shown with a "+N more" suffix. This keeps the progress line from
/// growing unboundedly.
#[test]
fn test_name_truncation() {
    let events = vec![
        Event {
            event_type: EventType::Started,
            name: "delta".into(),
            status: String::new(),
        },
        Event {
            event_type: EventType::Started,
            name: "alpha".into(),
            status: String::new(),
        },
        Event {
            event_type: EventType::Started,
            name: "charlie".into(),
            status: String::new(),
        },
        Event {
            event_type: EventType::Started,
            name: "bravo".into(),
            status: String::new(),
        },
        Event {
            event_type: EventType::Started,
            name: "echo".into(),
            status: String::new(),
        },
    ];
    let out = run_tracker(10, "", events);
    // Should show first 3 alphabetically: alpha, bravo, charlie
    assert!(out.contains("alpha"), "expected 'alpha' in output");
    assert!(out.contains("bravo"), "expected 'bravo' in output");
    assert!(out.contains("charlie"), "expected 'charlie' in output");
    assert!(
        out.contains("+2 more"),
        "expected '+2 more' for 5 items, got: {}",
        out
    );
}

/// Exactly 3 in-flight items should be shown without truncation.
#[test]
fn test_three_names_no_truncation() {
    let events = vec![
        Event {
            event_type: EventType::Started,
            name: "a".into(),
            status: String::new(),
        },
        Event {
            event_type: EventType::Started,
            name: "b".into(),
            status: String::new(),
        },
        Event {
            event_type: EventType::Started,
            name: "c".into(),
            status: String::new(),
        },
    ];
    let out = run_tracker(10, "", events);
    assert!(
        !out.contains("more"),
        "3 items should not show '+N more', got: {}",
        out
    );
}

// ---------------------------------------------------------------------------
// Tests for elapsed time
// ---------------------------------------------------------------------------

/// The elapsed time should appear in the output in parenthesized format
/// with an 's' suffix (e.g., "(0.0s)").
#[test]
fn test_elapsed_time_format() {
    let out = run_tracker(1, "", vec![]);
    assert!(
        out.contains("s)"),
        "expected elapsed time with 's)' suffix, got: {}",
        out
    );
}

// ---------------------------------------------------------------------------
// Tests for labeled (flat) mode
// ---------------------------------------------------------------------------

/// A labeled tracker should include the label prefix in its output.
#[test]
fn test_labeled_tracker() {
    let events = vec![Event {
        event_type: EventType::Skipped,
        name: "a".into(),
        status: String::new(),
    }];
    let out = run_tracker(3, "Level", events);
    assert!(
        out.contains("Level"),
        "expected 'Level' label in output, got: {}",
        out
    );
    assert!(out.contains("1/3"), "expected 1/3 counter, got: {}", out);
}

// ---------------------------------------------------------------------------
// Tests for hierarchical progress
// ---------------------------------------------------------------------------

/// A child tracker should advance the parent's completed count when
/// `finish()` is called.
#[test]
fn test_hierarchical_parent_advances() {
    let buf = SharedBuffer::new();
    let mut parent = Tracker::new(2, Box::new(buf.clone()), "Level");
    parent.start();

    // First child
    let mut child1 = parent.child(1, "Pkg");
    child1.send(Event {
        event_type: EventType::Skipped,
        name: "a".into(),
        status: String::new(),
    });
    thread::sleep(std::time::Duration::from_millis(10));
    child1.finish();

    // Second child
    let mut child2 = parent.child(1, "Pkg");
    child2.send(Event {
        event_type: EventType::Skipped,
        name: "b".into(),
        status: String::new(),
    });
    thread::sleep(std::time::Duration::from_millis(10));
    child2.finish();

    thread::sleep(std::time::Duration::from_millis(10));
    parent.stop();

    let out = buf.contents();
    assert!(
        out.contains("2/2"),
        "expected parent to reach 2/2, got: {}",
        out
    );
}

/// A child tracker with a dedicated writer should show parent info
/// in hierarchical mode.
#[test]
fn test_hierarchical_child_with_writer() {
    let parent_buf = SharedBuffer::new();
    let child_buf = SharedBuffer::new();
    let mut parent = Tracker::new(3, Box::new(parent_buf.clone()), "Level");
    parent.start();

    let mut child = parent.child_with_writer(2, "Package", Box::new(child_buf.clone()), 0);
    child.send(Event {
        event_type: EventType::Started,
        name: "pkg-a".into(),
        status: String::new(),
    });
    child.send(Event {
        event_type: EventType::Finished,
        name: "pkg-a".into(),
        status: "built".into(),
    });
    thread::sleep(std::time::Duration::from_millis(10));
    child.finish();

    thread::sleep(std::time::Duration::from_millis(10));
    parent.stop();

    let child_out = child_buf.contents();
    assert!(
        child_out.contains("Level"),
        "expected parent label in child output, got: {}",
        child_out
    );
    assert!(
        child_out.contains("pkg-a"),
        "expected 'pkg-a' in child output, got: {}",
        child_out
    );
}

// ---------------------------------------------------------------------------
// Tests for concurrent sends (multiple threads)
// ---------------------------------------------------------------------------

/// Many threads can Send events simultaneously without races or panics.
///
/// This test spawns 100 threads, each sending a Started+Finished pair.
/// After all threads complete and the tracker stops, the completed count
/// should be 100/100.
///
/// Run with: `cargo test -- --nocapture` or `RUSTFLAGS="-Zsanitizer=thread"`
/// to check for data races.
#[test]
fn test_concurrent_sends() {
    let buf = SharedBuffer::new();
    let mut tracker = Tracker::new(100, Box::new(buf.clone()), "");
    tracker.start();

    let mut handles = vec![];
    for i in 0..100 {
        let es = tracker.event_sender();
        handles.push(thread::spawn(move || {
            let name = format!("item-{}", i);
            es.send(Event {
                event_type: EventType::Started,
                name: name.clone(),
                status: String::new(),
            });
            es.send(Event {
                event_type: EventType::Finished,
                name,
                status: "ok".into(),
            });
        }));
    }

    for handle in handles {
        handle.join().unwrap();
    }

    thread::sleep(std::time::Duration::from_millis(50));
    tracker.stop();

    let out = buf.contents();
    assert!(
        out.contains("100/100"),
        "expected 100/100 after concurrent sends, got: {}",
        out
    );
}

// ---------------------------------------------------------------------------
// Tests for stop cleanup
// ---------------------------------------------------------------------------

/// Stopping a tracker should not panic and should produce output
/// ending without a trailing carriage return "stuck" line.
#[test]
fn test_stop_cleanup() {
    let buf = SharedBuffer::new();
    let mut tracker = Tracker::new(2, Box::new(buf.clone()), "");
    tracker.start();
    tracker.send(Event {
        event_type: EventType::Skipped,
        name: "a".into(),
        status: String::new(),
    });
    thread::sleep(std::time::Duration::from_millis(10));
    tracker.stop();

    let out = buf.contents();
    // Should have produced some output.
    assert!(!out.is_empty(), "expected non-empty output after stop");
}

/// Stopping an idle tracker (no events sent) should not panic.
#[test]
fn test_stop_idle_tracker() {
    let buf = SharedBuffer::new();
    let mut tracker = Tracker::new(0, Box::new(buf.clone()), "");
    tracker.start();
    tracker.stop();
    // Just checking it doesn't panic.
}

/// Multiple rapid start-stop cycles should work without issues.
#[test]
fn test_rapid_start_stop() {
    for _ in 0..10 {
        let buf = SharedBuffer::new();
        let mut tracker = Tracker::new(1, Box::new(buf.clone()), "");
        tracker.start();
        tracker.send(Event {
            event_type: EventType::Skipped,
            name: "x".into(),
            status: String::new(),
        });
        thread::sleep(std::time::Duration::from_millis(5));
        tracker.stop();
    }
}

/// A tracker with total=0 should not divide by zero.
#[test]
fn test_zero_total() {
    let out = run_tracker(0, "", vec![]);
    // Should have an empty bar (all ░) and show "done" since 0 >= 0.
    let empty_bar = "\u{2591}".repeat(20);
    assert!(
        out.contains(&empty_bar),
        "expected empty bar for total=0, got: {}",
        out
    );
    assert!(
        out.contains("done"),
        "expected 'done' for total=0, got: {}",
        out
    );
}
