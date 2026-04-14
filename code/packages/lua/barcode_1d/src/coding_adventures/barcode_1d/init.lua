local codabar = require("coding_adventures.codabar")
local code128 = require("coding_adventures.code128")
local code39 = require("coding_adventures.code39")
local ean_13 = require("coding_adventures.ean_13")
local itf = require("coding_adventures.itf")
local upc_a = require("coding_adventures.upc_a")

local M = {}

M.VERSION = "0.1.0"
M.DEFAULT_LAYOUT_CONFIG = code39.DEFAULT_LAYOUT_CONFIG
M.DEFAULT_RENDER_CONFIG = M.DEFAULT_LAYOUT_CONFIG

local function normalize_symbology(symbology)
    local normalized = tostring(symbology or "code39"):gsub("[_-]", ""):lower()
    if normalized == "codabar" then
        return "codabar"
    end
    if normalized == "code128" then
        return "code128"
    end
    if normalized == "code39" then
        return "code39"
    end
    if normalized == "ean13" then
        return "ean13"
    end
    if normalized == "itf" then
        return "itf"
    end
    if normalized == "upca" then
        return "upca"
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
    if normalized == "codabar" then
        return codabar.layout_codabar(data, layout_config or M.DEFAULT_LAYOUT_CONFIG)
    end
    if normalized == "code128" then
        return code128.layout_code128(data, layout_config or M.DEFAULT_LAYOUT_CONFIG)
    end
    if normalized == "code39" then
        return code39.layout_code39(data, layout_config or M.DEFAULT_LAYOUT_CONFIG)
    end
    if normalized == "ean13" then
        return ean_13.layout_ean_13(data, layout_config or M.DEFAULT_LAYOUT_CONFIG)
    end
    if normalized == "itf" then
        return itf.layout_itf(data, layout_config or M.DEFAULT_LAYOUT_CONFIG)
    end
    if normalized == "upca" then
        return upc_a.layout_upc_a(data, layout_config or M.DEFAULT_LAYOUT_CONFIG)
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
