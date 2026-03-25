defmodule CodingAdventures.EventLoop do
  @moduledoc """
  A pluggable, generic event loop — the heartbeat of any interactive application.

  ## What is an event loop?

  An event loop is the outermost structure of any interactive program. It runs
  forever (until told to stop), repeatedly asking "did anything happen?" and
  dispatching whatever happened to registered handlers:

      while running:
          collect events from all sources
          for each event:
              dispatch to handlers
              if any handler says :exit → stop

  ## Why functional in Elixir?

  Unlike Go or Rust where the loop mutates struct fields, Elixir uses
  immutable values and tail recursion. The loop is a recursive function
  that passes updated source state forward on each iteration — no mutation,
  no side effects except in the handlers you provide.

  This models a fundamental Elixir pattern: evolving state through function
  calls rather than mutation.

  ## Sources in Elixir

  A source is a `{poll_fn, state}` tuple where:
  - `poll_fn` is a function of arity 1: `fn state -> {[events], new_state}`
  - `state` is any term (the source's current state)

  On each iteration the loop calls `poll_fn.(state)` and uses `new_state`
  for the next iteration. This is the standard Elixir way to handle stateful
  computation without mutation.

  ## Handlers

  A handler is a function of arity 1: `fn event -> :continue | :exit`.

  ## Quick start

      source_fn = fn count ->
        if count > 0, do: {[count], count - 1}, else: {[:done], 0}
      end

      handler = fn
        :done -> :exit
        event -> IO.inspect(event); :continue
      end

      CodingAdventures.EventLoop.run(
        [{source_fn, 3}],
        [handler]
      )

  """

  @typedoc """
  Control flow signal returned by handlers.

  - `:continue` — keep the loop running
  - `:exit`     — stop the loop after this event
  """
  @type control_flow :: :continue | :exit

  @typedoc """
  An event source: a `{poll_fn, state}` tuple.

  `poll_fn` has signature `fn state -> {[event], new_state}`.
  """
  @type source(e, s) :: {(s -> {[e], s}), s}

  @typedoc "A handler function: receives one event, returns control_flow."
  @type handler(e) :: (e -> control_flow)

  # ══════════════════════════════════════════════════════════════════════════
  # Public API
  # ══════════════════════════════════════════════════════════════════════════

  @doc """
  Run the event loop.

  `sources` is a list of `{poll_fn, initial_state}` tuples.

  `handlers` is a list of functions `fn event -> :continue | :exit`.

  Returns `:ok` when the loop exits (a handler returned `:exit`).

  ## Example

      # A source that counts down from 3, then signals done.
      source_fn = fn
        0 -> {[:done], 0}
        n -> {[n], n - 1}
      end

      CodingAdventures.EventLoop.run(
        [{source_fn, 3}],
        [fn :done -> :exit; _ -> :continue end]
      )
      #=> :ok
  """
  @spec run([source(any, any)], [handler(any)]) :: :ok
  def run(sources, handlers) do
    loop(sources, handlers)
  end

  # ══════════════════════════════════════════════════════════════════════════
  # Private loop logic
  # ══════════════════════════════════════════════════════════════════════════

  # The main loop is a tail-recursive function. Elixir (via BEAM) optimises
  # tail calls so this never overflows the stack no matter how many iterations.
  defp loop(sources, handlers) do
    # ── Phase 1: Collect ──────────────────────────────────────────────────
    #
    # Poll each source. Accumulate the events and the updated source state.
    # We use Enum.reduce/3 to thread the accumulator through the list.
    {events, updated_sources} = collect(sources)

    # ── Phase 2: Dispatch ─────────────────────────────────────────────────
    #
    # Deliver each event to all handlers in order. Stop the moment any
    # handler returns :exit.
    case dispatch_all(events, handlers) do
      :exit ->
        :ok

      :continue ->
        # ── Phase 3: Idle ──────────────────────────────────────────────────
        #
        # If nothing happened, briefly yield. Process.sleep(0) releases the
        # BEAM scheduler so other processes get CPU time. Without this an
        # idle loop would starve other Elixir processes.
        if events == [] do
          Process.sleep(0)
        end

        loop(updated_sources, handlers)
    end
  end

  # Poll all sources. Returns `{all_events, updated_sources}`.
  #
  # We use Enum.map_reduce/3 to simultaneously transform each source
  # (updating its state) and collect the events it produced.
  defp collect(sources) do
    Enum.map_reduce(sources, [], fn {poll_fn, state}, acc ->
      {new_events, new_state} = poll_fn.(state)
      {{poll_fn, new_state}, acc ++ new_events}
    end)
    |> then(fn {updated_sources, events} -> {events, updated_sources} end)
  end

  # Dispatch a list of events to all handlers. Returns :exit or :continue.
  defp dispatch_all([], _handlers), do: :continue

  defp dispatch_all([event | rest], handlers) do
    case dispatch_one(event, handlers) do
      :exit -> :exit
      :continue -> dispatch_all(rest, handlers)
    end
  end

  # Run a single event through all handlers in order.
  # Return :exit as soon as any handler says so.
  defp dispatch_one(_event, []), do: :continue

  defp dispatch_one(event, [handler | rest]) do
    case handler.(event) do
      :exit -> :exit
      :continue -> dispatch_one(event, rest)
    end
  end
end
