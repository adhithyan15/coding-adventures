-- token_classifier.lua -- Categorizes argv tokens into typed events
-- ===============================================================

local TokenClassifier = {}
TokenClassifier.__index = TokenClassifier

-- TokenKind constants
TokenClassifier.KIND = {
    END_OF_FLAGS      = "end_of_flags",
    LONG_FLAG         = "long_flag",
    LONG_FLAG_VAL     = "long_flag_with_value",
    SDL               = "single_dash_long",
    SHORT             = "short_flag",
    SHORT_VAL         = "short_flag_with_value",
    STACKED           = "stacked_flags",
    POSITIONAL        = "positional",
    UNKNOWN           = "unknown_flag",
}

local function stringField(tbl, field)
    local v = tbl[field]
    if type(v) == "string" then return v end
    return ""
end

function TokenClassifier.new(activeFlags)
    local self = setmetatable({}, TokenClassifier)
    self.longFlags = {}
    self.shortFlags = {}
    self.singleDashLongs = {}

    for _, f in ipairs(activeFlags) do
        local long = stringField(f, "long")
        if long ~= "" and not self.longFlags[long] then
            self.longFlags[long] = f
        end
        local short = stringField(f, "short")
        if short ~= "" and not self.shortFlags[short] then
            self.shortFlags[short] = f
        end
        local sdl = stringField(f, "single_dash_long")
        if sdl ~= "" and not self.singleDashLongs[sdl] then
            self.singleDashLongs[sdl] = f
        end
    end

    return self
end

function TokenClassifier:classify(token)
    local raw = token
    if token == "-" then
        return { kind = self.KIND.POSITIONAL, name = token, raw = raw }
    end
    if token == "--" then
        return { kind = self.KIND.END_OF_FLAGS, raw = raw }
    end

    -- Long flags
    if token:sub(1, 2) == "--" then
        local rest = token:sub(3)
        local eq = rest:find("=")
        if eq then
            local name = rest:sub(1, eq - 1)
            local val = rest:sub(eq + 1)
            return { kind = self.KIND.LONG_FLAG_VAL, name = name, value = val, raw = raw }
        end
        if self.longFlags[rest] then
            return { kind = self.KIND.LONG_FLAG, name = rest, raw = raw }
        end
        return { kind = self.KIND.UNKNOWN, name = rest, raw = raw }
    end

    -- Single-dash tokens
    if token:sub(1, 1) == "-" and #token >= 2 then
        local rest = token:sub(2)

        -- SDL
        if self.singleDashLongs[rest] then
            return { kind = self.KIND.SDL, name = rest, raw = raw }
        end

        -- Short flag
        local firstChar = rest:sub(1, 1)
        local flagDef = self.shortFlags[firstChar]
        if flagDef then
            local flagType = stringField(flagDef, "type")
            if flagType == "boolean" or flagType == "count" then
                if #rest == 1 then
                    return { kind = self.KIND.SHORT, name = firstChar, raw = raw }
                end
                return self:classifyStacked(rest, raw)
            end
            if #rest == 1 then
                return { kind = self.KIND.SHORT, name = firstChar, raw = raw }
            end
            -- Inline value check
            local allKnown = true
            for i = 2, #rest do
                if not self.shortFlags[rest:sub(i, i)] then
                    allKnown = false
                    break
                end
            end
            if allKnown then
                return { kind = self.KIND.UNKNOWN, name = firstChar, raw = raw }
            end
            return { kind = self.KIND.SHORT_VAL, name = firstChar, value = rest:sub(2), raw = raw }
        end

        -- Stacked
        if #rest > 1 then
            return self:classifyStacked(rest, raw)
        end

        return { kind = self.KIND.UNKNOWN, name = rest, raw = raw }
    end

    return { kind = self.KIND.POSITIONAL, name = token, raw = raw }
end

function TokenClassifier:classifyStacked(chars, raw)
    local result = {}
    for i = 1, #chars do
        local ch = chars:sub(i, i)
        local flagDef = self.shortFlags[ch]
        if not flagDef then
            return { kind = self.KIND.UNKNOWN, name = ch, raw = raw }
        end
        local flagType = stringField(flagDef, "type")
        if flagType ~= "boolean" and flagType ~= "count" and i < #chars then
            return { kind = self.KIND.UNKNOWN, name = ch, raw = raw }
        end
        table.insert(result, ch)
    end
    return { kind = self.KIND.STACKED, chars = result, raw = raw }
end

return TokenClassifier
