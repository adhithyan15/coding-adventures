package.path = (
    "../src/?.lua;" ..
    "../src/?/init.lua;" ..
    "../../paint_instructions/src/?.lua;" ..
    "../../paint_instructions/src/?/init.lua;" ..
    "../../sha256/src/?.lua;" ..
    "../../sha256/src/?/init.lua;" ..
    package.path
)

local json = require("dkjson")
local sha256 = require("coding_adventures.sha256")
local layout = require("coding_adventures.barcode_layout_1d")

local fixture_root = "../../../../specs/fixtures/barcode-layout-1d-v1/"
local max_bytes = 131072

-- The corpus is test data, not a source of production authority. This small
-- syntax walk rejects duplicate keys before dkjson turns an object into a Lua
-- table and silently discards an earlier value. It also bounds nesting before
-- the decoder constructs the result.
local function strict_decode(raw, depth_limit)
    assert(#raw <= max_bytes, "fixture-size-limit")
    assert(utf8.len(raw) ~= nil, "fixture-invalid-utf8")
    local cursor = 1

    local function space()
        while cursor <= #raw and raw:sub(cursor, cursor):match("%s") do
            cursor = cursor + 1
        end
    end

    local function string_token()
        assert(raw:sub(cursor, cursor) == '"', "fixture-invalid-json")
        local start = cursor
        cursor = cursor + 1
        while cursor <= #raw do
            local char = raw:sub(cursor, cursor)
            if char == "\\" then
                cursor = cursor + 2
            elseif char == '"' then
                cursor = cursor + 1
                local value, next_cursor, err = json.decode(raw:sub(start, cursor - 1), 1, json.null)
                assert(err == nil and next_cursor == cursor - start + 1, "fixture-invalid-json")
                assert(utf8.len(value) ~= nil, "fixture-invalid-scalar")
                return value
            else
                cursor = cursor + 1
            end
        end
        error("fixture-invalid-json")
    end

    local value
    value = function(depth)
        assert(depth <= depth_limit, "fixture-depth-limit")
        space()
        local char = raw:sub(cursor, cursor)
        if char == '"' then
            string_token()
        elseif char == "{" then
            cursor = cursor + 1
            space()
            local keys = {}
            if raw:sub(cursor, cursor) ~= "}" then
                while true do
                    local key = string_token()
                    assert(not keys[key], "fixture-duplicate-key")
                    keys[key] = true
                    space()
                    assert(raw:sub(cursor, cursor) == ":", "fixture-invalid-json")
                    cursor = cursor + 1
                    value(depth + 1)
                    space()
                    local separator = raw:sub(cursor, cursor)
                    if separator == "}" then break end
                    assert(separator == ",", "fixture-invalid-json")
                    cursor = cursor + 1
                    space()
                end
            end
            cursor = cursor + 1
        elseif char == "[" then
            cursor = cursor + 1
            space()
            if raw:sub(cursor, cursor) ~= "]" then
                while true do
                    value(depth + 1)
                    space()
                    local separator = raw:sub(cursor, cursor)
                    if separator == "]" then break end
                    assert(separator == ",", "fixture-invalid-json")
                    cursor = cursor + 1
                end
            end
            cursor = cursor + 1
        else
            local start = cursor
            while cursor <= #raw and not raw:sub(cursor, cursor):match("[%s,%]%}]") do
                cursor = cursor + 1
            end
            assert(cursor > start, "fixture-invalid-json")
            local token = raw:sub(start, cursor - 1)
            local decoded, next_cursor, err = json.decode(token, 1, json.null)
            assert(err == nil and next_cursor == #token + 1 and decoded ~= nil,
                "fixture-invalid-json")
            assert(type(decoded) ~= "number" or (decoded == decoded and decoded ~= math.huge and decoded ~= -math.huge),
                "fixture-nonfinite")
        end
    end

    value(0)
    space()
    assert(cursor == #raw + 1, "fixture-trailing-data")
    local decoded, next_cursor, err = json.decode(raw, 1, json.null)
    assert(err == nil and raw:sub(next_cursor):match("^%s*$"), "fixture-invalid-json")
    return decoded
end

local function read_bounded(name, depth_limit)
    local file = assert(io.open(fixture_root .. name, "rb"))
    local raw = file:read(max_bytes + 1)
    file:close()
    assert(raw ~= nil, "fixture-empty")
    return strict_decode(raw, depth_limit)
end

local function corpus_cases()
    local schema = read_bounded("schema.json", 24)
    assert(type(schema) == "table", "fixture-schema-invalid")
    local document = read_bounded("cases.json", 8)
    assert(document.schema_version == 1 and document.profile == "barcode-layout-1d-v1")
    assert(type(document.cases) == "table" and #document.cases > 0 and #document.cases <= 64)
    local seen = {}
    for _, row in ipairs(document.cases) do
        assert(type(row.id) == "string" and not seen[row.id], "fixture-case-id")
        assert(type(row.input) == "table" and type(row.expected) == "table", "fixture-case-shape")
        seen[row.id] = true
    end
    return document.cases
end

local function pattern(input)
    if input.pattern ~= nil then return input.pattern end
    local repeated = input["repeat"]
    assert(type(repeated.count) == "number" and repeated.count >= 0 and repeated.count <= 65569)
    assert(type(repeated.token) == "string" and utf8.len(repeated.token) >= 1 and utf8.len(repeated.token) <= 2)
    local suffix = repeated.suffix or ""
    assert(utf8.len(suffix) <= 1)
    return string.rep(repeated.token, repeated.count) .. suffix
end

local function runs(input)
    local rows = input.runs
    if rows == nil then
        local repeated = input.repeatRuns
        assert(repeated.count >= 0 and repeated.count <= 40980)
        rows = {}
        for index = 1, repeated.count do
            local first = repeated.firstColor
            rows[index] = {
                color = index % 2 == 1 and first or (first == "bar" and "space" or "bar"),
                modules = repeated.modules,
                sourceLabel = repeated.sourceLabel,
                sourceIndex = repeated.sourceIndex,
                role = repeated.role,
            }
        end
    end
    local copied = {}
    for index, row in ipairs(rows) do
        copied[index] = {
            color = row.color, modules = row.modules, source_label = row.sourceLabel,
            source_index = row.sourceIndex, role = row.role,
        }
    end
    return copied
end

local function symbols(input)
    if input.symbols ~= nil then return input.symbols end
    local repeated = input.repeatSymbols
    if repeated == nil then return nil end
    assert(repeated.count >= 0 and repeated.count <= 40980)
    local result = {}
    for index = 1, repeated.count do
        result[index] = {
            label = repeated.label, modules = repeated.modules,
            sourceIndex = index - 1, role = repeated.role,
        }
    end
    return result
end

local function execute(row)
    local input = row.input
    if row.operation == "expand-binary" then
        return layout.runs_from_binary_pattern_v1(pattern(input), {
            source_label = input.sourceLabel, source_index = input.sourceIndex, role = input.role,
        })
    elseif row.operation == "expand-width" then
        return layout.runs_from_width_pattern_v1(pattern(input), {
            source_label = input.sourceLabel, source_index = input.sourceIndex, role = input.role,
            narrow_marker = input.narrowMarker or "N", wide_marker = input.wideMarker or "W",
            narrow_modules = input.narrowModules or 1, wide_modules = input.wideModules or 3,
            starting_color = input.startingColor or "bar",
        })
    elseif row.operation == "compute-layout" then
        return layout.compute_barcode_1d_layout_v1(runs(input), input.quietZoneModules, symbols(input))
    elseif row.operation == "project-scene" then
        local render = input.renderConfig or {}
        return layout.project_barcode_1d_scene_v1(runs(input), input.quietZoneModules, {
            render_config = {
                module_width = render.moduleWidth or 4,
                bar_height = render.barHeight or 120,
                foreground = render.foreground or "#000000",
                background = render.background or "#ffffff",
                include_human_readable_text = render.includeHumanReadableText or false,
            },
            label = input.label or "1D barcode", metadata = input.metadata or {},
            human_readable_text = input.humanReadableText, symbols = symbols(input),
        })
    end
    error("fixture-unknown-operation")
end

local function projected_runs(runs_value)
    local result = {}
    for index, run in ipairs(runs_value) do
        result[index] = {
            color = run.color, modules = run.modules,
            sourceLabel = run.source_label, sourceIndex = run.source_index, role = run.role,
        }
    end
    return result
end

local function canonical_runs(rows)
    local parts = {}
    for index, row in ipairs(rows) do
        parts[index] = '{"color":' .. json.encode(row.color) ..
            ',"modules":' .. tostring(row.modules) ..
            ',"role":' .. json.encode(row.role) ..
            ',"sourceIndex":' .. tostring(row.sourceIndex) ..
            ',"sourceLabel":' .. json.encode(row.sourceLabel) .. '}'
    end
    return '[' .. table.concat(parts, ',') .. ']'
end

local function projected_scene(scene)
    local rectangles = {}
    for index, rect in ipairs(scene.instructions) do
        rectangles[index] = {
            x = rect.x, y = rect.y, width = rect.width, height = rect.height,
            fill = rect.fill, metadata = rect.metadata,
        }
    end
    return {
        width = scene.width, height = scene.height, background = scene.background,
        rectangles = rectangles, metadata = scene.metadata,
    }
end

describe("barcode-layout-1d-v1 neutral corpus", function()
    it("executes all 56 cases through the native Lua facade", function()
        local cases = corpus_cases()
        assert.equal(56, #cases)
        local counts = {}
        for _, row in ipairs(cases) do
            counts[row.operation] = (counts[row.operation] or 0) + 1
            local expected = row.expected
            if expected.error ~= nil then
                local ok, caught = pcall(execute, row)
                assert.is_false(ok, row.id)
                assert.equal(expected.error, caught, row.id)
            else
                local actual = execute(row)
                if expected.runs ~= nil then
                    assert.same(expected.runs, projected_runs(actual), row.id)
                elseif expected.runDigest ~= nil then
                    local projected = projected_runs(actual)
                    local digest = expected.runDigest
                    local modules = 0
                    for _, run in ipairs(projected) do modules = modules + run.modules end
                    assert.equal(digest.runCount, #projected, row.id)
                    assert.equal(digest.contentModules, modules, row.id)
                    assert.same(digest.firstRun, projected[1], row.id)
                    assert.same(digest.lastRun, projected[#projected], row.id)
                    assert.equal(digest.runsSha256, sha256.sha256_hex(canonical_runs(projected)), row.id)
                elseif expected.layout ~= nil then
                    assert.same(expected.layout, actual, row.id)
                else
                    assert.same(expected.scene, projected_scene(actual), row.id)
                end
            end
        end
        assert.same({["expand-binary"] = 12, ["expand-width"] = 12,
            ["compute-layout"] = 19, ["project-scene"] = 13}, counts)
    end)

    it("rejects both text forms before invoking a native resolver", function()
        local calls = 0
        local resolver = function() calls = calls + 1 end
        local malformed = {{color = "invalid", modules = 0}}
        local ok, caught = pcall(layout.project_barcode_1d_scene_v1, malformed, 0,
            {human_readable_text = "text", font_resolver = resolver})
        assert.is_false(ok)
        assert.equal("human-readable-text-unsupported", caught)
        assert.equal(0, calls)
        ok, caught = pcall(layout.project_barcode_1d_scene_v1, malformed, 0,
            {render_config = {include_human_readable_text = true}, font_resolver = resolver})
        assert.is_false(ok)
        assert.equal("human-readable-text-unsupported", caught)
        assert.equal(0, calls)
    end)

    it("deep-copies mutable run and scene results", function()
        local runs_value = layout.runs_from_binary_pattern_v1("10", {
            source_label = "A", source_index = 0, role = "data",
        })
        local caller_metadata = {label = "collision", note = "original"}
        local first = layout.project_barcode_1d_scene_v1(runs_value, 10,
            {metadata = caller_metadata})
        runs_value[1].source_label = "changed"
        caller_metadata.note = "changed"
        assert.equal("A", first.instructions[1].metadata.sourceLabel)
        assert.equal("original", first.metadata.note)
        assert.equal("1D barcode", first.metadata.label)
        first.instructions[1].metadata.sourceLabel = "mutated result"
        local second = layout.project_barcode_1d_scene_v1(
            layout.runs_from_binary_pattern_v1("10", {
                source_label = "A", source_index = 0, role = "data",
            }), 10, {metadata = {note = "original"}})
        assert.equal("A", second.instructions[1].metadata.sourceLabel)
    end)

    it("rejects sparse caller run and symbol tables", function()
        local first = {color = "bar", modules = 1, source_label = "A",
            source_index = 0, role = "data"}
        local ok, caught = pcall(layout.compute_barcode_1d_layout_v1,
            {[1] = first, [3] = first}, 10)
        assert.is_false(ok)
        assert.equal("invalid-source-attribution", caught)
        ok, caught = pcall(layout.compute_barcode_1d_layout_v1, {first}, 10,
            {[1] = {label = "A", modules = 1, sourceIndex = 0, role = "data"},
                [3] = {label = "B", modules = 1, sourceIndex = 1, role = "data"}})
        assert.is_false(ok)
        assert.equal("invalid-source-attribution", caught)
    end)

    it("rejects explicit false options rather than silently defaulting them", function()
        local runs_value = layout.runs_from_binary_pattern_v1("10", {
            source_label = "A", source_index = 0, role = "data",
        })
        local ok, caught = pcall(layout.project_barcode_1d_scene_v1, runs_value, 10,
            {render_config = {module_width = false}})
        assert.is_false(ok)
        assert.equal("invalid-render-config", caught)
        ok, caught = pcall(layout.project_barcode_1d_scene_v1, runs_value, 10,
            {metadata = false})
        assert.is_false(ok)
        assert.equal("metadata-too-large", caught)
    end)

    it("rejects duplicate keys and hostile fixture envelopes", function()
        assert.has_error(function() strict_decode('{"a":1,"a":2}', 8) end)
        assert.has_error(function() strict_decode(string.rep("x", max_bytes + 1), 8) end)
        assert.has_error(function() strict_decode(string.char(255), 8) end)
        assert.has_error(function() strict_decode(string.rep("[", 9) .. "0" .. string.rep("]", 9), 8) end)
    end)
end)
