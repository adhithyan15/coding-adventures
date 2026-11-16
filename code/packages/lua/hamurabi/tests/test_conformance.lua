-- tests/test_conformance.lua
-- ============================================================================
-- Shared corpus conformance tests against code/specs/fixtures/hamurabi-v1/cases.json.
--
-- Verifies 100% bit-exact parity across:
--   - All 31 conformance cases
--   - 10-year trajectories and intermediate states
--   - All 8 HamurabiError error arms
--   - Authentic teletype report strings
-- ============================================================================

package.path = "../src/?.lua;" .. "../src/?/init.lua;" .. package.path
local hamurabi = require("coding_adventures.hamurabi")
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
    assert = assert or {}
    assert.are = {
        equal = function(expected, actual, msg)
            if expected ~= actual then
                error((msg and (msg .. ": ") or "") .. "expected " .. tostring(expected) .. ", got " .. tostring(actual), 2)
            end
        end,
        same = function(expected, actual, msg)
            if type(expected) ~= type(actual) then
                error((msg and (msg .. ": ") or "") .. "type mismatch", 2)
            end
            if type(expected) == "table" then
                for k, v in pairs(expected) do
                    if actual[k] ~= v then
                        error((msg and (msg .. ": ") or "") .. "key mismatch " .. tostring(k) .. ": expected " .. tostring(v) .. ", got " .. tostring(actual[k]), 2)
                    end
                end
            elseif expected ~= actual then
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

local function find_cases_json()
    local candidates = {
        "../../../../specs/fixtures/hamurabi-v1/cases.json",
        "../../../specs/fixtures/hamurabi-v1/cases.json",
        "../../specs/fixtures/hamurabi-v1/cases.json",
        "specs/fixtures/hamurabi-v1/cases.json",
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

local function clone_state(s)
    local c = {}
    for k, v in pairs(s) do c[k] = v end
    return c
end

describe("hamurabi-v1 shared corpus conformance", function()
    local raw_json, fixture_path = find_cases_json()
    local suite = json_helper.decode(raw_json)

    it("loads suite containing 31 test cases", function()
        assert.is_not_nil(suite.cases)
        assert.equals(31, #suite.cases)
    end)

    for _, case in ipairs(suite.cases) do
        it(case.id .. ": " .. (case.description or ""), function()
            local state
            if case.initial_state then
                state = clone_state(case.initial_state)
            else
                state = hamurabi.new_game(case.seed)
            end

            -- If seed and initial_state are present, verify new_game matches initial_state
            if case.seed and case.initial_state and state.year == 1 and state.population == 100 and state.acres == 1000 then
                local fresh = hamurabi.new_game(case.seed)
                for k, exp_val in pairs(case.initial_state) do
                    assert.are.equal(exp_val, fresh[k], "initial_state mismatch on field " .. k)
                end
            end

            if case.turns then
                local current_state = state
                for _, turn_data in ipairs(case.turns) do
                    local order = turn_data.order
                    local exp_err = turn_data.expected_error

                    if exp_err then
                        local ok, err = hamurabi.validate_order(current_state, order)
                        assert.is_false(ok, "validate_order should have failed on turn " .. turn_data.turn)
                        assert.are.equal(exp_err, err, "validate_order error mismatch on turn " .. turn_data.turn)

                        local next_st, step_err = hamurabi.step_year(current_state, order)
                        assert.are.equal(nil, next_st, "step_year should have returned nil on turn " .. turn_data.turn)
                        assert.are.equal(exp_err, step_err, "step_year error mismatch on turn " .. turn_data.turn)
                    else
                        local val_ok, val_err = hamurabi.validate_order(current_state, order)
                        assert.is_true(val_ok, "validate_order failed on turn " .. turn_data.turn .. ": " .. tostring(val_err))
                        assert.are.equal(nil, val_err)

                        local next_st, step_err = hamurabi.step_year(current_state, order)
                        assert.are.equal(nil, step_err, "step_year error on turn " .. turn_data.turn .. ": " .. tostring(step_err))
                        assert.is_not_nil(next_st, "step_year returned nil on turn " .. turn_data.turn)

                        -- Invariance: returned state must be a fresh table (VER04m)
                        assert.is_true(next_st ~= current_state, "step_year must return fresh state table")

                        -- Verify all expected fields
                        local exp_st = turn_data.expected_state
                        for k, exp_val in pairs(exp_st) do
                            assert.are.equal(exp_val, next_st[k], "Turn " .. turn_data.turn .. " mismatch on field '" .. k .. "'")
                        end

                        -- Verify expected teletype report if present
                        if turn_data.expected_report then
                            local report = hamurabi.format_report(next_st)
                            assert.are.equal(turn_data.expected_report, report, "Turn " .. turn_data.turn .. " report mismatch")
                        end

                        current_state = next_st
                    end
                end
            elseif case.order and case.expected_error then
                local order = case.order
                local exp_err = case.expected_error

                local val_ok, val_err = hamurabi.validate_order(state, order)
                assert.is_false(val_ok, "validate_order should have failed for case " .. case.id)
                assert.are.equal(exp_err, val_err, "validate_order error mismatch for case " .. case.id)

                local next_st, step_err = hamurabi.step_year(state, order)
                assert.are.equal(nil, next_st, "step_year should have returned nil for case " .. case.id)
                assert.are.equal(exp_err, step_err, "step_year error mismatch for case " .. case.id)
            end
        end)
    end
end)
