// Package progress provides a reusable, text-based progress bar for tracking
// concurrent operations in the terminal.
//
// # The postal worker analogy
//
// Imagine a post office with a single clerk (the renderer) and a mail slot
// (the channel). Workers from all over town (goroutines) drop letters (events)
// into the slot. The clerk picks them up one at a time and updates the
// scoreboard on the wall (the progress bar). Because only the clerk touches
// the scoreboard, there's no confusion or conflict — even if a hundred
// workers drop letters at the same time.
//
// This is Go's channel pattern: many writers, one reader, no locks needed.
//
// # Usage
//
// Flat (simple) mode:
//
//	t := progress.New(21, os.Stderr, "")
//	t.Start()
//	t.Send(progress.Event{Type: progress.Started, Name: "pkg-a"})
//	t.Send(progress.Event{Type: progress.Finished, Name: "pkg-a", Status: "built"})
//	t.Send(progress.Event{Type: progress.Skipped, Name: "pkg-b"})
//	t.Stop()
//
// Hierarchical mode (e.g., build levels):
//
//	parent := progress.New(3, os.Stderr, "Level")
//	parent.Start()
//	child := parent.Child(7, "Package")
//	child.Send(progress.Event{Type: progress.Started, Name: "pkg-a"})
//	child.Send(progress.Event{Type: progress.Finished, Name: "pkg-a", Status: "built"})
//	child.Finish()   // advances parent by 1
//	parent.Stop()
package progress

import (
	"fmt"
	"io"
	"sort"
	"strings"
	"time"
)

// ---------------------------------------------------------------------------
// Event types — what can happen to a tracked item
// ---------------------------------------------------------------------------

// EventType distinguishes the three things that can happen to an item.
//
// Think of it like a traffic light:
//
//	Started  = green  (item is actively being processed)
//	Finished = red    (item is done — success or failure)
//	Skipped  = yellow (item was bypassed without processing)
type EventType int

const (
	Started  EventType = iota // An item began processing (now "in-flight").
	Finished                  // An item completed (success or failure).
	Skipped                   // An item was skipped without processing.
)

// Event is the message that workers send to the tracker.
//
// It's deliberately minimal — just three fields:
//
//	Type   — what happened (Started, Finished, Skipped)
//	Name   — human-readable identifier (e.g., "python/logic-gates")
//	Status — outcome label, only meaningful for Finished events
//	         (e.g., "built", "failed", "cached")
type Event struct {
	Type   EventType
	Name   string
	Status string
}

// ---------------------------------------------------------------------------
// Tracker — the progress bar engine
// ---------------------------------------------------------------------------

// Tracker receives events from concurrent workers and renders a text-based
// progress bar. It is safe to call Send from any goroutine.
//
// Internally, the Tracker uses a single goroutine (the "renderer") that reads
// from a buffered channel. All state mutation happens inside this goroutine,
// so no mutexes are needed. This is the key design decision — channels give
// us thread safety for free.
//
// # State tracking
//
// The renderer maintains:
//
//	completed — count of items that are Finished or Skipped
//	building  — set of item names currently in-flight (Started but not Finished)
//	total     — the target count (set at creation time)
//
// Truth table for state transitions:
//
//	Event     | completed | building
//	----------+-----------+---------
//	Started   | unchanged | add name
//	Finished  | +1        | remove name
//	Skipped   | +1        | unchanged
type Tracker struct {
	total     int
	completed int
	building  map[string]bool
	events    chan Event
	done      chan struct{}
	writer    io.Writer
	startTime time.Time
	label     string

	// Parent link for hierarchical progress.
	parent *Tracker
}

// New creates a Tracker that expects `total` items and writes to `w`.
//
// The optional `label` parameter adds a prefix to the display line
// (e.g., "Level" produces "Level 2/3 [████░░░░] ..."). Pass "" for
// no label (flat mode).
//
// The channel is buffered to 64 events — enough to absorb bursts from
// many concurrent goroutines without blocking. The buffer size is a
// pragmatic choice: large enough to prevent backpressure, small enough
// to keep memory negligible.
func New(total int, w io.Writer, label string) *Tracker {
	return &Tracker{
		total:    total,
		building: make(map[string]bool),
		events:   make(chan Event, 64),
		done:     make(chan struct{}),
		writer:   w,
		label:    label,
	}
}

// Start launches the background renderer goroutine. Call this once before
// sending any events.
//
// The renderer goroutine is the "postal clerk" — it sits in a loop reading
// events from the channel, updating internal counters, and redrawing the
// progress bar after each event.
func (t *Tracker) Start() {
	if t == nil {
		return
	}
	t.startTime = time.Now()
	go t.render()
}

// Send submits an event to the tracker. This is safe to call from any
// goroutine — it just writes to the buffered channel.
//
// If the tracker is nil, Send is a no-op. This is a deliberate design
// choice: callers can unconditionally call Send without nil-checking,
// which keeps integration code clean.
//
//	// The caller doesn't need to know if progress tracking is enabled:
//	tracker.Send(progress.Event{Type: progress.Started, Name: "pkg-a"})
func (t *Tracker) Send(e Event) {
	if t == nil {
		return
	}
	t.events <- e
}

// Child creates a nested sub-tracker for hierarchical progress.
//
// The child shares the parent's writer and start time. When the child
// calls Finish(), it advances the parent's completed count by 1.
//
// Example: a build system has 3 dependency levels, each with N packages.
// The parent tracks levels (total=3, label="Level"), and each child
// tracks packages within that level (total=N, label="Package").
//
//	parent := progress.New(3, os.Stderr, "Level")
//	child := parent.Child(7, "Package")
//	// Display: Level 1/3  [████░░░░]  3/7  Building: pkg-a  (2.1s)
func (t *Tracker) Child(total int, label string) *Tracker {
	if t == nil {
		return nil
	}
	child := &Tracker{
		total:     total,
		building:  make(map[string]bool),
		events:    make(chan Event, 64),
		done:      make(chan struct{}),
		writer:    t.writer,
		startTime: t.startTime,
		label:     label,
		parent:    t,
	}
	go child.render()
	return child
}

