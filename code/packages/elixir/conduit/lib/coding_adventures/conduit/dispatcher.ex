defmodule CodingAdventures.Conduit.Dispatcher do
  @moduledoc """
  GenServer that runs Elixir handlers when the Rust I/O thread asks.

  ## Why a GenServer?

  Three reasons:

  1. **A stable PID.** The Rust side needs an addressable destination for
     `enif_send`. A GenServer is the simplest way to get a PID linked to
     a supervisor, with a known restart strategy.

  2. **State container.** The struct of compiled `handlers` (a map from
     `handler_id` to anonymous function) lives in the GenServer's state.
     The dispatcher closes over it via `state.handlers`, making lookups
     O(1) and never needing to round-trip back to the Application struct.

  3. **OTP supervision integration.** A misbehaving handler that crashes
     the dispatcher gets restarted by the supervisor. The Rust side keeps
     running; in-flight requests time out (30 s) and return 500.

  ## Message protocol

  The dispatcher receives:

      {:conduit_request, slot_id, handler_id, env_map}

  It looks up the handler, runs it inside a `try/catch/rescue` triad,
  and signals completion via `Conduit.Native.respond/2` with one of:

  - `{status, headers, body}` — handler returned a response tuple
  - `{0, %{}, ""}`           — handler returned `nil` (no override)
  - `{500, %{}, message}`    — unhandled exception (Rust will re-dispatch
                                 to the error_handler if set)

  ## Concurrency

  This is intentionally a single-process serial dispatcher: messages are
  processed in arrival order. For high-throughput workloads you can
  shard via `PartitionSupervisor` (one Dispatcher per scheduler), or
  upgrade to per-request worker processes a la WEB07 (Conduit OTP).
  This single-dispatcher model is easier to reason about and matches the
  user's mental model of "filters run in registration order".
  """

  use GenServer
  require Logger

  alias CodingAdventures.Conduit.{HandlerContext, Native, Request}

  # ── Client API ────────────────────────────────────────────────────────────

  @doc """
  Start the dispatcher with the compiled `handlers` map (from Application).
  """
  @spec start_link(map) :: GenServer.on_start()
  def start_link(handlers) when is_map(handlers) do
    GenServer.start_link(__MODULE__, handlers)
  end

  # ── GenServer callbacks ───────────────────────────────────────────────────

  @impl true
  def init(handlers) do
    {:ok, %{handlers: handlers}}
  end

  @impl true
  def handle_info({:conduit_request, slot_id, handler_id, env_map}, state) do
    response = run_handler(state.handlers, handler_id, env_map)
    Native.respond(slot_id, response)
    {:noreply, state}
  end

  # Catch-all so unrelated messages don't crash the dispatcher.
  def handle_info(_other, state), do: {:noreply, state}

  # ── Handler execution ─────────────────────────────────────────────────────

  @doc false
  # Public for testability. Returns either a 3-tuple or `nil`.
  def run_handler(handlers, handler_id, env_map) do
    case Map.fetch(handlers, handler_id) do
      :error ->
        # Unknown handler ID — this should never happen if the App was
        # built correctly. Return a 500 so the route handler loop can
        # decide what to do (e.g. re-dispatch to error_handler).
        {500, %{}, "no handler registered for id #{handler_id}"}

      {:ok, fun} ->
        request = Request.from_env(env_map)

        try do
          case fun.(request) do
            nil ->
              # No override — Rust treats {0, %{}, ""} as the sentinel.
              {0, %{}, ""}

            {status, headers, body}
            when is_integer(status) and is_map(headers) and is_binary(body) ->
              {status, headers, body}

            other ->
              # Any other shape: best effort — a string body becomes a 200/text/plain.
              if is_binary(other) do
                HandlerContext.text(other)
              else
                # Log the offending shape server-side; do NOT echo it back.
                Logger.warning(fn ->
                  "Conduit handler #{handler_id} returned invalid shape: " <>
                    inspect(other, limit: 100)
                end)

                {500, %{}, "Internal Server Error"}
              end
          end
        rescue
          # Bare 500 with NO headers — sentinel to the Rust side meaning
          # "an exception was thrown; re-dispatch to error_handler if set".
          # See WEB05 lib.rs for the matching `headers.is_empty()` check.
          #
          # SECURITY: we used to put `Exception.format/3` (which includes
          # stack frames, file paths, and local-variable inspection) in the
          # response body. That leaks server internals to clients. Now we
          # log the trace at :error level and put only the exception
          # *message* in the body — which is the same string the user's
          # error_handler will receive in `req.env["conduit.error"]`.
          e ->
            stack = __STACKTRACE__
            Logger.error(fn -> Exception.format(:error, e, stack) end)
            {500, %{}, Exception.message(e)}
        catch
          :throw, {:conduit_halt, status, body, headers}
          when is_integer(status) and is_binary(body) and is_map(headers) ->
            {status, headers, body}

          kind, reason ->
            stack = __STACKTRACE__
            Logger.error(fn -> Exception.format(kind, reason, stack) end)
            {500, %{}, short_error_message(kind, reason)}
        end
    end
  end

  # short_error_message — single-line, non-leaky description of a non-rescue
  # error. Stack traces are logged separately; this string is what goes into
  # the response body and the `conduit.error` env entry.
  defp short_error_message(:throw, value), do: "throw: " <> inspect(value, limit: 50)
  defp short_error_message(:exit, reason), do: "exit: " <> inspect(reason, limit: 50)
  defp short_error_message(_kind, _reason), do: "Internal Server Error"
end
