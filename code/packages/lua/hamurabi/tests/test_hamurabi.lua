-- tests/test_hamurabi.lua
-- ============================================================================
-- Unit and Adversarial Security Tests for Hamurabi Simulation Core (Lua 5.4)
--
-- Covers:
--   - Package metadata and exports
--   - Initial state construction and deterministic seeding
--   - All 8 HamurabiError error arms with exact precedence
--   - Integer overflow & saturating addition guards (CWE-190)
--   - Type confusion and float rejection (CWE-843)
--   - Infinite loop / LCG collapse guard for even rng_inc (CWE-835 / CWE-400)
--   - State invariant boundaries (VER04m state_inv)
--   - Immutability of state records across transitions (VER04m)
--   - Teletype report formatting and ratings (Charlemagne, Acre, Nero, Disaster)
-- ============================================================================

package.path = "../src/?.lua;" .. "../src/?/init.lua;" .. package.path
local hamurabi = require("coding_adventures.hamurabi")

-- Busted fallback shim when run directly via standalone lua interpreter
if not describe then
    local current_suite = ""
    describe = function(name, fn)
        current_suite = name
        fn()
    end
    it = function(name, fn)
        local ok, err = pcall(fn)
        if ok then
            print("  [PASS] " .. current_suite .. ": " .. name)
        else
            print("  [FAIL] " .. current_suite .. ": " .. name .. " -> " .. tostring(err))
            error(err)
        end
    end
    assert = assert or {}
    assert.are = {
        equal = function(expected, actual, msg)
            if expected ~= actual then
                error((msg and (msg .. ": ") or "") .. "expected " .. tostring(expected) .. ", got " .. tostring(actual), 2)
            end
        end,
    }
    assert.equals = assert.are.equal
    assert.is_true = function(val, msg) if not val then error(msg or "expected true, got " .. tostring(val), 2) end end
    assert.is_false = function(val, msg) if val then error(msg or "expected false, got " .. tostring(val), 2) end end
    assert.is_not_nil = function(val, msg) if val == nil then error(msg or "expected not nil", 2) end end
    assert.has_error = function(fn, expected_msg)
        local ok, err = pcall(fn)
        if ok then error("expected error but call succeeded", 2) end
        if expected_msg and not tostring(err):find(expected_msg, 1, true) then
            error("expected error containing '" .. expected_msg .. "', got: " .. tostring(err), 2)
        end
    end
end

describe("hamurabi module metadata", function()
    it("exports VERSION 0.1.0", function()
        assert.is_not_nil(hamurabi.VERSION)
        assert.are.equal("0.1.0", hamurabi.VERSION)
    end)

    it("exports canonical constants", function()
        assert.are.equal(6364136223846793005, hamurabi.PCG_MULTIPLIER)
        assert.are.equal(54, hamurabi.HAMURABI_STREAM_ID)
        assert.are.equal(109, hamurabi.HAMURABI_INC)
    end)

    it("exports Ratings table", function()
        assert.are.equal("Charlemagne", hamurabi.Rating.Charlemagne)
        assert.are.equal("Acre", hamurabi.Rating.Acre)
        assert.are.equal("Nero", hamurabi.Rating.Nero)
        assert.are.equal("Disaster", hamurabi.Rating.Disaster)
    end)

    it("exports HamurabiError enumeration", function()
        assert.are.equal("GameAlreadyEnded", hamurabi.HamurabiError.GameAlreadyEnded)
        assert.are.equal("InvalidOrderBounds", hamurabi.HamurabiError.InvalidOrderBounds)
        assert.are.equal("InsufficientBushelsForLand", hamurabi.HamurabiError.InsufficientBushelsForLand)
        assert.are.equal("InsufficientAcresToSell", hamurabi.HamurabiError.InsufficientAcresToSell)
        assert.are.equal("InsufficientBushelsToFeed", hamurabi.HamurabiError.InsufficientBushelsToFeed)
        assert.are.equal("InsufficientAcresToPlant", hamurabi.HamurabiError.InsufficientAcresToPlant)
        assert.are.equal("InsufficientPopulationToPlant", hamurabi.HamurabiError.InsufficientPopulationToPlant)
        assert.are.equal("InsufficientGrainForSeed", hamurabi.HamurabiError.InsufficientGrainForSeed)
    end)
end)

