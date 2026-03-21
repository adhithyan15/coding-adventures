# frozen_string_literal: true

# ==========================================================================
# Tracker -- The Progress Bar Engine
# ==========================================================================
#
# This file implements a thread-safe progress bar using Ruby's Thread::Queue
# and a background rendering thread. It is the Ruby port of the Go
# progress-bar package and follows the same architecture.
#
# # The Postal Worker Analogy
#
# Imagine a post office with a single clerk (the renderer thread) and a
# mail slot (the Thread::Queue). Workers from all over town (application
# threads) drop letters (events) into the slot. The clerk picks them up
# one at a time and updates the scoreboard on the wall (the progress bar).
#
# Because only the clerk touches the scoreboard, there is no confusion or
# conflict -- even if a hundred workers drop letters at the same time.
#
#   +----------+     +----------+     +----------+
#   | Worker 1 |     | Worker 2 |     | Worker 3 |   (many threads)
#   +----+-----+     +----+-----+     +----+-----+
#        |                |                |
#        v                v                v
#   +------------------------------------------+
#   |          Thread::Queue (mail slot)       |    (thread-safe FIFO)
#   +--------------------+---------------------+
#                        |
#                        v
#              +---------+---------+
#              | Renderer Thread   |               (single consumer)
#              | (the postal clerk)|
#              +---------+---------+
#                        |
#                        v
#              +---------+---------+
#              | Terminal / IO     |               (the scoreboard)
#              +-------------------+
#
# This is Ruby's equivalent of Go's channel pattern: many writers, one
# reader, no explicit locks needed. Thread::Queue handles all the
# synchronization internally.
#
# # Why Thread::Queue instead of Mutex?
#
# A Mutex-based approach would require every caller to:
#   1. Acquire the lock
#   2. Update shared state
#   3. Release the lock
#   4. Trigger a redraw (somehow)
#
# With Thread::Queue, callers just push an event and walk away. The
# renderer thread owns ALL mutable state (completed count, building set),
# so there are zero race conditions by construction.
# ==========================================================================

