package progress

import (
	"bytes"
	"strings"
	"sync"
	"testing"
	"time"
)

// ---------------------------------------------------------------------------
// Helper: collect output from a tracker after sending events
// ---------------------------------------------------------------------------

// runTracker creates a Tracker backed by a buffer, sends the given events,
// then stops it and returns everything written to the buffer.
func runTracker(t *testing.T, total int, label string, events []Event) string {
	t.Helper()
	var buf bytes.Buffer
	tracker := New(total, &buf, label)
	tracker.Start()
	for _, e := range events {
		tracker.Send(e)
	}
	// Small sleep to let renderer process events before stopping.
	time.Sleep(20 * time.Millisecond)
	tracker.Stop()
	return buf.String()
}

// ---------------------------------------------------------------------------
// Tests for event counting and basic rendering
// ---------------------------------------------------------------------------

// TestEmptyTracker verifies that a tracker with zero events renders
// a zeroed-out bar with "waiting..." status.
func TestEmptyTracker(t *testing.T) {
	out := runTracker(t, 5, "", nil)
	if !strings.Contains(out, "0/5") {
		t.Errorf("expected 0/5 counter, got: %s", out)
	}
	if !strings.Contains(out, "waiting...") {
		t.Errorf("expected 'waiting...' for idle state, got: %s", out)
	}
}

// TestStartedEvent verifies that a Started event adds the item name
// to the "Building:" display without incrementing the completed counter.
func TestStartedEvent(t *testing.T) {
	events := []Event{
		{Type: Started, Name: "pkg-a"},
	}
	out := runTracker(t, 5, "", events)
	if !strings.Contains(out, "0/5") {
		t.Errorf("expected 0/5 (Started doesn't complete), got: %s", out)
	}
	if !strings.Contains(out, "pkg-a") {
		t.Errorf("expected 'pkg-a' in building list, got: %s", out)
	}
}

// TestFinishedEvent verifies that a Finished event increments the
// completed counter and removes the item from the building set.
func TestFinishedEvent(t *testing.T) {
	events := []Event{
		{Type: Started, Name: "pkg-a"},
		{Type: Finished, Name: "pkg-a", Status: "built"},
	}
	out := runTracker(t, 1, "", events)
	if !strings.Contains(out, "1/1") {
		t.Errorf("expected 1/1, got: %s", out)
	}
	if !strings.Contains(out, "done") {
		t.Errorf("expected 'done' when all items complete, got: %s", out)
	}
}

// TestSkippedEvent verifies that a Skipped event increments the
// completed counter without going through the building state.
func TestSkippedEvent(t *testing.T) {
	events := []Event{
		{Type: Skipped, Name: "pkg-b"},
	}
	out := runTracker(t, 3, "", events)
	if !strings.Contains(out, "1/3") {
		t.Errorf("expected 1/3, got: %s", out)
	}
}

// TestMixedEvents verifies a realistic sequence: some started+finished,
// some skipped.
func TestMixedEvents(t *testing.T) {
	events := []Event{
		{Type: Skipped, Name: "pkg-a"},
		{Type: Skipped, Name: "pkg-b"},
		{Type: Started, Name: "pkg-c"},
		{Type: Finished, Name: "pkg-c", Status: "built"},
	}
	out := runTracker(t, 3, "", events)
	if !strings.Contains(out, "3/3") {
		t.Errorf("expected 3/3, got: %s", out)
	}
	if !strings.Contains(out, "done") {
		t.Errorf("expected 'done', got: %s", out)
	}
}

// ---------------------------------------------------------------------------
// Tests for bar rendering
// ---------------------------------------------------------------------------

// TestBarCharacters verifies the bar contains the expected Unicode
// block characters.
func TestBarCharacters(t *testing.T) {
	events := []Event{
		{Type: Skipped, Name: "a"},
		{Type: Skipped, Name: "b"},
	}
	out := runTracker(t, 4, "", events)
	// 2/4 = 50% → 10 filled, 10 empty
	if !strings.Contains(out, "\u2588") {
		t.Error("expected filled block character █")
	}
	if !strings.Contains(out, "\u2591") {
		t.Error("expected empty block character ░")
	}
}