describe("initial game state (new_game)", function()
    it("initializes year 1 with standard Sumerian balances", function()
        local state = hamurabi.new_game(1337)
        assert.are.equal(1, state.year)
        assert.are.equal(100, state.population)
        assert.are.equal(2800, state.bushels)
        assert.are.equal(1000, state.acres)
        assert.is_true(state.land_price >= 17 and state.land_price <= 26)
        assert.are.equal(3, state.harvest_per_acre)
        assert.are.equal(200, state.rats_eaten)
        assert.are.equal(0, state.starved_last_year)
        assert.are.equal(0, state.starved_total)
        assert.are.equal(0, state.percent_starved_sum)
        assert.are.equal(5, state.immigrants_last_year)
        assert.is_false(state.plague_struck)
        assert.are.equal("Playing", state.status)
        assert.are.equal(1, state.rng_inc & 1)
        assert.are.equal(109, state.rng_inc)
    end)

    it("rejects non-integer seed types strictly (CWE-843)", function()
        assert.has_error(function() hamurabi.new_game("1337") end)
        assert.has_error(function() hamurabi.new_game(1337.5) end)
        assert.has_error(function() hamurabi.new_game(true) end)
        assert.has_error(function() hamurabi.new_game(nil) end)
        assert.has_error(function() hamurabi.new_game({}) end)
    end)
end)

describe("all eight error arms (order refusal)", function()
    local state = hamurabi.new_game(1337)
    local normal_order = { acres_delta = 0, bushels_to_feed = 2000, acres_to_plant = 100 }

    it("Arm 1: GameAlreadyEnded when status is not Playing", function()
        local won_state = {}
        for k, v in pairs(state) do won_state[k] = v end
        won_state.status = "Won(Charlemagne)"

        local ok, err = hamurabi.validate_order(won_state, normal_order)
        assert.is_false(ok)
        assert.are.equal(hamurabi.HamurabiError.GameAlreadyEnded, err)

        local next_st, step_err = hamurabi.step_year(won_state, normal_order)
        assert.are.equal(nil, next_st)
        assert.are.equal(hamurabi.HamurabiError.GameAlreadyEnded, step_err)
    end)

    it("Arm 2: InvalidOrderBounds on invalid parameters or corrupted state", function()
        local bad_state = {}
        for k, v in pairs(state) do bad_state[k] = v end
        bad_state.land_price = 10 -- valid is 17..=26

        local ok, err = hamurabi.validate_order(bad_state, normal_order)
        assert.is_false(ok)
        assert.are.equal(hamurabi.HamurabiError.InvalidOrderBounds, err)

        -- Non-integer in order
        local float_order = { acres_delta = 0.5, bushels_to_feed = 2000, acres_to_plant = 100 }
        local ok2, err2 = hamurabi.validate_order(state, float_order)
        assert.is_false(ok2)
        assert.are.equal(hamurabi.HamurabiError.InvalidOrderBounds, err2)
    end)

    it("Arm 3: InsufficientBushelsForLand when buying more than bushels afford", function()
        local buy_too_much = { acres_delta = 200, bushels_to_feed = 0, acres_to_plant = 0 }
        local ok, err = hamurabi.validate_order(state, buy_too_much)
        assert.is_false(ok)
        assert.are.equal(hamurabi.HamurabiError.InsufficientBushelsForLand, err)
    end)

    it("Arm 4: InsufficientAcresToSell when selling more than owned", function()
        local sell_too_much = { acres_delta = -1001, bushels_to_feed = 0, acres_to_plant = 0 }
        local ok, err = hamurabi.validate_order(state, sell_too_much)
        assert.is_false(ok)
        assert.are.equal(hamurabi.HamurabiError.InsufficientAcresToSell, err)
    end)

    it("Arm 5: InsufficientBushelsToFeed when feeding exceeds store", function()
        local feed_too_much = { acres_delta = 0, bushels_to_feed = 2801, acres_to_plant = 0 }
        local ok, err = hamurabi.validate_order(state, feed_too_much)
        assert.is_false(ok)
        assert.are.equal(hamurabi.HamurabiError.InsufficientBushelsToFeed, err)
    end)

    it("Arm 6: InsufficientAcresToPlant when planting exceeds acres after trade", function()
        local plant_too_much = { acres_delta = 0, bushels_to_feed = 1000, acres_to_plant = 1001 }
        local ok, err = hamurabi.validate_order(state, plant_too_much)
        assert.is_false(ok)
        assert.are.equal(hamurabi.HamurabiError.InsufficientAcresToPlant, err)
    end)

    it("Arm 7: InsufficientPopulationToPlant when labor capacity exceeded", function()
        local small_pop = {}
        for k, v in pairs(state) do small_pop[k] = v end
        small_pop.population = 50 -- can tend at most 500 acres

        local labor_exceeded = { acres_delta = 0, bushels_to_feed = 1000, acres_to_plant = 501 }
        local ok, err = hamurabi.validate_order(small_pop, labor_exceeded)
        assert.is_false(ok)
        assert.are.equal(hamurabi.HamurabiError.InsufficientPopulationToPlant, err)
    end)

    it("Arm 8: InsufficientGrainForSeed when seed grain is insufficient", function()
        local seed_starved = { acres_delta = 0, bushels_to_feed = 2500, acres_to_plant = 800 }
        -- Left after feed: 300 bushels. 800 acres needs 400 bushels seed.
        local ok, err = hamurabi.validate_order(state, seed_starved)
        assert.is_false(ok)
        assert.are.equal(hamurabi.HamurabiError.InsufficientGrainForSeed, err)
    end)
end)

