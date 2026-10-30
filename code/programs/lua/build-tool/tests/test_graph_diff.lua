-- Native conformance for the process-free graph and diff-selection core.
-- Fixture discovery and JSON decoding deliberately stay outside production.
local here = debug.getinfo(1, "S").source:sub(2):gsub("\\", "/"):match("(.*/)") or "./"
package.path = here .. "../lib/?.lua;" .. here .. "../lib/?/init.lua;"
    .. here .. "../../../../packages/lua/sha256/src/?.lua;"
    .. here .. "../../../../packages/lua/sha256/src/?/init.lua;" .. package.path

local json = require("dkjson")
local lfs = require("lfs")
local sha256 = require("coding_adventures.sha256")
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
    local packages = {}
    for index, value in ipairs(options.packages) do
        packages[index] = {
            name = value.name,
            rel_path = value.rel_path,
            source_mode = value.source_mode,
            source_globs = value.source_globs or {},
        }
    end
    return {
        packages = packages,
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

local function json_array(values)
    return setmetatable(values or {}, {__jsontype = "array"})
end

local function canonical_json(value)
    local kind = type(value)
    if kind == "string" then return json.encode(value) end
    if kind == "boolean" then return value and "true" or "false" end
    if kind == "number" then return tostring(value) end
    if kind == "nil" then return "null" end
    local mt = getmetatable(value)
    if type(mt) == "table" and rawget(mt, "__jsontype") == "array" then
        local elements = {}
        for index = 1, #value do elements[index] = canonical_json(value[index]) end
        return "[" .. table.concat(elements, ",") .. "]"
    end
    local keys = {}
    for key in pairs(value) do keys[#keys + 1] = key end
    table.sort(keys)
    local fields = {}
    for _, key in ipairs(keys) do
        fields[#fields + 1] = json.encode(key) .. ":" .. canonical_json(value[key])
    end
    return "{" .. table.concat(fields, ",") .. "}"
end

local function boundary_digest(value)
    local encoded = canonical_json(value)
    local frame = "coding-adventures/build-tool-repository-source-input-boundary/v1\0"
        .. string.pack(">I8", #encoded) .. encoded
    return sha256.sha256_hex(frame)
end

local function boundary_document(boundaries)
    return {
        schema_version = 1,
        language_source_input_registry_sha256 = string.rep("0", 64),
        boundaries = json_array(boundaries),
    }
end

local function retained_kibibytes(operation)
    collectgarbage("collect")
    local before = collectgarbage("count")
    local result = operation()
    local retained = collectgarbage("count") - before
    collectgarbage("collect")
    return result, retained
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

    it("treats a package root as zero path components", function()
        local function evaluate(glob)
            return GraphDiff.evaluate_diff_selection({
                packages = {{name = "fixture/root", rel_path = "code/root",
                    source_mode = "strict_globs", source_globs = {glob}}},
                edges = {}, forced_packages = {}, unknown_path_policy = "error",
                changed_paths = {"code/root"}, boundary_sha256 = "",
            })
        end

        for _, glob in ipairs({"*", "?"}) do
            assert.same({ok = true, value = {changed_packages = {},
                affected_packages = {}, prerequisite_packages = {}}}, evaluate(glob))
        end
        assert.same({ok = true, value = {changed_packages = {"fixture/root"},
            affected_packages = {"fixture/root"}, prerequisite_packages = {}}},
            evaluate("**"))
    end)

    it("compiles a maximum-length unmatched bracket run without suffix rescans", function()
        local maximum = string.rep("[", 512)
        assert.same({ok = true, value = {changed_packages = {}, affected_packages = {},
            prerequisite_packages = {}}}, GraphDiff.evaluate_diff_selection({
                packages = {{name = "fixture/maximum", rel_path = "maximum",
                    source_mode = "strict_globs", source_globs = {maximum}}},
                edges = {}, forced_packages = {}, unknown_path_policy = "error",
                changed_paths = {"maximum"}, boundary_sha256 = "",
            }))

        local literal = string.rep("[", 500)
        assert.same({ok = true, value = {changed_packages = {"fixture/brackets"},
            affected_packages = {"fixture/brackets"}, prerequisite_packages = {}}},
            GraphDiff.evaluate_diff_selection({
                packages = {{name = "fixture/brackets", rel_path = "p",
                    source_mode = "strict_globs", source_globs = {literal}}},
                edges = {}, forced_packages = {}, unknown_path_policy = "error",
                changed_paths = {"p/" .. literal}, boundary_sha256 = "",
            }))
    end)

    it("validates maximum-scale many-segment roots without per-segment nodes", function()
        local packages = {}
        for index = 1, 4096 do
            local segments = {}
            for bit = 11, 0, -1 do
                segments[#segments + 1] = ((index - 1) // (1 << bit)) % 2 == 0
                    and "a" or "b"
            end
            for _ = 13, 256 do segments[#segments + 1] = "a" end
            packages[index] = package_prefix(
                string.format("fixture/p%04d", index),
                table.concat(segments, "/"))
        end
        local actual, retained = retained_kibibytes(function()
            return GraphDiff.evaluate_diff_selection(simple_diff(packages))
        end)
        assert.same({ok = true, value = {changed_packages = {}, affected_packages = {},
            prerequisite_packages = {}}}, actual)
        assert.is_true(retained < 32768,
            string.format("root validation retained %.1f MiB", retained / 1024))

        packages[1].rel_path = "code/shared/child"
        packages[4096].rel_path = "code/shared"
        assert.same({ok = false, code = "DIFF_PATH_INVALID"},
            GraphDiff.evaluate_diff_selection(simple_diff(packages)))
    end)

    it("preserves domain errors after inert array snapshotting", function()
        local oversized = {}
        for index = 1, 4098 do oversized[index] = string.format("fixture/p%04d", index) end
        assert.same({ok = false, code = "GRAPH_PACKAGE_LIMIT_EXCEEDED"},
            GraphDiff.evaluate_graph({packages = oversized, edges = {}}))
        assert.same({ok = false, code = "GRAPH_EDGE_UNKNOWN"},
            GraphDiff.evaluate_graph({packages = {"fixture/a", "fixture/b"},
                edges = {{"fixture/a", "fixture/b", "fixture/a", "fixture/b"}}}))

        local oversized_diff = {}
        for index = 1, 4098 do
            oversized_diff[index] = package_prefix(
                string.format("fixture/p%04d", index), string.format("code/p%04d", index))
        end
        assert.same({ok = false, code = "DIFF_PACKAGE_INVALID"},
            GraphDiff.evaluate_diff_selection(simple_diff(oversized_diff)))

        local missing = {name = "fixture/a", rel_path = "code/a",
            source_mode = "package_prefix"}
        assert.same({ok = false, code = "DIFF_INPUT_INVALID"},
            GraphDiff.evaluate_diff_selection(simple_diff({missing})))
        missing.source_globs = false
        assert.same({ok = false, code = "DIFF_INPUT_INVALID"},
            GraphDiff.evaluate_diff_selection(simple_diff({missing})))
    end)

    it("validates many maximum glob lists without retaining compiled tokens", function()
        local globs = {}
        for index = 1, 256 do
            globs[index] = string.format("%03d", index) .. string.rep("a", 509)
        end
        local packages = {}
        for index = 1, 50 do
            packages[index] = {name = string.format("fixture/p%d", index),
                rel_path = string.format("code/p%d", index), source_mode = "strict_globs",
                source_globs = globs}
        end
        local started = os.clock()
        local actual, retained = retained_kibibytes(function()
            return GraphDiff.evaluate_diff_selection(simple_diff(packages))
        end)
        local elapsed = os.clock() - started
        assert.same({ok = true, value = {changed_packages = {}, affected_packages = {},
            prerequisite_packages = {}}}, actual)
        assert.is_true(retained < 24576,
            string.format("glob validation retained %.1f MiB", retained / 1024))
        assert.is_true(elapsed < 2,
            string.format("shared glob validation took %.2f seconds", elapsed))

        packages[50].source_globs = {globs[1], globs[1]}
        assert.same({ok = false, code = "DIFF_GLOB_INVALID"},
            GraphDiff.evaluate_diff_selection(simple_diff(packages)))
    end)

    it("charges exact unique glob bytes at and above the aggregate limit", function()
        local packages, total_bytes = {}, 0
        local serial = 0
        local repeated = utf8.char(0x10000)
        for package_index = 1, 8 do
            local globs = {}
            for glob_index = 1, 256 do
                serial = serial + 1
                globs[glob_index] = utf8.char(0x10000 + serial)
                    .. string.rep(repeated, 511)
                total_bytes = total_bytes + #globs[glob_index]
            end
            packages[package_index] = {
                name = string.format("fixture/unique%d", package_index),
                rel_path = string.format("code/unique%d", package_index),
                source_mode = "strict_globs",
                source_globs = globs,
            }
        end
        assert.equals(4194304, total_bytes)
        assert.same({ok = true, value = {changed_packages = {}, affected_packages = {},
            prerequisite_packages = {}}},
            GraphDiff.evaluate_diff_selection(simple_diff(packages)))

        packages[9] = {name = "fixture/over-limit", rel_path = "code/over-limit",
            source_mode = "strict_globs", source_globs = {"x"}}
        assert.same({ok = false, code = "DIFF_MATCH_LIMIT_EXCEEDED"},
            GraphDiff.evaluate_diff_selection(simple_diff(packages)))
    end)

    it("indexes full-scale disjoint package roots and changed paths", function()
        local packages, changed_paths = {}, {}
        local prefix = string.rep("a", 500)
        for index = 1, 4096 do
            packages[index] = {
                name = string.format("fixture/p%04d", index),
                rel_path = prefix .. string.format("%04d", index),
                source_mode = "strict_globs",
                source_globs = {},
            }
            changed_paths[index] = prefix .. string.format("b%04d", index)
        end
        local input = simple_diff(packages)
        input.unknown_path_policy = "all"
        input.changed_paths = changed_paths

        local started = os.clock()
        local actual = GraphDiff.evaluate_diff_selection(input)
        local elapsed = os.clock() - started
        assert.is_true(actual.ok, actual.code)
        assert.equals(4096, #actual.value.changed_packages)
        assert.equals(4096, #actual.value.affected_packages)
        assert.same({}, actual.value.prerequisite_packages)
        assert.equals("fixture/p0001", actual.value.changed_packages[1])
        assert.equals("fixture/p4096", actual.value.changed_packages[4096])
        assert.is_true(elapsed < 12,
            string.format("indexed root classification took %.2f seconds", elapsed))
    end)

    it("bounds slash-heavy package-root classification before prefix allocation", function()
        local packages, changed_paths = {}, {}
        local path_prefix = string.rep("a/", 250)
        for index = 1, 1024 do
            packages[index] = package_prefix(
                string.format("fixture/p%04d", index),
                string.format("code/p%04d", index))
            changed_paths[index] = path_prefix .. string.format("%04d", index)
        end
        local input = simple_diff(packages)
        input.changed_paths = changed_paths

        local started = os.clock()
        local actual = GraphDiff.evaluate_diff_selection(input)
        local elapsed = os.clock() - started
        assert.same({ok = false, code = "DIFF_MATCH_LIMIT_EXCEEDED"}, actual)
        assert.is_true(elapsed < 5,
            string.format("root classification preflight took %.2f seconds", elapsed))
    end)

    it("reports a bad boundary digest before slash-heavy root work", function()
        local packages, changed_paths = {}, {}
        local path_prefix = string.rep("a/", 250)
        for index = 1, 1024 do
            packages[index] = package_prefix(
                string.format("fixture/p%04d", index),
                string.format("code/p%04d", index))
            changed_paths[index] = path_prefix .. string.format("%04d", index)
        end
        local input = simple_diff(packages)
        input.changed_paths = changed_paths
        input.boundary_sha256 = string.rep("0", 64)
        input.boundary = {}
        assert.same({ok = false, code = "DIFF_BOUNDARY_DIGEST_MISMATCH"},
            GraphDiff.evaluate_diff_selection(input))
    end)

    it("does not expand max-shape boundary consumers when no paths changed", function()
        local rules = {}
        for rule_index = 1, 58 do
            local inputs = {}
            for input_index = 1, 64 do
                inputs[input_index] = {path = string.format("shared/rule-%d/input-%d",
                    rule_index, input_index), role = "cross_package_exact"}
            end
            rules[rule_index] = {
                id = string.format("rule-%d", rule_index),
                input_origin = "repository",
                applies_to = {exact_roots = json_array(),
                    descendant_roots = json_array({"roots"}),
                    excluded_roots = json_array()},
                inputs = json_array(inputs),
                reason = "max-shape zero-path regression",
                owner = "test-owner",
            }
        end
        local boundary = boundary_document(rules)
        local packages = {}
        for index = 1, 4096 do
            packages[index] = package_prefix(string.format("fixture/p%04d", index),
                string.format("roots/p%04d", index))
        end
        local input = simple_diff(packages)
        input.boundary_sha256 = boundary_digest(boundary)
        input.boundary = boundary
        assert.same({ok = true, value = {changed_packages = {}, affected_packages = {},
            prerequisite_packages = {}}}, GraphDiff.evaluate_diff_selection(input))
    end)

    it("indexes relevant max-shape boundary exclusions", function()
        local rules = {}
        for rule_index = 1, 56 do
            local excluded = {}
            for excluded_index = 1, 1024 do
                excluded[excluded_index] = string.format("excluded/rule-%d/root-%d",
                    rule_index, excluded_index)
            end
            rules[rule_index] = {
                id = string.format("rule-%d", rule_index),
                input_origin = "repository",
                applies_to = {exact_roots = json_array(),
                    descendant_roots = json_array({"roots"}),
                    excluded_roots = json_array(excluded)},
                inputs = json_array({{path = "shared/relevant-input",
                    role = "cross_package_exact"}}),
                reason = "large exclusion index regression",
                owner = "test-owner",
            }
        end
        local boundary = boundary_document(rules)
        local packages = {}
        for index = 1, 4096 do
            packages[index] = package_prefix(string.format("fixture/p%04d", index),
                string.format("roots/p%04d", index))
        end
        local input = simple_diff(packages)
        input.changed_paths = {"shared/relevant-input"}
        input.boundary_sha256 = boundary_digest(boundary)
        input.boundary = boundary
        local actual = GraphDiff.evaluate_diff_selection(input)
        assert.is_true(actual.ok, actual.code)
        assert.equals(4096, #actual.value.changed_packages)
        assert.equals("fixture/p0001", actual.value.changed_packages[1])
        assert.equals("fixture/p4096", actual.value.changed_packages[4096])
    end)

    it("ignores hostile extra fields without weakening metatable rejection", function()
        local package = package_prefix("fixture/a", "code/a")
        local input = simple_diff({package})
        for index = 1, 32768 do
            input["ignored_" .. index] = true
            package[index] = true
        end
        assert.same({ok = true, value = {changed_packages = {}, affected_packages = {},
            prerequisite_packages = {}}}, GraphDiff.evaluate_diff_selection(input))

        setmetatable(input, {__index = {changed_paths = {"code/a/injected.lua"}}})
        assert.same({ok = false, code = "DIFF_INPUT_INVALID"},
            GraphDiff.evaluate_diff_selection(input))
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
