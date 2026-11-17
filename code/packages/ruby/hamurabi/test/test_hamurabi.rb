# frozen_string_literal: true

require_relative "test_helper"

class TestHamurabi < Minitest::Test
  include CodingAdventures::Hamurabi

  def test_initial_state
    state = CodingAdventures::Hamurabi.new_game(1337)
    assert_equal 1, state.year
    assert_equal 100, state.population
    assert_equal 2800, state.bushels
    assert_equal 1000, state.acres
    assert_includes 17..26, state.land_price
    assert_equal 3, state.harvest_per_acre
    assert_equal 200, state.rats_eaten
    assert_equal 0, state.starved_last_year
    assert_equal 0, state.starved_total
    assert_equal 0, state.percent_starved_sum
    assert_equal 5, state.immigrants_last_year
    refute state.plague_struck
    assert state.status.playing?
    assert_equal "Playing", state.status.to_s
    assert_equal 109, state.rng_inc
    assert state.frozen?
  end

  def test_all_eight_error_arms
    state = CodingAdventures::Hamurabi.new_game(1337)
    normal_order = TurnOrder.new(acres_delta: 0, bushels_to_feed: 2000, acres_to_plant: 100)

    # Arm 1: GameAlreadyEnded
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
    err = assert_raises(GameAlreadyEndedError) { validate_order(ended_state, normal_order) }
    assert_equal "GameAlreadyEnded", err.code
    assert_equal :game_already_ended, err.class.symbolic_name
    assert_raises(GameAlreadyEndedError) { step_year(ended_state, normal_order) }

    # Arm 2: InvalidOrderBounds (invalid land_price invariant)
    bad_price_state = HamurabiState.new(
      year: 1,
      population: 100,
      bushels: 2800,
      acres: 1000,
      land_price: 10, # invalid: must be 17..26
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
    err = assert_raises(InvalidOrderBoundsError) { validate_order(bad_price_state, normal_order) }
    assert_equal "InvalidOrderBounds", err.code

    # Overflow in bushels revenue on land sale
    overflow_sell = TurnOrder.new(acres_delta: -1_000_000_000, bushels_to_feed: 0, acres_to_plant: 0)
    rich_state = HamurabiState.new(
      year: 1,
      population: 100,
      bushels: 4_000_000_000,
      acres: 2_000_000_000,
      land_price: 25,
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
    assert_raises(InvalidOrderBoundsError) { validate_order(rich_state, overflow_sell) }

    # Arm 3: InsufficientBushelsForLand
    buy_too_much = TurnOrder.new(acres_delta: 200, bushels_to_feed: 0, acres_to_plant: 0)
    err = assert_raises(InsufficientBushelsForLandError) { validate_order(state, buy_too_much) }
    assert_equal "InsufficientBushelsForLand", err.code
    assert_raises(InsufficientBushelsForLandError) { step_year(state, buy_too_much) }

    # Arm 4: InsufficientAcresToSell
    sell_too_much = TurnOrder.new(acres_delta: -1001, bushels_to_feed: 0, acres_to_plant: 0)
    err = assert_raises(InsufficientAcresToSellError) { validate_order(state, sell_too_much) }
    assert_equal "InsufficientAcresToSell", err.code
    assert_raises(InsufficientAcresToSellError) { step_year(state, sell_too_much) }

    # Arm 5: InsufficientBushelsToFeed
    feed_too_much = TurnOrder.new(acres_delta: 0, bushels_to_feed: 2801, acres_to_plant: 0)
    err = assert_raises(InsufficientBushelsToFeedError) { validate_order(state, feed_too_much) }
    assert_equal "InsufficientBushelsToFeed", err.code
    assert_raises(InsufficientBushelsToFeedError) { step_year(state, feed_too_much) }

    # Arm 6: InsufficientAcresToPlant
    plant_too_much_land = TurnOrder.new(acres_delta: 0, bushels_to_feed: 0, acres_to_plant: 1001)
    err = assert_raises(InsufficientAcresToPlantError) { validate_order(state, plant_too_much_land) }
    assert_equal "InsufficientAcresToPlant", err.code
    assert_raises(InsufficientAcresToPlantError) { step_year(state, plant_too_much_land) }

    # Arm 7: InsufficientPopulationToPlant
    pop_state = HamurabiState.new(
      year: 1,
      population: 10, # Can only tend 10 * 10 = 100 acres
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
    plant_too_much_pop = TurnOrder.new(acres_delta: 0, bushels_to_feed: 0, acres_to_plant: 101)
    err = assert_raises(InsufficientPopulationToPlantError) { validate_order(pop_state, plant_too_much_pop) }
    assert_equal "InsufficientPopulationToPlant", err.code
    assert_raises(InsufficientPopulationToPlantError) { step_year(pop_state, plant_too_much_pop) }

    # Arm 8: InsufficientGrainForSeed
    # Feed 2798 bushels leaving 2 bushels. 2 bushels can seed 4 acres. Attempting to plant 5 acres needs 3 bushels.
    short_seed_order = TurnOrder.new(acres_delta: 0, bushels_to_feed: 2798, acres_to_plant: 5)
    err = assert_raises(InsufficientGrainForSeedError) { validate_order(state, short_seed_order) }
    assert_equal "InsufficientGrainForSeed", err.code
    assert_raises(InsufficientGrainForSeedError) { step_year(state, short_seed_order) }
  end

  def test_format_report_playing
    state = CodingAdventures::Hamurabi.new_game(1337)
    report = CodingAdventures::Hamurabi.format_report(state)
    assert_includes report, "HAMURABI: I BEG TO REPORT TO YOU"
    assert_includes report, "IN YEAR 1, 0 PEOPLE STARVED, 5 CAME TO THE CITY."
    assert_includes report, "POPULATION IS NOW 100."
  end

  def test_format_report_impeached
    status = GameStatus.new(:impeached, starved_this_turn: 50, percent: 50)
    state = HamurabiState.new(
      year: 2,
      population: 50,
      bushels: 1000,
      acres: 1000,
      land_price: 20,
      harvest_per_acre: 3,
      rats_eaten: 0,
      starved_last_year: 50,
      starved_total: 50,
      percent_starved_sum: 50,
      immigrants_last_year: 0,
      plague_struck: false,
      status: status,
      rng_state: 1,
      rng_inc: 109
    )
    report = CodingAdventures::Hamurabi.format_report(state)
    assert_includes report, "YOU STARVED 50 PEOPLE IN ONE YEAR!!!"
    assert_includes report, "DUE TO THIS EXTREME MISMANAGEMENT YOU HAVE BEEN IMPEACHED"
  end

  def test_format_report_extinct
    state = HamurabiState.new(
      year: 2,
      population: 0,
      bushels: 0,
      acres: 1000,
      land_price: 20,
      harvest_per_acre: 1,
      rats_eaten: 0,
      starved_last_year: 100,
      starved_total: 100,
      percent_starved_sum: 100,
      immigrants_last_year: 0,
      plague_struck: false,
      status: GameStatus::EXTINCT,
      rng_state: 1,
      rng_inc: 109
    )
    report = CodingAdventures::Hamurabi.format_report(state)
    assert_includes report, "EVERYONE HAS DIED. THE CITY OF LAGASH IS EXTINCT!"
  end

  def test_format_report_won_ratings
    [Rating::CHARLEMAGNE, Rating::ACRE, Rating::NERO, Rating::DISASTER].each do |rating|
      state = HamurabiState.new(
        year: 11,
        population: 100,
        bushels: 3000,
        acres: 1000,
        land_price: 20,
        harvest_per_acre: 4,
        rats_eaten: 0,
        starved_last_year: 0,
        starved_total: 0,
        percent_starved_sum: 0,
        immigrants_last_year: 5,
        plague_struck: false,
        status: GameStatus.new(:won, rating: rating),
        rng_state: 1,
        rng_inc: 109
      )
      report = CodingAdventures::Hamurabi.format_report(state)
      assert_includes report, "IN YOUR 10-YEAR TERM OF OFFICE"
      assert_includes report, "A TOTAL OF 0 PEOPLE DIED!!"
    end
  end

  def test_state_immutability
    state = CodingAdventures::Hamurabi.new_game(42)
    assert state.frozen?
    assert_raises(FrozenError) { state.instance_variable_set(:@bushels, 999999) }
  end

  def test_state_hash_access_and_equality
    state = CodingAdventures::Hamurabi.new_game(42)
    assert_equal state.population, state[:population]
    assert_equal state.population, state["population"]
    h = state.to_h
    assert_equal 100, h[:population]
    restored = HamurabiState.from_h(h)
    assert_equal state, restored
    refute_equal state, 42
  end

  def test_turn_order_methods
    order = TurnOrder.new(acres_delta: 10, bushels_to_feed: 200, acres_to_plant: 50)
    assert_equal 10, order[:acres_delta]
    assert_equal 200, order["bushels_to_feed"]
    assert_equal 50, order[:acres_to_plant]
    assert_nil order[:invalid_key]

    h = order.to_h
    assert_equal 10, h[:acres_delta]

    order2 = TurnOrder.new(acres_delta: 10, bushels_to_feed: 200, acres_to_plant: 50)
    assert_equal order, order2
    refute_equal order, 123

    # From duck-typed object
    duck = Struct.new(:acres_delta, :bushels_to_feed, :acres_to_plant).new(-5, 100, 20)
    from_duck = TurnOrder.from(duck)
    assert_equal(-5, from_duck.acres_delta)

    # From invalid object
    assert_raises(InvalidOrderBoundsError) { TurnOrder.from("invalid") }
  end

  def test_game_status_predicates_and_parsing
    playing = GameStatus::PLAYING
    assert playing.playing?
    refute playing.won?
    refute playing.impeached?
    refute playing.extinct?
    assert_equal :playing, playing.tag
    assert_includes playing.inspect, "Playing"

    won = GameStatus.new(:won, rating: Rating::NERO)
    assert won.won?
    refute won.playing?

    impeached = GameStatus.new(:impeached, starved_this_turn: 10, percent: 20)
    assert impeached.impeached?

    extinct = GameStatus::EXTINCT
    assert extinct.extinct?

    assert_equal playing, "Playing"
    assert_equal playing, :playing
    refute_equal playing, 999

    assert_equal GameStatus::PLAYING, GameStatus.parse("Playing")
    assert_equal GameStatus::EXTINCT, GameStatus.parse("GameOverExtinction")
    assert_equal won, GameStatus.parse("Won(Nero)")
    assert_equal impeached, GameStatus.parse("Impeached(starved=10, percent=20)")
    custom = GameStatus.parse("CustomStatus")
    assert_equal :customstatus, custom.tag
  end

  def test_error_equality_and_defaults
    base_err = HamurabiError.new
    assert_equal "HamurabiError", base_err.code
    assert_equal "Hamurabi simulation error", base_err.message
    assert_equal :hamurabi_error, HamurabiError.symbolic_name

    specific = InsufficientGrainForSeedError.new
    assert_equal "InsufficientGrainForSeed", specific.code
    assert_equal specific, "InsufficientGrainForSeed"
    assert_equal specific, :insufficient_grain_for_seed
    assert_equal specific, InsufficientGrainForSeedError
    refute_equal specific, 12345
  end

  def test_format_report_plague_playing
    state = HamurabiState.new(
      year: 2,
      population: 50,
      bushels: 2000,
      acres: 1000,
      land_price: 20,
      harvest_per_acre: 3,
      rats_eaten: 0,
      starved_last_year: 0,
      starved_total: 0,
      percent_starved_sum: 0,
      immigrants_last_year: 0,
      plague_struck: true,
      status: GameStatus::PLAYING,
      rng_state: 1,
      rng_inc: 109
    )
    report = CodingAdventures::Hamurabi.format_report(state)
    assert_includes report, "A HORRIBLE PLAGUE STRUCK! HALF THE PEOPLE DIED."
  end
end
