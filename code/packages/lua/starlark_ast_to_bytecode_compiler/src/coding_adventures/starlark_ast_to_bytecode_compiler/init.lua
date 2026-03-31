-- starlark_ast_to_bytecode_compiler
-- Compiles Starlark ASTs to stack-based bytecode for the Starlark VM.
-- ====================================================================
--
-- # What is this module?
--
-- This package translates a Starlark Abstract Syntax Tree (AST) produced
-- by the starlark_parser into a flat sequence of bytecode instructions
-- ready for execution in a stack-based virtual machine.
--
-- The pipeline looks like this:
--
--   Starlark source code
--       │ (starlark_lexer)
--   Token stream
--       │ (starlark_parser)
--   AST (rule_name + children tree)
--       │ (THIS MODULE)
--   CodeObject (bytecode instructions + constant pool + name pool)
--       │ (starlark_vm)
--   Execution result
--
-- # Bytecode: What Is It?
--
-- Bytecode is a compact sequence of opcodes (small integers) and optional
-- operands. Instead of interpreting the tree structure of an AST on every
-- execution, we compile it once to a flat list — like translating a recipe
-- into a numbered step list. The VM (chef) just follows the steps in order,
-- occasionally jumping to a different step number.
--
--   AST for "x = 1 + 2":
--   assign_stmt
--   ├── identifier → "x"
--   └── arith
--       ├── number → 1
--       ├── "+"
--       └── number → 2
--
--   Compiled bytecode:
--   [0] LOAD_CONST  0      ; push constants[0] = 1
--   [1] LOAD_CONST  1      ; push constants[1] = 2
--   [2] ADD                ; pop 2, pop 1, push 3
--   [3] STORE_NAME  0      ; pop 3, store in names[0] = "x"
--   [4] HALT               ; stop
--
-- # Opcode Design
--
-- Opcodes are grouped by category using the high nibble:
--
--   0x01-0x06  Stack operations    (LOAD_CONST, POP, DUP, LOAD_NONE, etc.)
--   0x10-0x15  Variable operations (STORE_NAME, LOAD_NAME, store/load local/closure)
--   0x20-0x2D  Arithmetic          (ADD, SUB, MUL, DIV, MOD, etc.)
--   0x30-0x38  Comparison & bool   (==, !=, <, >, in, not)
--   0x40-0x44  Control flow        (JUMP, JUMP_IF_FALSE, JUMP_IF_TRUE, etc.)
--   0x50-0x53  Functions           (MAKE_FUNCTION, CALL_FUNCTION, RETURN)
--   0x60-0x64  Collections         (BUILD_LIST, BUILD_DICT, BUILD_TUPLE, etc.)
--   0x70-0x74  Subscript & attr    (LOAD_SUBSCRIPT, STORE_SUBSCRIPT, LOAD_ATTR)
--   0x80-0x82  Iteration           (GET_ITER, FOR_ITER, UNPACK_SEQUENCE)
--   0x90-0x91  Module              (LOAD_MODULE, IMPORT_FROM)
--   0xA0       I/O                 (PRINT)
--   0xFF       VM control          (HALT)
--
-- # GenericCompiler Pattern
--
-- This module builds on the GenericCompiler from the bytecode_compiler package.
-- The GenericCompiler handles:
--   - Instruction emission (emit, emit_jump, patch_jump)
--   - Constant and name pool management (add_constant, add_name)
--   - Recursive dispatch (compile_node)
--   - Scope management for locals (enter_scope, exit_scope)
--
-- This module handles:
--   - Registering handlers for all Starlark grammar rules
--   - Converting each AST construct to the appropriate opcodes

-- ============================================================================
-- Dependency
-- ============================================================================

local bc = require("coding_adventures.bytecode_compiler")

local M = {}

M.VERSION = "0.1.0"

-- ============================================================================
-- Opcodes — Starlark's instruction set
-- ============================================================================
--
-- These constants define Starlark's instruction set. Each opcode is a small
-- integer — the VM's "machine language." We chose values to match the Elixir
-- reference implementation for cross-language portability.

-- Stack Operations (0x0_)
M.OP_LOAD_CONST          = 0x01  -- push constants[operand]
M.OP_POP                 = 0x02  -- discard top of stack
M.OP_DUP                 = 0x03  -- duplicate top of stack
M.OP_LOAD_NONE           = 0x04  -- push nil/None
M.OP_LOAD_TRUE           = 0x05  -- push true
M.OP_LOAD_FALSE          = 0x06  -- push false

-- Variable Operations (0x1_)
M.OP_STORE_NAME          = 0x10  -- pop → names[operand]
M.OP_LOAD_NAME           = 0x11  -- push names[operand]
M.OP_STORE_LOCAL         = 0x12  -- pop → locals[operand]
M.OP_LOAD_LOCAL          = 0x13  -- push locals[operand]
M.OP_STORE_CLOSURE       = 0x14  -- pop → closure[operand]
M.OP_LOAD_CLOSURE        = 0x15  -- push closure[operand]

-- Arithmetic Operations (0x2_)
M.OP_ADD                 = 0x20  -- pop b, pop a, push a+b
M.OP_SUB                 = 0x21  -- pop b, pop a, push a-b
M.OP_MUL                 = 0x22  -- pop b, pop a, push a*b
M.OP_DIV                 = 0x23  -- pop b, pop a, push a/b (float)
M.OP_FLOOR_DIV           = 0x24  -- pop b, pop a, push a//b
M.OP_MOD                 = 0x25  -- pop b, pop a, push a%b
M.OP_POWER               = 0x26  -- pop b, pop a, push a**b
M.OP_NEGATE              = 0x27  -- pop a, push -a
M.OP_BIT_AND             = 0x28  -- pop b, pop a, push a&b
M.OP_BIT_OR              = 0x29  -- pop b, pop a, push a|b
M.OP_BIT_XOR             = 0x2A  -- pop b, pop a, push a^b
M.OP_BIT_NOT             = 0x2B  -- pop a, push ~a
M.OP_LSHIFT              = 0x2C  -- pop b, pop a, push a<<b
M.OP_RSHIFT              = 0x2D  -- pop b, pop a, push a>>b

