-- ============================================================================
-- Bit-exact conformance of trig against the shared TRIG-REF-1 vectors
-- ============================================================================
--
-- code/specs/fixtures/trig-reference-v1 holds every vector as one row
--
--     op <TAB> x <TAB> y <TAB> cr <TAB> flags
--
-- where x and y are IEEE binary64 BIT PATTERNS (16 lowercase hex digits),
-- never decimals.  This test decodes x by a bit cast, calls the public
-- function, encodes the result back to bits, and compares the two INTEGERS
-- (PHY00a 6.5).  Printed floats are never compared: tostring() prints only
-- 14 significant digits in Lua, which would hide the very last bits this
-- test exists to check.  The one relaxation is NaN, which matches any NaN
-- (VER04p "same").
--
-- Bit casts in Lua 5.4:
--
--     string.unpack("<d", string.pack("<i8", bits))   -- integer -> float
--     string.unpack("<i8", string.pack("<d", value))  -- float -> integer
--
-- "<i8" is a signed little-endian 64-bit integer and "<d" a little-endian
-- double, so the pair is a pure reinterpretation of the same eight bytes.
-- Lua integers are 64-bit two's complement, and tonumber(hex, 16) wraps
-- modulo 2^64, so a pattern with the top bit set (a negative double) simply
-- becomes a negative Lua integer; the bytes are the same.

package.path = "../src/?.lua;" .. "../src/?/init.lua;" .. package.path

local trig = require("coding_adventures.trig")

-- ---------------------------------------------------------------------------
-- Locating the fixture
-- ---------------------------------------------------------------------------

-- The BUILD runs busted from the tests/ directory, so the fixture is four
-- levels up and then specs/fixtures/...  We derive the path from this file's
-- own location when the interpreter tells us, and fall back to the
-- working-directory-relative path otherwise.
local function here()
    local src = debug.getinfo(1, "S").source
    if src:sub(1, 1) == "@" then
        local dir = src:sub(2):match("^(.*)[/\\][^/\\]*$")
        if dir then return dir end
    end
    return "."
end

local FIXTURE = here() .. "/../../../../specs/fixtures/trig-reference-v1"

local GROUPS = { "edge", "sweep-binade", "sweep-cw", "sweep-prng" }

local QNAN_BITS = 0x7FF8000000000000

local OPS = {}
for _, op in ipairs({ "sin", "cos", "tan", "radians", "degrees", "sqrt", "atan", "atan2" }) do
    OPS[op] = true
end

-- Accuracy limits in representable steps (ulps) against the `cr` column.
-- PHY00a 3.8 makes NO accuracy claim for tan, so it is named here as a
-- no-claim op rather than silently skipped.  Part II ops carry no `cr`.
local LIMITS = { sin = 3, cos = 3, radians = 1, degrees = 1 }
local NO_CLAIM = { tan = true }

-- ---------------------------------------------------------------------------
-- Bit casts
-- ---------------------------------------------------------------------------

local function bits_to_float(bits)
    return (string.unpack("<d", string.pack("<i8", bits)))
end

local function float_to_bits(value)
    return (string.unpack("<i8", string.pack("<d", value)))
end

local function hex(s)
    return tonumber(s, 16)
end

local function fmt(bits)
    return string.format("%016x", bits)
end

-- ---------------------------------------------------------------------------
-- Loading the vectors
-- ---------------------------------------------------------------------------

local function read_file(path)
    local f = assert(io.open(path, "rb"))
    local text = f:read("a")
    f:close()
    return text
end

