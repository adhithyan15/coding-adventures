-- coding_adventures.lunar_lander
-- ============================================================================
--
-- Pure, deterministic simulation core and ASCII telemetry for the classic
-- discrete lunar landing flight simulation (GAME02).
--
-- Architectural Guarantees:
--   - Zero ambient system authority: declared capabilities: [] (CAP001 / VER08a).
--   - Pure state transition: step_burn(state, cmd) -> (LanderState, nil)
--     or (nil, LanderError).
--   - Bit-exact integer arithmetic using 64-bit integers with zero floating-point
--     operations in the simulation core.
--   - Exact Euclidean division (div_euclid) and bounded integer square root (isqrt).
--   - Declarative 76x5 PaintScene and ASCII character-grid telemetry HUD (P2D02).
--   - Complete 26-CWE weakness closure (CWE-190, CWE-369, CWE-843, CWE-681).
--   - Immutable fresh records (VER04m): original state is never modified.
--
-- ============================================================================

local M = {}

M.VERSION = "0.1.0"

-- ============================================================================
-- Physical Constants & Apollo LM Parameters
-- ============================================================================

local G_LUNAR_MFT_S2 = 5310
local DRY_MASS_LBS = 16500
local INITIAL_FUEL_LBS = 16500
local TOTAL_INITIAL_MASS_LBS = 33000
local INITIAL_ALTITUDE_MFT = 120000000
local INITIAL_VELOCITY_MFT_S = -3000000
local EXHAUST_VELOCITY_MFT_S = 8000000
local MAX_BURN_RATE_LBS_S = 200
local MIN_DURATION_S = 1
local MAX_DURATION_S = 10
local MAX_TIME_SECONDS = 86400
local MAX_ALTITUDE_MFT = 200000000
local MIN_VELOCITY_MFT_S = -10000000
local MAX_VELOCITY_MFT_S = 10000000

M.G_LUNAR_MFT_S2 = G_LUNAR_MFT_S2
M.DRY_MASS_LBS = DRY_MASS_LBS
M.INITIAL_FUEL_LBS = INITIAL_FUEL_LBS
M.TOTAL_INITIAL_MASS_LBS = TOTAL_INITIAL_MASS_LBS
M.INITIAL_ALTITUDE_MFT = INITIAL_ALTITUDE_MFT
M.INITIAL_VELOCITY_MFT_S = INITIAL_VELOCITY_MFT_S
M.EXHAUST_VELOCITY_MFT_S = EXHAUST_VELOCITY_MFT_S
M.MAX_BURN_RATE_LBS_S = MAX_BURN_RATE_LBS_S
M.MIN_DURATION_S = MIN_DURATION_S
M.MAX_DURATION_S = MAX_DURATION_S
M.MAX_TIME_SECONDS = MAX_TIME_SECONDS
M.MAX_ALTITUDE_MFT = MAX_ALTITUDE_MFT
M.MIN_VELOCITY_MFT_S = MIN_VELOCITY_MFT_S
M.MAX_VELOCITY_MFT_S = MAX_VELOCITY_MFT_S

-- ============================================================================
-- Domain Enums & Error Constants
-- ============================================================================

M.LandingOutcome = {
    Perfect = "Perfect",
    Hard = "Hard",
    Crash = "Crash",
    Fatal = "Fatal",
    Crater = "Crater",
}

M.LanderError = {
    FlightAlreadyEnded = "FlightAlreadyEnded",
    ExcessiveBurnRate = "ExcessiveBurnRate",
    InvalidDuration = "InvalidDuration",
    ArithmeticOverflow = "ArithmeticOverflow",
}

local LanderError = M.LanderError
local LandingOutcome = M.LandingOutcome

-- ============================================================================
-- CWE-843 / CWE-681 Strict Integer Validation
-- ============================================================================

--- Strict check for Lua 64-bit integer subtype.
--- Rejects floats (including 10.0), booleans, strings, nil, tables.
--- @param v any
--- @return boolean
local function is_valid_int(v)
    return type(v) == "number" and math.type(v) == "integer"
end
M.is_valid_int = is_valid_int

-- ============================================================================
-- Arithmetic Helpers: trunc_div, div_euclid, isqrt
-- ============================================================================

--- Integer division truncated toward zero.
--- Unlike Lua's `//` operator (which floors toward -infinity), this truncates
--- fractional components toward zero identically to Rust/C integer division.
--- @param a integer
--- @param b integer
--- @return integer
local function trunc_div(a, b)
    if b == 0 then
        error("division by zero in trunc_div")
    end
    local q = a // b
    local r = a % b
    if r ~= 0 and ((a < 0) ~= (b < 0)) then
        return q + 1
    end
    return q
