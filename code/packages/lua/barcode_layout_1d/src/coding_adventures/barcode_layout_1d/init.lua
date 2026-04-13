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

return M