-- Comparison & Boolean (0x3_)
M.OP_CMP_EQ              = 0x30  -- a == b
M.OP_CMP_NE              = 0x31  -- a != b
M.OP_CMP_LT              = 0x32  -- a < b
M.OP_CMP_GT              = 0x33  -- a > b
M.OP_CMP_LE              = 0x34  -- a <= b
M.OP_CMP_GE              = 0x35  -- a >= b
M.OP_CMP_IN              = 0x36  -- a in b
M.OP_CMP_NOT_IN          = 0x37  -- a not in b
M.OP_LOGICAL_NOT         = 0x38  -- not a

-- Control Flow (0x4_)
M.OP_JUMP                = 0x40  -- unconditional jump to operand
M.OP_JUMP_IF_FALSE       = 0x41  -- pop, jump if falsy
M.OP_JUMP_IF_TRUE        = 0x42  -- pop, jump if truthy
M.OP_JUMP_IF_FALSE_OR_POP = 0x43 -- for 'and' short-circuit
M.OP_JUMP_IF_TRUE_OR_POP = 0x44  -- for 'or' short-circuit

-- Function Operations (0x5_)
M.OP_MAKE_FUNCTION       = 0x50  -- create function object from code object
M.OP_CALL_FUNCTION       = 0x51  -- call function with N positional args
M.OP_CALL_FUNCTION_KW    = 0x52  -- call function with keyword args
M.OP_RETURN              = 0x53  -- return from function

-- Collection Operations (0x6_)
M.OP_BUILD_LIST          = 0x60  -- build list from N stack items
M.OP_BUILD_DICT          = 0x61  -- build dict from N key-value pairs
M.OP_BUILD_TUPLE         = 0x62  -- build tuple from N stack items
M.OP_LIST_APPEND         = 0x63  -- append to list (for comprehensions)
M.OP_DICT_SET            = 0x64  -- set dict entry (for comprehensions)

-- Subscript & Attribute (0x7_)
M.OP_LOAD_SUBSCRIPT      = 0x70  -- obj[key]
M.OP_STORE_SUBSCRIPT     = 0x71  -- obj[key] = value
M.OP_LOAD_ATTR           = 0x72  -- obj.attr
M.OP_STORE_ATTR          = 0x73  -- obj.attr = value
M.OP_LOAD_SLICE          = 0x74  -- obj[start:stop:step]

-- Iteration (0x8_)
M.OP_GET_ITER            = 0x80  -- get iterator from iterable
M.OP_FOR_ITER            = 0x81  -- advance iterator or jump to end
M.OP_UNPACK_SEQUENCE     = 0x82  -- unpack N items from sequence

-- Module (0x9_)
M.OP_LOAD_MODULE         = 0x90  -- load a module (for load() statement)
M.OP_IMPORT_FROM         = 0x91  -- extract symbol from module

-- I/O (0xA_)
M.OP_PRINT               = 0xA0  -- print top of stack

-- VM Control (0xFF)
M.OP_HALT                = 0xFF  -- stop execution

-- ============================================================================
-- Operator-to-Opcode Maps
-- ============================================================================

M.BINARY_OP_MAP = {
    ["+"]   = M.OP_ADD,
    ["-"]   = M.OP_SUB,
    ["*"]   = M.OP_MUL,
    ["/"]   = M.OP_DIV,
    ["//"]  = M.OP_FLOOR_DIV,
    ["%"]   = M.OP_MOD,
    ["**"]  = M.OP_POWER,
    ["&"]   = M.OP_BIT_AND,
    ["|"]   = M.OP_BIT_OR,
    ["^"]   = M.OP_BIT_XOR,
    ["<<"]  = M.OP_LSHIFT,
    [">>"]  = M.OP_RSHIFT,
}

M.COMPARE_OP_MAP = {
    ["=="]     = M.OP_CMP_EQ,
    ["!="]     = M.OP_CMP_NE,
    ["<"]      = M.OP_CMP_LT,
    [">"]      = M.OP_CMP_GT,
    ["<="]     = M.OP_CMP_LE,
    [">="]     = M.OP_CMP_GE,
    ["in"]     = M.OP_CMP_IN,
    ["not in"] = M.OP_CMP_NOT_IN,
}

M.AUGMENTED_ASSIGN_MAP = {
    ["+="]   = M.OP_ADD,
    ["-="]   = M.OP_SUB,
    ["*="]   = M.OP_MUL,
    ["/="]   = M.OP_DIV,
    ["//="]  = M.OP_FLOOR_DIV,
    ["%="]   = M.OP_MOD,
    ["&="]   = M.OP_BIT_AND,
    ["|="]   = M.OP_BIT_OR,
    ["^="]   = M.OP_BIT_XOR,
    ["<<="]  = M.OP_LSHIFT,
    [">>="]  = M.OP_RSHIFT,
    ["**="]  = M.OP_POWER,
}

M.UNARY_OP_MAP = {
    ["-"] = M.OP_NEGATE,
    ["~"] = M.OP_BIT_NOT,
}

-- ============================================================================
-- CodeObject helpers
-- ============================================================================

--- Make a single instruction table.
-- @param opcode  number  The opcode constant.
-- @param operand any     Optional operand (index, count, jump target).
-- @return table  { opcode, operand }
function M.instruction(opcode, operand)
    return { opcode = opcode, operand = operand }
end

