-- Native conformance for the process-free graph and diff-selection core.
-- Fixture discovery and JSON decoding deliberately stay outside production.
local here = debug.getinfo(1, "S").source:sub(2):gsub("\\", "/"):match("(.*/)") or "./"
package.path = here .. "../lib/?.lua;" .. here .. "../lib/?/init.lua;"
    .. here .. "../../../../packages/lua/sha256/src/?.lua;"
    .. here .. "../../../../packages/lua/sha256/src/?/init.lua;" .. package.path

local json = require("dkjson")
local lfs = require("lfs")
local GraphDiff = require("build_tool.graph_diff")

local fixture_root = here .. "../../../../specs/fixtures/build-tool-v1/"
local case_root = fixture_root .. "cases/"

local GRAPH_IDS = {
    "graph/canonical-edge-order",
    "graph/chain",
    "graph/cycle",
    "graph/diamond",
    "graph/empty",
    "graph/isolated",
    "graph/multiple-components",
    "graph/partial-cycle-no-output",
}

local DIFF_IDS = {
    "diff-selection/exact-build-fronts",
    "diff-selection/forced-package",
    "diff-selection/known-unmatched-near-build",
    "diff-selection/match-work-at-limit",
    "diff-selection/match-work-over-limit",
    "diff-selection/package-prefix",
    "diff-selection/repository-boundary-reverse-index",
    "diff-selection/shared-input-multiconsumer",
    "diff-selection/strict-glob-character-classes",
    "diff-selection/transitive-package-change",
    "diff-selection/unknown-path-all",
    "diff-selection/unknown-path-error",
}

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

