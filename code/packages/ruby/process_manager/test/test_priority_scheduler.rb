# frozen_string_literal: true

require_relative "test_helper"

# = Tests for PriorityScheduler
#
# The priority scheduler picks the highest-priority (lowest-numbered)
# process to run. Within the same priority, it uses round-robin (FIFO).
# These tests verify correct ordering, priority changes, and edge cases.

class TestPriorityScheduler < Minitest::Test
  include CodingAdventures::ProcessManager

  def setup
    @scheduler = PriorityScheduler.new
  end

  # Helper to create a PCB with a given PID and priority.
  def make_pcb(pid, name, priority: 20)
    ProcessControlBlock.new(pid: pid, name: name, priority: priority)
  end

  # -- add_process and schedule --

  def test_schedule_returns_nil_when_empty
    assert_nil @scheduler.schedule
  end

  def test_schedule_returns_only_process
    pcb = make_pcb(1, "solo", priority: 20)
    @scheduler.add_process(pcb)

    result = @scheduler.schedule
    assert_equal 1, result.pid
  end

  def test_schedule_picks_highest_priority
    # Priority 5 should run before priority 20, which runs before priority 39.
    high   = make_pcb(1, "high",   priority: 5)
    normal = make_pcb(2, "normal", priority: 20)
    low    = make_pcb(3, "low",    priority: 39)

    # Add in reverse order to prove it sorts by priority, not insertion order.
    @scheduler.add_process(low)
    @scheduler.add_process(normal)
    @scheduler.add_process(high)

    assert_equal 1, @scheduler.schedule.pid   # high first
    assert_equal 2, @scheduler.schedule.pid   # normal second
    assert_equal 3, @scheduler.schedule.pid   # low third
    assert_nil @scheduler.schedule             # empty
  end

  def test_round_robin_within_same_priority
    # Two processes at priority 20 should alternate.
    a = make_pcb(1, "a", priority: 20)
    b = make_pcb(2, "b", priority: 20)

    @scheduler.add_process(a)
    @scheduler.add_process(b)

    first = @scheduler.schedule
    assert_equal 1, first.pid

    second = @scheduler.schedule
    assert_equal 2, second.pid

    assert_nil @scheduler.schedule
  end

  def test_round_robin_with_readd
    a = make_pcb(1, "a", priority: 20)
    b = make_pcb(2, "b", priority: 20)

    @scheduler.add_process(a)
    @scheduler.add_process(b)

    # Schedule a, then put it back at the end.
    first = @scheduler.schedule
    assert_equal 1, first.pid
    @scheduler.add_process(first)

    # b should be next (it was in front of re-added a).
    second = @scheduler.schedule
    assert_equal 2, second.pid
  end

  # -- remove_process --

  def test_remove_process
    pcb = make_pcb(1, "removable", priority: 20)
    @scheduler.add_process(pcb)

    removed = @scheduler.remove_process(1)
    assert_equal 1, removed.pid
    assert_nil @scheduler.schedule
  end

  def test_remove_nonexistent_process
    result = @scheduler.remove_process(999)
    assert_nil result
  end

  def test_remove_from_middle_of_queue
    a = make_pcb(1, "a", priority: 20)
    b = make_pcb(2, "b", priority: 20)
    c = make_pcb(3, "c", priority: 20)

    @scheduler.add_process(a)
    @scheduler.add_process(b)
    @scheduler.add_process(c)

    @scheduler.remove_process(2)

    assert_equal 1, @scheduler.schedule.pid
    assert_equal 3, @scheduler.schedule.pid
    assert_nil @scheduler.schedule
  end

  # -- set_priority --

  def test_set_priority_moves_process
    pcb = make_pcb(1, "moving", priority: 20)
    @scheduler.add_process(pcb)

    @scheduler.set_priority(1, 5)

    # Process should now be in priority 5 queue.
    result = @scheduler.schedule
    assert_equal 1, result.pid
    assert_equal 5, result.priority
  end

  def test_set_priority_nonexistent_returns_false
    refute @scheduler.set_priority(999, 10)
  end

  def test_set_priority_clamps_to_valid_range
    pcb = make_pcb(1, "clamped", priority: 20)
    @scheduler.add_process(pcb)

    @scheduler.set_priority(1, -10)
    result = @scheduler.schedule
    assert_equal 0, result.priority
  end

  def test_set_priority_affects_scheduling_order
    a = make_pcb(1, "a", priority: 20)
    b = make_pcb(2, "b", priority: 20)

    @scheduler.add_process(a)
    @scheduler.add_process(b)

    # Boost b to higher priority.
    @scheduler.set_priority(2, 5)

    # b should now be scheduled first despite being added second.
    assert_equal 2, @scheduler.schedule.pid
    assert_equal 1, @scheduler.schedule.pid
  end

  # -- time_quantum_for --

  def test_time_quantum_highest_priority
    assert_equal 200, PriorityScheduler.time_quantum_for(0)
  end

  def test_time_quantum_lowest_priority
    assert_equal 50, PriorityScheduler.time_quantum_for(39)
  end

  def test_time_quantum_default_priority
    quantum = PriorityScheduler.time_quantum_for(20)
    # Should be between min and max.
    assert quantum > 50
    assert quantum < 200
  end

  def test_time_quantum_clamped
    # Out-of-range priorities should be clamped.
    assert_equal 200, PriorityScheduler.time_quantum_for(-5)
    assert_equal 50, PriorityScheduler.time_quantum_for(100)
  end

  # -- total_ready and empty? --

  def test_total_ready
    assert_equal 0, @scheduler.total_ready

    @scheduler.add_process(make_pcb(1, "a"))
    assert_equal 1, @scheduler.total_ready

    @scheduler.add_process(make_pcb(2, "b"))
    assert_equal 2, @scheduler.total_ready
  end

  def test_empty_when_no_processes
    assert @scheduler.empty?
  end

  def test_not_empty_with_processes
    @scheduler.add_process(make_pcb(1, "a"))
    refute @scheduler.empty?
  end

  # -- Integration --

  def test_preemption_scenario
    # Low-priority process is running.
    low = make_pcb(1, "background", priority: 30)
    @scheduler.add_process(low)

    scheduled = @scheduler.schedule
    assert_equal 1, scheduled.pid

    # High-priority process becomes ready (e.g., unblocked by I/O).
    high = make_pcb(2, "keyboard", priority: 0)
    @scheduler.add_process(high)

    # Put the low-priority process back (preempted).
    @scheduler.add_process(low)

    # Next schedule picks the high-priority process.
    next_up = @scheduler.schedule
    assert_equal 2, next_up.pid
  end
end