local function load_group(group)
    local text = read_file(FIXTURE .. "/" .. group .. ".tsv")
    assert(text:sub(-1) == "\n", group .. ".tsv does not end in a newline")
    local rows = {}
    for line in text:gmatch("([^\n]*)\n") do
        local fields = {}
        for field in (line .. "\t"):gmatch("([^\t]*)\t") do
            fields[#fields + 1] = field
        end
        rows[#rows + 1] = {
            group = group,
            op = fields[1],
            x = fields[2],
            y = fields[3],
            cr = fields[4],
            flags = fields[5],
        }
    end
    return rows
end

local ROWS = {}
local ROWS_BY_GROUP = {}
for _, group in ipairs(GROUPS) do
    ROWS_BY_GROUP[group] = load_group(group)
    for _, row in ipairs(ROWS_BY_GROUP[group]) do
        ROWS[#ROWS + 1] = row
    end
end

--- Call the public function for a row.  atan2 rows carry "<y>,<x>".
local function call_op(op, x)
    local fn = trig[op]
    if fn == nil then
        error("trig has no function " .. tostring(op))
    end
    if op == "atan2" then
        local ys, xs = x:match("^(%x+),(%x+)$")
        return fn(bits_to_float(hex(ys)), bits_to_float(hex(xs)))
    end
    return fn(bits_to_float(hex(x)))
end

local function is_nan(v)
    return v ~= v
end

-- Map a bit pattern onto a signed integer line on which adjacent doubles
-- differ by exactly one, so |ordered(a) - ordered(b)| is the distance in
-- steps.  (Bit 63 set means the integer is negative in Lua.)
local function ordered(bits)
    local mag = bits & 0x7FFFFFFFFFFFFFFF
    if bits < 0 then return -mag end
    return mag
end

-- ---------------------------------------------------------------------------
-- The tests
-- ---------------------------------------------------------------------------

describe("TRIG-REF-1 conformance", function()
    it("has the fixture groups the manifest describes (row counts)", function()
        local manifest = read_file(FIXTURE .. "/manifest.json")
        assert.is_truthy(manifest:find('"algorithm": "TRIG%-REF%-1"'))
        for _, group in ipairs(GROUPS) do
            local count = #ROWS_BY_GROUP[group]
            assert.is_true(count > 0, group)
            -- manifest.json lists {"file": "<g>.tsv", "name": ..., "rows": N, ...}
            -- Escape the "-" in "sweep-binade": it is a pattern quantifier.
            local escaped = group:gsub("%-", "%%-")
            local rows = manifest:match('"file": "' .. escaped .. '%.tsv"[^}]-"rows": (%d+)')
            assert.are.equal(tonumber(rows), count, group)
        end
    end)

    it("has well-formed rows with known ops", function()
        local hexdigit = "^%x%x%x%x%x%x%x%x%x%x%x%x%x%x%x%x$"
        for _, row in ipairs(ROWS) do
            assert.is_true(OPS[row.op] == true, "unknown op " .. tostring(row.op))
            local parts = {}
            for p in row.x:gmatch("[^,]+") do parts[#parts + 1] = p end
            assert.are.equal(row.op == "atan2" and 2 or 1, #parts)
            for _, p in ipairs(parts) do
                assert.is_truthy(p:match(hexdigit), p)
                assert.is_true(p == p:lower(), p)
            end
            if row.y ~= "error" then assert.is_truthy(row.y:match(hexdigit), row.y) end
            if row.cr ~= "-" then assert.is_truthy(row.cr:match(hexdigit), row.cr) end
        end
    end)

    it("embeds constants whose bits equal the manifest's", function()
        local manifest = read_file(FIXTURE .. "/manifest.json")
        local consts = {}
        local block = manifest:match('"constants": (%b{})')
        for name, value in block:gmatch('"([%w_]+)": "(%x+)"') do
            consts[name] = hex(value)
        end
        local function check(name, value)
            assert.is_not_nil(value, name .. " is not exported by trig")
            assert.is_true(math.type(value) == "float", name .. " must be a float")
            assert.are.equal(consts[name], float_to_bits(value), name)
        end
        check("PI", trig.PI)
        check("TWO_PI", trig.TWO_PI)
        check("HALF_PI", trig.HALF_PI)
        check("INV_PIO2", trig.INV_PIO2)
        assert.is_not_nil(trig.PIO2, "trig.PIO2 is not exported")
        for i = 1, 5 do check("PIO2_" .. i, trig.PIO2[i]) end
        check("MAGIC", trig.MAGIC)
        check("LIMIT", trig.LIMIT)
        check("TINY", trig.TINY)
        assert.is_not_nil(trig.S, "trig.S is not exported")
        assert.is_not_nil(trig.C, "trig.C is not exported")
        for n = 1, 9 do check("S" .. n, trig.S[n]) end
        for n = 2, 9 do check("C" .. n, trig.C[n]) end
        check("RAD_PER_DEG", trig.RAD_PER_DEG)
        check("DEG_PER_RAD", trig.DEG_PER_RAD)
    end)

    -- Decimal literals written in the source must parse to the pinned bits
    -- (PHY00a 6.5e): "the literal parses to the nearest double" is a
    -- property of the interpreter's strtod, not of this package.
    it("parses the decimal literals the source uses to the pinned bits", function()
        local literals = {
            { 1e-15, 0x3CD203AF9EE75616 },
            { 1.0e308, 0x7FE1CCF385EBC8A0 },
            { 1e-17, 0x3C670EF54646D497 },
            { 1e-300, 0x01A56E1FC2F8F359 },
            { 7.450580596923828e-9, 0x3E40000000000000 },
        }
        for _, pair in ipairs(literals) do
            assert.are.equal(pair[2], float_to_bits(pair[1]))
        end
    end)

    it("has degree constants equal to the divisions", function()
        assert.is_not_nil(trig.RAD_PER_DEG, "trig.RAD_PER_DEG is not exported")
        assert.is_not_nil(trig.DEG_PER_RAD, "trig.DEG_PER_RAD is not exported")
        assert.are.equal(float_to_bits(trig.PI / 180.0), float_to_bits(trig.RAD_PER_DEG))
        assert.are.equal(float_to_bits(180.0 / trig.PI), float_to_bits(trig.DEG_PER_RAD))
    end)

    -- Every vector, bit for bit.  A failing group reports the number of
    -- failing rows per op and the first few examples.
    for _, group in ipairs(GROUPS) do
        it("matches every vector bit for bit: " .. group, function()
            local failures, examples, checked = {}, {}, 0
            for _, row in ipairs(ROWS_BY_GROUP[group]) do
                checked = checked + 1
                local op = row.op
                local problem
                local ok, got = pcall(call_op, op, row.x)
                if row.y == "error" then
                    if ok then problem = "no error raised" end
                elseif not ok then
                    problem = "raised: " .. tostring(got)
                elseif hex(row.y) == QNAN_BITS then
                    if not is_nan(got) then
                        problem = "expected NaN, got " .. fmt(float_to_bits(got))
                    end
                elseif math.type(got) ~= "float" then
                    problem = "expected a float, got " .. tostring(math.type(got))
                elseif float_to_bits(got) ~= hex(row.y) then
                    problem = "expected " .. row.y .. ", got " .. fmt(float_to_bits(got))
                end
                if problem then
                    failures[op] = (failures[op] or 0) + 1
                    if #examples < 5 then
                        examples[#examples + 1] = op .. " " .. row.x .. ": " .. problem
                    end
                end
            end
            assert.is_true(checked > 0)
            local summary = {}
            for op, n in pairs(failures) do summary[#summary + 1] = op .. "=" .. n end
            table.sort(summary)
            assert.is_true(#summary == 0,
                "failing rows per op: " .. table.concat(summary, ", ")
                .. "; first: " .. table.concat(examples, " | "))
        end)
    end

    -- Accuracy against the correctly rounded column.
    it("is within the accuracy limits of the correctly rounded column", function()
        local worst = {}
        for op in pairs(LIMITS) do worst[op] = 0 end
        for _, row in ipairs(ROWS) do
            local op = row.op
            if row.cr ~= "-" and not NO_CLAIM[op] then
                -- Fail closed: an op with a cr value but no limit is an
                -- error, so a future op cannot be skipped silently.
                assert.is_true(LIMITS[op] ~= nil,
                    "op " .. op .. " has a cr value but no accuracy limit")
                local got = call_op(op, row.x)
                local steps = math.abs(ordered(float_to_bits(got)) - ordered(hex(row.cr)))
                if steps > worst[op] then worst[op] = steps end
            end
        end
        for op, bound in pairs(LIMITS) do
            assert.is_true(worst[op] <= bound, op .. " worst=" .. worst[op])
        end
    end)

    it("fails on an unknown op", function()
        assert.is_false((pcall(call_op, "no_such_op", "0000000000000000")))
        -- A row naming an op we do not know is itself a corpus error.
        for _, row in ipairs(ROWS) do assert.is_true(OPS[row.op] == true) end
    end)

    -- ---------------------------------------------------------------------
    -- The behaviours PHY00a 3.3 names, asserted directly
    -- ---------------------------------------------------------------------

    it("sin(-0.0) is -0.0 and sin(0.0) is +0.0", function()
        assert.are.equal(0x8000000000000000 | 0, float_to_bits(trig.sin(-0.0)))
        assert.are.equal(0, float_to_bits(trig.sin(0.0)))
    end)

    it("cos(+-0.0) is 1", function()
        assert.are.equal(1.0, trig.cos(0.0))
        assert.are.equal(1.0, trig.cos(-0.0))
    end)

    it("returns tiny arguments unchanged (sin) and 1 (cos)", function()
        local tiny = bits_to_float(0x3E3FFFFFFFFFFFFF)
        assert.are.equal(float_to_bits(tiny), float_to_bits(trig.sin(tiny)))
        assert.are.equal(float_to_bits(5e-324), float_to_bits(trig.sin(5e-324)))
        assert.are.equal(1.0, trig.cos(tiny))
    end)

    it("returns NaN outside the domain |x| <= 2^29", function()
        local inf = math.huge
        local cases = { 0.0 / 0.0, inf, -inf, 1e20, 2.0 ^ 29 * (1 + 2.0 ^ -52), 1.7976931348623157e308 }
        for _, x in ipairs(cases) do
            for _, fn in ipairs({ "sin", "cos", "tan" }) do
                assert.is_true(is_nan(trig[fn](x)), fn .. "(" .. tostring(x) .. ")")
            end
        end
    end)

    it("returns a finite value at the last in-domain argument", function()
        assert.are.equal(0x3FD4E67C0E2622DF, float_to_bits(trig.sin(2.0 ^ 29)))
        assert.are.equal(0xBFEE3EDD2DFEEF90 | 0, float_to_bits(trig.cos(2.0 ^ 29)))
    end)

    -- Lua 5.4 has an integer subtype.  An integer argument must be converted
    -- once at entry and never reach integer arithmetic inside the algorithm.
    it("converts integer arguments to floats once", function()
        assert.are.equal(float_to_bits(trig.sin(1.0)), float_to_bits(trig.sin(1)))
        assert.are.equal(float_to_bits(trig.cos(3.0)), float_to_bits(trig.cos(3)))
        assert.are.equal(float_to_bits(trig.PI), float_to_bits(trig.radians(180)))
        assert.are.equal("float", math.type(trig.sin(0)))
        assert.are.equal("float", math.type(trig.cos(0)))
        assert.are.equal("float", math.type(trig.radians(0)))
        assert.are.equal("float", math.type(trig.degrees(0)))
        assert.are.equal("float", math.type(trig.tan(0)))
        assert.are.equal("float", math.type(trig.atan(0)))
        assert.are.equal("float", math.type(trig.atan2(0, 1)))
        assert.are.equal("float", math.type(trig.sqrt(0)))
        assert.are.equal("float", math.type(trig.sqrt(4)))
        -- A huge integer must not wrap: -x*x on an integer would.
        assert.are.equal(float_to_bits(trig.sin(1e9 * 1.0)), float_to_bits(trig.sin(1000000000)))
        assert.are.equal(0x8000000000000000 | 0, float_to_bits(trig.radians(-0.0)))
        assert.are.equal(0x8000000000000000 | 0, float_to_bits(trig.degrees(-0.0)))
    end)

    it("radians and degrees are single multiplies", function()
        assert.are.equal(float_to_bits(trig.PI), float_to_bits(trig.radians(180.0)))
        assert.are.equal(180.0, trig.degrees(trig.PI))
        assert.are.equal(math.huge, trig.degrees(1.7e308))
        assert.is_true(is_nan(trig.radians(0.0 / 0.0)))
    end)

    it("keeps the tan pole sentinel", function()
        assert.are.equal(1e308, trig.tan(trig.HALF_PI))
        assert.are.equal(-1e308, trig.tan(-trig.HALF_PI))
    end)

    it("atan2 of a NaN argument is NaN", function()
        local nan = 0.0 / 0.0
        assert.is_true(is_nan(trig.atan2(nan, 1.0)))
        assert.is_true(is_nan(trig.atan2(1.0, nan)))
        assert.is_true(is_nan(trig.atan2(-1.0, nan)))
        assert.is_true(is_nan(trig.atan2(nan, nan)))
    end)

    it("pins the atan2 conventions that are not IEEE's", function()
        assert.are.equal(float_to_bits(trig.PI), float_to_bits(trig.atan2(-0.0, -1.0)))
        assert.are.equal(0, float_to_bits(trig.atan2(0.0, 0.0)))
        assert.is_true(is_nan(trig.atan2(math.huge, math.huge)))
        assert.are.equal(float_to_bits(trig.HALF_PI), float_to_bits(trig.atan2(1e300, 1e-300)))
    end)

    it("atan keeps the sign of zero", function()
        assert.are.equal(0x8000000000000000 | 0, float_to_bits(trig.atan(-0.0)))
    end)

    it("uses floor-modulus quadrants for negative k", function()
        -- k = -1 and k = -5 map to quadrant 3: "& 3" is the floor modulus.
        assert.are.equal(float_to_bits(-trig.sin(trig.HALF_PI)), float_to_bits(trig.sin(-trig.HALF_PI)))
        assert.is_true(math.abs(trig.cos(-5 * trig.HALF_PI)) < 1e-15)
    end)
end)
