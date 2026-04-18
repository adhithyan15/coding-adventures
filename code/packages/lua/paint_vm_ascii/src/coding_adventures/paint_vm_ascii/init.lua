local M = {}

M.VERSION = "0.1.0"

local function scale_x(options)
    if options == nil or options.scale_x == nil or options.scale_x == 0 then
        return 8
    end
    return options.scale_x
end

local function scale_y(options)
    if options == nil or options.scale_y == nil or options.scale_y == 0 then
        return 16
    end
    return options.scale_y
end

local function to_col(x, sx)
    return math.floor((x / sx) + 0.5)
end

local function to_row(y, sy)
    return math.floor((y / sy) + 0.5)
end

local function new_buffer(rows, cols)
    local chars = {}
    for row = 1, rows do
        chars[row] = {}
        for col = 1, cols do
            chars[row][col] = " "
        end
    end
    return {
        rows = rows,
        cols = cols,
        chars = chars,
    }
end

local function write_char(buffer, row, col, ch)
    if row < 0 or row >= buffer.rows or col < 0 or col >= buffer.cols then
        return
    end
    buffer.chars[row + 1][col + 1] = ch
end

local function buffer_to_string(buffer)
    local lines = {}
    for row = 1, buffer.rows do
        local line = table.concat(buffer.chars[row]):gsub("%s+$", "")
        lines[#lines + 1] = line
    end
    return table.concat(lines, "\n"):gsub("[%s\n]+$", "")
end

local function render_rect(inst, buffer, sx, sy)
    if inst.fill == nil or inst.fill == "" or inst.fill == "transparent" or inst.fill == "none" then
        return
    end

    local c1 = to_col(inst.x, sx)
    local r1 = to_row(inst.y, sy)
    local c2 = to_col(inst.x + inst.width, sx)
    local r2 = to_row(inst.y + inst.height, sy)

    for row = r1, r2 do
        for col = c1, c2 do
            write_char(buffer, row, col, "█")
        end
    end
end

function M.render(scene, options)
    local sx = scale_x(options)
    local sy = scale_y(options)
    local cols = math.ceil(scene.width / sx)
    local rows = math.ceil(scene.height / sy)
    local buffer = new_buffer(rows, cols)

    for _, inst in ipairs(scene.instructions or {}) do
        if inst.kind == "rect" then
            render_rect(inst, buffer, sx, sy)
        else
            error("paint_vm_ascii: unsupported paint instruction kind: " .. tostring(inst.kind))
        end
    end

    return buffer_to_string(buffer)
end

return M
