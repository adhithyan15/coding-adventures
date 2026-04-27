local nib_parser = require("coding_adventures.nib_parser")
local protocol = require("coding_adventures.type_checker_protocol")

local M = {}
M.VERSION = "0.1.0"

local U4 = "u4"
local U8 = "u8"
local BCD = "bcd"
local BOOL = "bool"
local VOID = "void"
local LITERAL = "literal"

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

local ScopeChain = {}
ScopeChain.__index = ScopeChain

function ScopeChain.new()
    return setmetatable({ globals = {}, locals = {} }, ScopeChain)
end

function ScopeChain:define_global(name, symbol)
    self.globals[name] = symbol
    return self
end

function ScopeChain:push()
    table.insert(self.locals, {})
    return self
end

function ScopeChain:pop()
    table.remove(self.locals)
    return self
end

function ScopeChain:define_local(name, symbol)
    local frame = self.locals[#self.locals]
    if frame == nil then
        self:define_global(name, symbol)
    else
        frame[name] = symbol
    end
    return self
end

function ScopeChain:lookup(name)
    for i = #self.locals, 1, -1 do
        local frame = self.locals[i]
        if frame[name] ~= nil then
            return frame[name]
        end
    end
    return self.globals[name]
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

local function expression_children(node)
    local nodes = {}
    for _, child in ipairs(child_nodes(node)) do
        if expression_rules[child.rule_name] then
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

local function first_name_token(node)
    for _, token in ipairs(tokens_in(node)) do
        if token_type(token) == "NAME" then
            return token
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

local function type_node(node)
    return first_rule(node, "type")
end

local function resolve_type(node)
    if node == nil then
        return nil
    end
    local token = tokens_in(node)[1]
    if token == nil then
        return nil
    end
    if token.value == "u4" then
        return U4
    elseif token.value == "u8" then
        return U8
    elseif token.value == "bcd" then
        return BCD
    elseif token.value == "bool" then
        return BOOL
    end
    return nil
end

local function compatible(expected, actual)
    return expected == actual or (actual == LITERAL and (expected == U4 or expected == U8 or expected == BCD))
end

local function numeric(kind)
    return kind == U4 or kind == U8 or kind == BCD
end

local function numericish(kind)
    return numeric(kind) or kind == LITERAL
end

local function error(state, message, subject)
    local line = 1
    local column = 1
    if subject ~= nil then
        if subject.line ~= nil then
            line = subject.line
            column = subject.column or subject.col or 1
        else
            line = subject.start_line or 1
            column = subject.start_column or 1
        end
    end
    table.insert(state.errors, protocol.new_type_error_diagnostic(message, line, column))
    return state
end

local function annotate(state, node, inferred)
    if node ~= nil and inferred ~= nil then
        state.types[node] = inferred
    end
    return state
end

local function unwrap_top_decl(node)
    return child_nodes(node)[1]
end

local function collect_const_or_static(node, scope, is_const)
    local name = first_name_token(node)
    local nib_type = resolve_type(type_node(node))
    if name ~= nil and nib_type ~= nil then
        scope:define_global(name.value, {
            name = name.value,
            nib_type = nib_type,
            is_const = is_const,
            is_static = not is_const,
        })
    end
    return scope
end

local function extract_params(node)
    local params = {}
    local param_list = first_rule(node, "param_list")
    if param_list == nil then
        return params
    end
    for _, param in ipairs(child_nodes(param_list)) do
        if param.rule_name == "param" then
            local name = first_name_token(param)
            local nib_type = resolve_type(type_node(param))
            if name ~= nil and nib_type ~= nil then
                params[#params + 1] = { name.value, nib_type }
            end
        end
    end
    return params
end

local function collect_fn_signature(node, scope)
    local name = first_name_token(node)
    if name ~= nil then
        scope:define_global(name.value, {
            name = name.value,
            is_fn = true,
            fn_params = extract_params(node),
            fn_return_type = resolve_type(type_node(node)) or VOID,
            nib_type = resolve_type(type_node(node)) or VOID,
        })
    end
    return scope
end

local function infer_primary(node, scope)
    local token = tokens_in(node)[1]
    if token == nil then
        return nil
    end
    local ttype = token_type(token)
    if ttype == "INT_LIT" or ttype == "HEX_LIT" then
        return LITERAL
    elseif ttype == "true" or ttype == "false" or token.value == "true" or token.value == "false" then
        return BOOL
    elseif ttype == "NAME" then
        local symbol = scope:lookup(token.value)
        return symbol and symbol.nib_type or nil
    end
    return nil
end

local function check_expr(node, scope, state)
    if node == nil then
        return nil, state
    end

    if node.rule_name == "add_expr" then
        local operands = expression_children(node)
        if #operands >= 2 then
            local left_type
            left_type, state = check_expr(operands[1], scope, state)
            local right_type
            right_type, state = check_expr(operands[2], scope, state)
            local inferred = nil
            if left_type == LITERAL and numeric(right_type) then
                inferred = right_type
            elseif right_type == LITERAL and numeric(left_type) then
                inferred = left_type
            elseif left_type == LITERAL and right_type == LITERAL then
                inferred = LITERAL
            elseif left_type == right_type and numeric(left_type) then
                inferred = left_type
            else
                state = error(state, "binary expression type mismatch: " .. tostring(left_type) .. " vs " .. tostring(right_type), node)
            end
            return inferred, annotate(state, node, inferred)
        end
    elseif node.rule_name == "call_expr" then
        local name = first_name_token(node)
        local symbol = name and scope:lookup(name.value) or nil
        if symbol == nil or not symbol.is_fn then
            return nil, error(state, "unknown function `" .. tostring(name and name.value or "?") .. "`", name or node)
        end

        local arg_nodes = {}
        local arg_list = first_rule(node, "arg_list")
        if arg_list ~= nil then
            for _, child in ipairs(child_nodes(arg_list)) do
                if child.rule_name == "expr" then
                    arg_nodes[#arg_nodes + 1] = child
                end
            end
        end

        if #arg_nodes ~= #symbol.fn_params then
            state = error(state, "function `" .. name.value .. "` expects " .. #symbol.fn_params .. " args, got " .. #arg_nodes, node)
        else
            for index, param in ipairs(symbol.fn_params) do
                local actual
                actual, state = check_expr(arg_nodes[index], scope, state)
                if not compatible(param[2], actual) then
                    state = error(state, "argument `" .. param[1] .. "` expects " .. param[2] .. ", got " .. tostring(actual), arg_nodes[index])
                end
            end
        end

        return symbol.fn_return_type, annotate(state, node, symbol.fn_return_type)
    end

    local expr_child = expression_children(node)[1] or child_nodes(node)[1]
    local inferred = nil
    if expr_child ~= nil and expr_child ~= node then
        inferred, state = check_expr(expr_child, scope, state)
    else
        inferred = infer_primary(node, scope)
    end
    return inferred, annotate(state, node, inferred)
end

local function check_block(block, scope, state, return_type)
    for _, stmt in ipairs(child_nodes(block)) do
        local inner = stmt.rule_name == "stmt" and child_nodes(stmt)[1] or stmt
        if inner ~= nil then
            if inner.rule_name == "let_stmt" then
                local name = first_name_token(inner)
                local declared = resolve_type(type_node(inner))
                local expr = first_rule(inner, "expr")
                if name ~= nil and declared ~= nil and expr ~= nil then
                    local actual
                    actual, state = check_expr(expr, scope, state)
                    if not compatible(declared, actual) then
                        state = error(state, "let `" .. name.value .. "` expects " .. declared .. ", got " .. tostring(actual), expr)
                    end
                    scope:define_local(name.value, { name = name.value, nib_type = declared })
                end
            elseif inner.rule_name == "assign_stmt" then
                local name = first_name_token(inner)
                local expr = first_rule(inner, "expr")
                if name ~= nil and expr ~= nil then
                    local symbol = scope:lookup(name.value)
                    if symbol == nil then
                        state = error(state, "unknown variable `" .. name.value .. "`", name)
                    else
                        local actual
                        actual, state = check_expr(expr, scope, state)
                        if not compatible(symbol.nib_type, actual) then
                            state = error(state, "assignment to `" .. name.value .. "` expects " .. symbol.nib_type .. ", got " .. tostring(actual), expr)
                        end
                    end
                end
            elseif inner.rule_name == "return_stmt" then
                local expr = first_rule(inner, "expr")
                if expr ~= nil then
                    local actual
                    actual, state = check_expr(expr, scope, state)
                    if not compatible(return_type, actual) then
                        state = error(state, "return expects " .. return_type .. ", got " .. tostring(actual), expr)
                    end
                end
            elseif inner.rule_name == "for_stmt" then
                local name = first_name_token(inner)
                local declared = resolve_type(type_node(inner))
                local exprs = {}
                for _, child in ipairs(child_nodes(inner)) do
                    if child.rule_name == "expr" then
                        exprs[#exprs + 1] = child
                    end
                end
                local loop_block = first_rule(inner, "block")
                if name ~= nil and declared ~= nil and #exprs >= 2 and loop_block ~= nil then
                    local lower_type
                    lower_type, state = check_expr(exprs[1], scope, state)
                    local upper_type
                    upper_type, state = check_expr(exprs[2], scope, state)
                    if not (numericish(lower_type) and numericish(upper_type)) then
                        state = error(state, "for loop bounds must be numeric", inner)
                    end
                    scope:push()
                    scope:define_local(name.value, { name = name.value, nib_type = declared })
                    state = check_block(loop_block, scope, state, return_type)
                    scope:pop()
                end
            elseif inner.rule_name == "expr_stmt" then
                local expr = first_rule(inner, "expr")
                if expr ~= nil then
                    local _
                    _, state = check_expr(expr, scope, state)
                end
            end
        end
    end
    return state
end

function M.check(ast)
    local scope = ScopeChain.new()
    for _, top_decl in ipairs(child_nodes(ast)) do
        local decl = unwrap_top_decl(top_decl)
        if decl ~= nil then
            if decl.rule_name == "const_decl" then
                collect_const_or_static(decl, scope, true)
            elseif decl.rule_name == "static_decl" then
                collect_const_or_static(decl, scope, false)
            elseif decl.rule_name == "fn_decl" then
                collect_fn_signature(decl, scope)
            end
        end
    end

    local state = { errors = {}, types = {} }
    for _, top_decl in ipairs(child_nodes(ast)) do
        local decl = unwrap_top_decl(top_decl)
        if decl ~= nil and decl.rule_name == "fn_decl" then
            local name = first_name_token(decl)
            local symbol = name and scope:lookup(name.value) or nil
            local block = first_rule(decl, "block")
            if symbol ~= nil and block ~= nil then
                scope:push()
                for _, param in ipairs(symbol.fn_params) do
                    scope:define_local(param[1], { name = param[1], nib_type = param[2] })
                end
                state = check_block(block, scope, state, symbol.fn_return_type or VOID)
                scope:pop()
            end
        end
    end

    return protocol.new_type_check_result({
        root = ast,
        types = state.types,
        type_of = function(self, node)
            return self.types[node]
        end,
    }, state.errors)
end

function M.check_source(source)
    local ok, ast_or_err = pcall(nib_parser.parse, source)
    if not ok then
        return protocol.new_type_check_result({
            root = nil,
            types = {},
        }, { protocol.new_type_error_diagnostic(tostring(ast_or_err), 1, 1) })
    end
    return M.check(ast_or_err)
end

return M
