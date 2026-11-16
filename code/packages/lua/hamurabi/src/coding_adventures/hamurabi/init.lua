-- coding_adventures.hamurabi
-- ============================================================================
--
-- Pure, deterministic simulation core for the seminal 1968 mainframe resource
-- allocation game.
--
-- Architectural Guarantees:
--   - Zero ambient system authority: declared capabilities: [] (CAP001 / VER08a).
--   - Pure state transition: step_year(state, order) -> (HamurabiState, nil)
--     or (nil, HamurabiError).
--   - Explicit PCG32 pseudorandomness with stream ID 54 (inc = 109).
--   - Bit-exact integer arithmetic using 64-bit wrapping integers.
--   - Complete 26-CWE weakness closure (CWE-190, CWE-400, CWE-835, CWE-843).
--   - Immutable fresh records (VER04m): original state is never modified.
--
-- ============================================================================

local M = {}

M.VERSION = "0.1.0"

-- ============================================================================
-- CONSTANTS
-- ============================================================================

-- 64-bit LCG multiplier constant (Knuth / Numerical Recipes).
local PCG_MULTIPLIER = 6364136223846793005

-- Stream ID for Hamurabi PCG32 PRNG stream.
local HAMURABI_STREAM_ID = 54

-- Sequence increment for PCG32: (HAMURABI_STREAM_ID << 1) | 1 = 109.
local HAMURABI_INC = (HAMURABI_STREAM_ID << 1) | 1

M.PCG_MULTIPLIER = PCG_MULTIPLIER
M.HAMURABI_STREAM_ID = HAMURABI_STREAM_ID
M.HAMURABI_INC = HAMURABI_INC

local MASK32 = 0xFFFFFFFF
local TWO_32 = 1 << 32
local MAX_U32 = 0xFFFFFFFF
local MIN_I32 = -2147483648
local MAX_I32 = 2147483647

M.MAX_U32 = MAX_U32
M.MIN_I32 = MIN_I32
M.MAX_I32 = MAX_I32

-- ============================================================================
-- DOMAIN ENUMS & TYPES
-- ============================================================================

-- Rating awarded upon completing a 10-year term.
M.Rating = {
    Charlemagne = "Charlemagne",
    Acre = "Acre",
    Nero = "Nero",
    Disaster = "Disaster",
}

-- Closed enumeration of order rejection errors.
M.HamurabiError = {
    GameAlreadyEnded = "GameAlreadyEnded",
    InvalidOrderBounds = "InvalidOrderBounds",
    InsufficientBushelsForLand = "InsufficientBushelsForLand",
    InsufficientAcresToSell = "InsufficientAcresToSell",
    InsufficientBushelsToFeed = "InsufficientBushelsToFeed",
    InsufficientAcresToPlant = "InsufficientAcresToPlant",
    InsufficientPopulationToPlant = "InsufficientPopulationToPlant",
    InsufficientGrainForSeed = "InsufficientGrainForSeed",
}

-- Lifecycle status constructors and string constants.
M.GameStatus = {
    Playing = "Playing",
    GameOverExtinction = "GameOverExtinction",
    Won = function(rating)
        return "Won(" .. tostring(rating) .. ")"
    end,
    Impeached = function(starved, percent)
        return string.format("Impeached(starved=%d, percent=%d)", starved, percent)
    end,
}

-- Status helper predicates
function M.is_playing(status)
    return status == "Playing"
end

function M.is_game_over_extinction(status)
    return status == "GameOverExtinction"
end

function M.is_won(status)
    return type(status) == "string" and status:sub(1, 4) == "Won(" and status:sub(-1) == ")"
end

function M.get_won_rating(status)
    if M.is_won(status) then
        return status:sub(5, -2)
    end
    return nil
end

function M.is_impeached(status)
    return type(status) == "string" and status:sub(1, 10) == "Impeached(" and status:sub(-1) == ")"
end

function M.parse_impeached(status)
    if not M.is_impeached(status) then
        return nil
    end
    local starved, percent = status:match("^Impeached%(starved=(%d+),%s*percent=(%d+)%)$")
    if starved and percent then
        return math.tointeger(starved), math.tointeger(percent)
    end
    return nil
end

-- Aliases for camelCase convenience
M.isPlaying = M.is_playing
M.isWon = M.is_won
M.getWonRating = M.get_won_rating
M.isImpeached = M.is_impeached
M.parseImpeached = M.parse_impeached

