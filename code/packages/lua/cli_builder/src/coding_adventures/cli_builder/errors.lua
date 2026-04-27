-- errors.lua -- CLI Builder Error Types
-- =====================================

local Errors = {}

-- SpecError is for load-time failures in the JSON specification.
Errors.SpecError = function(message)
    return {
        type = "spec_error",
        message = message
    }
end

-- ParseError identifiers (snake_case)
Errors.PARSE_ERRORS = {
    UNKNOWN_COMMAND           = "unknown_command",
    UNKNOWN_FLAG              = "unknown_flag",
    MISSING_REQUIRED_FLAG     = "missing_required_flag",
    MISSING_REQUIRED_ARG      = "missing_required_argument",
    CONFLICTING_FLAGS         = "conflicting_flags",
    MISSING_DEP_FLAG          = "missing_dependency_flag",
    TOO_FEW_ARGS              = "too_few_arguments",
    TOO_MANY_ARGS             = "too_many_arguments",
    INVALID_VALUE             = "invalid_value",
    INVALID_ENUM_VALUE        = "invalid_enum_value",
    EXCLUSIVE_GROUP_VIOLATION = "exclusive_group_violation",
    MISSING_EXCLUSIVE_GROUP   = "missing_exclusive_group",
    DUPLICATE_FLAG            = "duplicate_flag",
    INVALID_STACK             = "invalid_stack",
}

--- Create a single parse error.
--
-- @param error_type string One of Errors.PARSE_ERRORS.
-- @param message string Human-readable explanation.
-- @param suggestion string|nil Optional hint (fuzzy match).
-- @param context table|nil Command path where detected.
-- @return table The error object.
Errors.ParseError = function(error_type, message, suggestion, context)
    return {
        type = error_type,
        message = message,
        suggestion = suggestion,
        context = context
    }
end

--- levenshtein computes the edit distance between two strings.
-- Distances ≤ 2 are suitable for fuzzy matching.
--
-- @param s string
-- @param t string
-- @return number
Errors.levenshtein = function(s, t)
    if s == t then return 0 end
    local m = #s
    local n = #t
    if m == 0 then return n end
    if n == 0 then return m end

    local prev = {}
    local curr = {}
    for j = 0, n do prev[j] = j end

    for i = 1, m do
        curr[0] = i
        local si = s:sub(i, i)
        for j = 1, n do
            local cost = (si == t:sub(j, j)) and 0 or 1
            local ins = curr[j - 1] + 1
            local del = prev[j] + 1
            local sub = prev[j - 1] + cost
            curr[j] = math.min(ins, math.min(del, sub))
        end
        for j = 0, n do prev[j] = curr[j] end
    end
    return prev[n]
end

--- fuzzy_match finds the best match for the unknown token.
--
-- @param unknown string The token to match.
-- @param candidates table List of possible strings.
-- @return string|nil The suggestion, or nil if none within distance 2.
Errors.fuzzy_match = function(unknown, candidates)
    local best = nil
    local best_dist = 3
    for _, c in ipairs(candidates) do
        local d = Errors.levenshtein(unknown, c)
        if d < best_dist then
            best_dist = d
            best = c
        end
    end
    return best
end

return Errors
