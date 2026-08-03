-- Shared build-tool resolution contract tests for Lua rockspec metadata.

-- Add lib/ to the module search path.
package.path = "../lib/?.lua;" .. "../lib/?/init.lua;" .. package.path

local json = require("dkjson")
local lfs = require("lfs")
local Discovery = require("build_tool.discovery")
local Resolver = require("build_tool.resolver")

local fixture_dir = "../../../../specs/fixtures/build-tool-v1/cases"
local temporary_roots = {}

local function read_file(path)
    local file = assert(io.open(path, "rb"))
    local contents = file:read("*a")
    file:close()
    return contents
end

local function write_file(path, contents)
    local file = assert(io.open(path, "wb"))
    file:write(contents)
    file:close()
end

local function ensure_directory(path)
    local normalized = path:gsub("\\", "/")
    local current = ""

    if normalized:match("^[A-Za-z]:/") then
        current = normalized:sub(1, 3)
        normalized = normalized:sub(4)
    elseif normalized:sub(1, 1) == "/" then
        current = "/"
        normalized = normalized:sub(2)
    end

    for segment in normalized:gmatch("[^/]+") do
        if current == "" then
            current = segment
        elseif current == "/" or current:match("^[A-Za-z]:/$") then
            current = current .. segment
        else
            current = current .. "/" .. segment
        end
        lfs.mkdir(current)
    end
end

local function remove_tree(path)
    local attributes = lfs.attributes(path)
    if not attributes then return end
    if attributes.mode ~= "directory" then
        os.remove(path)
        return
    end

    for name in lfs.dir(path) do
        if name ~= "." and name ~= ".." then
            remove_tree(path .. "/" .. name)
        end
    end
    lfs.rmdir(path)
end

