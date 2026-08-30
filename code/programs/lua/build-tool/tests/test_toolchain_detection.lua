package.path = "../lib/?.lua;../lib/?/init.lua;" .. package.path

local json = require("dkjson")
local lfs = require("lfs")
local ToolchainDetection = require("build_tool.toolchain_detection")

local fixture_root = "../../../../specs/fixtures/build-tool-v1/cases"
local expected_fixtures = {
    "toolchain-detection-affected-only.json",
    "toolchain-detection-crlf-grammar.json",
    "toolchain-detection-declarations.json",
    "toolchain-detection-empty.json",
    "toolchain-detection-force-full.json",
    "toolchain-detection-null-all.json",
    "toolchain-detection-platform-darwin.json",
    "toolchain-detection-platform-linux.json",
    "toolchain-detection-platform-windows.json",
    "toolchain-detection-shared.json",
    "toolchain-detection-unsupported.json",
}

local function read_file(pathname)
    local file = assert(io.open(pathname, "rb"))
    local contents = file:read("*a")
    file:close()
    return contents
end

local function load_fixture(filename)
    local fixture, _, decode_error = json.decode(read_file(fixture_root .. "/" .. filename))
    assert.is_nil(decode_error)
    return fixture
end

local function discovered_fixtures()
    local filenames = {}
    for filename in lfs.dir(fixture_root) do
        if filename:match("^toolchain%-detection%-.+%.json$") then
            filenames[#filenames + 1] = filename
        end
    end
    table.sort(filenames)
    return filenames
end

describe("ToolchainDetection", function()
    it("independently consumes every neutral toolchain-detection fixture", function()
        local fixtures = discovered_fixtures()
        assert.are.same(expected_fixtures, fixtures)

        for _, filename in ipairs(fixtures) do
            local fixture = load_fixture(filename)
            local options = fixture.input.options
            local expected = fixture.expected
            local actual = ToolchainDetection.evaluate_snapshot(
                options.platform,
                options.force_full,
                options.packages,
                options.scheduled_packages,
                options.forced_toolchains
            )

            assert.are.equal(expected.outcome, actual.outcome, fixture.id)
            assert.are.same(expected.result.toolchains or {}, actual.toolchains, fixture.id)
            assert.are.same(expected.diagnostics, actual.diagnostics, fixture.id)
        end
    end)

    it("rejects per-file byte, line, and aggregate snapshot overruns", function()
        local base = {name = "rust/app", language = "rust"}

        assert.has_error(function()
            ToolchainDetection.evaluate_snapshot("linux", false, {
                {name = base.name, language = base.language, build_files = {BUILD = string.rep("x", 65537)}},
            }, nil, {})
        end, "toolchain BUILD snapshot exceeds its per-file resource ceiling")

        assert.has_error(function()
            ToolchainDetection.evaluate_snapshot("linux", false, {
                {name = base.name, language = base.language, build_files = {BUILD = string.rep("\n", 4096)}},
            }, nil, {})
        end, "toolchain BUILD snapshot exceeds its per-file resource ceiling")

        local build_files = {}
        for index = 0, 16 do
            build_files["BUILD_" .. index] = string.rep("x", 65536)
        end
        assert.has_error(function()
            ToolchainDetection.evaluate_snapshot("linux", false, {
                {name = base.name, language = base.language, build_files = build_files},
            }, nil, {})
        end, "toolchain BUILD snapshot exceeds its aggregate resource ceiling")
    end)

    it("keeps declaration grammar byte-exact across CRLF and lone CR", function()
        assert.are.same(
            {"python", "java"},
            ToolchainDetection.parse_extra_toolchains(
                "  # needs-toolchain: python  \r\n\t# needs-toolchain:\tjava\t\r\n"
            )
        )
        assert.are.same({}, ToolchainDetection.parse_extra_toolchains("# needs-toolchain: python\r"))
        assert.are.same({}, ToolchainDetection.parse_extra_toolchains("# needs-toolchain: lua\r  "))
        assert.are.same({}, ToolchainDetection.parse_extra_toolchains("# needs-toolchain: swift\r\r\n"))
    end)

    it("returns a fresh canonical registry table", function()
        local first = ToolchainDetection.canonical_toolchains()
        first[1] = "changed"
        assert.are.equal("cpp", ToolchainDetection.canonical_toolchains()[1])
    end)
end)
