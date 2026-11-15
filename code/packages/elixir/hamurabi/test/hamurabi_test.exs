defmodule CodingAdventures.HamurabiTest do
  use ExUnit.Case, async: true

  import Bitwise

  alias CodingAdventures.Hamurabi
  alias CodingAdventures.Hamurabi.{Error, TurnOrder}

  @u32_max 0xFFFF_FFFF

  test "initial state satisfies all invariants" do
    state = Hamurabi.new_game(1337)
    assert state.year == 1
    assert state.population == 100
    assert state.bushels == 2800
    assert state.acres == 1000
    assert state.land_price >= 17 and state.land_price <= 26
    assert state.harvest_per_acre == 3
    assert state.rats_eaten == 200
    assert state.starved_last_year == 0
    assert state.starved_total == 0
    assert state.percent_starved_sum == 0
    assert state.immigrants_last_year == 5
    refute state.plague_struck
    assert state.status == :playing
    assert band(state.rng_inc, 1) == 1
  end

  test "all eight error arms are cleanly triggered and categorized" do
    state = Hamurabi.new_game(1337)
    normal_order = %TurnOrder{acres_delta: 0, bushels_to_feed: 2000, acres_to_plant: 100}

    # Arm 1: GameAlreadyEnded
    ended_state = %{state | status: {:won, :charlemagne}}
    assert {:error, :game_already_ended} == Hamurabi.validate_order(ended_state, normal_order)
    assert {:error, :game_already_ended} == Hamurabi.step_year(ended_state, normal_order)

    # Arm 2: InvalidOrderBounds (corrupted land price)
    bad_price_state = %{state | land_price: 10}

    assert {:error, :invalid_order_bounds} ==
             Hamurabi.validate_order(bad_price_state, normal_order)

    # Arm 2: InvalidOrderBounds (negative trade revenue exceeds u32)
    rich_state = %{state | acres: 2_000_000_000, bushels: 4_000_000_000}

    overflow_sell = %TurnOrder{
      acres_delta: -1_000_000_000,
      bushels_to_feed: 0,
      acres_to_plant: 0
    }

    assert {:error, :invalid_order_bounds} == Hamurabi.validate_order(rich_state, overflow_sell)

    # Arm 3: InsufficientBushelsForLand
    buy_too_much = %TurnOrder{acres_delta: 200, bushels_to_feed: 0, acres_to_plant: 0}

    assert {:error, :insufficient_bushels_for_land} ==
             Hamurabi.validate_order(state, buy_too_much)

    # Arm 4: InsufficientAcresToSell
    sell_too_much = %TurnOrder{acres_delta: -1001, bushels_to_feed: 0, acres_to_plant: 0}
    assert {:error, :insufficient_acres_to_sell} == Hamurabi.validate_order(state, sell_too_much)

    # Arm 5: InsufficientBushelsToFeed
    feed_too_much = %TurnOrder{acres_delta: 0, bushels_to_feed: 2801, acres_to_plant: 0}

    assert {:error, :insufficient_bushels_to_feed} ==
             Hamurabi.validate_order(state, feed_too_much)

    # Arm 6: InsufficientAcresToPlant
    plant_too_much_acres = %TurnOrder{acres_delta: 0, bushels_to_feed: 1000, acres_to_plant: 1001}

    assert {:error, :insufficient_acres_to_plant} ==
             Hamurabi.validate_order(state, plant_too_much_acres)

    # Arm 7: InsufficientPopulationToPlant (50 people tend max 500 acres)
    small_pop_state = %{state | population: 50}

    plant_too_much_labor = %TurnOrder{
      acres_delta: 0,
      bushels_to_feed: 1000,
      acres_to_plant: 501
    }

    assert {:error, :insufficient_population_to_plant} ==
             Hamurabi.validate_order(small_pop_state, plant_too_much_labor)

    # Arm 8: InsufficientGrainForSeed (1 bu seeds 2 acres; 800 acres needs 400 bu; only 300 left)
    seed_starved = %TurnOrder{acres_delta: 0, bushels_to_feed: 2500, acres_to_plant: 800}
    assert {:error, :insufficient_grain_for_seed} == Hamurabi.validate_order(state, seed_starved)
  end

  test "CWE-190 integer overflow guards" do
    state = Hamurabi.new_game(42)

    # Huge acres_delta to buy: acres_delta * land_price must not wrap
    huge_buy = %TurnOrder{acres_delta: 0x7FFF_FFFF, bushels_to_feed: 0, acres_to_plant: 0}
    assert {:error, :insufficient_bushels_for_land} == Hamurabi.validate_order(state, huge_buy)

    # Huge acres_to_plant: @u32_max
    huge_plant = %TurnOrder{acres_delta: 0, bushels_to_feed: 0, acres_to_plant: @u32_max}
    assert {:error, :insufficient_acres_to_plant} == Hamurabi.validate_order(state, huge_plant)

    # Max harvest overflow guard: acres_to_plant * 5 + bushels_after_seed > @u32_max
    big_land_state = %{
      state
      | acres: 1_000_000_000,
        population: 500_000_000,
        bushels: 2_000_000_000
    }

    harvest_overflow = %TurnOrder{acres_delta: 0, bushels_to_feed: 0, acres_to_plant: 900_000_000}

    assert {:error, :invalid_order_bounds} ==
             Hamurabi.validate_order(big_land_state, harvest_overflow)

    # Saturating addition on starved totals
    saturated_state = %{state | starved_total: @u32_max, percent_starved_sum: @u32_max - 10}
    starve_order = %TurnOrder{acres_delta: 0, bushels_to_feed: 1000, acres_to_plant: 0}
    assert {:ok, next_state} = Hamurabi.step_year(saturated_state, starve_order)
    assert next_state.starved_total == @u32_max
    assert next_state.percent_starved_sum == @u32_max
  end

  test "CWE-843 type confusion hardening with strict is_integer/1 guards" do
    state = Hamurabi.new_game(1)

    # Float coercion attempts
    assert {:error, :invalid_order_bounds} ==
             Hamurabi.validate_order(state, %{
               acres_delta: 10.5,
               bushels_to_feed: 1000,
               acres_to_plant: 100
             })

    assert {:error, :invalid_order_bounds} ==
             Hamurabi.validate_order(state, %{
               acres_delta: 0,
               bushels_to_feed: 2000.0,
               acres_to_plant: 100
             })

    assert {:error, :invalid_order_bounds} ==
             Hamurabi.validate_order(state, %{
               acres_delta: 0,
               bushels_to_feed: 2000,
               acres_to_plant: 100.25
             })

    # Booleans (in Elixir is_boolean(true) is true, but is_integer(true) is false)
    assert {:error, :invalid_order_bounds} ==
             Hamurabi.validate_order(state, %{
               acres_delta: true,
               bushels_to_feed: 2000,
               acres_to_plant: 100
             })

    # Strings
    assert {:error, :invalid_order_bounds} ==
             Hamurabi.validate_order(state, %{
               acres_delta: 0,
               bushels_to_feed: "2000",
               acres_to_plant: 100
             })

    # Nil
    assert {:error, :invalid_order_bounds} ==
             Hamurabi.validate_order(state, %{
               acres_delta: 0,
               bushels_to_feed: nil,
               acres_to_plant: 100
             })

    # Non-integer seed guard
    assert_raise FunctionClauseError, fn ->
      Hamurabi.new_game("bad_seed")
    end

    assert_raise FunctionClauseError, fn ->
      Hamurabi.new_game(3.14159)
    end
  end

  test "CWE-400 rejection of even rng_inc to prevent LCG period collapse" do
    state = Hamurabi.new_game(1)
    normal_order = %TurnOrder{acres_delta: 0, bushels_to_feed: 2000, acres_to_plant: 1000}

    bad_inc_0 = %{state | rng_inc: 0}
    assert {:error, :invalid_order_bounds} == Hamurabi.validate_order(bad_inc_0, normal_order)
    assert {:error, :invalid_order_bounds} == Hamurabi.step_year(bad_inc_0, normal_order)

    bad_inc_2 = %{state | rng_inc: 2}
    assert {:error, :invalid_order_bounds} == Hamurabi.validate_order(bad_inc_2, normal_order)

    bad_inc_108 = %{state | rng_inc: 108}
    assert {:error, :invalid_order_bounds} == Hamurabi.validate_order(bad_inc_108, normal_order)
  end

  test "adversarial state invariants bounds" do
    valid_order = %TurnOrder{acres_delta: 0, bushels_to_feed: 2000, acres_to_plant: 1000}
    state = Hamurabi.new_game(1)

    # Hostile year bounds (valid is 1..10 during play)
    assert {:error, :invalid_order_bounds} ==
             Hamurabi.validate_order(%{state | year: 0}, valid_order)

    assert {:error, :invalid_order_bounds} ==
             Hamurabi.validate_order(%{state | year: 11}, valid_order)

    assert {:error, :invalid_order_bounds} ==
             Hamurabi.validate_order(%{state | year: 255}, valid_order)

    # Hostile land price bounds (valid is 17..26)
    assert {:error, :invalid_order_bounds} ==
             Hamurabi.validate_order(%{state | land_price: 0}, valid_order)

    assert {:error, :invalid_order_bounds} ==
             Hamurabi.validate_order(%{state | land_price: 16}, valid_order)

    assert {:error, :invalid_order_bounds} ==
             Hamurabi.validate_order(%{state | land_price: 27}, valid_order)

    assert {:error, :invalid_order_bounds} ==
             Hamurabi.validate_order(%{state | land_price: 100}, valid_order)

    # Hostile harvest per acre bounds (valid is 1..5)
    assert {:error, :invalid_order_bounds} ==
             Hamurabi.validate_order(%{state | harvest_per_acre: 0}, valid_order)

    assert {:error, :invalid_order_bounds} ==
             Hamurabi.validate_order(%{state | harvest_per_acre: 6}, valid_order)

    assert {:error, :invalid_order_bounds} ==
             Hamurabi.validate_order(%{state | harvest_per_acre: 99}, valid_order)
  end

  test "rating classifications and reports formatting" do
    base = Hamurabi.new_game(1)
    base = %{base | year: 11}

    charlemagne = %{base | status: {:won, :charlemagne}}
    rep_c = Hamurabi.format_report(charlemagne)
    assert String.contains?(rep_c, "A FANTASTIC PERFORMANCE!")

    acre = %{base | status: {:won, :acre}}
    rep_a = Hamurabi.format_report(acre)
    assert String.contains?(rep_a, "YOUR PERFORMANCE COULD HAVE BEEN WORSE")

    nero = %{base | status: {:won, :nero}}
    rep_n = Hamurabi.format_report(nero)
    assert String.contains?(rep_n, "NERO WOULD HAVE BEEN PROUD OF YOU")

    disaster = %{base | status: {:won, :disaster}}
    rep_d = Hamurabi.format_report(disaster)
    assert String.contains?(rep_d, "OFFICIAL NATIONAL DISASTER")

    extinct = %{base | status: :game_over_extinction}
    rep_e = Hamurabi.format_report(extinct)
    assert String.contains?(rep_e, "EVERYONE HAS DIED. THE CITY OF LAGASH IS EXTINCT!")

    impeached = %{base | status: {:impeached, 48, 48}}
    rep_i = Hamurabi.format_report(impeached)
    assert String.contains?(rep_i, "YOU STARVED 48 PEOPLE IN ONE YEAR!!!")
    assert String.contains?(rep_i, "IMPEACHED AND THROWN OUT OF OFFICE")
  end

  test "step_year! raises Hamurabi.Error on refusal" do
    state = Hamurabi.new_game(1)
    bad_order = %TurnOrder{acres_delta: 0, bushels_to_feed: 999_999, acres_to_plant: 0}

    assert_raise Error, fn ->
      Hamurabi.step_year!(state, bad_order)
    end
  end

  test "state immutability on refusal" do
    state = Hamurabi.new_game(42)
    bad_order = %TurnOrder{acres_delta: 0, bushels_to_feed: 9999, acres_to_plant: 0}

    assert {:error, :insufficient_bushels_to_feed} == Hamurabi.step_year(state, bad_order)
    assert state.year == 1
    assert state.bushels == 2800
    assert state.population == 100
  end

  test "full 10-year deterministic game loop" do
    state = Hamurabi.new_game(4242)

    final_state =
      Enum.reduce(1..10, state, fn expected_year, cur ->
        assert cur.year == expected_year
        assert cur.status == :playing

        feed_needed = cur.population * 20

        acres_delta =
          cond do
            cur.bushels < feed_needed + 500 ->
              deficit = feed_needed + 500 - cur.bushels

              acres_to_sell =
                min(div(deficit + cur.land_price - 1, cur.land_price), div(cur.acres, 3))

              -acres_to_sell

            cur.bushels > feed_needed + 2000 ->
              surplus = cur.bushels - (feed_needed + 1000)
              min(div(surplus, cur.land_price), 50)

            true ->
              0
          end

        acres_after_trade =
          if acres_delta >= 0 do
            cur.acres + acres_delta
          else
            cur.acres - -acres_delta
          end

        bushels_after_trade =
          cond do
            acres_delta > 0 -> cur.bushels - acres_delta * cur.land_price
            acres_delta < 0 -> cur.bushels + -acres_delta * cur.land_price
            true -> cur.bushels
          end

        bushels_to_feed = min(feed_needed, bushels_after_trade)
        bushels_remaining = bushels_after_trade - bushels_to_feed

        max_plantable = min(acres_after_trade, cur.population * 10)
        max_from_seed = bushels_remaining * 2
        acres_to_plant = min(max_plantable, max_from_seed)

        order = %TurnOrder{
          acres_delta: acres_delta,
          bushels_to_feed: bushels_to_feed,
          acres_to_plant: acres_to_plant
        }

        {:ok, next} = Hamurabi.step_year(cur, order)
        next
      end)

    assert final_state.year == 11
    assert {:won, rating} = final_state.status
    assert rating in [:charlemagne, :acre, :nero, :disaster]
    report = Hamurabi.format_report(final_state)
    assert String.contains?(report, "IN YOUR 10-YEAR TERM OF OFFICE")
  end
end