// TestBarFullyFilled verifies the bar is 100% filled when all items
// are complete.
func TestBarFullyFilled(t *testing.T) {
	events := []Event{
		{Type: Skipped, Name: "a"},
	}
	out := runTracker(t, 1, "", events)
	fullBar := strings.Repeat("\u2588", 20)
	if !strings.Contains(out, fullBar) {
		t.Errorf("expected fully filled bar, got: %s", out)
	}
}

// TestBarEmpty verifies the bar is 0% filled when no items are complete.
func TestBarEmpty(t *testing.T) {
	out := runTracker(t, 5, "", nil)
	emptyBar := strings.Repeat("\u2591", 20)
	if !strings.Contains(out, emptyBar) {
		t.Errorf("expected empty bar, got: %s", out)
	}
}

// ---------------------------------------------------------------------------
// Tests for name truncation
// ---------------------------------------------------------------------------

// TestNameTruncation verifies that when more than 3 items are in-flight,
// only the first 3 (alphabetically) are shown with a "+N more" suffix.
func TestNameTruncation(t *testing.T) {
	events := []Event{
		{Type: Started, Name: "delta"},
		{Type: Started, Name: "alpha"},
		{Type: Started, Name: "charlie"},
		{Type: Started, Name: "bravo"},
		{Type: Started, Name: "echo"},
	}
	out := runTracker(t, 10, "", events)
	// Should show first 3 alphabetically: alpha, bravo, charlie
	if !strings.Contains(out, "alpha") {
		t.Error("expected 'alpha' in output")
	}
	if !strings.Contains(out, "bravo") {
		t.Error("expected 'bravo' in output")
	}
	if !strings.Contains(out, "charlie") {
		t.Error("expected 'charlie' in output")
	}
	if !strings.Contains(out, "+2 more") {
		t.Errorf("expected '+2 more' for 5 items with max 3, got: %s", out)
	}
}

// TestThreeNamesNoTruncation verifies that exactly 3 in-flight items
// are shown without the "+N more" suffix.
func TestThreeNamesNoTruncation(t *testing.T) {
	events := []Event{
		{Type: Started, Name: "a"},
		{Type: Started, Name: "b"},
		{Type: Started, Name: "c"},
	}
	out := runTracker(t, 10, "", events)
	if strings.Contains(out, "more") {
		t.Errorf("3 items should not show '+N more', got: %s", out)
	}
}

// ---------------------------------------------------------------------------
// Tests for elapsed time
// ---------------------------------------------------------------------------

// TestElapsedTimeFormat verifies that the elapsed time appears in the
// output in the expected format (parenthesized, with 's' suffix).
func TestElapsedTimeFormat(t *testing.T) {
	out := runTracker(t, 1, "", nil)
	// Should contain something like "(0.0s)" — we just check the format
	if !strings.Contains(out, "s)") {
		t.Errorf("expected elapsed time with 's)' suffix, got: %s", out)
	}
}

// ---------------------------------------------------------------------------
// Tests for labeled (flat) mode
// ---------------------------------------------------------------------------

// TestLabeledTracker verifies that a label prefix appears in the output.
func TestLabeledTracker(t *testing.T) {
	events := []Event{
		{Type: Skipped, Name: "a"},
	}
	out := runTracker(t, 3, "Level", events)
	if !strings.Contains(out, "Level") {
		t.Errorf("expected 'Level' label in output, got: %s", out)
	}
	if !strings.Contains(out, "1/3") {
		t.Errorf("expected 1/3 counter, got: %s", out)
	}
}

// ---------------------------------------------------------------------------
// Tests for hierarchical progress
// ---------------------------------------------------------------------------

// TestHierarchicalProgress verifies that a child tracker shows the
// parent's label and count alongside the child's progress.
func TestHierarchicalProgress(t *testing.T) {
	var buf bytes.Buffer
	parent := New(3, &buf, "Level")
	parent.Start()

	child := parent.Child(2, "Package")
	child.Send(Event{Type: Started, Name: "pkg-a"})
	child.Send(Event{Type: Finished, Name: "pkg-a", Status: "built"})
	child.Send(Event{Type: Skipped, Name: "pkg-b"})
	time.Sleep(20 * time.Millisecond)
	child.Finish()

	time.Sleep(20 * time.Millisecond)
	parent.Stop()

	out := buf.String()
	if !strings.Contains(out, "Level") {
		t.Errorf("expected parent label 'Level' in output, got: %s", out)
	}
	if !strings.Contains(out, "pkg-a") {
		t.Errorf("expected 'pkg-a' in output, got: %s", out)
	}
}