end
M.trunc_div = trunc_div

--- Exact Euclidean division where remainder r satisfies 0 <= r < |b|.
--- Guaranteed: a == div_euclid(a, b) * b + r with 0 <= r < |b|.
--- @param a integer
--- @param b integer
--- @return integer
local function div_euclid(a, b)
    if b == 0 then
        error("division by zero in div_euclid")
    end
    local q = trunc_div(a, b)
    local r = a - q * b
    if r < 0 then
        if b > 0 then
            return q - 1
        else
            return q + 1
        end
    end
    return q
end
M.div_euclid = div_euclid

--- Exact integer square root floor(sqrt(n)) using bounded digit-by-digit binary loop.
--- Division-free (CWE-369 safe) and runs in at most 31 iterations for 64-bit integers.
--- @param n integer Non-negative 64-bit integer
--- @return integer
local function isqrt(n)
    if not is_valid_int(n) or n <= 0 then
        return 0
    end
    local res = 0
    local bit = 1 << 60
    while bit > n do
        bit = bit >> 2
    end
    local num = n
    while bit ~= 0 do
        if num >= res + bit then
            num = num - (res + bit)
            res = (res >> 1) + bit
        else
            res = res >> 1
        end
        bit = bit >> 2
    end
    return res
end
M.isqrt = isqrt
M.isqrt_u64 = isqrt
M.isqrt_u128 = isqrt

-- ============================================================================
-- State & Command Management
-- ============================================================================

--- Initializes a new Lunar Module descent mission at standard initial conditions.
--- @return table Fresh LanderState record
local function new_flight()
    return {
        time_seconds = 0,
        altitude_mft = INITIAL_ALTITUDE_MFT,
        velocity_mft_s = INITIAL_VELOCITY_MFT_S,
        fuel_lbs = INITIAL_FUEL_LBS,
        dry_mass_lbs = DRY_MASS_LBS,
        status = "InFlight",
        impact_velocity_mft_s = nil,
    }
end
M.new_flight = new_flight

--- Checks the formal state invariant (VER04m).
--- @param s table LanderState record
--- @return boolean
local function state_inv(s)
    if type(s) ~= "table" then return false end
    if not is_valid_int(s.dry_mass_lbs) or s.dry_mass_lbs ~= DRY_MASS_LBS then return false end
    if not is_valid_int(s.fuel_lbs) or s.fuel_lbs < 0 or s.fuel_lbs > INITIAL_FUEL_LBS then return false end
    if not is_valid_int(s.altitude_mft) or s.altitude_mft < 0 or s.altitude_mft > MAX_ALTITUDE_MFT then return false end
    if not is_valid_int(s.velocity_mft_s) or s.velocity_mft_s < MIN_VELOCITY_MFT_S or s.velocity_mft_s > MAX_VELOCITY_MFT_S then return false end
    if not is_valid_int(s.time_seconds) or s.time_seconds < 0 or s.time_seconds > MAX_TIME_SECONDS then return false end
    return true
end
M.state_inv = state_inv

--- Validates a candidate burn command against current state and physical invariants.
--- Canonical validation precedence:
--- 1. Lifecycle Check: craft must be InFlight.
--- 2. State Invariants: dry mass, fuel capacity, altitude, velocity, mission time, no impact vel.
--- 3. Command Bounds: burn rate (0..=200), duration (1..=10), non-overflowing elapsed time.
--- @param state table LanderState record
--- @param cmd table BurnCommand record
--- @return boolean ok
--- @return string|nil error_code
local function validate_burn(state, cmd)
    if type(state) ~= "table" or type(cmd) ~= "table" then
        return false, LanderError.ArithmeticOverflow
    end

    -- 1. Lifecycle Check: craft must be InFlight
    if state.status ~= "InFlight" then
        return false, LanderError.FlightAlreadyEnded
    end

    -- 2. State Invariants
    if not is_valid_int(state.dry_mass_lbs) or state.dry_mass_lbs ~= DRY_MASS_LBS then
        return false, LanderError.ArithmeticOverflow
    end
    if not is_valid_int(state.fuel_lbs) or state.fuel_lbs < 0 or state.fuel_lbs > INITIAL_FUEL_LBS then
        return false, LanderError.ArithmeticOverflow
    end
    if not is_valid_int(state.altitude_mft) or state.altitude_mft <= 0 or state.altitude_mft > MAX_ALTITUDE_MFT then
        return false, LanderError.ArithmeticOverflow
    end
    if not is_valid_int(state.velocity_mft_s) or state.velocity_mft_s < MIN_VELOCITY_MFT_S or state.velocity_mft_s > MAX_VELOCITY_MFT_S then
        return false, LanderError.ArithmeticOverflow
    end
    if not is_valid_int(state.time_seconds) or state.time_seconds < 0 or state.time_seconds > MAX_TIME_SECONDS then
        return false, LanderError.ArithmeticOverflow
    end
    if state.impact_velocity_mft_s ~= nil then
        return false, LanderError.ArithmeticOverflow
    end

    -- 3. Command Bounds
    if not is_valid_int(cmd.burn_rate_lbs_s) or cmd.burn_rate_lbs_s < 0 then
        return false, LanderError.ArithmeticOverflow
    end
    if cmd.burn_rate_lbs_s > MAX_BURN_RATE_LBS_S then
        return false, LanderError.ExcessiveBurnRate
    end

    if not is_valid_int(cmd.duration_seconds) or cmd.duration_seconds < 0 then
        return false, LanderError.ArithmeticOverflow
    end
    if cmd.duration_seconds < MIN_DURATION_S or cmd.duration_seconds > MAX_DURATION_S then
        return false, LanderError.InvalidDuration
    end

    local t_next = state.time_seconds + cmd.duration_seconds
    if t_next > MAX_TIME_SECONDS then
        return false, LanderError.ArithmeticOverflow
    end

    return true, nil
