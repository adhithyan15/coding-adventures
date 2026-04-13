local M = {}

local function dirname(path)
    return path:match("^(.*)[/\\][^/\\]+$")
end

local native_module = false

local function load_native()
    if native_module ~= false then
        return native_module
    end

    local ok, module = pcall(require, "paint_codec_png_native")
    if ok then
        native_module = module
        return native_module
    end

    local source = debug.getinfo(1, "S").source
    local file = source:sub(1, 1) == "@" and source:sub(2) or source
    local base = dirname(file) .. "/../../../target/release/"
    local candidates = {
        base .. "libpaint_codec_png_native.dylib",
        base .. "libpaint_codec_png_native.so",
        base .. "paint_codec_png_native.dll",
    }

    for _, path in ipairs(candidates) do
        local loader = package.loadlib(path, "luaopen_paint_codec_png_native")
        if loader then
            native_module = loader()
            return native_module
        end
    end

    native_module = nil
    return native_module
end

local function byte_table_to_string(bytes)
    local parts = {}
    local chunk_size = 4096
    for i = 1, #bytes, chunk_size do
        local chars = {}
        local last = math.min(i + chunk_size - 1, #bytes)
        for j = i, last do
            chars[#chars + 1] = string.char(bytes[j])
        end
        parts[#parts + 1] = table.concat(chars)
    end
    return table.concat(parts)
end

function M.available()
    return load_native() ~= nil
end

function M.encode(pixels)
    local native = load_native()
    if native == nil then
        error("paint_codec_png_native extension is not available")
    end

    return native.encode_rgba8_native(pixels.width, pixels.height, byte_table_to_string(pixels.data))
end

return M
