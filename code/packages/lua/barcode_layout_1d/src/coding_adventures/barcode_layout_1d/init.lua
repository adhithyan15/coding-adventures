local paint = require("coding_adventures.paint_instructions")

local M = {}

M.VERSION = "0.1.0"

M.DEFAULT_LAYOUT_CONFIG = {
    module_unit = 4,
    bar_height = 120,
    quiet_zone_modules = 10,
}

M.DEFAULT_PAINT_OPTIONS = {
    fill = "#000000",
    background = "#ffffff",
    metadata = {},
}

local function copy_metadata(metadata)
    if metadata == nil then
        return {}
    end

    local copy = {}
    for key, value in pairs(metadata) do
        copy[key] = value
    end
    return copy
end

local function validate_layout_config(config)
    if config.module_unit <= 0 then
        error("module_unit must be a positive integer")
    end
    if config.bar_height <= 0 then
        error("bar_height must be a positive integer")
    end
    if config.quiet_zone_modules < 0 then
        error("quiet_zone_modules must be zero or a positive integer")
    end
end

local function validate_run(run)
    if run.color ~= "bar" and run.color ~= "space" then
        error("run color must be 'bar' or 'space'")
    end
    if run.modules <= 0 then
        error("run modules must be a positive integer")
    end
end

function M.runs_from_binary_pattern(pattern, opts)
    opts = opts or {}
    if pattern == "" then
        return {}
    end

    local runs = {}
    local bar_char = opts.bar_char or "1"
    local space_char = opts.space_char or "0"
    local current = pattern:sub(1, 1)
    local count = 1

    local function flush(token, modules)
        local color
        if token == bar_char then
            color = "bar"
        elseif token == space_char then
            color = "space"
        else
            error(string.format("binary pattern contains unsupported token: %q", token))
        end

        runs[#runs + 1] = {
            color = color,
            modules = modules,
            source_char = opts.source_char or "",
            source_index = opts.source_index or 0,
            role = "data",
            metadata = copy_metadata(opts.metadata),
        }
    end

    for index = 2, #pattern do
        local token = pattern:sub(index, index)
        if token == current then
            count = count + 1
        else
            flush(current, count)
            current = token
            count = 1
        end
    end

    flush(current, count)
    return runs
end

function M.runs_from_width_pattern(pattern, colors, opts)
    opts = opts or {}
    local narrow_modules = opts.narrow_modules or 1
    local wide_modules = opts.wide_modules or 3
    if #pattern ~= #colors then
        error("pattern length must match colors length")
    end
    if narrow_modules <= 0 or wide_modules <= 0 then
        error("narrow_modules and wide_modules must be positive integers")
    end

    local runs = {}
    for index = 1, #pattern do
        local token = pattern:sub(index, index)
        if token ~= "N" and token ~= "W" then
            error(string.format("width pattern contains unsupported token: %q", token))
        end
        runs[#runs + 1] = {
            color = colors[index],
            modules = token == "W" and wide_modules or narrow_modules,
            source_char = opts.source_char,
            source_index = opts.source_index,
            role = opts.role or "data",
            metadata = copy_metadata(opts.metadata),
        }
    end
    return runs
end

function M.layout_barcode_1d(runs, config, options)
    config = config or M.DEFAULT_LAYOUT_CONFIG
    options = options or M.DEFAULT_PAINT_OPTIONS

    validate_layout_config(config)

    local quiet_zone_width = config.quiet_zone_modules * config.module_unit
    local cursor_x = quiet_zone_width
    local instructions = {}

    for _, run in ipairs(runs) do
        validate_run(run)
        local width = run.modules * config.module_unit
        if run.color == "bar" then
            local metadata = copy_metadata(run.metadata)
            metadata.source_char = run.source_char
            metadata.source_index = run.source_index
            metadata.modules = run.modules
            metadata.role = run.role
            instructions[#instructions + 1] = paint.paint_rect(
                cursor_x,
                0,
                width,
                config.bar_height,
                options.fill,
                metadata
            )
        end
        cursor_x = cursor_x + width
    end

    local metadata = copy_metadata(options.metadata)
    metadata.content_width = cursor_x - quiet_zone_width
    metadata.quiet_zone_width = quiet_zone_width
    metadata.module_unit = config.module_unit
    metadata.bar_height = config.bar_height

    return paint.paint_scene(
        cursor_x + quiet_zone_width,
        config.bar_height,
        instructions,
        options.background,
        metadata
    )
end

function M.draw_one_dimensional_barcode(runs, config, options)
    return M.layout_barcode_1d(runs, config, options)
end

-- The bounded, zero-authority v1 adapter is additive to the legacy API.
local function fail(id) error(id, 0) end
local function scalar_count(value, id)
    if type(value) ~= "string" then fail(id) end
    local count = utf8.len(value)
    if count == nil then fail(id) end
    return count