describe("adversarial CWE security tests", function()
    local valid_order = { acres_delta = 0, bushels_to_feed = 2000, acres_to_plant = 1000 }

    it("rejects even rng_inc to prevent LCG period collapse / infinite loop (CWE-835 / CWE-400)", function()
        local state = hamurabi.new_game(1)
        state.rng_inc = 0
        local ok, err = hamurabi.validate_order(state, valid_order)
        assert.is_false(ok)
        assert.are.equal(hamurabi.HamurabiError.InvalidOrderBounds, err)

        state.rng_inc = 2
        local ok2, err2 = hamurabi.validate_order(state, valid_order)
        assert.is_false(ok2)
        assert.are.equal(hamurabi.HamurabiError.InvalidOrderBounds, err2)
    end)

    it("rejects hostile state bounds (VER04m state_inv)", function()
        local state = hamurabi.new_game(1)

        -- Hostile year bounds
        state.year = 0
        assert.are.equal(hamurabi.HamurabiError.InvalidOrderBounds, select(2, hamurabi.validate_order(state, valid_order)))
        state.year = 11
        assert.are.equal(hamurabi.HamurabiError.InvalidOrderBounds, select(2, hamurabi.validate_order(state, valid_order)))
        state.year = 255
        assert.are.equal(hamurabi.HamurabiError.InvalidOrderBounds, select(2, hamurabi.validate_order(state, valid_order)))
        state.year = 1

        -- Hostile land price bounds [17, 26]
        state.land_price = 0
        assert.are.equal(hamurabi.HamurabiError.InvalidOrderBounds, select(2, hamurabi.validate_order(state, valid_order)))
        state.land_price = 100
        assert.are.equal(hamurabi.HamurabiError.InvalidOrderBounds, select(2, hamurabi.validate_order(state, valid_order)))
        state.land_price = 20

        -- Hostile harvest per acre bounds [1, 5]
        state.harvest_per_acre = 0
        assert.are.equal(hamurabi.HamurabiError.InvalidOrderBounds, select(2, hamurabi.validate_order(state, valid_order)))
        state.harvest_per_acre = 99
        assert.are.equal(hamurabi.HamurabiError.InvalidOrderBounds, select(2, hamurabi.validate_order(state, valid_order)))
    end)

    it("rejects revenue overflow on huge land sale (CWE-190)", function()
        local state = hamurabi.new_game(1)
        state.acres = 200000000
        state.bushels = 2800
        state.land_price = 25
        local order = { acres_delta = -180000000, bushels_to_feed = 0, acres_to_plant = 0 }
        local ok, err = hamurabi.validate_order(state, order)
        assert.is_false(ok)
        assert.are.equal(hamurabi.HamurabiError.InvalidOrderBounds, err)
    end)

    it("rejects harvest overflow when acres_to_plant * 5 overflows u32 (CWE-190)", function()
        local state = hamurabi.new_game(1)
        state.population = 500000000
        state.acres = 1000000000
        state.bushels = 2000000000
        local order = { acres_delta = 0, bushels_to_feed = 0, acres_to_plant = 900000000 }
        local ok, err = hamurabi.validate_order(state, order)
        assert.is_false(ok)
        assert.are.equal(hamurabi.HamurabiError.InvalidOrderBounds, err)
    end)

    it("saturates starved_total and percent_starved_sum at MAX_U32 without overflow", function()
        local state = hamurabi.new_game(1)
        state.starved_total = hamurabi.MAX_U32
        state.percent_starved_sum = hamurabi.MAX_U32 - 10
        local order = { acres_delta = 0, bushels_to_feed = 1000, acres_to_plant = 0 }
        local next_st, err = hamurabi.step_year(state, order)
        assert.are.equal(nil, err)
        assert.is_not_nil(next_st)
        assert.are.equal(hamurabi.MAX_U32, next_st.starved_total)
        assert.are.equal(hamurabi.MAX_U32, next_st.percent_starved_sum)
    end)

    it("preserves immutability of state tables (VER04m)", function()
        local state = hamurabi.new_game(1)
        local orig_pop = state.population
        local orig_bushels = state.bushels
        local order = { acres_delta = 0, bushels_to_feed = 2000, acres_to_plant = 500 }

        local next_st = hamurabi.step_year(state, order)
        assert.is_true(next_st ~= state)
        assert.are.equal(orig_pop, state.population)
        assert.are.equal(orig_bushels, state.bushels)
    end)
end)

