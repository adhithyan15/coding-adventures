defmodule CodingAdventures.ProgressBar do
  @moduledoc """
  A reusable, text-based progress bar for tracking concurrent operations in the
  terminal.

  ## The postal worker analogy (GenServer edition)

  Imagine a post office with a single clerk (the GenServer process) and a mail
  slot (the process mailbox). Workers from all over town (Tasks, GenServers,
  other processes) drop letters (events) into the slot via `cast`. The clerk
  picks them up one at a time and updates the scoreboard on the wall (the
  progress bar). Because only the clerk touches the scoreboard, there is no
  confusion or conflict -- even if a hundred workers drop letters at the same
  time.

  In Go, this pattern uses channels. In Elixir, it is built into the language
  itself: every process has a mailbox, and `GenServer` provides the structured
  protocol for reading from it. The BEAM scheduler ensures fair mailbox
  delivery across thousands of lightweight processes, making this pattern
  virtually free compared to OS threads.

  ## Usage

  ### Flat mode (simple)

      {:ok, tracker} = CodingAdventures.ProgressBar.start_link(total: 21, writer: :stderr)
      CodingAdventures.ProgressBar.send_event(tracker, :started, "pkg-a")
      CodingAdventures.ProgressBar.send_event(tracker, :finished, "pkg-a", "built")
      CodingAdventures.ProgressBar.send_event(tracker, :skipped, "pkg-b")
      CodingAdventures.ProgressBar.stop(tracker)

  ### Hierarchical mode (grouped progress)

      {:ok, parent} = CodingAdventures.ProgressBar.start_link(total: 3, writer: :stderr, label: "Level")
      {:ok, child} = CodingAdventures.ProgressBar.child(parent, 7, "Package")
      CodingAdventures.ProgressBar.send_event(child, :started, "pkg-a")
      CodingAdventures.ProgressBar.send_event(child, :finished, "pkg-a", "built")
      CodingAdventures.ProgressBar.finish(child)   # advances parent by 1
      CodingAdventures.ProgressBar.stop(parent)

  ### Nil safety

  All public functions accept `nil` as the pid argument and return `:ok`
  (a no-op). This lets callers unconditionally call functions without
  nil-checking:

      tracker = nil
      CodingAdventures.ProgressBar.send_event(tracker, :started, "test")  # no-op
  """

  alias CodingAdventures.ProgressBar.Tracker

  @doc """
  Starts a new progress bar tracker process.

  ## Options

    * `:total` (required) -- the number of items to track.
    * `:writer` -- the IO device to write to. Defaults to `:stderr`.
    * `:label` -- an optional prefix label (e.g., `"Level"`). Defaults to `""`.
    * `:parent_pid` -- for internal use by `child/3`. The parent tracker pid.

  Returns `{:ok, pid}` on success.
  """
  defdelegate start_link(opts), to: Tracker

  @doc """
  Sends an event to the tracker process.

  Event types:
    * `:started` -- an item began processing (now "in-flight").
    * `:finished` -- an item completed (success or failure).
    * `:skipped` -- an item was skipped without processing.

  The optional `status` argument is only meaningful for `:finished` events
  (e.g., `"built"`, `"failed"`, `"cached"`).

  Returns `:ok`. If `pid` is `nil`, this is a no-op.
  """
  def send_event(pid, type, name, status \\ "") do
    Tracker.send_event(pid, type, name, status)
  end

  @doc """
  Creates a child tracker linked to the given parent.

  The child shares the parent's writer and start time. When the child calls
  `finish/1`, it advances the parent's completed count by 1.

  Returns `{:ok, child_pid}` or `nil` if `parent_pid` is `nil`.
  """
  defdelegate child(parent_pid, total, label), to: Tracker

  @doc """
  Marks a child tracker as complete and advances the parent.

  Stops the child GenServer and sends a `:finished` event to the parent.
  If `pid` is `nil`, this is a no-op.
  """
  defdelegate finish(pid), to: Tracker

  @doc """
  Stops the tracker, printing a final newline to preserve the last progress
  line in the terminal scrollback.

  If `pid` is `nil`, this is a no-op.
  """
  defdelegate stop(pid), to: Tracker
end
