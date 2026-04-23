-- ============================================================================
-- ir_to_wasm_compiler — Lower generic compiler IR into a WebAssembly module
-- ============================================================================

local ir = require("coding_adventures.compiler_ir")
local leb128 = require("coding_adventures.wasm_leb128")
local wasm_opcodes = require("coding_adventures.wasm_opcodes")
local wasm_types = require("coding_adventures.wasm_types")

local M = {}

M.VERSION = "0.1.0"

local SYSCALL_WRITE = 1
local SYSCALL_READ = 2
local SYSCALL_EXIT = 10
local SYSCALL_ARG0 = 4

local WASI_MODULE = "wasi_snapshot_preview1"
local WASI_IOVEC_OFFSET = 0
local WASI_COUNT_OFFSET = 8
local WASI_BYTE_OFFSET = 12
local WASI_SCRATCH_SIZE = 16

local REG_SCRATCH = 1
local REG_VAR_BASE = 2

local function must_opcode(name)
    for byte, info in pairs(wasm_opcodes.OPCODES) do
        if info.name == name then
            return byte
        end
    end
    error("ir_to_wasm_compiler: missing wasm opcode " .. name)
end

local OPCODE = {
    nop = must_opcode("nop"),
    block = must_opcode("block"),
    loop_ = must_opcode("loop"),
    if_ = must_opcode("if"),
    else_ = must_opcode("else"),
    end_ = must_opcode("end"),
    br = must_opcode("br"),
    br_if = must_opcode("br_if"),
    return_ = must_opcode("return"),
    call = must_opcode("call"),
    local_get = must_opcode("local.get"),
    local_set = must_opcode("local.set"),
    i32_load = must_opcode("i32.load"),
    i32_load8_u = must_opcode("i32.load8_u"),
    i32_store = must_opcode("i32.store"),
    i32_store8 = must_opcode("i32.store8"),
    i32_const = must_opcode("i32.const"),
    i32_eqz = must_opcode("i32.eqz"),
    i32_eq = must_opcode("i32.eq"),
    i32_ne = must_opcode("i32.ne"),
    i32_lt_s = must_opcode("i32.lt_s"),
    i32_gt_s = must_opcode("i32.gt_s"),
    i32_add = must_opcode("i32.add"),
    i32_sub = must_opcode("i32.sub"),
    i32_and = must_opcode("i32.and"),
}

local function max2(a, b)
    if a > b then
        return a
    end
    return b
end

local function align_up(value, alignment)
    return math.floor((value + alignment - 1) / alignment) * alignment
end

local function bytes_of_size(size, value)
    local bytes = {}
    for index = 1, size do
        bytes[index] = value
    end
    return bytes
end

local function total_data_size(decls)
    local total = 0
    for _, decl in ipairs(decls or {}) do
        total = total + decl.size
    end
    return total
end

