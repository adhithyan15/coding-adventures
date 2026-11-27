package.path = "src/?.lua;src/?/init.lua;" .. package.path
local stats = require("coding_adventures.rank_statistics")

local function equal(actual, expected, message)
    assert(actual == expected, (message or "values differ") .. ": got " .. tostring(actual) .. ", want " .. tostring(expected))
end
local function rejects(fn, fragment)
    local ok, err = pcall(fn)
    assert(not ok and tostring(err):find(fragment, 1, true), "expected error containing " .. fragment)
end

local input = { 4, 1, 3, 2 }
equal(stats.median(input), 2.5, "even median")
equal(table.concat(input, ","), "4,1,3,2", "caller array preserved")
local d = 4.9406564584124654e-324
equal(stats.median({ d, 2 * d }), 2 * d, "positive subnormal ties-to-even")
equal(stats.median({ -2 * d, -d }), -2 * d, "negative subnormal ties-to-even")
local normal = 2.2250738585072014e-308
equal(stats.median({ normal - d, normal }), normal, "normal boundary")
equal(stats.median({ -1.7976931348623157e308, 1.7976931348623157e308 }), 0, "opposite maximum finite values")
equal(stats.median({ 1.7976931348623157e308, 1.7976931348623157e308 }), 1.7976931348623157e308, "same-sign maximum finite values")
equal(stats.median_interval({ 1 }, 95), nil, "insufficient sample has no interval")
rejects(function() stats.median({}) end, "empty")
rejects(function() stats.median({ 1, 0 / 0 }) end, "finite")
rejects(function() stats.median({ [1] = 1, [3] = 2 }) end, "dense")
rejects(function() stats.median_interval({ 1 }, 0) end, "confidence")

-- Direct bounded oracle: enumerate every assignment of n signs, then compare
-- exact integer counts instead of reusing the production coefficient recurrence.
for n = 1, 12 do
    for confidence = 1, 99 do
        local expected = 0
        for k = 1, n // 2 do
            local tail = 0
            for mask = 0, (1 << n) - 1 do
                local bits, value = 0, mask
                while value > 0 do bits, value = bits + value % 2, value // 2 end
                if bits < k then tail = tail + 1 end
            end
            if 200 * tail <= (100 - confidence) * (1 << n) then expected = k else break end
        end
        local interval = stats.median_interval({ table.unpack((function()
            local values = {}; for i = 1, n do values[i] = i end; return values
        end)()) }, confidence)
        if expected == 0 then equal(interval, nil, "bounded oracle no interval")
        else equal(interval[1], expected, "bounded oracle lower endpoint") end
    end
end

local cases = {
    {64, 95, 23, 40}, {65, 99, 21, 43}, {127, 95, 51, 75},
    {128, 95, 52, 75}, {256, 99, 106, 149}, {1000, 95, 468, 531},
}
if os.getenv("CA_COVERAGE_MODE") ~= "1" then
    cases[#cases + 1] = {10000, 50, 4965, 5034}
    cases[#cases + 1] = {10000, 95, 4901, 5098}
    cases[#cases + 1] = {10000, 99, 4870, 5129}
end
for _, case in ipairs(cases) do
    local values = {}; for i = 0, case[1] - 1 do values[#values + 1] = i end
    local interval = stats.median_interval(values, case[2])
    equal(interval[1], case[3], "wide cutoff low n=" .. case[1])
    equal(interval[2], case[4], "wide cutoff high n=" .. case[1])
end

local state = 0x5eed
for trial = 1, 300 do
    state = (1103515245 * state + 12345) % 2147483648
    local values = {}; for i = 1, 1 + state % 64 do
        state = (1103515245 * state + 12345) % 2147483648
        values[i] = state % 2001 - 1000
    end
    local original = table.concat(values, ",")
    local median = stats.median(values)
    local interval = stats.median_interval(values, 90)
    assert(median >= -1000 and median <= 1000)
    assert(interval == nil or (interval[1] <= median and median <= interval[2]))
    equal(table.concat(values, ","), original, "generated input unchanged")
end

if os.getenv("CA_COVERAGE_MODE") == "1" then
    print("Lua rank-statistics: 1,188 bounded cutoffs, 6 representative wide vectors, 300 generated samples passed")
else
    print("Lua rank-statistics: 1,188 bounded cutoffs, 9 wide vectors, 300 generated samples passed")
end
