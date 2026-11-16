# frozen_string_literal: true

module CodingAdventures
  module Hamurabi
    # LCG multiplier constant for 64-bit PCG32 (Knuth / Numerical Recipes).
    PCG_MULTIPLIER = 6_364_136_223_846_793_005

    # Stream ID for PCG32 PRNG stream in Hamurabi simulation.
    HAMURABI_STREAM_ID = 54

    # PCG32 increment constant: (stream_id << 1) | 1. Always odd (109).
    PCG_INCREMENT = (HAMURABI_STREAM_ID << 1) | 1

    # 64-bit and 32-bit masks to emulate unsigned integer wraparound.
    MASK64 = 0xFFFF_FFFF_FFFF_FFFF
    MASK32 = 0xFFFF_FFFF

    # ── PRNG Internal Primitives (PCG32 XSH RR 64/32) ──────────────────────────

    # Advances the 64-bit PCG32 state and returns [new_state, 32_bit_output].
    def pcg32_step(state, inc)
      old_state = state & MASK64
      new_state = (old_state * PCG_MULTIPLIER + inc) & MASK64
      xorshifted = (((old_state >> 18) ^ old_state) >> 27) & MASK32
      rot = (old_state >> 59) & 0x1F
      out = if rot.zero?
        xorshifted
      else
        ((xorshifted >> rot) | (xorshifted << (32 - rot))) & MASK32
      end
      [new_state, out]
    end

    # Draws an unbiased random integer in [min_val, max_val] using Knuth/O'Neill
    # rejection sampling against threshold = ((1 << 32) - span) % span.
    def pcg32_next_int_in_range(state, inc, min_val, max_val)
      span = (max_val - min_val) + 1
      threshold = ((1 << 32) - span) % span
      loop do
        state, r = pcg32_step(state, inc)
        if r >= threshold
          return [state, min_val + (r % span)]
        end
      end
    end

    # ── Simulation Engine API ───────────────────────────────────────────────────

    # Initializes a new, deterministic 10-year reign for ruler Hamurabi.
    #
    # Seeds PCG32 with `(seed, stream_id = 54)` and performs the initial draw
    # for Year 1 land price (range 17..=26).
    def new_game(seed)
      unless seed.is_a?(Integer) && seed >= 0 && seed <= MASK64
        raise InvalidOrderBoundsError, "Seed must be an unsigned 64-bit integer"
      end

      inc = PCG_INCREMENT
      rng_state = 0
      # Standard PCG initseq warm-up
      rng_state = (rng_state * PCG_MULTIPLIER + inc) & MASK64
      rng_state = (rng_state + (seed & MASK64)) & MASK64
      rng_state = (rng_state * PCG_MULTIPLIER + inc) & MASK64

      # Initial draw: land price for year 1
      rng_state, land_price = pcg32_next_int_in_range(rng_state, inc, 17, 26)

      HamurabiState.new(
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
        status: GameStatus::PLAYING,
        rng_state: rng_state,
        rng_inc: inc
      )
    end

    # Validates player orders against current state constraints without advancing state.
    #
    # Evaluates checks in the exact canonical order specified by GAME01:
    # 1. Status check (GameStatus::Playing)
    # 2. State invariant checks (VER04m state_inv & CWE-400 / CWE-835)
    # 3. Order parameter type validation (Integer) & bounds guards (i32/u32)
    # 4. Bounds and overflow checks on land trade
    # 5. Food allocation feasibility
    # 6. Planting acreage and labor limits
    # 7. Seed grain availability
    # 8. Maximum harvest overflow guard (CWE-190)
    def validate_order(state, order)
      state_obj = state.is_a?(HamurabiState) ? state : HamurabiState.from_h(state)

      # 1. Status Check
      unless state_obj.status.playing?
        raise GameAlreadyEndedError, "Game has already ended"
      end

      # 2. State Invariant Checks (VER04m state_inv & CWE-400 / CWE-835)
      # - Enforce odd rng_inc to prevent short period / infinite loop in PCG32 LCG
      unless state_obj.rng_inc.is_a?(Integer) && (state_obj.rng_inc & 1) == 1 &&
             state_obj.rng_inc >= 0 && state_obj.rng_inc <= MASK64
        raise InvalidOrderBoundsError, "Invalid rng_inc invariant: must be odd u64"
      end

      # - Enforce reign year in valid turn bounds [1, 10]
      unless state_obj.year.is_a?(Integer) && state_obj.year >= 1 && state_obj.year <= 10
        raise InvalidOrderBoundsError, "Invalid reign year: must be between 1 and 10"
      end

      # - Enforce land price within canonical range [17, 26]
      unless state_obj.land_price.is_a?(Integer) && state_obj.land_price >= 17 && state_obj.land_price <= 26
        raise InvalidOrderBoundsError, "Invalid land price: must be between 17 and 26"
      end

      # - Enforce harvest per acre within canonical range [1, 5]
      unless state_obj.harvest_per_acre.is_a?(Integer) && state_obj.harvest_per_acre >= 1 && state_obj.harvest_per_acre <= 5
        raise InvalidOrderBoundsError, "Invalid harvest yield: must be between 1 and 5"
      end

      # - Enforce 64-bit bounds on rng_state
      unless state_obj.rng_state.is_a?(Integer) &&
             state_obj.rng_state >= 0 && state_obj.rng_state <= MASK64
        raise InvalidOrderBoundsError, "Invalid rng_state: must be unsigned 64-bit integer"
      end

      # - Enforce 32-bit unsigned bounds on all u32 state fields
      u32_fields = {
        population: state_obj.population,
        bushels: state_obj.bushels,
        acres: state_obj.acres,
        rats_eaten: state_obj.rats_eaten,
        starved_last_year: state_obj.starved_last_year,
        starved_total: state_obj.starved_total,
        percent_starved_sum: state_obj.percent_starved_sum,
        immigrants_last_year: state_obj.immigrants_last_year
      }

      u32_fields.each do |name, val|
        unless val.is_a?(Integer) && val >= 0 && val <= MASK32
          raise InvalidOrderBoundsError, "State field '#{name}' must be an unsigned 32-bit integer"
        end
      end

      # 3. Order parameter extraction and strict type & bound validation (CWE-843, CWE-190)
      raw_ad = if order.is_a?(Hash)
        if order.key?(:acres_delta)
          order[:acres_delta]
        elsif order.key?("acres_delta")
          order["acres_delta"]
        end
      elsif order.respond_to?(:acres_delta)
        order.acres_delta
      end

      raw_bf = if order.is_a?(Hash)
        if order.key?(:bushels_to_feed)
          order[:bushels_to_feed]
        elsif order.key?("bushels_to_feed")
          order["bushels_to_feed"]
        end
      elsif order.respond_to?(:bushels_to_feed)
        order.bushels_to_feed
      end

      raw_ap = if order.is_a?(Hash)
        if order.key?(:acres_to_plant)
          order[:acres_to_plant]
        elsif order.key?("acres_to_plant")
          order["acres_to_plant"]
        end
      elsif order.respond_to?(:acres_to_plant)
        order.acres_to_plant
      end

      # Strict rejection of Floats, Strings, nil, Booleans
      unless raw_ad.is_a?(Integer) && raw_bf.is_a?(Integer) && raw_ap.is_a?(Integer)
        raise InvalidOrderBoundsError, "Order parameters must be integers (rejecting float/string/bool/nil)"
      end

      # Signed 32-bit bounds on acres_delta [-2^31, 2^31 - 1]
      unless raw_ad >= -0x8000_0000 && raw_ad <= 0x7FFF_FFFF
        raise InvalidOrderBoundsError, "acres_delta exceeds 32-bit signed integer bounds"
      end

      # Unsigned 32-bit bounds on bushels_to_feed [0, 2^32 - 1]
      unless raw_bf >= 0 && raw_bf <= MASK32
        raise InvalidOrderBoundsError, "bushels_to_feed exceeds 32-bit unsigned integer bounds"
      end

      # Unsigned 32-bit bounds on acres_to_plant [0, 2^32 - 1]
      unless raw_ap >= 0 && raw_ap <= MASK32
        raise InvalidOrderBoundsError, "acres_to_plant exceeds 32-bit unsigned integer bounds"
      end

      # 4. Bounds & Overflow Guard on Land Trade
      if raw_ad > 0
        cost = raw_ad * state_obj.land_price
        if cost > state_obj.bushels
          raise InsufficientBushelsForLandError, "Insufficient bushels to buy specified acreage"
        end
      elsif raw_ad < 0
        sell_count = -raw_ad
        if sell_count > state_obj.acres
          raise InsufficientAcresToSellError, "Cannot sell more acres than the city owns"
        end
      end

      # 5. Land Trade Balances
      acres_after_trade = if raw_ad >= 0
        res = state_obj.acres + raw_ad
        raise InvalidOrderBoundsError, "Acres calculation overflowed u32" if res > MASK32
        res
      else
        state_obj.acres - (-raw_ad)
      end

      bushels_after_trade = if raw_ad > 0
        cost = raw_ad * state_obj.land_price
        state_obj.bushels - cost
      elsif raw_ad < 0
        sell_count = -raw_ad
        revenue = sell_count * state_obj.land_price
        res = state_obj.bushels + revenue
        raise InvalidOrderBoundsError, "Bushels revenue calculation overflowed u32" if res > MASK32
        res
      else
        state_obj.bushels
      end

      # 6. Food Allocation Check
      if raw_bf > bushels_after_trade
        raise InsufficientBushelsToFeedError, "Insufficient bushels in store to feed population"
      end
      bushels_after_feed = bushels_after_trade - raw_bf

      # 7. Planting Land & Labor Checks
      if raw_ap > acres_after_trade
        raise InsufficientAcresToPlantError, "Cannot plant more acres than owned after land trade"
      end

      max_labor_acres = state_obj.population * 10
      if raw_ap > max_labor_acres
        raise InsufficientPopulationToPlantError, "Insufficient population to tend planted acreage"
      end

      # 8. Planting Seed Check: each bushel seeds 2 acres (ceil(acres / 2))
      seed_needed = (raw_ap + 1) / 2
      if seed_needed > bushels_after_feed
        raise InsufficientGrainForSeedError, "Insufficient grain to seed planted acreage"
      end
      bushels_after_seed = bushels_after_feed - seed_needed

      # 9. Maximum Harvest Overflow Guard (CWE-190)
      max_harvest = raw_ap * 5
      max_stored = bushels_after_seed + max_harvest
      if max_stored > MASK32
        raise InvalidOrderBoundsError, "Maximum harvest calculation overflows u32"
      end

      true
    end

    # Advances the simulation by exactly one year according to player orders.
    #
    # Executes the strict two-phase transition:
    # - Phase A: Validation & constraint checking in canonical order.
    # - Phase B: Strictly locked PRNG draw sequence (harvest, rats, plague, starvation, immigration, next land price).
    #
    # Returns the updated immutable HamurabiState on success, or raises HamurabiError on refusal.
    # The original simulation state is never modified on error.
    def step_year(state, order)
      state_obj = state.is_a?(HamurabiState) ? state : HamurabiState.from_h(state)

      unless state_obj.status.playing?
        raise GameAlreadyEndedError, "Game has already ended"
      end

      order_obj = TurnOrder.from(order)

      validate_order(state_obj, order_obj)

      pcg_state = state_obj.rng_state
      pcg_inc = state_obj.rng_inc

      # Intermediate balances from validated trade
      acres_after_trade = if order_obj.acres_delta >= 0
        state_obj.acres + order_obj.acres_delta
      else
        state_obj.acres - (-order_obj.acres_delta)
      end

      bushels_after_trade = if order_obj.acres_delta > 0
        cost = order_obj.acres_delta * state_obj.land_price
        state_obj.bushels - cost
      elsif order_obj.acres_delta < 0
        revenue = (-order_obj.acres_delta) * state_obj.land_price
        state_obj.bushels + revenue
      else
        state_obj.bushels
      end

      bushels_after_feed = bushels_after_trade - order_obj.bushels_to_feed
      seed_needed = (order_obj.acres_to_plant + 1) / 2
      bushels_after_seed = bushels_after_feed - seed_needed

      # Step 1: Harvest Yield Draw
      # Draw 1: pcg.next_int_in_range(1, 5) -> harvest_per_acre (u32)
      pcg_state, harvest_per_acre = pcg32_next_int_in_range(pcg_state, pcg_inc, 1, 5)
      total_harvest = order_obj.acres_to_plant * harvest_per_acre
      bushels_stored = bushels_after_seed + total_harvest

      # Step 2: Rat Infestation Draws
      # Draw 2: pcg.next_int_in_range(1, 100) -> rat_roll (u32)
      pcg_state, rat_roll = pcg32_next_int_in_range(pcg_state, pcg_inc, 1, 100)
      rats_eaten = if rat_roll <= 40
        # Draw 3: pcg.next_int_in_range(1, 5) -> rat_factor (u32)
        pcg_state, rat_factor = pcg32_next_int_in_range(pcg_state, pcg_inc, 1, 5)
        eaten = bushels_stored / rat_factor
        [eaten, MASK32].min
      else
        0
      end
      bushels_final = [[bushels_stored - rats_eaten, 0].max, MASK32].min

      # Step 3: Plague Draw
      # Draw 4: pcg.next_int_in_range(1, 100) -> plague_roll (u32)
      pcg_state, plague_roll = pcg32_next_int_in_range(pcg_state, pcg_inc, 1, 100)
      plague_struck = (plague_roll <= 15)

      # Step 4: Starvation & Impeachment Evaluation
      people_fed = order_obj.bushels_to_feed / 20
      starved, survivors = if people_fed >= state_obj.population
        [0, state_obj.population]
      else
        [state_obj.population - people_fed, people_fed]
      end

      percent_starved = if state_obj.population > 0
        [((starved * 100) / state_obj.population), 100].min
      else
        0
      end
      new_starved_total = [state_obj.starved_total + starved, MASK32].min
      new_percent_sum = [state_obj.percent_starved_sum + percent_starved, MASK32].min

      new_status = if starved * 100 > state_obj.population * 45
        GameStatus.new(:impeached, starved_this_turn: starved, percent: percent_starved)
      elsif survivors == 0
        GameStatus::EXTINCT
      else
        GameStatus::PLAYING
      end

      # Step 5: Plague Resolution
      if plague_struck && new_status.playing?
        survivors /= 2
      end

      # Step 6: Immigration Draw
      # Draw 5: pcg.next_int_in_range(1, 5) -> imm_factor (u32)
      pcg_state, imm_factor = pcg32_next_int_in_range(pcg_state, pcg_inc, 1, 5)
      immigrants = if starved == 0 && new_status.playing?
        acres_term = acres_after_trade * 20
        inner = acres_term + bushels_final
        numerator = imm_factor * inner
        denominator = 100 * state_obj.population
        if denominator > 0
          [((numerator / denominator) + 1), MASK32].min
        else
          0
        end
      else
        0
      end

      final_population = [survivors + immigrants, MASK32].min

      # Step 7: Next Year Land Price Draw
      # Draw 6: pcg.next_int_in_range(17, 26) -> next_land_price (u32)
      pcg_state, next_land_price = pcg32_next_int_in_range(pcg_state, pcg_inc, 17, 26)

      # Step 8: Turn Advance & Rating
      next_year = [state_obj.year + 1, 255].min
      if next_year == 11 && new_status.playing?
        avg_starved = new_percent_sum / 10
        acres_per_person = final_population > 0 ? (acres_after_trade / final_population) : 0
        rating = if avg_starved <= 3 && acres_per_person >= 10
          Rating::CHARLEMAGNE
        elsif avg_starved <= 10 && acres_per_person >= 9
          Rating::ACRE
        elsif avg_starved <= 33 && acres_per_person >= 7
          Rating::NERO
        else
          Rating::DISASTER
        end
        new_status = GameStatus.new(:won, rating: rating)
      end

      HamurabiState.new(
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
        status: new_status,
        rng_state: pcg_state,
        rng_inc: pcg_inc
      )
    end

    # Formats the annual teletype report or final evaluation in authentic 1970s teletype styling.
    def format_report(state)
      state_obj = state.is_a?(HamurabiState) ? state : HamurabiState.from_h(state)

      case state_obj.status.tag
      when :impeached
        "YOU STARVED #{state_obj.status.starved_this_turn} PEOPLE IN ONE YEAR!!!\n" \
        "DUE TO THIS EXTREME MISMANAGEMENT YOU HAVE BEEN IMPEACHED AND THROWN OUT OF OFFICE!\n"
      when :extinct
        "EVERYONE HAS DIED. THE CITY OF LAGASH IS EXTINCT!\n"
      when :won
        avg_starved = state_obj.percent_starved_sum / 10
        acres_per_person = state_obj.population > 0 ? (state_obj.acres / state_obj.population) : 0
        rating_msg = case state_obj.status.rating.to_s
        when "Charlemagne"
          "A FANTASTIC PERFORMANCE! CHARLEMAGNE, DISRAELI, AND JEFFERSON COMBINED COULD NOT HAVE DONE BETTER!"
        when "Acre"
          "YOUR PERFORMANCE COULD HAVE BEEN WORSE, BUT YOU LEFT THEM REASONABLE ACREAGE."
        when "Nero"
          "YOUR PERFORMANCE LEAVES MUCH TO BE DESIRED. NERO WOULD HAVE BEEN PROUD OF YOU."
        when "Disaster"
          "YOU DID SO POORLY YOU HAVE BEEN DECLARED AN OFFICIAL NATIONAL DISASTER!"
        end
        "IN YOUR 10-YEAR TERM OF OFFICE, #{avg_starved} PERCENT OF THE\n" \
        "POPULATION STARVED PER YEAR ON AVERAGE.\n" \
        "A TOTAL OF #{state_obj.starved_total} PEOPLE DIED!!\n" \
        "YOU STARTED WITH 10 ACRES PER PERSON AND ENDED WITH\n" \
        "#{acres_per_person} ACRES PER PERSON.\n\n" \
        "#{rating_msg}\n"
      when :playing
        out = "HAMURABI: I BEG TO REPORT TO YOU,\n" \
              "IN YEAR #{state_obj.year}, #{state_obj.starved_last_year} PEOPLE STARVED, #{state_obj.immigrants_last_year} CAME TO THE CITY.\n"
        if state_obj.plague_struck
          out += "A HORRIBLE PLAGUE STRUCK! HALF THE PEOPLE DIED.\n"
        end
        out += "POPULATION IS NOW #{state_obj.population}.\n" \
               "THE CITY NOW OWNS #{state_obj.acres} ACRES.\n" \
               "YOU HARVESTED #{state_obj.harvest_per_acre} BUSHELS PER ACRE.\n" \
               "RATS ATE #{state_obj.rats_eaten} BUSHELS.\n" \
               "YOU NOW HAVE #{state_obj.bushels} BUSHELS IN STORE.\n" \
               "LAND IS TRADING AT #{state_obj.land_price} BUSHELS PER ACRE.\n"
        out
      end
    end
  end
end