local function func_type_key(func_type)
    local params = {}
    local results = {}
    for _, value in ipairs(func_type.params or {}) do
        params[#params + 1] = tostring(value)
    end
    for _, value in ipairs(func_type.results or {}) do
        results[#results + 1] = tostring(value)
    end
    return table.concat(params, ",") .. "=>" .. table.concat(results, ",")
end

local function const_expr(value)
    local bytes = { OPCODE.i32_const }
    local encoded = leb128.encode_signed(value)
    for _, byte in ipairs(encoded) do
        bytes[#bytes + 1] = byte
    end
    bytes[#bytes + 1] = OPCODE.end_
    return bytes
end

local function label_name_from_operand(operand)
    if operand and operand.kind == "label" then
        return operand.name
    end
    return nil
end

local function label_name_from_instruction(instruction)
    if instruction.opcode ~= ir.IrOp.LABEL or #(instruction.operands or {}) == 0 then
        return nil
    end
    return label_name_from_operand(instruction.operands[1])
end

local function function_label_name(instruction)
    local label = label_name_from_instruction(instruction)
    if label == "_start" then
        return label
    end
    if label and label:sub(1, 4) == "_fn_" then
        return label
    end
    return nil
end

local function is_loop_start_label(name)
    return type(name) == "string" and name:match("^loop_%d+_start$") ~= nil
end

local function is_if_else_label(name)
    return type(name) == "string" and name:match("^if_%d+_else$") ~= nil
end

local function describe_operand(operand)
    if operand == nil then
        return "nil"
    end
    if operand.kind == "register" then
        return "v" .. operand.index
    end
    if operand.kind == "immediate" then
        return tostring(operand.value)
    end
    if operand.kind == "label" then
        return operand.name
    end
    return tostring(operand.kind)
end

local function expect_register(operand, context)
    if operand == nil or operand.kind ~= "register" then
        return nil, string.format("%s: expected register, got %s", context, describe_operand(operand))
    end
    return operand, nil
end

local function expect_immediate(operand, context)
    if operand == nil or operand.kind ~= "immediate" then
        return nil, string.format("%s: expected immediate, got %s", context, describe_operand(operand))
    end
    return operand, nil
end

local function expect_label(operand, context)
    if operand == nil or operand.kind ~= "label" then
        return nil, string.format("%s: expected label, got %s", context, describe_operand(operand))
    end
    return operand, nil
end

local function slice(values, first, last)
    local result = {}
    for index = first, last do
        result[#result + 1] = values[index]
    end
    return result
end

function M.new_function_signature(label, param_count, export_name)
    return {
        label = label,
        param_count = param_count,
        export_name = export_name,
    }
end

function M.infer_function_signatures_from_comments(program)
    local signatures = {}
    local pending_comment = nil

    for _, instruction in ipairs(program.instructions or {}) do
        if instruction.opcode == ir.IrOp.COMMENT then
            pending_comment = label_name_from_operand((instruction.operands or {})[1])
        else
            local label = function_label_name(instruction)
            if label then
                if label == "_start" then
                    signatures[label] = M.new_function_signature(label, 0, "_start")
                elseif label:sub(1, 4) == "_fn_" and pending_comment then
                    local export_name = label:sub(5)
                    local comment_name, params_blob =
                        pending_comment:match("^function:%s*([%a_][%w_]*)%((.*)%)$")
                    if comment_name == export_name then
                        local param_count = 0
                        local blob = params_blob and params_blob:match("^%s*(.-)%s*$") or ""
                        if blob ~= "" then
                            for piece in blob:gmatch("[^,]+") do
                                if piece:match("%S") then
                                    param_count = param_count + 1
                                end
                            end
                        end
                        signatures[label] = M.new_function_signature(label, param_count, export_name)
                    end
                end
                pending_comment = nil
            else
                pending_comment = nil
            end
        end
    end

    return signatures
end

local function make_function_ir(label, instructions, signatures)
    local signature = signatures[label]
    if signature == nil and label == "_start" then
        signature = M.new_function_signature(label, 0, "_start")
    end
    if signature == nil then
        return nil, string.format("missing function signature for %s", label)
    end

    local max_reg = max2(1, REG_VAR_BASE + max2(signature.param_count - 1, 0))
    local has_syscall = false
    for _, instruction in ipairs(instructions) do
        if instruction.opcode == ir.IrOp.SYSCALL then
            has_syscall = true
        end
        for _, operand in ipairs(instruction.operands or {}) do
            if operand.kind == "register" then
                max_reg = max2(max_reg, operand.index)
            end
        end
    end
    if has_syscall then
        max_reg = max2(max_reg, SYSCALL_ARG0)
    end

    return {
        label = label,
        instructions = instructions,
        signature = signature,
        max_reg = max_reg,
    }, nil
end

local FunctionLowerer = {}
FunctionLowerer.__index = FunctionLowerer

function FunctionLowerer.new(options)
    local self = setmetatable({}, FunctionLowerer)
    self.options = options
    self.param_count = options.fn.signature.param_count
    self.instructions = options.fn.instructions
    self.bytes = {}
    self.label_to_index = {}

    for index, instruction in ipairs(self.instructions) do
        local label = label_name_from_instruction(instruction)
        if label then
            self.label_to_index[label] = index
        end
    end

    return self
end

function FunctionLowerer:emit_opcode(value)
    self.bytes[#self.bytes + 1] = value
end

function FunctionLowerer:emit_bytes(values)
    for _, byte in ipairs(values) do
        self.bytes[#self.bytes + 1] = byte
    end
end

function FunctionLowerer:emit_u32(value)
    self:emit_bytes(leb128.encode_unsigned(value))
end

function FunctionLowerer:emit_i32_const(value)
    self:emit_opcode(OPCODE.i32_const)
    self:emit_bytes(leb128.encode_signed(value))
end

function FunctionLowerer:emit_memarg(align, offset)
    self:emit_u32(align)
    self:emit_u32(offset)
end

function FunctionLowerer:local_index(reg_index)
    return self.param_count + reg_index
end

function FunctionLowerer:emit_local_get(reg_index)
    self:emit_opcode(OPCODE.local_get)
    self:emit_u32(self:local_index(reg_index))
end

function FunctionLowerer:emit_local_set(reg_index)
    self:emit_opcode(OPCODE.local_set)
    self:emit_u32(self:local_index(reg_index))
end

function FunctionLowerer:emit_address(base_index, offset_index)
    self:emit_local_get(base_index)
    self:emit_local_get(offset_index)
    self:emit_opcode(OPCODE.i32_add)
end

function FunctionLowerer:emit_store_const_i32(address, value)
    self:emit_i32_const(address)
    self:emit_i32_const(value)
    self:emit_opcode(OPCODE.i32_store)
    self:emit_memarg(2, 0)
end

function FunctionLowerer:require_wasi_scratch()
    if self.options.wasi_context.scratch_base == nil then
        return nil, "SYSCALL lowering requires WASM scratch memory"
    end
    return self.options.wasi_context.scratch_base, nil
end

function FunctionLowerer:emit_wasi_call(syscall_number)
    local function_index = self.options.wasi_context.function_indices[syscall_number]
    if function_index == nil then
        return string.format("missing WASI import for SYSCALL %d", syscall_number)
    end
    self:emit_opcode(OPCODE.call)
    self:emit_u32(function_index)
    return nil
end

function FunctionLowerer:emit_wasi_write()
    local scratch_base, err = self:require_wasi_scratch()
    if not scratch_base then
        return err
    end

    local iovec_ptr = scratch_base + WASI_IOVEC_OFFSET
    local nwritten_ptr = scratch_base + WASI_COUNT_OFFSET
    local byte_ptr = scratch_base + WASI_BYTE_OFFSET

    self:emit_i32_const(byte_ptr)
    self:emit_local_get(SYSCALL_ARG0)
    self:emit_opcode(OPCODE.i32_store8)
    self:emit_memarg(0, 0)

    self:emit_store_const_i32(iovec_ptr, byte_ptr)
    self:emit_store_const_i32(iovec_ptr + 4, 1)

    self:emit_i32_const(1)
    self:emit_i32_const(iovec_ptr)
    self:emit_i32_const(1)
    self:emit_i32_const(nwritten_ptr)
    err = self:emit_wasi_call(SYSCALL_WRITE)
    if err then
        return err
    end
    self:emit_local_set(REG_SCRATCH)
    return nil
end

function FunctionLowerer:emit_wasi_read()
    local scratch_base, err = self:require_wasi_scratch()
    if not scratch_base then
        return err
    end

    local iovec_ptr = scratch_base + WASI_IOVEC_OFFSET
    local nread_ptr = scratch_base + WASI_COUNT_OFFSET
    local byte_ptr = scratch_base + WASI_BYTE_OFFSET

    self:emit_i32_const(byte_ptr)
    self:emit_i32_const(0)
    self:emit_opcode(OPCODE.i32_store8)
    self:emit_memarg(0, 0)

    self:emit_store_const_i32(iovec_ptr, byte_ptr)
    self:emit_store_const_i32(iovec_ptr + 4, 1)

    self:emit_i32_const(0)
    self:emit_i32_const(iovec_ptr)
    self:emit_i32_const(1)
    self:emit_i32_const(nread_ptr)
    err = self:emit_wasi_call(SYSCALL_READ)
    if err then
        return err
    end
    self:emit_local_set(REG_SCRATCH)

    self:emit_i32_const(byte_ptr)
    self:emit_opcode(OPCODE.i32_load8_u)
    self:emit_memarg(0, 0)
    self:emit_local_set(SYSCALL_ARG0)
    return nil
end

function FunctionLowerer:emit_wasi_exit()
    self:emit_local_get(SYSCALL_ARG0)
    local err = self:emit_wasi_call(SYSCALL_EXIT)
    if err then
        return err
    end
    self:emit_i32_const(0)
    self:emit_opcode(OPCODE.return_)
    return nil
end

function FunctionLowerer:emit_binary_numeric(opcode, instruction)
    local dst, err = expect_register(instruction.operands[1], ir.IrOpName[instruction.opcode] .. " dst")
    if not dst then
        return err
    end
    local left = nil
    left, err = expect_register(instruction.operands[2], ir.IrOpName[instruction.opcode] .. " lhs")
    if not left then
        return err
    end
    local right = nil
    right, err = expect_register(instruction.operands[3], ir.IrOpName[instruction.opcode] .. " rhs")
    if not right then
        return err
    end

    self:emit_local_get(left.index)
    self:emit_local_get(right.index)
    self:emit_opcode(opcode)
    self:emit_local_set(dst.index)
    return nil
end

function FunctionLowerer:emit_syscall(instruction)
    local syscall, err = expect_immediate(instruction.operands[1], "SYSCALL number")
    if not syscall then
        return err
    end
    if syscall.value == SYSCALL_WRITE then
        return self:emit_wasi_write()
    elseif syscall.value == SYSCALL_READ then
        return self:emit_wasi_read()
    elseif syscall.value == SYSCALL_EXIT then
        return self:emit_wasi_exit()
    end
    return string.format("unsupported SYSCALL number: %d", syscall.value)
end

function FunctionLowerer:emit_simple(instruction)
    local err

    if instruction.opcode == ir.IrOp.LOAD_IMM then
        local dst, imm
        dst, err = expect_register(instruction.operands[1], "LOAD_IMM dst")
        if not dst then return err end
        imm, err = expect_immediate(instruction.operands[2], "LOAD_IMM imm")
        if not imm then return err end
        self:emit_i32_const(imm.value)
        self:emit_local_set(dst.index)
        return nil
    elseif instruction.opcode == ir.IrOp.LOAD_ADDR then
        local dst, label
        dst, err = expect_register(instruction.operands[1], "LOAD_ADDR dst")
        if not dst then return err end
        label, err = expect_label(instruction.operands[2], "LOAD_ADDR label")
        if not label then return err end
        local offset = self.options.data_offsets[label.name]
        if offset == nil then
            return "unknown data label: " .. label.name
        end
        self:emit_i32_const(offset)
        self:emit_local_set(dst.index)
        return nil
    elseif instruction.opcode == ir.IrOp.LOAD_BYTE then
        local dst, base, offset
        dst, err = expect_register(instruction.operands[1], "LOAD_BYTE dst")
        if not dst then return err end
        base, err = expect_register(instruction.operands[2], "LOAD_BYTE base")
        if not base then return err end
        offset, err = expect_register(instruction.operands[3], "LOAD_BYTE offset")
        if not offset then return err end
        self:emit_address(base.index, offset.index)
        self:emit_opcode(OPCODE.i32_load8_u)
        self:emit_memarg(0, 0)
        self:emit_local_set(dst.index)
        return nil
    elseif instruction.opcode == ir.IrOp.STORE_BYTE then
        local src, base, offset
        src, err = expect_register(instruction.operands[1], "STORE_BYTE src")
        if not src then return err end
        base, err = expect_register(instruction.operands[2], "STORE_BYTE base")
        if not base then return err end
        offset, err = expect_register(instruction.operands[3], "STORE_BYTE offset")
        if not offset then return err end
        self:emit_address(base.index, offset.index)
        self:emit_local_get(src.index)
        self:emit_opcode(OPCODE.i32_store8)
        self:emit_memarg(0, 0)
        return nil
    elseif instruction.opcode == ir.IrOp.LOAD_WORD then
        local dst, base, offset
        dst, err = expect_register(instruction.operands[1], "LOAD_WORD dst")
        if not dst then return err end
        base, err = expect_register(instruction.operands[2], "LOAD_WORD base")
        if not base then return err end
        offset, err = expect_register(instruction.operands[3], "LOAD_WORD offset")
        if not offset then return err end
        self:emit_address(base.index, offset.index)
        self:emit_opcode(OPCODE.i32_load)
        self:emit_memarg(2, 0)
        self:emit_local_set(dst.index)
        return nil
    elseif instruction.opcode == ir.IrOp.STORE_WORD then
        local src, base, offset
        src, err = expect_register(instruction.operands[1], "STORE_WORD src")
        if not src then return err end
        base, err = expect_register(instruction.operands[2], "STORE_WORD base")
        if not base then return err end
        offset, err = expect_register(instruction.operands[3], "STORE_WORD offset")
        if not offset then return err end
        self:emit_address(base.index, offset.index)
        self:emit_local_get(src.index)
        self:emit_opcode(OPCODE.i32_store)
        self:emit_memarg(2, 0)
        return nil
    elseif instruction.opcode == ir.IrOp.ADD then
        return self:emit_binary_numeric(OPCODE.i32_add, instruction)
    elseif instruction.opcode == ir.IrOp.ADD_IMM then
        local dst, src, imm
        dst, err = expect_register(instruction.operands[1], "ADD_IMM dst")
        if not dst then return err end
        src, err = expect_register(instruction.operands[2], "ADD_IMM src")
        if not src then return err end
        imm, err = expect_immediate(instruction.operands[3], "ADD_IMM imm")
        if not imm then return err end
        self:emit_local_get(src.index)
        self:emit_i32_const(imm.value)
        self:emit_opcode(OPCODE.i32_add)
        self:emit_local_set(dst.index)
        return nil
    elseif instruction.opcode == ir.IrOp.SUB then
        return self:emit_binary_numeric(OPCODE.i32_sub, instruction)
    elseif instruction.opcode == ir.IrOp.AND then
        return self:emit_binary_numeric(OPCODE.i32_and, instruction)
    elseif instruction.opcode == ir.IrOp.AND_IMM then
        local dst, src, imm
        dst, err = expect_register(instruction.operands[1], "AND_IMM dst")
        if not dst then return err end
        src, err = expect_register(instruction.operands[2], "AND_IMM src")
        if not src then return err end
        imm, err = expect_immediate(instruction.operands[3], "AND_IMM imm")
        if not imm then return err end
        self:emit_local_get(src.index)
        self:emit_i32_const(imm.value)
        self:emit_opcode(OPCODE.i32_and)
        self:emit_local_set(dst.index)
        return nil
    elseif instruction.opcode == ir.IrOp.CMP_EQ then
        return self:emit_binary_numeric(OPCODE.i32_eq, instruction)
    elseif instruction.opcode == ir.IrOp.CMP_NE then
        return self:emit_binary_numeric(OPCODE.i32_ne, instruction)
    elseif instruction.opcode == ir.IrOp.CMP_LT then
        return self:emit_binary_numeric(OPCODE.i32_lt_s, instruction)
    elseif instruction.opcode == ir.IrOp.CMP_GT then
        return self:emit_binary_numeric(OPCODE.i32_gt_s, instruction)
    elseif instruction.opcode == ir.IrOp.CALL then
        local label, signature, function_index
        label, err = expect_label(instruction.operands[1], "CALL target")
        if not label then return err end
        signature = self.options.signatures[label.name]
        if signature == nil then
            return "missing function signature for " .. label.name
        end
        function_index = self.options.function_indices[label.name]
        if function_index == nil then
            return "unknown function label: " .. label.name
        end
        for param_index = 0, signature.param_count - 1 do
            self:emit_local_get(REG_VAR_BASE + param_index)
        end
        self:emit_opcode(OPCODE.call)
        self:emit_u32(function_index)
        self:emit_local_set(REG_SCRATCH)
        return nil
    elseif instruction.opcode == ir.IrOp.RET or instruction.opcode == ir.IrOp.HALT then
        self:emit_local_get(REG_SCRATCH)
        self:emit_opcode(OPCODE.return_)
        return nil
    elseif instruction.opcode == ir.IrOp.NOP then
        self:emit_opcode(OPCODE.nop)
        return nil
    elseif instruction.opcode == ir.IrOp.SYSCALL then
        return self:emit_syscall(instruction)
    end

    return "unsupported opcode: " .. tostring(ir.IrOpName[instruction.opcode] or instruction.opcode)
end

function FunctionLowerer:require_label_index(label)
    local index = self.label_to_index[label]
    if index == nil then
        return nil, string.format("missing label %s in %s", label, self.options.fn.label)
    end
    return index, nil
end

function FunctionLowerer:find_first_branch_to_label(first, last, label)
    for index = first, last do
        local instruction = self.instructions[index]
        if instruction.opcode == ir.IrOp.BRANCH_Z or instruction.opcode == ir.IrOp.BRANCH_NZ then
            if label_name_from_operand((instruction.operands or {})[2]) == label then
                return index, nil
            end
        end
    end
    return nil, string.format("expected branch to %s in %s", label, self.options.fn.label)
end

function FunctionLowerer:find_last_jump_to_label(first, last, label)
    for index = last, first, -1 do
        local instruction = self.instructions[index]
        if instruction.opcode == ir.IrOp.JUMP then
            if label_name_from_operand((instruction.operands or {})[1]) == label then
                return index, nil
            end
        end
    end
    return nil, string.format("expected jump to %s in %s", label, self.options.fn.label)
end

function FunctionLowerer:emit_if(branch_index)
    local branch = self.instructions[branch_index]
    local cond_reg, else_label, err
    cond_reg, err = expect_register(branch.operands[1], "if condition")
    if not cond_reg then return nil, err end
    else_label, err = expect_label(branch.operands[2], "if else label")
    if not else_label then return nil, err end

    local end_label = else_label.name:gsub("_else$", "_end")
    if end_label == else_label.name then
        end_label = else_label.name .. "_end"
    end

    local else_index, end_index, jump_index
    else_index, err = self:require_label_index(else_label.name)
    if not else_index then return nil, err end
    end_index, err = self:require_label_index(end_label)
    if not end_index then return nil, err end
    jump_index, err = self:find_last_jump_to_label(branch_index + 1, else_index, end_label)
    if not jump_index then return nil, err end

    self:emit_local_get(cond_reg.index)
    if branch.opcode == ir.IrOp.BRANCH_NZ then
        self:emit_opcode(OPCODE.i32_eqz)
    end
    self:emit_opcode(OPCODE.if_)
    self:emit_opcode(wasm_types.BlockType.empty)

    err = self:emit_region(branch_index + 1, jump_index - 1)
    if err then return nil, err end
    if else_index + 1 <= end_index - 1 then
        self:emit_opcode(OPCODE.else_)
        err = self:emit_region(else_index + 1, end_index - 1)
        if err then return nil, err end
    end

    self:emit_opcode(OPCODE.end_)
    return end_index + 1, nil
end

function FunctionLowerer:emit_loop(label_index)
    local start_label = label_name_from_instruction(self.instructions[label_index])
    if not start_label then
        return nil, "loop lowering expected a start label"
    end

    local end_label = start_label:gsub("_start$", "_end")
    if end_label == start_label then
        end_label = start_label .. "_end"
    end

    local end_index, branch_index, backedge_index, err
    end_index, err = self:require_label_index(end_label)
    if not end_index then return nil, err end
    branch_index, err = self:find_first_branch_to_label(label_index + 1, end_index, end_label)
    if not branch_index then return nil, err end
    backedge_index, err = self:find_last_jump_to_label(branch_index + 1, end_index, start_label)
    if not backedge_index then return nil, err end

    local branch = self.instructions[branch_index]
    local cond_reg
    cond_reg, err = expect_register(branch.operands[1], "loop condition")
    if not cond_reg then return nil, err end

    self:emit_opcode(OPCODE.block)
    self:emit_opcode(wasm_types.BlockType.empty)
    self:emit_opcode(OPCODE.loop_)
    self:emit_opcode(wasm_types.BlockType.empty)

    err = self:emit_region(label_index + 1, branch_index - 1)
    if err then return nil, err end

    self:emit_local_get(cond_reg.index)
    if branch.opcode == ir.IrOp.BRANCH_Z then
        self:emit_opcode(OPCODE.i32_eqz)
    end
    self:emit_opcode(OPCODE.br_if)
    self:emit_u32(1)

    err = self:emit_region(branch_index + 1, backedge_index - 1)
    if err then return nil, err end

    self:emit_opcode(OPCODE.br)
    self:emit_u32(0)
    self:emit_opcode(OPCODE.end_)
    self:emit_opcode(OPCODE.end_)
    return end_index + 1, nil
end

function FunctionLowerer:emit_region(first, last)
    if first > last then
        return nil
    end

    local index = first
    while index <= last do
        local instruction = self.instructions[index]

        if instruction.opcode == ir.IrOp.COMMENT then
            index = index + 1
        else
            local label = label_name_from_instruction(instruction)
            if label and is_loop_start_label(label) then
                local next_index, err = self:emit_loop(index)
                if not next_index then return err end
                index = next_index
            elseif (instruction.opcode == ir.IrOp.BRANCH_Z or instruction.opcode == ir.IrOp.BRANCH_NZ)
                and is_if_else_label(label_name_from_operand((instruction.operands or {})[2])) then
                local next_index, err = self:emit_if(index)
                if not next_index then return err end
                index = next_index
            elseif instruction.opcode == ir.IrOp.LABEL then
                index = index + 1
            elseif instruction.opcode == ir.IrOp.JUMP
                or instruction.opcode == ir.IrOp.BRANCH_Z
                or instruction.opcode == ir.IrOp.BRANCH_NZ then
                return string.format("unexpected unstructured control flow in %s", self.options.fn.label)
            else
                local err = self:emit_simple(instruction)
                if err then
                    return err
                end
                index = index + 1
            end
        end
    end

    return nil
end

function FunctionLowerer:copy_params_into_ir_registers()
    for param_index = 0, self.param_count - 1 do
        self:emit_opcode(OPCODE.local_get)
        self:emit_u32(param_index)
        self:emit_opcode(OPCODE.local_set)
        self:emit_u32(self:local_index(REG_VAR_BASE + param_index))
    end
end

function FunctionLowerer:lower()
    self:copy_params_into_ir_registers()
    local err = self:emit_region(2, #self.instructions)
    if err then
        return nil, err
    end
    self:emit_opcode(OPCODE.end_)

    return {
        locals = {
            { count = self.options.fn.max_reg + 1, type = wasm_types.ValType.i32 },
        },
        body = self.bytes,
    }, nil
end

local IrToWasmCompiler = {}
IrToWasmCompiler.__index = IrToWasmCompiler

function IrToWasmCompiler.new()
    return setmetatable({}, IrToWasmCompiler)
end

function IrToWasmCompiler:build_type_table(functions, imports)
    local seen = {}
    local types = {}
    local type_indices = {}

    local function remember_type(key, func_type)
        local signature_key = func_type_key(func_type)
        local index = seen[signature_key]
        if index == nil then
            index = #types
            seen[signature_key] = index
            types[#types + 1] = func_type
        end
        type_indices[key] = index
    end

    for _, entry in ipairs(imports) do
        remember_type(entry.type_key, entry.func_type)
    end
    for _, fn in ipairs(functions) do
        local params = {}
        for _ = 1, fn.signature.param_count do
            params[#params + 1] = wasm_types.ValType.i32
        end
        remember_type(fn.label, {
            params = params,
            results = { wasm_types.ValType.i32 },
        })
    end

    return type_indices, types
end

function IrToWasmCompiler:layout_data(decls)
    local offsets = {}
    local cursor = 0
    for _, decl in ipairs(decls or {}) do
        offsets[decl.label] = cursor
        cursor = cursor + decl.size
    end
    return offsets
end

function IrToWasmCompiler:needs_memory(program)
    if #(program.data or {}) > 0 then
        return true
    end
    for _, instruction in ipairs(program.instructions or {}) do
        local op = instruction.opcode
        if op == ir.IrOp.LOAD_ADDR
            or op == ir.IrOp.LOAD_BYTE
            or op == ir.IrOp.STORE_BYTE
            or op == ir.IrOp.LOAD_WORD
            or op == ir.IrOp.STORE_WORD then
            return true
        end
    end
    return false
end

function IrToWasmCompiler:needs_wasi_scratch(program)
    for _, instruction in ipairs(program.instructions or {}) do
        if instruction.opcode == ir.IrOp.SYSCALL then
            local syscall = instruction.operands and instruction.operands[1]
            if syscall and syscall.kind == "immediate" then
                if syscall.value == SYSCALL_WRITE or syscall.value == SYSCALL_READ then
                    return true
                end
            end
        end
    end
    return false
end

function IrToWasmCompiler:collect_wasi_imports(program)
    local required = {}
    for _, instruction in ipairs(program.instructions or {}) do
        if instruction.opcode == ir.IrOp.SYSCALL then
            local syscall, err = expect_immediate((instruction.operands or {})[1], "SYSCALL number")
            if not syscall then
                return nil, err
            end
            required[syscall.value] = true
        end
    end

    local ordered = {
        {
            syscall_number = SYSCALL_WRITE,
            name = "fd_write",
            func_type = {
                params = { wasm_types.ValType.i32, wasm_types.ValType.i32, wasm_types.ValType.i32, wasm_types.ValType.i32 },
                results = { wasm_types.ValType.i32 },
            },
            type_key = "wasi::fd_write",
        },
        {
            syscall_number = SYSCALL_READ,
            name = "fd_read",
            func_type = {
                params = { wasm_types.ValType.i32, wasm_types.ValType.i32, wasm_types.ValType.i32, wasm_types.ValType.i32 },
                results = { wasm_types.ValType.i32 },
            },
            type_key = "wasi::fd_read",
        },
        {
            syscall_number = SYSCALL_EXIT,
            name = "proc_exit",
            func_type = {
                params = { wasm_types.ValType.i32 },
                results = {},
            },
            type_key = "wasi::proc_exit",
        },
    }

    local supported = {}
    for _, entry in ipairs(ordered) do
        supported[entry.syscall_number] = true
    end

    local unsupported = {}
    for syscall in pairs(required) do
        if not supported[syscall] then
            unsupported[#unsupported + 1] = syscall
        end
    end
    table.sort(unsupported)
    if #unsupported > 0 then
        local names = {}
        for _, value in ipairs(unsupported) do
            names[#names + 1] = tostring(value)
        end
        return nil, "unsupported SYSCALL number(s): " .. table.concat(names, ", ")
    end

    local imports = {}
    for _, entry in ipairs(ordered) do
        if required[entry.syscall_number] then
            imports[#imports + 1] = entry
        end
    end
    return imports, nil
end

function IrToWasmCompiler:split_functions(program, signatures)
    local functions = {}
    local start_index = nil
    local start_label = nil

    for index, instruction in ipairs(program.instructions or {}) do
        local label = function_label_name(instruction)
        if label then
            if start_label ~= nil and start_index ~= nil then
                local fn, err = make_function_ir(
                    start_label,
                    slice(program.instructions, start_index, index - 1),
                    signatures
                )
                if not fn then
                    return nil, err
                end
                functions[#functions + 1] = fn
            end
            start_label = label
            start_index = index
        end
    end

    if start_label ~= nil and start_index ~= nil then
        local fn, err = make_function_ir(
            start_label,
            slice(program.instructions, start_index, #program.instructions),
            signatures
        )
        if not fn then
            return nil, err
        end
        functions[#functions + 1] = fn
    end

    return functions, nil
end

function IrToWasmCompiler:compile(program, function_signatures)
    local signatures = M.infer_function_signatures_from_comments(program)
    for _, signature in ipairs(function_signatures or {}) do
        signatures[signature.label] = signature
    end

    local functions, err = self:split_functions(program, signatures)
    if not functions then return nil, err end
    local imports = nil
    imports, err = self:collect_wasi_imports(program)
    if not imports then return nil, err end

    local type_indices, types = self:build_type_table(functions, imports)
    local data_offsets = self:layout_data(program.data or {})
    local scratch_base = nil
    if self:needs_wasi_scratch(program) then
        scratch_base = align_up(total_data_size(program.data or {}), 4)
    end

    local module = {
        types = types,
        imports = {},
        functions = {},
        tables = {},
        memories = {},
        globals = {},
        exports = {},
        start = nil,
        elements = {},
        codes = {},
        data = {},
        custom = {},
    }

    for _, entry in ipairs(imports) do
        module.imports[#module.imports + 1] = {
            module_name = WASI_MODULE,
            name = entry.name,
            kind = wasm_types.ExternType.func,
            type_index = type_indices[entry.type_key],
        }
    end

    local function_index_base = #imports
    local function_indices = {}
    for index, fn in ipairs(functions) do
        function_indices[fn.label] = function_index_base + index - 1
        module.functions[#module.functions + 1] = type_indices[fn.label]
    end

    local total_bytes = total_data_size(program.data or {})
    if scratch_base ~= nil then
        total_bytes = max2(total_bytes, scratch_base + WASI_SCRATCH_SIZE)
    end

    if self:needs_memory(program) or scratch_base ~= nil then
        local page_count = 1
        if total_bytes > 0 then
            page_count = max2(1, math.floor((total_bytes + 65535) / 65536))
        end
        module.memories[#module.memories + 1] = {
            limits = { min = page_count, max = nil },
        }
        module.exports[#module.exports + 1] = {
            name = "memory",
            kind = wasm_types.ExternType.mem,
            index = 0,
        }
        for _, decl in ipairs(program.data or {}) do
            module.data[#module.data + 1] = {
                memory_index = 0,
                offset_expr = const_expr(data_offsets[decl.label]),
                data = bytes_of_size(decl.size, decl.init & 0xFF),
            }
        end
    end

    local wasi_context = {
        function_indices = {},
        scratch_base = scratch_base,
    }
    for index, entry in ipairs(imports) do
        wasi_context.function_indices[entry.syscall_number] = index - 1
    end

    for _, fn in ipairs(functions) do
        local body
        body, err = FunctionLowerer.new({
            fn = fn,
            signatures = signatures,
            function_indices = function_indices,
            data_offsets = data_offsets,
            wasi_context = wasi_context,
        }):lower()
        if not body then
            return nil, err
        end
        module.codes[#module.codes + 1] = body
        if fn.signature.export_name ~= nil then
            module.exports[#module.exports + 1] = {
                name = fn.signature.export_name,
                kind = wasm_types.ExternType.func,
                index = function_indices[fn.label],
            }
        end
    end

    return module, nil
end

function M.compile(program, function_signatures)
    return IrToWasmCompiler.new():compile(program, function_signatures)
end

M.IrToWasmCompiler = IrToWasmCompiler

return M
