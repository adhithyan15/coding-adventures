# frozen_string_literal: true

require_relative "test_helper"

# = Tests for ProcessManager
#
# These tests cover the core Unix process lifecycle: fork, exec, wait,
# kill, and exit. Each operation is tested independently and in
# combination to ensure correctness.

class TestProcessManager < Minitest::Test
  include CodingAdventures::ProcessManager

  def setup
    @pm = ProcessManager.new
    # Create an init process (PID 0) as the root of the process tree.
    @init_pid = @pm.create_process("init")
  end

  # -- create_process --

  def test_create_process_assigns_sequential_pids
    pid1 = @pm.create_process("proc1")
    pid2 = @pm.create_process("proc2")
    assert_equal pid1 + 1, pid2
  end

  def test_create_process_sets_name
    pid = @pm.create_process("my_process")
    assert_equal "my_process", @pm.get_process(pid).name
  end

  def test_create_process_sets_default_priority
    pid = @pm.create_process("normal")
    assert_equal 20, @pm.get_process(pid).priority
  end

  def test_create_process_with_custom_priority
    pid = @pm.create_process("kernel_task", priority: 0)
    assert_equal 0, @pm.get_process(pid).priority
  end

  def test_create_process_state_is_ready
    pid = @pm.create_process("new")
    assert @pm.get_process(pid).ready?
  end

  def test_process_count
    initial_count = @pm.process_count
    @pm.create_process("extra")
    assert_equal initial_count + 1, @pm.process_count
  end

  def test_process_exists
    pid = @pm.create_process("exists")
    assert @pm.process_exists?(pid)
    refute @pm.process_exists?(9999)
  end

  # -- fork --

  def test_fork_returns_child_pid
    child_pid = @pm.fork(@init_pid)
    assert child_pid > @init_pid
  end

  def test_fork_child_has_different_pid
    child_pid = @pm.fork(@init_pid)
    refute_equal @init_pid, child_pid
  end

  def test_fork_child_parent_pid_set
    child_pid = @pm.fork(@init_pid)
    child = @pm.get_process(child_pid)
    assert_equal @init_pid, child.parent_pid
  end

  def test_fork_child_in_parents_children_list
    child_pid = @pm.fork(@init_pid)
    parent = @pm.get_process(@init_pid)
    assert_includes parent.children, child_pid
  end

  def test_fork_child_state_is_ready
    child_pid = @pm.fork(@init_pid)
    child = @pm.get_process(child_pid)
    assert child.ready?
  end

  def test_fork_child_cpu_time_is_zero
    parent = @pm.get_process(@init_pid)
    parent.cpu_time = 500
    child_pid = @pm.fork(@init_pid)
    child = @pm.get_process(child_pid)
    assert_equal 0, child.cpu_time
  end

  def test_fork_child_inherits_priority
    parent = @pm.get_process(@init_pid)
    parent.priority = 5
    child_pid = @pm.fork(@init_pid)
    child = @pm.get_process(child_pid)
    assert_equal 5, child.priority
  end

  def test_fork_return_values_in_registers
    # In RISC-V, register x10 (a0) holds the return value.
    # Parent sees child_pid in a0, child sees 0 in a0.
    child_pid = @pm.fork(@init_pid)
    parent = @pm.get_process(@init_pid)
    child = @pm.get_process(child_pid)

    assert_equal child_pid, parent.registers[10]
    assert_equal 0, child.registers[10]
  end

  def test_fork_child_copies_registers
    parent = @pm.get_process(@init_pid)
    parent.registers[5] = 42
    parent.registers[15] = 99

    child_pid = @pm.fork(@init_pid)
    child = @pm.get_process(child_pid)

    assert_equal 42, child.registers[5]
    assert_equal 99, child.registers[15]
  end

  def test_fork_child_copies_pc_and_sp
    parent = @pm.get_process(@init_pid)
    parent.pc = 0x1000
    parent.sp = 0x7FFF

    child_pid = @pm.fork(@init_pid)
    child = @pm.get_process(child_pid)

    assert_equal 0x1000, child.pc
    assert_equal 0x7FFF, child.sp
  end

  def test_fork_child_inherits_signal_handlers
    parent = @pm.get_process(@init_pid)
    parent.signal_handlers[Signal::SIGTERM] = 0x3000

    child_pid = @pm.fork(@init_pid)
    child = @pm.get_process(child_pid)

    assert_equal 0x3000, child.signal_handlers[Signal::SIGTERM]
  end

  def test_fork_child_has_empty_children_list
    child_pid = @pm.fork(@init_pid)
    child = @pm.get_process(child_pid)
    assert_empty child.children
  end

  def test_fork_child_has_no_pending_signals
    parent = @pm.get_process(@init_pid)
    parent.pending_signals << Signal::SIGTERM

    child_pid = @pm.fork(@init_pid)
    child = @pm.get_process(child_pid)
    assert_empty child.pending_signals
  end

  def test_fork_nonexistent_parent_returns_negative
    result = @pm.fork(9999)
    assert_equal(-1, result)
  end

  def test_fork_adds_to_process_table
    count_before = @pm.process_count
    @pm.fork(@init_pid)
    assert_equal count_before + 1, @pm.process_count
  end

  # -- exec --

  def test_exec_sets_pc_to_entry_point
    child_pid = @pm.fork(@init_pid)
    @pm.exec(child_pid, entry_point: 0x10000, stack_pointer: 0x7FFFF)

    child = @pm.get_process(child_pid)
    assert_equal 0x10000, child.pc
  end

  def test_exec_sets_stack_pointer
    child_pid = @pm.fork(@init_pid)
    @pm.exec(child_pid, entry_point: 0x10000, stack_pointer: 0x7FFFF)

    child = @pm.get_process(child_pid)
    assert_equal 0x7FFFF, child.sp
  end

  def test_exec_zeros_registers
    child_pid = @pm.fork(@init_pid)
    child = @pm.get_process(child_pid)
    child.registers[5] = 42

    @pm.exec(child_pid, entry_point: 0x10000, stack_pointer: 0x7FFFF)

    assert child.registers.all?(&:zero?)
  end

  def test_exec_clears_signal_handlers
    child_pid = @pm.fork(@init_pid)
    child = @pm.get_process(child_pid)
    child.signal_handlers[Signal::SIGTERM] = 0x1000

    @pm.exec(child_pid, entry_point: 0x10000, stack_pointer: 0x7FFFF)

    assert_empty child.signal_handlers
  end

  def test_exec_clears_pending_signals
    child_pid = @pm.fork(@init_pid)
    child = @pm.get_process(child_pid)
    child.pending_signals << Signal::SIGTERM

    @pm.exec(child_pid, entry_point: 0x10000, stack_pointer: 0x7FFFF)

    assert_empty child.pending_signals
  end

  def test_exec_preserves_pid
    child_pid = @pm.fork(@init_pid)
    @pm.exec(child_pid, entry_point: 0x10000, stack_pointer: 0x7FFFF)

    child = @pm.get_process(child_pid)
    assert_equal child_pid, child.pid
  end

  def test_exec_preserves_parent_pid
    child_pid = @pm.fork(@init_pid)
    @pm.exec(child_pid, entry_point: 0x10000, stack_pointer: 0x7FFFF)

    child = @pm.get_process(child_pid)
    assert_equal @init_pid, child.parent_pid
  end

  def test_exec_nonexistent_pid_returns_false
    refute @pm.exec(9999, entry_point: 0x1000, stack_pointer: 0x7FFF)
  end

  # -- wait --

  def test_wait_returns_zombie_child
    child_pid = @pm.fork(@init_pid)
    @pm.exit_process(child_pid, exit_code: 42)

    result = @pm.wait(@init_pid)
    assert_equal child_pid, result[:pid]
    assert_equal 42, result[:exit_code]
  end

  def test_wait_removes_zombie_from_table
    child_pid = @pm.fork(@init_pid)
    @pm.exit_process(child_pid, exit_code: 0)

    @pm.wait(@init_pid)
    refute @pm.process_exists?(child_pid)
  end

  def test_wait_removes_child_from_parent_children_list
    child_pid = @pm.fork(@init_pid)
    @pm.exit_process(child_pid, exit_code: 0)

    @pm.wait(@init_pid)
    parent = @pm.get_process(@init_pid)
    refute_includes parent.children, child_pid
  end

  def test_wait_returns_nil_when_no_zombies
    @pm.fork(@init_pid)  # child is READY, not ZOMBIE
    result = @pm.wait(@init_pid)
    assert_nil result
  end

  def test_wait_returns_nil_when_no_children
    result = @pm.wait(@init_pid)
    assert_nil result
  end

  def test_wait_nonexistent_parent_returns_nil
    result = @pm.wait(9999)
    assert_nil result
  end

  def test_wait_reaps_first_zombie_when_multiple_children
    child1 = @pm.fork(@init_pid)
    child2 = @pm.fork(@init_pid)

    @pm.exit_process(child1, exit_code: 1)
    @pm.exit_process(child2, exit_code: 2)

    result1 = @pm.wait(@init_pid)
    result2 = @pm.wait(@init_pid)

    pids = [result1[:pid], result2[:pid]].sort
    assert_equal [child1, child2].sort, pids
  end

  # -- kill --

  def test_kill_sends_signal
    child_pid = @pm.fork(@init_pid)
    result = @pm.kill(child_pid, Signal::SIGTERM)

    assert result
    child = @pm.get_process(child_pid)
    assert_includes child.pending_signals, Signal::SIGTERM
  end

  def test_kill_nonexistent_pid_returns_false
    refute @pm.kill(9999, Signal::SIGTERM)
  end

  # -- exit_process --

  def test_exit_sets_zombie_state
    child_pid = @pm.fork(@init_pid)
    @pm.exit_process(child_pid, exit_code: 0)

    child = @pm.get_process(child_pid)
    assert child.zombie?
  end

  def test_exit_sets_exit_code
    child_pid = @pm.fork(@init_pid)
    @pm.exit_process(child_pid, exit_code: 42)

    child = @pm.get_process(child_pid)
    assert_equal 42, child.exit_code
  end

  def test_exit_reparents_children_to_pid_0
    child_pid = @pm.fork(@init_pid)
    grandchild_pid = @pm.fork(child_pid)

    @pm.exit_process(child_pid, exit_code: 0)

    grandchild = @pm.get_process(grandchild_pid)
    assert_equal 0, grandchild.parent_pid
  end

  def test_exit_sends_sigchld_to_parent
    child_pid = @pm.fork(@init_pid)
    @pm.exit_process(child_pid, exit_code: 0)

    parent = @pm.get_process(@init_pid)
    assert_includes parent.pending_signals, Signal::SIGCHLD
  end

  def test_exit_clears_children_list
    child_pid = @pm.fork(@init_pid)
    @pm.fork(child_pid)  # grandchild

    @pm.exit_process(child_pid, exit_code: 0)

    child = @pm.get_process(child_pid)
    assert_empty child.children
  end

  def test_exit_nonexistent_pid_returns_false
    refute @pm.exit_process(9999)
  end

  # -- Integration: fork + exec + wait lifecycle --

  def test_fork_exec_wait_lifecycle
    # 1. Fork a child from init.
    child_pid = @pm.fork(@init_pid)
    assert @pm.process_exists?(child_pid)

    # 2. Exec a new program in the child.
    @pm.exec(child_pid, entry_point: 0x10000, stack_pointer: 0x7FFFF)
    child = @pm.get_process(child_pid)
    assert_equal 0x10000, child.pc

    # 3. Child exits with status 42.
    @pm.exit_process(child_pid, exit_code: 42)
    assert child.zombie?

    # 4. Parent waits and retrieves the exit status.
    result = @pm.wait(@init_pid)
    assert_equal child_pid, result[:pid]
    assert_equal 42, result[:exit_code]

    # 5. Zombie is reaped -- no longer in process table.
    refute @pm.process_exists?(child_pid)
  end

  def test_multiple_fork_and_wait
    child1 = @pm.fork(@init_pid)
    child2 = @pm.fork(@init_pid)
    child3 = @pm.fork(@init_pid)

    @pm.exit_process(child2, exit_code: 20)
    @pm.exit_process(child1, exit_code: 10)
    @pm.exit_process(child3, exit_code: 30)

    results = []
    3.times do
      r = @pm.wait(@init_pid)
      results << r if r
    end

    assert_equal 3, results.size
    exit_codes = results.map { |r| r[:exit_code] }.sort
    assert_equal [10, 20, 30], exit_codes
  end
end
