-- mosaic_parser — Hand-written recursive descent parser for the Mosaic language
-- ==============================================================================
--
-- # What does the parser do?
--
-- The lexer produces a flat stream of tokens:
--
--   COMPONENT NAME LBRACE SLOT NAME COLON KEYWORD SEMICOLON ...
--
-- The parser takes that stream and builds a nested AST (Abstract Syntax Tree)
-- that reflects the *structure* of the Mosaic component:
--
--   file
--   └── component_decl
--       ├── name: "ProfileCard"
--       ├── slot_decl (name="title", type="text")
--       ├── slot_decl (name="count", type="number", default=0)
--       └── node_tree
--           └── node_element (tag="Column")
--               ├── node_content → property_assignment
--               │   (name="padding", value=16dp)
--               └── node_content → node_element (tag="Text")
--                   └── node_content → property_assignment
--                       (name="content", value=@title)
--
-- # Grammar
--
-- The full grammar is in code/grammars/mosaic.grammar. Key rules:
--
--   file           = { import_decl } component_decl
--   import_decl    = IMPORT NAME [ AS NAME ] FROM STRING SEMICOLON
--   component_decl = COMPONENT NAME LBRACE { slot_decl } node_tree RBRACE
--   slot_decl      = SLOT NAME COLON slot_type [ EQUALS default_value ] SEMICOLON
--   slot_type      = KEYWORD | NAME | list_type
--   list_type      = KEYWORD(list) LANGLE slot_type RANGLE
--   default_value  = STRING | NUMBER | DIMENSION | HEX_COLOR | KEYWORD
--   node_tree      = node_element
--   node_element   = NAME LBRACE { node_content } RBRACE
--   node_content   = property_assignment | child_node | slot_reference
--                  | when_block | each_block
--   property_assignment = (NAME|KEYWORD) COLON property_value SEMICOLON
--   property_value = AT NAME | STRING | DIMENSION | NUMBER | HEX_COLOR
--                  | KEYWORD | NAME DOT NAME | NAME
--   slot_reference = AT NAME SEMICOLON
--   when_block     = WHEN AT NAME LBRACE { node_content } RBRACE
--   each_block     = EACH AT NAME AS NAME LBRACE { node_content } RBRACE
--
-- # AST Node Format
--
-- Each AST node is a Lua table:
--
--   { rule = "component_decl",
--     name = "ProfileCard",
--     slots = { ... },
--     tree  = { ... }   -- the root node_element
--   }
--
-- Tokens inside nodes are stored as plain tables:
--
--   { type = "NAME", value = "title", line = 2, col = 8 }
--
-- # API
--
--   MosaicParser.parse(source) → ast, nil      on success
--                               → nil, errmsg  on failure
--
-- The returned `ast` is the root file node.

local mosaic_lexer = require("coding_adventures.mosaic_lexer")

local M = {}
M.VERSION = "0.1.0"

-- ============================================================================
-- Parser State
-- ============================================================================

--- Create a new parser state from a token list.
-- @param tokens table  Flat array of token tables from the lexer.
-- @return table        Mutable parser state.
local function new_parser(tokens)
    return {
        tokens = tokens,
        pos    = 1,
        len    = #tokens,
    }
end

--- Return the current token without consuming it.
-- @param p table  Parser state.
-- @return table   The current token.
local function current(p)
    return p.tokens[p.pos] or { type = "EOF", value = "", line = 0, col = 0 }
end

--- Look ahead by `n` positions (0 = current, 1 = next, etc.).
-- @param p table   Parser state.
-- @param n number  Offset.
-- @return table    Token at that offset.
local function lookahead(p, n)
    local i = p.pos + (n or 0)
    return p.tokens[i] or { type = "EOF", value = "", line = 0, col = 0 }
end

--- Advance past the current token and return it.
-- @param p table  Parser state.
-- @return table   The consumed token.
local function consume(p)
    local tok = current(p)
    p.pos = p.pos + 1
    return tok
end

--- Expect a token of `typ`, consume it, and return it.
-- Raises an error if the current token does not match.
-- @param p   table   Parser state.
-- @param typ string  Expected token type.
-- @return table      The matched token.
local function expect(p, typ)
    local tok = current(p)
    if tok.type ~= typ then
        error(
            ("mosaic_parser: expected %s but got %s (%q) at line %d:%d"):format(
                typ, tok.type, tok.value, tok.line, tok.col
            )
        )
    end
    return consume(p)
end

--- Expect a specific token type AND value.
-- @param p     table   Parser state.
-- @param typ   string  Expected token type.
-- @param value string  Expected token value.
-- @return table        The matched token.
local function expect_value(p, typ, value)
    local tok = current(p)
    if tok.type ~= typ or tok.value ~= value then
        error(
            ("mosaic_parser: expected %s(%q) but got %s(%q) at line %d:%d"):format(
                typ, value, tok.type, tok.value, tok.line, tok.col
            )
        )
    end
    return consume(p)
