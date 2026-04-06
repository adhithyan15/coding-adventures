defmodule CodingAdventures.Repl.SilentWaiting do
  @moduledoc """
  SilentWaiting — the no-op waiting plugin.

  ## Philosophy: Do Nothing, Do It Well

  SilentWaiting implements the Waiting behaviour with every callback as a
  true no-op. It is the right choice when:

  1. **Evaluation is fast** — If the language evaluator responds in
     microseconds (as EchoLanguage does), any visual waiting indicator would
     appear and disappear too quickly to be useful and might even cause
     visible flickering.

  2. **Testing** — Spinner animations and timed displays complicate test
     output. SilentWaiting keeps test runs clean and deterministic.

  3. **Non-interactive use** — When the REPL is driven by piped input or used
     programmatically, visual indicators are noise.

  4. **Baseline** — It is the default waiting plugin. Any new REPL session
     starts silent and you opt in to visual feedback by swapping this out.

  ## The Tick Rate

  Even though `tick/1` is a no-op, `tick_ms/0` returns 100 ms. This controls
  how often the loop polls for task completion via `Task.yield/2`. 100 ms
  means the loop checks 10 times per second — responsive enough that a fast
  evaluator doesn't wait needlessly, slow enough to avoid busy-looping.

  ## State

  `start/0` returns `nil`. `tick/1` accepts and ignores it, returning `nil`.
  `stop/1` accepts and ignores it, returning `:ok`. There is nothing to track.
  """

  @behaviour CodingAdventures.Repl.Waiting

  # No state to initialise — return nil as a placeholder that satisfies
  # the type system and documents "this waiting plugin is stateless."
  @impl true
  def start(), do: nil

  # Nothing to animate. Accept the (nil) state and hand it back unchanged.
  # The loop will call this every tick_ms() milliseconds; we simply ignore it.
  @impl true
  def tick(_state), do: nil

  # 100 ms is the poll interval. This is the only knob that matters for
  # SilentWaiting: it controls how quickly the loop responds when eval
  # finishes between ticks.
  @impl true
  def tick_ms(), do: 100

  # Nothing to clean up. The :ok return satisfies the callback contract.
  @impl true
  def stop(_state), do: :ok
end