module CodingAdventures
  module ProgressBar
    # -----------------------------------------------------------------------
    # Event Types -- What Can Happen to a Tracked Item
    # -----------------------------------------------------------------------
    #
    # Think of these like a traffic light:
    #
    #   STARTED  = green  (item is actively being processed)
    #   FINISHED = red    (item is done -- success or failure)
    #   SKIPPED  = yellow (item was bypassed without processing)
    #
    # There are exactly three things that can happen to any tracked item,
    # and each event type maps to exactly one state transition. No item
    # can go backwards (you can't un-finish something).
    module EventType
      STARTED  = :started
      FINISHED = :finished
      SKIPPED  = :skipped
    end

    # -----------------------------------------------------------------------
    # Event -- The Message Workers Send to the Tracker
    # -----------------------------------------------------------------------
    #
    # An Event is deliberately minimal -- just three fields:
    #
    #   type   -- what happened (STARTED, FINISHED, SKIPPED)
    #   name   -- human-readable identifier (e.g., "python/logic-gates")
    #   status -- outcome label, only meaningful for FINISHED events
    #             (e.g., "built", "failed", "cached")
    #
    # Using Ruby's Data class (immutable value object, introduced in Ruby
    # 3.2) ensures events cannot be accidentally mutated after creation.
    Event = Data.define(:type, :name, :status) do
      # Provide a default for status so callers can omit it for
      # STARTED and SKIPPED events.
      def initialize(type:, name:, status: "")
        super
      end
    end

    # -----------------------------------------------------------------------
    # Tracker -- The Main Progress Bar Class
    # -----------------------------------------------------------------------
    #
    # A Tracker receives events from concurrent threads and renders a
    # text-based progress bar. It is safe to call send_event from any
    # thread -- it just pushes onto the Thread::Queue.
    #
    # # State Tracking
    #
    # The renderer thread maintains:
    #
    #   completed -- count of items that are FINISHED or SKIPPED
    #   building  -- set of item names currently in-flight (STARTED but
    #                not yet FINISHED)
    #   total     -- the target count (set at creation time)
    #
    # # Truth Table for State Transitions
    #
    # This table shows exactly how each event type modifies the two
    # pieces of mutable state. There are no other state changes anywhere
    # in the system.
    #
    #   Event     | completed | building
    #   ----------+-----------+-------------------
    #   STARTED   | unchanged | add name to set
    #   FINISHED  | +1        | remove name from set
    #   SKIPPED   | +1        | unchanged
    #
    # Notice how simple this is: each event type touches at most two
    # fields, and the transitions are unconditional (no "if this then
    # that" logic). This simplicity is what makes the system reliable
    # under concurrency.
    #
    # # Bar Rendering
    #
    # The bar is 20 characters wide, using Unicode block characters:
    #
    #   \u2588 (full block) -- filled portion
    #   \u2591 (light shade) -- empty portion
    #
    # The number of filled characters is: (completed * 20) / total
    #
    # Integer division naturally rounds down, so the bar only shows
    # 100% when all items are truly complete.
    #
    # We use \r (carriage return) to overwrite the current line. This
    # works on all platforms -- Windows cmd, PowerShell, Git Bash, and
    # Unix terminals. No ANSI escape codes needed.
    class Tracker
      # The width of the progress bar in characters. 20 is a good balance
      # between readability and terminal width.
      BAR_WIDTH = 20

      # Maximum number of in-flight names to display. Beyond this count,
      # we show "+N more" to avoid line overflow.
      MAX_NAMES = 3

      # A sentinel object pushed onto the queue to signal the renderer
      # thread to stop. We use a unique frozen object so it can never
      # be confused with a real Event.
      STOP_SENTINEL = :__stop__

      attr_reader :total, :completed, :label

      # Creates a new Tracker.
      #
      # @param total [Integer] the number of items to track
      # @param writer [IO] the output stream (e.g., $stderr, StringIO)
      # @param label [String] optional prefix label (e.g., "Level")
      def initialize(total, writer, label = "")
        @total     = total
        @writer    = writer
        @label     = label
        @completed = 0
        @building  = {}
        @queue     = Thread::Queue.new
        @renderer  = nil
        @start_time = nil
        @parent    = nil
      end

      # Launches the background renderer thread. Call this once before
      # sending any events.
      #
      # The renderer thread is the "postal clerk" -- it sits in a loop
      # reading events from the queue, updating internal counters, and
      # redrawing the progress bar after each event.
      def start
        @start_time = Process.clock_gettime(Process::CLOCK_MONOTONIC)
        draw # Draw initial frame so the user sees "waiting..." immediately.
        @renderer = Thread.new { render_loop }
      end

      # Submits an event to the tracker. This is safe to call from any
      # thread -- it just pushes onto the Thread::Queue.
      #
      # @param event [Event] the event to process
      def send_event(event)
        @queue.push(event)
      end

      # Creates a nested sub-tracker for hierarchical progress.
      #
      # The child shares the parent's writer and start time. When the
      # child calls finish(), it advances the parent's completed count
      # by 1 (by sending a FINISHED event to the parent).
      #
      # Example: a build system has 3 dependency levels, each with N
      # packages. The parent tracks levels (total=3, label="Level"),
      # and each child tracks packages within that level.
      #
      #   parent = Tracker.new(3, $stderr, "Level")
      #   parent.start
      #   child = parent.child(7, "Package")
      #   # Display: Level 1/3  [####....] 3/7 Building: pkg-a (2.1s)
      #
      # @param child_total [Integer] number of items in the child
      # @param child_label [String] label for the child
      # @return [Tracker] the child tracker (already started)
      def child(child_total, child_label)
        c = Tracker.new(child_total, @writer, child_label)
        c.instance_variable_set(:@start_time, @start_time)
        c.instance_variable_set(:@parent, self)
        c.instance_variable_set(:@renderer, Thread.new { c.send(:render_loop) })
        c
      end

      # Marks this child tracker as complete and advances the parent
      # tracker by one. Call this when all items in the child are done.
      #
      # This pushes a stop sentinel onto the child's queue, waits for
      # the renderer to drain, then sends a FINISHED event to the parent.
      def finish
        @queue.push(STOP_SENTINEL)
        @renderer&.join
        @parent&.send_event(Event.new(type: EventType::FINISHED, name: @label))
      end

      # Shuts down the tracker. Pushes the stop sentinel, waits for
      # the renderer thread to drain and exit, then prints a final
      # newline so the last progress line is preserved in terminal
      # scrollback.
      def stop
        @queue.push(STOP_SENTINEL)
        @renderer&.join
        @writer.write("\n")
      end

      private

      # -------------------------------------------------------------------
      # Internal: The Renderer Loop
      # -------------------------------------------------------------------
      #
      # This is the background thread that processes events and redraws
      # the progress bar. It runs until it receives the STOP_SENTINEL.
      #
      # The loop is simple: read event -> update state -> redraw.
      # Because this is the only thread that reads or writes tracker
      # state (completed, building), there are no race conditions.
      def render_loop
        loop do
          event = @queue.pop
          break if event == STOP_SENTINEL

          case event.type
          when EventType::STARTED
            # A new item has begun processing. Add it to the in-flight
            # set so it appears in the "Building: ..." display.
            @building[event.name] = true
          when EventType::FINISHED
            # An item has completed. Remove it from in-flight and bump
            # the completed counter.
            @building.delete(event.name)
            @completed += 1
          when EventType::SKIPPED
            # An item was bypassed. Just bump the completed counter --
            # it was never in the building set.
            @completed += 1
          end

          draw
        end

        # Final draw after loop exits -- ensures the bar shows the
        # terminal state (often 100%).
        draw
      end

      # -------------------------------------------------------------------
      # Internal: Drawing One Progress Line
      # -------------------------------------------------------------------
      #
      # Composes and writes one progress line to the writer.
      #
      # The line format depends on whether we have a parent (hierarchical)
      # or not (flat):
      #
      # Flat (no label):
      #   [########............]  7/21  Building: pkg-a, pkg-b  (12.3s)
      #
      # Flat (with label, used as parent):
      #   Level 2/3  [####............]  waiting...  (8.2s)
      #
      # Hierarchical (child with parent):
      #   Level 2/3  [########........]  5/12  Building: pkg-a  (8.2s)
      def draw
        elapsed = Process.clock_gettime(Process::CLOCK_MONOTONIC) - @start_time

        # --- Build the progress bar ---
        #
        # filled = (completed * BAR_WIDTH) / total
        #
        # Integer division naturally rounds down, so the bar only shows
        # 100% when all items are truly complete.
        filled = @total > 0 ? (@completed * BAR_WIDTH) / @total : 0
        filled = BAR_WIDTH if filled > BAR_WIDTH
        bar = "\u2588" * filled + "\u2591" * (BAR_WIDTH - filled)

        # --- Build the in-flight names list ---
        activity = format_activity

        # --- Compose the line ---
        line = if @parent
                 # Hierarchical: show parent label and count.
                 parent_completed = @parent.completed + 1
                 format("\r%s %d/%d  [%s]  %d/%d  %s  (%.1fs)",
                   @parent.label, parent_completed, @parent.total,
                   bar, @completed, @total, activity, elapsed)
               elsif !@label.empty?
                 # Labeled flat tracker (used as parent).
                 format("\r%s %d/%d  [%s]  %s  (%.1fs)",
                   @label, @completed, @total, bar, activity, elapsed)
               else
                 # Flat mode: just the bar.
                 format("\r[%s]  %d/%d  %s  (%.1fs)",
                   bar, @completed, @total, activity, elapsed)
               end

        # Pad to 80 characters to overwrite any previous longer line.
        @writer.write(format("%-80s", line))
      end

      # -------------------------------------------------------------------
      # Internal: Formatting the Activity String
      # -------------------------------------------------------------------
      #
      # Builds the "Building: pkg-a, pkg-b" or "waiting..." or "done"
      # string from the current in-flight set.
      #
      # The rules:
      #
      #   | In-flight count | Completed vs Total | Output                      |
      #   |-----------------|--------------------|-----------------------------|
      #   | 0               | completed < total  | "waiting..."                |
      #   | 0               | completed >= total | "done"                      |
      #   | 1-3             | any                | "Building: a, b, c"         |
      #   | 4+              | any                | "Building: a, b, c +N more" |
      #
      # Names are sorted alphabetically for deterministic output.
      # This matters for testing and for user sanity -- the display
      # shouldn't jump around randomly.
      def format_activity
        if @building.empty?
          return @completed >= @total ? "done" : "waiting..."
        end

        names = @building.keys.sort

        if names.length <= MAX_NAMES
          "Building: #{names.join(", ")}"
        else
          shown = names.first(MAX_NAMES).join(", ")
          "Building: #{shown} +#{names.length - MAX_NAMES} more"
        end
      end
    end

    # -----------------------------------------------------------------------
    # NullTracker -- A No-Op Progress Bar
    # -----------------------------------------------------------------------
    #
    # NullTracker has the exact same public interface as Tracker, but every
    # method is a no-op. This is the Null Object pattern -- it lets calling
    # code use a progress bar unconditionally without nil-checking:
    #
    #   tracker = verbose ? Tracker.new(10, $stderr) : NullTracker.new
    #   tracker.start
    #   tracker.send_event(event)  # works either way
    #   tracker.stop
    #
    # This is analogous to /dev/null -- you can write to it all day and
    # nothing happens. The advantage over using nil is that callers don't
    # need to guard every call with `tracker&.send_event(event)`.
    class NullTracker
      attr_reader :total, :completed, :label

      def initialize
        @total = 0
        @completed = 0
        @label = ""
      end

      def start; end

      def send_event(_event); end

      def child(_total, _label)
        NullTracker.new
      end

      def finish; end

      def stop; end
    end
  end
end