local function new_temporary_root()
    local root = os.tmpname()
    os.remove(root)
    ensure_directory(root)
    temporary_roots[#temporary_roots + 1] = root
    return root
end

local base64_values = {}
do
    local alphabet = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/"
    for index = 1, #alphabet do
        base64_values[alphabet:sub(index, index)] = index - 1
    end
end

local function decode_base64(encoded)
    local clean = encoded:gsub("%s", "")
    local decoded = {}

    for offset = 1, #clean, 4 do
        local first = assert(base64_values[clean:sub(offset, offset)])
        local second = assert(base64_values[clean:sub(offset + 1, offset + 1)])
        local third_char = clean:sub(offset + 2, offset + 2)
        local fourth_char = clean:sub(offset + 3, offset + 3)
        local third = third_char == "=" and 0 or assert(base64_values[third_char])
        local fourth = fourth_char == "=" and 0 or assert(base64_values[fourth_char])
        local value = (first << 18) | (second << 12) | (third << 6) | fourth

        decoded[#decoded + 1] = string.char((value >> 16) & 0xff)
        if third_char ~= "=" then
            decoded[#decoded + 1] = string.char((value >> 8) & 0xff)
        end
        if fourth_char ~= "=" then
            decoded[#decoded + 1] = string.char(value & 0xff)
        end
    end

    return table.concat(decoded)
end

local function load_fixture(filename)
    local fixture_text = read_file(fixture_dir .. "/" .. filename)
    local fixture, _, decode_error = json.decode(fixture_text)
    assert.is_nil(decode_error)
    return fixture
end

local function materialize_fixture(filename)
    local fixture = load_fixture(filename)
    local root = new_temporary_root()

    for _, entry in ipairs(fixture.workspace.files) do
        local path = root .. "/" .. entry.path
        local parent = path:gsub("\\", "/"):match("^(.*)/[^/]+$")
        ensure_directory(parent)
        local contents = entry.content_utf8 or decode_base64(entry.content_base64)
        write_file(path, contents)
    end

    return fixture, root
end

local function graph_edges(graph)
    local edges = {}
    for _, source in ipairs(graph:nodes()) do
        for _, target in ipairs(graph:successors(source)) do
            edges[#edges + 1] = {source, target}
        end
    end
    table.sort(edges, function(left, right)
        return table.concat(left, "\0") < table.concat(right, "\0")
    end)
    return edges
end

local function shell_quote(value)
    return '"' .. value:gsub('"', '\\"') .. '"'
end

describe("Lua rockspec UTF-8 resolution contract", function()
    after_each(function()
        for _, root in ipairs(temporary_roots) do
            remove_tree(root)
        end
        temporary_roots = {}
    end)

    it("matches the shared UTF-8 resolution fixture exactly", function()
        local fixture, root = materialize_fixture("resolution-lua-utf8.json")
        local packages = Discovery.discover_packages(root .. "/code")
        local graph = Resolver.resolve_dependencies(packages)

        assert.are.same(fixture.expected.result.edges, graph_edges(graph))
    end)

    it("fails closed with the shared diagnostic for invalid UTF-8", function()
        local fixture, root = materialize_fixture("resolution-lua-invalid-utf8.json")
        local packages = Discovery.discover_packages(root .. "/code")
        local ok, resolution_error = pcall(Resolver.resolve_dependencies, packages)
        local diagnostic = fixture.expected.diagnostics[1]

        assert.is_false(ok)
        assert.is_true(Resolver.is_metadata_encoding_error(resolution_error))
        assert.are.equal(diagnostic.code, resolution_error.code)
        assert.are.equal(diagnostic.package, resolution_error.package)
        assert.are.equal(diagnostic.path, resolution_error.manifest)
        assert.are.equal(diagnostic.details.encoding, resolution_error.encoding)
        assert.are.equal(
            "METADATA_INVALID_UTF8: package=lua/pkg " ..
                "manifest=code/packages/lua/pkg/coding-adventures-pkg-0.1.0-1.rockspec " ..
                "encoding=UTF-8",
            tostring(resolution_error)
        )
        assert.is_nil(tostring(resolution_error):find(root, 1, true))
    end)

    it("accepts a valid literal replacement character", function()
        local _, root = materialize_fixture("resolution-lua-utf8.json")
        local rockspec = root ..
            "/code/packages/lua/pkg/coding-adventures-pkg-0.1.0-1.rockspec"
        local contents = read_file(rockspec)
        write_file(rockspec, contents .. "-- valid literal: \239\191\189\n")

        local packages = Discovery.discover_packages(root .. "/code")
        local graph = Resolver.resolve_dependencies(packages)
        assert.are.same({{"lua/other", "lua/pkg"}}, graph_edges(graph))
    end)

    it("rejects malformed, overlong, surrogate, and out-of-range sequences", function()
        local invalid_sequences = {
            {"overlong two-byte form", string.char(0xc0, 0xaf)},
            {"overlong three-byte form", string.char(0xe0, 0x80, 0x80)},
            {"UTF-16 surrogate", string.char(0xed, 0xa0, 0x80)},
            {"overlong four-byte form", string.char(0xf0, 0x80, 0x80, 0x80)},
            {"code point above U+10FFFF", string.char(0xf4, 0x90, 0x80, 0x80)},
            {"truncated sequence", string.char(0xe2, 0x82)},
            {"non-continuation tail", string.char(0xf1, 0x80, 0x80, 0x41)},
        }

        for _, case in ipairs(invalid_sequences) do
            local root = new_temporary_root()
            local package_dir = root .. "/code/packages/lua/pkg"
            ensure_directory(package_dir)
            write_file(package_dir .. "/BUILD", "echo building\n")
            write_file(
                package_dir .. "/coding-adventures-pkg-0.1.0-1.rockspec",
                "package = \"coding-adventures-pkg\"\n-- " .. case[1] .. ": " .. case[2]
            )

            local packages = Discovery.discover_packages(root .. "/code")
            local ok, resolution_error = pcall(Resolver.resolve_dependencies, packages)
            assert.is_false(ok, case[1] .. " must fail closed")
            assert.is_true(
                Resolver.is_metadata_encoding_error(resolution_error),
                case[1] .. " must return MetadataEncodingError"
            )
        end
    end)

    it("returns exit 2 and only the stable diagnostic from the real CLI", function()
        local _, root = materialize_fixture("resolution-lua-invalid-utf8.json")
        local stdout_path = root .. "/stdout.txt"
        local stderr_path = root .. "/stderr.txt"
        local command = table.concat({
            "lua",
            shell_quote("../build.lua"),
            "--root",
            shell_quote(root),
            "--force",
            "--dry-run",
            "--language",
            "lua",
            "1>" .. shell_quote(stdout_path),
            "2>" .. shell_quote(stderr_path),
        }, " ")

        local success, _, exit_code = os.execute(command)
        if success then exit_code = 0 end

        assert.are.equal(2, exit_code)
        local stderr = read_file(stderr_path)
        local portable_stderr = stderr:gsub("\r\n", "\n")
        assert.are.equal(
            "METADATA_INVALID_UTF8: package=lua/pkg " ..
                "manifest=code/packages/lua/pkg/coding-adventures-pkg-0.1.0-1.rockspec " ..
                "encoding=UTF-8\n",
            portable_stderr
        )
        assert.is_nil(stderr:find(root, 1, true))
    end)
end)
