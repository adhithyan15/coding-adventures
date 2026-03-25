# frozen_string_literal: true

require "minitest/autorun"
require "coding_adventures_event_loop"

# Bring the module into scope for readability in tests.
CF = CodingAdventures::EventLoop::ControlFlow

# ════════════════════════════════════════════════════════════════════════════
# Helpers — mock sources
# ════════════════════════════════════════════════════════════════════════════

# FixedSource emits a predefined list of event batches, one batch per poll.
# After all batches are exhausted, subsequent poll calls return [].
class FixedSource
  def initialize(*batches)
    @batches = batches
    @index   = 0
  end

  def poll
    return [] if @index >= @batches.size

    batch = @batches[@index]
    @index += 1
    batch
  end
end

# InfiniteSource returns one incrementing integer per poll. Never stops.
class InfiniteSource
  def initialize
    @n = 0
  end

  def poll
    @n += 1
    [@n]
  end
end

# ════════════════════════════════════════════════════════════════════════════
# Tests
# ════════════════════════════════════════════════════════════════════════════

class TestEventLoop < Minitest::Test
  # Every event emitted by a source must reach registered handlers.
  def test_delivers_all_events
    loop = CodingAdventures::EventLoop::Loop.new
    loop.add_source(FixedSource.new([1, 2, 3], [-1]))

    received = []
    loop.on_event do |e|
      if e == -1
        CF::EXIT
      else
        received << e
        CF::CONTINUE
      end
    end

    loop.run
    assert_equal [1, 2, 3], received
  end

  # When a handler returns EXIT, subsequent events must not be dispatched.
  def test_exit_stops_loop_immediately
    loop = CodingAdventures::EventLoop::Loop.new
    loop.add_source(FixedSource.new(%w[a b stop c d]))

    seen = []
    loop.on_event do |e|
      seen << e
      e == "stop" ? CF::EXIT : CF::CONTINUE
    end

    loop.run

    assert_equal %w[a b stop], seen
    refute_includes seen, "c"
    refute_includes seen, "d"
  end

  # stop() called from within a handler terminates the loop.
  def test_stop_from_handler
    loop = CodingAdventures::EventLoop::Loop.new
    loop.add_source(InfiniteSource.new)

    count = 0
    loop.on_event do |_e|
      count += 1
      loop.stop if count >= 5
      CF::CONTINUE
    end

    loop.run
    assert count >= 5
  end

  # All registered handlers must receive the same event.
  def test_multiple_handlers_all_see_event
    loop = CodingAdventures::EventLoop::Loop.new
    loop.add_source(FixedSource.new([99], [-1]))

    h1_saw = nil
    h2_saw = nil

    loop.on_event do |e|
      h1_saw = e if e == 99
      e == -1 ? CF::EXIT : CF::CONTINUE
    end

    loop.on_event do |e|
      h2_saw = e if e == 99
      CF::CONTINUE
    end

    loop.run

    assert_equal 99, h1_saw
    assert_equal 99, h2_saw
  end

  # Events from all sources must be collected and dispatched.
  def test_multiple_sources_merged
    loop = CodingAdventures::EventLoop::Loop.new
    loop.add_source(FixedSource.new(["alpha"]))
    loop.add_source(FixedSource.new(["beta"]))
    loop.add_source(FixedSource.new([], ["stop"]))

    seen = []
    loop.on_event do |e|
      next CF::EXIT if e == "stop"

      seen << e
      CF::CONTINUE
    end

    loop.run

    assert_equal 2, seen.size
    assert_includes seen, "alpha"
    assert_includes seen, "beta"
  end

  # Events from a single source arrive in the order the source returned them.
  def test_handler_sees_events_in_order
    loop = CodingAdventures::EventLoop::Loop.new
    loop.add_source(FixedSource.new([3, 1, 4, 1, 5], [-1]))

    received = []
    loop.on_event do |e|
      next CF::EXIT if e == -1

      received << e
      CF::CONTINUE
    end

    loop.run
    assert_equal [3, 1, 4, 1, 5], received
  end

  # ControlFlow constants must be distinct values.
  def test_control_flow_constants_distinct
    refute_equal CF::CONTINUE, CF::EXIT
  end

  # add_source returns self so calls can be chained.
  def test_add_source_chainable
    loop = CodingAdventures::EventLoop::Loop.new
    result = loop.add_source(FixedSource.new)
    assert_same loop, result
  end

  # on_event returns self so calls can be chained.
  def test_on_event_chainable
    loop = CodingAdventures::EventLoop::Loop.new
    result = loop.on_event { CF::CONTINUE }
    assert_same loop, result
  end

  # version constant must exist.
  def test_version_exists
    refute_nil CodingAdventures::EventLoop::VERSION
  end
end
