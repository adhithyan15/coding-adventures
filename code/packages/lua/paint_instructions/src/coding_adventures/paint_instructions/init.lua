local M = {}

M.VERSION = "0.1.0"

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

function M.paint_rect(x, y, width, height, fill, metadata)
    return {
        kind = "rect",
        x = x,
        y = y,
        width = width,
        height = height,
        fill = fill or "#000000",
        metadata = copy_metadata(metadata),
    }
end

function M.paint_scene(width, height, instructions, background, metadata)
    return {
        width = width,
        height = height,
        instructions = instructions or {},
        background = background or "#ffffff",
        metadata = copy_metadata(metadata),
    }
end

function M.create_scene(width, height, instructions, background, metadata)
    return M.paint_scene(width, height, instructions, background, metadata)
end

return M
