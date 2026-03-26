defmodule CodingAdventures.ProcessManager do
  @moduledoc """
  # Process Manager — fork, exec, wait, signals, and priority scheduling

  Every program you run on your computer is a **process** — an instance of a
  program in execution. When you open a text editor, that is a process. When
  you run `ls` in the terminal, that is a process.

  But how are processes *created*? How does the shell run a command? The answer
  lies in three elegant Unix system calls: **fork**, **exec**, and **wait**.

  ## Analogy: The Restaurant Kitchen

  Think of a restaurant kitchen. The head chef (parent process) can:

  - **fork():** Clone themselves — two identical chefs, same knowledge.
  - **exec():** The clone throws away their recipe book and picks up a new one.
    Same person (same PID), completely different work.
  - **wait():** The head chef watches the clone work. When the clone finishes,
    the head chef resumes.

  This is exactly how your shell works when you type `ls`:

      Shell (PID 100)
      |
      +-- fork() --> creates child (PID 101), exact copy of the shell
      |   |
      |   +-- [Child PID 101]: exec("ls")
      |   |     Replaces shell code with ls code.
      |   |
      |   +-- [Parent PID 100]: wait(101)
      |         Pauses until child exits.
      |
      +-- Shell prompt appears again.

  ## Signals

  Signals are **software interrupts** sent between processes. When you press
  Ctrl+C, the terminal sends SIGINT. When you run `kill <pid>`, the shell
  sends SIGTERM. Some signals like SIGKILL and SIGSTOP are **uncatchable**.

  ## Priority Scheduling

  Not all processes are equal. A keyboard handler should respond instantly,
  while a background indexer can wait. Priority 0 is highest, 39 is lowest.

  ## Elixir Implementation Notes

  Since Elixir uses immutable data structures, all operations return new
  structs rather than mutating in place. The process table is represented
  as a map from PID to PCB, and all manager functions take and return the
  full state.
  """

  # ============================================================================
  # Process State
  # ============================================================================

  @doc """
  ProcessState represents the lifecycle stages of a process.

  - `:ready` (0) — Loaded in memory, waiting for CPU time.
  - `:running` (1) — Currently executing on the CPU.
  - `:blocked` (2) — Waiting for I/O or an event.
  - `:terminated` (3) — Finished execution (transient before zombie).
  - `:zombie` (4) — Exited but parent has not called wait() yet.

  State Transition Diagram:

      fork() --> :ready --[scheduled]--> :running --[exit]--> :zombie
                   ^                       |                     |
                   |                       v                  [wait()]
                   +---- :blocked <--[I/O/wait]                  |
                                                              REMOVED

      SIGSTOP --> :blocked --[SIGCONT]--> :ready
  """

  @process_states %{
    ready: 0,
    running: 1,
    blocked: 2,
    terminated: 3,
    zombie: 4
  }

  @type process_state :: :ready | :running | :blocked | :terminated | :zombie

  def process_state_value(state_atom), do: Map.fetch!(@process_states, state_atom)

  # ============================================================================
  # Signal
  # ============================================================================

  @doc """
  Standard POSIX signal numbers.

  These numbers are standardized by POSIX and are the same on every Unix system.

      +----------+--------+-------------------+-------------+
      | Name     | Number | Default Action    | Catchable?  |
      +----------+--------+-------------------+-------------+
      | SIGINT   |   2    | Terminate         | Yes         |
      | SIGKILL  |   9    | Terminate         | NO          |
      | SIGTERM  |  15    | Terminate         | Yes         |
      | SIGCHLD  |  17    | Ignore            | Yes         |
      | SIGCONT  |  18    | Continue          | Yes         |
      | SIGSTOP  |  19    | Stop              | NO          |
      +----------+--------+-------------------+-------------+

  SIGKILL and SIGSTOP cannot be caught because the kernel must always have
  a way to forcibly terminate or suspend any process.
  """

  @type signal :: :sigint | :sigkill | :sigterm | :sigchld | :sigcont | :sigstop

  @signal_numbers %{
    sigint: 2,
    sigkill: 9,
    sigterm: 15,
    sigchld: 17,
    sigcont: 18,
    sigstop: 19
  }

  def signal_number(signal_atom), do: Map.fetch!(@signal_numbers, signal_atom)
  def signal_numbers, do: @signal_numbers

  # ============================================================================
  # Process Control Block (PCB)
  # ============================================================================

  # The ProcessControlBlock is the kernel's data structure for tracking a process.
  #
  # Think of it as the process's "passport" — it contains everything the kernel
  # needs to know: who the process is (PID, name), what it was doing (registers,
  # program counter), its family (parent, children), and pending mail (signals).
  #
  # ## Fields
  #
  #     +------------------+---------------------------------------------------+
  #     | Field            | Purpose                                           |
  #     +------------------+---------------------------------------------------+
  #     | pid              | Unique identifier                                 |
  #     | name             | Human-readable label                              |
  #     | process_state    | Current lifecycle stage                           |
  #     | registers        | Saved CPU registers (32 integers)                 |
  #     | pc               | Program counter                                   |
  #     | sp               | Stack pointer                                     |
  #     | memory_base      | Start of memory region                            |
  #     | memory_size      | Size of memory region                             |
  #     | parent_pid       | Who created this process                          |
  #     | children         | List of child PIDs                                |
  #     | pending_signals  | Signals waiting to be delivered                   |
  #     | signal_handlers  | Custom handlers (map signal -> address)           |
  #     | signal_mask      | Signals blocked from delivery (MapSet)            |
  #     | priority         | Scheduling priority (0=highest, 39=lowest)        |
  #     | cpu_time         | Total CPU cycles consumed                         |
  #     | exit_code        | Exit status (meaningful when zombie)              |
  #     +------------------+---------------------------------------------------+
  #
  # Note: We use `process_state` instead of `state` to avoid confusion with
  # Elixir's own process state concepts.

  defmodule PCB do
    @moduledoc "Process Control Block — the kernel's record for one process."

    @enforce_keys [:pid, :name]
    defstruct [
      :pid,
      :name,
      process_state: :ready,
      registers: List.duplicate(0, 32),
      pc: 0,
      sp: 0,
      memory_base: 0,
      memory_size: 0,
      parent_pid: 0,
      children: [],
      pending_signals: [],
      signal_handlers: %{},
      signal_mask: MapSet.new(),
      priority: 20,
      cpu_time: 0,
      exit_code: 0
    ]
  end

  @doc """
  Creates a new PCB with sensible defaults.

  Every new process starts in `:ready` state, with zeroed registers, no
  children, no pending signals, default priority (20), and zero CPU time.

  ## Examples

      iex> pcb = CodingAdventures.ProcessManager.create_pcb(1, "init")
      iex> pcb.pid
      1
      iex> pcb.process_state
      :ready
      iex> pcb.priority
      20
  """
  def create_pcb(pid, name, parent_pid \\ 0) do
    %PCB{pid: pid, name: name, parent_pid: parent_pid}
  end

  # ============================================================================
  # Signal Manager
  # ============================================================================

  # The SignalManager module handles all signal-related operations.
  #
  # ## Signal Delivery Flow
  #
  # When process A sends a signal to process B:
  #
  #     1. send_signal(B, :sigterm)
  #        --> :sigterm added to B's pending_signals
  #
  #     2. When B is next scheduled:
  #        --> deliver_pending(B) is called
  #        --> Check: is :sigterm masked? If yes, skip.
  #        --> Check: does B have a handler? YES: use it. NO: default action.

  defmodule SignalManager do
    @moduledoc "Handles signal sending, delivery, masking, and handling."

    alias CodingAdventures.ProcessManager.PCB

    @doc """
    Sends a signal to a process by adding it to the pending list.

    SIGKILL and SIGSTOP are handled immediately (not enqueued).
    Returns `{:immediate, updated_pcb}` or `{:enqueued, updated_pcb}`.

    ## Examples

        iex> pcb = CodingAdventures.ProcessManager.create_pcb(1, "test")
        iex> {status, updated} = CodingAdventures.ProcessManager.SignalManager.send_signal(pcb, :sigterm)
        iex> status
        :enqueued
        iex> :sigterm in updated.pending_signals
        true
    """
    def send_signal(%PCB{} = pcb, :sigkill) do
      # SIGKILL: the nuclear option. Immediately terminates. No handler,
      # no mask, no escape. Exit code = 128 + signal number.
      updated = %{pcb | process_state: :zombie, exit_code: 128 + 9}
      {:immediate, updated}
    end

    def send_signal(%PCB{} = pcb, :sigstop) do
      # SIGSTOP: freeze the process. Cannot be caught.
      updated = %{pcb | process_state: :blocked}
      {:immediate, updated}
    end

    def send_signal(%PCB{} = pcb, :sigcont) do
      # SIGCONT: resume a stopped process.
      updated =
        if pcb.process_state == :blocked do
          %{pcb | process_state: :ready}
        else
          pcb
        end

      {:immediate, updated}
    end

    def send_signal(%PCB{} = pcb, signal) do
      # All other signals: add to pending list for deferred delivery.
      updated = %{pcb | pending_signals: pcb.pending_signals ++ [signal]}
      {:enqueued, updated}
    end

    @doc """
    Delivers all pending signals to a process.

    Returns `{delivered_actions, updated_pcb}` where delivered_actions is
    a list of `{signal, action}` tuples. Action is either a handler address
    (integer) or `:default_action`.
    """
    def deliver_pending(%PCB{} = pcb) do
      {delivered, still_pending, updated_pcb} =
        Enum.reduce(pcb.pending_signals, {[], [], pcb}, fn signal, {del, pend, current_pcb} ->
          if MapSet.member?(current_pcb.signal_mask, signal) do
            # Masked: keep pending.
            {del, pend ++ [signal], current_pcb}
          else
            handler = Map.get(current_pcb.signal_handlers, signal)

            if handler != nil do
              # Custom handler exists.
              {del ++ [{signal, handler}], pend, current_pcb}
            else
              # Default action.
              new_pcb =
                if is_fatal(signal) do
                  %{current_pcb | process_state: :zombie, exit_code: 128 + signal_num(signal)}
                else
                  current_pcb
                end

              {del ++ [{signal, :default_action}], pend, new_pcb}
            end
          end
        end)

      final_pcb = %{updated_pcb | pending_signals: still_pending}
      {delivered, final_pcb}
    end

    @doc """
    Registers a custom signal handler for a process.

    SIGKILL and SIGSTOP cannot have custom handlers.
    Returns `{:ok, updated_pcb}` or `{:error, :uncatchable}`.
    """
    def register_handler(%PCB{} = pcb, signal, handler_addr) do
      if signal in [:sigkill, :sigstop] do
        {:error, :uncatchable}
      else
        updated = %{pcb | signal_handlers: Map.put(pcb.signal_handlers, signal, handler_addr)}
        {:ok, updated}
      end
    end

    @doc """
    Adds a signal to the process's signal mask, blocking delivery.

    SIGKILL and SIGSTOP cannot be masked.
    """
    def mask_signal(%PCB{} = pcb, signal) do
      if signal in [:sigkill, :sigstop] do
        {:error, :unmaskable}
      else
        updated = %{pcb | signal_mask: MapSet.put(pcb.signal_mask, signal)}
        {:ok, updated}
      end
    end

    @doc """
    Removes a signal from the process's mask, allowing delivery.
    """
    def unmask_signal(%PCB{} = pcb, signal) do
      %{pcb | signal_mask: MapSet.delete(pcb.signal_mask, signal)}
    end

    @doc """
    Returns true if a signal's default action is fatal (terminates the process).

    SIGCHLD default action is "ignore" (non-fatal).
    SIGCONT default action is "continue" (non-fatal).
    All others are fatal.
    """
    def is_fatal(:sigchld), do: false
    def is_fatal(:sigcont), do: false
    def is_fatal(_signal), do: true

    # Helper to convert signal atom to number.
    defp signal_num(:sigint), do: 2
    defp signal_num(:sigkill), do: 9
    defp signal_num(:sigterm), do: 15
    defp signal_num(:sigchld), do: 17
    defp signal_num(:sigcont), do: 18
    defp signal_num(:sigstop), do: 19
  end

  # ============================================================================
  # Process Manager
  # ============================================================================

  # The ProcessManager manages the process table and implements fork, exec,
  # wait, kill, and exit.
  #
  # Since Elixir data is immutable, the ProcessManager state is a struct that
  # gets passed through and returned from each operation.

  defmodule Manager do
    @moduledoc """
    Core process management: create, fork, exec, wait, kill, exit.

    The manager holds the process table (map of PID -> PCB) and a counter
    for the next PID to allocate.
    """

    alias CodingAdventures.ProcessManager.PCB
    alias CodingAdventures.ProcessManager.SignalManager

    defstruct process_table: %{}, next_pid: 0

    @doc """
    Creates a new process and adds it to the process table.

    Returns `{pcb, updated_manager}`.
    """
    def create_process(%__MODULE__{} = mgr, name, parent_pid \\ 0) do
      pid = mgr.next_pid
      pcb = CodingAdventures.ProcessManager.create_pcb(pid, name, parent_pid)

      # Add child to parent's children list (if parent exists and is not self).
      updated_table =
        if parent_pid != pid do
          case Map.get(mgr.process_table, parent_pid) do
            nil ->
              Map.put(mgr.process_table, pid, pcb)

            parent ->
              updated_parent = %{parent | children: parent.children ++ [pid]}

              mgr.process_table
              |> Map.put(parent_pid, updated_parent)
              |> Map.put(pid, pcb)
          end
        else
          Map.put(mgr.process_table, pid, pcb)
        end

      updated_mgr = %{mgr | process_table: updated_table, next_pid: pid + 1}
      {pcb, updated_mgr}
    end

    @doc """
    Forks a process — creates an almost exact copy with a new PID.

    The child is a clone of the parent with these differences:
    - New PID
    - parent_pid set to the original process's PID
    - Empty children list
    - Empty pending_signals
    - cpu_time reset to 0
    - Register a0 (index 10): parent gets child PID, child gets 0

    Returns `{:ok, parent_result, child_pid, updated_manager}` or
    `{:error, :not_found}`.
    """
    def fork(%__MODULE__{} = mgr, parent_pid) do
      case Map.get(mgr.process_table, parent_pid) do
        nil ->
          {:error, :not_found}

        parent ->
          child_pid = mgr.next_pid

          # Create child as a copy of parent with differences.
          child_registers = List.replace_at(parent.registers, 10, 0)
          parent_registers = List.replace_at(parent.registers, 10, child_pid)

          child = %PCB{
            pid: child_pid,
            name: parent.name,
            process_state: :ready,
            registers: child_registers,
            pc: parent.pc,
            sp: parent.sp,
            memory_base: parent.memory_base,
            memory_size: parent.memory_size,
            parent_pid: parent.pid,
            children: [],
            pending_signals: [],
            signal_handlers: Map.new(parent.signal_handlers),
            signal_mask: MapSet.new(parent.signal_mask),
            priority: parent.priority,
            cpu_time: 0,
            exit_code: 0
          }

          updated_parent = %{parent |
            registers: parent_registers,
            children: parent.children ++ [child_pid]
          }

          updated_table =
            mgr.process_table
            |> Map.put(parent_pid, updated_parent)
            |> Map.put(child_pid, child)

          updated_mgr = %{mgr | process_table: updated_table, next_pid: child_pid + 1}
          {:ok, child_pid, child_pid, updated_mgr}
      end
    end

    @doc """
    Replaces a process's program with a new one.

    Resets registers, sets PC and SP, clears signal handlers and pending
    signals. Preserves PID, parent_pid, children, priority, and cpu_time.

    Returns `{:ok, updated_manager}` or `{:error, :not_found}`.
    """
    def exec(%__MODULE__{} = mgr, pid, entry_point, stack_top) do
      case Map.get(mgr.process_table, pid) do
        nil ->
          {:error, :not_found}

        pcb ->
          updated_pcb = %{pcb |
            registers: List.duplicate(0, 32),
            pc: entry_point,
            sp: stack_top,
            signal_handlers: %{},
            pending_signals: []
          }

          updated_table = Map.put(mgr.process_table, pid, updated_pcb)
          {:ok, %{mgr | process_table: updated_table}}
      end
    end

    @doc """
    Waits for a child process to exit and reaps its zombie.

    Searches the parent's children for any zombie. If found, removes it
    from the process table and returns its PID and exit code.

    Returns `{:ok, child_pid, exit_code, updated_manager}` or
    `{:error, :no_zombie}` or `{:error, :not_found}`.
    """
    def wait_for_child(%__MODULE__{} = mgr, parent_pid) do
      case Map.get(mgr.process_table, parent_pid) do
        nil ->
          {:error, :not_found}

        parent ->
          # Find the first zombie child.
          zombie =
            Enum.find(parent.children, fn child_pid ->
              case Map.get(mgr.process_table, child_pid) do
                %PCB{process_state: :zombie} -> true
                _ -> false
              end
            end)

          case zombie do
            nil ->
              {:error, :no_zombie}

            zombie_pid ->
              zombie_pcb = Map.get(mgr.process_table, zombie_pid)
              exit_code = zombie_pcb.exit_code

              # Remove zombie from parent's children list and process table.
              updated_parent = %{parent |
                children: List.delete(parent.children, zombie_pid)
              }

              updated_table =
                mgr.process_table
                |> Map.put(parent_pid, updated_parent)
                |> Map.delete(zombie_pid)

              {:ok, zombie_pid, exit_code, %{mgr | process_table: updated_table}}
          end
      end
    end

    @doc """
    Sends a signal to a process.

    Returns `{:ok, updated_manager}` or `{:error, :not_found}`.
    If the signal causes termination (SIGKILL), SIGCHLD is sent to the parent.
    """
    def kill(%__MODULE__{} = mgr, target_pid, signal) do
      case Map.get(mgr.process_table, target_pid) do
        nil ->
          {:error, :not_found}

        target ->
          {_status, updated_target} = SignalManager.send_signal(target, signal)
          updated_table = Map.put(mgr.process_table, target_pid, updated_target)

          # If the signal caused the process to become a zombie, send SIGCHLD
          # to its parent.
          final_table =
            if updated_target.process_state == :zombie do
              case Map.get(updated_table, updated_target.parent_pid) do
                nil ->
                  updated_table

                parent_pcb ->
                  {_s, updated_parent} = SignalManager.send_signal(parent_pcb, :sigchld)
                  Map.put(updated_table, updated_target.parent_pid, updated_parent)
              end
            else
              updated_table
            end

          {:ok, %{mgr | process_table: final_table}}
      end
    end

    @doc """
    Terminates a process, making it a zombie.

    1. Sets state to :zombie with the given exit code.
    2. Reparents all children to init (PID 0).
    3. Sends SIGCHLD to the parent.

    Returns `{:ok, updated_manager}` or `{:error, :not_found}`.
    """
    def exit_process(%__MODULE__{} = mgr, pid, exit_code) do
      case Map.get(mgr.process_table, pid) do
        nil ->
          {:error, :not_found}

        pcb ->
          # Step 1: Set zombie state.
          updated_pcb = %{pcb |
            process_state: :zombie,
            exit_code: exit_code,
            children: []
          }

          updated_table = Map.put(mgr.process_table, pid, updated_pcb)

          # Step 2: Reparent children to init (PID 0).
          updated_table =
            Enum.reduce(pcb.children, updated_table, fn child_pid, table ->
              case Map.get(table, child_pid) do
                nil ->
                  table

                child ->
                  updated_child = %{child | parent_pid: 0}
                  table = Map.put(table, child_pid, updated_child)

                  # Add to init's children list.
                  case Map.get(table, 0) do
                    nil ->
                      table

                    init_pcb when init_pcb.pid != pcb.pid ->
                      updated_init = %{init_pcb | children: init_pcb.children ++ [child_pid]}
                      Map.put(table, 0, updated_init)

                    _ ->
                      table
                  end
              end
            end)

          # Step 3: Send SIGCHLD to parent.
          final_table =
            case Map.get(updated_table, pcb.parent_pid) do
              nil ->
                updated_table

              parent_pcb ->
                {_s, updated_parent} = SignalManager.send_signal(parent_pcb, :sigchld)
                Map.put(updated_table, pcb.parent_pid, updated_parent)
            end

          {:ok, %{mgr | process_table: final_table}}
      end
    end

    @doc "Looks up a process by PID."
    def get_process(%__MODULE__{} = mgr, pid) do
      Map.get(mgr.process_table, pid)
    end
  end

  # ============================================================================
  # Priority Scheduler
  # ============================================================================

  # The PriorityScheduler uses 40 priority levels (0-39) with FIFO (round-robin)
  # within each level. Lower number = higher priority.
  #
  #     Ready Queues:
  #     Priority 0:  [kernel_timer]        <-- runs first
  #     Priority 5:  [keyboard_handler]    <-- runs second
  #     Priority 20: [bash, vim, firefox]  <-- round-robin among these
  #     Priority 39: [backup_daemon]       <-- runs last
  #
  # Time quantum varies by priority:
  #     Priority 0  --> 200 cycles
  #     Priority 20 --> ~123 cycles
  #     Priority 39 --> 50 cycles

  defmodule Scheduler do
    @moduledoc "Priority-based process scheduler with 40 levels."

    alias CodingAdventures.ProcessManager.PCB

    defstruct ready_queues: List.duplicate([], 40), current_process: nil

    @doc """
    Adds a process to the appropriate ready queue based on its priority.
    Sets the process state to :ready.
    """
    def enqueue(%__MODULE__{} = sched, %PCB{} = pcb) do
      priority = pcb.priority

      if priority < 0 or priority > 39 do
        raise ArgumentError, "Priority #{priority} is out of range (0-39)."
      end

      updated_pcb = %{pcb | process_state: :ready}
      updated_queues = List.update_at(sched.ready_queues, priority, &(&1 ++ [updated_pcb]))
      %{sched | ready_queues: updated_queues}
    end

    @doc """
    Selects the highest-priority (lowest number) process to run.

    Returns `{pcb, updated_scheduler}` or `{nil, scheduler}` if empty.
    """
    def schedule(%__MODULE__{} = sched) do
      result =
        Enum.find_value(0..39, fn priority ->
          queue = Enum.at(sched.ready_queues, priority)

          case queue do
            [head | tail] ->
              running_pcb = %{head | process_state: :running}
              updated_queues = List.replace_at(sched.ready_queues, priority, tail)

              updated_sched = %{sched |
                ready_queues: updated_queues,
                current_process: running_pcb
              }

              {running_pcb, updated_sched}

            _ ->
              nil
          end
        end)

      case result do
        nil -> {nil, %{sched | current_process: nil}}
        {pcb, updated_sched} -> {pcb, updated_sched}
      end
    end

    @doc """
    Preempts a process by putting it back at the end of its priority queue.
    """
    def preempt(%__MODULE__{} = sched, %PCB{} = pcb) do
      ready_pcb = %{pcb | process_state: :ready}
      enqueue(sched, ready_pcb)
    end

    @doc """
    Changes a process's priority, moving it between queues if needed.

    Returns `{updated_pcb, updated_scheduler}`.
    """
    def set_priority(%__MODULE__{} = sched, %PCB{} = pcb, new_priority) do
      if new_priority < 0 or new_priority > 39 do
        raise ArgumentError, "Priority #{new_priority} is out of range (0-39)."
      end

      old_priority = pcb.priority

      if old_priority == new_priority do
        {pcb, sched}
      else
        # Remove from old queue if present.
        old_queue = Enum.at(sched.ready_queues, old_priority)
        {found, new_old_queue} = remove_by_pid(old_queue, pcb.pid)

        updated_pcb = %{pcb | priority: new_priority}

        if found do
          updated_queues =
            sched.ready_queues
            |> List.replace_at(old_priority, new_old_queue)
            |> List.update_at(new_priority, &(&1 ++ [updated_pcb]))

          {updated_pcb, %{sched | ready_queues: updated_queues}}
        else
          {updated_pcb, sched}
        end
      end
    end

    @doc """
    Calculates the time quantum for a given priority.

    Formula: 200 - floor(priority * 150 / 39)

        Priority  0 --> 200 cycles
        Priority 20 --> 123 cycles
        Priority 39 -->  50 cycles
    """
    def get_time_quantum(priority) do
      if priority < 0 or priority > 39 do
        raise ArgumentError, "Priority #{priority} is out of range (0-39)."
      end

      200 - div(priority * 150, 39)
    end

    @doc "Returns the currently running process."
    def get_current(%__MODULE__{} = sched), do: sched.current_process

    @doc "Returns the ready queues for inspection."
    def get_ready_queues(%__MODULE__{} = sched), do: sched.ready_queues

    # Helper: remove a PCB by PID from a list, returning {found?, updated_list}.
    defp remove_by_pid(queue, pid) do
      case Enum.find_index(queue, fn p -> p.pid == pid end) do
        nil -> {false, queue}
        idx -> {true, List.delete_at(queue, idx)}
      end
    end
  end
end