end

--- Return true if the current token matches `typ` (optionally also `value`).
-- @param p     table   Parser state.
-- @param typ   string  Token type to test.
-- @param value string  Optional: also require this value.
-- @return boolean
local function check(p, typ, value)
    local tok = current(p)
    if tok.type ~= typ then return false end
    if value ~= nil and tok.value ~= value then return false end
    return true
end

--- Consume the current token if it matches `typ` (and optionally `value`).
-- Returns the consumed token or nil.
-- @param p     table   Parser state.
-- @param typ   string  Token type to match.
-- @param value string  Optional value constraint.
-- @return table|nil    Token if matched, nil otherwise.
local function match(p, typ, value)
    if check(p, typ, value) then
        return consume(p)
    end
    return nil
end

-- ============================================================================
-- Grammar Rules
-- ============================================================================

-- Forward declarations for mutually recursive rules
local parse_node_element
local parse_node_content

-- ============================================================================
-- import_decl = IMPORT NAME [ AS NAME ] FROM STRING SEMICOLON

--- Parse an import declaration.
-- @param p table  Parser state.
-- @return table   import_decl node.
local function parse_import_decl(p)
    expect(p, "IMPORT")
    local component_name = expect(p, "NAME").value
    local alias = nil
    if check(p, "AS") then
        consume(p)
        alias = expect(p, "NAME").value
    end
    -- "from" is tokenized as the FROM control keyword
    expect(p, "FROM")
    local path = expect(p, "STRING").value
    expect(p, "SEMICOLON")
    return {
        rule           = "import_decl",
        component_name = component_name,
        alias          = alias,
        path           = path,
    }
end

-- ============================================================================
-- slot_type = KEYWORD | NAME | list_type
-- list_type = KEYWORD(list) LANGLE slot_type RANGLE

--- Parse a slot type annotation.
-- @param p table  Parser state.
-- @return table   slot_type node: { kind, ... }
local function parse_slot_type(p)
    -- list<X>
    if check(p, "KEYWORD", "list") then
        consume(p)  -- consume "list"
        expect(p, "LANGLE")
        local element_type = parse_slot_type(p)  -- recursive for list<list<X>>
        expect(p, "RANGLE")
        return { kind = "list", element_type = element_type }
    end

    -- Primitive type keyword: text, number, bool, image, color, node
    if check(p, "KEYWORD") then
        local kw = consume(p).value
        return { kind = "primitive", name = kw }
    end

    -- Component type name
    if check(p, "NAME") then
        local name = consume(p).value
        return { kind = "component", name = name }
    end

    local tok = current(p)
    error(("mosaic_parser: expected slot type at line %d:%d, got %s(%q)"):format(
        tok.line, tok.col, tok.type, tok.value
    ))
end

-- ============================================================================
-- default_value = STRING | NUMBER | DIMENSION | HEX_COLOR | KEYWORD

--- Parse a slot default value.
-- @param p table  Parser state.
-- @return table   default_value node: { kind, value }
local function parse_default_value(p)
    if check(p, "STRING") then
        return { kind = "string", value = consume(p).value }
    end
    if check(p, "DIMENSION") then
        return { kind = "dimension", value = consume(p).value }
    end
    if check(p, "NUMBER") then
        return { kind = "number", value = consume(p).value }
    end
    if check(p, "HEX_COLOR") then
        return { kind = "color_hex", value = consume(p).value }
    end
    if check(p, "KEYWORD") then
        local v = consume(p).value
        if v == "true"  then return { kind = "bool", value = true  } end
        if v == "false" then return { kind = "bool", value = false } end
        return { kind = "ident", value = v }
    end

    local tok = current(p)
    error(("mosaic_parser: expected default value at line %d:%d, got %s(%q)"):format(
        tok.line, tok.col, tok.type, tok.value
    ))
end

-- ============================================================================
-- slot_decl = SLOT NAME COLON slot_type [ EQUALS default_value ] SEMICOLON

--- Parse a slot declaration.
-- @param p table  Parser state.
-- @return table   slot_decl node.
local function parse_slot_decl(p)
    expect(p, "SLOT")
    local name = expect(p, "NAME").value
    expect(p, "COLON")
    local slot_type = parse_slot_type(p)
    local default_value = nil
    if match(p, "EQUALS") then
        default_value = parse_default_value(p)
    end
    expect(p, "SEMICOLON")
    return {
        rule          = "slot_decl",
        name          = name,
        slot_type     = slot_type,
        default_value = default_value,
        required      = (default_value == nil),
    }
end

-- ============================================================================
-- property_value = AT NAME | STRING | DIMENSION | NUMBER | HEX_COLOR
--                | KEYWORD | NAME DOT NAME | NAME

