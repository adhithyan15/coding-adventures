defmodule CodingAdventures.ProgressBar.Tracker do
  @moduledoc """
  A GenServer that receives events from concurrent processes and renders a
  text-based progress bar to an IO device.

  ## The GenServer as postal clerk

  In the "postal worker" analogy, Go uses a buffered channel as the mail slot
  and a goroutine as the clerk. Elixir takes this further: every process
  already *has* a mailbox (built into the BEAM VM), and `GenServer` provides
  a structured protocol for reading from it.

  The mapping is:

      Go concept              Elixir equivalent
      ─────────────────────   ────────────────────────────
      buffered channel        process mailbox (unbounded)
      goroutine               GenServer process
      channel send (`ch <-`)  GenServer.cast (async)
      channel close + <-done  GenServer.stop (sync)

  Because the BEAM scheduler preemptively switches between lightweight
  processes (not OS threads), we get concurrency-safe progress tracking
  without locks, mutexes, or explicit channel management. The mailbox
  *is* the channel.

  ## State machine

  The tracker maintains a simple state machine for each tracked item:

      Event      | completed | building (MapSet)
      ───────────┼───────────┼──────────────────
      :started   | unchanged | add name
      :finished  | +1        | remove name
      :skipped   | +1        | unchanged

  ## Rendering

  After every state change, the tracker redraws the progress bar using `\\r`
  (carriage return) to overwrite the current line. The bar is 20 characters
  wide, using Unicode block characters:

      █ (U+2588) -- filled portion
      ░ (U+2591) -- empty portion

  Example output:

      [████████░░░░░░░░░░░░]  7/21  Building: pkg-a, pkg-b  (12.3s)

  In hierarchical mode (with a parent), the parent's label and count are
  prepended:

      Level 2/3  [████░░░░░░░░░░░░░░░░]  5/12  Building: pkg-a  (8.2s)
  """

  use GenServer

  # ---------------------------------------------------------------------------
  # Event struct
  # ---------------------------------------------------------------------------

  @doc """
  An event that workers send to the tracker.

  Fields:
    * `type` -- what happened (`:started`, `:finished`, `:skipped`)
    * `name` -- human-readable identifier (e.g., `"python/logic-gates"`)
    * `status` -- outcome label, only meaningful for `:finished` events
                  (e.g., `"built"`, `"failed"`, `"cached"`)
  """
  defmodule Event do
    @moduledoc """
    A minimal event struct representing something that happened to a tracked
    item.

    Think of it like a traffic light:

        :started  = green  (item is actively being processed)
        :finished = red    (item is done -- success or failure)
        :skipped  = yellow (item was bypassed without processing)
    """

    @enforce_keys [:type, :name]
    defstruct [:type, :name, status: ""]

    @type t :: %__MODULE__{
            type: :started | :finished | :skipped,
            name: String.t(),
            status: String.t()
          }
  end

  # ---------------------------------------------------------------------------
  # Internal state
  # ---------------------------------------------------------------------------
  #
  # The GenServer's state is a simple map. Every field is set at init time
  # and updated only inside handle_cast -- guaranteeing single-writer
  # semantics without any explicit locking.

  defmodule State do
    @moduledoc false

    defstruct [
      :total,
      :writer,
      :label,
      :start_time,
      :parent_pid,
      completed: 0,
      building: MapSet.new()
    ]
  end

  # ---------------------------------------------------------------------------
  # Public API
  # ---------------------------------------------------------------------------

  @doc """
  Starts a new Tracker GenServer process.

  ## Options

    * `:total` (required) -- the number of items to track.
    * `:writer` -- the IO device to write to. Defaults to `:stderr`.
    * `:label` -- an optional prefix label (e.g., `"Level"`). Defaults to `""`.
    * `:parent_pid` -- the parent tracker pid (used by `child/3`).
    * `:start_time` -- the start time (used by `child/3` to share parent's time).

  ## Example

      {:ok, pid} = Tracker.start_link(total: 10, writer: :stderr, label: "Build")
  """
  def start_link(opts) do
    GenServer.start_link(__MODULE__, opts)
  end

  @doc """
  Sends an event to the tracker via `GenServer.cast`.

  This is the "drop a letter in the mail slot" operation. It is asynchronous
  and returns immediately -- the tracker will process the event in its own
  time.

  If `pid` is `nil`, this is a no-op. This is a deliberate design choice:
  callers can unconditionally call `send_event` without nil-checking, which
  keeps integration code clean.

  ## Parameters

    * `pid` -- the tracker process (or `nil` for no-op)
    * `type` -- `:started`, `:finished`, or `:skipped`
    * `name` -- the item name (e.g., `"pkg-a"`)
    * `status` -- outcome label for `:finished` events (default: `""`)

  ## Example

      send_event(tracker, :started, "pkg-a")
      send_event(tracker, :finished, "pkg-a", "built")
      send_event(tracker, :skipped, "pkg-b")
  """
  def send_event(pid, type, name, status \\ "")

  def send_event(nil, _type, _name, _status), do: :ok

  def send_event(pid, type, name, status) do
    GenServer.cast(pid, {:event, %Event{type: type, name: name, status: status}})
  end

  @doc """
  Creates a child tracker linked to the given parent.

  The child shares the parent's writer and start time. When `finish/1` is
  called on the child, it sends a `:finished` event back to the parent,
  advancing the parent's completed count by one.

  This enables hierarchical progress -- for example, a build system with
  3 dependency levels, each containing N packages:

      parent tracks levels  (total=3,  label="Level")
      child tracks packages (total=N, label="Package")

  Display: `Level 1/3  [████░░░░]  3/7  Building: pkg-a  (2.1s)`

  If `parent_pid` is `nil`, returns `nil` (no-op).
  """
  def child(nil, _total, _label), do: nil

  def child(parent_pid, total, label) do
    # Ask the parent for its writer and start_time so the child can share them.
    {writer, start_time} = GenServer.call(parent_pid, :get_child_info)

    start_link(
      total: total,
      writer: writer,
      label: label,
      parent_pid: parent_pid,
      start_time: start_time
    )
  end

  @doc """
  Marks a child tracker as complete and advances the parent.

  This stops the child GenServer (which prints a final draw) and then
  sends a `:finished` event to the parent tracker.

  If `pid` is `nil`, this is a no-op.
  """
  def finish(nil), do: :ok

  def finish(pid) do
    # Get parent_pid before stopping
    parent_pid = GenServer.call(pid, :get_parent_pid)
    GenServer.stop(pid, :normal)

    if parent_pid do
      send_event(parent_pid, :finished, "child")
    end

    :ok
  end

  @doc """
  Stops the tracker, printing a final newline so the last progress line
  is preserved in the terminal scrollback.

  If `pid` is `nil`, this is a no-op.
  """
  def stop(nil), do: :ok

  def stop(pid) do
    writer = GenServer.call(pid, :get_writer)
    GenServer.stop(pid, :normal)
    IO.write(writer, "\n")
    :ok
  end

  # ---------------------------------------------------------------------------
  # GenServer callbacks
  # ---------------------------------------------------------------------------

  @doc false
  @impl true
  def init(opts) do
    state = %State{
      total: Keyword.fetch!(opts, :total),
      writer: Keyword.get(opts, :writer, :stderr),
      label: Keyword.get(opts, :label, ""),
      start_time: Keyword.get(opts, :start_time, System.monotonic_time(:millisecond)),
      parent_pid: Keyword.get(opts, :parent_pid)
    }

    {:ok, state}
  end

  @doc """
  Handles asynchronous event messages.

  This is the core of the "postal clerk" loop. Each cast delivers one event,
  and we:

    1. Update the state (completed count, building set)
    2. Redraw the progress bar

  Because GenServer processes messages sequentially from its mailbox, there
  are no race conditions -- even if hundreds of processes send events
  simultaneously.
  """
  @impl true
  def handle_cast({:event, %Event{type: :started, name: name}}, state) do
    new_state = %{state | building: MapSet.put(state.building, name)}
    draw(new_state)
    {:noreply, new_state}
  end

  def handle_cast({:event, %Event{type: :finished, name: name}}, state) do
    new_state = %{
      state
      | completed: state.completed + 1,
        building: MapSet.delete(state.building, name)
    }

    draw(new_state)
    {:noreply, new_state}
  end

  def handle_cast({:event, %Event{type: :skipped}}, state) do
    new_state = %{state | completed: state.completed + 1}
    draw(new_state)
    {:noreply, new_state}
  end

  @doc false
  @impl true
  def handle_call(:get_child_info, _from, state) do
    {:reply, {state.writer, state.start_time}, state}
  end

  def handle_call(:get_parent_pid, _from, state) do
    {:reply, state.parent_pid, state}
  end

  def handle_call(:get_writer, _from, state) do
    {:reply, state.writer, state}
  end

  def handle_call(:get_state, _from, state) do
    {:reply, state, state}
  end

  @doc """
  When the GenServer terminates (via `stop/1` or `finish/1`), we draw
  one final time to ensure the bar shows the final state.
  """
  @impl true
  def terminate(_reason, state) do
    draw(state)
    :ok
  end

  # ---------------------------------------------------------------------------
  # Rendering
  # ---------------------------------------------------------------------------
  #
  # The draw function composes one progress line and writes it to the IO
  # device. It uses \r (carriage return) to overwrite the current line --
  # this works on all platforms (Windows cmd, PowerShell, Git Bash, Unix
  # terminals) without needing ANSI escape codes.

  @bar_width 20

  @doc false
  defp draw(state) do
    elapsed = elapsed_seconds(state.start_time)
    bar = render_bar(state.completed, state.total)
    activity = format_activity(state.building, state.completed, state.total)

    line =
      if state.parent_pid do
        # Hierarchical mode: show parent's label and count.
        # We query the parent's state to get its label and progress.
        # If the parent is gone, fall back to flat mode.
        case safe_call(state.parent_pid, :get_state) do
          {:ok, parent_state} ->
            parent_completed = parent_state.completed + 1

            "\r#{parent_state.label} #{parent_completed}/#{parent_state.total}  [#{bar}]  #{state.completed}/#{state.total}  #{activity}  (#{elapsed}s)"

          :error ->
            "\r[#{bar}]  #{state.completed}/#{state.total}  #{activity}  (#{elapsed}s)"
        end
      else
        if state.label != "" do
          # Labeled flat tracker (used as parent -- shows own state).
          "\r#{state.label} #{state.completed}/#{state.total}  [#{bar}]  #{activity}  (#{elapsed}s)"
        else
          # Flat mode: just the bar.
          "\r[#{bar}]  #{state.completed}/#{state.total}  #{activity}  (#{elapsed}s)"
        end
      end

    # Pad to 80 characters to overwrite any previous longer line.
    padded = String.pad_trailing(line, 80)
    IO.write(state.writer, padded)
  end

  # ---------------------------------------------------------------------------
  # Bar rendering
  # ---------------------------------------------------------------------------
  #
  # The bar is 20 characters wide. The number of filled characters is
  # proportional to completed/total:
  #
  #     filled = (completed * 20) / total
  #
  # Integer division naturally rounds down, so the bar only shows 100%
  # when all items are truly complete.
  #
  # We use Unicode block characters:
  #
  #     █ (U+2588) -- filled portion
  #     ░ (U+2591) -- empty portion

  defp render_bar(completed, total) when total <= 0 do
    String.duplicate("\u2591", @bar_width)
  end

  defp render_bar(completed, total) do
    filled = min(div(completed * @bar_width, total), @bar_width)
    String.duplicate("\u2588", filled) <> String.duplicate("\u2591", @bar_width - filled)
  end

  # ---------------------------------------------------------------------------
  # Activity formatting
  # ---------------------------------------------------------------------------
  #
  # Builds the "Building: pkg-a, pkg-b" or "waiting..." or "done" string
  # from the current in-flight set.
  #
  # The rules (truth table):
  #
  #     | In-flight count | Completed vs Total | Output                      |
  #     |─────────────────|────────────────────|─────────────────────────────|
  #     | 0               | completed < total  | "waiting..."                |
  #     | 0               | completed >= total | "done"                      |
  #     | 1-3             | any                | "Building: a, b, c"         |
  #     | 4+              | any                | "Building: a, b, c +N more" |

  @max_names 3

  defp format_activity(building, completed, total) do
    names = building |> MapSet.to_list() |> Enum.sort()

    case length(names) do
      0 ->
        if completed >= total, do: "done", else: "waiting..."

      n when n <= @max_names ->
        "Building: " <> Enum.join(names, ", ")

      n ->
        shown = names |> Enum.take(@max_names) |> Enum.join(", ")
        "Building: #{shown} +#{n - @max_names} more"
    end
  end

  # ---------------------------------------------------------------------------
  # Helpers
  # ---------------------------------------------------------------------------

  defp elapsed_seconds(start_time) do
    now = System.monotonic_time(:millisecond)
    diff_ms = now - start_time
    :erlang.float_to_binary(diff_ms / 1000.0, decimals: 1)
  end

  defp safe_call(pid, msg) do
    try do
      {:ok, GenServer.call(pid, msg, 100)}
    catch
      :exit, _ -> :error
    end
  end
end
