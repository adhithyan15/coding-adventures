-- tests/test_lunar_lander.lua
-- ============================================================================
-- Unit tests, CWE weakness closure, and ASCII telemetry verification
-- for coding_adventures.lunar_lander.
-- ============================================================================

local info = debug.getinfo(1, "S").source
local script_path = info:match("^@(.+)$")
local script_dir = script_path and script_path:match("^(.*)[/\\]") or "."
package.path = script_dir .. "/../src/?.lua;" .. script_dir .. "/../src/?/init.lua;" .. script_dir .. "/?.lua;" .. package.path

local lander = require("coding_adventures.lunar_lander")

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

describe("lunar_lander core simulation", function()
    it("initial flight invariants", function()
        local flight = lander.new_flight()
        assert.equals(0, flight.time_seconds)
        assert.equals(lander.INITIAL_ALTITUDE_MFT, flight.altitude_mft)
        assert.equals(lander.INITIAL_VELOCITY_MFT_S, flight.velocity_mft_s)
        assert.equals(lander.INITIAL_FUEL_LBS, flight.fuel_lbs)
        assert.equals(lander.DRY_MASS_LBS, flight.dry_mass_lbs)
        assert.equals("InFlight", flight.status)
        assert.is_nil(flight.impact_velocity_mft_s)
        assert.is_true(lander.state_inv(flight))
    end)

    it("isqrt exactness", function()
        assert.equals(0, lander.isqrt(0))
        assert.equals(1, lander.isqrt(1))
        assert.equals(1, lander.isqrt(2))
        assert.equals(1, lander.isqrt(3))
        assert.equals(2, lander.isqrt(4))
        assert.equals(3, lander.isqrt(15))
        assert.equals(4, lander.isqrt(16))
        assert.equals(3205370, lander.isqrt(10274400000000))
        assert.equals(3205370, lander.isqrt_u64(10274400000000))
        assert.equals(3205370, lander.isqrt_u128(10274400000000))
    end)

    it("trunc_div toward zero exactness", function()
        assert.equals(-1, lander.trunc_div(-1500, 1000))
        assert.equals(1, lander.trunc_div(1500, 1000))
        assert.equals(-1, lander.trunc_div(1500, -1000))
        assert.equals(1, lander.trunc_div(-1500, -1000))
        assert.equals(0, lander.trunc_div(500, 1000))
        assert.equals(0, lander.trunc_div(-500, 1000))
    end)

    it("div_euclid exactness", function()
        assert.equals(9, lander.div_euclid(-46070, -5310))
        assert.equals(-9, lander.div_euclid(-46070, 5310))
        assert.equals(8, lander.div_euclid(46070, 5310))
        assert.equals(-8, lander.div_euclid(46070, -5310))

        -- Remainder non-negativity test
        local a = -46070
        local b = -5310
        local q = lander.div_euclid(a, b)
        local r = a - q * b
        assert.is_true(r >= 0 and r < math.abs(b))
    end)

    it("CWE-843 / CWE-681 type confusion prevention: rejects floats, strings, booleans", function()
        local state = lander.new_flight()
        local valid_cmd = { burn_rate_lbs_s = 100, duration_seconds = 5 }

        -- Float altitude
        local float_alt_state = lander.new_flight()
        float_alt_state.altitude_mft = 120000000.0
        local ok, err = lander.validate_burn(float_alt_state, valid_cmd)
        assert.is_false(ok)
        assert.equals(lander.LanderError.ArithmeticOverflow, err)

        -- Float burn rate
        local float_burn_cmd = { burn_rate_lbs_s = 100.0, duration_seconds = 5 }
        local ok2, err2 = lander.validate_burn(state, float_burn_cmd)
        assert.is_false(ok2)
        assert.equals(lander.LanderError.ArithmeticOverflow, err2)

        -- Float duration
        local float_dur_cmd = { burn_rate_lbs_s = 100, duration_seconds = 5.0 }
        local ok3, err3 = lander.validate_burn(state, float_dur_cmd)
        assert.is_false(ok3)
        assert.equals(lander.LanderError.ArithmeticOverflow, err3)

        -- String burn rate
        local str_cmd = { burn_rate_lbs_s = "100", duration_seconds = 5 }
        local ok4, err4 = lander.validate_burn(state, str_cmd)
        assert.is_false(ok4)
        assert.equals(lander.LanderError.ArithmeticOverflow, err4)

        -- Boolean duration
        local bool_cmd = { burn_rate_lbs_s = 100, duration_seconds = true }
        local ok5, err5 = lander.validate_burn(state, bool_cmd)
        assert.is_false(ok5)
        assert.equals(lander.LanderError.ArithmeticOverflow, err5)
    end)

    it("CWE-369 division by zero prevention", function()
        local hostile_state = {
            time_seconds = 0,
            altitude_mft = 100000000,
            velocity_mft_s = -2000000,
            fuel_lbs = 0,
            dry_mass_lbs = 0,
            status = "InFlight",
            impact_velocity_mft_s = nil,
        }
        local cmd = { burn_rate_lbs_s = 100, duration_seconds = 5 }
        local ok, err = lander.validate_burn(hostile_state, cmd)
        assert.is_false(ok)
        assert.equals(lander.LanderError.ArithmeticOverflow, err)

        local next_state, step_err = lander.step_burn(hostile_state, cmd)
        assert.is_nil(next_state)
        assert.equals(lander.LanderError.ArithmeticOverflow, step_err)
    end)

    it("CWE-190 integer overflow & boundary bounds checking", function()
        local state = lander.new_flight()
        local valid_cmd = { burn_rate_lbs_s = 100, duration_seconds = 5 }

        -- Altitude boundaries
        state.altitude_mft = lander.MAX_ALTITUDE_MFT + 1
        local ok, err = lander.validate_burn(state, valid_cmd)
        assert.is_false(ok)
        assert.equals(lander.LanderError.ArithmeticOverflow, err)

        -- Velocity boundaries
        state = lander.new_flight()
        state.velocity_mft_s = lander.MIN_VELOCITY_MFT_S - 1
        local ok2, err2 = lander.validate_burn(state, valid_cmd)
        assert.is_false(ok2)
        assert.equals(lander.LanderError.ArithmeticOverflow, err2)

        state = lander.new_flight()
        state.velocity_mft_s = lander.MAX_VELOCITY_MFT_S + 1
        local ok3, err3 = lander.validate_burn(state, valid_cmd)
        assert.is_false(ok3)
        assert.equals(lander.LanderError.ArithmeticOverflow, err3)

        -- Fuel boundaries
        state = lander.new_flight()
        state.fuel_lbs = lander.INITIAL_FUEL_LBS + 1
        local ok4, err4 = lander.validate_burn(state, valid_cmd)
        assert.is_false(ok4)
        assert.equals(lander.LanderError.ArithmeticOverflow, err4)

        -- Time boundaries
        state = lander.new_flight()
        state.time_seconds = lander.MAX_TIME_SECONDS
        local add_one_s = { burn_rate_lbs_s = 0, duration_seconds = 1 }
        local ok5, err5 = lander.validate_burn(state, add_one_s)
        assert.is_false(ok5)
        assert.equals(lander.LanderError.ArithmeticOverflow, err5)

        -- Command bounds
        state = lander.new_flight()
        local excessive_burn = { burn_rate_lbs_s = lander.MAX_BURN_RATE_LBS_S + 1, duration_seconds = 5 }
        local ok6, err6 = lander.validate_burn(state, excessive_burn)
        assert.is_false(ok6)
        assert.equals(lander.LanderError.ExcessiveBurnRate, err6)

        local zero_duration = { burn_rate_lbs_s = 50, duration_seconds = 0 }
        local ok7, err7 = lander.validate_burn(state, zero_duration)
        assert.is_false(ok7)
        assert.equals(lander.LanderError.InvalidDuration, err7)

        local excessive_dur = { burn_rate_lbs_s = 50, duration_seconds = lander.MAX_DURATION_S + 1 }
        local ok8, err8 = lander.validate_burn(state, excessive_dur)
        assert.is_false(ok8)
        assert.equals(lander.LanderError.InvalidDuration, err8)
    end)

    it("PaintScene & ASCII telemetry HUD rendering", function()
        local state = lander.new_flight()
        local scene = lander.to_paint_scene(state, 0)
        assert.equals(76.0, scene.width)
        assert.equals(5.0, scene.height)

        local hud = lander.render_telemetry_ascii(state, 0)
        local lines = {}
        for line in (hud .. "\n"):gmatch("(.-)\r?\n") do
            if #line > 0 then
                lines[#lines + 1] = line
            end
        end
        assert.equals(5, #lines)

        for idx, line in ipairs(lines) do
            local char_count = utf8.len(line)
            assert.equals(76, char_count, "Line " .. idx .. " length mismatch: " .. line)
        end

        assert.is_true(hud:find("LUNAR LANDER TELEMETRY", 1, true) ~= nil)
        assert.is_true(hud:find("TIME:", 1, true) ~= nil)
        assert.is_true(hud:find("ALTITUDE:", 1, true) ~= nil)
        assert.is_true(hud:find("VELOCITY:", 1, true) ~= nil)
        assert.is_true(hud:find("FUEL:", 1, true) ~= nil)
        assert.is_true(hud:find("BURN:", 1, true) ~= nil)
        assert.is_true(hud:find("RATE:", 1, true) ~= nil)
        assert.is_true(hud:find("DESCENT", 1, true) ~= nil)
        assert.is_true(hud:find("█", 1, true) ~= nil)

        -- Partial fill gauge check
        local mid_state = lander.new_flight()
        mid_state.fuel_lbs = 8250
        mid_state.altitude_mft = 60000000
        local hud_mid = lander.render_telemetry_ascii(mid_state, 100)
        assert.is_true(hud_mid:find("█", 1, true) ~= nil)
        assert.is_true(hud_mid:find("░", 1, true) ~= nil)

        -- Convenience helpers
        local scene2 = lander.telemetry_scene(state)
        assert.equals(76.0, scene2.width)
        local hud2 = lander.render_telemetry(state)
        assert.equals(hud, hud2)
    end)

    it("Telemetry states and landing outcomes", function()
        local state = lander.new_flight()

        -- Ascent
        state.velocity_mft_s = 50000
        local hud_ascent = lander.render_telemetry_ascii(state, 100)
        assert.is_true(hud_ascent:find("ASCENT", 1, true) ~= nil)
        assert.is_true(hud_ascent:find("▲", 1, true) ~= nil)

        -- Hover
        state.velocity_mft_s = 0
        local hud_hover = lander.render_telemetry_ascii(state, 50)
        assert.is_true(hud_hover:find("HOVER", 1, true) ~= nil)
        assert.is_true(hud_hover:find("◆", 1, true) ~= nil)

        -- Landed outcomes
        local outcomes = {
            { outcome = "Perfect", text = "PERFECT" },
            { outcome = "Hard", text = "HARD" },
            { outcome = "Crash", text = "CRASH" },
            { outcome = "Fatal", text = "FATAL" },
            { outcome = "Crater", text = "CRATER" },
        }

        for _, item in ipairs(outcomes) do
            state.status = "Landed(" .. item.outcome .. ")"
            state.impact_velocity_mft_s = -1000
            local hud_landed = lander.render_telemetry_ascii(state, 0)
            assert.is_true(hud_landed:find("STATUS:", 1, true) ~= nil, "missing STATUS:")
            assert.is_true(hud_landed:find(item.text, 1, true) ~= nil, "missing outcome " .. item.text)
            assert.is_true(hud_landed:find("★", 1, true) ~= nil, "missing ★ symbol")
        end
    end)
end)