end
local function integer(value)
    return type(value) == "number" and math.type(value) == "integer"
end
local roles = {data = true, start = true, stop = true, guard = true,
    check = true, ["inter-character-gap"] = true}
local symbol_roles = {data = true, start = true, stop = true, guard = true, check = true}
local function source(label, index, role)
    if scalar_count(label, "invalid-source-attribution") > 4096 or
        not integer(index) or index < -2147483648 or index > 2147483647 or
        not roles[role] then fail("invalid-source-attribution") end
end
local function new_run(color, modules, label, index, role)
    return {color = color, modules = modules, source_label = label,
        source_index = index, role = role}
end

function M.runs_from_binary_pattern_v1(pattern, opts)
    opts = opts or {}
    local length = scalar_count(pattern, "invalid-binary-token")
    if length > 65567 then fail("pattern-too-long") end
    if length == 0 then fail("empty-pattern") end
    if pattern:find("[^01]") then fail("invalid-binary-token") end
    source(opts.source_label, opts.source_index, opts.role)
    local result, token, count = {}, nil, 0
    for index = 1, #pattern do
        local bit = pattern:sub(index, index)
        if bit == token then
            count = count + 1
        else
            if token ~= nil then
                result[#result + 1] = new_run(token == "1" and "bar" or "space",
                    count, opts.source_label, opts.source_index, opts.role)
                if #result >= 40979 then fail("too-many-runs") end
            end
            token, count = bit, 1
        end
    end
    result[#result + 1] = new_run(token == "1" and "bar" or "space",
        count, opts.source_label, opts.source_index, opts.role)
    return result
end

