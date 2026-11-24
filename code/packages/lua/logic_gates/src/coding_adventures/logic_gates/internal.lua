-- internal.lua — private kit shared by gates.lua and sequential.lua
-- ==================================================================
--
-- This module owns everything that can mint a GENUINE logic_gates error:
-- the private registry, the error constructors, the bit validator and the
-- strict table snapshot.  None of it is re-exported by the public modules,
-- so a caller cannot manufacture an error that `is_invalid_bit` & co. accept.
--
-- This module returns a FACTORY.  Each call builds a brand-new kit with its
-- OWN private registry.  gates.lua calls it exactly once, keeps the kit in a
-- local, and passes it as an argument to the sequential factory; the kit is
-- never stored anywhere a caller can reach.  Anyone who calls the factory
-- themselves gets an unrelated kit whose errors the package does not accept.
--
-- Capturing globals
-- -----------------
--
-- Every global used anywhere in this package is copied into a local when
-- the file loads.  If a caller later does `_G.error = function() end` or
-- replaces `string.format`, the package still runs the ORIGINAL functions:
-- validation can never fail open.  (`luac -l` shows no GETTABUP _ENV in the
-- function bodies below; the test-suite replaces the globals and checks.)

local type, mtype, sformat, select, setmetatable, getmetatable, rawget, rawequal, next, error =
    type, math.type, string.format, select, setmetatable, getmetatable, rawget, rawequal, next, error