--- Parse a property value.
-- @param p table  Parser state.
-- @return table   property_value node: { kind, ... }
local function parse_property_value(p)
    -- Slot reference: @slotName
    if check(p, "AT") then
        consume(p)
        local name = expect(p, "NAME").value
        return { kind = "slot_ref", slot_name = name }
    end

    if check(p, "STRING") then
        return { kind = "string", value = consume(p).value }
    end

    if check(p, "DIMENSION") then
        return { kind = "dimension", value = consume(p).value }
    end

    if check(p, "NUMBER") then
        return { kind = "number", value = consume(p).value }
    end

    if check(p, "HEX_COLOR") then
        return { kind = "color_hex", value = consume(p).value }
    end

    -- KEYWORD values: true, false, or type keywords used as enum-like values
    if check(p, "KEYWORD") then
        local v = consume(p).value
        if v == "true"  then return { kind = "bool", value = true  } end
        if v == "false" then return { kind = "bool", value = false } end
        return { kind = "ident", value = v }
    end

    -- NAME possibly followed by DOT NAME (enum value like Alignment.center)
    if check(p, "NAME") then
        local name = consume(p).value
        if check(p, "DOT") then
            consume(p)
            local member = expect(p, "NAME").value
            return { kind = "enum", namespace = name, member = member }
        end
        return { kind = "ident", value = name }
    end

    local tok = current(p)
    error(("mosaic_parser: expected property value at line %d:%d, got %s(%q)"):format(
        tok.line, tok.col, tok.type, tok.value
    ))
end

-- ============================================================================
-- property_assignment = (NAME | KEYWORD) COLON property_value SEMICOLON

--- Parse a property assignment.
-- @param p table  Parser state.
-- @return table   property_assignment node.
local function parse_property_assignment(p)
    -- Property name can be NAME or KEYWORD (e.g., "color:", "node:")
    local name
    if check(p, "NAME") then
        name = consume(p).value
    elseif check(p, "KEYWORD") then
        name = consume(p).value
    else
        local tok = current(p)
        error(("mosaic_parser: expected property name at line %d:%d"):format(tok.line, tok.col))
    end
    expect(p, "COLON")
    local value = parse_property_value(p)
    expect(p, "SEMICOLON")
    return {
        rule  = "property_assignment",
        name  = name,
        value = value,
    }
end

-- ============================================================================
-- slot_reference = AT NAME SEMICOLON

--- Parse a slot reference used as a child element: @slotName;
-- Precondition: current is AT.
-- @param p table  Parser state.
-- @return table   slot_reference node.
local function parse_slot_reference(p)
    expect(p, "AT")
    local name = expect(p, "NAME").value
    expect(p, "SEMICOLON")
    return {
        rule      = "slot_reference",
        slot_name = name,
    }
end

-- ============================================================================
-- when_block = WHEN AT NAME LBRACE { node_content } RBRACE