end
M.validate_burn = validate_burn

--- Integrates a single discrete flight step under commanded burn rate and duration.
--- Implements second-order Euler-Verlet numerical kinematics in pure fixed-point millifeet.
--- Returns an immutable fresh table (VER04m).
--- @param state table LanderState record
--- @param cmd table BurnCommand record
--- @return table|nil next_state Fresh LanderState record or nil on failure
--- @return string|nil error_code LanderError error code or nil on success
local function step_burn(state, cmd)
    local ok, err = validate_burn(state, cmd)
    if not ok then
        return nil, err
    end

    local dt = cmd.duration_seconds
    local fuel_req = cmd.burn_rate_lbs_s * cmd.duration_seconds
    local delta_f = math.min(state.fuel_lbs, fuel_req)
    local fuel_next = state.fuel_lbs - delta_f

    local avg_mass = state.dry_mass_lbs + state.fuel_lbs - trunc_div(delta_f, 2)
    local impulse = delta_f * EXHAUST_VELOCITY_MFT_S
    local a_thrust = trunc_div(impulse, dt * avg_mass)
    local a_net = a_thrust - G_LUNAR_MFT_S2

    local v_next = state.velocity_mft_s + a_net * dt
    local y_next = state.altitude_mft + state.velocity_mft_s * dt + div_euclid(a_net * dt * dt, 2)
    local t_next = state.time_seconds + cmd.duration_seconds

    if y_next > MAX_ALTITUDE_MFT or v_next < MIN_VELOCITY_MFT_S or v_next > MAX_VELOCITY_MFT_S then
        return nil, LanderError.ArithmeticOverflow
    end

    if y_next > 0 then
        return {
            time_seconds = t_next,
            altitude_mft = y_next,
            velocity_mft_s = v_next,
            fuel_lbs = fuel_next,
            dry_mass_lbs = state.dry_mass_lbs,
            status = "InFlight",
            impact_velocity_mft_s = nil,
        }, nil
    else
        -- Ground touchdown solver
        local a_half = div_euclid(a_net, 2)
        local delta_t_raw
        if a_half == 0 then
            if state.velocity_mft_s == 0 then
                return nil, LanderError.ArithmeticOverflow
            end
            delta_t_raw = div_euclid(-state.altitude_mft, state.velocity_mft_s)
        else
            local v = state.velocity_mft_s
            local y = state.altitude_mft
            local disc = v * v - 4 * a_half * y
            local disc_pos = math.max(0, disc)
            local root = isqrt(disc_pos)
            delta_t_raw = div_euclid(-state.velocity_mft_s - root, 2 * a_half)
        end

        local delta_t = math.min(math.max(delta_t_raw, 1), dt)
        local v_touchdown = state.velocity_mft_s + a_net * delta_t
        local t_final = state.time_seconds + delta_t

        local fuel_burned = math.min(state.fuel_lbs, delta_t * cmd.burn_rate_lbs_s)
        local fuel_final = state.fuel_lbs - fuel_burned

        local speed = math.abs(v_touchdown)
        local outcome
        if speed <= 5000 then
            outcome = LandingOutcome.Perfect
        elseif speed <= 15000 then
            outcome = LandingOutcome.Hard
        elseif speed <= 30000 then
            outcome = LandingOutcome.Crash
        elseif speed <= 60000 then
            outcome = LandingOutcome.Fatal
        else
            outcome = LandingOutcome.Crater
        end

        return {
            time_seconds = t_final,
            altitude_mft = 0,
            velocity_mft_s = v_touchdown,
            fuel_lbs = fuel_final,
            dry_mass_lbs = state.dry_mass_lbs,
            status = "Landed(" .. outcome .. ")",
            impact_velocity_mft_s = v_touchdown,
        }, nil
    end
