local pixel_container = require("coding_adventures.pixel_container")

local M = {}

local function dirname(path)
    return path:match("^(.*)[/\\][^/\\]+$")
end

local native_module = false

local function load_native()
    if native_module ~= false then
        return native_module
    end

    local ok, module = pcall(require, "paint_vm_metal_native")
    if ok then
        native_module = module
        return native_module
    end

    local source = debug.getinfo(1, "S").source
    local file = source:sub(1, 1) == "@" and source:sub(2) or source
    local base = dirname(file) .. "/../../../target/release/"
    local candidates = {
        base .. "libpaint_vm_metal_native.dylib",
        base .. "libpaint_vm_metal_native.so",
        base .. "paint_vm_metal_native.dll",
    }

    for _, path in ipairs(candidates) do
        local loader = package.loadlib(path, "luaopen_paint_vm_metal_native")
        if loader then
            native_module = loader()
            return native_module
        end
    end

    native_module = nil
    return native_module
end

local function fetch_value(object, key, default)
    if object[key] ~= nil then
        return object[key]
    end
    if default ~= nil then
        return default
    end
    error("scene is missing " .. tostring(key))
end

local function encode_scene(scene)
    local rects = {}
    for _, instruction in ipairs(fetch_value(scene, "instructions")) do
        local kind = fetch_value(instruction, "kind")
        if kind ~= "rect" then
            error("only rect paint instructions are supported right now")
        end
        rects[#rects + 1] = {
            fetch_value(instruction, "x"),
            fetch_value(instruction, "y"),
            fetch_value(instruction, "width"),
            fetch_value(instruction, "height"),
            fetch_value(instruction, "fill", "#000000"),
        }
    end

    return fetch_value(scene, "width"), fetch_value(scene, "height"), fetch_value(scene, "background", "#ffffff"), rects
end

function M.available()
    return load_native() ~= nil
end

function M.supported_runtime()
    return M.available()
end

function M.render(scene)
    local native = load_native()
    if native == nil then
        error("paint_vm_metal_native extension is not available")
    end

    local width, height, background, rects = encode_scene(scene)
    local payload = native.render_rect_scene_native(width, height, background, rects)

    local pixels = pixel_container.new(payload.width, payload.height)
    for index = 1, #payload.data do
        pixels.data[index] = payload.data:byte(index)
    end
    return pixels
end

return M