--- Parse a conditional when block.
-- Precondition: current is WHEN.
-- @param p table  Parser state.
-- @return table   when_block node.
local function parse_when_block(p)
    expect(p, "WHEN")
    expect(p, "AT")
    local slot_name = expect(p, "NAME").value
    expect(p, "LBRACE")
    local children = {}
    while not check(p, "RBRACE") and not check(p, "EOF") do
        children[#children + 1] = parse_node_content(p)
    end
    expect(p, "RBRACE")
    return {
        rule      = "when_block",
        slot_name = slot_name,
        children  = children,
    }
end

-- ============================================================================
-- each_block = EACH AT NAME AS NAME LBRACE { node_content } RBRACE

--- Parse an iteration each block.
-- Precondition: current is EACH.
-- @param p table  Parser state.
-- @return table   each_block node.
local function parse_each_block(p)
    expect(p, "EACH")
    expect(p, "AT")
    local slot_name = expect(p, "NAME").value
    expect(p, "AS")
    local item_name = expect(p, "NAME").value
    expect(p, "LBRACE")
    local children = {}
    while not check(p, "RBRACE") and not check(p, "EOF") do
        children[#children + 1] = parse_node_content(p)
    end
    expect(p, "RBRACE")
    return {
        rule      = "each_block",
        slot_name = slot_name,
        item_name = item_name,
        children  = children,
    }
end

-- ============================================================================
-- node_element = NAME LBRACE { node_content } RBRACE

--- Parse a node element.
-- Precondition: current is NAME.
-- @param p table  Parser state.
-- @return table   node_element node.
parse_node_element = function(p)
    local tag = expect(p, "NAME").value
    expect(p, "LBRACE")
    local properties = {}
    local children   = {}
    while not check(p, "RBRACE") and not check(p, "EOF") do
        local content = parse_node_content(p)
        if content.rule == "property_assignment" then
            properties[#properties + 1] = content
        else
            children[#children + 1] = content
        end
    end
    expect(p, "RBRACE")
    return {
        rule       = "node_element",
        tag        = tag,
        properties = properties,
        children   = children,
    }
end

-- ============================================================================
-- node_content = property_assignment | child_node | slot_reference
--              | when_block | each_block

--- Parse a single node content item.
--
-- Disambiguation rules:
--   WHEN token          → when_block
--   EACH token          → each_block
--   AT NAME SEMICOLON   → slot_reference
--   AT NAME LBRACE      → impossible by grammar; LBRACE → slot_ref error
--   NAME LBRACE         → child node_element (child_node)
--   (NAME|KEYWORD) COLON → property_assignment
--
-- @param p table  Parser state.
-- @return table   One of: property_assignment, node_element, slot_reference,
--                         when_block, each_block.
parse_node_content = function(p)
    if check(p, "WHEN") then
        return parse_when_block(p)
    end

    if check(p, "EACH") then
        return parse_each_block(p)
    end

    -- slot_reference: @slotName;
    if check(p, "AT") then
        return parse_slot_reference(p)
    end

    -- Look ahead to distinguish:
    --   NAME LBRACE → child node_element
    --   NAME COLON  → property_assignment
    --   KEYWORD COLON → property_assignment (for e.g. "color: ...")
    if check(p, "NAME") then
        if lookahead(p, 1).type == "LBRACE" then
            return parse_node_element(p)
        else
            return parse_property_assignment(p)
        end
    end

    if check(p, "KEYWORD") then
        -- KEYWORD can only be a property name here (e.g., "color:", "text:")
        return parse_property_assignment(p)
    end

    local tok = current(p)
    error(("mosaic_parser: unexpected token in node body: %s(%q) at line %d:%d"):format(
        tok.type, tok.value, tok.line, tok.col
    ))
end

-- ============================================================================
-- component_decl = COMPONENT NAME LBRACE { slot_decl } node_tree RBRACE

--- Parse a component declaration.
-- @param p table  Parser state.
-- @return table   component_decl node.
local function parse_component_decl(p)
    expect(p, "COMPONENT")
    local name = expect(p, "NAME").value
    expect(p, "LBRACE")

    local slots = {}
    while check(p, "SLOT") do
        slots[#slots + 1] = parse_slot_decl(p)
    end

    -- The next token must begin the root node element (NAME)
    if not check(p, "NAME") then
        local tok = current(p)
        error(("mosaic_parser: expected node element (NAME) in component %q but got %s(%q) at line %d:%d"):format(
            name, tok.type, tok.value, tok.line, tok.col
        ))
    end

    local tree = parse_node_element(p)
    expect(p, "RBRACE")

    return {
        rule  = "component_decl",
        name  = name,
        slots = slots,
        tree  = tree,
    }
end

-- ============================================================================
-- file = { import_decl } component_decl

--- Parse a complete Mosaic file.
-- @param p table  Parser state.
-- @return table   file node.
local function parse_file(p)
    local imports = {}
    while check(p, "IMPORT") do
        imports[#imports + 1] = parse_import_decl(p)
    end

    if not check(p, "COMPONENT") then
        local tok = current(p)
        error(("mosaic_parser: expected component declaration but got %s(%q) at line %d:%d"):format(
            tok.type, tok.value, tok.line, tok.col
        ))
    end

    local component = parse_component_decl(p)

    -- Trailing content after the component is an error
    if not check(p, "EOF") then
        local tok = current(p)
        error(("mosaic_parser: unexpected tokens after component at line %d:%d"):format(
            tok.line, tok.col
        ))
    end

    return {
        rule      = "file",
        imports   = imports,
        component = component,
    }
end

-- ============================================================================
-- Public API
-- ============================================================================

--- Parse a Mosaic source string into an AST.
--
-- The returned AST is a nested Lua table. On success, the root node has
-- `rule = "file"` and contains `.imports` and `.component`.
--
-- @param source string  The Mosaic source text to parse.
-- @return table|nil     The AST root node, or nil on error.
-- @return nil|string    nil on success, error message on failure.
--
-- Example:
--
--   local parser = require("coding_adventures.mosaic_parser")
--   local ast, err = parser.parse([[
--     component Label {
--       slot text: text;
--       Text { content: @text; }
--     }
--   ]])
--   -- ast.rule              → "file"
--   -- ast.component.name    → "Label"
--   -- ast.component.slots[1].name → "text"
function M.parse(source)
    local toks, lex_err = mosaic_lexer.tokenize(source)
    if not toks then
        return nil, "lex error: " .. tostring(lex_err)
    end

    local ok, result = pcall(function()
        local p = new_parser(toks)
        return parse_file(p)
    end)

    if ok then
        return result, nil
    else
        return nil, result
    end
end

return M