end
M.step_burn = step_burn

--- Parses a flight status string into normalized representation.
--- @param s string
--- @return string
local function parse_status(s)
    if s == "InFlight" then
        return "InFlight"
    end
    local outcome = s:match("^Landed%((%w+)%)$")
    if outcome then
        return s
    end
    error("unknown flight status: " .. tostring(s))
end
M.parse_status = parse_status

-- ============================================================================
-- Declarative 2D PaintScene & ASCII Telemetry HUD
-- ============================================================================

--- Formats an integer with commas as thousands separators.
--- @param n integer
--- @return string
local function format_with_commas(n)
    local negative = n < 0
    local s = tostring(math.abs(n))
    local k
    while true do
        s, k = s:gsub("^(-?%d+)(%d%d%d)", "%1,%2")
        if k == 0 then break end
    end
    if negative then
        return "-" .. s
    end
    return s
end
M.format_with_commas = format_with_commas

--- Right-pads a UTF-8 string with spaces to a minimum character width.
--- @param str string
--- @param width integer
--- @return string
local function pad_right(str, width)
    local len = utf8.len(str) or #str
    if len < width then
        return str .. string.rep(" ", width - len)
    end
    return str
end

--- Translates a flight state into a declarative 76x5 PaintScene.
--- @param state table LanderState record
--- @param last_burn_rate_lbs_s integer|nil Last applied burn rate in lbs/s (default: 0)
--- @return table PaintScene structured table
local function to_paint_scene(state, last_burn_rate_lbs_s)
    last_burn_rate_lbs_s = last_burn_rate_lbs_s or 0

    local alt_clamped = math.min(math.max(state.altitude_mft, 0), INITIAL_ALTITUDE_MFT)
    local alt_pct = (alt_clamped * 100) // INITIAL_ALTITUDE_MFT
    local alt_bars = (alt_clamped * 20) // INITIAL_ALTITUDE_MFT

    local fuel_clamped = math.min(state.fuel_lbs, INITIAL_FUEL_LBS)
    local fuel_pct = (fuel_clamped * 100) // INITIAL_FUEL_LBS
    local fuel_bars = (fuel_clamped * 20) // INITIAL_FUEL_LBS

    local alt_ft = trunc_div(state.altitude_mft, 1000)
    local vel_fps = trunc_div(state.velocity_mft_s, 1000)
    local fuel_lbs = state.fuel_lbs

    local alt_str = format_with_commas(alt_ft)
    local fuel_str = format_with_commas(fuel_lbs)

    local rate_label, rate_text, rate_symbol
    if state.status == "InFlight" then
        if state.velocity_mft_s < 0 then
            rate_label, rate_text, rate_symbol = "RATE:", "DESCENT", "▼"
        elseif state.velocity_mft_s > 0 then
            rate_label, rate_text, rate_symbol = "RATE:", "ASCENT", "▲"
        else
            rate_label, rate_text, rate_symbol = "RATE:", "HOVER", "◆"
        end
    else
        local outcome = state.status:match("^Landed%((%w+)%)$") or "PERFECT"
        rate_label, rate_text, rate_symbol = "STATUS:", outcome:upper(), "★"
    end

    local row0 = "┌───────────────────────── LUNAR LANDER TELEMETRY ─────────────────────────┐"
    local left1 = string.format(" TIME: %6d SEC    ", state.time_seconds)
    local right1 = string.format(" ALTITUDE: %7s FT [░░░░░░░░░░░░░░░░░░░░] %3d%%   ", alt_str, alt_pct)
    local row1 = "│" .. left1 .. "│" .. right1 .. "│"

    local left2 = string.format(" VELOCITY: %5d FT/S", vel_fps)
    local right2 = string.format(" FUEL:    %7s LBS [░░░░░░░░░░░░░░░░░░░░] %3d%%   ", fuel_str, fuel_pct)
    local row2 = "│" .. left2 .. "│" .. right2 .. "│"

    local left3 = string.format(" BURN:     %4d LB/S ", last_burn_rate_lbs_s)
    local rate_col = pad_right(rate_label, 7) .. " " .. pad_right(rate_text, 8) .. " " .. rate_symbol
    local row3 = "│" .. left3 .. "│ " .. pad_right(rate_col, 51) .. "│"
    local row4 = "└──────────────────────────────────────────────────────────────────────────┘"

    local full_text = row0 .. "\n" .. row1 .. "\n" .. row2 .. "\n" .. row3 .. "\n" .. row4

    local instructions = {
        {
            kind = "text",
            type = "text",
            x = 0.0,
            y = 0.0,
            text = full_text,
            fill = "#ffffff",
            font_size = 1.0,
            text_align = "left",
        }
    }

    if alt_bars > 0 then
        instructions[#instructions + 1] = {
            kind = "rect",
            type = "rect",
            x = 46.0,
            y = 1.0,
            width = (alt_bars - 1) * 1.0,
            height = 0.0,
            fill = "#00ff00",
        }
    end

    if fuel_bars > 0 then
        instructions[#instructions + 1] = {
            kind = "rect",
            type = "rect",
            x = 46.0,
            y = 2.0,
            width = (fuel_bars - 1) * 1.0,
            height = 0.0,
            fill = "#00ff00",
        }
    end

    return {
        width = 76.0,
        height = 5.0,
        background = "#000000",
        instructions = instructions,
    }