-- ============================================================================
-- TYPE HARDENING (CWE-843 / CWE-190)
-- ============================================================================

local function is_int(v)
    return math.type(v) == "integer"
end

local function is_u32(v)
    return is_int(v) and v >= 0 and v <= MAX_U32
end

local function is_i32(v)
    return is_int(v) and v >= MIN_I32 and v <= MAX_I32
end

-- ============================================================================
-- PRNG PRIMITIVES (PCG32 XSH RR 64/32)
-- ============================================================================

local function pcg32_step(state, inc)
    local old_state = state
    local new_state = old_state * PCG_MULTIPLIER + inc
    local xorshifted = (((old_state >> 18) ~ old_state) >> 27) & MASK32
    local rot = (old_state >> 59) & 31
    local output = ((xorshifted >> rot) | (xorshifted << ((32 - rot) & 31))) & MASK32
    return new_state, output
end

local function pcg32_next_int_in_range(state, inc, min_val, max_val)
    local span = (max_val - min_val) + 1
    local threshold = (TWO_32 - span) % span
    while true do
        local next_state, r = pcg32_step(state, inc)
        state = next_state
        if r >= threshold then
            return state, min_val + (r % span)
        end
    end
end

-- ============================================================================
-- SIMULATION CORE API
-- ============================================================================

--- Initializes a new, deterministic 10-year reign for ruler Hamurabi.
---
--- @param seed integer 64-bit integer seed
--- @return table HamurabiState
function M.new_game(seed)
    if not is_int(seed) then
        error("new_game: seed must be an integer, got " .. type(seed))
    end

    local inc = HAMURABI_INC
    local state = 0

    -- Standard PCG initseq warm-up
    state = state * PCG_MULTIPLIER + inc
    state = state + seed
    state = state * PCG_MULTIPLIER + inc

    -- Initial draw: land price for year 1 (17..=26)
    local next_state, land_price = pcg32_next_int_in_range(state, inc, 17, 26)

    return {
        year = 1,
        population = 100,
        bushels = 2800,
        acres = 1000,
        land_price = land_price,
        harvest_per_acre = 3,
        rats_eaten = 200,
        starved_last_year = 0,
        starved_total = 0,
        percent_starved_sum = 0,
        immigrants_last_year = 5,
        plague_struck = false,
        status = "Playing",
        rng_state = next_state,
        rng_inc = inc,
    }
end

