-- excel_parser — Hand-written recursive-descent parser for Excel formulas
-- =========================================================================
--
-- This package is part of the coding-adventures monorepo.  It sits above
-- the excel_lexer in the language-tooling layer and produces an Abstract
-- Syntax Tree (AST) from Excel formula text.
--
-- # What is an Excel formula?
--
-- An Excel formula is a mini-language embedded in a spreadsheet cell.  It
-- always starts with "=", describes a computation, and can reference other
-- cells, ranges, functions, and literal values.
--
-- Some representative formulas and what they compute:
--
--   =A1+B2                        → add two cells
--   =SUM(A1:B10)                  → sum a range
--   =IF(A1>0, "positive", "neg")  → conditional
--   =Sheet1!A1 * 1.1              → cross-sheet reference
--   =A1*100%                      → percentage (postfix)
--   ={1,2;3,4}                    → literal 2-D array constant
--
-- # Grammar (BNF)
--
-- The grammar implemented here mirrors excel.grammar:
--
--   formula             = [ EQUALS ] expression ;
--   expression          = comparison_expr ;
--   comparison_expr     = concat_expr { comp_op concat_expr } ;
--   comp_op             = EQUALS | NOT_EQUALS | LESS_THAN | LESS_EQUALS
--                       | GREATER_THAN | GREATER_EQUALS ;
--   concat_expr         = additive_expr { AMP additive_expr } ;
--   additive_expr       = multiplicative_expr { (PLUS | MINUS) multiplicative_expr } ;
--   multiplicative_expr = power_expr { (STAR | SLASH) power_expr } ;
--   power_expr          = unary_expr { CARET unary_expr } ;
--   unary_expr          = { (PLUS | MINUS) } postfix_expr ;
--   postfix_expr        = primary { PERCENT } ;
--
--   primary = LPAREN expression RPAREN
--           | array_constant
--           | function_call
--           | ref_prefix_expr
--           | cell_range
--           | CELL
--           | NAME
--           | NUMBER
--           | STRING
--           | BOOL
--           | ERROR_CONSTANT ;
--
--   cell_range    = ( REF_PREFIX CELL | CELL ) [ COLON ( REF_PREFIX CELL | CELL ) ] ;
--   function_call = NAME LPAREN [ arg_list ] RPAREN ;
--   arg_list      = arg { (COMMA | SEMICOLON) arg } ;
--   arg           = [ expression ] ;   (empty args allowed: IF(,TRUE,FALSE))
--
--   array_constant = LBRACE array_row { SEMICOLON array_row } RBRACE ;
--   array_row      = array_item { COMMA array_item } ;
--   array_item     = NUMBER | STRING | BOOL | ERROR_CONSTANT
--                  | MINUS NUMBER | PLUS NUMBER ;
--
-- # AST node types
--
-- Every `parse_*` function returns a table with a `kind` field:
--
--   { kind = "formula",    eq = token_or_nil, body = node }
--   { kind = "binop",      op = token, left = node, right = node }
--   { kind = "unop",       op = token, operand = node }
--   { kind = "postfix",    op = token, operand = node }
--   { kind = "call",       name = token, args = { node, ... } }
--   { kind = "range",      start_ref = node, end_ref = node }
--   { kind = "ref_prefix", prefix = token, ref = node | nil }
--   { kind = "cell",       token = token }
--   { kind = "number",     token = token }
--   { kind = "string",     token = token }
--   { kind = "bool",       token = token }
--   { kind = "error",      token = token }
--   { kind = "name",       token = token }
--   { kind = "array",      rows = { { node, ... }, ... } }
--   { kind = "group",      expr = node }
--
-- # Operator precedence (lowest to highest)
--
--   1. comparison:      =  <>  <  <=  >  >=
--   2. concatenation:   &
--   3. additive:        + -
--   4. multiplicative:  * /
--   5. power:           ^
--   6. unary prefix:    + -   (right-associative via recursion)
--   7. postfix:         %
--   8. primary:         literals, references, function calls, (expr)
--
-- # Why hand-written?
--
-- The excel.grammar PEG grammar is complex enough that the generic
-- grammar-driven GrammarParser would require significant plumbing for the
-- SPACE-as-intersection-operator rule, context-sensitive disambiguation,
-- and empty arguments in function calls.  A hand-written parser gives
-- full control over disambiguation, error messages, and operator precedence.
--
-- # Path navigation
--
-- This file lives at:
--   code/packages/lua/excel_parser/src/coding_adventures/excel_parser/init.lua
-- No grammar file is loaded here — all grammar knowledge is in this file.

