-- coding_adventures.rank_statistics
--
-- Reusable medians and exact distribution-free confidence intervals for a
-- population median. Lua's native integer is signed 64-bit, but binomial
-- counts quickly exceed that width; the small natural-number type below keeps
-- only the operations needed by the cutoff recurrence.

local M = { VERSION = "0.1.0" }
local BASE = 1000000000

local function nat(value)
    local digits = {}
    repeat
        digits[#digits + 1] = value % BASE
        value = value // BASE
    until value == 0
    return digits
end

local function nat_add(left, right)
    local out, carry = {}, 0
    local count = math.max(#left, #right)
    for i = 1, count do
        local sum = (left[i] or 0) + (right[i] or 0) + carry
        if sum >= BASE then sum, carry = sum - BASE, 1 else carry = 0 end
        out[i] = sum
    end
    if carry ~= 0 then out[#out + 1] = carry end
    return out
end

local function nat_mul_small(value, factor)
    local out, carry = {}, 0
    for i = 1, #value do
        local product = value[i] * factor + carry
        out[i] = product % BASE
        carry = product // BASE
    end
    while carry > 0 do
        out[#out + 1] = carry % BASE
        carry = carry // BASE
    end
    return out
end

local function nat_div_small(value, divisor)
    local out, remainder = {}, 0
    for i = #value, 1, -1 do
        local combined = remainder * BASE + value[i]
        out[i] = combined // divisor
        remainder = combined % divisor
    end
    assert(remainder == 0, "binomial recurrence division must be exact")
    while #out > 1 and out[#out] == 0 do out[#out] = nil end
    return out
end

local function nat_compare(left, right)
    if #left ~= #right then return #left < #right and -1 or 1 end
    for i = #left, 1, -1 do
        if left[i] ~= right[i] then return left[i] < right[i] and -1 or 1 end
    end
    return 0
end

local function exact_median_rank(count, confidence_percent)
    local maximum_tail = nat(100 - confidence_percent)
    for _ = 1, count do maximum_tail = nat_mul_small(maximum_tail, 2) end
    local coefficient, tail, rank = nat(1), nat(0), 0
    for j = 0, count // 2 do
        tail = nat_add(tail, coefficient)
        if nat_compare(nat_mul_small(tail, 200), maximum_tail) > 0 then break end
        rank = j + 1
        coefficient = nat_div_small(nat_mul_small(coefficient, count - j), j + 1)
    end
    return rank
end

local function finite_number(value)
    return type(value) == "number" and value == value and value ~= math.huge and value ~= -math.huge
end

local function sorted_copy(values)
    assert(type(values) == "table", "sample must be an array table")
    local count = #values
    assert(count > 0, "sample must not be empty")
    local observed = 0
    for key in pairs(values) do
        assert(math.type(key) == "integer" and key >= 1 and key <= count, "sample must be a dense array")
        observed = observed + 1
    end
    assert(observed == count, "sample must be a dense array")
    local copy = {}
    for i = 1, count do
        local value = values[i]
        assert(finite_number(value), "sample values must be finite numbers")
        copy[i] = value + 0.0 -- Normalize Lua integers to binary64 observations.
    end
    table.sort(copy)
    return copy
end

local function midpoint(low, high)
    local sum = low + high
    if finite_number(sum) then return sum / 2.0 end
    return low / 2.0 + high / 2.0
end

--- Return the non-mutating sample median of a non-empty dense array.
function M.median(values)
    local ordered = sorted_copy(values)
    local middle = #ordered // 2
    if #ordered % 2 == 1 then return ordered[middle + 1] end
    return midpoint(ordered[middle], ordered[middle + 1])
end

--- Return {low, high} or nil when no exact rank meets the confidence cutoff.
function M.median_interval(values, confidence_percent)
    assert(math.type(confidence_percent) == "integer" and confidence_percent >= 1 and confidence_percent <= 99,
        "confidence must be an integer from 1 through 99")
    local ordered = sorted_copy(values)
    local rank = exact_median_rank(#ordered, confidence_percent)
    if rank == 0 then return nil end
    return { ordered[rank], ordered[#ordered - rank + 1] }
end

return M