--- Validates player orders against current state constraints without advancing state.
---
--- Canonical evaluation order:
--- 1. Status Check (Playing)
--- 2. Bounds and overflow checks (CWE-400, CWE-835, CWE-190, CWE-843)
--- 3. Land trade resource availability
--- 4. Land trade balance overflow
--- 5. Food allocation feasibility
--- 6. Planting acreage and labor limits
--- 7. Seed grain availability
--- 8. Maximum harvest overflow guard
---
--- @param state table HamurabiState
--- @param order table TurnOrder
--- @return boolean ok
--- @return string|nil err HamurabiError
function M.validate_order(state, order)
    if type(state) ~= "table" or type(order) ~= "table" then
        return false, M.HamurabiError.InvalidOrderBounds
    end

    -- 1. Status Check
    if state.status ~= "Playing" then
        return false, M.HamurabiError.GameAlreadyEnded
    end

    -- State Invariant Checks (VER04m state_inv & CWE-400 / CWE-835)
    -- Enforce odd rng_inc to prevent infinite loop / short period in PCG32 LCG
    if not is_int(state.rng_inc) or (state.rng_inc & 1) ~= 1 then
        return false, M.HamurabiError.InvalidOrderBounds
    end

    -- Enforce reign year in valid turn bounds [1, 10]
    if not is_int(state.year) or state.year < 1 or state.year > 10 then
        return false, M.HamurabiError.InvalidOrderBounds
    end

    -- Enforce land price within canonical range [17, 26]
    if not is_int(state.land_price) or state.land_price < 17 or state.land_price > 26 then
        return false, M.HamurabiError.InvalidOrderBounds
    end

    -- Enforce harvest per acre within canonical range [1, 5]
    if not is_int(state.harvest_per_acre) or state.harvest_per_acre < 1 or state.harvest_per_acre > 5 then
        return false, M.HamurabiError.InvalidOrderBounds
    end

    -- Validate remaining state fields
    if not is_u32(state.population) or not is_u32(state.bushels) or not is_u32(state.acres) or
       not is_u32(state.rats_eaten) or not is_u32(state.starved_last_year) or
       not is_u32(state.starved_total) or not is_u32(state.percent_starved_sum) or
       not is_u32(state.immigrants_last_year) or type(state.plague_struck) ~= "boolean" or
       not is_int(state.rng_state) then
        return false, M.HamurabiError.InvalidOrderBounds
    end

    -- Validate order types & bounds
    if not is_i32(order.acres_delta) or not is_u32(order.bushels_to_feed) or not is_u32(order.acres_to_plant) then
        return false, M.HamurabiError.InvalidOrderBounds
    end

    -- 2. Bounds & Overflow Guard on Land Trade
    if order.acres_delta > 0 then
        local buy_count = order.acres_delta
        local cost = buy_count * state.land_price
        if cost > state.bushels then
            return false, M.HamurabiError.InsufficientBushelsForLand
        end
    elseif order.acres_delta < 0 then
        local sell_count = -order.acres_delta
        if sell_count > state.acres then
            return false, M.HamurabiError.InsufficientAcresToSell
        end
    end

    -- 3. Land Trade Balances
    local acres_after_trade
    if order.acres_delta >= 0 then
        acres_after_trade = state.acres + order.acres_delta
        if acres_after_trade > MAX_U32 then
            return false, M.HamurabiError.InvalidOrderBounds
        end
    else
        local sell_count = -order.acres_delta
        acres_after_trade = state.acres - sell_count
    end

    local bushels_after_trade
    if order.acres_delta > 0 then
        local cost = order.acres_delta * state.land_price
        bushels_after_trade = state.bushels - cost
    elseif order.acres_delta < 0 then
        local sell_count = -order.acres_delta
        local revenue = sell_count * state.land_price
        bushels_after_trade = state.bushels + revenue
        if bushels_after_trade > MAX_U32 then
            return false, M.HamurabiError.InvalidOrderBounds
        end
    else
        bushels_after_trade = state.bushels
    end

    -- 4. Food Allocation Check
    if order.bushels_to_feed > bushels_after_trade then
        return false, M.HamurabiError.InsufficientBushelsToFeed
    end
    local bushels_after_feed = bushels_after_trade - order.bushels_to_feed

    -- 5. Planting Land & Labor Checks
    if order.acres_to_plant > acres_after_trade then
        return false, M.HamurabiError.InsufficientAcresToPlant
    end

    local max_labor_acres = state.population * 10
    if order.acres_to_plant > max_labor_acres then
        return false, M.HamurabiError.InsufficientPopulationToPlant
    end

    -- 6. Planting Seed Check: each bushel seeds 2 acres: (acres_to_plant + 1) // 2
    local seed_needed = (order.acres_to_plant + 1) // 2
    if seed_needed > bushels_after_feed then
        return false, M.HamurabiError.InsufficientGrainForSeed
    end
    local bushels_after_seed = bushels_after_feed - seed_needed

    -- 7. Maximum Harvest Overflow Guard (CWE-190)
    local max_harvest = order.acres_to_plant * 5
    local max_stored = bushels_after_seed + max_harvest
    if max_stored > MAX_U32 then
        return false, M.HamurabiError.InvalidOrderBounds
    end

    return true, nil
end