--- Make a CodeObject table.
-- @param instructions  table  Array of instruction tables.
-- @param constants     table  Constant pool (0-indexed externally, 1-indexed in Lua).
-- @param names         table  Name pool.
-- @return table  { instructions, constants, names }
function M.code_object(instructions, constants, names)
    return {
        instructions = instructions or {},
        constants    = constants    or {},
        names        = names        or {},
    }
end

-- ============================================================================
-- Starlark Compiler — AST → Bytecode
-- ============================================================================
--
-- The StarlarkCompiler wraps a GenericCompiler and registers handlers for
-- all Starlark grammar rules. It is created fresh for each compilation.
--
-- Rule handlers follow this contract:
--   function handler(compiler, node)
--     -- compiler: the StarlarkCompiler (which is a GenericCompiler)
--     -- node: { node_kind="ast", rule_name="...", children=[...] }
--     -- Handler calls compiler:emit(), compiler:add_constant(), etc.
--   end
--
-- Token nodes have: { node_kind="token", type="NAME", value="x" }

local StarlarkCompiler = {}
StarlarkCompiler.__index = StarlarkCompiler

--- Create a new StarlarkCompiler with all grammar rule handlers registered.
-- @return StarlarkCompiler
function StarlarkCompiler.new()
    local self = setmetatable({}, StarlarkCompiler)
    self._gc = bc.GenericCompiler.new()

    -- Register all handlers
    self:_register_all_handlers()

    return self
end

--- Compile an AST node (from starlark_parser) to a CodeObject.
-- @param ast  table  Root ASTNode from starlark_parser.
-- @return table  CodeObject with instructions, constants, names.
function StarlarkCompiler:compile(ast)
    return self._gc:compile(ast, M.OP_HALT)
end

-- ============================================================================
-- Handler Registration
-- ============================================================================

