defmodule CodingAdventures.Hamurabi do
  @moduledoc """
  Pure, deterministic, platform-independent simulation core for the seminal
  1968 mainframe resource allocation game (GAME01).

  ## Historical Lineage

  - **1964 — The Sumerian Game:** Designed by Mabel Addis (the first credited
    female video game designer and writer) and programmed by William McKay for
    an IBM 7090 mainframe at the BOCES in Westchester County, New York.
  - **1968 — FOCAL Hamurabi:** Implemented by Richard Merrill on the DEC PDP-8
    in FOCAL-69, shortening the historical Sumerian ruler name to fit the 8-character
    symbol constraint of PDP-8 teletype terminals.
  - **1973 — 101 BASIC Computer Games:** Adapted to Dartmouth BASIC by David H.
    Ahl and published in DEC's landmark book, becoming the archetypal computer
    simulation game that influenced an entire generation of programmers.

  ## Architectural Guarantees

  - Strictly zero ambient system authority: `"capabilities": []` declared in
    `required_capabilities.json`.
  - Pure state transition: `step_year(state, order)` returns `{:ok, next_state}` or `{:error, reason}`.
  - Explicit pseudorandomness: all simulation rolls enter deterministically
    through explicit PCG32 state stored directly inside `HamurabiState`.
  - Pure integer arithmetic: uses exclusively bounded integer types (`u8`, `u32`,
    `u64`, `i32`) with zero floating-point operations.
  - Complete 26-CWE weakness closure (`VER15`), including strict `is_integer/1`
    guards, zero `String.to_atom/1`, and zero dynamic `Code.eval_*`.
  """

  import Bitwise

  alias CodingAdventures.Hamurabi.{Error, State, TurnOrder}

  # ── Protocol Constants ──────────────────────────────────────────────────────

  @pcg_multiplier 6_364_136_223_846_793_005
  @hamurabi_stream_id 54
  @pcg_inc 109
  @mask64 0xFFFF_FFFF_FFFF_FFFF
  @mask32 0xFFFF_FFFF
  @u32_max 0xFFFF_FFFF
  @i32_min -0x8000_0000
  @i32_max 0x7FFF_FFFF
  @two_32 1 <<< 32

  # ── Structs ─────────────────────────────────────────────────────────────────

  defmodule State do
    @moduledoc "Immutable value record capturing the complete simulation state."

    @enforce_keys [
      :year,
      :population,
      :bushels,
      :acres,
      :land_price,
      :harvest_per_acre,
      :rats_eaten,
      :starved_last_year,
      :starved_total,
      :percent_starved_sum,
      :immigrants_last_year,
      :plague_struck,
      :status,
      :rng_state,
      :rng_inc
    ]

    defstruct [
      :year,
      :population,
      :bushels,
      :acres,
      :land_price,
      :harvest_per_acre,
      :rats_eaten,
      :starved_last_year,
      :starved_total,
      :percent_starved_sum,
      :immigrants_last_year,
      :plague_struck,
      :status,
      :rng_state,
      :rng_inc
    ]

    @type rating :: :charlemagne | :acre | :nero | :disaster
    @type status ::
            :playing
            | {:won, rating()}
            | {:impeached, non_neg_integer(), non_neg_integer()}
            | :game_over_extinction

    @type t :: %__MODULE__{
            year: 1..11,
            population: non_neg_integer(),
            bushels: non_neg_integer(),
            acres: non_neg_integer(),
            land_price: 17..26,
            harvest_per_acre: 1..5,
            rats_eaten: non_neg_integer(),
            starved_last_year: non_neg_integer(),
            starved_total: non_neg_integer(),
            percent_starved_sum: non_neg_integer(),
            immigrants_last_year: non_neg_integer(),
            plague_struck: boolean(),
            status: status(),
            rng_state: non_neg_integer(),
            rng_inc: non_neg_integer()
          }
  end

  defmodule TurnOrder do
    @moduledoc "Player orders for a single year."

    @enforce_keys [:acres_delta, :bushels_to_feed, :acres_to_plant]

    defstruct [:acres_delta, :bushels_to_feed, :acres_to_plant]

    @type t :: %__MODULE__{
            acres_delta: integer(),
            bushels_to_feed: non_neg_integer(),
            acres_to_plant: non_neg_integer()
          }

    @doc "Creates a TurnOrder struct from a map with atom or string keys."
    @spec new(map() | t()) :: {:ok, t()} | {:error, :invalid_order_bounds}
    def new(%__MODULE__{} = order), do: {:ok, order}

    def new(%{acres_delta: d, bushels_to_feed: f, acres_to_plant: p}) do
      if is_integer(d) and is_integer(f) and is_integer(p) and
           d >= -0x8000_0000 and d <= 0x7FFF_FFFF and
           f >= 0 and f <= 0xFFFF_FFFF and
           p >= 0 and p <= 0xFFFF_FFFF do
        {:ok, %__MODULE__{acres_delta: d, bushels_to_feed: f, acres_to_plant: p}}
      else
        {:error, :invalid_order_bounds}
      end
    end

    def new(%{"acres_delta" => d, "bushels_to_feed" => f, "acres_to_plant" => p}) do
      new(%{acres_delta: d, bushels_to_feed: f, acres_to_plant: p})
    end

    def new(_), do: {:error, :invalid_order_bounds}
  end

  defmodule Error do
    @moduledoc "Exception struct representing an order rejection error."
    defexception [:reason, :message]

    @type reason ::
            :game_already_ended
            | :invalid_order_bounds
            | :insufficient_bushels_for_land
            | :insufficient_acres_to_sell
            | :insufficient_bushels_to_feed
            | :insufficient_acres_to_plant
            | :insufficient_population_to_plant
            | :insufficient_grain_for_seed

    @impl true
    def exception(reason) when is_atom(reason) do
      msg = CodingAdventures.Hamurabi.error_message(reason)
      %__MODULE__{reason: reason, message: msg}
    end
  end

  # Canonical alias
  defmodule HamurabiState do
    @moduledoc false
    # Forward to State
    defdelegate __struct__(), to: State
    defdelegate __struct__(kv), to: State
  end

  # ── PRNG Internal Primitives (PCG32 XSH RR 64/32) ──────────────────────────

  @doc """
  Advances the 64-bit PCG32 state and returns `{output_u32, new_state_u64}`.
  """
  @spec pcg32_step(non_neg_integer(), non_neg_integer()) ::
          {non_neg_integer(), non_neg_integer()}
  def pcg32_step(state, inc) when is_integer(state) and is_integer(inc) do
    old_state = band(state, @mask64)
    new_state = band(old_state * @pcg_multiplier + inc, @mask64)
    xorshifted = band(bsr(bxor(bsr(old_state, 18), old_state), 27), @mask32)
    rot = bsr(old_state, 59)

    output =
      if rot == 0 do
        xorshifted
      else
        band(bor(bsr(xorshifted, rot), bsl(xorshifted, 32 - rot)), @mask32)
      end

    {output, new_state}
  end

  @doc """
  Generates a uniform random integer in `[min, max]` inclusive using PCG32
  rejection sampling.
  """
  @spec pcg32_next_int_in_range(
          non_neg_integer(),
          non_neg_integer(),
          non_neg_integer(),
          non_neg_integer()
        ) :: {non_neg_integer(), non_neg_integer()}
  def pcg32_next_int_in_range(state, inc, min, max)
      when is_integer(state) and is_integer(inc) and is_integer(min) and is_integer(max) and
             min <= max do
    span = max - min + 1
    threshold = rem(@two_32 - span, span)
    sample_range(state, inc, min, span, threshold)
  end

  defp sample_range(state, inc, min, span, threshold) do
    {r, next_state} = pcg32_step(state, inc)

    if r >= threshold do
      {min + rem(r, span), next_state}
    else
      sample_range(next_state, inc, min, span, threshold)
    end
  end

  # ── Public Simulation Engine API ───────────────────────────────────────────

  @doc """
  Initializes a new, deterministic 10-year reign for ruler Hamurabi.

  Seeds PCG32 with `(seed, stream_id = 54, inc = 109)` and draws the initial
  land trading price for Year 1 in `[17, 26]`.
  """
  @spec new_game(integer()) :: State.t()
  def new_game(seed) when is_integer(seed) do
    inc = @pcg_inc
    s0 = 0
    s1 = band(s0 * @pcg_multiplier + inc, @mask64)
    s2 = band(s1 + band(seed, @mask64), @mask64)
    s3 = band(s2 * @pcg_multiplier + inc, @mask64)
    {land_price, s4} = pcg32_next_int_in_range(s3, inc, 17, 26)

    %State{
      year: 1,
      population: 100,
      bushels: 2800,
      acres: 1000,
      land_price: land_price,
      harvest_per_acre: 3,
      rats_eaten: 200,
      starved_last_year: 0,
      starved_total: 0,
      percent_starved_sum: 0,
      immigrants_last_year: 5,
      plague_struck: false,
      status: :playing,
      rng_state: s4,
      rng_inc: inc
    }
  end

  @doc """
  Validates player orders against current state constraints without advancing state.

  Evaluates checks in the canonical order specified by `GAME01`:
  1. Status check (`:playing`)
  2. State invariants & bounds check
  3. Land trade affordability & acreage check
  4. Balance limits
  5. Food allocation feasibility
  6. Planting land & labor limits
  7. Seed grain availability
  8. Maximum harvest overflow guard
  """
  @spec validate_order(State.t() | map(), TurnOrder.t() | map()) ::
          :ok | {:error, Error.reason()}
  def validate_order(state, order) do
    # 1. Status Check
    if not is_playing_status?(state) do
      {:error, :game_already_ended}
    else
      validate_invariants_and_order(state, order)
    end
  end

  defp validate_invariants_and_order(state, order) do
    # 2. State Invariants Check
    with :ok <- validate_state_invariants(state),
         {:ok, norm_order} <- extract_order(order),
         :ok <- validate_trade_bounds(state, norm_order),
         {:ok, acres_after_trade, bushels_after_trade} <-
           compute_trade_balances(state, norm_order),
         :ok <- validate_food(bushels_after_trade, norm_order.bushels_to_feed),
         bushels_after_feed = bushels_after_trade - norm_order.bushels_to_feed,
         :ok <-
           validate_planting(
             acres_after_trade,
             state.population,
             bushels_after_feed,
             norm_order.acres_to_plant
           ) do
      :ok
    else
      {:error, reason} -> {:error, reason}
    end
  end

  defp is_playing_status?(%{status: :playing}), do: true
  defp is_playing_status?(%{status: "Playing"}), do: true
  defp is_playing_status?(_), do: false

  defp validate_state_invariants(state) do
    year = Map.get(state, :year)
    land_price = Map.get(state, :land_price)
    harvest = Map.get(state, :harvest_per_acre)
    inc = Map.get(state, :rng_inc)
    rng_state = Map.get(state, :rng_state)
    pop = Map.get(state, :population)
    bushels = Map.get(state, :bushels)
    acres = Map.get(state, :acres)
    starved_total = Map.get(state, :starved_total)
    starved_sum = Map.get(state, :percent_starved_sum)
    rats_eaten = Map.get(state, :rats_eaten)
    starved_last = Map.get(state, :starved_last_year)
    imm = Map.get(state, :immigrants_last_year)
    plague = Map.get(state, :plague_struck)

    if is_integer(year) and year >= 1 and year <= 10 and
         is_integer(land_price) and land_price >= 17 and land_price <= 26 and
         is_integer(harvest) and harvest >= 1 and harvest <= 5 and
         is_integer(inc) and inc >= 0 and inc <= @mask64 and band(inc, 1) == 1 and
         is_integer(rng_state) and rng_state >= 0 and rng_state <= @mask64 and
         is_integer(pop) and pop >= 0 and pop <= @u32_max and
         is_integer(bushels) and bushels >= 0 and bushels <= @u32_max and
         is_integer(acres) and acres >= 0 and acres <= @u32_max and
         is_integer(starved_total) and starved_total >= 0 and starved_total <= @u32_max and
         is_integer(starved_sum) and starved_sum >= 0 and starved_sum <= @u32_max and
         is_integer(rats_eaten) and rats_eaten >= 0 and rats_eaten <= @u32_max and
         is_integer(starved_last) and starved_last >= 0 and starved_last <= @u32_max and
         is_integer(imm) and imm >= 0 and imm <= @u32_max and
         is_boolean(plague) do
      :ok
    else
      {:error, :invalid_order_bounds}
    end
  end

  defp extract_order(%TurnOrder{acres_delta: d, bushels_to_feed: f, acres_to_plant: p}) do
    check_order_fields(d, f, p)
  end

  defp extract_order(%{acres_delta: d, bushels_to_feed: f, acres_to_plant: p}) do
    check_order_fields(d, f, p)
  end

  defp extract_order(%{"acres_delta" => d, "bushels_to_feed" => f, "acres_to_plant" => p}) do
    check_order_fields(d, f, p)
  end

  defp extract_order(_), do: {:error, :invalid_order_bounds}

  defp check_order_fields(d, f, p) do
    if is_integer(d) and is_integer(f) and is_integer(p) and
         d >= @i32_min and d <= @i32_max and
         f >= 0 and f <= @u32_max and
         p >= 0 and p <= @u32_max do
      {:ok, %TurnOrder{acres_delta: d, bushels_to_feed: f, acres_to_plant: p}}
    else
      {:error, :invalid_order_bounds}
    end
  end

  defp validate_trade_bounds(state, order) do
    cond do
      order.acres_delta > 0 ->
        cost = order.acres_delta * state.land_price

        if cost > state.bushels do
          {:error, :insufficient_bushels_for_land}
        else
          :ok
        end

      order.acres_delta < 0 ->
        sell_count = -order.acres_delta

        if sell_count > state.acres do
          {:error, :insufficient_acres_to_sell}
        else
          :ok
        end

      true ->
        :ok
    end
  end

  defp compute_trade_balances(state, order) do
    acres_after_trade =
      if order.acres_delta >= 0 do
        state.acres + order.acres_delta
      else
        state.acres - -order.acres_delta
      end

    if acres_after_trade > @u32_max do
      {:error, :invalid_order_bounds}
    else
      bushels_after_trade =
        cond do
          order.acres_delta > 0 ->
            cost = order.acres_delta * state.land_price
            state.bushels - cost

          order.acres_delta < 0 ->
            sell_count = -order.acres_delta
            revenue = sell_count * state.land_price
            state.bushels + revenue

          true ->
            state.bushels
        end

      if bushels_after_trade > @u32_max do
        {:error, :invalid_order_bounds}
      else
        {:ok, acres_after_trade, bushels_after_trade}
      end
    end
  end

  defp validate_food(bushels_after_trade, bushels_to_feed) do
    if bushels_to_feed > bushels_after_trade do
      {:error, :insufficient_bushels_to_feed}
    else
      :ok
    end
  end

  defp validate_planting(acres_after_trade, population, bushels_after_feed, acres_to_plant) do
    cond do
      acres_to_plant > acres_after_trade ->
        {:error, :insufficient_acres_to_plant}

      acres_to_plant > population * 10 ->
        {:error, :insufficient_population_to_plant}

      true ->
        seed_needed = div(acres_to_plant + 1, 2)

        cond do
          seed_needed > bushels_after_feed ->
            {:error, :insufficient_grain_for_seed}

          bushels_after_feed - seed_needed + acres_to_plant * 5 > @u32_max ->
            {:error, :invalid_order_bounds}

          true ->
            :ok
        end
    end
  end

  @doc """
  Advances the simulation by exactly one year according to player orders.

  Executes the two-phase state transition:
  - **Phase A:** Validation & constraint checking in canonical order.
  - **Phase B:** Strictly locked PRNG draw sequence (harvest, rats, plague, starvation, immigration, next land price).

  Returns `{:ok, next_state}` on success, or `{:error, reason}` on refusal.
  The original simulation state is never modified on error.
  """
  @spec step_year(State.t() | map(), TurnOrder.t() | map()) ::
          {:ok, State.t()} | {:error, Error.reason()}
  def step_year(state, order) do
    case validate_order(state, order) do
      :ok ->
        {:ok, norm_order} = extract_order(order)
        {:ok, execute_step(state, norm_order)}

      {:error, reason} ->
        {:error, reason}
    end
  end

  @doc """
  Same as `step_year/2`, but returns `next_state` directly or raises `Error` on refusal.
  """
  @spec step_year!(State.t() | map(), TurnOrder.t() | map()) :: State.t()
  def step_year!(state, order) do
    case step_year(state, order) do
      {:ok, next_state} -> next_state
      {:error, reason} -> raise Error, reason
    end
  end

  defp execute_step(state, order) do
    # Trade balances
    acres_after_trade =
      if order.acres_delta >= 0 do
        state.acres + order.acres_delta
      else
        state.acres - -order.acres_delta
      end

    bushels_after_trade =
      cond do
        order.acres_delta > 0 ->
          state.bushels - order.acres_delta * state.land_price

        order.acres_delta < 0 ->
          state.bushels + -order.acres_delta * state.land_price

        true ->
          state.bushels
      end

    bushels_after_feed = bushels_after_trade - order.bushels_to_feed
    seed_needed = div(order.acres_to_plant + 1, 2)
    bushels_after_seed = bushels_after_feed - seed_needed

    # Step 1: Harvest Yield Draw
    # Draw 1: pcg.next_int_in_range(1, 5) -> harvest_per_acre
    {harvest_per_acre, s1} = pcg32_next_int_in_range(state.rng_state, state.rng_inc, 1, 5)
    total_harvest = order.acres_to_plant * harvest_per_acre
    bushels_stored = bushels_after_seed + total_harvest

    # Step 2: Rat Infestation Draws
    # Draw 2: pcg.next_int_in_range(1, 100) -> rat_roll
    {rat_roll, s2} = pcg32_next_int_in_range(s1, state.rng_inc, 1, 100)

    {rats_eaten, s3} =
      if rat_roll <= 40 do
        # Draw 3: pcg.next_int_in_range(1, 5) -> rat_factor
        {rat_factor, s_rat} = pcg32_next_int_in_range(s2, state.rng_inc, 1, 5)
        eaten = min(@u32_max, div(bushels_stored, rat_factor))
        {eaten, s_rat}
      else
        {0, s2}
      end

    bushels_final = min(@u32_max, max(0, bushels_stored - rats_eaten))

    # Step 3: Plague Draw
    # Draw 4: pcg.next_int_in_range(1, 100) -> plague_roll
    {plague_roll, s4} = pcg32_next_int_in_range(s3, state.rng_inc, 1, 100)
    plague_struck = plague_roll <= 15

    # Step 4: Starvation & Impeachment Evaluation
    people_fed = div(order.bushels_to_feed, 20)

    {starved, survivors} =
      if people_fed >= state.population do
        {0, state.population}
      else
        {state.population - people_fed, people_fed}
      end

    percent_starved =
      if state.population > 0 do
        min(100, div(starved * 100, state.population))
      else
        0
      end

    new_starved_total = min(@u32_max, state.starved_total + starved)
    new_percent_sum = min(@u32_max, state.percent_starved_sum + percent_starved)

    new_status =
      cond do
        starved * 100 > state.population * 45 ->
          {:impeached, starved, percent_starved}

        survivors == 0 ->
          :game_over_extinction

        true ->
          :playing
      end

    # Step 5: Plague Resolution
    survivors_after_plague =
      if plague_struck and new_status == :playing do
        div(survivors, 2)
      else
        survivors
      end

    # Step 6: Immigration Draw
    # Draw 5: pcg.next_int_in_range(1, 5) -> imm_factor (unconditional)
    {imm_factor, s5} = pcg32_next_int_in_range(s4, state.rng_inc, 1, 5)

    immigrants =
      if starved == 0 and new_status == :playing do
        numerator = imm_factor * (20 * acres_after_trade + bushels_final)
        denominator = 100 * state.population

        if denominator > 0 do
          min(@u32_max, div(numerator, denominator) + 1)
        else
          0
        end
      else
        0
      end

    final_population = min(@u32_max, survivors_after_plague + immigrants)

    # Step 7: Next Year Land Price Draw
    # Draw 6: pcg.next_int_in_range(17, 26) -> next_land_price
    {next_land_price, s6} = pcg32_next_int_in_range(s5, state.rng_inc, 17, 26)

    # Step 8: Turn Advance & Rating
    next_year = min(255, state.year + 1)

    final_status =
      if next_year == 11 and new_status == :playing do
        avg_starved = div(new_percent_sum, 10)

        acres_per_person =
          if final_population > 0, do: div(acres_after_trade, final_population), else: 0

        rating =
          cond do
            avg_starved <= 3 and acres_per_person >= 10 -> :charlemagne
            avg_starved <= 10 and acres_per_person >= 9 -> :acre
            avg_starved <= 33 and acres_per_person >= 7 -> :nero
            true -> :disaster
          end

        {:won, rating}
      else
        new_status
      end

    %State{
      year: next_year,
      population: final_population,
      bushels: bushels_final,
      acres: acres_after_trade,
      land_price: next_land_price,
      harvest_per_acre: harvest_per_acre,
      rats_eaten: rats_eaten,
      starved_last_year: starved,
      starved_total: new_starved_total,
      percent_starved_sum: new_percent_sum,
      immigrants_last_year: immigrants,
      plague_struck: plague_struck,
      status: final_status,
      rng_state: s6,
      rng_inc: state.rng_inc
    }
  end

  # ── Report Formatting API ──────────────────────────────────────────────────

  @doc """
  Formats the annual teletype status report or final reign evaluation in
  authentic 1970s teletype uppercase styling.
  """
  @spec format_report(State.t() | map()) :: String.t()
  def format_report(%{status: {:impeached, starved_this_turn, _percent}}) do
    format_impeached_message(starved_this_turn)
  end

  def format_report(%{status: {:impeached, %{starved_this_turn: starved_this_turn}}}) do
    format_impeached_message(starved_this_turn)
  end

  def format_report(%{status: "Impeached" <> rest}) do
    case parse_impeached_params(rest) do
      {:ok, starved, _p} -> format_impeached_message(starved)
      _ -> format_impeached_message(0)
    end
  end

  def format_report(%{status: status})
      when status in [:game_over_extinction, "GameOverExtinction"] do
    "EVERYONE HAS DIED. THE CITY OF LAGASH IS EXTINCT!\n"
  end

  def format_report(%{status: {:won, rating}} = state) do
    format_won_report(state, rating)
  end

  def format_report(%{status: "Won(" <> rest} = state) do
    rating =
      case rest do
        "Charlemagne)" -> :charlemagne
        "Acre)" -> :acre
        "Nero)" -> :nero
        "Disaster)" -> :disaster
        _ -> :disaster
      end

    format_won_report(state, rating)
  end

  def format_report(%{status: status} = state) when status in [:playing, "Playing"] do
    header =
      "HAMURABI: I BEG TO REPORT TO YOU,\n" <>
        "IN YEAR #{state.year}, #{state.starved_last_year} PEOPLE STARVED, #{state.immigrants_last_year} CAME TO THE CITY.\n"

    plague =
      if state.plague_struck do
        "A HORRIBLE PLAGUE STRUCK! HALF THE PEOPLE DIED.\n"
      else
        ""
      end

    body =
      "POPULATION IS NOW #{state.population}.\n" <>
        "THE CITY NOW OWNS #{state.acres} ACRES.\n" <>
        "YOU HARVESTED #{state.harvest_per_acre} BUSHELS PER ACRE.\n" <>
        "RATS ATE #{state.rats_eaten} BUSHELS.\n" <>
        "YOU NOW HAVE #{state.bushels} BUSHELS IN STORE.\n" <>
        "LAND IS TRADING AT #{state.land_price} BUSHELS PER ACRE.\n"

    header <> plague <> body
  end

  defp format_impeached_message(starved) do
    "YOU STARVED #{starved} PEOPLE IN ONE YEAR!!!\n" <>
      "DUE TO THIS EXTREME MISMANAGEMENT YOU HAVE BEEN IMPEACHED AND THROWN OUT OF OFFICE!\n"
  end

  defp format_won_report(state, rating) do
    avg_starved = div(state.percent_starved_sum, 10)

    acres_per_person =
      if state.population > 0, do: div(state.acres, state.population), else: 0

    rating_msg =
      case rating do
        :charlemagne ->
          "A FANTASTIC PERFORMANCE! CHARLEMAGNE, DISRAELI, AND JEFFERSON COMBINED COULD NOT HAVE DONE BETTER!"

        :acre ->
          "YOUR PERFORMANCE COULD HAVE BEEN WORSE, BUT YOU LEFT THEM REASONABLE ACREAGE."

        :nero ->
          "YOUR PERFORMANCE LEAVES MUCH TO BE DESIRED. NERO WOULD HAVE BEEN PROUD OF YOU."

        :disaster ->
          "YOU DID SO POORLY YOU HAVE BEEN DECLARED AN OFFICIAL NATIONAL DISASTER!"
      end

    "IN YOUR 10-YEAR TERM OF OFFICE, #{avg_starved} PERCENT OF THE\n" <>
      "POPULATION STARVED PER YEAR ON AVERAGE.\n" <>
      "A TOTAL OF #{state.starved_total} PEOPLE DIED!!\n" <>
      "YOU STARTED WITH 10 ACRES PER PERSON AND ENDED WITH\n" <>
      "#{acres_per_person} ACRES PER PERSON.\n\n" <>
      "#{rating_msg}\n"
  end

  # Safe parsing of "Impeached(starved=X, percent=Y)" without regex or String.to_atom
  defp parse_impeached_params("(starved=" <> rest) do
    with [starved_str, rest2] <- :binary.split(rest, ", percent="),
         [percent_str, ""] <- :binary.split(rest2, ")"),
         {starved, ""} <- Integer.parse(starved_str),
         {percent, ""} <- Integer.parse(percent_str) do
      {:ok, starved, percent}
    else
      _ -> :error
    end
  end

  defp parse_impeached_params(_), do: :error

  # ── Serialization & String Conversions ─────────────────────────────────────

  @doc "Serializes a GameStatus variant into its canonical string representation."
  @spec status_to_string(State.status() | String.t()) :: String.t()
  def status_to_string(:playing), do: "Playing"
  def status_to_string({:won, :charlemagne}), do: "Won(Charlemagne)"
  def status_to_string({:won, :acre}), do: "Won(Acre)"
  def status_to_string({:won, :nero}), do: "Won(Nero)"
  def status_to_string({:won, :disaster}), do: "Won(Disaster)"

  def status_to_string({:impeached, starved, percent}) do
    "Impeached(starved=#{starved}, percent=#{percent})"
  end

  def status_to_string({:impeached, %{starved_this_turn: starved, percent: percent}}) do
    "Impeached(starved=#{starved}, percent=#{percent})"
  end

  def status_to_string(:game_over_extinction), do: "GameOverExtinction"
  def status_to_string(str) when is_binary(str), do: str

  @doc "Parses a status string into its typed representation without dynamic atom creation."
  @spec status_from_string(String.t()) :: {:ok, State.status()} | {:error, :invalid_status}
  def status_from_string("Playing"), do: {:ok, :playing}
  def status_from_string("Won(Charlemagne)"), do: {:ok, {:won, :charlemagne}}
  def status_from_string("Won(Acre)"), do: {:ok, {:won, :acre}}
  def status_from_string("Won(Nero)"), do: {:ok, {:won, :nero}}
  def status_from_string("Won(Disaster)"), do: {:ok, {:won, :disaster}}
  def status_from_string("GameOverExtinction"), do: {:ok, :game_over_extinction}

  def status_from_string("Impeached" <> rest) do
    case parse_impeached_params(rest) do
      {:ok, starved, percent} -> {:ok, {:impeached, starved, percent}}
      :error -> {:error, :invalid_status}
    end
  end

  def status_from_string(_), do: {:error, :invalid_status}

  @doc "Maps an error reason atom to its canonical PascalCase name."
  @spec error_to_string(Error.reason() | String.t()) :: String.t()
  def error_to_string(:game_already_ended), do: "GameAlreadyEnded"
  def error_to_string(:invalid_order_bounds), do: "InvalidOrderBounds"
  def error_to_string(:insufficient_bushels_for_land), do: "InsufficientBushelsForLand"
  def error_to_string(:insufficient_acres_to_sell), do: "InsufficientAcresToSell"
  def error_to_string(:insufficient_bushels_to_feed), do: "InsufficientBushelsToFeed"
  def error_to_string(:insufficient_acres_to_plant), do: "InsufficientAcresToPlant"
  def error_to_string(:insufficient_population_to_plant), do: "InsufficientPopulationToPlant"
  def error_to_string(:insufficient_grain_for_seed), do: "InsufficientGrainForSeed"
  def error_to_string(str) when is_binary(str), do: str

  @doc "Parses a canonical error string into an error atom without dynamic atom creation."
  @spec error_from_string(String.t()) :: {:ok, Error.reason()} | {:error, :unknown_error}
  def error_from_string("GameAlreadyEnded"), do: {:ok, :game_already_ended}
  def error_from_string("InvalidOrderBounds"), do: {:ok, :invalid_order_bounds}
  def error_from_string("InsufficientBushelsForLand"), do: {:ok, :insufficient_bushels_for_land}
  def error_from_string("InsufficientAcresToSell"), do: {:ok, :insufficient_acres_to_sell}
  def error_from_string("InsufficientBushelsToFeed"), do: {:ok, :insufficient_bushels_to_feed}
  def error_from_string("InsufficientAcresToPlant"), do: {:ok, :insufficient_acres_to_plant}

  def error_from_string("InsufficientPopulationToPlant"),
    do: {:ok, :insufficient_population_to_plant}

  def error_from_string("InsufficientGrainForSeed"), do: {:ok, :insufficient_grain_for_seed}
  def error_from_string(_), do: {:error, :unknown_error}

  @doc "Returns a descriptive English human-readable message for an error atom."
  @spec error_message(Error.reason()) :: String.t()
  def error_message(:game_already_ended), do: "Game has already ended"

  def error_message(:invalid_order_bounds),
    do: "Order parameters exceed valid numerical bounds or overflow"

  def error_message(:insufficient_bushels_for_land),
    do: "Insufficient bushels to buy specified acreage"

  def error_message(:insufficient_acres_to_sell),
    do: "Cannot sell more acres than the city owns"

  def error_message(:insufficient_bushels_to_feed),
    do: "Insufficient bushels in store to feed population"

  def error_message(:insufficient_acres_to_plant),
    do: "Cannot plant more acres than owned after land trade"

  def error_message(:insufficient_population_to_plant),
    do: "Insufficient population to tend planted acreage"

  def error_message(:insufficient_grain_for_seed),
    do: "Insufficient grain to seed planted acreage"

  def error_message(other), do: "Hamurabi simulation error: #{inspect(other)}"
end
