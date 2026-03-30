-- coding_adventures.lattice_ast_to_css — Lattice AST → CSS compiler
-- ====================================================================
--
-- This module walks a Lattice AST (produced by `lattice_parser`) and
-- emits CSS text.  It is the semantic core of the Lattice transpiler.
--
-- # What is Lattice?
--
-- Lattice is a CSS superset language.  Every valid CSS file is valid
-- Lattice.  On top of CSS3, Lattice adds:
--
--   Variables:     $primary: #4a90d9;   color: $primary;
--   Mixins:        @mixin button($bg) { background: $bg; }
--                  .btn { @include button(red); }
--   Control flow:  @if $debug { color: red; }
--                  @for $i from 1 through 3 { .col-#{$i} { ... } }
--                  @each $c in red, blue { .t { color: $c; } }
--   Functions:     @function spacing($n) { @return $n * 8px; }
--   Nesting:       .parent { .child { color: blue; } }
--   Modules:       @use "colors";
--
-- # Architecture
--
-- The compiler runs in two conceptual passes:
--
--   Pass 1 — Symbol Collection:
--     Walk the top-level stylesheet and collect:
--       - Variable declarations  → stored in the root `env`
--       - Mixin definitions      → stored in env.mixins
--       - Function definitions   → stored in env.functions
--     These nodes produce no CSS output.
--
--   Pass 2 — Expansion and Emission:
--     Recursively walk the remaining AST nodes, expanding:
--       - $var references        → substituted with resolved value text
--       - @include directives    → mixin body expansion
--       - @if / @for / @each     → conditional / loop unrolling
--       - @function call sites   → function evaluation
--     Nested rules are flattened into top-level CSS selectors:
--       .parent { .child { color: blue; } }
--       → .parent .child { color: blue; }
--
-- # Scope (Environment) Chain
--
-- Each nested scope is a table:
--
--   env = {
--     vars      = {},      -- local variable bindings (name → text value)
--     mixins    = {},      -- mixin definitions (only in root env)
--     functions = {},      -- function definitions (only in root env)
--     parent    = nil,     -- parent env (nil at root)
--   }
--
-- Variable lookup walks the `parent` chain until the root.
-- Mixin and function lookup always search the root env.
--
-- # AST Structure
--
-- The parser produces ASTNode tables with:
--   node.rule_name  string — grammar rule name (e.g. "qualified_rule")
--   node.children   list   — child ASTNodes or Token tables
--
-- Tokens have:
--   token.type      string — token type (e.g. "VARIABLE", "IDENT")
--   token.value     string — raw token text (e.g. "$primary", "red")
--
-- The grammar is defined in code/grammars/lattice.grammar.

local M = {}
M.VERSION = "0.1.0"

-- =========================================================================
-- CSS built-in functions — pass these through unchanged
-- =========================================================================
--
-- When we encounter a function_call node, we check if its name is a
-- known CSS function.  If so, we do NOT attempt Lattice function lookup —
-- we emit it as-is.  This prevents `rgb(`, `calc(` etc. from being
-- treated as Lattice @function call sites.

local CSS_FUNCTIONS = {
    rgb=true, rgba=true, hsl=true, hsla=true, hwb=true, lab=true,
    lch=true, oklch=true, oklab=true, color=true, ["color-mix"]=true,
    calc=true, min=true, max=true, clamp=true, abs=true, sign=true,
    round=true, mod=true, rem=true, sin=true, cos=true, tan=true,
    asin=true, acos=true, atan=true, atan2=true, pow=true, sqrt=true,
    hypot=true, log=true, exp=true, var=true, env=true,
    url=true, format=true, ["local"]=true,
    ["linear-gradient"]=true, ["radial-gradient"]=true,
    ["conic-gradient"]=true, ["repeating-linear-gradient"]=true,
    ["repeating-radial-gradient"]=true, ["repeating-conic-gradient"]=true,
    counter=true, counters=true, attr=true, element=true,
    translate=true, translateX=true, translateY=true, translateZ=true,
    rotate=true, rotateX=true, rotateY=true, rotateZ=true,
    scale=true, scaleX=true, scaleY=true, scaleZ=true,
    skew=true, skewX=true, skewY=true,
    matrix=true, matrix3d=true, perspective=true,
    ["cubic-bezier"]=true, steps=true,
    path=true, polygon=true, circle=true, ellipse=true, inset=true,
    ["image-set"]=true, ["cross-fade"]=true,
    ["fit-content"]=true, minmax=true, repeat=true,
    blur=true, brightness=true, contrast=true, ["drop-shadow"]=true,
    grayscale=true, ["hue-rotate"]=true, invert=true, opacity=true,
    saturate=true, sepia=true,
}

--- Is this function name a CSS built-in?
-- FUNCTION tokens include "(" at the end: "rgb(" → check "rgb".
-- @param name string  Token value (may end with "(")
-- @return boolean
local function is_css_function(name)
    local clean = name:gsub("%(+$", "")
    return CSS_FUNCTIONS[clean] == true
end

-- =========================================================================
-- Environment / Scope helpers
-- =========================================================================

--- Create a new root environment.
-- @return table  env with empty vars, mixins, functions, parent=nil
local function new_env()
    return { vars = {}, mixins = {}, functions = {}, parent = nil }
end

--- Create a child environment that delegates to a parent for lookups.
-- @param parent table  The parent env
-- @return table        A new child env
local function child_env(parent)
    return { vars = {}, mixins = {}, functions = {}, parent = parent }
end

--- Look up a variable name in the env chain.
-- Searches local `vars` first, then walks `parent` links to root.
-- @param env  table   The current environment
-- @param name string  Variable name WITHOUT the leading `$`
-- @return string|nil  The resolved text value, or nil if not found
local function lookup_var(env, name)
    local e = env
    while e do
        if e.vars[name] ~= nil then
            return e.vars[name]
        end
        e = e.parent
    end
    return nil
end

--- Look up a mixin name.  Mixins are always stored on the root env.
-- @param env  table   Any env in the chain
-- @param name string  Mixin name
-- @return table|nil   Mixin definition {params, defaults, body}
local function lookup_mixin(env, name)
    -- Walk to root
    local e = env
    while e.parent do e = e.parent end
    return e.mixins[name]
end

--- Look up a function name.  Functions are always stored on the root env.
-- @param env  table   Any env in the chain
-- @param name string  Function name
-- @return table|nil   Function definition {params, defaults, body}
local function lookup_function(env, name)
    local e = env
    while e.parent do e = e.parent end
    return e.functions[name]
end

--- Store a variable in the current (local) scope.
-- @param env  table   The current environment
-- @param name string  Variable name (without "$")
-- @param val  string  The text value to store
local function set_var(env, name, val)
    env.vars[name] = val
end

-- =========================================================================
-- AST helpers — navigating ASTNode / Token tables
-- =========================================================================

--- Is `node` a Token (has `.type` but not `.rule_name`)?
-- @return boolean
local function is_token(node)
    return type(node) == "table" and node.type ~= nil and node.rule_name == nil
end

--- Is `node` an ASTNode (has `.rule_name`)?
-- @return boolean
local function is_node(node)
    return type(node) == "table" and node.rule_name ~= nil
end

--- Find the first ASTNode child with a given rule_name.
-- @param children list    List of ASTNode|Token
-- @param rule_name string
-- @return ASTNode|nil
local function find_child(children, rule_name)
    for _, c in ipairs(children) do
        if is_node(c) and c.rule_name == rule_name then
            return c
        end
    end
    return nil
end

--- Find all ASTNode children with a given rule_name.
-- @param children list
-- @param rule_name string
-- @return list of ASTNode
local function find_children(children, rule_name)
    local result = {}
    for _, c in ipairs(children) do
        if is_node(c) and c.rule_name == rule_name then
            result[#result + 1] = c
        end
    end
    return result
end

--- Find the first Token child with a given token type.
-- @param children list
-- @param tok_type string  e.g. "VARIABLE", "IDENT", "FUNCTION"
-- @return Token|nil
local function find_token(children, tok_type)
    for _, c in ipairs(children) do
        if is_token(c) and c.type == tok_type then
            return c
        end
    end
    return nil
end

--- Find all Token children with a given token type.
-- @param children list
-- @param tok_type string
-- @return list of Token
local function find_tokens(children, tok_type)
    local result = {}
    for _, c in ipairs(children) do
        if is_token(c) and c.type == tok_type then
            result[#result + 1] = c
        end
    end
    return result
end

--- Collect the raw text of tokens (of specified types) from a node tree.
-- Useful for emitting selector text or value text.
-- @param node ASTNode|Token  The subtree to walk
-- @param sep  string          Separator between tokens (default " ")
-- @return string
local function collect_tokens_text(node, sep)
    sep = sep or " "
    if is_token(node) then
        if node.type == "STRING" then
            return '"' .. node.value .. '"'
        end
        return node.value
    end
    if not is_node(node) then return "" end
    local parts = {}
    for _, child in ipairs(node.children or {}) do
        local t = collect_tokens_text(child, sep)
        if t ~= "" then
            parts[#parts + 1] = t
        end
    end
    return table.concat(parts, sep)
end

-- =========================================================================
-- Expression evaluation
-- =========================================================================
--
-- Lattice expressions appear in:
--   - @if conditions   (@if $x > 2 { ... })
--   - @for bounds      (@for $i from 1 through 12 { ... })
--   - @return values   (@return $n * 8px;)
--
-- The grammar defines a standard precedence hierarchy:
--   or → and → comparison → additive → multiplicative → unary → primary
--
-- We evaluate expressions at compile time.  The result type depends on the
-- operands: numbers stay as numbers, dimensions keep their unit, strings
-- stay as strings, booleans become true/false.
--
-- For @if conditions, we evaluate the expression as a Lua value and check
-- if it is truthy (not nil and not false).

-- Forward declarations (mutual recursion in expression evaluation)
local eval_expression

--- Extract numeric magnitude from a value that may be a number or dimension.
-- e.g. "16px" → 16, "42" → 42, "50%" → 50
-- @param val string  A text value
-- @return number|nil, string|nil  (magnitude, unit)
local function parse_numeric(val)
    if type(val) == "number" then return val, "" end
    if type(val) ~= "string" then return nil, nil end
    local n, unit = val:match("^(-?%d+%.?%d*)(.*)$")
    if n then
        return tonumber(n), unit
    end
    return nil, nil
end

--- Evaluate a `lattice_primary` node to a Lua value.
-- Returns a string (for CSS values), a number, or a boolean.
-- @param node ASTNode  The primary expression node
-- @param env  table    The current environment
-- @return any
local function eval_primary(node, env)
    local children = node.children or {}

    -- Single-token primaries
    if #children == 1 then
        local c = children[1]
        if is_token(c) then
            if c.type == "VARIABLE" then
                -- $varname — strip the "$" and look up
                local name = c.value:sub(2)
                local val = lookup_var(env, name)
                if val == nil then
                    -- Return the variable reference as-is (may not be defined yet)
                    return c.value
                end
                return val
            elseif c.type == "NUMBER" then
                return tonumber(c.value) or c.value
            elseif c.type == "DIMENSION" or c.type == "PERCENTAGE" then
                return c.value  -- keep as string with unit
            elseif c.type == "STRING" then
                return c.value  -- strip quotes for internal use
            elseif c.type == "IDENT" then
                -- true/false/null literals
                if c.value == "true" then return true end
                if c.value == "false" then return false end
                if c.value == "null" then return nil end
                return c.value
            elseif c.type == "HASH" then
                return c.value  -- e.g. "#4a90d9"
            end
        end
        -- Nested node (map_literal, function_call, or parenthesized expr)
        if is_node(c) then
            if c.rule_name == "function_call" then
                return eval_function_call(c, env)
            elseif c.rule_name == "map_literal" then
                return collect_tokens_text(c, " ")
            end
        end
    end

    -- Parenthesized expression: LPAREN lattice_expression RPAREN
    -- children = [LPAREN, lattice_expression, RPAREN]
    for _, c in ipairs(children) do
        if is_node(c) and c.rule_name == "lattice_expression" then
            return eval_expression(c, env)
        end
    end

    -- "true" / "false" / "null" literals encoded as IDENT children
    for _, c in ipairs(children) do
        if is_token(c) then
            if c.value == "true" then return true end
            if c.value == "false" then return false end
            if c.value == "null" then return nil end
        end
    end

    -- Fallback: collect all token text
    return collect_tokens_text(node, " ")
end

--- Evaluate a function call node.
-- If it's a CSS built-in, emit it as text.
-- If it's a Lattice @function, evaluate the body and return the @return value.
-- @param node ASTNode  function_call node
-- @param env  table    current environment
-- @return any
function eval_function_call(node, env)
    local children = node.children or {}
    -- function_call = FUNCTION function_args RPAREN | URL_TOKEN
    local func_tok = find_token(children, "FUNCTION")
    if not func_tok then
        -- URL_TOKEN or other
        return collect_tokens_text(node, "")
    end

    local func_name_raw = func_tok.value            -- e.g. "spacing("
    local func_name = func_name_raw:gsub("%(+$", "") -- e.g. "spacing"

    if is_css_function(func_name_raw) then
        -- CSS built-in — emit as-is
        return collect_tokens_text(node, "")
    end

    -- Look up Lattice function
    local func_def = lookup_function(env, func_name)
    if not func_def then
        -- Unknown function — emit as text
        return collect_tokens_text(node, "")
    end

    -- Evaluate arguments
    local args_node = find_child(children, "function_args")
    local arg_vals = eval_include_args(args_node, env)

    -- Build a child scope and bind parameters
    local call_env = child_env(env)
    for i, param_name in ipairs(func_def.params or {}) do
        local param_key = param_name:sub(2)  -- strip "$"
        local val = arg_vals[i]
        if val == nil then
            -- Use default if available
            local default_node = func_def.defaults and func_def.defaults[param_name]
            if default_node then
                val = eval_value_list_node(default_node, env)
            end
        end
        if val ~= nil then
            set_var(call_env, param_key, tostring(val))
        end
    end

    -- Execute function body and return the @return value
    local return_val = exec_function_body(func_def.body, call_env)
    return return_val or ""
end

--- Evaluate the function body, returning the value from @return.
-- @param body_node ASTNode  function_body node
-- @param env       table    call environment
-- @return any
function exec_function_body(body_node, env)
    local items = body_node.children or {}
    for _, item in ipairs(items) do
        if is_node(item) then
            if item.rule_name == "function_body_item" then
                -- function_body_item = variable_declaration | return_directive | lattice_control
                for _, inner in ipairs(item.children or {}) do
                    if is_node(inner) then
                        if inner.rule_name == "return_directive" then
                            -- @return lattice_expression SEMICOLON
                            local expr = find_child(inner.children, "lattice_expression")
                            if expr then
                                return eval_expression(expr, env)
                            end
                        elseif inner.rule_name == "variable_declaration" then
                            exec_variable_decl(inner, env)
                        end
                    end
                end
            elseif item.rule_name == "return_directive" then
                local expr = find_child(item.children, "lattice_expression")
                if expr then
                    return eval_expression(expr, env)
                end
            elseif item.rule_name == "variable_declaration" then
                exec_variable_decl(item, env)
            end
        end
    end
    return nil
end

--- Store a variable declaration into the given env.
-- @param node ASTNode  variable_declaration node
-- @param env  table    current environment
function exec_variable_decl(node, env)
    local children = node.children or {}
    local var_tok = find_token(children, "VARIABLE")
    if not var_tok then return end
    local name = var_tok.value:sub(2)  -- strip "$"
    local vl_node = find_child(children, "value_list")
    if vl_node then
        local val = eval_value_list_node(vl_node, env)
        set_var(env, name, tostring(val))
    end
end

--- Evaluate a value_list node to a string (expanding $vars).
-- @param node ASTNode  value_list node
-- @param env  table    current environment
-- @return string
function eval_value_list_node(node, env)
    if not is_node(node) then return "" end
    local parts = {}
    for _, child in ipairs(node.children or {}) do
        parts[#parts + 1] = eval_value_node(child, env)
    end
    return table.concat(parts, " ")
end

--- Evaluate a single value or token to a string, resolving $vars.
-- @param node ASTNode|Token
-- @param env  table
-- @return string
function eval_value_node(node, env)
    if is_token(node) then
        if node.type == "VARIABLE" then
            local name = node.value:sub(2)
            local val = lookup_var(env, name)
            return val ~= nil and tostring(val) or node.value
        elseif node.type == "STRING" then
            return '"' .. node.value .. '"'
        end
        return node.value
    end
    if is_node(node) then
        if node.rule_name == "value" then
            local parts = {}
            for _, c in ipairs(node.children or {}) do
                parts[#parts + 1] = eval_value_node(c, env)
            end
            return table.concat(parts, "")
        elseif node.rule_name == "function_call" then
            return tostring(eval_function_call(node, env))
        elseif node.rule_name == "mixin_value_list" or node.rule_name == "value_list" then
            return eval_value_list_node(node, env)
        end
        -- Generic: concatenate children
        local parts = {}
        for _, c in ipairs(node.children or {}) do
            parts[#parts + 1] = eval_value_node(c, env)
        end
        return table.concat(parts, " ")
    end
    return ""
end

--- Evaluate include_args node — returns an ordered list of argument values.
-- include_args = include_arg { COMMA include_arg }
-- include_arg  = VARIABLE COLON value_list | value_list
-- @param args_node ASTNode|nil  include_args or function_args node
-- @param env       table        current environment
-- @return list of string values
function eval_include_args(args_node, env)
    if not args_node then return {} end
    local vals = {}

    -- Collect all include_arg / function_arg children
    for _, child in ipairs(args_node.children or {}) do
        if is_node(child) then
            if child.rule_name == "include_arg" or child.rule_name == "function_arg" then
                -- Named arg: VARIABLE COLON value_list → skip $name, use value
                local vl = find_child(child.children, "value_list")
                if vl then
                    vals[#vals + 1] = eval_value_list_node(vl, env)
                else
                    -- Ordered arg: just emit the tokens
                    vals[#vals + 1] = eval_value_node(child, env)
                end
            elseif child.rule_name == "value_list" then
                vals[#vals + 1] = eval_value_list_node(child, env)
            end
        end
        -- Skip COMMA tokens
    end
    return vals
end

--- Evaluate a `lattice_unary` node.
-- @param node ASTNode
-- @param env  table
-- @return any
local function eval_unary(node, env)
    local children = node.children or {}
    -- MINUS lattice_unary
    if #children == 2 then
        local tok = children[1]
        if is_token(tok) and tok.type == "MINUS" then
            local val = eval_unary(children[2], env)
            local n, unit = parse_numeric(val)
            if n then
                local result = -n
                if unit and unit ~= "" then
                    return tostring(result) .. unit
                end
                return result
            end
            return "-" .. tostring(val)
        end
    end
    -- lattice_primary
    for _, c in ipairs(children) do
        if is_node(c) and c.rule_name == "lattice_primary" then
            return eval_primary(c, env)
        end
    end
    return eval_primary(node, env)
end

--- Evaluate a `lattice_multiplicative` node.
-- Handles STAR (multiply) and SLASH (divide).
-- @param node ASTNode
-- @param env  table
-- @return any
local function eval_multiplicative(node, env)
    local children = node.children or {}
    if #children == 0 then return "" end

    -- Find the first lattice_unary and evaluate it
    local result
    local i = 1
    while i <= #children do
        local c = children[i]
        if is_node(c) and c.rule_name == "lattice_unary" then
            result = eval_unary(c, env)
            i = i + 1
            break
        end
        i = i + 1
    end

    -- Process operator-operand pairs: (STAR | SLASH) lattice_unary
    while i <= #children do
        local op = children[i]
        local rhs_node = children[i + 1]
        if is_token(op) and rhs_node and is_node(rhs_node) and rhs_node.rule_name == "lattice_unary" then
            local rhs = eval_unary(rhs_node, env)
            local ln, lunit = parse_numeric(result)
            local rn, _    = parse_numeric(rhs)
            if ln and rn then
                local res
                if op.type == "STAR" then
                    res = ln * rn
                else  -- SLASH
                    if rn ~= 0 then res = ln / rn else res = 0 end
                end
                -- Carry the left-hand unit (e.g. "16" * "2" = 32, "8px" * 2 = 16px)
                if lunit and lunit ~= "" then
                    result = tostring(res) .. lunit
                else
                    result = res
                end
            else
                -- Can't evaluate numerically — keep as text
                result = tostring(result or "") .. op.value .. tostring(rhs or "")
            end
        end
        i = i + 2
    end

    return result
end

--- Evaluate a `lattice_additive` node.
-- Handles PLUS (add) and MINUS (subtract).
-- @param node ASTNode
-- @param env  table
-- @return any
local function eval_additive(node, env)
    local children = node.children or {}
    if #children == 0 then return "" end

    local result
    local i = 1
    while i <= #children do
        local c = children[i]
        if is_node(c) and c.rule_name == "lattice_multiplicative" then
            result = eval_multiplicative(c, env)
            i = i + 1
            break
        end
        i = i + 1
    end

    while i <= #children do
        local op = children[i]
        local rhs_node = children[i + 1]
        if is_token(op) and rhs_node and is_node(rhs_node) and rhs_node.rule_name == "lattice_multiplicative" then
            local rhs = eval_multiplicative(rhs_node, env)
            local ln, lunit = parse_numeric(result)
            local rn, _    = parse_numeric(rhs)
            if ln and rn then
                local res
                if op.type == "PLUS" then
                    res = ln + rn
                else  -- MINUS
                    res = ln - rn
                end
                if lunit and lunit ~= "" then
                    result = tostring(res) .. lunit
                else
                    result = res
                end
            else
                -- String concatenation / text fallback
                local sep = op.type == "PLUS" and "" or " - "
                result = tostring(result or "") .. sep .. tostring(rhs or "")
            end
        end
        i = i + 2
    end

    return result
end

--- Evaluate a `lattice_comparison` node.
-- @param node ASTNode
-- @param env  table
-- @return boolean or original value
local function eval_comparison(node, env)
    local children = node.children or {}
    if #children == 0 then return "" end

    -- Find first lattice_additive
    local lhs_node, op_node, rhs_node
    for idx, c in ipairs(children) do
        if is_node(c) and c.rule_name == "lattice_additive" then
            if not lhs_node then
                lhs_node = c
            else
                rhs_node = c
            end
        elseif is_node(c) and c.rule_name == "comparison_op" then
            op_node = c
        end
    end

    if not lhs_node then return "" end
    local lhs = eval_additive(lhs_node, env)
    if not op_node or not rhs_node then return lhs end

    local rhs = eval_additive(rhs_node, env)

    -- Get the comparison operator token
    local op_tok
    for _, c in ipairs(op_node.children or {}) do
        if is_token(c) then op_tok = c; break end
    end
    if not op_tok then return lhs end

    local lnum, _ = parse_numeric(lhs)
    local rnum, _ = parse_numeric(rhs)

    local op_type = op_tok.type
    if op_type == "EQUALS_EQUALS" then
        if lnum and rnum then return lnum == rnum end
        return tostring(lhs) == tostring(rhs)
    elseif op_type == "NOT_EQUALS" then
        if lnum and rnum then return lnum ~= rnum end
        return tostring(lhs) ~= tostring(rhs)
    elseif op_type == "GREATER" then
        if lnum and rnum then return lnum > rnum end
    elseif op_type == "GREATER_EQUALS" then
        if lnum and rnum then return lnum >= rnum end
    elseif op_type == "LESS" then
        if lnum and rnum then return lnum < rnum end
    elseif op_type == "LESS_EQUALS" then
        if lnum and rnum then return lnum <= rnum end
    end
    return lhs
end

--- Evaluate a `lattice_and_expr` node.
-- @param node ASTNode
-- @param env  table
-- @return any
local function eval_and_expr(node, env)
    local children = node.children or {}
    local result = true
    for _, c in ipairs(children) do
        if is_node(c) and c.rule_name == "lattice_comparison" then
            local val = eval_comparison(c, env)
            -- Short-circuit: nil or false → false
            if val == nil or val == false then
                return false
            end
            result = val
        end
        -- Skip "and" tokens
    end
    return result
end

--- Evaluate a `lattice_or_expr` node.
-- @param node ASTNode
-- @param env  table
-- @return any
local function eval_or_expr(node, env)
    local children = node.children or {}
    for _, c in ipairs(children) do
        if is_node(c) and c.rule_name == "lattice_and_expr" then
            local val = eval_and_expr(c, env)
            if val ~= nil and val ~= false then
                return val
            end
        end
        -- Skip "or" tokens
    end
    return false
end

--- Evaluate a `lattice_expression` node.
-- This is the top-level entry for expression evaluation.
-- @param node ASTNode  lattice_expression
-- @param env  table    current environment
-- @return any  (boolean, number, string)
eval_expression = function(node, env)
    if not is_node(node) then return "" end
    for _, c in ipairs(node.children or {}) do
        if is_node(c) and c.rule_name == "lattice_or_expr" then
            return eval_or_expr(c, env)
        end
    end
    return ""
end

-- =========================================================================
-- Selector emission
-- =========================================================================
--
-- CSS selectors are combinations of simple selectors, combinators, and
-- pseudo-classes.  We walk the selector subtree and reconstruct text.
--
-- In Lattice, the `&` token (AMPERSAND) refers to the parent selector.
-- When nesting, `.parent { &:hover { } }` → `.parent:hover { }`.

--- Emit a single compound_selector as text.
-- compound_selector = simple_selector { subclass_selector }
--                   | subclass_selector { subclass_selector }
-- @param node ASTNode  compound_selector
-- @return string
local function emit_compound_selector(node)
    local parts = {}
    for _, c in ipairs(node.children or {}) do
        if is_token(c) then
            parts[#parts + 1] = c.value
        elseif is_node(c) then
            if c.rule_name == "simple_selector" then
                for _, sc in ipairs(c.children or {}) do
                    if is_token(sc) then parts[#parts + 1] = sc.value end
                end
            elseif c.rule_name == "subclass_selector" then
                -- class_selector, id_selector, pseudo_class, pseudo_element, etc.
                for _, sc in ipairs(c.children or {}) do
                    if is_node(sc) then
                        if sc.rule_name == "class_selector" then
                            parts[#parts + 1] = "." .. (find_token(sc.children, "IDENT") or {value=""}).value
                        elseif sc.rule_name == "id_selector" then
                            local h = find_token(sc.children, "HASH")
                            if h then parts[#parts + 1] = h.value end
                        elseif sc.rule_name == "pseudo_class" then
                            local colon = find_token(sc.children, "COLON")
                            local ident = find_token(sc.children, "IDENT")
                            local func  = find_token(sc.children, "FUNCTION")
                            if func then
                                -- :nth-child(2n+1) style
                                parts[#parts + 1] = ":" .. func.value
                                local args = find_child(sc.children, "pseudo_class_args")
                                if args then
                                    parts[#parts + 1] = collect_tokens_text(args, "")
                                end
                                parts[#parts + 1] = ")"
                            elseif ident then
                                parts[#parts + 1] = ":" .. ident.value
                            end
                        elseif sc.rule_name == "pseudo_element" then
                            local ident = find_token(sc.children, "IDENT")
                            if ident then parts[#parts + 1] = "::" .. ident.value end
                        elseif sc.rule_name == "attribute_selector" then
                            parts[#parts + 1] = collect_tokens_text(sc, "")
                        elseif sc.rule_name == "placeholder_selector" then
                            local ph = find_token(sc.children, "PLACEHOLDER")
                            if ph then parts[#parts + 1] = ph.value end
                        else
                            parts[#parts + 1] = collect_tokens_text(sc, "")
                        end
                    elseif is_token(sc) then
                        parts[#parts + 1] = sc.value
                    end
                end
            elseif c.rule_name == "placeholder_selector" then
                local ph = find_token(c.children, "PLACEHOLDER")
                if ph then parts[#parts + 1] = ph.value end
            else
                parts[#parts + 1] = collect_tokens_text(c, "")
            end
        end
    end
    return table.concat(parts, "")
end

--- Emit a complex_selector (compound selectors joined by combinators) as text.
-- complex_selector = compound_selector { [ combinator ] compound_selector }
-- @param node ASTNode  complex_selector
-- @return string
local function emit_complex_selector(node)
    local parts = {}
    for _, c in ipairs(node.children or {}) do
        if is_node(c) then
            if c.rule_name == "compound_selector" then
                parts[#parts + 1] = emit_compound_selector(c)
            elseif c.rule_name == "combinator" then
                -- GREATER (">"), PLUS ("+"), TILDE ("~")
                local tok
                for _, t in ipairs(c.children or {}) do
                    if is_token(t) then tok = t; break end
                end
                if tok then
                    parts[#parts + 1] = " " .. tok.value
                end
            end
        elseif is_token(c) then
            parts[#parts + 1] = c.value
        end
    end
    return table.concat(parts, " "):gsub("%s+", " "):gsub("^%s+", ""):gsub("%s+$", "")
end

--- Emit a selector_list as text (comma-separated complex_selectors).
-- @param node ASTNode  selector_list
-- @return string       e.g. "h1, h2, .title"
local function emit_selector_list(node)
    local parts = {}
    for _, c in ipairs(node.children or {}) do
        if is_node(c) and c.rule_name == "complex_selector" then
            parts[#parts + 1] = emit_complex_selector(c)
        end
    end
    return table.concat(parts, ", ")
end

--- Resolve `&` (parent reference) in a selector string.
-- When nesting, the `&` in `.child { &:hover }` refers to the parent selector.
-- If there's no `&`, we prepend the parent: `.parent .child`.
-- @param parent_selector string  The outer rule's selector text
-- @param child_selector  string  The inner rule's selector text
-- @return string
local function resolve_selector(parent_selector, child_selector)
    if parent_selector == "" then
        return child_selector
    end
    if child_selector:find("&") then
        -- Replace `&` with the parent selector
        return child_selector:gsub("&", parent_selector)
    end
    -- Descendant combinator: ".parent .child"
    return parent_selector .. " " .. child_selector
end

-- =========================================================================
-- CSS text emission
-- =========================================================================
--
-- After expansion, we walk the (now-pure-CSS) node tree and produce text.
-- Each function returns a string.

-- Forward declarations (mutual recursion for nested rules)
local compile_block_items
local compile_rule_block

--- Emit a value_list node as CSS text, resolving $var references.
-- @param node ASTNode  value_list
-- @param env  table    current environment
-- @return string
local function emit_value_list(node, env)
    if not is_node(node) then return "" end
    local parts = {}
    for _, child in ipairs(node.children or {}) do
        parts[#parts + 1] = eval_value_node(child, env)
    end
    return table.concat(parts, " ")
end

--- Emit a declaration node as a CSS "property: value;" string.
-- declaration = property COLON value_list [ priority ] SEMICOLON
-- @param node   ASTNode  declaration
-- @param env    table    current environment
-- @param indent string   current indentation prefix
-- @return string
local function emit_declaration(node, env, indent)
    local children = node.children or {}
    local prop_node = find_child(children, "property")
    local vl_node   = find_child(children, "value_list")
    local pri_node  = find_child(children, "priority")

    if not prop_node then return "" end

    -- Property name: IDENT or CUSTOM_PROPERTY
    local prop = collect_tokens_text(prop_node, "")

    -- Value
    local val = ""
    if vl_node then
        val = emit_value_list(vl_node, env)
    end

    -- !important
    local important = ""
    if pri_node then
        important = " !important"
    end

    return indent .. prop .. ": " .. val .. important .. ";\n"
end

--- Collect mixin parameter definitions from a mixin_params node.
-- Returns (params_list, defaults_table).
-- params_list   = list of parameter names (with "$")
-- defaults_table = map of name → default value string
-- @param params_node ASTNode|nil  mixin_params node
-- @param env         table        current environment (for default evaluation)
-- @return list, table
local function extract_mixin_params(params_node, env)
    if not params_node then return {}, {} end
    local params   = {}
    local defaults = {}
    for _, child in ipairs(params_node.children or {}) do
        if is_node(child) and child.rule_name == "mixin_param" then
            local var_tok = find_token(child.children, "VARIABLE")
            if var_tok then
                params[#params + 1] = var_tok.value
                local default_node = find_child(child.children, "mixin_value_list")
                               or find_child(child.children, "value_list")
                if default_node then
                    defaults[var_tok.value] = eval_value_list_node(default_node, env)
                end
            end
        end
    end
    return params, defaults
end

--- Compile the contents of a block (declarations + nested rules).
-- This is a key function: it handles all block_item nodes:
--   - variable_declaration   → store in env, emit nothing
--   - declaration            → emit "prop: val;"
--   - qualified_rule         → flatten nested selector
--   - include_directive      → expand mixin
--   - lattice_control        → expand @if/@for/@each
-- @param block_node     ASTNode  The block node (LBRACE ... RBRACE)
-- @param env            table    Current environment
-- @param parent_sel     string   Parent selector for nesting (empty at root)
-- @param declarations   list     Output: CSS declaration lines (to be collected)
-- @param nested_rules   list     Output: Flattened nested rule CSS strings
local function compile_block(block_node, env, parent_sel, declarations, nested_rules)
    -- block = LBRACE block_contents RBRACE
    local contents = find_child(block_node.children, "block_contents")
    if not contents then return end

    for _, item in ipairs(contents.children or {}) do
        if is_node(item) and item.rule_name == "block_item" then
            compile_block_items(item, env, parent_sel, declarations, nested_rules)
        end
    end
end

--- Compile a single block_item node.
-- block_item = lattice_block_item | at_rule | declaration_or_nested
-- @param item         ASTNode  block_item
-- @param env          table    current environment
-- @param parent_sel   string   parent selector for nesting
-- @param declarations list     output accumulator for declarations
-- @param nested_rules list     output accumulator for nested rule CSS strings
compile_block_items = function(item, env, parent_sel, declarations, nested_rules)
    local children = item.children or {}
    for _, inner in ipairs(children) do
        if is_node(inner) then
            if inner.rule_name == "lattice_block_item" then
                -- Variable declarations, @include, @if/@for/@each
                compile_lattice_block_item(inner, env, parent_sel, declarations, nested_rules)
            elseif inner.rule_name == "declaration_or_nested" then
                compile_declaration_or_nested(inner, env, parent_sel, declarations, nested_rules)
            elseif inner.rule_name == "at_rule" then
                -- CSS at-rules like @media (pass through)
                nested_rules[#nested_rules + 1] = emit_at_rule(inner, parent_sel, env)
            elseif inner.rule_name == "declaration" then
                declarations[#declarations + 1] = emit_declaration(inner, env, "  ")
            elseif inner.rule_name == "qualified_rule" then
                -- Directly nested qualified rule
                compile_nested_rule(inner, env, parent_sel, nested_rules)
            end
        end
    end
end

--- Compile a `lattice_block_item` node.
-- lattice_block_item = variable_declaration | include_directive
--                    | lattice_control | content_directive
--                    | extend_directive | at_root_directive
-- @param node         ASTNode
-- @param env          table
-- @param parent_sel   string
-- @param declarations list
-- @param nested_rules list
function compile_lattice_block_item(node, env, parent_sel, declarations, nested_rules)
    for _, child in ipairs(node.children or {}) do
        if is_node(child) then
            if child.rule_name == "variable_declaration" then
                exec_variable_decl(child, env)
            elseif child.rule_name == "include_directive" then
                compile_include(child, env, parent_sel, declarations, nested_rules)
            elseif child.rule_name == "lattice_control" then
                compile_control(child, env, parent_sel, declarations, nested_rules)
            end
            -- @content, @extend, @at-root: skip for now (advanced features)
        end
    end
end

--- Compile a `declaration_or_nested` node.
-- declaration_or_nested = declaration | qualified_rule
-- @param node         ASTNode
-- @param env          table
-- @param parent_sel   string
-- @param declarations list
-- @param nested_rules list
function compile_declaration_or_nested(node, env, parent_sel, declarations, nested_rules)
    for _, child in ipairs(node.children or {}) do
        if is_node(child) then
            if child.rule_name == "declaration" then
                declarations[#declarations + 1] = emit_declaration(child, env, "  ")
            elseif child.rule_name == "qualified_rule" then
                compile_nested_rule(child, env, parent_sel, nested_rules)
            end
        end
    end
end

--- Compile a nested qualified_rule, producing flattened CSS.
-- When `.child { color: blue; }` appears inside `.parent { }`, the output is:
--   .parent .child {
--     color: blue;
--   }
-- @param node       ASTNode  qualified_rule node
-- @param env        table    current environment (child scope created internally)
-- @param parent_sel string   parent CSS selector
-- @param output     list     accumulator for CSS strings
function compile_nested_rule(node, env, parent_sel, output)
    local children  = node.children or {}
    local sel_node  = find_child(children, "selector_list")
    local blk_node  = find_child(children, "block")
    if not sel_node or not blk_node then return end

    local raw_sel  = emit_selector_list(sel_node)
    local full_sel = resolve_selector(parent_sel, raw_sel)

    -- Create a child scope so declarations inside don't leak out
    local child_scope = child_env(env)
    local decls   = {}
    local nested  = {}
    compile_block(blk_node, child_scope, full_sel, decls, nested)

    if #decls > 0 then
        output[#output + 1] = full_sel .. " {\n" .. table.concat(decls, "") .. "}\n"
    end
    for _, ns in ipairs(nested) do
        output[#output + 1] = ns
    end
end

--- Compile an `@include` directive — expand a mixin into the current block.
-- include_directive = "@include" FUNCTION [ include_args ] RPAREN (SEMICOLON|block)
--                   | "@include" IDENT (SEMICOLON|block)
-- @param node         ASTNode
-- @param env          table
-- @param parent_sel   string
-- @param declarations list
-- @param nested_rules list
function compile_include(node, env, parent_sel, declarations, nested_rules)
    local children = node.children or {}

    -- Determine mixin name
    local func_tok = find_token(children, "FUNCTION")
    local ident_tok = find_token(children, "IDENT")
    local mixin_name
    if func_tok then
        mixin_name = func_tok.value:gsub("%(+$", "")
    elseif ident_tok then
        mixin_name = ident_tok.value
    end
    if not mixin_name then return end

    local mixin_def = lookup_mixin(env, mixin_name)
    if not mixin_def then
        -- Unknown mixin — silently skip
        return
    end

    -- Evaluate arguments
    local args_node = find_child(children, "include_args")
    local arg_vals  = eval_include_args(args_node, env)

    -- Build a call environment: bind mixin parameters to argument values
    local call_env = child_env(env)
    for i, param_name in ipairs(mixin_def.params or {}) do
        local param_key = param_name:sub(2)  -- strip "$"
        local val = arg_vals[i]
        if val == nil then
            -- Use default value if provided
            val = mixin_def.defaults and mixin_def.defaults[param_name]
        end
        if val ~= nil then
            set_var(call_env, param_key, tostring(val))
        end
    end

    -- Expand the mixin body into this block's declarations and nested rules
    compile_block(mixin_def.body, call_env, parent_sel, declarations, nested_rules)
end

--- Compile a `lattice_control` node (@if, @for, @each, @while).
-- @param node         ASTNode
-- @param env          table
-- @param parent_sel   string
-- @param declarations list
-- @param nested_rules list
function compile_control(node, env, parent_sel, declarations, nested_rules)
    for _, child in ipairs(node.children or {}) do
        if is_node(child) then
            if child.rule_name == "if_directive" then
                compile_if(child, env, parent_sel, declarations, nested_rules)
            elseif child.rule_name == "for_directive" then
                compile_for(child, env, parent_sel, declarations, nested_rules)
            elseif child.rule_name == "each_directive" then
                compile_each(child, env, parent_sel, declarations, nested_rules)
            elseif child.rule_name == "while_directive" then
                compile_while(child, env, parent_sel, declarations, nested_rules)
            end
        end
    end
end

--- Compile an `@if` directive.
-- if_directive = "@if" lattice_expression block
--                { "@else" "if" lattice_expression block }
--                [ "@else" block ]
-- @param node         ASTNode  if_directive
-- @param env          table
-- @param parent_sel   string
-- @param declarations list
-- @param nested_rules list
function compile_if(node, env, parent_sel, declarations, nested_rules)
    local children = node.children or {}
    -- Collect (condition, block) pairs and an optional else block
    local branches = {}  -- list of {cond=ASTNode|nil, block=ASTNode}
    local i = 1
    local state = "if"   -- "if" | "else_if" | "else"

    while i <= #children do
        local c = children[i]
        if is_token(c) then
            -- Keyword tokens: look at the value to advance state
            -- "@if" → stay in "if" state (already consumed first branch)
            -- "@else" → transition to else state
        elseif is_node(c) then
            if c.rule_name == "lattice_expression" and state ~= "else" then
                -- Found a condition for the current branch
                local cond = c
                -- The next node should be the block
                local blk = nil
                for j = i + 1, #children do
                    if is_node(children[j]) and children[j].rule_name == "block" then
                        blk = children[j]
                        i = j
                        break
                    end
                end
                branches[#branches + 1] = { cond = cond, block = blk }
            elseif c.rule_name == "block" and state == "else" then
                -- Unconditional @else branch
                branches[#branches + 1] = { cond = nil, block = c }
            end
        end
        i = i + 1
    end

    -- Determine the "@else" keyword position to split if/else-if/else
    -- Re-scan: we need to correctly identify the @else branches.
    -- Rebuild branches more carefully:
    branches = {}
    local expect_cond  = true
    local cur_cond     = nil

    for _, c in ipairs(children) do
        if is_token(c) then
            if c.value == "@else" then
                expect_cond = false  -- next might be "if" keyword or a block
            elseif c.value == "@if" then
                expect_cond = true
            elseif c.type == "IDENT" and c.value == "if" then
                expect_cond = true  -- "@else if" → condition follows
            end
        elseif is_node(c) then
            if c.rule_name == "lattice_expression" then
                cur_cond = c
            elseif c.rule_name == "block" then
                branches[#branches + 1] = { cond = cur_cond, block = c }
                cur_cond = nil
                expect_cond = true  -- reset for subsequent branches
            end
        end
    end

    -- Evaluate branches in order, execute the first truthy one
    for _, branch in ipairs(branches) do
        local take = true
        if branch.cond then
            local val = eval_expression(branch.cond, env)
            take = (val ~= nil and val ~= false and val ~= "false" and val ~= "null")
        end
        if take and branch.block then
            local branch_env = child_env(env)
            compile_block(branch.block, branch_env, parent_sel, declarations, nested_rules)
            return
        end
    end
end

--- Compile a `@for` directive.
-- @for $i from 1 through 12 { ... }
-- Iterates $i from `start` to `finish` (inclusive for "through",
-- exclusive for "to").
-- @param node         ASTNode  for_directive
-- @param env          table
-- @param parent_sel   string
-- @param declarations list
-- @param nested_rules list
function compile_for(node, env, parent_sel, declarations, nested_rules)
    local children = node.children or {}
    -- for_directive = "@for" VARIABLE "from" lattice_expression
    --                 ("through"|"to") lattice_expression block
    local var_tok = find_token(children, "VARIABLE")
    if not var_tok then return end
    local var_name = var_tok.value:sub(2)  -- strip "$"

    -- Collect the two lattice_expression nodes and the block
    local exprs = {}
    local blk_node = nil
    local exclusive = false  -- "to" is exclusive, "through" is inclusive

    for _, c in ipairs(children) do
        if is_node(c) then
            if c.rule_name == "lattice_expression" then
                exprs[#exprs + 1] = c
            elseif c.rule_name == "block" then
                blk_node = c
            end
        elseif is_token(c) then
            if c.value == "to" then exclusive = true end
        end
    end

    if #exprs < 2 or not blk_node then return end

    local start_val  = eval_expression(exprs[1], env)
    local finish_val = eval_expression(exprs[2], env)
    local start_n  = parse_numeric(start_val)
    local finish_n = parse_numeric(finish_val)
    if not start_n or not finish_n then return end

    -- Guard against runaway loops
    local limit = math.abs(finish_n - start_n) + 2
    if limit > 1000 then limit = 1000 end

    local step = start_n <= finish_n and 1 or -1
    local i_val = math.floor(start_n)
    local end_val = math.floor(finish_n)
    local count = 0

    while count < limit do
        -- "through" is inclusive, "to" is exclusive
        if exclusive then
            if step > 0 and i_val >= end_val then break end
            if step < 0 and i_val <= end_val then break end
        else
            if step > 0 and i_val > end_val then break end
            if step < 0 and i_val < end_val then break end
        end

        local iter_env = child_env(env)
        set_var(iter_env, var_name, tostring(i_val))
        compile_block(blk_node, iter_env, parent_sel, declarations, nested_rules)

        i_val = i_val + step
        count = count + 1
    end
end

--- Compile a `@each` directive.
-- @each $color in red, green, blue { .t { color: $color; } }
-- Iterates over a comma-separated list of values.
-- @param node         ASTNode  each_directive
-- @param env          table
-- @param parent_sel   string
-- @param declarations list
-- @param nested_rules list
function compile_each(node, env, parent_sel, declarations, nested_rules)
    local children = node.children or {}
    -- each_directive = "@each" VARIABLE { COMMA VARIABLE } "in" each_list block
    local var_toks = find_tokens(children, "VARIABLE")
    if #var_toks == 0 then return end
    local var_name = var_toks[1].value:sub(2)  -- primary iteration variable

    local each_list = find_child(children, "each_list")
    local blk_node  = find_child(children, "block")
    if not each_list or not blk_node then return end

    -- Collect values from each_list = value { COMMA value }
    local values = {}
    for _, c in ipairs(each_list.children or {}) do
        if is_node(c) and c.rule_name == "value" then
            values[#values + 1] = eval_value_node(c, env)
        elseif is_token(c) and c.type ~= "COMMA" then
            values[#values + 1] = c.value
        end
    end

    for _, val in ipairs(values) do
        local iter_env = child_env(env)
        set_var(iter_env, var_name, tostring(val))
        compile_block(blk_node, iter_env, parent_sel, declarations, nested_rules)
    end
end

--- Compile a `@while` directive.
-- @while $i <= 12 { .col { width: $i * 8%; } $i: $i + 1; }
-- Loops until the condition is false.  Capped at 1000 iterations.
-- @param node         ASTNode  while_directive
-- @param env          table
-- @param parent_sel   string
-- @param declarations list
-- @param nested_rules list
function compile_while(node, env, parent_sel, declarations, nested_rules)
    local children = node.children or {}
    local cond_node = find_child(children, "lattice_expression")
    local blk_node  = find_child(children, "block")
    if not cond_node or not blk_node then return end

    local MAX_ITER = 1000
    local count = 0
    while count < MAX_ITER do
        local val = eval_expression(cond_node, env)
        if val == nil or val == false or val == "false" then break end
        compile_block(blk_node, env, parent_sel, declarations, nested_rules)
        count = count + 1
    end
end

--- Emit a CSS `at_rule` node as text.
-- at_rule = AT_KEYWORD at_prelude (SEMICOLON | block)
-- @param node       ASTNode  at_rule
-- @param parent_sel string   parent selector (for nested @media etc.)
-- @param env        table    current environment
-- @return string
function emit_at_rule(node, parent_sel, env)
    local children = node.children or {}
    local kw = find_token(children, "AT_KEYWORD")
    if not kw then return "" end

    local prelude_node = find_child(children, "at_prelude")
    local prelude = prelude_node and collect_tokens_text(prelude_node, " ") or ""

    local blk = find_child(children, "block")
    if blk then
        -- Nested rule (e.g. @media) — recurse inside
        local inner_decls   = {}
        local inner_nested  = {}
        -- For @media inside a rule, we need to re-wrap the parent selector
        if parent_sel ~= "" then
            -- @media query { parent_sel { declarations } }
            local inner_inner_decls = {}
            local inner_inner_nested = {}
            compile_block(blk, child_env(env), parent_sel, inner_inner_decls, inner_inner_nested)
            local body_parts = {}
            if #inner_inner_decls > 0 then
                body_parts[#body_parts + 1] = "  " .. parent_sel .. " {\n"
                for _, d in ipairs(inner_inner_decls) do
                    body_parts[#body_parts + 1] = "  " .. d
                end
                body_parts[#body_parts + 1] = "  }\n"
            end
            for _, nr in ipairs(inner_inner_nested) do
                body_parts[#body_parts + 1] = "  " .. nr:gsub("\n", "\n  "):gsub("  $", "") .. "\n"
            end
            if #body_parts > 0 then
                return kw.value .. " " .. prelude .. " {\n" .. table.concat(body_parts, "") .. "}\n"
            end
            return ""
        else
            compile_block(blk, child_env(env), "", inner_decls, inner_nested)
            local body = table.concat(inner_decls, "") .. table.concat(inner_nested, "")
            if body ~= "" then
                return kw.value .. " " .. prelude .. " {\n" .. body .. "}\n"
            end
            return kw.value .. " " .. prelude .. " {}\n"
        end
    else
        -- Statement at-rule (e.g. @import "file.css";)
        if prelude ~= "" then
            return kw.value .. " " .. prelude .. ";\n"
        end
        return kw.value .. ";\n"
    end
end

-- =========================================================================
-- Pass 1: Symbol Collection
-- =========================================================================
--
-- Walk the top-level `stylesheet` node and collect all variable,
-- mixin, and function definitions into the root env.  These nodes
-- produce no CSS output and are removed from the active node list.

--- Collect all top-level definitions from the stylesheet into env.
-- Modifies `env` in place.
-- @param stylesheet ASTNode  the root stylesheet node
-- @param env        table    the root environment
-- @return list of ASTNode    the remaining (non-definition) top-level rules
local function collect_top_level_symbols(stylesheet, env)
    local remaining = {}
    for _, rule in ipairs(stylesheet.children or {}) do
        if is_node(rule) then
            -- rule = lattice_rule | at_rule | qualified_rule
            -- We look for lattice_rule > variable_declaration|mixin_definition|function_definition
            local collected = false
            if rule.rule_name == "rule" then
                local inner = rule.children and rule.children[1]
                if is_node(inner) and inner.rule_name == "lattice_rule" then
                    local lat = inner.children and inner.children[1]
                    if is_node(lat) then
                        if lat.rule_name == "variable_declaration" then
                            exec_variable_decl(lat, env)
                            collected = true
                        elseif lat.rule_name == "mixin_definition" then
                            collect_mixin_def(lat, env)
                            collected = true
                        elseif lat.rule_name == "function_definition" then
                            collect_function_def(lat, env)
                            collected = true
                        elseif lat.rule_name == "use_directive" then
                            -- @use — module loading not implemented; skip
                            collected = true
                        end
                    end
                end
            end
            if not collected then
                remaining[#remaining + 1] = rule
            end
        end
    end
    return remaining
end

--- Collect a mixin definition into the root env.
-- mixin_definition = "@mixin" FUNCTION [ mixin_params ] RPAREN block
--                  | "@mixin" IDENT block
-- @param node ASTNode  mixin_definition
-- @param env  table    root environment
function collect_mixin_def(node, env)
    local children = node.children or {}
    local func_tok  = find_token(children, "FUNCTION")
    local ident_tok = find_token(children, "IDENT")
    local name
    if func_tok then
        name = func_tok.value:gsub("%(+$", "")
    elseif ident_tok then
        name = ident_tok.value
    end
    if not name then return end

    local params_node = find_child(children, "mixin_params")
    local body_node   = find_child(children, "block")
    if not body_node then return end

    local params, defaults = extract_mixin_params(params_node, env)
    -- Walk to root to store mixin
    local root = env
    while root.parent do root = root.parent end
    root.mixins[name] = { params = params, defaults = defaults, body = body_node }
end

--- Collect a function definition into the root env.
-- function_definition = "@function" FUNCTION [ mixin_params ] RPAREN function_body
--                     | "@function" IDENT function_body
-- @param node ASTNode  function_definition
-- @param env  table    root environment
function collect_function_def(node, env)
    local children = node.children or {}
    local func_tok  = find_token(children, "FUNCTION")
    local ident_tok = find_token(children, "IDENT")
    local name
    if func_tok then
        name = func_tok.value:gsub("%(+$", "")
    elseif ident_tok then
        name = ident_tok.value
    end
    if not name then return end

    local params_node = find_child(children, "mixin_params")
    local body_node   = find_child(children, "function_body")
    if not body_node then return end

    local params, defaults = extract_mixin_params(params_node, env)
    local root = env
    while root.parent do root = root.parent end
    root.functions[name] = { params = params, defaults = defaults, body = body_node }
end

-- =========================================================================
-- Pass 2: Top-Level Expansion
-- =========================================================================
--
-- Walk the remaining (non-definition) top-level nodes and emit CSS text.

--- Compile a top-level `rule` node.
-- rule = lattice_rule | at_rule | qualified_rule
-- @param rule_node ASTNode
-- @param env       table
-- @return string
local function compile_top_level_rule(rule_node, env)
    if not is_node(rule_node) then return "" end
    local inner = rule_node.rule_name == "rule"
                  and rule_node.children
                  and rule_node.children[1]
                  or rule_node

    if not is_node(inner) then return "" end

    if inner.rule_name == "qualified_rule" then
        -- Top-level CSS rule with possible nesting
        local sel_node = find_child(inner.children, "selector_list")
        local blk_node = find_child(inner.children, "block")
        if not sel_node or not blk_node then return "" end

        local selector = emit_selector_list(sel_node)
        local rule_env = child_env(env)
        local decls    = {}
        local nested   = {}
        compile_block(blk_node, rule_env, selector, decls, nested)

        local result = {}
        if #decls > 0 then
            result[#result + 1] = selector .. " {\n" .. table.concat(decls, "") .. "}\n"
        end
        for _, nr in ipairs(nested) do
            result[#result + 1] = nr
        end
        return table.concat(result, "\n")

    elseif inner.rule_name == "at_rule" then
        return emit_at_rule(inner, "", env)

    elseif inner.rule_name == "lattice_rule" then
        -- Top-level Lattice control flow (@if/@for/@each at stylesheet level)
        local lat = inner.children and inner.children[1]
        if is_node(lat) and lat.rule_name == "lattice_control" then
            local decls   = {}
            local nested  = {}
            compile_control(lat, env, "", decls, nested)
            -- At the top level, "declarations" aren't valid — wrap them if any
            return table.concat(nested, "\n")
        end
        return ""
    end

    return ""
end

-- =========================================================================
-- Public API
-- =========================================================================

--- Compile a Lattice AST into CSS text.
--
-- This is the main entry point.  Pass the root `stylesheet` ASTNode
-- returned by `lattice_parser.parse()` and receive CSS text.
--
-- The compilation pipeline:
--   1. Create a root environment.
--   2. Collect top-level variable/mixin/function definitions (Pass 1).
--   3. Compile remaining nodes to CSS text (Pass 2).
--
-- @param ast ASTNode  The root `stylesheet` node from the parser.
-- @return string      The compiled CSS text.
--
-- Example:
--
--   local lattice_parser  = require("coding_adventures.lattice_parser")
--   local lattice_ast_to_css = require("coding_adventures.lattice_ast_to_css")
--
--   local ast = lattice_parser.parse("$c: red; h1 { color: $c; }")
--   local css = lattice_ast_to_css.compile(ast)
--   -- css == "h1 {\n  color: red;\n}\n"
function M.compile(ast)
    if not is_node(ast) then
        error("lattice_ast_to_css.compile: expected an ASTNode, got " .. type(ast))
    end

    local env = new_env()

    -- Pass 1: collect top-level definitions (variables, mixins, functions)
    local remaining = collect_top_level_symbols(ast, env)

    -- Pass 2: compile remaining nodes to CSS
    local parts = {}
    for _, rule_node in ipairs(remaining) do
        local css = compile_top_level_rule(rule_node, env)
        if css ~= "" then
            parts[#parts + 1] = css
        end
    end

    local result = table.concat(parts, "")
    -- Normalize: ensure the result ends with a single newline
    if result ~= "" and result:sub(-1) ~= "\n" then
        result = result .. "\n"
    end
    return result
end

return M
