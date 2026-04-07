defmodule CodingAdventures.WasmRuntime.WasiClock do
  @moduledoc """
  Behaviour for WASI clock operations.

  WASI defines four clock IDs that WASM modules can query:

      +----+---------------------------+--------------------------------------+
      | ID | Name                      | Meaning                              |
      +----+---------------------------+--------------------------------------+
      |  0 | REALTIME                  | Wall-clock time (Unix epoch, ns)     |
      |  1 | MONOTONIC                 | Monotonically increasing, never goes |
      |    |                           | backward, suitable for timing        |
      |  2 | PROCESS_CPUTIME_ID        | CPU time used by this process        |
      |  3 | THREAD_CPUTIME_ID         | CPU time used by this thread         |
      +----+---------------------------+--------------------------------------+

  ## Why a Behaviour?

  A behaviour is Elixir's way of defining an interface — like a Java
  interface or a Rust trait. Any module that says
  `@behaviour CodingAdventures.WasmRuntime.WasiClock` must implement
  all the `@callback` functions, or Elixir will warn at compile time.

  This lets us swap implementations at runtime:

  - **Tests** inject a `FakeClock` that always returns deterministic
    values, making tests reproducible regardless of wall-clock time.
  - **Production** uses `SystemClock` that delegates to the real OS.

  ## Literate Note: Why nanoseconds?

  WASI's `clock_time_get` returns nanoseconds (i64). That gives us
  2^63 / 1_000_000_000 / 3600 / 24 / 365 ≈ 292 years of headroom
  from the Unix epoch — more than sufficient for any WASM program.
  """

  @doc """
  Return the current wall-clock time in nanoseconds since Unix epoch.

  Corresponds to WASI clock ID 0 (REALTIME). May jump backward if the
  system clock is adjusted (e.g., NTP sync). Do NOT use for measuring
  elapsed time.
  """
  @callback realtime_ns() :: integer()

  @doc """
  Return the current monotonic clock time in nanoseconds.

  Corresponds to WASI clock ID 1 (MONOTONIC). Guaranteed to never
  decrease between calls, even if the wall clock is adjusted. Use this
  for measuring elapsed time.

  The absolute value is meaningless — only differences matter.
  """
  @callback monotonic_ns() :: integer()

  @doc """
  Return the resolution of the given clock in nanoseconds.

  In practice, "resolution" means the smallest time increment the clock
  can distinguish. A resolution of 1_000_000 (1 ms) means the clock
  updates at most once per millisecond.
  """
  @callback resolution_ns(clock_id :: integer()) :: integer()
end

defmodule CodingAdventures.WasmRuntime.SystemClock do
  @moduledoc """
  Production clock implementation backed by the OS.

  Uses Erlang's `:os.system_time/1` for wall-clock time and
  `:erlang.monotonic_time/1` for monotonic time. Both functions
  accept `:nanosecond` as the time unit.

  ## Erlang Time API

      :os.system_time(:nanosecond)
        → nanoseconds since Unix epoch (wall clock)
        → may jump if NTP adjusts the clock

      :erlang.monotonic_time(:nanosecond)
        → nanoseconds on the Erlang monotonic clock
        → never decreases, but the zero point is arbitrary
        → convert to POSIX time if needed:
            :erlang.system_time(:nanosecond)

  We use `:erlang.monotonic_time/1` for clock ID 1 (MONOTONIC) because
  the WASI spec only requires it never decreases — the absolute value
  doesn't matter to callers measuring elapsed time.
  """

  @behaviour CodingAdventures.WasmRuntime.WasiClock

  @impl true
  def realtime_ns(), do: :os.system_time(:nanosecond)

  @impl true
  def monotonic_ns(), do: :erlang.monotonic_time(:nanosecond)

  @doc """
  Return 1_000_000 ns (1 ms) as the clock resolution for all clock IDs.

  On modern Linux, `clock_getres(CLOCK_REALTIME, ...)` typically reports
  1 ns, but Erlang's scheduler resolution is usually 1–10 ms. Reporting
  1 ms is a safe, honest approximation.
  """
  @impl true
  def resolution_ns(_clock_id), do: 1_000_000
end
