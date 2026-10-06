-- The portable source-hashing contract is exercised through the production
-- selector, not through a second selector copied into this test.
local here = debug.getinfo(1, "S").source:sub(2):gsub("\\", "/"):match("(.*/)") or "./"
package.path = here .. "../lib/?.lua;" .. here .. "../lib/?/init.lua;"
    .. here .. "../../../../packages/lua/sha256/src/?.lua;"
    .. here .. "../../../../packages/lua/sha256/src/?/init.lua;" .. package.path

local json = require("dkjson")
local sha256 = require("coding_adventures.sha256")
local SourceHashing = require("build_tool.source_hashing")

local fixture_dir = here .. "../../../../specs/fixtures/build-tool-v1/"

local function read_file(path)
    local file = assert(io.open(path, "rb"))
    local bytes = assert(file:read("*a"))
    assert(file:close())
    return bytes
end

local function decode(path)
    local value, _, failure = json.decode(read_file(path))
    assert.is_nil(failure)
    return value
end

local function hex_bytes(hex)
    return (hex:gsub("..", function(pair)
        return string.char(tonumber(pair, 16))
    end))
end

local local_cases = {
    "source-collection-extension.json",
    "source-collection-declared.json",
    "source-collection-registry-roles.json",
    "source-collection-engram-wasm-exact-inputs.json",
    "source-collection-typescript-blog-exact-inputs.json",
    "source-collection-typescript-landing-page-exact-inputs.json",
    "source-collection-typescript-site-foreign-package.json",
}

