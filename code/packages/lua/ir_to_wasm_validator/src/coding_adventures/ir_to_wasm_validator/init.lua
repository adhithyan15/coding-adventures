local compiler = require("coding_adventures.ir_to_wasm_compiler")

local M = {}

M.VERSION = "0.1.0"

local WasmIrValidator = {}
WasmIrValidator.__index = WasmIrValidator

function WasmIrValidator.new()
    return setmetatable({}, WasmIrValidator)
end

function WasmIrValidator:validate(program, function_signatures)
    local _, err = compiler.compile(program, function_signatures)
    if err then
        return {
            {
                rule = "lowering",
                message = err,
            },
        }
    end
    return {}
end

function M.validate(program, function_signatures)
    return WasmIrValidator.new():validate(program, function_signatures)
end

M.WasmIrValidator = WasmIrValidator

return M
