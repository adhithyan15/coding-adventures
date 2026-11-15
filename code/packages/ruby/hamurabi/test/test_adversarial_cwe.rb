# frozen_string_literal: true

require_relative "test_helper"

class TestAdversarialCWE < Minitest::Test
  include CodingAdventures::Hamurabi

  def setup
    @state = CodingAdventures::Hamurabi.new_game(1337)
  end

  # CWE-843: Type Confusion (reject Float, String, Boolean, nil)
  def test_reject_float_inputs
    [
      { acres_delta: 1.5, bushels_to_feed: 1000, acres_to_plant: 100 },
      { acres_delta: 0, bushels_to_feed: 1000.0, acres_to_plant: 100 },
      { acres_delta: 0, bushels_to_feed: 1000, acres_to_plant: 100.25 }
    ].each do |bad_order|
      err = assert_raises(InvalidOrderBoundsError) do
        CodingAdventures::Hamurabi.validate_order(@state, bad_order)
      end
      assert_equal "InvalidOrderBounds", err.code
    end
  end

  def test_reject_string_coercion_inputs
    [
      { acres_delta: "100", bushels_to_feed: 1000, acres_to_plant: 100 },
      { acres_delta: 0, bushels_to_feed: "2000 bushels", acres_to_plant: 100 },
      { acres_delta: 0, bushels_to_feed: 1000, acres_to_plant: "100" }
    ].each do |bad_order|
      err = assert_raises(InvalidOrderBoundsError) do
        CodingAdventures::Hamurabi.validate_order(@state, bad_order)
      end
      assert_equal "InvalidOrderBounds", err.code
    end
  end

  def test_reject_boolean_inputs
    [
      { acres_delta: true, bushels_to_feed: 1000, acres_to_plant: 100 },
      { acres_delta: 0, bushels_to_feed: false, acres_to_plant: 100 },
      { acres_delta: 0, bushels_to_feed: 1000, acres_to_plant: true }
    ].each do |bad_order|
      err = assert_raises(InvalidOrderBoundsError) do
        CodingAdventures::Hamurabi.validate_order(@state, bad_order)
      end
      assert_equal "InvalidOrderBounds", err.code
    end
  end

  def test_reject_nil_in_order_attributes
    order = TurnOrder.new(acres_delta: 0, bushels_to_feed: nil, acres_to_plant: 100)
    assert_raises(InvalidOrderBoundsError) do
      CodingAdventures::Hamurabi.validate_order(@state, order)
    end
  end

  # CWE-190: Integer Overflow guards
  def test_acres_delta_i32_bounds
    # Exceeds signed 32-bit positive bound
    order_hi = { acres_delta: 2_147_483_648, bushels_to_feed: 0, acres_to_plant: 0 }
    assert_raises(InvalidOrderBoundsError) { CodingAdventures::Hamurabi.validate_order(@state, order_hi) }

    # Exceeds signed 32-bit negative bound
    order_lo = { acres_delta: -2_147_483_649, bushels_to_feed: 0, acres_to_plant: 0 }
    assert_raises(InvalidOrderBoundsError) { CodingAdventures::Hamurabi.validate_order(@state, order_lo) }
  end

  def test_bushels_u32_bounds
    order = { acres_delta: 0, bushels_to_feed: 4_294_967_296, acres_to_plant: 0 }
    assert_raises(InvalidOrderBoundsError) { CodingAdventures::Hamurabi.validate_order(@state, order) }

    order_neg = { acres_delta: 0, bushels_to_feed: -1, acres_to_plant: 0 }
    assert_raises(InvalidOrderBoundsError) { CodingAdventures::Hamurabi.validate_order(@state, order_neg) }
  end

  def test_acres_to_plant_u32_bounds
    order = { acres_delta: 0, bushels_to_feed: 0, acres_to_plant: 4_294_967_296 }
    assert_raises(InvalidOrderBoundsError) { CodingAdventures::Hamurabi.validate_order(@state, order) }

    order_neg = { acres_delta: 0, bushels_to_feed: 0, acres_to_plant: -1 }
    assert_raises(InvalidOrderBoundsError) { CodingAdventures::Hamurabi.validate_order(@state, order_neg) }
  end

  def test_harvest_u32_overflow_rejection
    # 900M acres planted * 5 harvest per acre = 4.5B > u32::MAX
    high_plant_state = HamurabiState.new(
      year: 1,
      population: 500_000_000,
      bushels: 2_000_000_000,
      acres: 1_000_000_000,
      land_price: 20,
      harvest_per_acre: 3,
      rats_eaten: 0,
      starved_last_year: 0,
      starved_total: 0,
      percent_starved_sum: 0,
      immigrants_last_year: 0,
      plague_struck: false,
      status: GameStatus::PLAYING,
      rng_state: 12345,
      rng_inc: 109
    )
    overflow_order = { acres_delta: 0, bushels_to_feed: 0, acres_to_plant: 900_000_000 }
    err = assert_raises(InvalidOrderBoundsError) do
      CodingAdventures::Hamurabi.validate_order(high_plant_state, overflow_order)
    end
    assert_equal "InvalidOrderBounds", err.code
  end

  # CWE-400 / CWE-835: Even rng_inc prevention
  def test_reject_even_rng_inc
    [0, 2, 4, 108, 0x1000].each do |even_inc|
      bad_state = HamurabiState.new(
        year: 1,
        population: 100,
        bushels: 2800,
        acres: 1000,
        land_price: 20,
        harvest_per_acre: 3,
        rats_eaten: 200,
        starved_last_year: 0,
        starved_total: 0,
        percent_starved_sum: 0,
        immigrants_last_year: 5,
        plague_struck: false,
        status: GameStatus::PLAYING,
        rng_state: 12345,
        rng_inc: even_inc
      )
      valid_order = { acres_delta: 0, bushels_to_feed: 2000, acres_to_plant: 800 }
      assert_raises(InvalidOrderBoundsError) do
        CodingAdventures::Hamurabi.validate_order(bad_state, valid_order)
      end
    end
  end

  # State invariant bounds violations
  def test_reject_invalid_reign_year
    [0, 11, 255].each do |bad_year|
      bad_state = HamurabiState.new(
        year: bad_year,
        population: 100,
        bushels: 2800,
        acres: 1000,
        land_price: 20,
        harvest_per_acre: 3,
        rats_eaten: 200,
        starved_last_year: 0,
        starved_total: 0,
        percent_starved_sum: 0,
        immigrants_last_year: 5,
        plague_struck: false,
        status: GameStatus::PLAYING,
        rng_state: 12345,
        rng_inc: 109
      )
      valid_order = { acres_delta: 0, bushels_to_feed: 2000, acres_to_plant: 800 }
      assert_raises(InvalidOrderBoundsError) do
        CodingAdventures::Hamurabi.validate_order(bad_state, valid_order)
      end
    end
  end

  def test_reject_invalid_land_price
    [0, 16, 27, 100].each do |bad_price|
      bad_state = HamurabiState.new(
        year: 1,
        population: 100,
        bushels: 2800,
        acres: 1000,
        land_price: bad_price,
        harvest_per_acre: 3,
        rats_eaten: 200,
        starved_last_year: 0,
        starved_total: 0,
        percent_starved_sum: 0,
        immigrants_last_year: 5,
        plague_struck: false,
        status: GameStatus::PLAYING,
        rng_state: 12345,
        rng_inc: 109
      )
      valid_order = { acres_delta: 0, bushels_to_feed: 2000, acres_to_plant: 800 }
      assert_raises(InvalidOrderBoundsError) do
        CodingAdventures::Hamurabi.validate_order(bad_state, valid_order)
      end
    end
  end

  def test_reject_invalid_harvest_yield
    [0, 6, 99].each do |bad_harvest|
      bad_state = HamurabiState.new(
        year: 1,
        population: 100,
        bushels: 2800,
        acres: 1000,
        land_price: 20,
        harvest_per_acre: bad_harvest,
        rats_eaten: 200,
        starved_last_year: 0,
        starved_total: 0,
        percent_starved_sum: 0,
        immigrants_last_year: 5,
        plague_struck: false,
        status: GameStatus::PLAYING,
        rng_state: 12345,
        rng_inc: 109
      )
      valid_order = { acres_delta: 0, bushels_to_feed: 2000, acres_to_plant: 800 }
      assert_raises(InvalidOrderBoundsError) do
        CodingAdventures::Hamurabi.validate_order(bad_state, valid_order)
      end
    end
  end

  # Invalid seed bounds in new_game
  def test_reject_invalid_seeds
    [-1, 1 << 64, "seed", 3.14, nil].each do |bad_seed|
      assert_raises(InvalidOrderBoundsError) do
        CodingAdventures::Hamurabi.new_game(bad_seed)
      end
    end
  end

  # Freeze immutability on returned state
  def test_freeze_fresh_value_semantics
    order = { acres_delta: 0, bushels_to_feed: 2000, acres_to_plant: 800 }
    next_state = CodingAdventures::Hamurabi.step_year(@state, order)
    assert next_state.frozen?, "Returned state must be frozen (VER04m fresh)"
    assert_raises(FrozenError) { next_state.instance_variable_set(:@population, 999) }
  end

  # Regression: do not silently coerce nil or false order fields to 0
  def test_reject_nil_and_boolean_in_step_year_order
    bad_order = { acres_delta: false, bushels_to_feed: nil, acres_to_plant: 100 }
    err = assert_raises(InvalidOrderBoundsError) do
      CodingAdventures::Hamurabi.step_year(@state, bad_order)
    end
    assert_equal "InvalidOrderBounds", err.code

    assert_raises(InvalidOrderBoundsError) do
      TurnOrder.from(bad_order)
    end
  end

  # Regression: state status check precedes order validation in step_year
  def test_step_year_ended_state_precedes_invalid_order
    ended_state = HamurabiState.new(
      year: 11,
      population: 100,
      bushels: 2800,
      acres: 1000,
      land_price: 20,
      harvest_per_acre: 3,
      rats_eaten: 0,
      starved_last_year: 0,
      starved_total: 0,
      percent_starved_sum: 0,
      immigrants_last_year: 0,
      plague_struck: false,
      status: GameStatus.new(:won, rating: Rating::CHARLEMAGNE),
      rng_state: 12345,
      rng_inc: 109
    )
    err = assert_raises(GameAlreadyEndedError) do
      CodingAdventures::Hamurabi.step_year(ended_state, "invalid")
    end
    assert_equal "GameAlreadyEnded", err.code
  end

  # Regression: enforce Integer and 0..MASK32 on starved_total and percent_starved_sum
  def test_reject_invalid_and_out_of_range_starved_total_and_percent_sum
    valid_order = { acres_delta: 0, bushels_to_feed: 2000, acres_to_plant: 800 }
    [-1, 1 << 32, "zero", 3.14, nil, false].each do |bad_val|
      bad_starved_total = HamurabiState.new(
        year: 1,
        population: 100,
        bushels: 2800,
        acres: 1000,
        land_price: 20,
        harvest_per_acre: 3,
        rats_eaten: 0,
        starved_last_year: 0,
        starved_total: bad_val,
        percent_starved_sum: 0,
        immigrants_last_year: 5,
        plague_struck: false,
        status: GameStatus::PLAYING,
        rng_state: 12345,
        rng_inc: 109
      )
      assert_raises(InvalidOrderBoundsError) do
        CodingAdventures::Hamurabi.validate_order(bad_starved_total, valid_order)
      end

      bad_percent_sum = HamurabiState.new(
        year: 1,
        population: 100,
        bushels: 2800,
        acres: 1000,
        land_price: 20,
        harvest_per_acre: 3,
        rats_eaten: 0,
        starved_last_year: 0,
        starved_total: 0,
        percent_starved_sum: bad_val,
        immigrants_last_year: 5,
        plague_struck: false,
        status: GameStatus::PLAYING,
        rng_state: 12345,
        rng_inc: 109
      )
      assert_raises(InvalidOrderBoundsError) do
        CodingAdventures::Hamurabi.validate_order(bad_percent_sum, valid_order)
      end
    end
  end

  # Regression: enforce Integer and 0..MASK64 on rng_state
  def test_reject_invalid_and_out_of_range_rng_state
    valid_order = { acres_delta: 0, bushels_to_feed: 2000, acres_to_plant: 800 }
    [-1, 1 << 64, "seed", 3.14, nil].each do |bad_val|
      bad_state = HamurabiState.new(
        year: 1,
        population: 100,
        bushels: 2800,
        acres: 1000,
        land_price: 20,
        harvest_per_acre: 3,
        rats_eaten: 0,
        starved_last_year: 0,
        starved_total: 0,
        percent_starved_sum: 0,
        immigrants_last_year: 5,
        plague_struck: false,
        status: GameStatus::PLAYING,
        rng_state: bad_val,
        rng_inc: 109
      )
      assert_raises(InvalidOrderBoundsError) do
        CodingAdventures::Hamurabi.validate_order(bad_state, valid_order)
      end
    end
  end
end