describe("portable Lua source hashing", function()
    it("packages the complete checked language registry", function()
        local fixture = decode(fixture_dir .. "language-source-input-registry.json")
        assert.same(fixture, SourceHashing.registry())
        assert.equals(
            "5201a045ea3e2086fd9be316f2692743ca329f1d84f1c0983a0da47e96b3f621",
            SourceHashing.registry_digest()
        )
    end)

    it("keeps production selectors isolated from caller-owned tables", function()
        local mutable = SourceHashing.registry()
        local original = mutable.languages[1].recursive_suffixes[1]
        mutable.languages[1].recursive_suffixes[1] = ".unreviewed"
        assert.equals(original, SourceHashing.registry().languages[1].recursive_suffixes[1])
        local exported = require("build_tool.source_registry_data")
        local saved = exported.data.languages[1].recursive_suffixes[1]
        exported.data.languages[1].recursive_suffixes[1] = ".unreviewed"
        assert.equals(saved, SourceHashing.registry().languages[1].recursive_suffixes[1])
        exported.data.languages[1].recursive_suffixes[1] = saved
    end)

    for _, filename in ipairs(local_cases) do
        it("consumes the neutral package-local case " .. filename, function()
            local case = decode(fixture_dir .. "cases/" .. filename)
            local expected = case.expected
            local options = case.input.options
            if expected.outcome == "ok" then
                assert.same(expected.result.files, SourceHashing.collect_source_files(options))
            else
                assert.has_error(function()
                    SourceHashing.collect_source_files(options)
                end)
            end
        end)
    end

    it("rejects an unknown language before examining candidates", function()
        assert.has_error(function()
            SourceHashing.collect_source_files({
                language = "unknown-lane", package_root = "code/packages/lua/example",
                mode = "extension", registry_sha256 = SourceHashing.registry_digest(),
                declared_srcs = {}, candidates = {},
            })
        end)
    end)

    it("prunes generated directories but not same-named regular files", function()
        local files = SourceHashing.collect_source_files({
            language = "lua", package_root = "code/packages/lua/example",
            mode = "declared_sources", registry_sha256 = SourceHashing.registry_digest(),
            declared_srcs = {"build", "**/*.lua"},
            candidates = {
                {path = "src/build/generated.lua", kind = "file", content_hex = "78"},
                {path = "src/Build/retained.lua", kind = "file", content_hex = "79"},
                {path = "build", kind = "file", content_hex = "7a"},
            },
        })
        assert.same({"build", "src/Build/retained.lua"}, {files[1].path, files[2].path})
    end)

    it("prevalidates all declared globs and rejects ambiguous classes", function()
        for _, invalid in ipairs({"[z-a].lua", "[a--b].lua", "[a&&b].lua"}) do
            assert.has_error(function()
                SourceHashing.collect_source_files({
                    language = "lua", package_root = "code/packages/lua/example",
                    mode = "declared_sources", registry_sha256 = SourceHashing.registry_digest(),
                    declared_srcs = {"**/*.lua", invalid}, candidates = {},
                })
            end)
        end
    end)

    it("matches portable declared character classes", function()
        local files = SourceHashing.collect_source_files({
            language = "lua", package_root = "code/packages/lua/example",
            mode = "declared_sources", registry_sha256 = SourceHashing.registry_digest(),
            declared_srcs = {"src/[a-c].lua", "src/[!ab].lua"},
            candidates = {
                {path = "src/a.lua", kind = "file", content_hex = "61"},
                {path = "src/c.lua", kind = "file", content_hex = "63"},
                {path = "src/d.lua", kind = "file", content_hex = "64"},
            },
        })
        assert.same({"src/a.lua", "src/c.lua", "src/d.lua"},
            {files[1].path, files[2].path, files[3].path})
    end)

    it("bounds cumulative declared-match work before the fifth large match", function()
        local patterns = {}
        for index = 1, 5 do
            patterns[index] = string.rep("**/", 1000) .. index
        end
        assert.has_error(function()
            SourceHashing.collect_source_files({
                language = "lua", package_root = "code/packages/lua/example",
                mode = "declared_sources", registry_sha256 = SourceHashing.registry_digest(),
                declared_srcs = patterns,
                candidates = {{path = string.rep("a", 4000),
                    kind = "file", content_hex = "78"}},
            })
        end)
    end)

    it("rejects platform aliases and file-prefix collisions", function()
        local options = {
            language = "lua", package_root = "code/packages/lua/example",
            mode = "extension", registry_sha256 = SourceHashing.registry_digest(),
            declared_srcs = {}, candidates = {
                {path = "src/A.lua", kind = "file", content_hex = "61"},
                {path = "src/a.lua", kind = "file", content_hex = "62"},
            },
        }
        assert.has_error(function() SourceHashing.collect_source_files(options) end)
        options.candidates[2].path = "src/A.lua/child.lua"
        assert.has_error(function() SourceHashing.collect_source_files(options) end)
    end)

    it("hashes an inert local and shared-input union with v1 length frames", function()
        local case = decode(fixture_dir .. "cases/hashing-cache-local-boundary-union.json")
        local content = {}
        for _, entry in ipairs(case.workspace.files) do
            content[entry.path] = entry.content_utf8 or hex_bytes(entry.content_hex)
        end
        assert.equals(
            case.expected.result.package_digest,
            SourceHashing.package_digest(case.input.options.include_paths, content)
        )
    end)

    it("distinguishes raw bytes, path renames, and input ordering", function()
        local path = "code/packages/lua/example/src/main.lua"
        local initial = SourceHashing.package_digest({path}, {[path] = "a\0b"})
        assert.not_equals(initial, SourceHashing.package_digest({path}, {[path] = "a\0c"}))
        assert.not_equals(initial, SourceHashing.package_digest({path .. "x"}, {[path .. "x"] = "a\0b"}))
        local second = "code/packages/lua/example/BUILD"
        assert.equals(
            SourceHashing.package_digest({path, second}, {[path] = "a\0b", [second] = "build\n"}),
            SourceHashing.package_digest({second, path, path}, {[path] = "a\0b", [second] = "build\n"})
        )
        assert.not_equals(sha256.sha256_hex(""), initial)
    end)
end)
