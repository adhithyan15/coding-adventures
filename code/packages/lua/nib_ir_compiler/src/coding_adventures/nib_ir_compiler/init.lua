local ir = require("coding_adventures.compiler_ir")

local M = {}
M.VERSION = "0.1.0"

local expression_rules = {
    expr = true,
    or_expr = true,
    and_expr = true,
    eq_expr = true,
    cmp_expr = true,
    add_expr = true,
    bitwise_expr = true,
    unary_expr = true,
    primary = true,
    call_expr = true,
}

function M.new_build_config(opts)
    opts = opts or {}
    return { optimize = opts.optimize ~= false }
end

function M.release_config()
    return M.new_build_config({ optimize = true })
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

local function tokens_in(node, acc)
    acc = acc or {}
    if node == nil or node.children == nil then
        return acc
    end
    for _, child in ipairs(node.children) do
        if type(child) == "table" and child.rule_name ~= nil then
            tokens_in(child, acc)
        else
            acc[#acc + 1] = child
        end
    end
    return acc
end

local function token_type(token)
    return token.type_name or token.type
end

local function first_name(node)
    for _, token in ipairs(tokens_in(node)) do
        if token_type(token) == "NAME" then
            return token.value
        end
    end
    return nil
end

local function first_rule(node, rule_name)
    for _, child in ipairs(child_nodes(node)) do
        if child.rule_name == rule_name then
            return child
        end
    end
    return nil
end

local function expression_children(node)
    local nodes = {}
    for _, child in ipairs(child_nodes(node)) do
        if expression_rules[child.rule_name] then
            nodes[#nodes + 1] = child
        end
    end
    return nodes
end

local function literal_value(node)
    for _, token in ipairs(tokens_in(node)) do
        local ttype = token_type(token)
        if ttype == "INT_LIT" then
            return tonumber(token.value)
        elseif ttype == "HEX_LIT" then
            return tonumber(token.value:sub(3), 16)
        elseif ttype == "true" or token.value == "true" then
            return 1
        elseif ttype == "false" or token.value == "false" then
            return 0
        end
    end
    return nil
end

local function operator_tokens(node)
    local names = {}
    for _, token in ipairs(tokens_in(node)) do
        names[#names + 1] = token_type(token)
    end
    return names
end

local function direct_token(node)
    local tokens = tokens_in(node)
    if #child_nodes(node) == 0 and #tokens == 1 then
        return tokens[1]
    end
    return nil
end

local function contains(list, value)
    for _, item in ipairs(list) do
        if item == value then
            return true
        end
    end
    return false
end

local Compiler = {}
Compiler.__index = Compiler

function Compiler.new()
    return setmetatable({
        program = ir.new_program("_start"),
        id_gen = ir.new_id_generator(),
        registers = {},
        next_register = 2,
        loop_index = 0,
    }, Compiler)
end

function Compiler:emit(opcode, operands)
    ir.add_instruction(self.program, ir.new_instruction(opcode, operands, ir.next_id(self.id_gen)))
end

function Compiler:emit_label(name)
    ir.add_instruction(self.program, ir.new_instruction(ir.IrOp.LABEL, { ir.new_label(name) }, -1))
end

function Compiler:allocate_register(name)
    if self.registers[name] ~= nil then
        return self.registers[name]
    end
    local register = self.next_register
    self.next_register = self.next_register + 1
    self.registers[name] = register
    return register
end

function Compiler:function_nodes(root)
    local nodes = {}
    for _, child in ipairs(child_nodes(root)) do
        local decl = child.rule_name == "top_decl" and child_nodes(child)[1] or child
        if decl ~= nil and decl.rule_name == "fn_decl" then
            nodes[#nodes + 1] = decl
        end
    end
    return nodes
end

function Compiler:params(node)
    local params = {}
    local param_list = first_rule(node, "param_list")
    if param_list == nil then
        return params
    end
    for _, param in ipairs(child_nodes(param_list)) do
        if param.rule_name == "param" then
            params[#params + 1] = { first_name(param), true }
        end
    end
    return params
end

function Compiler:emit_expr_into(node, register_index)
    if node.rule_name == "call_expr" then
        self:compile_call(node, register_index)
        return
    end

    if node.rule_name == "add_expr" then
        self:compile_add(node, register_index)
        return
    end

    local children = child_nodes(node)
    if expression_rules[node.rule_name] and #children == 1 then
        self:emit_expr_into(children[1], register_index)
        return
    end

    local token = direct_token(node)
    if token ~= nil then
        local ttype = token_type(token)
        if ttype == "INT_LIT" then
            self:emit(ir.IrOp.LOAD_IMM, { ir.new_register(register_index), ir.new_immediate(tonumber(token.value)) })
            return
        elseif ttype == "HEX_LIT" then
            self:emit(ir.IrOp.LOAD_IMM, { ir.new_register(register_index), ir.new_immediate(tonumber(token.value:sub(3), 16)) })
            return
        elseif ttype == "true" or token.value == "true" then
            self:emit(ir.IrOp.LOAD_IMM, { ir.new_register(register_index), ir.new_immediate(1) })
            return
        elseif ttype == "false" or token.value == "false" then
            self:emit(ir.IrOp.LOAD_IMM, { ir.new_register(register_index), ir.new_immediate(0) })
            return
        elseif ttype == "NAME" and self.registers[token.value] ~= nil then
            self:emit(ir.IrOp.ADD_IMM, { ir.new_register(register_index), ir.new_register(self.registers[token.value]), ir.new_immediate(0) })
            return
        end
    end

    local inner = children[1]
    if inner ~= nil then
        self:emit_expr_into(inner, register_index)
    end
end

function Compiler:compile_call(node, register_index)
    local arg_list = first_rule(node, "arg_list")
    local args = {}
    if arg_list ~= nil then
        for _, child in ipairs(child_nodes(arg_list)) do
            if child.rule_name == "expr" then
                args[#args + 1] = child
            end
        end
    end

    for index, arg in ipairs(args) do
        self:emit_expr_into(arg, index + 1)
    end

    self:emit(ir.IrOp.CALL, { ir.new_label("_fn_" .. first_name(node)) })
    if register_index ~= 1 then
        self:emit(ir.IrOp.ADD_IMM, { ir.new_register(register_index), ir.new_register(1), ir.new_immediate(0) })
    end
end

function Compiler:compile_add(node, register_index)
    local operands = expression_children(node)
    if #operands == 0 then
        return
    end
    self:emit_expr_into(operands[1], register_index)
    if #operands == 1 then
        return
    end

    local value = literal_value(operands[2])
    if value ~= nil then
        if contains(operator_tokens(node), "MINUS") then
            value = -value
        end
        self:emit(ir.IrOp.ADD_IMM, { ir.new_register(register_index), ir.new_register(register_index), ir.new_immediate(value) })
    else
        local scratch = self.next_register
        self.next_register = self.next_register + 1
        self:emit_expr_into(operands[2], scratch)
        if contains(operator_tokens(node), "MINUS") then
            self:emit(ir.IrOp.SUB, { ir.new_register(register_index), ir.new_register(register_index), ir.new_register(scratch) })
        else
            self:emit(ir.IrOp.ADD, { ir.new_register(register_index), ir.new_register(register_index), ir.new_register(scratch) })
        end
    end
end

function Compiler:compile_stmt(stmt)
    local inner = stmt.rule_name == "stmt" and child_nodes(stmt)[1] or stmt
    if inner == nil then
        return
    end

    if inner.rule_name == "let_stmt" then
        local name = first_name(inner)
        local expr = first_rule(inner, "expr")
        if name ~= nil and expr ~= nil then
            local register = self:allocate_register(name)
            self:emit_expr_into(expr, register)
        end
    elseif inner.rule_name == "assign_stmt" then
        local name = first_name(inner)
        local expr = first_rule(inner, "expr")
        if name ~= nil and expr ~= nil and self.registers[name] ~= nil then
            self:emit_expr_into(expr, self.registers[name])
        end
    elseif inner.rule_name == "return_stmt" then
        local expr = first_rule(inner, "expr")
        if expr ~= nil then
            self:emit_expr_into(expr, 1)
        end
        self:emit(ir.IrOp.RET, {})
    elseif inner.rule_name == "expr_stmt" then
        local expr = first_rule(inner, "expr")
        if expr ~= nil then
            self:emit_expr_into(expr, 1)
        end
    elseif inner.rule_name == "for_stmt" then
        local name = first_name(inner)
        local exprs = {}
        for _, child in ipairs(child_nodes(inner)) do
            if child.rule_name == "expr" then
                exprs[#exprs + 1] = child
            end
        end
        local loop_block = first_rule(inner, "block")
        if name ~= nil and #exprs >= 2 and loop_block ~= nil then
            local loop_register = self:allocate_register(name)
            self:emit_expr_into(exprs[1], loop_register)

            local end_register = self.next_register
            self.next_register = self.next_register + 1
            self:emit_expr_into(exprs[2], end_register)

            local cond_register = self.next_register
            self.next_register = self.next_register + 1

            local start_label = "loop_" .. self.loop_index .. "_start"
            local end_label = "loop_" .. self.loop_index .. "_end"
            self.loop_index = self.loop_index + 1

            self:emit_label(start_label)
            self:emit(ir.IrOp.CMP_LT, { ir.new_register(cond_register), ir.new_register(loop_register), ir.new_register(end_register) })
            self:emit(ir.IrOp.BRANCH_Z, { ir.new_register(cond_register), ir.new_label(end_label) })
            for _, nested in ipairs(child_nodes(loop_block)) do
                self:compile_stmt(nested)
            end
            self:emit(ir.IrOp.ADD_IMM, { ir.new_register(loop_register), ir.new_register(loop_register), ir.new_immediate(1) })
            self:emit(ir.IrOp.JUMP, { ir.new_label(start_label) })
            self:emit_label(end_label)
        end
    end
end

function Compiler:compile(root)
    self:emit_label("_start")
    for _, fn_decl in ipairs(self:function_nodes(root)) do
        if first_name(fn_decl) == "main" then
            self:emit(ir.IrOp.CALL, { ir.new_label("_fn_main") })
            break
        end
    end
    self:emit(ir.IrOp.HALT, {})

    for _, fn_decl in ipairs(self:function_nodes(root)) do
        self.registers = {}
        self.next_register = 2
        self:emit_label("_fn_" .. first_name(fn_decl))
        for index, param in ipairs(self:params(fn_decl)) do
            self.registers[param[1]] = index + 1
            self.next_register = index + 2
        end
        local block = first_rule(fn_decl, "block")
        if block ~= nil then
            for _, stmt in ipairs(child_nodes(block)) do
                self:compile_stmt(stmt)
            end
        end
        self:emit(ir.IrOp.RET, {})
    end

    return {
        program = self.program,
    }
end

function M.compile_nib(typed_ast, _config)
    return Compiler.new():compile(typed_ast.root)
end

return M
