local code39 = require("coding_adventures.code39")

local M = {}

M.VERSION = "0.1.0"
M.DEFAULT_LAYOUT_CONFIG = code39.DEFAULT_LAYOUT_CONFIG
M.DEFAULT_RENDER_CONFIG = M.DEFAULT_LAYOUT_CONFIG

local function normalize_symbology(symbology)
    local normalized = tostring(symbology or "code39"):gsub("[_-]", ""):lower()
    if normalized == "code39" then
        return "code39"
    end
    error("unsupported symbology: " .. tostring(symbology))
end

function M.current_backend()
    local ok, vm = pcall(require, "coding_adventures.paint_vm_metal_native")
    if ok and vm.available() then
        return "metal"
    end
    return nil
end

function M.build_scene(data, symbology, layout_config)
    local normalized = normalize_symbology(symbology or "code39")
    if normalized == "code39" then
        return code39.layout_code39(data, layout_config or M.DEFAULT_LAYOUT_CONFIG)
    end
end

function M.render_pixels(data, symbology, layout_config)
    if M.current_backend() ~= "metal" then
        error("no native Paint VM is available for this host")
    end

    local vm = require("coding_adventures.paint_vm_metal_native")
    return vm.render(M.build_scene(data, symbology, layout_config))
end

function M.render_png(data, symbology, layout_config)
    local codec = require("coding_adventures.paint_codec_png_native")
    return codec.encode(M.render_pixels(data, symbology, layout_config))
end

return M