// Finish marks this child tracker as complete and advances the parent
// tracker by one. Call this when all items in the child are done.
//
// This closes the child's event channel, waits for its renderer to
// finish, then sends a Finished event to the parent.
func (t *Tracker) Finish() {
	if t == nil {
		return
	}
	close(t.events)
	<-t.done
	if t.parent != nil {
		t.parent.Send(Event{Type: Finished, Name: t.label})
	}
}

// Stop shuts down the tracker. It closes the event channel, waits for
// the renderer goroutine to drain and exit, then prints a final newline
// so the last progress line is preserved in the terminal scrollback.
func (t *Tracker) Stop() {
	if t == nil {
		return
	}
	close(t.events)
	<-t.done
	fmt.Fprintln(t.writer)
}

// ---------------------------------------------------------------------------
// Internal: the renderer goroutine
// ---------------------------------------------------------------------------

// render is the background goroutine that processes events and redraws
// the progress bar. It runs until the events channel is closed.
//
// The loop is simple: read event → update state → redraw. Because this
// is the only goroutine that reads or writes tracker state (completed,
// building), there are no race conditions.
func (t *Tracker) render() {
	defer close(t.done)

	for event := range t.events {
		switch event.Type {
		case Started:
			t.building[event.Name] = true
		case Finished:
			delete(t.building, event.Name)
			t.completed++
		case Skipped:
			t.completed++
		}
		t.draw()
	}

	// Final draw after channel closes — ensures the bar shows 100%.
	t.draw()
}

// draw composes and writes one progress line to the writer.
//
// The line format depends on whether we have a parent (hierarchical)
// or not (flat):
//
// Flat:
//
//	[████████░░░░░░░░░░░░]  7/21  Building: pkg-a, pkg-b  (12.3s)
//
// Hierarchical:
//
//	Level 2/3  [████░░░░░░░░░░░░░░░░]  5/12  Building: pkg-a  (8.2s)
//
// The bar uses Unicode block characters:
//
//	█ (U+2588) — filled portion
//	░ (U+2591) — empty portion
//
// We use \r (carriage return) to overwrite the current line. This works
// on all platforms — Windows cmd, PowerShell, Git Bash, and Unix terminals.
// No ANSI escape codes needed.
func (t *Tracker) draw() {
	elapsed := time.Since(t.startTime).Seconds()

	// --- Build the progress bar ---
	//
	// The bar is 20 characters wide. The number of filled characters is
	// proportional to completed/total:
	//
	//	filled = (completed × 20) / total
	//
	// Integer division naturally rounds down, so the bar only shows 100%
	// when all items are truly complete.
	barWidth := 20
	filled := 0
	if t.total > 0 {
		filled = (t.completed * barWidth) / t.total
	}
	if filled > barWidth {
		filled = barWidth
	}
	bar := strings.Repeat("\u2588", filled) + strings.Repeat("\u2591", barWidth-filled)

	// --- Build the in-flight names list ---
	//
	// Show up to 3 names sorted alphabetically for deterministic output.
	// If there are more than 3, show the first 3 and "+N more".
	// If nothing is in-flight: "waiting..." (before all done) or "done".
	activity := formatActivity(t.building, t.completed, t.total)

	// --- Compose the line ---
	var line string
	if t.parent != nil {
		// Hierarchical: show parent label and count.
		parentCompleted := t.parent.completed + 1 // +1 because this child is "current"
		line = fmt.Sprintf("\r%s %d/%d  [%s]  %d/%d  %s  (%.1fs)",
			t.parent.label, parentCompleted, t.parent.total,
			bar, t.completed, t.total, activity, elapsed)
	} else if t.label != "" {
		// Labeled flat tracker (used as parent — shows own state).
		line = fmt.Sprintf("\r%s %d/%d  [%s]  %s  (%.1fs)",
			t.label, t.completed, t.total, bar, activity, elapsed)
	} else {
		// Flat mode: just the bar.
		line = fmt.Sprintf("\r[%s]  %d/%d  %s  (%.1fs)",
			bar, t.completed, t.total, activity, elapsed)
	}

	// Pad to 80 characters to overwrite any previous longer line.
	fmt.Fprintf(t.writer, "%-80s", line)
}

// formatActivity builds the "Building: pkg-a, pkg-b" or "waiting..."
// or "done" string from the current in-flight set.
//
// The rules:
//
//	| In-flight count | Completed vs Total | Output                       |
//	|-----------------|--------------------|------------------------------|
//	| 0               | completed < total  | "waiting..."                 |
//	| 0               | completed == total | "done"                       |
//	| 1-3             | any                | "Building: a, b, c"          |
//	| 4+              | any                | "Building: a, b, c +N more"  |
func formatActivity(building map[string]bool, completed, total int) string {
	if len(building) == 0 {
		if completed >= total {
			return "done"
		}
		return "waiting..."
	}

	names := make([]string, 0, len(building))
	for name := range building {
		names = append(names, name)
	}
	sort.Strings(names)

	const maxNames = 3
	if len(names) <= maxNames {
		return "Building: " + strings.Join(names, ", ")
	}
	shown := strings.Join(names[:maxNames], ", ")
	return fmt.Sprintf("Building: %s +%d more", shown, len(names)-maxNames)
}
