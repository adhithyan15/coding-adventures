local nib_parser = require("coding_adventures.nib_parser")
local nib_type_checker = require("coding_adventures.nib_type_checker")
local nib_ir_compiler = require("coding_adventures.nib_ir_compiler")
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

local function child_nodes(node)
    local nodes = {}
    if node == nil or node.children == nil then
        return nodes
    end
    for _, child in ipairs(node.children) do
        if type(child) == "table" and child.rule_name ~= nil then
            nodes[#nodes + 1] = child
        end
    end
    return nodes
end

local function first_name(node)
    if node == nil then
        return nil
    end
    for _, child in ipairs(node.children or {}) do
        if type(child) == "table" and child.rule_name ~= nil then
            local name = first_name(child)
            if name ~= nil then
                return name
            end
        elseif (child.type_name or child.type) == "NAME" then
            return child.value
        end
    end
    return nil
end

local function extract_signatures(root)
    local signatures = {
        ir_to_wasm_compiler.new_function_signature("_start", 0, "_start"),
    }

    for _, child in ipairs(child_nodes(root)) do
        local decl = child.rule_name == "top_decl" and child_nodes(child)[1] or child
        if decl ~= nil and decl.rule_name == "fn_decl" then
            local param_count = 0
            local param_list = nil
            for _, node in ipairs(child_nodes(decl)) do
                if node.rule_name == "param_list" then
                    param_list = node
                    break
                end
            end
            if param_list ~= nil then
                for _, param in ipairs(child_nodes(param_list)) do
                    if param.rule_name == "param" then
                        param_count = param_count + 1
                    end
                end
            end
            signatures[#signatures + 1] = ir_to_wasm_compiler.new_function_signature("_fn_" .. first_name(decl), param_count, first_name(decl))
        end
    end

    return signatures
end

local Compiler = {}
Compiler.__index = Compiler

function Compiler.new()
    return setmetatable({}, Compiler)
end

function Compiler:compile_source(source)
    local ok, ast_or_err = pcall(nib_parser.parse, source)
    if not ok then
        return nil, PackageError.new("parse", tostring(ast_or_err), ast_or_err)
    end

    local type_result = nib_type_checker.check(ast_or_err)
    if not type_result.ok then
        return nil, PackageError.new("type-check", type_result.errors[1].message)
    end

    local ir_result = nib_ir_compiler.compile_nib(type_result.typed_ast, nib_ir_compiler.release_config())
    local signatures = extract_signatures(type_result.typed_ast.root)
    local lowering_errors = ir_to_wasm_validator.validate(ir_result.program, signatures)
    if #lowering_errors > 0 then
        return nil, PackageError.new("validate-ir", lowering_errors[1].message)
    end

    local module, compile_err = ir_to_wasm_compiler.compile(ir_result.program, signatures)
    if module == nil then
        return nil, PackageError.new("compile", tostring(compile_err), compile_err)
    end

    local valid, validated_module = wasm_validator.validate(module)
    if not valid then
        return nil, PackageError.new("validate-wasm", tostring(validated_module), validated_module)
    end

    local encoded_ok, binary_or_err = pcall(wasm_module_encoder.encode_module, module)
    if not encoded_ok then
        return nil, PackageError.new("assemble", tostring(binary_or_err), binary_or_err)
    end

    return {
        source = source,
        ast = ast_or_err,
        typed_ast = type_result.typed_ast,
        raw_ir = ir_result.program,
        optimized_ir = ir_result.program,
        module = module,
        validated_module = validated_module,
        binary = binary_or_err,
        wasm_path = nil,
    }, nil
end

function Compiler:write_wasm_file(source, output_path)
    local result, err = self:compile_source(source)
    if result == nil then
        return nil, err
    end

    local file = io.open(output_path, "wb")
    if not file then
        return nil, PackageError.new("write", "failed to open " .. tostring(output_path))
    end
    file:write(result.binary)
    file:close()
    result.wasm_path = output_path
    return result, nil
end

function M.compile_source(source)
    return Compiler.new():compile_source(source)
end

function M.pack_source(source)
    return M.compile_source(source)
end

function M.write_wasm_file(source, output_path)
    return Compiler.new():write_wasm_file(source, output_path)
end

M.PackageError = PackageError
M.NibWasmCompiler = Compiler

return M
