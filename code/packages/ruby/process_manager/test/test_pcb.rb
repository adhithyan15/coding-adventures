# frozen_string_literal: true

require_relative "test_helper"

# = Tests for ProcessState and ProcessControlBlock
#
# These tests verify that the PCB correctly initializes, tracks state,
# and supports fork-style copying. The PCB is the kernel's "file" on
# each process, so correctness here is critical.

class TestProcessState < Minitest::Test
  include CodingAdventures::ProcessManager

  # -- ProcessState Constants --

  def test_state_values_match_posix_convention
    # Process states use conventional integer values.
    # READY=0, RUNNING=1, BLOCKED=2, TERMINATED=3, ZOMBIE=4.
    assert_equal 0, ProcessState::READY
    assert_equal 1, ProcessState::RUNNING
    assert_equal 2, ProcessState::BLOCKED
    assert_equal 3, ProcessState::TERMINATED
    assert_equal 4, ProcessState::ZOMBIE
  end

  def test_valid_state_check
    assert ProcessState.valid?(ProcessState::READY)
    assert ProcessState.valid?(ProcessState::RUNNING)
    assert ProcessState.valid?(ProcessState::BLOCKED)
    assert ProcessState.valid?(ProcessState::TERMINATED)
    assert ProcessState.valid?(ProcessState::ZOMBIE)
    refute ProcessState.valid?(99)
    refute ProcessState.valid?(-1)
  end

  def test_state_names
    assert_equal "READY",      ProcessState.name_for(ProcessState::READY)
    assert_equal "RUNNING",    ProcessState.name_for(ProcessState::RUNNING)
    assert_equal "BLOCKED",    ProcessState.name_for(ProcessState::BLOCKED)
    assert_equal "TERMINATED", ProcessState.name_for(ProcessState::TERMINATED)
    assert_equal "ZOMBIE",     ProcessState.name_for(ProcessState::ZOMBIE)
    assert_equal "UNKNOWN",    ProcessState.name_for(99)
  end
end

class TestProcessControlBlock < Minitest::Test
  include CodingAdventures::ProcessManager

  def setup
    @pcb = ProcessControlBlock.new(pid: 1, name: "test_process")
  end

  # -- Initialization --

  def test_pcb_creation_with_defaults
    assert_equal 1, @pcb.pid
    assert_equal "test_process", @pcb.name
    assert_equal ProcessState::READY, @pcb.state
    assert_equal 20, @pcb.priority  # default priority
    assert_equal 0, @pcb.pc
    assert_equal 0, @pcb.sp
    assert_equal 0, @pcb.memory_base
    assert_equal 0, @pcb.memory_size
    assert_equal 0, @pcb.parent_pid
    assert_equal 0, @pcb.cpu_time
    assert_equal 0, @pcb.exit_code
  end

  def test_pcb_registers_initialized_to_zero
    assert_equal 32, @pcb.registers.length
    assert @pcb.registers.all?(&:zero?)
  end

  def test_pcb_children_initially_empty
    assert_empty @pcb.children
  end

  def test_pcb_pending_signals_initially_empty
    assert_empty @pcb.pending_signals
  end

  def test_pcb_signal_handlers_initially_empty
    assert_empty @pcb.signal_handlers
  end

  def test_pcb_signal_mask_initially_empty
    assert_empty @pcb.signal_mask
  end

  def test_pcb_custom_priority
    pcb = ProcessControlBlock.new(pid: 2, name: "high_prio", priority: 5)
    assert_equal 5, pcb.priority
  end

  def test_pcb_priority_clamped_to_valid_range
    pcb_low = ProcessControlBlock.new(pid: 3, name: "below_min", priority: -5)
    assert_equal 0, pcb_low.priority

    pcb_high = ProcessControlBlock.new(pid: 4, name: "above_max", priority: 100)
    assert_equal 39, pcb_high.priority
  end

  # -- State Query Methods --

  def test_ready_predicate
    @pcb.state = ProcessState::READY
    assert @pcb.ready?
    refute @pcb.running?
    refute @pcb.blocked?
    refute @pcb.zombie?
    refute @pcb.terminated?
  end

  def test_running_predicate
    @pcb.state = ProcessState::RUNNING
    assert @pcb.running?
    refute @pcb.ready?
  end

  def test_blocked_predicate
    @pcb.state = ProcessState::BLOCKED
    assert @pcb.blocked?
    refute @pcb.ready?
  end

  def test_zombie_predicate
    @pcb.state = ProcessState::ZOMBIE
    assert @pcb.zombie?
    refute @pcb.ready?
  end

  def test_terminated_predicate
    @pcb.state = ProcessState::TERMINATED
    assert @pcb.terminated?
    refute @pcb.ready?
  end

  # -- State Transitions --

  def test_state_transition_ready_to_running
    @pcb.state = ProcessState::RUNNING
    assert @pcb.running?
  end

  def test_state_transition_running_to_blocked
    @pcb.state = ProcessState::RUNNING
    @pcb.state = ProcessState::BLOCKED
    assert @pcb.blocked?
  end

  def test_state_transition_running_to_zombie
    @pcb.state = ProcessState::RUNNING
    @pcb.state = ProcessState::ZOMBIE
    assert @pcb.zombie?
  end

  # -- Fork Copy --

  def test_fork_copy_creates_new_pcb
    @pcb.registers[5] = 42
    @pcb.pc = 0x1000
    @pcb.sp = 0x7FFF
    @pcb.memory_base = 0x2000
    @pcb.memory_size = 4096
    @pcb.priority = 10
    @pcb.signal_handlers[Signal::SIGTERM] = 0x3000

    child = @pcb.fork_copy(99)

    # New PID
    assert_equal 99, child.pid

    # Parent PID set to original's PID
    assert_equal 1, child.parent_pid

    # Inherited fields
    assert_equal "test_process", child.name
    assert_equal 42, child.registers[5]
    assert_equal 0x1000, child.pc
    assert_equal 0x7FFF, child.sp
    assert_equal 0x2000, child.memory_base
    assert_equal 4096, child.memory_size
    assert_equal 10, child.priority
    assert_equal({Signal::SIGTERM => 0x3000}, child.signal_handlers)

    # Reset fields
    assert_equal ProcessState::READY, child.state
    assert_empty child.children
    assert_empty child.pending_signals
    assert_equal 0, child.cpu_time
    assert_equal 0, child.exit_code
  end

  def test_fork_copy_has_independent_registers
    @pcb.registers[0] = 100
    child = @pcb.fork_copy(2)

    # Modifying child's registers doesn't affect parent's.
    child.registers[0] = 200
    assert_equal 100, @pcb.registers[0]
  end

  def test_fork_copy_has_independent_children_list
    child = @pcb.fork_copy(2)
    child.children << 99
    assert_empty @pcb.children
  end

  def test_fork_copy_has_independent_signal_handlers
    @pcb.signal_handlers[Signal::SIGTERM] = 0x1000
    child = @pcb.fork_copy(2)
    child.signal_handlers[Signal::SIGINT] = 0x2000

    refute @pcb.signal_handlers.key?(Signal::SIGINT)
  end

  # -- to_s --

  def test_to_s_includes_key_fields
    s = @pcb.to_s
    assert_includes s, "pid=1"
    assert_includes s, "name=test_process"
    assert_includes s, "READY"
    assert_includes s, "priority=20"
  end
end
