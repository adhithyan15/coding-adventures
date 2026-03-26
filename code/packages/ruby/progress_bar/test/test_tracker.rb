# frozen_string_literal: true

require_relative "test_helper"
require "stringio"

# ==========================================================================
# Tests for CodingAdventures::ProgressBar::Tracker
# ==========================================================================
#
# These tests use StringIO as the writer so we can capture and inspect
# the rendered output without touching a real terminal.
#
# Test strategy:
#   1. Event counting -- verify completed counter changes correctly
#   2. Bar rendering  -- verify Unicode characters appear
#   3. Name display   -- verify truncation and sorting
#   4. Hierarchical   -- verify parent/child interaction
#   5. Concurrency    -- verify thread safety
#   6. NullTracker    -- verify all methods are safe no-ops

class TestTracker < Minitest::Test
  include CodingAdventures::ProgressBar

  # Helper: create a tracker backed by a StringIO, send events, stop,
  # and return the captured output.
  def run_tracker(total, label, events, sleep_ms: 20)
    writer = StringIO.new
    tracker = Tracker.new(total, writer, label)
    tracker.start
    events.each { |e| tracker.send_event(e) }
    sleep(sleep_ms / 1000.0) # let renderer process
    tracker.stop
    writer.string
  end

  # -----------------------------------------------------------------------
  # Event counting tests
  # -----------------------------------------------------------------------

  def test_empty_tracker
    out = run_tracker(5, "", [])
    assert_includes out, "0/5", "expected 0/5 counter"
    assert_includes out, "waiting...", "expected 'waiting...' for idle state"
  end

  def test_started_event_does_not_increment_completed
    events = [Event.new(type: EventType::STARTED, name: "pkg-a")]
    out = run_tracker(5, "", events)
    assert_includes out, "0/5", "Started should not increment completed"
    assert_includes out, "pkg-a", "Started item should appear in building list"
  end

  def test_finished_event_increments_completed
    events = [
      Event.new(type: EventType::STARTED, name: "pkg-a"),
      Event.new(type: EventType::FINISHED, name: "pkg-a", status: "built")
    ]
    out = run_tracker(1, "", events)
    assert_includes out, "1/1", "Finished should increment completed"
    assert_includes out, "done", "All items done should show 'done'"
  end

  def test_skipped_event_increments_completed
    events = [Event.new(type: EventType::SKIPPED, name: "pkg-b")]
    out = run_tracker(3, "", events)
    assert_includes out, "1/3", "Skipped should increment completed"
  end

  def test_mixed_events
    events = [
      Event.new(type: EventType::SKIPPED, name: "pkg-a"),
      Event.new(type: EventType::SKIPPED, name: "pkg-b"),
      Event.new(type: EventType::STARTED, name: "pkg-c"),
      Event.new(type: EventType::FINISHED, name: "pkg-c", status: "built")
    ]
    out = run_tracker(3, "", events)
    assert_includes out, "3/3", "Mixed events should sum to 3/3"
    assert_includes out, "done", "All complete should show 'done'"
  end

  # -----------------------------------------------------------------------
  # Bar rendering tests
  # -----------------------------------------------------------------------

  def test_bar_contains_unicode_characters
    events = [
      Event.new(type: EventType::SKIPPED, name: "a"),
      Event.new(type: EventType::SKIPPED, name: "b")
    ]
    out = run_tracker(4, "", events)
    # 2/4 = 50% -> 10 filled, 10 empty
    assert_includes out, "\u2588", "expected filled block character"
    assert_includes out, "\u2591", "expected empty block character"
  end

  def test_bar_fully_filled
    events = [Event.new(type: EventType::SKIPPED, name: "a")]
    out = run_tracker(1, "", events)
    full_bar = "\u2588" * 20
    assert_includes out, full_bar, "1/1 should produce a fully filled bar"
  end

  def test_bar_empty
    out = run_tracker(5, "", [])
    empty_bar = "\u2591" * 20
    assert_includes out, empty_bar, "0/5 should produce an empty bar"
  end

  def test_bar_half_filled
    events = (1..5).map { |i| Event.new(type: EventType::SKIPPED, name: "p#{i}") }
    out = run_tracker(10, "", events)
    # 5/10 = 50% -> 10 filled, 10 empty
    half_bar = "\u2588" * 10 + "\u2591" * 10
    assert_includes out, half_bar, "5/10 should produce a half-filled bar"
  end

  # -----------------------------------------------------------------------
  # Name display and truncation tests
  # -----------------------------------------------------------------------

  def test_single_building_name
    events = [Event.new(type: EventType::STARTED, name: "alpha")]
    out = run_tracker(10, "", events)
    assert_includes out, "Building: alpha", "Single in-flight should show name"
  end

  def test_three_names_no_truncation
    events = [
      Event.new(type: EventType::STARTED, name: "a"),
      Event.new(type: EventType::STARTED, name: "b"),
      Event.new(type: EventType::STARTED, name: "c")
    ]
    out = run_tracker(10, "", events)
    refute_includes out, "more", "3 items should not show '+N more'"
    assert_includes out, "Building: a, b, c", "All three names should appear sorted"
  end

  def test_name_truncation_with_more
    events = [
      Event.new(type: EventType::STARTED, name: "delta"),
      Event.new(type: EventType::STARTED, name: "alpha"),
      Event.new(type: EventType::STARTED, name: "charlie"),
      Event.new(type: EventType::STARTED, name: "bravo"),
      Event.new(type: EventType::STARTED, name: "echo")
    ]
    out = run_tracker(10, "", events)
    assert_includes out, "alpha", "First alpha name should appear"
    assert_includes out, "bravo", "Second bravo name should appear"
    assert_includes out, "charlie", "Third charlie name should appear"
    assert_includes out, "+2 more", "5 items with max 3 should show '+2 more'"
  end

  def test_names_sorted_alphabetically
    events = [
      Event.new(type: EventType::STARTED, name: "zebra"),
      Event.new(type: EventType::STARTED, name: "apple")
    ]
    out = run_tracker(10, "", events)
    assert_includes out, "Building: apple, zebra",
      "Names should be sorted alphabetically"
  end

  # -----------------------------------------------------------------------
  # Elapsed time tests
  # -----------------------------------------------------------------------

  def test_elapsed_time_format
    out = run_tracker(1, "", [])
    assert_match(/\(\d+\.\ds\)/, out, "expected elapsed time in (N.Ns) format")
  end

  # -----------------------------------------------------------------------
  # Label tests
  # -----------------------------------------------------------------------

  def test_labeled_tracker
    events = [Event.new(type: EventType::SKIPPED, name: "a")]
    out = run_tracker(3, "Level", events)
    assert_includes out, "Level", "Label should appear in output"
    assert_includes out, "1/3", "Counter should appear"
  end

  # -----------------------------------------------------------------------
  # Hierarchical progress tests
  # -----------------------------------------------------------------------

  def test_hierarchical_child_shows_parent_label
    writer = StringIO.new
    parent = Tracker.new(3, writer, "Level")
    parent.start

    child = parent.child(2, "Package")
    child.send_event(Event.new(type: EventType::STARTED, name: "pkg-a"))
    child.send_event(Event.new(type: EventType::FINISHED, name: "pkg-a", status: "built"))
    child.send_event(Event.new(type: EventType::SKIPPED, name: "pkg-b"))
    sleep(0.02)
    child.finish

    sleep(0.02)
    parent.stop

    out = writer.string
    assert_includes out, "Level", "Parent label should appear in child output"
    assert_includes out, "pkg-a", "Child item should appear in output"
  end

  def test_hierarchical_parent_advances
    writer = StringIO.new
    parent = Tracker.new(2, writer, "Level")
    parent.start

    child1 = parent.child(1, "Pkg")
    child1.send_event(Event.new(type: EventType::SKIPPED, name: "a"))
    sleep(0.01)
    child1.finish

    child2 = parent.child(1, "Pkg")
    child2.send_event(Event.new(type: EventType::SKIPPED, name: "b"))
    sleep(0.01)
    child2.finish

    sleep(0.01)
    parent.stop

    out = writer.string
    assert_includes out, "2/2", "Parent should reach 2/2 after two child finishes"
  end

  def test_child_finish_sends_event_to_parent
    writer = StringIO.new
    parent = Tracker.new(1, writer, "Root")
    parent.start

    child = parent.child(1, "Sub")
    child.send_event(Event.new(type: EventType::SKIPPED, name: "item"))
    sleep(0.01)
    child.finish

    sleep(0.01)
    parent.stop

    # After child.finish, parent should have received a FINISHED event
    # and show 1/1
    out = writer.string
    assert_includes out, "1/1", "Parent should be 1/1 after child finish"
  end

  # -----------------------------------------------------------------------
  # Concurrency tests
  # -----------------------------------------------------------------------

  def test_concurrent_sends
    writer = StringIO.new
    tracker = Tracker.new(100, writer, "")
    tracker.start

    threads = (0...100).map do |i|
      Thread.new do
        name = "item-#{i}"
        tracker.send_event(Event.new(type: EventType::STARTED, name: name))
        tracker.send_event(Event.new(type: EventType::FINISHED, name: name, status: "ok"))
      end
    end

    threads.each(&:join)
    sleep(0.05) # let renderer catch up
    tracker.stop

    out = writer.string
    assert_includes out, "100/100",
      "All 100 items should complete after concurrent sends"
  end

  def test_concurrent_sends_no_crash
    # Stress test: many threads, rapid fire, no assertions on output --
    # just verify it doesn't raise or deadlock.
    writer = StringIO.new
    tracker = Tracker.new(200, writer, "")
    tracker.start

    threads = (0...200).map do |i|
      Thread.new do
        name = "stress-#{i}"
        tracker.send_event(Event.new(type: EventType::STARTED, name: name))
        sleep(rand * 0.005)
        tracker.send_event(Event.new(type: EventType::FINISHED, name: name, status: "done"))
      end
    end

    threads.each(&:join)
    sleep(0.05)
    tracker.stop

    # If we got here without hanging or crashing, the test passes.
    assert true, "Stress test completed without crash or deadlock"
  end

  # -----------------------------------------------------------------------
  # NullTracker tests
  # -----------------------------------------------------------------------

  def test_null_tracker_start_is_noop
    tracker = NullTracker.new
    tracker.start # should not raise
    assert true
  end

  def test_null_tracker_send_event_is_noop
    tracker = NullTracker.new
    tracker.send_event(Event.new(type: EventType::STARTED, name: "test"))
    assert true
  end

  def test_null_tracker_stop_is_noop
    tracker = NullTracker.new
    tracker.stop
    assert true
  end

  def test_null_tracker_finish_is_noop
    tracker = NullTracker.new
    tracker.finish
    assert true
  end

  def test_null_tracker_child_returns_null_tracker
    tracker = NullTracker.new
    child = tracker.child(5, "test")
    assert_instance_of NullTracker, child,
      "NullTracker.child should return another NullTracker"
  end

  def test_null_tracker_has_zero_defaults
    tracker = NullTracker.new
    assert_equal 0, tracker.total
    assert_equal 0, tracker.completed
    assert_equal "", tracker.label
  end

  # -----------------------------------------------------------------------
  # Event struct tests
  # -----------------------------------------------------------------------

  def test_event_default_status
    event = Event.new(type: EventType::STARTED, name: "test")
    assert_equal "", event.status, "Default status should be empty string"
  end

  def test_event_with_status
    event = Event.new(type: EventType::FINISHED, name: "pkg", status: "built")
    assert_equal "built", event.status
  end

  def test_event_is_frozen
    event = Event.new(type: EventType::STARTED, name: "test")
    assert event.frozen?, "Data objects should be frozen"
  end

  # -----------------------------------------------------------------------
  # Edge case tests
  # -----------------------------------------------------------------------

  def test_zero_total_tracker
    out = run_tracker(0, "", [])
    # Should not crash, bar should be all empty
    empty_bar = "\u2591" * 20
    assert_includes out, empty_bar, "Zero total should produce empty bar"
  end

  def test_finish_item_not_in_building_set
    # Finishing an item that was never started should still increment
    # completed (defensive behavior matching the Go implementation).
    events = [
      Event.new(type: EventType::FINISHED, name: "ghost", status: "ok")
    ]
    out = run_tracker(2, "", events)
    assert_includes out, "1/2", "Finishing unknown item should still increment"
  end

  def test_carriage_return_used
    out = run_tracker(1, "", [])
    assert_includes out, "\r", "Output should use carriage return for line overwrite"
  end

  def test_output_ends_with_newline
    out = run_tracker(1, "", [])
    assert out.end_with?("\n"), "Output should end with newline from stop()"
  end

  def test_waiting_then_done_transition
    # Verify the full lifecycle: waiting -> building -> done
    writer = StringIO.new
    tracker = Tracker.new(1, writer, "")
    tracker.start
    sleep(0.01)

    # Should show "waiting..." at this point
    early_out = writer.string
    assert_includes early_out, "waiting..."

    tracker.send_event(Event.new(type: EventType::STARTED, name: "x"))
    sleep(0.01)

    # Should show "Building: x"
    mid_out = writer.string
    assert_includes mid_out, "Building: x"

    tracker.send_event(Event.new(type: EventType::FINISHED, name: "x", status: "ok"))
    sleep(0.01)
    tracker.stop

    # Should show "done"
    final_out = writer.string
    assert_includes final_out, "done"
  end
end