--- Advances the simulation by exactly one year according to player orders.
--- Returns a fresh HamurabiState record on success, or (nil, HamurabiError) on refusal.
--- Input state is never modified.
---
--- @param state table HamurabiState
--- @param order table TurnOrder
--- @return table|nil next_state
--- @return string|nil err HamurabiError
function M.step_year(state, order)
    local ok, err = M.validate_order(state, order)
    if not ok then
        return nil, err
    end

    local rng_state = state.rng_state
    local rng_inc = state.rng_inc

    -- Intermediate balances from validated trade
    local acres_after_trade
    if order.acres_delta >= 0 then
        acres_after_trade = state.acres + order.acres_delta
    else
        local sell_count = -order.acres_delta
        acres_after_trade = state.acres - sell_count
    end

    local bushels_after_trade
    if order.acres_delta > 0 then
        local cost = order.acres_delta * state.land_price
        bushels_after_trade = state.bushels - cost
    elseif order.acres_delta < 0 then
        local sell_count = -order.acres_delta
        local revenue = sell_count * state.land_price
        bushels_after_trade = state.bushels + revenue
    else
        bushels_after_trade = state.bushels
    end

    local bushels_after_feed = bushels_after_trade - order.bushels_to_feed
    local seed_needed = (order.acres_to_plant + 1) // 2
    local bushels_after_seed = bushels_after_feed - seed_needed

    -- Step 1: Harvest Yield Draw
    -- Draw 1: pcg.next_int_in_range(1, 5) -> harvest_per_acre
    local harvest_per_acre
    rng_state, harvest_per_acre = pcg32_next_int_in_range(rng_state, rng_inc, 1, 5)
    local total_harvest = order.acres_to_plant * harvest_per_acre
    local bushels_stored = bushels_after_seed + total_harvest

    -- Step 2: Rat Infestation Draws
    -- Draw 2: pcg.next_int_in_range(1, 100) -> rat_roll
    local rat_roll
    rng_state, rat_roll = pcg32_next_int_in_range(rng_state, rng_inc, 1, 100)
    local rats_eaten = 0
    if rat_roll <= 40 then
        -- Draw 3: pcg.next_int_in_range(1, 5) -> rat_factor
        local rat_factor
        rng_state, rat_factor = pcg32_next_int_in_range(rng_state, rng_inc, 1, 5)
        rats_eaten = bushels_stored // rat_factor
        if rats_eaten > MAX_U32 then rats_eaten = MAX_U32 end
    end
    local bushels_final = bushels_stored - rats_eaten
    if bushels_final < 0 then bushels_final = 0 end
    if bushels_final > MAX_U32 then bushels_final = MAX_U32 end

    -- Step 3: Plague Draw
    -- Draw 4: pcg.next_int_in_range(1, 100) -> plague_roll
    local plague_roll
    rng_state, plague_roll = pcg32_next_int_in_range(rng_state, rng_inc, 1, 100)
    local plague_struck = (plague_roll <= 15)

    -- Step 4: Starvation & Impeachment Evaluation
    local people_fed = order.bushels_to_feed // 20
    local starved, survivors
    if people_fed >= state.population then
        starved = 0
        survivors = state.population
    else
        starved = state.population - people_fed
        survivors = people_fed
    end

    local percent_starved = 0
    if state.population > 0 then
        percent_starved = (starved * 100) // state.population
        if percent_starved > 100 then percent_starved = 100 end
    end

    local new_starved_total = state.starved_total + starved
    if new_starved_total > MAX_U32 then new_starved_total = MAX_U32 end

    local new_percent_sum = state.percent_starved_sum + percent_starved
    if new_percent_sum > MAX_U32 then new_percent_sum = MAX_U32 end

    local new_status
    if starved * 100 > state.population * 45 then
        new_status = string.format("Impeached(starved=%d, percent=%d)", starved, percent_starved)
    elseif survivors == 0 then
        new_status = "GameOverExtinction"
    else
        new_status = "Playing"
    end

    -- Step 5: Plague Resolution
    if plague_struck and new_status == "Playing" then
        survivors = survivors // 2
    end

    -- Step 6: Immigration Draw (unconditional to preserve PRNG stream alignment)
    -- Draw 5: pcg.next_int_in_range(1, 5) -> imm_factor
    local imm_factor
    rng_state, imm_factor = pcg32_next_int_in_range(rng_state, rng_inc, 1, 5)
    local immigrants = 0
    if starved == 0 and new_status == "Playing" then
        local numerator = imm_factor * (20 * acres_after_trade + bushels_final)
        local denominator = 100 * state.population
        if denominator > 0 then
            immigrants = (numerator // denominator) + 1
            if immigrants > MAX_U32 then immigrants = MAX_U32 end
        end
    end

    local final_population = survivors + immigrants
    if final_population > MAX_U32 then final_population = MAX_U32 end

    -- Step 7: Next Year Land Price Draw
    -- Draw 6: pcg.next_int_in_range(17, 26) -> next_land_price
    local next_land_price
    rng_state, next_land_price = pcg32_next_int_in_range(rng_state, rng_inc, 17, 26)

    -- Step 8: Turn Advance & Rating
    local next_year = state.year + 1
    if next_year > 255 then next_year = 255 end
    if next_year == 11 and new_status == "Playing" then
        local avg_starved = new_percent_sum // 10
        local acres_per_person = 0
        if final_population > 0 then
            acres_per_person = acres_after_trade // final_population
        end
        local rating
        if avg_starved <= 3 and acres_per_person >= 10 then
            rating = "Charlemagne"
        elseif avg_starved <= 10 and acres_per_person >= 9 then
            rating = "Acre"
        elseif avg_starved <= 33 and acres_per_person >= 7 then
            rating = "Nero"
        else
            rating = "Disaster"
        end
        new_status = "Won(" .. rating .. ")"
    end

    return {
        year = next_year,
        population = final_population,
        bushels = bushels_final,
        acres = acres_after_trade,
        land_price = next_land_price,
        harvest_per_acre = harvest_per_acre,
        rats_eaten = rats_eaten,
        starved_last_year = starved,
        starved_total = new_starved_total,
        percent_starved_sum = new_percent_sum,
        immigrants_last_year = immigrants,
        plague_struck = plague_struck,
        status = new_status,
        rng_state = rng_state,
        rng_inc = rng_inc,
    }, nil
end

--- Formats the annual teletype report or final evaluation in authentic 1970s styling.
---
--- @param state table HamurabiState
--- @return string
function M.format_report(state)
    local status = state.status

    if M.is_impeached(status) then
        local starved, _ = M.parse_impeached(status)
        return string.format(
            "YOU STARVED %d PEOPLE IN ONE YEAR!!!\n" ..
            "DUE TO THIS EXTREME MISMANAGEMENT YOU HAVE BEEN IMPEACHED AND THROWN OUT OF OFFICE!\n",
            starved or 0
        )
    elseif status == "GameOverExtinction" then
        return "EVERYONE HAS DIED. THE CITY OF LAGASH IS EXTINCT!\n"
    elseif M.is_won(status) then
        local rating = M.get_won_rating(status)
        local avg_starved = state.percent_starved_sum // 10
        local acres_per_person = 0
        if state.population > 0 then
            acres_per_person = state.acres // state.population
        end

        local rating_msg
        if rating == "Charlemagne" then
            rating_msg = "A FANTASTIC PERFORMANCE! CHARLEMAGNE, DISRAELI, AND JEFFERSON COMBINED COULD NOT HAVE DONE BETTER!"
        elseif rating == "Acre" then
            rating_msg = "YOUR PERFORMANCE COULD HAVE BEEN WORSE, BUT YOU LEFT THEM REASONABLE ACREAGE."
        elseif rating == "Nero" then
            rating_msg = "YOUR PERFORMANCE LEAVES MUCH TO BE DESIRED. NERO WOULD HAVE BEEN PROUD OF YOU."
        else
            rating_msg = "YOU DID SO POORLY YOU HAVE BEEN DECLARED AN OFFICIAL NATIONAL DISASTER!"
        end

        return string.format(
            "IN YOUR 10-YEAR TERM OF OFFICE, %d PERCENT OF THE\n" ..
            "POPULATION STARVED PER YEAR ON AVERAGE.\n" ..
            "A TOTAL OF %d PEOPLE DIED!!\n" ..
            "YOU STARTED WITH 10 ACRES PER PERSON AND ENDED WITH\n" ..
            "%d ACRES PER PERSON.\n\n" ..
            "%s\n",
            avg_starved, state.starved_total, acres_per_person, rating_msg
        )
    else
        -- Active playing year report
        local parts = {}
        parts[#parts + 1] = string.format(
            "HAMURABI: I BEG TO REPORT TO YOU,\n" ..
            "IN YEAR %d, %d PEOPLE STARVED, %d CAME TO THE CITY.\n",
            state.year, state.starved_last_year, state.immigrants_last_year
        )
        if state.plague_struck then
            parts[#parts + 1] = "A HORRIBLE PLAGUE STRUCK! HALF THE PEOPLE DIED.\n"
        end
        parts[#parts + 1] = string.format(
            "POPULATION IS NOW %d.\n" ..
            "THE CITY NOW OWNS %d ACRES.\n" ..
            "YOU HARVESTED %d BUSHELS PER ACRE.\n" ..
            "RATS ATE %d BUSHELS.\n" ..
            "YOU NOW HAVE %d BUSHELS IN STORE.\n" ..
            "LAND IS TRADING AT %d BUSHELS PER ACRE.\n",
            state.population,
            state.acres,
            state.harvest_per_acre,
            state.rats_eaten,
            state.bushels,
            state.land_price
        )
        return table.concat(parts)
    end
end

-- Aliases for camelCase convenience
M.newGame = M.new_game
M.validateOrder = M.validate_order
M.stepYear = M.step_year
M.formatReport = M.format_report

return M