function M.runs_from_width_pattern_v1(pattern, opts)
    opts = opts or {}
    local length = scalar_count(pattern, "invalid-width-token")
    if length > 65567 then fail("pattern-too-long") end
    if length == 0 then fail("empty-pattern") end
    local narrow, wide = opts.narrow_marker or "N", opts.wide_marker or "W"
    if scalar_count(narrow, "invalid-marker-configuration") ~= 1 or
        scalar_count(wide, "invalid-marker-configuration") ~= 1 or narrow == wide then
        fail("invalid-marker-configuration")
    end
    local tokens = {}
    for _, codepoint in utf8.codes(pattern) do
        local token = utf8.char(codepoint)
        if token ~= narrow and token ~= wide then fail("invalid-width-token") end
        tokens[#tokens + 1] = token
    end
    source(opts.source_label, opts.source_index, opts.role)
    local narrow_modules = opts.narrow_modules or 1
    local wide_modules = opts.wide_modules or 3
    if not integer(narrow_modules) or narrow_modules <= 0 or
        not integer(wide_modules) or wide_modules <= 0 then fail("invalid-module-count") end
    local starting = opts.starting_color or "bar"
    if starting ~= "bar" and starting ~= "space" then fail("invalid-marker-configuration") end
    if length > 40979 then fail("too-many-runs") end
    local content, rows = 0, {}
    for index, token in ipairs(tokens) do
        local modules = token == wide and wide_modules or narrow_modules
        if modules > 65567 - content then fail("content-too-wide") end
        content = content + modules
        local color = index % 2 == 1 and starting or (starting == "bar" and "space" or "bar")
        rows[index] = new_run(color, modules, opts.source_label, opts.source_index, opts.role)
    end
    return rows
end

local function symbol_layout(label, first, ending, index, role)
    return {label = label, startModule = first, endModule = ending,
        sourceIndex = index, role = role}
end
local function infer_symbols(runs)
    local rows, cursor, active, first = {}, 0, nil, 0
    for _, run in ipairs(runs) do
        local candidate = {run.source_label, run.source_index, run.role}
        if run.role ~= "inter-character-gap" then
            if active == nil then
                active, first = candidate, cursor
            elseif active[1] ~= candidate[1] or active[2] ~= candidate[2] or active[3] ~= candidate[3] then
                rows[#rows + 1] = symbol_layout(active[1], first, cursor, active[2], active[3])
                active, first = candidate, cursor
            end
        end
        cursor = cursor + run.modules
    end
    if active ~= nil then
        rows[#rows + 1] = symbol_layout(active[1], first, cursor, active[2], active[3])
    end
    if #rows > 40979 then fail("too-many-symbols") end
    return rows
end
local function explicit_symbols(symbols, content)
    if type(symbols) ~= "table" then fail("invalid-source-attribution") end
    if #symbols > 40979 then fail("too-many-symbols") end
    local rows, cursor = {}, 0
    for index, symbol in ipairs(symbols) do
        if type(symbol) ~= "table" then fail("invalid-source-attribution") end
        local modules = symbol.modules
        if not integer(modules) or modules <= 0 then fail("invalid-module-count") end
        local source_index = symbol.sourceIndex or symbol.source_index
        source(symbol.label, source_index, symbol.role)
        if not symbol_roles[symbol.role] then fail("invalid-source-attribution") end
        if modules > 65567 - cursor then fail("symbol-width-mismatch") end
        rows[index] = symbol_layout(symbol.label, cursor, cursor + modules,
            source_index, symbol.role)
        cursor = cursor + modules
    end
    if cursor ~= content then fail("symbol-width-mismatch") end
    return rows
end

function M.compute_barcode_1d_layout_v1(runs, quiet, symbols)
    if type(runs) ~= "table" then fail("invalid-source-attribution") end
    if #runs > 40979 then fail("too-many-runs") end
    local copied, content, previous = {}, 0, nil
    for index, run in ipairs(runs) do
        if type(run) ~= "table" or (run.color ~= "bar" and run.color ~= "space") or
            not roles[run.role] then fail("invalid-source-attribution") end
        if not integer(run.modules) or run.modules <= 0 then fail("invalid-module-count") end
        source(run.source_label, run.source_index, run.role)
        if run.modules > 65567 - content then fail("content-too-wide") end
        content = content + run.modules
        if previous == run.color then fail("non-alternating-runs") end
        previous = run.color
        copied[index] = new_run(run.color, run.modules, run.source_label,
            run.source_index, run.role)
    end
    if not integer(quiet) or quiet < 1 or quiet > 4096 then fail("invalid-quiet-zone") end
    local layouts = symbols == nil and infer_symbols(copied) or explicit_symbols(symbols, content)
    return {leftQuietZoneModules = quiet, rightQuietZoneModules = quiet,
        contentModules = content, totalModules = content + 2 * quiet,
        symbolLayouts = layouts}
end

local function bounded_metadata(metadata)
    if type(metadata) ~= "table" then fail("metadata-too-large") end
    local copied, entries, bytes = {}, 0, 0
    for key, value in pairs(metadata) do
        entries = entries + 1
        if entries > 64 or scalar_count(key, "metadata-too-large") > 128 or
            scalar_count(value, "metadata-too-large") > 4096 or
            #key + #value > 65536 - bytes then fail("metadata-too-large") end
        bytes = bytes + #key + #value
        copied[key] = value
    end
    return copied
end

function M.project_barcode_1d_scene_v1(runs, quiet, options)
    options = options or {}
    local render = options.render_config or {}
    if options.human_readable_text ~= nil or
        (type(render) == "table" and render.include_human_readable_text) then
        fail("human-readable-text-unsupported")
    end
    if type(render) ~= "table" then fail("invalid-render-config") end
    local module_width = render.module_width or 4
    local bar_height = render.bar_height or 120
    local foreground = render.foreground or "#000000"
    local background = render.background or "#ffffff"
    if not integer(module_width) or module_width < 1 or module_width > 8192 or
        not integer(bar_height) or bar_height < 1 or bar_height > 8192 or
        scalar_count(foreground, "invalid-render-config") > 128 or
        scalar_count(background, "invalid-render-config") > 128 then
        fail("invalid-render-config")
    end
    local layout = M.compute_barcode_1d_layout_v1(runs, quiet, options.symbols)
    local metadata = bounded_metadata(options.metadata or {})
    local label = options.label or "1D barcode"
    if scalar_count(label, "metadata-too-large") > 4096 then fail("metadata-too-large") end
    local rectangles, cursor = {}, quiet
    for _, run in ipairs(runs) do
        local ending = cursor + run.modules
        if run.color == "bar" then
            rectangles[#rectangles + 1] = paint.paint_rect(cursor * module_width, 0,
                run.modules * module_width, bar_height, foreground,
                {sourceLabel = run.source_label, sourceIndex = tostring(run.source_index),
                role = run.role, moduleStart = tostring(cursor), moduleEnd = tostring(ending)})
        end
        cursor = ending
    end
    local width = layout.totalModules * module_width
    metadata.label = label
    metadata.leftQuietZoneModules = tostring(quiet)
    metadata.rightQuietZoneModules = tostring(quiet)
    metadata.contentModules = tostring(layout.contentModules)
    metadata.totalModules = tostring(layout.totalModules)
    metadata.moduleWidthPx = tostring(module_width)
    metadata.barHeightPx = tostring(bar_height)
    metadata.sceneWidthPx = tostring(width)
    metadata.sceneHeightPx = tostring(bar_height)
    metadata.symbolCount = tostring(#layout.symbolLayouts)
    return paint.paint_scene(width, bar_height, rectangles, background, metadata)
end

return M