end
M.to_paint_scene = to_paint_scene

--- Convenience helper creating a PaintScene with zero initial burn.
--- @param state table LanderState record
--- @return table PaintScene structured table
local function telemetry_scene(state)
    return to_paint_scene(state, 0)
end
M.telemetry_scene = telemetry_scene

--- Renders a flight state telemetry dashboard into a deterministic ASCII string.
--- Uses a 5x76 character-cell grid indexed by Unicode codepoints (utf8.codes).
--- @param state table LanderState record
--- @param last_burn_rate_lbs_s integer|nil Last applied burn rate in lbs/s (default: 0)
--- @return string Exactly 5 lines of 76 Unicode characters
local function render_telemetry_ascii(state, last_burn_rate_lbs_s)
    last_burn_rate_lbs_s = last_burn_rate_lbs_s or 0
    local scene = to_paint_scene(state, last_burn_rate_lbs_s)
    local cols = math.tointeger(scene.width) or 76
    local rows = math.tointeger(scene.height) or 5

    -- Initialize 5x76 grid of spaces
    local grid = {}
    for r = 1, rows do
        local row = {}
        for c = 1, cols do
            row[c] = " "
        end
        grid[r] = row
    end

    for _, inst in ipairs(scene.instructions) do
        local itype = inst.kind or inst.type
        if itype == "text" then
            local anchor_col = math.floor(inst.x + 0.5)
            local first_row = math.floor(inst.y + 0.5)
            local line_idx = 0
            for line in (inst.text .. "\n"):gmatch("(.-)\r?\n") do
                local chars = {}
                for _, cp in utf8.codes(line) do
                    chars[#chars + 1] = utf8.char(cp)
                end
                local width = #chars
                local start_col = anchor_col
                if inst.text_align == "center" then
                    start_col = anchor_col - (width // 2)
                elseif inst.text_align == "right" then
                    start_col = anchor_col - width
                end
                local r = first_row + line_idx + 1
                if r >= 1 and r <= rows then
                    for c_idx, ch in ipairs(chars) do
                        local c = start_col + c_idx
                        if c >= 1 and c <= cols then
                            grid[r][c] = ch
                        end
                    end
                end
                line_idx = line_idx + 1
            end
        elseif itype == "rect" then
            local r1 = math.floor(inst.y + 0.5) + 1
            local r2 = math.floor(inst.y + (inst.height or 0.0) + 0.5) + 1
            local c1 = math.floor(inst.x + 0.5) + 1
            local c2 = math.floor(inst.x + (inst.width or 0.0) + 0.5) + 1
            for r = r1, r2 do
                if r >= 1 and r <= rows then
                    for c = c1, c2 do
                        if c >= 1 and c <= cols then
                            grid[r][c] = "█"
                        end
                    end
                end
            end
        end
    end

    local lines = {}
    for r = 1, rows do
        lines[r] = table.concat(grid[r])
    end
    return table.concat(lines, "\n")
end
M.render_telemetry_ascii = render_telemetry_ascii

--- Convenience helper rendering a flight state telemetry dashboard with zero burn rate.
--- @param state table LanderState record
--- @return string Exactly 5 lines of 76 Unicode characters
local function render_telemetry(state)
    return render_telemetry_ascii(state, 0)
end
M.render_telemetry = render_telemetry

return M