local function build()
    local INVALID_BIT = "INVALID_BIT"
    local INVALID_ARITY = "INVALID_ARITY"
    local INVALID_WIDTH = "INVALID_WIDTH"
    local INVALID_SHAPE = "INVALID_SHAPE"
    local INTERNAL = "INTERNAL"

    local kit = {
        INVALID_BIT = INVALID_BIT, INVALID_ARITY = INVALID_ARITY,
        INVALID_WIDTH = INVALID_WIDTH, INVALID_SHAPE = INVALID_SHAPE,
        INTERNAL = INTERNAL,
        --- Largest number of inputs ANDn / ORn accept.
        MAX_INPUTS = 65536,
        --- Largest list a table-taking function will snapshot.
        MAX_LIST = 65536,
    }

    -- Unforgeable, read-only error objects
    -- ------------------------------------
    --
    -- A genuine error is recognised by IDENTITY: every error we create is
    -- recorded in a private weak-keyed registry (GENUINE); the predicates do a
    -- `rawget` on it.  A forged table that copies our fields or metatable is not
    -- accepted, and a genuine error stays genuine when tampered with.
    --
    -- The caller receives an EMPTY proxy served from a private record through
    -- `__index`.  `__newindex` raises, so `err.code = "x"` fails, and
    -- `__metatable` is locked.  `tostring(err)` formats the private record that
    -- was captured at creation (fixed text only), never anything a caller could
    -- have written.  `rawset` on the proxy remains possible in Lua; it can only
    -- shadow the convenience field a caller reads -- `error_code`, the
    -- predicates and `tostring` never consult it.
    local GENUINE = setmetatable({}, { __mode = "k" })

    local function readonly_newindex()
        error("logic_gates: object is read-only", 2)
    end

    local KIND_TEXT = {
        [INVALID_BIT] = function(r)
            return sformat("logic_gates: %s must be 0 or 1 (integer), got %s", r.param, r.value)
        end,
        [INVALID_ARITY] = function(r) return sformat("logic_gates: %s", r.value) end,
        [INVALID_WIDTH] = function(r)
            return sformat("logic_gates: %s invalid width: %s", r.param, r.value)
        end,
        [INVALID_SHAPE] = function(r)
            return sformat("logic_gates: %s invalid shape: %s", r.param, r.value)
        end,
        [INTERNAL] = function(r) return sformat("logic_gates: internal error: %s", r.value) end,
    }

    local function new_error(code, param, value)
        local record = { code = code, param = param, value = value }
        local text = KIND_TEXT[code]
        local proxy = setmetatable({}, {
            __index = record,
            __newindex = readonly_newindex,
            __metatable = "logic_gates error",
            __tostring = function() return text(record) end,
        })
        GENUINE[proxy] = record
        return proxy
    end

    function kit.is_kind(err, code)
        if type(err) ~= "table" then return false end
        local record = rawget(GENUINE, err)
        return record ~= nil and record.code == code
    end

    function kit.error_code(err)
        if type(err) ~= "table" then return nil end
        local record = rawget(GENUINE, err)
        return record and record.code or nil
    end

    --- Raise a genuine error of `code` (level 0: the object is the error).
    local function raise(code, param, value)
        error(new_error(code, param, value), 0)
    end
    kit.raise = raise

    --- Describe an arbitrary value with FIXED text, never running its code.
    --
    -- Only `type()` and `math.type()` are consulted (neither reads
    -- metamethods).  Strings are never echoed; floats are not formatted; an
    -- integer is shown by value only when small, else by size.
    local function describe(v)
        local t = type(v)
        if t == "number" then
            if mtype(v) == "integer" then
                if v > -1000000 and v < 1000000 then return sformat("integer %d", v) end
                return "integer (large magnitude)"
            end
            return "float"
        end
        return t
    end
    kit.describe = describe

    --- Validate that a value is a valid binary digit: the INTEGER 0 or 1.
    local function validate_bit(value, name)
        if type(value) == "number" and mtype(value) == "integer"
            and (value == 0 or value == 1) then
            return value
        end
        raise(INVALID_BIT, name, describe(value))
    end
    kit.validate_bit = validate_bit

    -- Strict table snapshots
    -- ----------------------
    --
    -- `#t` and `rawlen(t)` return ANY border of a table with holes, so
    -- `{1, nil, 1}` or `{[1]=1, [3]=1}` would be silently truncated (and which
    -- border is chosen differs between Lua 5.4 and 5.5).  Instead we demand a
    -- plain sequence: a table with NO metatable (so no __index / __len / __eq /
    -- __pairs can run caller code; `getmetatable` returns nil only when there is
    -- truly none -- a faked `__metatable` field makes it non-nil), whose entries
    -- number n <= cap (counted with `next`, stopping at cap + 1), and whose
    -- slots 1..n are all present.  Entry count == n with slots 1..n filled means
    -- the keys are EXACTLY 1..n -- the same verdict on every Lua version.
    -- Values are read with `rawget` into a fresh array.
    function kit.snapshot_list(t, cap, name)
        if type(t) ~= "table" then
            raise(INVALID_SHAPE, name, "must be a table, got " .. describe(t))
        end
        if not rawequal(getmetatable(t), nil) then
            raise(INVALID_SHAPE, name, "must be a plain table without a metatable")
        end
        local n = 0
        local k = next(t)
        while k ~= nil do
            n = n + 1
            if n > cap then
                raise(INVALID_WIDTH, name, sformat("has more than %d entries", cap))
            end
            k = next(t, k)
        end
        local out = {}
        for i = 1, n do
            local v = rawget(t, i)
            if v == nil then
                raise(INVALID_SHAPE, name,
                    "must be a sequence with keys exactly 1..n (no holes or extra keys)")
            end
            out[i] = v
        end
        return out, n
    end

    --- Snapshot a plain list and validate every element as a bit.
    function kit.snapshot_bits(t, cap, name)
        local out, n = kit.snapshot_list(t, cap, name)
        for i = 1, n do
            out[i] = validate_bit(out[i], name .. "[" .. i .. "]")
        end
        return out, n
    end

    --- Read-only view of a module table: reads pass through, writes raise.
    --
    -- `__metatable` is locked.  `rawset` / the debug library still defeat this
    -- (Lua cannot forbid them); that is the host-sandbox boundary.
    function kit.readonly(t)
        return setmetatable({}, {
            __index = t,
            __newindex = readonly_newindex,
            __metatable = "logic_gates module (read-only)",
            __pairs = function() return next, t, nil end,
        })
    end
    return kit
end

return function() return build() end