// TestHierarchicalParentAdvances verifies that calling Finish() on a
// child advances the parent's completed count.
func TestHierarchicalParentAdvances(t *testing.T) {
	var buf bytes.Buffer
	parent := New(2, &buf, "Level")
	parent.Start()

	child1 := parent.Child(1, "Pkg")
	child1.Send(Event{Type: Skipped, Name: "a"})
	time.Sleep(10 * time.Millisecond)
	child1.Finish()

	child2 := parent.Child(1, "Pkg")
	child2.Send(Event{Type: Skipped, Name: "b"})
	time.Sleep(10 * time.Millisecond)
	child2.Finish()

	time.Sleep(10 * time.Millisecond)
	parent.Stop()

	out := buf.String()
	if !strings.Contains(out, "2/2") {
		t.Errorf("expected parent to reach 2/2, got: %s", out)
	}
}

// ---------------------------------------------------------------------------
// Tests for concurrency safety
// ---------------------------------------------------------------------------

// TestConcurrentSends verifies that many goroutines can Send events
// simultaneously without races or panics.
//
// Run with: go test -race ./...
func TestConcurrentSends(t *testing.T) {
	var buf bytes.Buffer
	tracker := New(100, &buf, "")
	tracker.Start()

	var wg sync.WaitGroup
	for i := 0; i < 100; i++ {
		wg.Add(1)
		go func(n int) {
			defer wg.Done()
			name := strings.Repeat("x", n%10+1)
			tracker.Send(Event{Type: Started, Name: name})
			tracker.Send(Event{Type: Finished, Name: name, Status: "ok"})
		}(i)
	}

	wg.Wait()
	time.Sleep(20 * time.Millisecond)
	tracker.Stop()

	out := buf.String()
	if !strings.Contains(out, "100/100") {
		t.Errorf("expected 100/100 after concurrent sends, got: %s", out)
	}
}

// ---------------------------------------------------------------------------
// Tests for nil safety
// ---------------------------------------------------------------------------

// TestNilTrackerSend verifies that calling Send on a nil Tracker does
// not panic. This enables callers to unconditionally call Send without
// nil-checking.
func TestNilTrackerSend(t *testing.T) {
	var tracker *Tracker
	// These should all be no-ops, not panics.
	tracker.Start()
	tracker.Send(Event{Type: Started, Name: "test"})
	tracker.Stop()
}

// TestNilTrackerChild verifies that calling Child on a nil Tracker
// returns nil without panicking.
func TestNilTrackerChild(t *testing.T) {
	var tracker *Tracker
	child := tracker.Child(5, "test")
	if child != nil {
		t.Error("expected nil child from nil parent")
	}
}

// TestNilTrackerFinish verifies that calling Finish on a nil Tracker
// does not panic.
func TestNilTrackerFinish(t *testing.T) {
	var tracker *Tracker
	tracker.Finish() // should be a no-op
}

// ---------------------------------------------------------------------------
// Tests for formatActivity helper
// ---------------------------------------------------------------------------

func TestFormatActivityEmpty(t *testing.T) {
	result := formatActivity(map[string]bool{}, 0, 5)
	if result != "waiting..." {
		t.Errorf("expected 'waiting...', got: %s", result)
	}
}

func TestFormatActivityDone(t *testing.T) {
	result := formatActivity(map[string]bool{}, 5, 5)
	if result != "done" {
		t.Errorf("expected 'done', got: %s", result)
	}
}

func TestFormatActivityOneItem(t *testing.T) {
	building := map[string]bool{"alpha": true}
	result := formatActivity(building, 0, 5)
	if result != "Building: alpha" {
		t.Errorf("expected 'Building: alpha', got: %s", result)
	}
}

func TestFormatActivityTruncated(t *testing.T) {
	building := map[string]bool{
		"alpha": true, "bravo": true, "charlie": true,
		"delta": true, "echo": true,
	}
	result := formatActivity(building, 0, 10)
	if !strings.Contains(result, "+2 more") {
		t.Errorf("expected '+2 more', got: %s", result)
	}
	if !strings.HasPrefix(result, "Building: alpha") {
		t.Errorf("expected to start with 'Building: alpha', got: %s", result)
	}
}