describe("format_report and status helpers", function()
    it("formats playing report with teletype uppercase styling", function()
        local state = hamurabi.new_game(1337)
        local report = hamurabi.format_report(state)
        assert.is_true(report:find("HAMURABI: I BEG TO REPORT TO YOU") ~= nil)
        assert.is_true(report:find("POPULATION IS NOW 100.") ~= nil)
    end)

    it("formats plague notice when plague strikes", function()
        local state = hamurabi.new_game(1337)
        state.plague_struck = true
        local report = hamurabi.format_report(state)
        assert.is_true(report:find("A HORRIBLE PLAGUE STRUCK! HALF THE PEOPLE DIED.") ~= nil)
    end)

    it("formats impeachment report", function()
        local state = hamurabi.new_game(1337)
        state.status = "Impeached(starved=50, percent=50)"
        local report = hamurabi.format_report(state)
        assert.is_true(report:find("YOU STARVED 50 PEOPLE IN ONE YEAR!!!") ~= nil)
        assert.is_true(report:find("IMPEACHED AND THROWN OUT OF OFFICE") ~= nil)
    end)

    it("formats extinction report", function()
        local state = hamurabi.new_game(1337)
        state.status = "GameOverExtinction"
        local report = hamurabi.format_report(state)
        assert.are.equal("EVERYONE HAS DIED. THE CITY OF LAGASH IS EXTINCT!\n", report)
    end)

    it("formats winning reports for all 4 ratings", function()
        local state = hamurabi.new_game(1337)
        state.year = 11

        state.status = "Won(Charlemagne)"
        assert.is_true(hamurabi.format_report(state):find("CHARLEMAGNE, DISRAELI, AND JEFFERSON") ~= nil)

        state.status = "Won(Acre)"
        assert.is_true(hamurabi.format_report(state):find("REASONABLE ACREAGE") ~= nil)

        state.status = "Won(Nero)"
        assert.is_true(hamurabi.format_report(state):find("NERO WOULD HAVE BEEN PROUD") ~= nil)

        state.status = "Won(Disaster)"
        assert.is_true(hamurabi.format_report(state):find("OFFICIAL NATIONAL DISASTER") ~= nil)
    end)

    it("tests status helper predicates", function()
        assert.is_true(hamurabi.is_playing("Playing"))
        assert.is_false(hamurabi.is_playing("Won(Charlemagne)"))

        assert.is_true(hamurabi.is_won("Won(Charlemagne)"))
        assert.are.equal("Charlemagne", hamurabi.get_won_rating("Won(Charlemagne)"))

        assert.is_true(hamurabi.is_impeached("Impeached(starved=46, percent=46)"))
        local starved, percent = hamurabi.parse_impeached("Impeached(starved=46, percent=46)")
        assert.are.equal(46, starved)
        assert.are.equal(46, percent)

        assert.is_true(hamurabi.is_game_over_extinction("GameOverExtinction"))
    end)
end)
