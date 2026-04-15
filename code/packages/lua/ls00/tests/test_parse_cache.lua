-- test_parse_cache.lua — ParseCache tests
-- ========================================
--
-- The ParseCache avoids re-parsing unchanged documents. It uses (uri, version)
-- as the cache key. These tests verify:
--
--   1. Cache miss triggers a parse
--   2. Cache hit returns the same result (same reference)
--   3. New version causes a cache miss
--   4. Eviction removes cached entries
--   5. Diagnostics are correctly populated from the bridge

local ls00 = require("coding_adventures.ls00")

-- A mock bridge for testing. Returns diagnostics when source contains "ERROR".
local function make_mock_bridge()
    local parse_count = 0
    return {
        tokenize = function(source)
            return {}, nil
        end,
        parse = function(source)
            parse_count = parse_count + 1
            local diags = {}
            if source:find("ERROR") then
                diags[1] = ls00.Diagnostic(
                    ls00.Range(ls00.Position(0, 0), ls00.Position(0, 5)),
                    ls00.SEVERITY_ERROR,
                    "syntax error: unexpected ERROR token"
                )
            end
            return source, diags, nil
        end,
        get_parse_count = function()
            return parse_count
        end,
    }
end

describe("ParseCache", function()
    it("returns a result on cache miss", function()
        local bridge = make_mock_bridge()
        local cache = ls00.ParseCache:new()

        local r1 = cache:get_or_parse("file:///a.txt", 1, "hello", bridge)
        assert.is_not_nil(r1)
        assert.are.equal("hello", r1.ast)
    end)

    it("returns the same result on cache hit", function()
        local bridge = make_mock_bridge()
        local cache = ls00.ParseCache:new()

        local r1 = cache:get_or_parse("file:///a.txt", 1, "hello", bridge)
        local r2 = cache:get_or_parse("file:///a.txt", 1, "hello", bridge)

        -- Same table reference means the cache was used.
        assert.are.equal(r1, r2)
        -- Parse was called only once.
        assert.are.equal(1, bridge.get_parse_count())
    end)

    it("misses on new version", function()
        local bridge = make_mock_bridge()
        local cache = ls00.ParseCache:new()

        local r1 = cache:get_or_parse("file:///a.txt", 1, "hello", bridge)
        local r2 = cache:get_or_parse("file:///a.txt", 2, "hello world", bridge)

        -- Different version = different result.
        assert.are_not.equal(r1, r2)
        -- Parse was called twice.
        assert.are.equal(2, bridge.get_parse_count())
    end)

    it("evicts cached entries", function()
        local bridge = make_mock_bridge()
        local cache = ls00.ParseCache:new()

        local r1 = cache:get_or_parse("file:///a.txt", 1, "hello", bridge)
        cache:evict("file:///a.txt")

        -- After eviction, same (uri, version) produces a new parse.
        local r2 = cache:get_or_parse("file:///a.txt", 1, "hello", bridge)
        assert.are_not.equal(r1, r2)
    end)

    it("populates diagnostics for error source", function()
        local bridge = make_mock_bridge()
        local cache = ls00.ParseCache:new()

        local result = cache:get_or_parse("file:///a.txt", 1, "source with ERROR token", bridge)
        assert.is_true(#result.diagnostics > 0)
        assert.are.equal("syntax error: unexpected ERROR token", result.diagnostics[1].message)
    end)

    it("returns empty diagnostics for clean source", function()
        local bridge = make_mock_bridge()
        local cache = ls00.ParseCache:new()

        local result = cache:get_or_parse("file:///clean.txt", 1, "hello world", bridge)
        assert.are.equal(0, #result.diagnostics)
    end)
end)