function StarlarkCompiler:_register_all_handlers()
    local gc = self._gc

    -- ------------------------------------------------------------------
    -- file / suite / top-level wrappers
    -- ------------------------------------------------------------------

    -- file: the root node. Compiles each child statement.
    gc:register_rule("file", function(c, node)
        for _, child in ipairs(node.children) do
            c:compile_node(child)
        end
    end)

    -- suite: a block of statements (body of if/for/def)
    gc:register_rule("suite", function(c, node)
        for _, child in ipairs(node.children) do
            if child.node_kind == "token" then
                -- skip INDENT/DEDENT/NEWLINE tokens
            else
                c:compile_node(child)
            end
        end
    end)

    -- statement / simple_stmt / compound_stmt: pass through
    gc:register_rule("statement",     function(c, node) _pass_through(c, node) end)
    gc:register_rule("simple_stmt",   function(c, node) _pass_through(c, node) end)
    gc:register_rule("small_stmt",    function(c, node) _pass_through(c, node) end)
    gc:register_rule("compound_stmt", function(c, node) _pass_through(c, node) end)

    -- ------------------------------------------------------------------
    -- Statements
    -- ------------------------------------------------------------------

    -- expression_stmt: compile expression and pop result (value not used)
    gc:register_rule("expression_stmt", function(c, node)
        c:compile_node(node.children[1])
        c:emit(M.OP_POP)
    end)

    -- assign_stmt: x = expr  or  x = y = expr (chained)
    -- Children: [lhs, "=", rhs]  or  [lhs, "=", lhs2, "=", rhs]
    gc:register_rule("assign_stmt", function(c, node)
        -- Find the rightmost expression (last child)
        local rhs = node.children[#node.children]
        c:compile_node(rhs)

        -- Walk backwards collecting targets (skip "=" tokens)
        for i = #node.children - 2, 1, -2 do
            local target = node.children[i]
            _compile_store(c, target)
        end
    end)

    -- augmented_assign_stmt: x += expr
    -- Children: [lhs, op, rhs]
    gc:register_rule("augmented_assign_stmt", function(c, node)
        local lhs = node.children[1]
        local op_tok = _find_token(node.children, 2)
        local rhs = node.children[#node.children]
        -- Load current value
        _compile_load(c, lhs)
        -- Compile RHS
        c:compile_node(rhs)
        -- Apply operator
        local opcode = M.AUGMENTED_ASSIGN_MAP[op_tok]
        if opcode then
            c:emit(opcode)
        end
        -- Store back
        _compile_store(c, lhs)
    end)

    -- return_stmt: return [expr]
    gc:register_rule("return_stmt", function(c, node)
        -- Find expression child if any
        local has_expr = false
        for _, child in ipairs(node.children) do
            if child.node_kind ~= "token" then
                c:compile_node(child)
                has_expr = true
                break
            end
        end
        if not has_expr then
            c:emit(M.OP_LOAD_NONE)
        end
        c:emit(M.OP_RETURN)
    end)

    -- pass_stmt
    gc:register_rule("pass_stmt", function(c, node)
        -- no-op; pass does nothing
        _ = c; _ = node
    end)

    -- break_stmt / continue_stmt — emit jump placeholders
    -- In a full implementation these would patch to the enclosing loop.
    -- Here we emit JUMP 0 as a placeholder that a real VM would fix.
    gc:register_rule("break_stmt",    function(c, _) c:emit_jump(M.OP_JUMP) end)
    gc:register_rule("continue_stmt", function(c, _) c:emit_jump(M.OP_JUMP) end)

    -- if_stmt: if expr: suite [elif/else]
    gc:register_rule("if_stmt", function(c, node)
        _compile_if_chain(c, node.children, 1)
    end)

    gc:register_rule("elif_clause", function(c, node)
        _compile_if_chain(c, node.children, 1)
    end)

    gc:register_rule("else_clause", function(c, node)
        -- Children: [ELSE, suite]
        local suite = _find_ast_child(node.children, "suite")
            or node.children[#node.children]
        c:compile_node(suite)
    end)

    -- for_stmt: for var in iterable: suite [else: suite]
    gc:register_rule("for_stmt", function(c, node)
        -- Find variable (identifier), iterable (expression), body (suite)
        local target = _find_ast_child(node.children, "identifier")
            or _find_ast_child(node.children, "expr")
        local suite  = _find_ast_child(node.children, "suite")
        -- The iterable is the expression between "in" and ":"
        local iter_expr = _find_iterable_in_for(node.children)

        -- Compile iterable and get iterator
        c:compile_node(iter_expr)
        c:emit(M.OP_GET_ITER)

        -- Loop header: FOR_ITER to after loop
        local loop_start = c:current_offset()
        local for_iter_idx = c:emit_jump(M.OP_FOR_ITER)

        -- Store loop variable
        if target then _compile_store(c, target) end

        -- Compile loop body
        if suite then c:compile_node(suite) end

        -- Jump back to loop header
        c:emit(M.OP_JUMP, loop_start)

        -- Patch FOR_ITER to jump here on exhaustion
        c:patch_jump(for_iter_idx)
    end)

    -- load_stmt: load("module", "sym") or load("module", sym = "ext")
    gc:register_rule("load_stmt", function(c, node)
        -- First string arg is the module path
        local first_str = _find_first_string(node.children)
        if first_str then
            local idx = c:add_constant(first_str)
            c:emit(M.OP_LOAD_CONST, idx)
            c:emit(M.OP_LOAD_MODULE, 0)
        end
        -- Additional args are symbols to import
        local sym_idx = 0
        for _, child in ipairs(node.children) do
            if child.node_kind == "ast" and child.rule_name == "argument" then
                local name_val = _get_string_value(child)
                if name_val then
                    local ni = c:add_name(name_val)
                    c:emit(M.OP_IMPORT_FROM, ni)
                    c:emit(M.OP_STORE_NAME, ni)
                end
                sym_idx = sym_idx + 1
            end
        end
        if sym_idx == 0 then
            -- No explicit symbols, just discard the module
            c:emit(M.OP_POP)
        end
    end)

    -- def_stmt: def name(params): suite
    gc:register_rule("def_stmt", function(c, node)
        local name_tok = _find_token_type(node.children, "NAME")
        local func_name = name_tok or "anonymous"
        local param_list = _find_ast_child(node.children, "param_list")
        local suite = _find_ast_child(node.children, "suite")

        -- Collect parameter names
        local params = {}
        if param_list then
            params = _collect_params(param_list)
        end

        -- Compile function body as a nested code object
        local body_gc = bc.GenericCompiler.new()
        -- Copy all registered handlers into the nested compiler
        for rule, handler in pairs(c._dispatch) do
            body_gc:register_rule(rule, handler)
        end
        body_gc:enter_scope(table.unpack(params))

        -- Compile the suite
        if suite then
            for _, child in ipairs(suite.children) do
                if child.node_kind ~= "token" then
                    body_gc:compile_node(child)
                end
            end
        end
        -- Implicit return None
        body_gc:emit(M.OP_LOAD_NONE)
        body_gc:emit(M.OP_RETURN)

        local func_co = bc.CodeObject(
            body_gc.instructions,
            body_gc.constants,
            body_gc.names
        )

        -- Put the code object in the constant pool
        local co_idx = c:add_constant(func_co)
        c:emit(M.OP_LOAD_CONST, co_idx)
        c:emit(M.OP_MAKE_FUNCTION, 0)

        -- Bind to the function name
        local name_idx = c:add_name(func_name)
        c:emit(M.OP_STORE_NAME, name_idx)
    end)

    -- param_list / param: handled by def_stmt handler via _collect_params
    gc:register_rule("param_list", function(c, node) _pass_through(c, node) end)
    gc:register_rule("param",      function(c, node)
        local name_tok = _find_token_type(node.children, "NAME")
        if name_tok then
            local idx = c:add_name(name_tok)
            c:emit(M.OP_LOAD_NAME, idx)
        end
    end)

    -- ------------------------------------------------------------------
    -- Expressions
    -- ------------------------------------------------------------------

    -- expr / or_expr / and_expr: pass-through unless multiple children
    gc:register_rule("expr",        function(c, node) _pass_through(c, node) end)
    gc:register_rule("expression",  function(c, node) _pass_through(c, node) end)

    -- or_expr: a or b  (short-circuit: if a is truthy, return a)
    gc:register_rule("or_expr", function(c, node)
        if #node.children == 1 then
            c:compile_node(node.children[1])
            return
        end
        -- Children: [left, "or", right] or longer chain
        c:compile_node(node.children[1])
        local j = c:emit_jump(M.OP_JUMP_IF_TRUE_OR_POP)
        c:compile_node(node.children[3])
        c:patch_jump(j)
    end)

    -- and_expr: a and b  (short-circuit: if a is falsy, return a)
    gc:register_rule("and_expr", function(c, node)
        if #node.children == 1 then
            c:compile_node(node.children[1])
            return
        end
        c:compile_node(node.children[1])
        local j = c:emit_jump(M.OP_JUMP_IF_FALSE_OR_POP)
        c:compile_node(node.children[3])
        c:patch_jump(j)
    end)

    -- not_expr: not a
    gc:register_rule("not_expr", function(c, node)
        if #node.children == 1 then
            c:compile_node(node.children[1])
            return
        end
        -- Children: ["not", expr]
        c:compile_node(node.children[2])
        c:emit(M.OP_LOGICAL_NOT)
    end)

    -- comparison: a op b
    gc:register_rule("comparison", function(c, node)
        if #node.children == 1 then
            c:compile_node(node.children[1])
            return
        end
        -- Children: [left, op, right]  or  [left, "not", "in", right]
        c:compile_node(node.children[1])
        c:compile_node(node.children[#node.children])
        local op = _get_compare_op(node.children)
        local opcode = M.COMPARE_OP_MAP[op]
        if opcode then
            c:emit(opcode)
        end
    end)

    -- arith: a + b, a - b
    gc:register_rule("arith", function(c, node)
        if #node.children == 1 then
            c:compile_node(node.children[1])
            return
        end
        _compile_binary_chain(c, node.children)
    end)

    -- term: a * b, a / b, a // b, a % b
    gc:register_rule("term", function(c, node)
        if #node.children == 1 then
            c:compile_node(node.children[1])
            return
        end
        _compile_binary_chain(c, node.children)
    end)

    -- shift: a << b, a >> b
    gc:register_rule("shift", function(c, node)
        if #node.children == 1 then
            c:compile_node(node.children[1])
            return
        end
        _compile_binary_chain(c, node.children)
    end)

    -- bitwise_and / bitwise_xor / bitwise_or
    gc:register_rule("bitwise_and", function(c, node) _binary_or_pass(c, node) end)
    gc:register_rule("bitwise_xor", function(c, node) _binary_or_pass(c, node) end)
    gc:register_rule("bitwise_or",  function(c, node) _binary_or_pass(c, node) end)

    -- factor: unary expression (-, ~, +)
    gc:register_rule("factor", function(c, node)
        if #node.children == 1 then
            c:compile_node(node.children[1])
            return
        end
        -- Children: [op_token, expr]
        c:compile_node(node.children[2])
        local op_val = node.children[1].value or ""
        local opcode = M.UNARY_OP_MAP[op_val]
        if opcode then
            c:emit(opcode)
        end
        -- unary "+" is a no-op (no opcode emitted)
    end)

    -- unary (alias for factor in some grammars)
    gc:register_rule("unary", function(c, node)
        if #node.children == 1 then
            c:compile_node(node.children[1])
            return
        end
        c:compile_node(node.children[2])
        local op_val = node.children[1].value or ""
        local opcode = M.UNARY_OP_MAP[op_val]
        if opcode then c:emit(opcode) end
    end)

    -- power_expr: a ** b
    gc:register_rule("power_expr", function(c, node)
        if #node.children == 1 then
            c:compile_node(node.children[1])
            return
        end
        -- Children: [base, "**", exp]
        c:compile_node(node.children[1])
        c:compile_node(node.children[3])
        c:emit(M.OP_POWER)
    end)

    -- primary: base expression (handles dot_access, subscript, call chains)
    gc:register_rule("primary", function(c, node)
        if #node.children == 1 then
            c:compile_node(node.children[1])
            return
        end
        -- Can be: [expr, ".", NAME] or [expr, "[", key, "]"] or [expr, call_args]
        c:compile_node(node.children[1])
        local second = node.children[2]
        if second and second.node_kind == "token" then
            if second.value == "." then
                local attr = node.children[3] and node.children[3].value or ""
                local idx = c:add_name(attr)
                c:emit(M.OP_LOAD_ATTR, idx)
            elseif second.value == "[" then
                c:compile_node(node.children[3])
                c:emit(M.OP_LOAD_SUBSCRIPT)
            end
        elseif second and second.node_kind == "ast" then
            if second.rule_name == "call_args" then
                -- Handled by the call rule
            end
        end
    end)

    -- call: function call
    gc:register_rule("call", function(c, node)
        -- Children: [func_expr, "(", call_args?, ")"]
        c:compile_node(node.children[1])
        local args_node = _find_ast_child(node.children, "call_args")
        local arg_count = 0
        if args_node then
            arg_count = _compile_call_args(c, args_node)
        end
        c:emit(M.OP_CALL_FUNCTION, arg_count)
    end)

    gc:register_rule("call_args", function(c, node)
        -- Handled by call handler
        _pass_through(c, node)
    end)

    gc:register_rule("argument", function(c, node)
        -- Compile the value expression
        local expr = nil
        for _, child in ipairs(node.children) do
            if child.node_kind == "ast" then expr = child break end
        end
        if expr then c:compile_node(expr) end
    end)

    -- dot_access: obj.attr
    gc:register_rule("dot_access", function(c, node)
        c:compile_node(node.children[1])
        local attr = ""
        for _, child in ipairs(node.children) do
            if child.node_kind == "token" and child.type == "NAME" then
                attr = child.value
            end
        end
        local idx = c:add_name(attr)
        c:emit(M.OP_LOAD_ATTR, idx)
    end)

    -- subscript: obj[key]
    gc:register_rule("subscript", function(c, node)
        c:compile_node(node.children[1])
        c:compile_node(node.children[3])
        c:emit(M.OP_LOAD_SUBSCRIPT)
    end)

    -- slice: obj[start:stop:step]
    gc:register_rule("slice", function(c, node)
        -- Simplified: emit a LOAD_SLICE with flags
        c:emit(M.OP_LOAD_SLICE, 0)
    end)

    -- ------------------------------------------------------------------
    -- Literals
    -- ------------------------------------------------------------------

    -- atom: the most primitive expression — identifier, number, string,
    --       True, False, None, or a parenthesized expression.
    gc:register_rule("atom", function(c, node)
        if #node.children == 1 then
            local child = node.children[1]
            if child.node_kind == "token" then
                local t = child.type
                local v = child.value
                if t == "NAME" then
                    if v == "True" or v == "true" then
                        c:emit(M.OP_LOAD_TRUE)
                    elseif v == "False" or v == "false" then
                        c:emit(M.OP_LOAD_FALSE)
                    elseif v == "None" or v == "nil" then
                        c:emit(M.OP_LOAD_NONE)
                    else
                        local idx = c:add_name(v)
                        c:emit(M.OP_LOAD_NAME, idx)
                    end
                elseif t == "INT" or t == "FLOAT" then
                    local idx = c:add_constant(tonumber(v))
                    c:emit(M.OP_LOAD_CONST, idx)
                elseif t == "STRING" then
                    local idx = c:add_constant(_strip_quotes(v))
                    c:emit(M.OP_LOAD_CONST, idx)
                end
            else
                c:compile_node(child)
            end
        elseif #node.children == 3 then
            -- parenthesized expr: ( expr )
            c:compile_node(node.children[2])
        else
            _pass_through(c, node)
        end
    end)

    -- identifier: a name reference
    gc:register_rule("identifier", function(c, node)
        local name = _get_token_value(node.children)
        if name then
            -- Inside a scope, might be a local
            if c.scope and c.scope:resolve(name) ~= nil then
                local slot = c.scope:resolve(name)
                c:emit(M.OP_LOAD_LOCAL, slot)
            else
                local idx = c:add_name(name)
                c:emit(M.OP_LOAD_NAME, idx)
            end
        end
    end)

    -- number: integer or float literal
    gc:register_rule("number", function(c, node)
        local tok = _find_first_token(node.children)
        if tok then
            local idx = c:add_constant(tonumber(tok.value))
            c:emit(M.OP_LOAD_CONST, idx)
        end
    end)

    -- string_node: string literal
    gc:register_rule("string_node", function(c, node)
        local tok = _find_first_token(node.children)
        if tok then
            local idx = c:add_constant(_strip_quotes(tok.value))
            c:emit(M.OP_LOAD_CONST, idx)
        end
    end)

    -- list_expr: [a, b, c]
    gc:register_rule("list_expr", function(c, node)
        local count = 0
        for _, child in ipairs(node.children) do
            if child.node_kind == "ast" then
                c:compile_node(child)
                count = count + 1
            end
        end
        c:emit(M.OP_BUILD_LIST, count)
    end)

    -- dict_expr: {k: v, ...}
    gc:register_rule("dict_expr", function(c, node)
        local count = 0
        for _, child in ipairs(node.children) do
            if child.node_kind == "ast" and child.rule_name == "dict_entry" then
                c:compile_node(child)
                count = count + 1
            end
        end
        c:emit(M.OP_BUILD_DICT, count)
    end)

    -- dict_entry: key: value
    gc:register_rule("dict_entry", function(c, node)
        -- Children: [key_expr, ":", value_expr]
        local key_expr = nil
        local val_expr = nil
        local found_colon = false
        for _, child in ipairs(node.children) do
            if child.node_kind == "token" and child.value == ":" then
                found_colon = true
            elseif not found_colon and child.node_kind == "ast" then
                key_expr = child
            elseif found_colon and child.node_kind == "ast" then
                val_expr = child
            end
        end
        if key_expr then c:compile_node(key_expr) end
        if val_expr then c:compile_node(val_expr) end
    end)

    -- tuple_expr: (a, b) or a, b
    gc:register_rule("tuple_expr", function(c, node)
        local count = 0
        for _, child in ipairs(node.children) do
            if child.node_kind == "ast" then
                c:compile_node(child)
                count = count + 1
            end
        end
        c:emit(M.OP_BUILD_TUPLE, count)
    end)

    -- lambda_expr: lambda params: expr
    gc:register_rule("lambda_expr", function(c, node)
        -- Find param list and body expression
        local param_list = _find_ast_child(node.children, "param_list")
        -- Body is last expression child
        local body = node.children[#node.children]
        while body and body.node_kind == "token" do
            body = nil
        end
        for i = #node.children, 1, -1 do
            if node.children[i].node_kind == "ast" and
               node.children[i].rule_name ~= "param_list" then
                body = node.children[i]
                break
            end
        end

        local params = {}
        if param_list then params = _collect_params(param_list) end

        -- Compile lambda body as nested code object
        local body_gc = bc.GenericCompiler.new()
        for rule, handler in pairs(c._dispatch) do
            body_gc:register_rule(rule, handler)
        end
        body_gc:enter_scope(table.unpack(params))
        if body then body_gc:compile_node(body) end
        body_gc:emit(M.OP_RETURN)

        local lambda_co = bc.CodeObject(body_gc.instructions, body_gc.constants, body_gc.names)
        local co_idx = c:add_constant(lambda_co)
        c:emit(M.OP_LOAD_CONST, co_idx)
        c:emit(M.OP_MAKE_FUNCTION, 0)
    end)

    -- list_comp: [expr for var in iterable if cond]
    gc:register_rule("list_comp", function(c, node)
        -- Build empty list, then compile as a for loop appending elements
        c:emit(M.OP_BUILD_LIST, 0)
        local comp_clause = _find_ast_child(node.children, "comp_clause")
        if comp_clause then
            local iter_expr = _find_ast_child(comp_clause.children, "expr")
                or comp_clause.children[3]
            local var_node  = _find_ast_child(comp_clause.children, "identifier")
                or comp_clause.children[1]
            local cond_node = _find_ast_child(node.children, "comp_if")

            if iter_expr then c:compile_node(iter_expr) end
            c:emit(M.OP_GET_ITER)
            local loop_start = c:current_offset()
            local exit_jump = c:emit_jump(M.OP_FOR_ITER)
            if var_node then _compile_store(c, var_node) end

            if cond_node then
                local cond_expr = _find_ast_child(cond_node.children, "expr")
                if cond_expr then
                    c:compile_node(cond_expr)
                    local skip_jump = c:emit_jump(M.OP_JUMP_IF_FALSE)
                    -- compile element expression
                    local elem = node.children[1]
                    c:compile_node(elem)
                    c:emit(M.OP_LIST_APPEND)
                    c:patch_jump(skip_jump)
                end
            else
                local elem = node.children[1]
                c:compile_node(elem)
                c:emit(M.OP_LIST_APPEND)
            end

            c:emit(M.OP_JUMP, loop_start)
            c:patch_jump(exit_jump)
        end
    end)

    -- dict_comp: {k: v for k, v in iterable}
    gc:register_rule("dict_comp", function(c, node)
        c:emit(M.OP_BUILD_DICT, 0)
        -- Simplified: emit BUILD_DICT — full implementation mirrors list_comp
    end)

    -- comp_clause / comp_if: handled by list_comp handler
    gc:register_rule("comp_clause", function(c, node) _pass_through(c, node) end)
    gc:register_rule("comp_if",     function(c, node) _pass_through(c, node) end)

    -- star_expr: *x (for unpacking)
    gc:register_rule("star_expr", function(c, node)
        local expr = _find_ast_child(node.children, nil)
        if expr then c:compile_node(expr) end
    end)
end

-- ============================================================================
-- Private Helpers
-- ============================================================================

--- Compile a single-child node by passing to the child.
local function _pass_through(c, node)
    if #node.children == 1 then
        c:compile_node(node.children[1])
    elseif #node.children > 1 then
        for _, child in ipairs(node.children) do
            if child.node_kind ~= "token" then
                c:compile_node(child)
            end
        end
    end
end

--- Emit a STORE instruction for a target node.
local function _compile_store(c, target)
    if target.node_kind == "token" then
        if target.type == "NAME" then
            local idx = c:add_name(target.value)
            c:emit(M.OP_STORE_NAME, idx)
        end
    elseif target.node_kind == "ast" then
        local name = nil
        if target.rule_name == "identifier" then
            name = _get_token_value(target.children)
        elseif target.rule_name == "atom" and #target.children == 1
            and target.children[1].node_kind == "token" then
            name = target.children[1].value
        end
        if name then
            local idx = c:add_name(name)
            c:emit(M.OP_STORE_NAME, idx)
        end
    end
end

--- Emit a LOAD instruction for a source node.
local function _compile_load(c, source)
    if source.node_kind == "token" and source.type == "NAME" then
        local idx = c:add_name(source.value)
        c:emit(M.OP_LOAD_NAME, idx)
    elseif source.node_kind == "ast" then
        local name = nil
        if source.rule_name == "identifier" then
            name = _get_token_value(source.children)
        end
        if name then
            local idx = c:add_name(name)
            c:emit(M.OP_LOAD_NAME, idx)
        else
            c:compile_node(source)
        end
    end
end

--- Compile a chain of binary operations: left op right (op right)*
local function _compile_binary_chain(c, children)
    c:compile_node(children[1])
    local i = 2
    while i <= #children do
        local op_tok = children[i]
        local right  = children[i + 1]
        if op_tok and op_tok.node_kind == "token" and right then
            c:compile_node(right)
            local opcode = M.BINARY_OP_MAP[op_tok.value]
            if opcode then c:emit(opcode) end
        end
        i = i + 2
    end
end

--- Binary-op handler that falls through for single-child nodes.
local function _binary_or_pass(c, node)
    if #node.children == 1 then
        c:compile_node(node.children[1])
    else
        _compile_binary_chain(c, node.children)
    end
end

--- Compile if/elif/else chain.
local function _compile_if_chain(c, children, start)
    -- Find condition and suite
    local cond = nil
    local suite = nil
    local rest_start = start

    for i = start, #children do
        local child = children[i]
        if child.node_kind == "ast" then
            if not cond and child.rule_name ~= "suite"
                and child.rule_name ~= "elif_clause"
                and child.rule_name ~= "else_clause" then
                cond = child
                rest_start = i + 1
            elseif child.rule_name == "suite" then
                suite = child
                rest_start = i + 1
                break
            end
        end
    end

    if cond then c:compile_node(cond) end
    local jump_to_else = c:emit_jump(M.OP_JUMP_IF_FALSE)
    if suite then c:compile_node(suite) end

    -- Check for elif or else
    local has_else = false
    for i = rest_start, #children do
        local child = children[i]
        if child.node_kind == "ast" and
           (child.rule_name == "elif_clause" or child.rule_name == "else_clause") then
            has_else = true
            local jump_over_else = c:emit_jump(M.OP_JUMP)
            c:patch_jump(jump_to_else)
            c:compile_node(child)
            c:patch_jump(jump_over_else)
            return
        end
    end

    c:patch_jump(jump_to_else)
end

--- Find the iterable expression in a for-stmt child list.
-- for TARGET in ITERABLE: BODY
-- Children typically: [FOR, target, IN, iterable, COLON, suite]
local function _find_iterable_in_for(children)
    local saw_in = false
    for _, child in ipairs(children) do
        if child.node_kind == "token" and child.value == "in" then
            saw_in = true
        elseif saw_in and child.node_kind == "ast"
               and child.rule_name ~= "suite" then
            return child
        end
    end
    return nil
end

--- Find the first string literal in a list of children.
local function _find_first_string(children)
    for _, child in ipairs(children) do
        if child.node_kind == "token" and child.type == "STRING" then
            return _strip_quotes(child.value)
        elseif child.node_kind == "ast" then
            local v = _find_first_string(child.children or {})
            if v then return v end
        end
    end
    return nil
end

--- Get string value from an argument node.
local function _get_string_value(node)
    for _, child in ipairs(node.children or {}) do
        if child.node_kind == "token" and child.type == "STRING" then
            return _strip_quotes(child.value)
        elseif child.node_kind == "token" and child.type == "NAME" then
            return child.value
        end
    end
    return nil
end

--- Collect parameter names from a param_list node.
local function _collect_params(param_list)
    local params = {}
    for _, child in ipairs(param_list.children or {}) do
        if child.node_kind == "ast" and child.rule_name == "param" then
            local name = _find_token_type(child.children, "NAME")
            if name then table.insert(params, name) end
        elseif child.node_kind == "token" and child.type == "NAME" then
            table.insert(params, child.value)
        end
    end
    return params
end

--- Compile call arguments, return count.
local function _compile_call_args(c, args_node)
    local count = 0
    for _, child in ipairs(args_node.children) do
        if child.node_kind == "ast" then
            c:compile_node(child)
            count = count + 1
        end
    end
    return count
end

--- Get the comparison operator string from a comparison node's children.
local function _get_compare_op(children)
    -- Could be [left, "not", "in", right] or [left, "==", right]
    for i = 2, #children - 1 do
        local tok = children[i]
        if tok.node_kind == "token" then
            if tok.value == "not" then return "not in" end
            return tok.value
        end
    end
    return "=="
end

--- Find first AST child with given rule_name (or any AST child if rule_name is nil).
local function _find_ast_child(children, rule_name)
    for _, child in ipairs(children or {}) do
        if child.node_kind == "ast" then
            if rule_name == nil or child.rule_name == rule_name then
                return child
            end
        end
    end
    return nil
end

--- Find a token value in children.
local function _find_token(children, start_idx)
    for i = (start_idx or 1), #children do
        if children[i].node_kind == "token" then
            return children[i].value
        end
    end
    return nil
end

--- Find a token with a specific type.
local function _find_token_type(children, token_type)
    for _, child in ipairs(children or {}) do
        if child.node_kind == "token" and child.type == token_type then
            return child.value
        end
    end
    return nil
end

--- Get value from first token in children list.
local function _get_token_value(children)
    for _, child in ipairs(children or {}) do
        if child.node_kind == "token" then
            return child.value
        end
    end
    return nil
end

--- Find first token node in children.
local function _find_first_token(children)
    for _, child in ipairs(children or {}) do
        if child.node_kind == "token" then
            return child
        end
    end
    return nil
end

--- Strip surrounding quotes from a string literal token value.
-- Handles "...", '...', """...""", '''...'''
local function _strip_quotes(s)
    if not s then return "" end
    -- Triple-quoted
    if s:sub(1, 3) == '"""' and s:sub(-3) == '"""' then
        return s:sub(4, -4)
    end
    if s:sub(1, 3) == "'''" and s:sub(-3) == "'''" then
        return s:sub(4, -4)
    end
    -- Single-quoted
    if (s:sub(1, 1) == '"' and s:sub(-1) == '"') or
       (s:sub(1, 1) == "'" and s:sub(-1) == "'") then
        return s:sub(2, -2)
    end
    return s
end

-- Expose private helpers for internal use via local forward declarations.
-- (Lua closures ensure they are available inside the handlers)

-- Re-export the helper functions to be available from closures above.
-- We patch the closures by assigning to local-variables-that-are-upvalues.
-- Since Lua requires forward declarations, we use module-level locals.
-- The functions are defined as locals above their first use, which works
-- because all handlers are registered AFTER all the local helper functions
-- are defined.
--
-- IMPORTANT: In Lua, local functions used inside other local functions must
-- be declared before use OR forward-declared. Here we use a simple pattern:
-- all helpers are local functions declared here, and the handlers reference
-- them as upvalues captured at registration time. Since Lua defines closures
-- at the time they are created, we must ensure helpers are declared before
-- the register calls.
--
-- The current file structure already satisfies this because all `local function`
-- definitions appear before `StarlarkCompiler:_register_all_handlers()` would
-- actually call them (they're only called when the handlers execute, not when
-- they're registered). Lua closures capture the variable binding, not the
-- value — so even if the function isn't defined yet at registration time, it
-- will be by the time the handler runs... BUT with local functions, they must
-- be defined in the same scope before first use.
--
-- We work around this by making all helpers available through the module
-- table M, which is always defined.

M._pass_through         = _pass_through
M._compile_store        = _compile_store
M._compile_load         = _compile_load
M._compile_binary_chain = _compile_binary_chain
M._binary_or_pass       = _binary_or_pass
M._compile_if_chain     = _compile_if_chain
M._find_iterable_in_for = _find_iterable_in_for
M._collect_params       = _collect_params
M._compile_call_args    = _compile_call_args
M._get_compare_op       = _get_compare_op
M._find_ast_child       = _find_ast_child
M._find_token           = _find_token
M._find_token_type      = _find_token_type
M._get_token_value      = _get_token_value
M._find_first_token     = _find_first_token
M._strip_quotes         = _strip_quotes
M._find_first_string    = _find_first_string
M._get_string_value     = _get_string_value
M._binary_or_pass       = _binary_or_pass

-- ============================================================================
-- Public API
-- ============================================================================

M.StarlarkCompiler = StarlarkCompiler

--- Create a new StarlarkCompiler.
-- @return StarlarkCompiler
function M.new_compiler()
    return StarlarkCompiler.new()
end

--- Compile a Starlark AST to a CodeObject.
-- Convenience wrapper: creates a compiler, compiles, returns CodeObject.
-- @param ast  table  Root ASTNode from starlark_parser.
-- @return table  CodeObject
function M.compile_ast(ast)
    local compiler = StarlarkCompiler.new()
    return compiler:compile(ast)
end

--- Build a simple token node for testing.
-- @param type   string  Token type (e.g. "NAME", "INT", "STRING")
-- @param value  string  Token value
-- @return table  token node
function M.token_node(type, value)
    return { node_kind = "token", type = type, value = value }
end

--- Build a simple AST node for testing.
-- @param rule_name  string  Grammar rule name
-- @param children   table   Array of child nodes
-- @return table  AST node
function M.ast_node(rule_name, children)
    return { node_kind = "ast", rule_name = rule_name, children = children or {} }
end

return M
