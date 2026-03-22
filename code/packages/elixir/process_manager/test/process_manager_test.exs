defmodule CodingAdventures.ProcessManagerTest do
  @moduledoc """
  Tests for the Process Manager — PCB creation, signal handling, fork, exec,
  wait, exit, kill, and priority scheduling.
  """

  use ExUnit.Case, async: true

  alias CodingAdventures.ProcessManager
  alias CodingAdventures.ProcessManager.SignalManager
  alias CodingAdventures.ProcessManager.Manager
  alias CodingAdventures.ProcessManager.Scheduler

  # ============================================================================
  # PCB Creation Tests
  # ============================================================================

  describe "create_pcb/3" do
    test "creates a PCB with correct defaults" do
      pcb = ProcessManager.create_pcb(1, "test-process")

      assert pcb.pid == 1
      assert pcb.name == "test-process"
      assert pcb.process_state == :ready
      assert length(pcb.registers) == 32
      assert Enum.all?(pcb.registers, &(&1 == 0))
      assert pcb.pc == 0
      assert pcb.sp == 0
      assert pcb.memory_base == 0
      assert pcb.memory_size == 0
      assert pcb.parent_pid == 0
      assert pcb.children == []
      assert pcb.pending_signals == []
      assert pcb.signal_handlers == %{}
      assert pcb.signal_mask == MapSet.new()
      assert pcb.priority == 20
      assert pcb.cpu_time == 0
      assert pcb.exit_code == 0
    end

    test "accepts a custom parent_pid" do
      pcb = ProcessManager.create_pcb(5, "child", 3)
      assert pcb.parent_pid == 3
    end

    test "has 32 registers initialized to zero" do
      pcb = ProcessManager.create_pcb(0, "init")
      assert length(pcb.registers) == 32
      Enum.each(pcb.registers, fn reg -> assert reg == 0 end)
    end
  end

  # ============================================================================
  # Process State Tests
  # ============================================================================

  describe "process_state_value/1" do
    test "returns correct numeric values" do
      assert ProcessManager.process_state_value(:ready) == 0
      assert ProcessManager.process_state_value(:running) == 1
      assert ProcessManager.process_state_value(:blocked) == 2
      assert ProcessManager.process_state_value(:terminated) == 3
      assert ProcessManager.process_state_value(:zombie) == 4
    end
  end

  # ============================================================================
  # Signal Tests
  # ============================================================================

  describe "signal_number/1" do
    test "returns correct POSIX signal numbers" do
      assert ProcessManager.signal_number(:sigint) == 2
      assert ProcessManager.signal_number(:sigkill) == 9
      assert ProcessManager.signal_number(:sigterm) == 15
      assert ProcessManager.signal_number(:sigchld) == 17
      assert ProcessManager.signal_number(:sigcont) == 18
      assert ProcessManager.signal_number(:sigstop) == 19
    end
  end

  # ============================================================================
  # Signal Manager Tests
  # ============================================================================

  describe "SignalManager.send_signal/2" do
    test "adds catchable signals to pending list" do
      pcb = ProcessManager.create_pcb(1, "test")
      {:enqueued, updated} = SignalManager.send_signal(pcb, :sigterm)

      assert :sigterm in updated.pending_signals
    end

    test "handles SIGKILL immediately" do
      pcb = ProcessManager.create_pcb(1, "test")
      {:immediate, updated} = SignalManager.send_signal(pcb, :sigkill)

      assert updated.process_state == :zombie
      assert updated.exit_code == 128 + 9
      assert updated.pending_signals == []
    end

    test "handles SIGSTOP immediately" do
      pcb = %{ProcessManager.create_pcb(1, "test") | process_state: :running}
      {:immediate, updated} = SignalManager.send_signal(pcb, :sigstop)

      assert updated.process_state == :blocked
    end

    test "handles SIGCONT by resuming a blocked process" do
      pcb = %{ProcessManager.create_pcb(1, "test") | process_state: :blocked}
      {:immediate, updated} = SignalManager.send_signal(pcb, :sigcont)

      assert updated.process_state == :ready
    end

    test "does not change state for SIGCONT on non-blocked process" do
      pcb = %{ProcessManager.create_pcb(1, "test") | process_state: :running}
      {:immediate, updated} = SignalManager.send_signal(pcb, :sigcont)

      assert updated.process_state == :running
    end

    test "enqueues SIGINT in pending list" do
      pcb = ProcessManager.create_pcb(1, "test")
      {:enqueued, updated} = SignalManager.send_signal(pcb, :sigint)

      assert :sigint in updated.pending_signals
    end

    test "enqueues SIGCHLD in pending list" do
      pcb = ProcessManager.create_pcb(1, "test")
      {:enqueued, updated} = SignalManager.send_signal(pcb, :sigchld)

      assert :sigchld in updated.pending_signals
    end
  end

  describe "SignalManager.deliver_pending/1" do
    test "delivers signals with default action (terminate)" do
      pcb = %{ProcessManager.create_pcb(1, "test") | pending_signals: [:sigterm]}
      {delivered, updated} = SignalManager.deliver_pending(pcb)

      assert delivered == [{:sigterm, :default_action}]
      assert updated.process_state == :zombie
      assert updated.exit_code == 128 + 15
      assert updated.pending_signals == []
    end

    test "delivers signals to custom handlers" do
      pcb = %{ProcessManager.create_pcb(1, "test") |
        signal_handlers: %{sigterm: 0x1000},
        pending_signals: [:sigterm]
      }
      {delivered, updated} = SignalManager.deliver_pending(pcb)

      assert delivered == [{:sigterm, 0x1000}]
      assert updated.process_state == :ready
      assert updated.pending_signals == []
    end

    test "skips masked signals (keeps them pending)" do
      pcb = %{ProcessManager.create_pcb(1, "test") |
        pending_signals: [:sigterm],
        signal_mask: MapSet.new([:sigterm])
      }
      {delivered, updated} = SignalManager.deliver_pending(pcb)

      assert delivered == []
      assert updated.pending_signals == [:sigterm]
    end

    test "delivers unmasked and keeps masked signals" do
      pcb = %{ProcessManager.create_pcb(1, "test") |
        signal_handlers: %{sigint: 0x2000},
        pending_signals: [:sigint, :sigterm],
        signal_mask: MapSet.new([:sigterm])
      }
      {delivered, updated} = SignalManager.deliver_pending(pcb)

      assert delivered == [{:sigint, 0x2000}]
      assert updated.pending_signals == [:sigterm]
    end

    test "handles SIGCHLD with default action (non-fatal)" do
      pcb = %{ProcessManager.create_pcb(1, "test") | pending_signals: [:sigchld]}
      {delivered, updated} = SignalManager.deliver_pending(pcb)

      assert delivered == [{:sigchld, :default_action}]
      assert updated.process_state == :ready
    end

    test "handles multiple pending signals in order" do
      pcb = %{ProcessManager.create_pcb(1, "test") |
        signal_handlers: %{sigint: 0x1000, sigterm: 0x2000},
        pending_signals: [:sigint, :sigterm]
      }
      {delivered, _updated} = SignalManager.deliver_pending(pcb)

      assert length(delivered) == 2
      assert Enum.at(delivered, 0) == {:sigint, 0x1000}
      assert Enum.at(delivered, 1) == {:sigterm, 0x2000}
    end
  end

  describe "SignalManager.register_handler/3" do
    test "registers a handler for catchable signals" do
      pcb = ProcessManager.create_pcb(1, "test")
      {:ok, updated} = SignalManager.register_handler(pcb, :sigterm, 0x1000)

      assert Map.get(updated.signal_handlers, :sigterm) == 0x1000
    end

    test "refuses to register handler for SIGKILL" do
      pcb = ProcessManager.create_pcb(1, "test")
      result = SignalManager.register_handler(pcb, :sigkill, 0x1000)

      assert result == {:error, :uncatchable}
    end

    test "refuses to register handler for SIGSTOP" do
      pcb = ProcessManager.create_pcb(1, "test")
      result = SignalManager.register_handler(pcb, :sigstop, 0x1000)

      assert result == {:error, :uncatchable}
    end
  end

  describe "SignalManager.mask_signal/2 and unmask_signal/2" do
    test "masks a catchable signal" do
      pcb = ProcessManager.create_pcb(1, "test")
      {:ok, updated} = SignalManager.mask_signal(pcb, :sigterm)

      assert MapSet.member?(updated.signal_mask, :sigterm)
    end

    test "refuses to mask SIGKILL" do
      pcb = ProcessManager.create_pcb(1, "test")
      result = SignalManager.mask_signal(pcb, :sigkill)

      assert result == {:error, :unmaskable}
    end

    test "refuses to mask SIGSTOP" do
      pcb = ProcessManager.create_pcb(1, "test")
      result = SignalManager.mask_signal(pcb, :sigstop)

      assert result == {:error, :unmaskable}
    end

    test "unmasks a signal" do
      pcb = %{ProcessManager.create_pcb(1, "test") |
        signal_mask: MapSet.new([:sigterm])
      }
      updated = SignalManager.unmask_signal(pcb, :sigterm)

      refute MapSet.member?(updated.signal_mask, :sigterm)
    end
  end

  describe "SignalManager.is_fatal/1" do
    test "returns true for SIGINT" do
      assert SignalManager.is_fatal(:sigint) == true
    end

    test "returns true for SIGKILL" do
      assert SignalManager.is_fatal(:sigkill) == true
    end

    test "returns true for SIGTERM" do
      assert SignalManager.is_fatal(:sigterm) == true
    end

    test "returns false for SIGCHLD" do
      assert SignalManager.is_fatal(:sigchld) == false
    end

    test "returns false for SIGCONT" do
      assert SignalManager.is_fatal(:sigcont) == false
    end

    test "returns true for SIGSTOP" do
      assert SignalManager.is_fatal(:sigstop) == true
    end
  end

  # ============================================================================
  # Process Manager Tests
  # ============================================================================

  describe "Manager.create_process/3" do
    test "creates processes with sequential PIDs" do
      mgr = %Manager{}
      {p0, mgr} = Manager.create_process(mgr, "init")
      {p1, mgr} = Manager.create_process(mgr, "shell", 0)
      {p2, _mgr} = Manager.create_process(mgr, "editor", 1)

      assert p0.pid == 0
      assert p1.pid == 1
      assert p2.pid == 2
    end

    test "adds child to parent's children list" do
      mgr = %Manager{}
      {_init, mgr} = Manager.create_process(mgr, "init")
      {shell, mgr} = Manager.create_process(mgr, "shell", 0)

      init = Manager.get_process(mgr, 0)
      assert shell.pid in init.children
    end

    test "sets parent_pid correctly" do
      mgr = %Manager{}
      {_init, mgr} = Manager.create_process(mgr, "init")
      {shell, _mgr} = Manager.create_process(mgr, "shell", 0)

      assert shell.parent_pid == 0
    end
  end

  describe "Manager.fork/2" do
    test "creates a child with a new PID" do
      mgr = %Manager{}
      {parent, mgr} = Manager.create_process(mgr, "shell")
      {:ok, _parent_result, child_pid, _mgr} = Manager.fork(mgr, parent.pid)

      assert child_pid != parent.pid
    end

    test "sets child's parent_pid to parent's PID" do
      mgr = %Manager{}
      {parent, mgr} = Manager.create_process(mgr, "shell")
      {:ok, _pr, child_pid, mgr} = Manager.fork(mgr, parent.pid)
      child = Manager.get_process(mgr, child_pid)

      assert child.parent_pid == parent.pid
    end

    test "returns child PID to parent and 0 to child" do
      mgr = %Manager{}
      {parent, mgr} = Manager.create_process(mgr, "shell")
      {:ok, _pr, child_pid, mgr} = Manager.fork(mgr, parent.pid)

      updated_parent = Manager.get_process(mgr, parent.pid)
      child = Manager.get_process(mgr, child_pid)

      # Parent's register a0 (index 10) = child PID
      assert Enum.at(updated_parent.registers, 10) == child_pid
      # Child's register a0 = 0
      assert Enum.at(child.registers, 10) == 0
    end

    test "adds child to parent's children list" do
      mgr = %Manager{}
      {parent, mgr} = Manager.create_process(mgr, "shell")
      {:ok, _pr, child_pid, mgr} = Manager.fork(mgr, parent.pid)

      updated_parent = Manager.get_process(mgr, parent.pid)
      assert child_pid in updated_parent.children
    end

    test "sets child's state to :ready" do
      mgr = %Manager{}
      {parent, mgr} = Manager.create_process(mgr, "shell")
      {:ok, _pr, child_pid, mgr} = Manager.fork(mgr, parent.pid)
      child = Manager.get_process(mgr, child_pid)

      assert child.process_state == :ready
    end

    test "resets child's cpu_time to 0" do
      mgr = %Manager{}
      {parent, mgr} = Manager.create_process(mgr, "shell")
      updated_parent = %{parent | cpu_time: 1000}
      mgr = %{mgr | process_table: Map.put(mgr.process_table, parent.pid, updated_parent)}

      {:ok, _pr, child_pid, mgr} = Manager.fork(mgr, parent.pid)
      child = Manager.get_process(mgr, child_pid)

      assert child.cpu_time == 0
    end

    test "inherits parent's priority" do
      mgr = %Manager{}
      {parent, mgr} = Manager.create_process(mgr, "shell")
      updated_parent = %{parent | priority: 10}
      mgr = %{mgr | process_table: Map.put(mgr.process_table, parent.pid, updated_parent)}

      {:ok, _pr, child_pid, mgr} = Manager.fork(mgr, parent.pid)
      child = Manager.get_process(mgr, child_pid)

      assert child.priority == 10
    end

    test "inherits parent's signal handlers" do
      mgr = %Manager{}
      {parent, mgr} = Manager.create_process(mgr, "shell")
      updated_parent = %{parent | signal_handlers: %{sigterm: 0x5000}}
      mgr = %{mgr | process_table: Map.put(mgr.process_table, parent.pid, updated_parent)}

      {:ok, _pr, child_pid, mgr} = Manager.fork(mgr, parent.pid)
      child = Manager.get_process(mgr, child_pid)

      assert Map.get(child.signal_handlers, :sigterm) == 0x5000
    end

    test "gives child an empty children list" do
      mgr = %Manager{}
      {parent, mgr} = Manager.create_process(mgr, "shell")
      {:ok, _pr, child_pid, mgr} = Manager.fork(mgr, parent.pid)
      child = Manager.get_process(mgr, child_pid)

      assert child.children == []
    end

    test "gives child an empty pending_signals list" do
      mgr = %Manager{}
      {parent, mgr} = Manager.create_process(mgr, "shell")
      updated_parent = %{parent | pending_signals: [:sigchld]}
      mgr = %{mgr | process_table: Map.put(mgr.process_table, parent.pid, updated_parent)}

      {:ok, _pr, child_pid, mgr} = Manager.fork(mgr, parent.pid)
      child = Manager.get_process(mgr, child_pid)

      assert child.pending_signals == []
    end

    test "returns :error for non-existent parent" do
      mgr = %Manager{}
      result = Manager.fork(mgr, 999)

      assert result == {:error, :not_found}
    end

    test "copies parent's PC and SP" do
      mgr = %Manager{}
      {parent, mgr} = Manager.create_process(mgr, "shell")
      updated_parent = %{parent | pc: 0x1000, sp: 0x7FFF}
      mgr = %{mgr | process_table: Map.put(mgr.process_table, parent.pid, updated_parent)}

      {:ok, _pr, child_pid, mgr} = Manager.fork(mgr, parent.pid)
      child = Manager.get_process(mgr, child_pid)

      assert child.pc == 0x1000
      assert child.sp == 0x7FFF
    end
  end

  describe "Manager.exec/4" do
    test "resets registers to zero" do
      mgr = %Manager{}
      {proc, mgr} = Manager.create_process(mgr, "shell")
      updated = %{proc | registers: List.replace_at(proc.registers, 5, 42)}
      mgr = %{mgr | process_table: Map.put(mgr.process_table, proc.pid, updated)}

      {:ok, mgr} = Manager.exec(mgr, proc.pid, 0x10000, 0x7FFFF000)
      result = Manager.get_process(mgr, proc.pid)

      assert Enum.all?(result.registers, &(&1 == 0))
    end

    test "sets PC to entry point" do
      mgr = %Manager{}
      {proc, mgr} = Manager.create_process(mgr, "shell")
      {:ok, mgr} = Manager.exec(mgr, proc.pid, 0x10000, 0x7FFFF000)
      result = Manager.get_process(mgr, proc.pid)

      assert result.pc == 0x10000
    end

    test "sets SP to stack top" do
      mgr = %Manager{}
      {proc, mgr} = Manager.create_process(mgr, "shell")
      {:ok, mgr} = Manager.exec(mgr, proc.pid, 0x10000, 0x7FFFF000)
      result = Manager.get_process(mgr, proc.pid)

      assert result.sp == 0x7FFFF000
    end

    test "clears signal handlers" do
      mgr = %Manager{}
      {proc, mgr} = Manager.create_process(mgr, "shell")
      updated = %{proc | signal_handlers: %{sigterm: 0x1000}}
      mgr = %{mgr | process_table: Map.put(mgr.process_table, proc.pid, updated)}

      {:ok, mgr} = Manager.exec(mgr, proc.pid, 0x10000, 0x7FFFF000)
      result = Manager.get_process(mgr, proc.pid)

      assert result.signal_handlers == %{}
    end

    test "clears pending signals" do
      mgr = %Manager{}
      {proc, mgr} = Manager.create_process(mgr, "shell")
      updated = %{proc | pending_signals: [:sigterm, :sigchld]}
      mgr = %{mgr | process_table: Map.put(mgr.process_table, proc.pid, updated)}

      {:ok, mgr} = Manager.exec(mgr, proc.pid, 0x10000, 0x7FFFF000)
      result = Manager.get_process(mgr, proc.pid)

      assert result.pending_signals == []
    end

    test "preserves PID" do
      mgr = %Manager{}
      {proc, mgr} = Manager.create_process(mgr, "shell")
      {:ok, mgr} = Manager.exec(mgr, proc.pid, 0x10000, 0x7FFFF000)
      result = Manager.get_process(mgr, proc.pid)

      assert result.pid == proc.pid
    end

    test "preserves parent_pid" do
      mgr = %Manager{}
      {_init, mgr} = Manager.create_process(mgr, "init")
      {proc, mgr} = Manager.create_process(mgr, "shell", 0)
      {:ok, mgr} = Manager.exec(mgr, proc.pid, 0x10000, 0x7FFFF000)
      result = Manager.get_process(mgr, proc.pid)

      assert result.parent_pid == 0
    end

    test "preserves children list" do
      mgr = %Manager{}
      {parent, mgr} = Manager.create_process(mgr, "shell")
      {:ok, _pr, child_pid, mgr} = Manager.fork(mgr, parent.pid)

      parent_before = Manager.get_process(mgr, parent.pid)
      children_before = parent_before.children

      {:ok, mgr} = Manager.exec(mgr, parent.pid, 0x10000, 0x7FFFF000)
      parent_result = Manager.get_process(mgr, parent.pid)

      assert parent_result.children == children_before
      assert child_pid in parent_result.children
    end

    test "preserves priority" do
      mgr = %Manager{}
      {proc, mgr} = Manager.create_process(mgr, "shell")
      updated = %{proc | priority: 5}
      mgr = %{mgr | process_table: Map.put(mgr.process_table, proc.pid, updated)}

      {:ok, mgr} = Manager.exec(mgr, proc.pid, 0x10000, 0x7FFFF000)
      result = Manager.get_process(mgr, proc.pid)

      assert result.priority == 5
    end

    test "preserves cpu_time" do
      mgr = %Manager{}
      {proc, mgr} = Manager.create_process(mgr, "shell")
      updated = %{proc | cpu_time: 500}
      mgr = %{mgr | process_table: Map.put(mgr.process_table, proc.pid, updated)}

      {:ok, mgr} = Manager.exec(mgr, proc.pid, 0x10000, 0x7FFFF000)
      result = Manager.get_process(mgr, proc.pid)

      assert result.cpu_time == 500
    end

    test "returns :error for non-existent PID" do
      mgr = %Manager{}
      result = Manager.exec(mgr, 999, 0x10000, 0x7FFFF000)

      assert result == {:error, :not_found}
    end
  end

  describe "Manager.wait_for_child/2" do
    test "returns zombie child's PID and exit code" do
      mgr = %Manager{}
      {parent, mgr} = Manager.create_process(mgr, "shell")
      {:ok, _pr, child_pid, mgr} = Manager.fork(mgr, parent.pid)

      # Make child a zombie.
      child = Manager.get_process(mgr, child_pid)
      zombie_child = %{child | process_state: :zombie, exit_code: 42}
      mgr = %{mgr | process_table: Map.put(mgr.process_table, child_pid, zombie_child)}

      {:ok, reaped_pid, exit_code, _mgr} = Manager.wait_for_child(mgr, parent.pid)

      assert reaped_pid == child_pid
      assert exit_code == 42
    end

    test "removes zombie from process table" do
      mgr = %Manager{}
      {parent, mgr} = Manager.create_process(mgr, "shell")
      {:ok, _pr, child_pid, mgr} = Manager.fork(mgr, parent.pid)

      child = Manager.get_process(mgr, child_pid)
      zombie_child = %{child | process_state: :zombie, exit_code: 0}
      mgr = %{mgr | process_table: Map.put(mgr.process_table, child_pid, zombie_child)}

      {:ok, _pid, _code, mgr} = Manager.wait_for_child(mgr, parent.pid)

      assert Manager.get_process(mgr, child_pid) == nil
    end

    test "removes child from parent's children list" do
      mgr = %Manager{}
      {parent, mgr} = Manager.create_process(mgr, "shell")
      {:ok, _pr, child_pid, mgr} = Manager.fork(mgr, parent.pid)

      child = Manager.get_process(mgr, child_pid)
      zombie_child = %{child | process_state: :zombie, exit_code: 0}
      mgr = %{mgr | process_table: Map.put(mgr.process_table, child_pid, zombie_child)}

      {:ok, _pid, _code, mgr} = Manager.wait_for_child(mgr, parent.pid)
      updated_parent = Manager.get_process(mgr, parent.pid)

      refute child_pid in updated_parent.children
    end

    test "returns :error when no zombie children exist" do
      mgr = %Manager{}
      {parent, mgr} = Manager.create_process(mgr, "shell")
      {:ok, _pr, _child_pid, mgr} = Manager.fork(mgr, parent.pid)

      result = Manager.wait_for_child(mgr, parent.pid)
      assert result == {:error, :no_zombie}
    end

    test "returns :error when parent has no children" do
      mgr = %Manager{}
      {parent, mgr} = Manager.create_process(mgr, "shell")

      result = Manager.wait_for_child(mgr, parent.pid)
      assert result == {:error, :no_zombie}
    end

    test "returns :error for non-existent parent" do
      mgr = %Manager{}
      result = Manager.wait_for_child(mgr, 999)

      assert result == {:error, :not_found}
    end
  end

  describe "Manager.exit_process/3" do
    test "sets process state to :zombie" do
      mgr = %Manager{}
      {proc, mgr} = Manager.create_process(mgr, "shell")
      {:ok, mgr} = Manager.exit_process(mgr, proc.pid, 0)
      result = Manager.get_process(mgr, proc.pid)

      assert result.process_state == :zombie
    end

    test "sets exit_code" do
      mgr = %Manager{}
      {proc, mgr} = Manager.create_process(mgr, "shell")
      {:ok, mgr} = Manager.exit_process(mgr, proc.pid, 42)
      result = Manager.get_process(mgr, proc.pid)

      assert result.exit_code == 42
    end

    test "reparents children to init (PID 0)" do
      mgr = %Manager{}
      {_init, mgr} = Manager.create_process(mgr, "init")
      {parent, mgr} = Manager.create_process(mgr, "shell", 0)
      {:ok, _pr, grandchild_pid, mgr} = Manager.fork(mgr, parent.pid)

      {:ok, mgr} = Manager.exit_process(mgr, parent.pid, 0)

      grandchild = Manager.get_process(mgr, grandchild_pid)
      assert grandchild.parent_pid == 0

      init = Manager.get_process(mgr, 0)
      assert grandchild_pid in init.children
    end

    test "sends SIGCHLD to parent" do
      mgr = %Manager{}
      {_init, mgr} = Manager.create_process(mgr, "init")
      {child, mgr} = Manager.create_process(mgr, "shell", 0)

      {:ok, mgr} = Manager.exit_process(mgr, child.pid, 0)

      init = Manager.get_process(mgr, 0)
      assert :sigchld in init.pending_signals
    end

    test "clears the process's children list" do
      mgr = %Manager{}
      {_init, mgr} = Manager.create_process(mgr, "init")
      {parent, mgr} = Manager.create_process(mgr, "shell", 0)
      {:ok, _pr, _child_pid, mgr} = Manager.fork(mgr, parent.pid)

      {:ok, mgr} = Manager.exit_process(mgr, parent.pid, 0)
      result = Manager.get_process(mgr, parent.pid)

      assert result.children == []
    end

    test "returns :error for non-existent process" do
      mgr = %Manager{}
      result = Manager.exit_process(mgr, 999, 0)

      assert result == {:error, :not_found}
    end
  end

  describe "Manager.kill/3" do
    test "sends SIGTERM (adds to pending)" do
      mgr = %Manager{}
      {proc, mgr} = Manager.create_process(mgr, "test")
      {:ok, mgr} = Manager.kill(mgr, proc.pid, :sigterm)
      result = Manager.get_process(mgr, proc.pid)

      assert :sigterm in result.pending_signals
    end

    test "sends SIGKILL (immediate termination)" do
      mgr = %Manager{}
      {_init, mgr} = Manager.create_process(mgr, "init")
      {proc, mgr} = Manager.create_process(mgr, "test", 0)
      {:ok, mgr} = Manager.kill(mgr, proc.pid, :sigkill)
      result = Manager.get_process(mgr, proc.pid)

      assert result.process_state == :zombie
    end

    test "sends SIGCHLD to parent when child is killed" do
      mgr = %Manager{}
      {parent, mgr} = Manager.create_process(mgr, "shell")
      {:ok, _pr, child_pid, mgr} = Manager.fork(mgr, parent.pid)

      {:ok, mgr} = Manager.kill(mgr, child_pid, :sigkill)
      updated_parent = Manager.get_process(mgr, parent.pid)

      assert :sigchld in updated_parent.pending_signals
    end

    test "returns :error for non-existent target" do
      mgr = %Manager{}
      result = Manager.kill(mgr, 999, :sigterm)

      assert result == {:error, :not_found}
    end

    test "sends SIGSTOP to block a process" do
      mgr = %Manager{}
      {proc, mgr} = Manager.create_process(mgr, "test")
      {:ok, mgr} = Manager.kill(mgr, proc.pid, :sigstop)
      result = Manager.get_process(mgr, proc.pid)

      assert result.process_state == :blocked
    end

    test "sends SIGCONT to resume a stopped process" do
      mgr = %Manager{}
      {proc, mgr} = Manager.create_process(mgr, "test")
      updated = %{proc | process_state: :blocked}
      mgr = %{mgr | process_table: Map.put(mgr.process_table, proc.pid, updated)}

      {:ok, mgr} = Manager.kill(mgr, proc.pid, :sigcont)
      result = Manager.get_process(mgr, proc.pid)

      assert result.process_state == :ready
    end
  end

  describe "fork + exec + wait lifecycle" do
    test "completes a full fork/exec/wait cycle" do
      mgr = %Manager{}
      {shell, mgr} = Manager.create_process(mgr, "shell")

      # Fork
      {:ok, _pr, child_pid, mgr} = Manager.fork(mgr, shell.pid)

      # Exec
      {:ok, mgr} = Manager.exec(mgr, child_pid, 0x10000, 0x7FFFF000)
      child = Manager.get_process(mgr, child_pid)
      assert child.pc == 0x10000

      # Exit
      {:ok, mgr} = Manager.exit_process(mgr, child_pid, 0)
      child = Manager.get_process(mgr, child_pid)
      assert child.process_state == :zombie

      # Wait (reap)
      {:ok, reaped_pid, exit_code, mgr} = Manager.wait_for_child(mgr, shell.pid)
      assert reaped_pid == child_pid
      assert exit_code == 0
      assert Manager.get_process(mgr, child_pid) == nil
    end
  end

  # ============================================================================
  # Priority Scheduler Tests
  # ============================================================================

  describe "Scheduler.enqueue/2" do
    test "places process in the correct priority queue" do
      sched = %Scheduler{}
      pcb = %{ProcessManager.create_pcb(1, "test") | priority: 5}

      sched = Scheduler.enqueue(sched, pcb)
      queues = Scheduler.get_ready_queues(sched)
      pids = Enum.map(Enum.at(queues, 5), & &1.pid)

      assert 1 in pids
    end

    test "sets process state to :ready" do
      sched = %Scheduler{}
      pcb = %{ProcessManager.create_pcb(1, "test") | process_state: :running}

      sched = Scheduler.enqueue(sched, pcb)
      queues = Scheduler.get_ready_queues(sched)
      enqueued = hd(Enum.at(queues, 20))

      assert enqueued.process_state == :ready
    end

    test "raises for out-of-range priority" do
      sched = %Scheduler{}
      pcb = %{ProcessManager.create_pcb(1, "test") | priority: 40}

      assert_raise ArgumentError, fn -> Scheduler.enqueue(sched, pcb) end
    end

    test "raises for negative priority" do
      sched = %Scheduler{}
      pcb = %{ProcessManager.create_pcb(1, "test") | priority: -1}

      assert_raise ArgumentError, fn -> Scheduler.enqueue(sched, pcb) end
    end
  end

  describe "Scheduler.schedule/1" do
    test "picks highest priority (lowest number) process first" do
      sched = %Scheduler{}
      low = %{ProcessManager.create_pcb(1, "low") | priority: 30}
      high = %{ProcessManager.create_pcb(2, "high") | priority: 5}
      mid = %{ProcessManager.create_pcb(3, "mid") | priority: 20}

      sched = sched |> Scheduler.enqueue(low) |> Scheduler.enqueue(high) |> Scheduler.enqueue(mid)
      {next, _sched} = Scheduler.schedule(sched)

      assert next.pid == 2
      assert next.process_state == :running
    end

    test "round-robins within the same priority" do
      sched = %Scheduler{}
      a = %{ProcessManager.create_pcb(1, "A") | priority: 20}
      b = %{ProcessManager.create_pcb(2, "B") | priority: 20}

      sched = sched |> Scheduler.enqueue(a) |> Scheduler.enqueue(b)
      {first, sched} = Scheduler.schedule(sched)
      assert first.pid == 1

      {second, _sched} = Scheduler.schedule(sched)
      assert second.pid == 2
    end

    test "returns nil when all queues are empty" do
      sched = %Scheduler{}
      {result, _sched} = Scheduler.schedule(sched)

      assert result == nil
    end

    test "sets current_process" do
      sched = %Scheduler{}
      pcb = %{ProcessManager.create_pcb(1, "test") | priority: 20}
      sched = Scheduler.enqueue(sched, pcb)

      {_next, sched} = Scheduler.schedule(sched)

      assert Scheduler.get_current(sched) != nil
      assert Scheduler.get_current(sched).pid == 1
    end

    test "clears current_process when nothing to schedule" do
      sched = %Scheduler{}
      {_next, sched} = Scheduler.schedule(sched)

      assert Scheduler.get_current(sched) == nil
    end
  end

  describe "Scheduler.preempt/2" do
    test "puts process back at the end of its queue" do
      sched = %Scheduler{}
      a = %{ProcessManager.create_pcb(1, "A") | priority: 20}
      b = %{ProcessManager.create_pcb(2, "B") | priority: 20}

      sched = sched |> Scheduler.enqueue(a) |> Scheduler.enqueue(b)
      {running, sched} = Scheduler.schedule(sched)
      assert running.pid == 1

      sched = Scheduler.preempt(sched, running)
      {next, sched} = Scheduler.schedule(sched)
      assert next.pid == 2

      {after_next, _sched} = Scheduler.schedule(sched)
      assert after_next.pid == 1
    end

    test "sets process state to :ready" do
      sched = %Scheduler{}
      pcb = %{ProcessManager.create_pcb(1, "test") | priority: 20, process_state: :running}

      sched = Scheduler.preempt(sched, pcb)
      queues = Scheduler.get_ready_queues(sched)
      enqueued = hd(Enum.at(queues, 20))

      assert enqueued.process_state == :ready
    end
  end

  describe "Scheduler.set_priority/3" do
    test "moves process to a different priority queue" do
      sched = %Scheduler{}
      pcb = %{ProcessManager.create_pcb(1, "test") | priority: 20}
      sched = Scheduler.enqueue(sched, pcb)

      {updated_pcb, sched} = Scheduler.set_priority(sched, pcb, 5)
      queues = Scheduler.get_ready_queues(sched)

      assert Enum.at(queues, 20) == []
      assert length(Enum.at(queues, 5)) == 1
      assert updated_pcb.priority == 5
    end

    test "updates priority even if not in any queue" do
      sched = %Scheduler{}
      pcb = %{ProcessManager.create_pcb(1, "test") | priority: 20}

      {updated_pcb, _sched} = Scheduler.set_priority(sched, pcb, 10)

      assert updated_pcb.priority == 10
    end

    test "raises for out-of-range priority" do
      sched = %Scheduler{}
      pcb = ProcessManager.create_pcb(1, "test")

      assert_raise ArgumentError, fn -> Scheduler.set_priority(sched, pcb, 40) end
      assert_raise ArgumentError, fn -> Scheduler.set_priority(sched, pcb, -1) end
    end

    test "does nothing when new priority equals old priority" do
      sched = %Scheduler{}
      pcb = %{ProcessManager.create_pcb(1, "test") | priority: 20}
      sched = Scheduler.enqueue(sched, pcb)

      {unchanged, sched} = Scheduler.set_priority(sched, pcb, 20)
      queues = Scheduler.get_ready_queues(sched)

      assert length(Enum.at(queues, 20)) == 1
      assert unchanged.priority == 20
    end
  end

  describe "Scheduler.get_time_quantum/1" do
    test "returns 200 for priority 0" do
      assert Scheduler.get_time_quantum(0) == 200
    end

    test "returns 50 for priority 39" do
      assert Scheduler.get_time_quantum(39) == 50
    end

    test "returns intermediate values for middle priorities" do
      q20 = Scheduler.get_time_quantum(20)
      assert q20 > 50
      assert q20 < 200
    end

    test "raises for out-of-range priority" do
      assert_raise ArgumentError, fn -> Scheduler.get_time_quantum(40) end
      assert_raise ArgumentError, fn -> Scheduler.get_time_quantum(-1) end
    end
  end
end