local excel_lexer = require("coding_adventures.excel_lexer")

local M = {}
M.VERSION = "0.1.0"

-- =========================================================================
-- Internal parse state
-- =========================================================================
--
-- Module-level state (not re-entrant, but parsing is always synchronous).

local _tokens  -- token list from the lexer
local _pos     -- current 1-based index

-- =========================================================================
-- Low-level helpers
-- =========================================================================

--- Return the token at the current position without consuming it.
local function peek()
    return _tokens[_pos] or _tokens[#_tokens]
end

--- Consume and return the current token, advancing position.
local function advance()
    local t = peek()
    _pos = _pos + 1
    return t
end

--- Expect the current token to be of type `typ`; consume and return it.
-- Raises a descriptive error if the type does not match.
local function expect(typ)
    local t = peek()
    if t.type ~= typ then
        error(string.format(
            "excel_parser: expected %s but got %s ('%s') at line %d col %d",
            typ, t.type, t.value, t.line, t.col
        ))
    end
    return advance()
end

--- Return true if the current token has the given type.
local function check(typ)
    return peek().type == typ
end

--- Consume and skip any SPACE tokens.
-- In most formula positions spaces are insignificant; only when the parser
-- explicitly wants the intersection operator does it not call skip_spaces().
local function skip_spaces()
    while check("SPACE") do
        advance()
    end
end

-- =========================================================================
-- AST node constructor
-- =========================================================================

local function node(kind, fields)
    local n = fields or {}
    n.kind = kind
    return n
end

-- =========================================================================
-- Forward declarations
-- =========================================================================
-- (Lua requires these because the grammar productions are mutually recursive.)

local parse_expression
local parse_primary

-- =========================================================================
-- Expression productions — one function per precedence level
-- =========================================================================

-- ---- comparison_expr -------------------------------------------------------
-- comparison_expr = concat_expr { comp_op concat_expr }

local COMP_OPS = {
    EQUALS         = true,
    NOT_EQUALS     = true,
    LESS_THAN      = true,
    LESS_EQUALS    = true,
    GREATER_THAN   = true,
    GREATER_EQUALS = true,
}

local function parse_concat()  -- forward decl
end

local function parse_comparison()
    skip_spaces()
    local left = parse_concat()
    skip_spaces()
    while COMP_OPS[peek().type] do
        local op = advance()
        skip_spaces()
        local right = parse_concat()
        skip_spaces()
        left = node("binop", { op = op, left = left, right = right })
    end
    return left
end

-- ---- concat_expr -----------------------------------------------------------
-- concat_expr = additive_expr { AMP additive_expr }

local function parse_additive()  -- forward decl
end

parse_concat = function()
    skip_spaces()
    local left = parse_additive()
    skip_spaces()
    while check("AMP") do
        local op = advance()
        skip_spaces()
        local right = parse_additive()
        skip_spaces()
        left = node("binop", { op = op, left = left, right = right })
    end
    return left
end

-- ---- additive_expr ---------------------------------------------------------
-- additive_expr = multiplicative_expr { (PLUS | MINUS) multiplicative_expr }

local function parse_multiplicative()  -- forward decl
end

parse_additive = function()
    skip_spaces()
    local left = parse_multiplicative()
    skip_spaces()
    while check("PLUS") or check("MINUS") do
        local op = advance()
        skip_spaces()
        local right = parse_multiplicative()
        skip_spaces()
        left = node("binop", { op = op, left = left, right = right })
    end
    return left
end

-- ---- multiplicative_expr ---------------------------------------------------
-- multiplicative_expr = power_expr { (STAR | SLASH) power_expr }

local function parse_power()  -- forward decl
end

parse_multiplicative = function()
    skip_spaces()
    local left = parse_power()
    skip_spaces()
    while check("STAR") or check("SLASH") do
        local op = advance()
        skip_spaces()
        local right = parse_power()
        skip_spaces()
        left = node("binop", { op = op, left = left, right = right })
    end
    return left
end

-- ---- power_expr ------------------------------------------------------------
-- power_expr = unary_expr { CARET unary_expr }
-- Left-associative: 2^3^2 = (2^3)^2 = 512  (same as Excel)

local function parse_unary()  -- forward decl
end

parse_power = function()
    skip_spaces()
    local left = parse_unary()
    skip_spaces()
    while check("CARET") do
        local op = advance()
        skip_spaces()
        local right = parse_unary()
        skip_spaces()
        left = node("binop", { op = op, left = left, right = right })
    end
    return left
end

-- ---- unary_expr ------------------------------------------------------------
-- unary_expr = { (PLUS | MINUS) } postfix_expr
-- Right-associative: -(-(A1)) — recurse to allow stacking.

local function parse_postfix()  -- forward decl
end

parse_unary = function()
    skip_spaces()
    if check("PLUS") or check("MINUS") then
        local op      = advance()
        skip_spaces()
        local operand = parse_unary()  -- right-associative recursion
        return node("unop", { op = op, operand = operand })
    end
    return parse_postfix()
end

-- ---- postfix_expr ----------------------------------------------------------
-- postfix_expr = primary { PERCENT }

parse_postfix = function()
    skip_spaces()
    local primary = parse_primary()
    skip_spaces()
    while check("PERCENT") do
        local op = advance()
        primary = node("postfix", { op = op, operand = primary })
        skip_spaces()
    end
    return primary
end

-- ---- Top-level expression entry point -------------------------------------

parse_expression = function()
    return parse_comparison()
end

-- =========================================================================
-- Argument list
-- =========================================================================
-- arg_list = arg { (COMMA | SEMICOLON) arg }
-- arg      = [ expression ]
-- Empty args (nil) represent omitted arguments: =IF(,TRUE,FALSE)

local function parse_arg_list()
    local args = {}

    if check("RPAREN") then
        return args  -- empty arg list
    end

    -- First argument
    skip_spaces()
    if check("COMMA") or check("SEMICOLON") then
        args[#args + 1] = nil  -- empty first arg
    else
        args[#args + 1] = parse_expression()
    end
    skip_spaces()

    while check("COMMA") or check("SEMICOLON") do
        advance()  -- consume separator
        skip_spaces()
        if check("RPAREN") then break end  -- trailing separator
        if check("COMMA") or check("SEMICOLON") then
            args[#args + 1] = nil  -- empty middle arg
        else
            args[#args + 1] = parse_expression()
        end
        skip_spaces()
    end

    return args
end

-- =========================================================================
-- Array constant
-- =========================================================================
-- array_constant = LBRACE array_row { SEMICOLON array_row } RBRACE
-- array_row      = array_item { COMMA array_item }
-- array_item     = [ (PLUS|MINUS) ] (NUMBER|STRING|BOOL|ERROR_CONSTANT)

local function parse_array_item()
    skip_spaces()
    if check("MINUS") or check("PLUS") then
        local sign = advance()
        skip_spaces()
        local num  = expect("NUMBER")
        return node("unop", { op = sign, operand = node("number", { token = num }) })
    end
    if check("NUMBER")         then return node("number",  { token = advance() }) end
    if check("STRING")         then return node("string",  { token = advance() }) end
    if check("TRUE") or check("FALSE") then
        return node("bool", { token = advance() })
    end
    if check("ERROR_CONSTANT") then return node("error",   { token = advance() }) end
    error(string.format(
        "excel_parser: expected array item, got %s ('%s') at line %d col %d",
        peek().type, peek().value, peek().line, peek().col
    ))
end

local function parse_array_row()
    local items = {}
    items[#items + 1] = parse_array_item()
    skip_spaces()
    while check("COMMA") do
        advance()
        items[#items + 1] = parse_array_item()
        skip_spaces()
    end
    return items
end

local function parse_array_constant()
    expect("LBRACE")
    skip_spaces()
    local rows = {}
    rows[#rows + 1] = parse_array_row()
    skip_spaces()
    while check("SEMICOLON") do
        advance()
        skip_spaces()
        if check("RBRACE") then break end  -- trailing semicolon
        rows[#rows + 1] = parse_array_row()
        skip_spaces()
    end
    expect("RBRACE")
    return node("array", { rows = rows })
end

-- =========================================================================
-- Primary expression
-- =========================================================================
--
-- primary = LPAREN expr RPAREN
--         | LBRACE array_constant RBRACE  (array constant)
--         | ERROR_CONSTANT
--         | STRING
--         | NUMBER
--         | TRUE | FALSE
--         | REF_PREFIX [ CELL | NAME ] [ COLON … ]
--         | CELL [ COLON … ]
--         | NAME LPAREN args RPAREN       (function call)
--         | NAME

parse_primary = function()
    skip_spaces()
    local t = peek()

    -- ---- ( expr ) -----------------------------------------------------------
    if t.type == "LPAREN" then
        advance()
        skip_spaces()
        local inner = parse_expression()
        skip_spaces()
        expect("RPAREN")
        return node("group", { expr = inner })
    end

    -- ---- {array} ------------------------------------------------------------
    if t.type == "LBRACE" then
        return parse_array_constant()
    end

    -- ---- Error constant: #DIV/0! etc. --------------------------------------
    if t.type == "ERROR_CONSTANT" then
        return node("error", { token = advance() })
    end

    -- ---- String literal -----------------------------------------------------
    if t.type == "STRING" then
        return node("string", { token = advance() })
    end

    -- ---- Number literal -----------------------------------------------------
    if t.type == "NUMBER" then
        return node("number", { token = advance() })
    end

    -- ---- Boolean keyword ---------------------------------------------------
    if t.type == "TRUE" or t.type == "FALSE" then
        return node("bool", { token = advance() })
    end

    -- ---- REF_PREFIX: Sheet1! or 'My Sheet'! --------------------------------
    if t.type == "REF_PREFIX" then
        local prefix = advance()
        skip_spaces()

        if check("CELL") then
            local cell = advance()
            skip_spaces()
            if check("COLON") then
                advance()  -- consume :
                skip_spaces()
                local end_ref
                if check("REF_PREFIX") then
                    local pfx2  = advance()
                    skip_spaces()
                    local cell2 = expect("CELL")
                    end_ref = node("ref_prefix", {
                        prefix = pfx2,
                        ref    = node("cell", { token = cell2 }),
                    })
                else
                    local cell2 = expect("CELL")
                    end_ref = node("cell", { token = cell2 })
                end
                return node("range", {
                    start_ref = node("ref_prefix", {
                        prefix = prefix,
                        ref    = node("cell", { token = cell }),
                    }),
                    end_ref = end_ref,
                })
            end
            return node("ref_prefix", {
                prefix = prefix,
                ref    = node("cell", { token = cell }),
            })
        end

        if check("NAME") then
            local nm = advance()
            return node("ref_prefix", {
                prefix = prefix,
                ref    = node("name", { token = nm }),
            })
        end

        -- Bare prefix (external reference)
        return node("ref_prefix", { prefix = prefix, ref = nil })
    end

    -- ---- CELL reference (possibly start of a range) -------------------------
    if t.type == "CELL" then
        local cell_tok = advance()
        skip_spaces()

        if check("COLON") then
            advance()  -- consume :
            skip_spaces()
            local end_ref
            if check("REF_PREFIX") then
                local pfx2  = advance()
                skip_spaces()
                local cell2 = expect("CELL")
                end_ref = node("ref_prefix", {
                    prefix = pfx2,
                    ref    = node("cell", { token = cell2 }),
                })
            elseif check("CELL") then
                end_ref = node("cell", { token = advance() })
            else
                error(string.format(
                    "excel_parser: expected CELL after COLON, got %s at line %d col %d",
                    peek().type, peek().line, peek().col
                ))
            end
            return node("range", {
                start_ref = node("cell", { token = cell_tok }),
                end_ref   = end_ref,
            })
        end

        return node("cell", { token = cell_tok })
    end

    -- ---- NAME — function call, column range (B:C), or named range -----------
    if t.type == "NAME" then
        local name_tok = advance()
        skip_spaces()

        if check("LPAREN") then
            advance()  -- consume (
            skip_spaces()
            local args = parse_arg_list()
            skip_spaces()
            expect("RPAREN")
            return node("call", { name = name_tok, args = args })
        end

        -- Column range: B:C or B:$C or $B:C (NAME COLON NAME/CELL)
        if check("COLON") then
            advance()  -- consume :
            skip_spaces()
            local end_ref
            if check("NAME") then
                end_ref = node("name", { token = advance() })
            elseif check("CELL") then
                end_ref = node("cell", { token = advance() })
            else
                error(string.format(
                    "excel_parser: expected NAME or CELL after COLON in range, got %s at line %d col %d",
                    peek().type, peek().line, peek().col
                ))
            end
            return node("range", {
                start_ref = node("name", { token = name_tok }),
                end_ref   = end_ref,
            })
        end

        return node("name", { token = name_tok })
    end

    -- ---- Unexpected token ---------------------------------------------------
    error(string.format(
        "excel_parser: unexpected token %s ('%s') at line %d col %d",
        t.type, t.value, t.line, t.col
    ))
end

-- =========================================================================
-- Public API
-- =========================================================================

--- Parse an Excel formula source string and return the root AST node.
--
-- The source may optionally begin with "=" (the formula prefix character
-- that Excel uses to distinguish formula cells from plain text cells).
-- The lexer handles case normalization (lowercasing) before tokenizing.
--
-- # Return value
--
-- Returns a table:  { kind = "formula", eq = token_or_nil, body = expr_node }
--
--   `eq`   — the EQUALS token if the formula started with "=", nil otherwise.
--   `body` — the root expression AST node.
--
-- # Error handling
--
-- Raises an error (via `error()`) on any lexer or parser failure.
--
-- @param source string  The Excel formula text to parse.
-- @return table         The root AST node (kind = "formula").
-- @error                Raises on any lexer or parser failure.
--
-- Example:
--
--   local excel_parser = require("coding_adventures.excel_parser")
--   local ast = excel_parser.parse("=A1+B2")
--   -- ast.kind         → "formula"
--   -- ast.body.kind    → "binop"
--   -- ast.body.op.type → "PLUS"
function M.parse(source)
    local tokens = excel_lexer.tokenize(source)

    _tokens = tokens
    _pos    = 1

    skip_spaces()
    local eq_tok = nil
    if check("EQUALS") then
        eq_tok = advance()
    end
    skip_spaces()

    local body = parse_expression()
    skip_spaces()

    local remaining = peek()
    if remaining.type ~= "EOF" then
        error(string.format(
            "excel_parser: trailing content at line %d col %d: unexpected %s ('%s')",
            remaining.line, remaining.col, remaining.type, remaining.value
        ))
    end

    return node("formula", { eq = eq_tok, body = body })
end

--- Return just the token list for a formula (for testing / introspection).
-- @param source string
-- @return table Token list from excel_lexer.
function M.tokenize(source)
    return excel_lexer.tokenize(source)
end

return M
