-- tests/test_conformance.lua
-- ============================================================================
-- Shared corpus conformance tests against
-- code/specs/fixtures/lunar-lander-v1/cases.json.
--
-- Verifies 100% bit-exact parity across:
--   - All 39 conformance cases
--   - Multi-turn trajectories and intermediate states
--   - Touchdown solver, ground penetration interpolation, fuel refund
--   - All LanderError error arms and canonical precedence
-- ============================================================================

local info = debug.getinfo(1, "S").source
local script_path = info:match("^@(.+)$")
local script_dir = script_path and script_path:match("^(.*)[/\\]") or "."
package.path = script_dir .. "/../src/?.lua;" .. script_dir .. "/../src/?/init.lua;" .. script_dir .. "/?.lua;" .. package.path
local lander = require("coding_adventures.lunar_lander")
local json_helper = require("json_helper")

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
    local custom_assert = {}
    custom_assert.are = {
        equal = function(expected, actual, msg)
            if expected ~= actual then
                error((msg and (msg .. ": ") or "") .. "expected " .. tostring(expected) .. ", got " .. tostring(actual), 2)
            end
        end,
    }
    custom_assert.equals = custom_assert.are.equal
    custom_assert.is_true = function(val, msg) if not val then error(msg or "expected true, got " .. tostring(val), 2) end end
    custom_assert.is_false = function(val, msg) if val then error(msg or "expected false, got " .. tostring(val), 2) end end
    custom_assert.is_not_nil = function(val, msg) if val == nil then error(msg or "expected not nil", 2) end end
    custom_assert.is_nil = function(val, msg) if val ~= nil then error(msg or "expected nil, got " .. tostring(val), 2) end end
    assert = custom_assert
end

local function find_cases_json()
    local candidates = {
        script_dir .. "/../../../../specs/fixtures/lunar-lander-v1/cases.json",
        script_dir .. "/../../../../../specs/fixtures/lunar-lander-v1/cases.json",
        "../../../../specs/fixtures/lunar-lander-v1/cases.json",
        "../../../../../specs/fixtures/lunar-lander-v1/cases.json",
        "../../../specs/fixtures/lunar-lander-v1/cases.json",
        "code/specs/fixtures/lunar-lander-v1/cases.json",
    }
    for _, path in ipairs(candidates) do
        local f = io.open(path, "rb")
        if f then
            local content = f:read("*a")
            f:close()
            return content, path
        end
    end
    error("could not locate cases.json fixture")
end

local function to_lua_state(s)
    return {
        time_seconds = s.time_seconds,
        altitude_mft = s.altitude_mft,
        velocity_mft_s = s.velocity_mft_s,
        fuel_lbs = s.fuel_lbs,
        dry_mass_lbs = s.dry_mass_lbs,
        status = s.status,
        impact_velocity_mft_s = s.impact_velocity_mft_s,
    }
end

local function to_lua_cmd(c)
    return {
        burn_rate_lbs_s = c.burn_rate_lbs_s,
        duration_seconds = c.duration_seconds,
    }
end

describe("lunar-lander-v1 shared corpus conformance", function()
    local raw_json, _ = find_cases_json()
    local suite = json_helper.decode(raw_json)

    it("loads suite containing 39 test cases", function()
        assert.is_not_nil(suite.cases)
        assert.equals(39, #suite.cases)
    end)

    for _, case in ipairs(suite.cases) do
        it("executes case " .. case.id .. " (" .. case.description .. ")", function()
            local initial_state = to_lua_state(case.initial_state)

            if case.turns and #case.turns > 0 then
                local current_state = initial_state
                for _, turn in ipairs(case.turns) do
                    local cmd = to_lua_cmd(turn.command)
                    local expected = to_lua_state(turn.expected_state)

                    local ok, err = lander.validate_burn(current_state, cmd)
                    assert.is_true(ok, "Case " .. case.id .. " turn " .. turn.turn .. " validate_burn failed: " .. tostring(err))

                    local next_state, step_err = lander.step_burn(current_state, cmd)
                    assert.is_nil(step_err, "Case " .. case.id .. " turn " .. turn.turn .. " step_burn returned error: " .. tostring(step_err))
                    assert.is_not_nil(next_state, "Case " .. case.id .. " turn " .. turn.turn .. " step_burn returned nil state")

                    assert.equals(expected.time_seconds, next_state.time_seconds, "Case " .. case.id .. " turn " .. turn.turn .. " time mismatch")
                    assert.equals(expected.altitude_mft, next_state.altitude_mft, "Case " .. case.id .. " turn " .. turn.turn .. " altitude mismatch")
                    assert.equals(expected.velocity_mft_s, next_state.velocity_mft_s, "Case " .. case.id .. " turn " .. turn.turn .. " velocity mismatch")
                    assert.equals(expected.fuel_lbs, next_state.fuel_lbs, "Case " .. case.id .. " turn " .. turn.turn .. " fuel mismatch")
                    assert.equals(expected.dry_mass_lbs, next_state.dry_mass_lbs, "Case " .. case.id .. " turn " .. turn.turn .. " dry mass mismatch")
                    assert.equals(expected.status, next_state.status, "Case " .. case.id .. " turn " .. turn.turn .. " status mismatch")
                    assert.equals(expected.impact_velocity_mft_s, next_state.impact_velocity_mft_s, "Case " .. case.id .. " turn " .. turn.turn .. " impact velocity mismatch")

                    current_state = next_state
                end
            elseif case.command then
                local cmd = to_lua_cmd(case.command)
                local expected_err = case.expected_error

                local ok, val_err = lander.validate_burn(initial_state, cmd)
                assert.is_false(ok, "Case " .. case.id .. " validate_burn unexpectedly succeeded")
                assert.equals(expected_err, val_err, "Case " .. case.id .. " validate_burn error mismatch")

                local next_state, step_err = lander.step_burn(initial_state, cmd)
                assert.is_nil(next_state, "Case " .. case.id .. " step_burn unexpectedly succeeded")
                assert.equals(expected_err, step_err, "Case " .. case.id .. " step_burn error mismatch")
            else
                error("Case " .. case.id .. " has neither turns nor command")
            end
        end)
    end
end)
