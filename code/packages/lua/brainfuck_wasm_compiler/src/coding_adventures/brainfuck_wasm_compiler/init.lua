-- ============================================================================
-- brainfuck_wasm_compiler — End-to-end Brainfuck source → WASM bytes
-- ============================================================================

local bf_parser = require("coding_adventures.brainfuck.parser")
local brainfuck_ir_compiler = require("coding_adventures.brainfuck_ir_compiler")
local ir_to_wasm_compiler = require("coding_adventures.ir_to_wasm_compiler")
local ir_to_wasm_validator = require("coding_adventures.ir_to_wasm_validator")
local wasm_module_encoder = require("coding_adventures.wasm_module_encoder")
local wasm_validator = require("coding_adventures.wasm_validator")

local M = {}

M.VERSION = "0.1.0"

local PackageError = {}
PackageError.__index = PackageError

function PackageError.new(stage, message, cause)
    return setmetatable({
        stage = stage,
        message = message,
        cause = cause,
    }, PackageError)
end

function PackageError:__tostring()
    return string.format("[%s] %s", self.stage, self.message)
end

local function wrap_stage(stage, err)
    if err == nil then
        return nil
    end
    if getmetatable(err) == PackageError then
        return err
    end
    return PackageError.new(stage, tostring(err), err)
end

local BrainfuckWasmCompiler = {}
BrainfuckWasmCompiler.__index = BrainfuckWasmCompiler

function BrainfuckWasmCompiler.new(options)
    options = options or {}
    return setmetatable({
        filename = options.filename or "program.bf",
        build_config = options.build_config,
    }, BrainfuckWasmCompiler)
end

function BrainfuckWasmCompiler:compile_source(source, options)
    options = options or {}

    local filename = options.filename or self.filename
    local build_config = options.build_config or self.build_config or brainfuck_ir_compiler.release_config()
    local signatures = {
        ir_to_wasm_compiler.new_function_signature("_start", 0, "_start"),
    }

    local ok, ast_or_err = pcall(bf_parser.parse, source)
    if not ok then
        return nil, wrap_stage("parse", ast_or_err)
    end
    local ast = ast_or_err

    local ir_result, err = brainfuck_ir_compiler.compile(ast, filename, build_config)
    if ir_result == nil then
        return nil, wrap_stage("ir-compile", err)
    end

    local lowering_errors = ir_to_wasm_validator.validate(ir_result.program, signatures)
    if #lowering_errors > 0 then
        return nil, wrap_stage("validate-ir", lowering_errors[1].message)
    end

    local module = nil
    module, err = ir_to_wasm_compiler.compile(ir_result.program, signatures)
    if module == nil then
        return nil, wrap_stage("lower", err)
    end

    local validated_module = nil
    local valid = false
    valid, validated_module = wasm_validator.validate(module)
    if not valid then
        return nil, wrap_stage("validate-wasm", validated_module)
    end

    local encoded_ok, binary_or_err = pcall(wasm_module_encoder.encode_module, module)
    if not encoded_ok then
        return nil, wrap_stage("encode", binary_or_err)
    end

    return {
        source = source,
        filename = filename,
        ast = ast,
        raw_ir = ir_result.program,
        optimized_ir = ir_result.program,
        module = module,
        validated_module = validated_module,
        binary = binary_or_err,
        wasm_path = nil,
    }, nil
end

function BrainfuckWasmCompiler:write_wasm_file(source, output_path, options)
    local result, err = self:compile_source(source, options)
    if result == nil then
        return nil, err
    end

    local file = io.open(output_path, "wb")
    if not file then
        return nil, wrap_stage("write", "failed to open " .. tostring(output_path))
    end

    local ok, write_err = pcall(function()
        file:write(result.binary)
        file:close()
    end)
    if not ok then
        return nil, wrap_stage("write", write_err)
    end

    result.wasm_path = output_path
    return result, nil
end

function M.compile_source(source, options)
    return BrainfuckWasmCompiler.new(options):compile_source(source, options)
end

function M.pack_source(source, options)
    return M.compile_source(source, options)
end

function M.write_wasm_file(source, output_path, options)
    return BrainfuckWasmCompiler.new(options):write_wasm_file(source, output_path, options)
end

M.PackageError = PackageError
M.BrainfuckWasmCompiler = BrainfuckWasmCompiler

return M
