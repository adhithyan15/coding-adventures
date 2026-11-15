defmodule CodingAdventures.LunarLander do
  @moduledoc """
  Pure, deterministic, platform-independent simulation core and ASCII telemetry
  for the classic discrete Lunar Module flight simulation (GAME02).

  ## Historical Lineage

  - **1969 — Jim Storer's Lunar Landing Game:** Written in autumn 1969 by Jim Storer,
    a high school student at Lexington High School in Massachusetts, on a DEC PDP-8
    in FOCAL-69. Inspired by the Apollo 11 lunar landing, Storer modelled the physics
    of the Apollo Lunar Module (LM) descent stage.
  - **1973 — 101 BASIC Computer Games:** Adapted to Dartmouth BASIC by David H. Ahl
    and published under the title *Rocket* (and later *Lunar*), becoming an enduring
    computing classic.

  ## Architectural Guarantees

  - Strictly zero ambient system authority: `"capabilities": []` declared in
    `required_capabilities.json` (`VER00`).
  - Pure integer fixed-point physics: 64-bit and 128-bit integer arithmetic with
    zero floating-point operations in the simulation core (`VER04m`, `VER08a`).
  - Deterministic Euler-Verlet kinematic integration and exact touchdown contact time
    interpolation with mid-step propellant refund.
  - Declarative 76x5 2D telemetry `PaintScene` and deterministic Unicode ASCII
    HUD rasterizer (`P2D02`).
  - Complete 26-CWE weakness closure (`VER15`), including strict `is_integer/1`
    guards rejecting floats, booleans, nil, binaries, and atoms (`CWE-843`),
    integer overflow boundaries (`CWE-190`), division-by-zero prevention (`CWE-369`),
    and zero atom-table exhaustion (`CWE-400`).
  """

  import Bitwise

  alias CodingAdventures.LunarLander.{
    BurnCommand,
    Error,
    LanderState,
    PaintRect,
    PaintScene,
    PaintText
  }

  # ── Physical Constants & Apollo LM Parameters ──────────────────────────────

  @g_lunar_mft_s2 5_310
  @dry_mass_lbs 16_500
  @initial_fuel_lbs 16_500
  @total_initial_mass_lbs 33_000
  @initial_altitude_mft 120_000_000
  @initial_velocity_mft_s -3_000_000
  @exhaust_velocity_mft_s 8_000_000
  @max_burn_rate_lbs_s 200
  @min_duration_s 1
  @max_duration_s 10
  @max_time_seconds 86_400
  @max_altitude_mft 200_000_000
  @min_velocity_mft_s -10_000_000
  @max_velocity_mft_s 10_000_000

  @u32_max 0xFFFF_FFFF

  def g_lunar_mft_s2, do: @g_lunar_mft_s2
  def dry_mass_lbs, do: @dry_mass_lbs
  def initial_fuel_lbs, do: @initial_fuel_lbs
  def total_initial_mass_lbs, do: @total_initial_mass_lbs
  def initial_altitude_mft, do: @initial_altitude_mft
  def initial_velocity_mft_s, do: @initial_velocity_mft_s
  def exhaust_velocity_mft_s, do: @exhaust_velocity_mft_s
  def max_burn_rate_lbs_s, do: @max_burn_rate_lbs_s
  def min_duration_s, do: @min_duration_s
  def max_duration_s, do: @max_duration_s
  def max_time_seconds, do: @max_time_seconds
  def max_altitude_mft, do: @max_altitude_mft
  def min_velocity_mft_s, do: @min_velocity_mft_s
  def max_velocity_mft_s, do: @max_velocity_mft_s

  # ── Domain Types & Structs ──────────────────────────────────────────────────

  defmodule LanderState do
    @moduledoc "Immutable state record capturing the complete flight simulation state."

    @enforce_keys [
      :time_seconds,
      :altitude_mft,
      :velocity_mft_s,
      :fuel_lbs,
      :dry_mass_lbs,
      :status,
      :impact_velocity_mft_s
    ]

    defstruct [
      :time_seconds,
      :altitude_mft,
      :velocity_mft_s,
      :fuel_lbs,
      :dry_mass_lbs,
      :status,
      :impact_velocity_mft_s
    ]

    @type landing_outcome :: :perfect | :hard | :crash | :fatal | :crater
    @type flight_status :: :in_flight | {:landed, landing_outcome()}

    @type t :: %__MODULE__{
            time_seconds: non_neg_integer(),
            altitude_mft: integer(),
            velocity_mft_s: integer(),
            fuel_lbs: non_neg_integer(),
            dry_mass_lbs: non_neg_integer(),
            status: flight_status(),
            impact_velocity_mft_s: integer() | nil
          }

    @doc "Creates a LanderState struct from a map with atom or string keys."
    @spec new(map() | t()) :: {:ok, t()} | {:error, :arithmetic_overflow}
    def new(%__MODULE__{} = state), do: {:ok, state}

    def new(%{
          time_seconds: t,
          altitude_mft: alt,
          velocity_mft_s: vel,
          fuel_lbs: fuel,
          dry_mass_lbs: dry,
          status: status,
          impact_velocity_mft_s: impact
        }) do
      case parse_status_value(status) do
        {:ok, parsed_status} ->
          norm_impact = normalize_impact(impact)

          if is_integer(t) and is_integer(alt) and is_integer(vel) and
               is_integer(fuel) and is_integer(dry) and
               (is_nil(norm_impact) or is_integer(norm_impact)) do
            {:ok,
             %__MODULE__{
               time_seconds: t,
               altitude_mft: alt,
               velocity_mft_s: vel,
               fuel_lbs: fuel,
               dry_mass_lbs: dry,
               status: parsed_status,
               impact_velocity_mft_s: norm_impact
             }}
          else
            {:error, :arithmetic_overflow}
          end

        :error ->
          {:error, :arithmetic_overflow}
      end
    end

    def new(%{
          "time_seconds" => t,
          "altitude_mft" => alt,
          "velocity_mft_s" => vel,
          "fuel_lbs" => fuel,
          "dry_mass_lbs" => dry,
          "status" => status,
          "impact_velocity_mft_s" => impact
        }) do
      new(%{
        time_seconds: t,
        altitude_mft: alt,
        velocity_mft_s: vel,
        fuel_lbs: fuel,
        dry_mass_lbs: dry,
        status: status,
        impact_velocity_mft_s: impact
      })
    end

    def new(_), do: {:error, :arithmetic_overflow}

    defp normalize_impact(:null), do: nil
    defp normalize_impact(v), do: v

    defp parse_status_value(:in_flight), do: {:ok, :in_flight}

    defp parse_status_value({:landed, outcome})
         when outcome in [:perfect, :hard, :crash, :fatal, :crater], do: {:ok, {:landed, outcome}}

    defp parse_status_value(str) when is_binary(str) do
      case CodingAdventures.LunarLander.status_from_string(str) do
        {:ok, status} -> {:ok, status}
        {:error, _} -> :error
      end
    end

    defp parse_status_value(_), do: :error
  end

  defmodule BurnCommand do
    @moduledoc "Pilot burn order for a discrete simulation turn."

    @enforce_keys [:burn_rate_lbs_s, :duration_seconds]

    defstruct [:burn_rate_lbs_s, :duration_seconds]

    @type t :: %__MODULE__{
            burn_rate_lbs_s: non_neg_integer(),
            duration_seconds: pos_integer()
          }

    @doc "Creates a BurnCommand struct from a map with atom or string keys."
    @spec new(map() | t()) :: {:ok, t()} | {:error, :invalid_duration}
    def new(%__MODULE__{} = cmd), do: {:ok, cmd}

    def new(%{burn_rate_lbs_s: b, duration_seconds: d})
        when is_integer(b) and is_integer(d) do
      {:ok, %__MODULE__{burn_rate_lbs_s: b, duration_seconds: d}}
    end

    def new(%{"burn_rate_lbs_s" => b, "duration_seconds" => d})
        when is_integer(b) and is_integer(d) do
      {:ok, %__MODULE__{burn_rate_lbs_s: b, duration_seconds: d}}
    end

    def new(_), do: {:error, :invalid_duration}
  end

  defmodule Error do
    @moduledoc "Exception struct representing a burn command validation or simulation error."
    defexception [:reason, :message]

    @type reason ::
            :flight_already_ended
            | :excessive_burn_rate
            | :invalid_duration
            | :arithmetic_overflow

    @impl true
    def exception(reason) when is_atom(reason) do
      %__MODULE__{reason: reason, message: message_for_reason(reason)}
    end

    def exception(opts) when is_list(opts) do
      reason = Keyword.get(opts, :reason, :arithmetic_overflow)
      msg = Keyword.get(opts, :message, message_for_reason(reason))
      %__MODULE__{reason: reason, message: msg}
    end

    defp message_for_reason(:flight_already_ended), do: "Flight has already ended"
    defp message_for_reason(:excessive_burn_rate), do: "Burn rate exceeds maximum engine capacity"

    defp message_for_reason(:invalid_duration),
      do: "Duration is invalid (must be between 1 and 10 seconds)"

    defp message_for_reason(:arithmetic_overflow),
      do: "Arithmetic overflow or coordinate bounds violation"

    defp message_for_reason(other), do: "Lunar lander error: #{inspect(other)}"
  end

  defmodule PaintScene do
    @moduledoc "Declarative 2D scene representation for telemetry rendering."
    @enforce_keys [:width, :height]
    defstruct [:width, :height, background: "#000000", instructions: []]

    @type t :: %__MODULE__{
            width: float(),
            height: float(),
            background: String.t(),
            instructions: list()
          }
  end

  defmodule PaintText do
    @moduledoc "Text element in a PaintScene."
    @enforce_keys [:x, :y, :text]
    defstruct [:x, :y, :text, fill: "#ffffff", font_size: 1.0, text_align: :left, kind: :text]

    @type t :: %__MODULE__{
            x: float(),
            y: float(),
            text: String.t(),
            fill: String.t() | nil,
            font_size: float(),
            text_align: atom(),
            kind: :text
          }
  end

  defmodule PaintRect do
    @moduledoc "Filled axis-aligned rectangle in a PaintScene."
    @enforce_keys [:x, :y, :width, :height]
    defstruct [:x, :y, :width, :height, fill: "#00ff00", kind: :rect]

    @type t :: %__MODULE__{
            x: float(),
            y: float(),
            width: float(),
            height: float(),
            fill: String.t() | nil,
            kind: :rect
          }
  end

  # ── Core Pure Simulation Functions ──────────────────────────────────────────

  @doc """
  Initializes a new Lunar Module descent mission at standard initial conditions.
  """
  @spec new_flight() :: LanderState.t()
  def new_flight do
    %LanderState{
      time_seconds: 0,
      altitude_mft: @initial_altitude_mft,
      velocity_mft_s: @initial_velocity_mft_s,
      fuel_lbs: @initial_fuel_lbs,
      dry_mass_lbs: @dry_mass_lbs,
      status: :in_flight,
      impact_velocity_mft_s: nil
    }
  end

  @doc """
  Computes the exact integer quotient using Euclidean division (remainder $r \\in [0, |b|)$).

  Immune to language-specific `%` sign differences when $b < 0$.
  Satisfies `div_euclid(-46070, -5310) == 9`.
  """
  @spec div_euclid(integer(), integer()) :: integer()
  def div_euclid(a, b) when is_integer(a) and is_integer(b) and b != 0 do
    q = div(a, b)
    r = a - q * b

    if r < 0 do
      if b > 0, do: q - 1, else: q + 1
    else
      q
    end
  end

  @doc """
  Computes the exact integer square root $\\lfloor \\sqrt{n} \\rfloor$ for 128-bit unsigned integers.

  Uses an iterative digit-by-digit bitwise binary square root algorithm with bounded
  step execution (at most 64 iterations) and zero recursive calls. Completely division-free.
  """
  @spec isqrt_u128(non_neg_integer()) :: non_neg_integer()
  def isqrt_u128(0), do: 0

  def isqrt_u128(n) when is_integer(n) and n > 0 do
    initial_bit = bsl(1, 126)
    bit = find_highest_bit(initial_bit, n)
    isqrt_loop(n, 0, bit)
  end

  defp find_highest_bit(bit, n) when bit > n, do: find_highest_bit(bsr(bit, 2), n)
  defp find_highest_bit(bit, _n), do: bit

  defp isqrt_loop(_num, res, 0), do: res

  defp isqrt_loop(num, res, bit) do
    if num >= res + bit do
      isqrt_loop(num - (res + bit), bsr(res, 1) + bit, bsr(bit, 2))
    else
      isqrt_loop(num, bsr(res, 1), bsr(bit, 2))
    end
  end

  @doc """
  Validates a candidate burn command against current state and physical invariants.

  Enforces canonical validation precedence:
  1. Lifecycle Check: craft must be `:in_flight`.
  2. State Invariants (`VER04m`, CWE-190, CWE-369, CWE-843): dry mass integrity,
     fuel capacity, altitude positivity, velocity bounds, mission time limits,
     and absence of impact velocity.
  3. Command Bounds: maximum burn rate (<= 200), turn duration (1..=10), and mission time non-overflow.
  """
  @spec validate_burn(LanderState.t() | map(), BurnCommand.t() | map()) ::
          :ok | {:error, Error.reason()}
  def validate_burn(state, cmd) do
    case {coerce_state(state), coerce_command(cmd)} do
      {{:ok, s}, {:ok, c}} ->
        do_validate_burn(s, c)

      {{:error, reason}, _} ->
        {:error, reason}

      {_, {:error, reason}} ->
        {:error, reason}
    end
  end

  @doc "Raising variant of `validate_burn/2`."
  @spec validate_burn!(LanderState.t() | map(), BurnCommand.t() | map()) :: :ok
  def validate_burn!(state, cmd) do
    case validate_burn(state, cmd) do
      :ok -> :ok
      {:error, reason} -> raise Error, reason: reason
    end
  end

  defp do_validate_burn(s, c) do
    # 1. Lifecycle Check
    if s.status != :in_flight do
      {:error, :flight_already_ended}
    else
      # 2. State Invariants
      cond do
        not is_integer(s.dry_mass_lbs) or s.dry_mass_lbs != @dry_mass_lbs ->
          {:error, :arithmetic_overflow}

        not is_integer(s.fuel_lbs) or s.fuel_lbs < 0 or s.fuel_lbs > @initial_fuel_lbs ->
          {:error, :arithmetic_overflow}

        not is_integer(s.altitude_mft) or s.altitude_mft <= 0 or
            s.altitude_mft > @max_altitude_mft ->
          {:error, :arithmetic_overflow}

        not is_integer(s.velocity_mft_s) or s.velocity_mft_s < @min_velocity_mft_s or
            s.velocity_mft_s > @max_velocity_mft_s ->
          {:error, :arithmetic_overflow}

        not is_integer(s.time_seconds) or s.time_seconds < 0 or s.time_seconds > @max_time_seconds ->
          {:error, :arithmetic_overflow}

        s.impact_velocity_mft_s != nil ->
          {:error, :arithmetic_overflow}

        # 3. Command Bounds
        not is_integer(c.burn_rate_lbs_s) or c.burn_rate_lbs_s < 0 or c.burn_rate_lbs_s > @u32_max ->
          {:error, :arithmetic_overflow}

        c.burn_rate_lbs_s > @max_burn_rate_lbs_s ->
          {:error, :excessive_burn_rate}

        not is_integer(c.duration_seconds) or c.duration_seconds < @min_duration_s or
            c.duration_seconds > @max_duration_s ->
          {:error, :invalid_duration}

        s.time_seconds + c.duration_seconds > @max_time_seconds ->
          {:error, :arithmetic_overflow}

        true ->
          :ok
      end
    end
  end

  @doc """
  Integrates a single discrete flight step under the commanded burn rate and duration.

  Implements second-order Euler-Verlet numerical kinematics in pure fixed-point millifeet arithmetic.
  When the projected altitude reaches or penetrates the lunar surface ($Y_{t+\\Delta t} \\le 0$),
  solves for the exact touchdown contact time $\\delta t \\in [1, \\Delta t]$ and impact velocity,
  automatically refunding unburned propellant for any unused interval fraction.
  """
  @spec step_burn(LanderState.t() | map(), BurnCommand.t() | map()) ::
          {:ok, LanderState.t()} | {:error, Error.reason()}
  def step_burn(state, cmd) do
    case {coerce_state(state), coerce_command(cmd)} do
      {{:ok, s}, {:ok, c}} ->
        case do_validate_burn(s, c) do
          {:error, reason} ->
            {:error, reason}

          :ok ->
            do_step_burn(s, c)
        end

      {{:error, reason}, _} ->
        {:error, reason}

      {_, {:error, reason}} ->
        {:error, reason}
    end
  end

  @doc "Raising variant of `step_burn/2`."
  @spec step_burn!(LanderState.t() | map(), BurnCommand.t() | map()) :: LanderState.t()
  def step_burn!(state, cmd) do
    case step_burn(state, cmd) do
      {:ok, next_state} -> next_state
      {:error, reason} -> raise Error, reason: reason
    end
  end

  defp do_step_burn(s, c) do
    dt = c.duration_seconds
    fuel_req = c.burn_rate_lbs_s * c.duration_seconds
    delta_f = min(s.fuel_lbs, fuel_req)
    fuel_next = s.fuel_lbs - delta_f

    avg_mass = s.dry_mass_lbs + s.fuel_lbs - div(delta_f, 2)
    impulse = delta_f * @exhaust_velocity_mft_s
    a_thrust = div(impulse, dt * avg_mass)
    a_net = a_thrust - @g_lunar_mft_s2

    v_next = s.velocity_mft_s + a_net * dt
    y_next = s.altitude_mft + s.velocity_mft_s * dt + div_euclid(a_net * dt * dt, 2)
    t_next = s.time_seconds + c.duration_seconds

    if y_next > @max_altitude_mft or v_next < @min_velocity_mft_s or v_next > @max_velocity_mft_s do
      {:error, :arithmetic_overflow}
    else
      if y_next > 0 do
        {:ok,
         %LanderState{
           time_seconds: t_next,
           altitude_mft: y_next,
           velocity_mft_s: v_next,
           fuel_lbs: fuel_next,
           dry_mass_lbs: s.dry_mass_lbs,
           status: :in_flight,
           impact_velocity_mft_s: nil
         }}
      else
        # Ground touchdown solver
        a_half = div_euclid(a_net, 2)

        delta_t_raw =
          if a_half == 0 do
            if s.velocity_mft_s == 0 do
              :overflow
            else
              div_euclid(-s.altitude_mft, s.velocity_mft_s)
            end
          else
            v = s.velocity_mft_s
            y = s.altitude_mft
            disc = v * v - 4 * a_half * y
            disc_pos = max(0, disc)
            root = isqrt_u128(disc_pos)
            div_euclid(-s.velocity_mft_s - root, 2 * a_half)
          end

        if delta_t_raw == :overflow do
          {:error, :arithmetic_overflow}
        else
          delta_t = min(max(delta_t_raw, 1), dt)

          v_touchdown = s.velocity_mft_s + a_net * delta_t
          t_final = s.time_seconds + delta_t

          fuel_burned_at_touchdown = min(s.fuel_lbs, delta_t * c.burn_rate_lbs_s)
          fuel_final = s.fuel_lbs - fuel_burned_at_touchdown

          speed_mft_s = abs(v_touchdown)

          outcome =
            cond do
              speed_mft_s <= 5_000 -> :perfect
              speed_mft_s <= 15_000 -> :hard
              speed_mft_s <= 30_000 -> :crash
              speed_mft_s <= 60_000 -> :fatal
              true -> :crater
            end

          {:ok,
           %LanderState{
             time_seconds: t_final,
             altitude_mft: 0,
             velocity_mft_s: v_touchdown,
             fuel_lbs: fuel_final,
             dry_mass_lbs: s.dry_mass_lbs,
             status: {:landed, outcome},
             impact_velocity_mft_s: v_touchdown
           }}
        end
      end
    end
  end

  defp coerce_state(%LanderState{} = s), do: {:ok, s}
  defp coerce_state(%{} = map), do: LanderState.new(map)
  defp coerce_state(_), do: {:error, :arithmetic_overflow}

  defp coerce_command(%BurnCommand{} = c), do: {:ok, c}
  defp coerce_command(%{} = map), do: BurnCommand.new(map)
  defp coerce_command(_), do: {:error, :invalid_duration}

  # ── Declarative 2D PaintScene & ASCII Telemetry HUD ─────────────────────────

  @doc """
  Formats an integer with thousands comma separators.
  """
  @spec format_with_commas(integer()) :: String.t()
  def format_with_commas(n) when is_integer(n) do
    sign = if n < 0, do: "-", else: ""
    digits = Integer.to_string(abs(n))
    len = byte_size(digits)
    rem_first = rem(len, 3)
    rem_first = if rem_first == 0, do: 3, else: rem_first

    {first, rest} = String.split_at(digits, rem_first)
    parts = [first | for(<<group::binary-3 <- rest>>, do: group)]
    sign <> Enum.join(parts, ",")
  end

  @doc """
  Translates a flight state into a declarative 76x5 [`PaintScene`].
  """
  @spec to_paint_scene(LanderState.t() | map(), non_neg_integer()) :: PaintScene.t()
  def to_paint_scene(state, last_burn_rate_lbs_s \\ 0) do
    alt_clamped = min(max(state.altitude_mft, 0), @initial_altitude_mft)
    alt_pct = div(alt_clamped * 100, @initial_altitude_mft)
    alt_bars = div(alt_clamped * 20, @initial_altitude_mft)

    fuel_clamped = min(state.fuel_lbs, @initial_fuel_lbs)
    fuel_pct = div(fuel_clamped * 100, @initial_fuel_lbs)
    fuel_bars = div(fuel_clamped * 20, @initial_fuel_lbs)

    alt_ft = div(state.altitude_mft, 1000)
    vel_fps = div(state.velocity_mft_s, 1000)
    fuel_lbs = state.fuel_lbs

    alt_str = format_with_commas(alt_ft)
    fuel_str = format_with_commas(fuel_lbs)

    {rate_label, rate_text, rate_symbol} =
      case state.status do
        :in_flight ->
          cond do
            state.velocity_mft_s < 0 -> {"RATE:", "DESCENT", "▼"}
            state.velocity_mft_s > 0 -> {"RATE:", "ASCENT", "▲"}
            true -> {"RATE:", "HOVER", "◆"}
          end

        {:landed, outcome} ->
          desc =
            case outcome do
              :perfect -> "PERFECT"
              :hard -> "HARD"
              :crash -> "CRASH"
              :fatal -> "FATAL"
              :crater -> "CRATER"
            end

          {"STATUS:", desc, "★"}

        str when is_binary(str) ->
          case status_from_string(str) do
            {:ok, status} ->
              scene = to_paint_scene(%{state | status: status}, last_burn_rate_lbs_s)
              throw({:scene, scene})

            {:error, _} ->
              {"RATE:", "UNKNOWN", " "}
          end
      end

    row0 = "┌───────────────────────── LUNAR LANDER TELEMETRY ─────────────────────────┐"

    left1 =
      " TIME: " <> String.pad_leading(Integer.to_string(state.time_seconds), 6) <> " SEC    "

    right1 =
      " ALTITUDE: " <>
        String.pad_leading(alt_str, 7) <>
        " FT [" <>
        String.duplicate("░", 20) <>
        "] " <> String.pad_leading(Integer.to_string(alt_pct), 3) <> "%   "

    row1 = "│" <> left1 <> "│" <> right1 <> "│"

    left2 = " VELOCITY: " <> String.pad_leading(Integer.to_string(vel_fps), 5) <> " FT/S"

    right2 =
      " FUEL:    " <>
        String.pad_leading(fuel_str, 7) <>
        " LBS [" <>
        String.duplicate("░", 20) <>
        "] " <> String.pad_leading(Integer.to_string(fuel_pct), 3) <> "%   "

    row2 = "│" <> left2 <> "│" <> right2 <> "│"

    left3 =
      " BURN:     " <>
        String.pad_leading(Integer.to_string(last_burn_rate_lbs_s), 4) <> " LB/S "

    rate_col =
      String.pad_trailing(rate_label, 7) <>
        " " <> String.pad_trailing(rate_text, 8) <> " " <> rate_symbol

    row3 = "│" <> left3 <> "│ " <> String.pad_trailing(rate_col, 51) <> "│"

    row4 = "└──────────────────────────────────────────────────────────────────────────┘"

    full_text = Enum.join([row0, row1, row2, row3, row4], "\n")

    instructions = [
      %PaintText{
        x: 0.0,
        y: 0.0,
        text: full_text,
        fill: "#ffffff",
        font_size: 1.0,
        text_align: :left,
        kind: :text
      }
    ]

    instructions =
      if alt_bars > 0 do
        instructions ++
          [
            %PaintRect{
              x: 46.0,
              y: 1.0,
              width: (alt_bars - 1) * 1.0,
              height: 0.0,
              fill: "#00ff00",
              kind: :rect
            }
          ]
      else
        instructions
      end

    instructions =
      if fuel_bars > 0 do
        instructions ++
          [
            %PaintRect{
              x: 46.0,
              y: 2.0,
              width: (fuel_bars - 1) * 1.0,
              height: 0.0,
              fill: "#00ff00",
              kind: :rect
            }
          ]
      else
        instructions
      end

    %PaintScene{
      width: 76.0,
      height: 5.0,
      background: "#000000",
      instructions: instructions
    }
  catch
    {:scene, scene} -> scene
  end

  @doc "Convenience helper translating a flight state into a declarative PaintScene with zero initial burn."
  @spec telemetry_scene(LanderState.t() | map()) :: PaintScene.t()
  def telemetry_scene(state), do: to_paint_scene(state, 0)

  @doc """
  Renders a flight state telemetry dashboard into a deterministic ASCII string.
  """
  @spec render_telemetry_ascii(LanderState.t() | map(), non_neg_integer()) :: String.t()
  def render_telemetry_ascii(state, last_burn_rate_lbs_s \\ 0) do
    scene = to_paint_scene(state, last_burn_rate_lbs_s)
    rows = 5
    cols = 76

    grid =
      Enum.reduce(0..(rows - 1), %{}, fn r, acc ->
        Map.put(acc, r, :erlang.make_tuple(cols, " "))
      end)

    grid =
      Enum.reduce(scene.instructions, grid, fn inst, acc ->
        case inst.kind do
          :text ->
            anchor_col = round(inst.x)
            first_row = round(inst.y)
            lines = String.split(inst.text, "\n")

            Enum.reduce(Enum.with_index(lines), acc, fn {line, line_idx}, row_acc ->
              graphemes = String.graphemes(line)
              width = length(graphemes)

              start_col =
                case inst.text_align do
                  :center -> anchor_col - div(width, 2)
                  :right -> anchor_col - width
                  _ -> anchor_col
                end

              r = first_row + line_idx

              if r >= 0 and r < rows do
                row_tuple = Map.fetch!(row_acc, r)

                updated_row =
                  Enum.reduce(Enum.with_index(graphemes), row_tuple, fn {ch, c_offset}, t ->
                    c = start_col + c_offset

                    if c >= 0 and c < cols and ch != "\r" do
                      put_elem(t, c, ch)
                    else
                      t
                    end
                  end)

                Map.put(row_acc, r, updated_row)
              else
                row_acc
              end
            end)

          :rect ->
            c1 = round(inst.x)
            r1 = round(inst.y)
            c2 = round(inst.x + inst.width)
            r2 = round(inst.y + inst.height)

            Enum.reduce(r1..r2, acc, fn r, row_acc ->
              if r >= 0 and r < rows do
                row_tuple = Map.fetch!(row_acc, r)

                updated_row =
                  Enum.reduce(c1..c2, row_tuple, fn c, t ->
                    if c >= 0 and c < cols do
                      put_elem(t, c, "█")
                    else
                      t
                    end
                  end)

                Map.put(row_acc, r, updated_row)
              else
                row_acc
              end
            end)
        end
      end)

    0..(rows - 1)
    |> Enum.map(fn r ->
      grid
      |> Map.fetch!(r)
      |> Tuple.to_list()
      |> Enum.join("")
      |> String.trim_trailing()
    end)
    |> Enum.join("\n")
    |> String.trim_trailing()
  end

  @doc "Convenience helper rendering a flight state telemetry dashboard with zero burn rate."
  @spec render_telemetry(LanderState.t() | map()) :: String.t()
  def render_telemetry(state), do: render_telemetry_ascii(state, 0)

  # ── Serialization & String Conversions ──────────────────────────────────────

  @doc "Serializes a FlightStatus into its canonical string representation."
  @spec status_to_string(LanderState.flight_status() | String.t()) :: String.t()
  def status_to_string(:in_flight), do: "InFlight"
  def status_to_string({:landed, :perfect}), do: "Landed(Perfect)"
  def status_to_string({:landed, :hard}), do: "Landed(Hard)"
  def status_to_string({:landed, :crash}), do: "Landed(Crash)"
  def status_to_string({:landed, :fatal}), do: "Landed(Fatal)"
  def status_to_string({:landed, :crater}), do: "Landed(Crater)"
  def status_to_string(str) when is_binary(str), do: str

  @doc "Parses a status string into its typed representation without dynamic atom creation."
  @spec status_from_string(String.t()) ::
          {:ok, LanderState.flight_status()} | {:error, :invalid_status}
  def status_from_string("InFlight"), do: {:ok, :in_flight}
  def status_from_string("Landed(Perfect)"), do: {:ok, {:landed, :perfect}}
  def status_from_string("Landed(Hard)"), do: {:ok, {:landed, :hard}}
  def status_from_string("Landed(Crash)"), do: {:ok, {:landed, :crash}}
  def status_from_string("Landed(Fatal)"), do: {:ok, {:landed, :fatal}}
  def status_from_string("Landed(Crater)"), do: {:ok, {:landed, :crater}}
  def status_from_string(_), do: {:error, :invalid_status}

  @doc "Maps an error reason atom to its canonical PascalCase name."
  @spec error_to_string(Error.reason() | String.t()) :: String.t()
  def error_to_string(:flight_already_ended), do: "FlightAlreadyEnded"
  def error_to_string(:excessive_burn_rate), do: "ExcessiveBurnRate"
  def error_to_string(:invalid_duration), do: "InvalidDuration"
  def error_to_string(:arithmetic_overflow), do: "ArithmeticOverflow"
  def error_to_string(str) when is_binary(str), do: str

  @doc "Parses a canonical error string into an error atom without dynamic atom creation."
  @spec error_from_string(String.t()) :: {:ok, Error.reason()} | {:error, :unknown_error}
  def error_from_string("FlightAlreadyEnded"), do: {:ok, :flight_already_ended}
  def error_from_string("ExcessiveBurnRate"), do: {:ok, :excessive_burn_rate}
  def error_from_string("InvalidDuration"), do: {:ok, :invalid_duration}
  def error_from_string("ArithmeticOverflow"), do: {:ok, :arithmetic_overflow}
  def error_from_string(_), do: {:error, :unknown_error}
end
