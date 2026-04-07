defmodule CodingAdventures.Repl.Waiting do
  @moduledoc """
  The Waiting behaviour — what the REPL displays while evaluation is in progress.

  ## The Problem: Async Evaluation

  When the REPL hands input to the language evaluator, it does so
  asynchronously (via `Task.async`). This means the main loop is free
  to *do something* while it waits for the result.

  That "something" is controlled by the Waiting plugin.

  ## Use Cases

  - **SilentWaiting** (the default) — do nothing. Simply block until eval
    completes. Good for fast languages where latency is imperceptible.

  - **SpinnerWaiting** — update a spinning ASCII animation (⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏)
    on the terminal. Good for slow evaluators like network calls.

  - **StatusWaiting** — print "Thinking..." or a progress percentage.

  - **BenchmarkWaiting** — record a start timestamp in `start()`, then in
    `stop()` print how long evaluation took.

  ## The Tick Model

  The loop calls:

  1. `start()` → get initial `state`
  2. `tick(state)` → advance animation, get new `state` (repeat every tick_ms)
  3. `stop(state)` → clean up (hide spinner, print elapsed time, etc.)

  The loop interleaves ticks with `Task.yield(task, tick_ms)` calls so that
  the tick interval is approximately honoured without sleeping past a result.

  ## Implementing a Waiting Plugin

  ```elixir
  defmodule SpinnerWaiting do
    @behaviour CodingAdventures.Repl.Waiting
    @frames ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]

    @impl true
    def start(), do: {0, @frames}

    @impl true
    def tick({idx, frames}) do
      frame = Enum.at(frames, rem(idx, length(frames)))
      IO.write("\\r\#{frame} ")
      {idx + 1, frames}
    end

    @impl true
    def tick_ms(), do: 80

    @impl true
    def stop(_state) do
      IO.write("\\r  \\r")  # erase the spinner
      :ok
    end
  end
  ```
  """

  @doc """
  Called once when the evaluator task is started.

  Return any state your plugin needs. The state is opaque to the loop —
  it will be passed back to `tick/1` and `stop/1` unchanged.

  Typical use: record `System.monotonic_time()` for elapsed-time tracking,
  or return `0` as a frame counter for a spinner.
  """
  @callback start() :: term()

  @doc """
  Called repeatedly, approximately every `tick_ms/0` milliseconds, while
  evaluation is in progress.

  Receives the current state and returns the next state. The loop replaces
  its state reference with the returned value on every call.

  Typical use: advance a spinner frame, update a progress display.
  """
  @callback tick(state :: term()) :: term()

  @doc """
  The interval in milliseconds between successive `tick/1` calls.

  The loop uses `Task.yield(task, tick_ms())` to wait for this duration
  before calling `tick/1` again. Lower values = smoother animation;
  higher values = less CPU churn.

  Must be a positive integer. 100 ms (10 ticks/sec) is a reasonable default.
  """
  @callback tick_ms() :: pos_integer()

  @doc """
  Called once when evaluation completes (or fails).

  Receives the final state. Should clean up any visual artifacts left by
  `tick/1` (erase spinner lines, hide the cursor, etc.). Must return `:ok`.
  """
  @callback stop(state :: term()) :: :ok
end