local function fixtures(prefix)
    local values = {}
    for filename in lfs.dir(case_root) do
        if filename:match("^" .. prefix .. "%-.+%.json$") then
            values[#values + 1] = decode(case_root .. filename)
        end
    end
    table.sort(values, function(left, right) return left.id < right.id end)
    return values
end

local function ids(values)
    local result = {}
    for index, value in ipairs(values) do result[index] = value.id end
    return result
end

local function diff_input(fixture, boundary)
    local options = fixture.input.options
    return {
        packages = options.packages,
        edges = options.edges,
        forced_packages = options.forced_packages,
        unknown_path_policy = options.unknown_path_policy,
        changed_paths = fixture.input.changed_paths,
        boundary_sha256 = options.boundary_sha256 or "",
        boundary = options.boundary_sha256 and boundary or nil,
    }
end

local function assert_fixture(fixture, actual)
    if actual.ok then
        assert.equals("ok", fixture.expected.outcome, fixture.id)
        assert.same({}, fixture.expected.diagnostics, fixture.id)
        assert.same(fixture.expected.result, actual.value, fixture.id)
    else
        assert.equals("error", fixture.expected.outcome, fixture.id)
        assert.same({}, fixture.expected.result, fixture.id)
        assert.same({fixture.expected.diagnostics[1].code}, {actual.code}, fixture.id)
    end
end

local function package_prefix(name, rel_path)
    return {name = name, rel_path = rel_path, source_mode = "package_prefix",
        source_globs = {}}
end

local function simple_diff(packages)
    return {packages = packages, edges = {}, forced_packages = {},
        unknown_path_policy = "error", changed_paths = {}, boundary_sha256 = ""}
end

describe("process-free Lua graph and diff selection", function()
    it("dynamically consumes the exact eight graph fixtures", function()
        local documents = fixtures("graph")
        assert.same(GRAPH_IDS, ids(documents))
        for _, document in ipairs(documents) do
            local options = document.input.options
            assert_fixture(document, GraphDiff.evaluate_graph({
                packages = options.packages,
                edges = options.edges,
            }))
        end
    end)

    it("dynamically consumes the exact twelve diff-selection fixtures", function()
        local documents = fixtures("diff%-selection")
        assert.same(DIFF_IDS, ids(documents))
        local boundary = decode(fixture_root .. "repository-source-input-boundary.json")
        for _, document in ipairs(documents) do
            assert_fixture(document,
                GraphDiff.evaluate_diff_selection(diff_input(document, boundary)))
        end
    end)

    it("fans one shared repository input out to both registered consumers", function()
        local boundary = decode(fixture_root .. "repository-source-input-boundary.json")
        local document
        for _, candidate in ipairs(fixtures("diff%-selection")) do
            if candidate.id == "diff-selection/shared-input-multiconsumer" then
                document = candidate
            end
        end
        assert.is_not_nil(document)
        local actual = GraphDiff.evaluate_diff_selection(diff_input(document, boundary))
        assert.same({ok = true, value = {
            changed_packages = {"swift/conduit", "swift/sha256-native"},
            affected_packages = {"swift/app", "swift/conduit", "swift/sha256-native"},
            prerequisite_packages = {"swift/base"},
        }}, actual)
    end)

    it("returns stable graph failures without partial levels", function()
        assert.same({ok = false, code = "GRAPH_CYCLE"}, GraphDiff.evaluate_graph({
            packages = {"fixture/a", "fixture/b", "fixture/free"},
            edges = {{"fixture/a", "fixture/b"}, {"fixture/b", "fixture/a"}},
        }))
        assert.same({ok = false, code = "GRAPH_PACKAGE_DUPLICATE"},
            GraphDiff.evaluate_graph({packages = {"fixture/a", "fixture/a"}, edges = {}}))
        assert.same({ok = false, code = "GRAPH_EDGE_DUPLICATE"},
            GraphDiff.evaluate_graph({packages = {"fixture/a", "fixture/b"},
                edges = {{"fixture/a", "fixture/b"}, {"fixture/a", "fixture/b"}}}))
        assert.same({ok = false, code = "GRAPH_EDGE_SELF"},
            GraphDiff.evaluate_graph({packages = {"fixture/a"},
                edges = {{"fixture/a", "fixture/a"}}}))
    end)

    it("rejects non-adjacent root ancestry and Unicode fold collisions", function()
        assert.same({ok = false, code = "DIFF_PATH_INVALID"},
            GraphDiff.evaluate_diff_selection(simple_diff({
                package_prefix("fixture/root", "a"),
                package_prefix("fixture/punctuation", "a!"),
                package_prefix("fixture/descendant", "a/x"),
            })))
        assert.same({ok = false, code = "DIFF_PATH_INVALID"},
            GraphDiff.evaluate_diff_selection(simple_diff({
                package_prefix("fixture/eszett", "code/ß"),
                package_prefix("fixture/double-s", "code/ss"),
            })))
    end)

    it("rejects shared and cyclic decoded boundary graphs before digesting", function()
        local leaf = {payload = string.rep("x", 32)}
        local shared = leaf
        for _ = 1, 12 do shared = {shared, shared} end
        local aliased = {schema_version = 1,
            language_source_input_registry_sha256 = string.rep("0", 64),
            boundaries = shared}
        local cyclic = {}
        cyclic.self = cyclic
        local base = simple_diff({package_prefix("fixture/a", "code/a")})
        base.boundary_sha256 = string.rep("0", 64)
        base.boundary = aliased
        assert.same({ok = false, code = "DIFF_BOUNDARY_INVALID"},
            GraphDiff.evaluate_diff_selection(base))
        base.boundary = cyclic
        assert.same({ok = false, code = "DIFF_BOUNDARY_INVALID"},
            GraphDiff.evaluate_diff_selection(base))
    end)

    it("does not expose forced or local selection on an unknown path", function()
        assert.same({ok = false, code = "DIFF_UNKNOWN_PATH"},
            GraphDiff.evaluate_diff_selection({
                packages = {package_prefix("fixture/a", "code/a"),
                    package_prefix("fixture/b", "code/b")},
                edges = {{"fixture/a", "fixture/b"}},
                forced_packages = {"fixture/a"},
                unknown_path_policy = "error",
                changed_paths = {"code/b/file.ts", "docs/unknown.md"},
                boundary_sha256 = "",
            }))
    end)

    it("matches question mark as exactly one Unicode scalar", function()
        assert.same({ok = true, value = {
            changed_packages = {"fixture/question"},
            affected_packages = {"fixture/question"},
            prerequisite_packages = {},
        }}, GraphDiff.evaluate_diff_selection({
            packages = {{name = "fixture/question", rel_path = "code/question",
                source_mode = "strict_globs", source_globs = {"src/?.ts"}}},
            edges = {}, forced_packages = {}, unknown_path_policy = "error",
            changed_paths = {"code/question/src/😀.ts"}, boundary_sha256 = "",
        }))
    end)

    it("rejects hostile table shapes and complete invalid glob lists", function()
        local sparse = {[2] = "fixture/a"}
        assert.same({ok = false, code = "GRAPH_INPUT_INVALID"},
            GraphDiff.evaluate_graph({packages = sparse, edges = {}}))
        local base = simple_diff({package_prefix("fixture/a", "code/a")})
        base.packages[1].source_mode = "strict_globs"
        base.packages[1].source_globs = {"src/**/*.lua", "[z-a].lua"}
        assert.same({ok = false, code = "DIFF_GLOB_INVALID"},
            GraphDiff.evaluate_diff_selection(base))
        setmetatable(base, {__index = {changed_paths = {"code/a/injected.lua"}}})
        assert.same({ok = false, code = "DIFF_INPUT_INVALID"},
            GraphDiff.evaluate_diff_selection(base))
    end)
end)
