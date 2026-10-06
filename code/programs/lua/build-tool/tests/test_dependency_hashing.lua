-- The neutral hashing-cache cases exercise the production, process-free Lua
-- hasher with caller-supplied bytes and cache records.
local here = debug.getinfo(1, "S").source:sub(2):gsub("\\", "/"):match("(.*/)") or "./"
package.path = here .. "../lib/?.lua;" .. here .. "../lib/?/init.lua;"
    .. here .. "../../../../packages/lua/sha256/src/?.lua;"
    .. here .. "../../../../packages/lua/sha256/src/?/init.lua;" .. package.path

local json = require("dkjson")
local sha256 = require("coding_adventures.sha256")
local SourceHashing = require("build_tool.source_hashing")
local fixture_dir = here .. "../../../../specs/fixtures/build-tool-v1/cases/"

local cases = {
    "hashing-cache-corrupt.json",
    "hashing-cache-dependency-change-after.json",
    "hashing-cache-dependency-order-before.json",
    "hashing-cache-failed-prior-record.json",
    "hashing-cache-hit.json",
    "hashing-cache-local-boundary-union.json",
    "hashing-cache-missing.json",
    "hashing-cache-shared-input-conduit-after.json",
    "hashing-cache-shared-input-conduit-before.json",
    "hashing-cache-shared-input-sha256-native-after.json",
    "hashing-cache-shared-input-sha256-native-before.json",
}

local function read_case(filename)
    local file = assert(io.open(fixture_dir .. filename, "rb"))
    local bytes = assert(file:read("*a"))
    assert(file:close())
    local value, _, failure = json.decode(bytes)
    assert.is_nil(failure)
    return value
end

local function hex_bytes(hex)
    return (hex:gsub("..", function(pair) return string.char(tonumber(pair, 16)) end))
end

describe("portable Lua dependency hashing and inert cache decisions", function()
    it("pins the complete neutral hashing-cache case roster", function()
        assert.equals(11, #cases)
    end)

    for _, filename in ipairs(cases) do
        it("consumes the neutral case " .. filename, function()
            local case = read_case(filename)
            local contents = {}
            for _, file in ipairs(case.workspace.files) do
                contents[file.path] = file.content_utf8 or hex_bytes(file.content_hex)
            end
            local actual = SourceHashing.evaluate_hashing_cache(case.input.options, contents)
            assert.same(case.expected.result, actual.result)
            assert.same(case.expected.diagnostics, actual.diagnostics)
        end)
    end

    it("frames sorted dependency names and decoded digests", function()
        local alpha = {package = "lua/alpha", digest = string.rep("11", 32)}
        local zeta = {package = "lua/zeta", digest = string.rep("bb", 32)}
        local frame = string.pack(">I8", #alpha.package) .. alpha.package
            .. string.pack(">I8", 32) .. hex_bytes(alpha.digest)
            .. string.pack(">I8", #zeta.package) .. zeta.package
            .. string.pack(">I8", 32) .. hex_bytes(zeta.digest)
        local expected = sha256.sha256_hex(frame)
        assert.equals(expected, SourceHashing.dependencies_digest({zeta, alpha}))
        assert.equals(expected, SourceHashing.dependencies_digest({alpha, zeta}))
        assert.equals(sha256.sha256_hex(""), SourceHashing.dependencies_digest({}))
    end)

    it("combines decoded 32-byte digests, not their hex text", function()
        local package_digest = string.rep("11", 32)
        local dependencies_digest = string.rep("bb", 32)
        assert.equals(sha256.sha256_hex(hex_bytes(package_digest)
            .. hex_bytes(dependencies_digest)),
            SourceHashing.combined_digest(package_digest, dependencies_digest))
    end)

    it("rejects malformed or duplicate dependency identities and digests", function()
        local valid = {package = "lua/base", digest = string.rep("11", 32)}
        for _, dependencies in ipairs({
            {{package = "../escape", digest = valid.digest}},
            {{package = "lua/base", digest = "not-a-digest"}},
            {valid, valid},
            {[2] = valid},
        }) do
            assert.has_error(function() SourceHashing.dependencies_digest(dependencies) end)
        end
        assert.has_error(function()
            SourceHashing.combined_digest(string.rep("11", 32), "invalid")
        end)
        assert.has_error(function()
            SourceHashing.dependencies_digest({{package = "lua/" .. string.rep("a", 240),
                digest = valid.digest}})
        end)
        assert.has_error(function()
            SourceHashing.dependencies_digest({{package = "lua/" .. string.rep("\204\129", 200),
                digest = valid.digest}})
        end)
        local oversized = {}
        for index = 1, 4097 do
            oversized[index] = {package = "lua/item" .. index, digest = valid.digest}
        end
        assert.has_error(function() SourceHashing.dependencies_digest(oversized) end)
    end)

    it("keeps prior-cache decisions inert and invalidation sorted", function()
        local options = {
            algorithm = "sha256-v1", package = "lua/demo", include_paths = {},
            dependency_digests = {}, dependents = {"lua/zeta", "lua/alpha", "lua/zeta"},
            prior_cache = {state = "missing"},
        }
        local first = SourceHashing.evaluate_hashing_cache(options, {})
        assert.equals("miss", first.result.cache_status)
        assert.same({"lua/alpha", "lua/demo", "lua/zeta"},
            first.result.invalidated_packages)
        options.prior_cache = {state = "record", status = "success",
            combined_digest = first.result.combined_digest}
        local hit = SourceHashing.evaluate_hashing_cache(options, {})
        assert.equals("hit", hit.result.cache_status)
        assert.same({}, hit.result.invalidated_packages)
        options.prior_cache = {state = "corrupt"}
        local recovered = SourceHashing.evaluate_hashing_cache(options, {})
        assert.equals("recovered", recovered.result.cache_status)
        assert.same({{code = "CACHE_CORRUPT_RECOVERED", severity = "warning",
            package = "lua/demo"}}, recovered.diagnostics)
    end)
end)
