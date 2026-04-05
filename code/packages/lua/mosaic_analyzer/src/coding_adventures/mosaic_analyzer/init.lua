-- mosaic_analyzer — Validates the Mosaic AST and produces a typed MosaicIR
-- ==========================================================================
--
-- # What is the analyzer?
--
-- The analyzer is the third stage of the Mosaic compiler pipeline:
--
--   Source text → Lexer → Tokens → Parser → AST → **Analyzer** → MosaicIR
--
-- The parser produces a faithful, unvalidated AST. The analyzer:
--
--   1. Validates structure — checks that required elements are present.
--   2. Resolves types — converts parser slot_type nodes to typed MosaicType tables.
--   3. Normalizes values — parses "16dp" → { kind="dimension", value=16, unit="dp" }.
--   4. Determines required/optional — slots with defaults are optional.
--   5. Builds a clean MosaicIR — a normalized IR ready for the VM/backends.
--
-- # MosaicIR Structure
--
-- The IR is a nested Lua table:
--
--   {
--     component = {
--       name  = "ProfileCard",
--       slots = { { name, type, default_value, required }, ... },
--       tree  = MosaicNode,
--     },
--     imports = { { component_name, alias, path }, ... },
--   }
--
-- A MosaicNode is:
--
--   {
--     tag         = "Column",
--     is_primitive = true,
--     properties  = { { name, value }, ... },
--     children    = { MosaicChild, ... },
--   }
--
-- MosaicChild is one of:
--   { kind="node",      node=MosaicNode }
--   { kind="slot_ref",  slot_name }
--   { kind="when",      slot_name, children }
--   { kind="each",      slot_name, item_name, children }
--
-- MosaicValue is one of:
--   { kind="string",    value }
--   { kind="number",    value }   (Lua number)
--   { kind="bool",      value }
--   { kind="dimension", value, unit }
--   { kind="color_hex", value }   (raw "#rrggbb" string)
--   { kind="slot_ref",  slot_name }
--   { kind="ident",     value }
--   { kind="enum",      namespace, member }
--
-- MosaicType is one of:
--   { kind="text" }
--   { kind="number" }
--   { kind="bool" }
--   { kind="image" }
--   { kind="color" }
--   { kind="node" }
--   { kind="list", element_type=MosaicType }
--   { kind="component", name }
--
-- # Primitive Node Registry
--
-- The built-in layout/display elements are:
--   Row, Column, Box, Stack, Text, Image, Icon, Spacer, Divider, Scroll
--
-- All other node names are treated as component types (imported or inline).

local mosaic_parser = require("coding_adventures.mosaic_parser")

local M = {}
M.VERSION = "0.1.0"

-- ============================================================================
-- Primitive Node Registry
-- ============================================================================

-- The standard set of built-in Mosaic layout and display elements.
-- When a node's tag is in this set, `is_primitive` is set to true.
local PRIMITIVE_NODES = {
    Row     = true,
    Column  = true,
    Box     = true,
    Stack   = true,
    Text    = true,
    Image   = true,
    Icon    = true,
    Spacer  = true,
    Divider = true,
    Scroll  = true,
}

-- ============================================================================
-- Value Helpers
-- ============================================================================

--- Parse a raw dimension string "16dp" → { kind="dimension", value=16, unit="dp" }.
-- The unit can be dp, sp, %, px, em, rem, or any letter sequence.
-- @param raw string  e.g. "16dp", "1.5sp", "100%"
-- @return table      { kind="dimension", value=number, unit=string }
local function parse_dimension(raw)
    local num_str, unit = raw:match("^(-?[0-9]*%.?[0-9]+)([a-zA-Z%%]+)$")
    if not num_str then
        error("mosaic_analyzer: invalid dimension: " .. tostring(raw))
    end
    return { kind = "dimension", value = tonumber(num_str), unit = unit }
end

-- ============================================================================
-- Type Resolution
-- ============================================================================

--- Convert a parser slot_type node → typed MosaicType table.
-- @param slot_type_node table  Parser-produced slot_type node.
-- @return table                MosaicType.
local function resolve_slot_type(slot_type_node)
    local k = slot_type_node.kind
    if k == "list" then
        return { kind = "list", element_type = resolve_slot_type(slot_type_node.element_type) }
    elseif k == "primitive" then
        -- The primitive name is already the kind (text, number, bool, image, color, node)
        return { kind = slot_type_node.name }
    elseif k == "component" then
        return { kind = "component", name = slot_type_node.name }
    else
        error("mosaic_analyzer: unknown slot_type kind: " .. tostring(k))
    end
end

-- ============================================================================
-- Value Normalization
-- ============================================================================

--- Normalize a parser property_value or default_value node → MosaicValue.
-- @param v table  Parser value node with .kind and .value fields.
-- @return table   MosaicValue.
local function normalize_value(v)
    local k = v.kind
    if k == "string" then
        return { kind = "string", value = v.value }
    elseif k == "number" then
        return { kind = "number", value = tonumber(v.value) }
    elseif k == "dimension" then
        return parse_dimension(v.value)
    elseif k == "color_hex" then
        return { kind = "color_hex", value = v.value }
    elseif k == "bool" then
        return { kind = "bool", value = v.value }
    elseif k == "slot_ref" then
        return { kind = "slot_ref", slot_name = v.slot_name }
    elseif k == "ident" then
        return { kind = "ident", value = v.value }
    elseif k == "enum" then
        return { kind = "enum", namespace = v.namespace, member = v.member }
    else
        error("mosaic_analyzer: unknown value kind: " .. tostring(k))
    end
end

-- ============================================================================
-- Node Tree Analysis
-- ============================================================================

-- Forward declaration for mutual recursion (node → children → node)
local analyze_node_element

--- Analyze a when_block child.
-- @param child table  Parser when_block node.
-- @return table       MosaicChild { kind="when", ... }.
local function analyze_when_block(child)
    local children = {}
    for _, c in ipairs(child.children) do
        children[#children + 1] = analyze_node_element(c)
    end
    return { kind = "when", slot_name = child.slot_name, children = children }
end

--- Analyze an each_block child.
-- @param child table  Parser each_block node.
-- @return table       MosaicChild { kind="each", ... }.
local function analyze_each_block(child)
    local children = {}
    for _, c in ipairs(child.children) do
        children[#children + 1] = analyze_node_element(c)
    end
    return {
        kind      = "each",
        slot_name = child.slot_name,
        item_name = child.item_name,
        children  = children,
    }
end

--- Analyze a single AST child node and return a MosaicChild.
-- Used recursively by analyze_node_element for when/each bodies.
-- @param child table  A parser AST child node.
-- @return table       MosaicChild.
analyze_node_element = function(child)
    local r = child.rule
    if r == "node_element" then
        -- Recurse into this nested node
        local tag = child.tag
        local is_prim = PRIMITIVE_NODES[tag] == true

        local properties = {}
        for _, prop in ipairs(child.properties or {}) do
            properties[#properties + 1] = {
                name  = prop.name,
                value = normalize_value(prop.value),
            }
        end

        local children = {}
        for _, c in ipairs(child.children or {}) do
            children[#children + 1] = analyze_node_element(c)
        end

        return {
            kind         = "node",
            node         = {
                tag          = tag,
                is_primitive = is_prim,
                properties   = properties,
                children     = children,
            },
        }
    elseif r == "slot_reference" then
        return { kind = "slot_ref", slot_name = child.slot_name }
    elseif r == "when_block" then
        return analyze_when_block(child)
    elseif r == "each_block" then
        return analyze_each_block(child)
    else
        error("mosaic_analyzer: unexpected child rule: " .. tostring(r))
    end
end

--- Analyze the root node_element (the component tree).
-- @param node_elem table  Parser node_element AST node.
-- @return table           MosaicNode.
local function analyze_root_node(node_elem)
    local tag    = node_elem.tag
    local is_prim = PRIMITIVE_NODES[tag] == true

    local properties = {}
    for _, prop in ipairs(node_elem.properties or {}) do
        properties[#properties + 1] = {
            name  = prop.name,
            value = normalize_value(prop.value),
        }
    end

    local children = {}
    for _, child in ipairs(node_elem.children or {}) do
        children[#children + 1] = analyze_node_element(child)
    end

    return {
        tag          = tag,
        is_primitive = is_prim,
        properties   = properties,
        children     = children,
    }
end

-- ============================================================================
-- Slot Analysis
-- ============================================================================

--- Analyze a single slot_decl node → MosaicSlot.
-- @param slot table  Parser slot_decl node.
-- @return table      MosaicSlot { name, type, default_value, required }.
local function analyze_slot(slot)
    local mosaic_type = resolve_slot_type(slot.slot_type)
    local default_value = nil
    if slot.default_value then
        default_value = normalize_value(slot.default_value)
    end
    return {
        name          = slot.name,
        type          = mosaic_type,
        default_value = default_value,
        required      = slot.required,
    }
end

-- ============================================================================
-- Import Analysis
-- ============================================================================

--- Analyze an import_decl node → MosaicImport.
-- @param imp table  Parser import_decl node.
-- @return table     MosaicImport { component_name, alias, path }.
local function analyze_import(imp)
    return {
        component_name = imp.component_name,
        alias          = imp.alias,
        path           = imp.path,
    }
end

-- ============================================================================
-- File Analysis
-- ============================================================================

--- Analyze a parsed file AST → MosaicIR.
-- @param ast table  Root file AST node from mosaic_parser.
-- @return table     MosaicIR.
local function analyze_file(ast)
    if ast.rule ~= "file" then
        error("mosaic_analyzer: expected root rule 'file', got: " .. tostring(ast.rule))
    end

    -- Analyze imports
    local imports = {}
    for _, imp in ipairs(ast.imports or {}) do
        imports[#imports + 1] = analyze_import(imp)
    end

    -- Analyze component
    local comp = ast.component
    if not comp then
        error("mosaic_analyzer: no component declaration found in file")
    end

    local slots = {}
    for _, slot in ipairs(comp.slots or {}) do
        slots[#slots + 1] = analyze_slot(slot)
    end

    if not comp.tree then
        error(("mosaic_analyzer: component %q has no node tree"):format(comp.name))
    end

    local tree = analyze_root_node(comp.tree)

    return {
        component = {
            name  = comp.name,
            slots = slots,
            tree  = tree,
        },
        imports = imports,
    }
end

-- ============================================================================
-- Public API
-- ============================================================================

--- Analyze a Mosaic source string and return a typed MosaicIR.
--
-- This is the main entry point. It parses the source with mosaic_parser,
-- then validates and normalizes the AST to produce a clean IR.
--
-- @param source string  The Mosaic source text.
-- @return table|nil     The MosaicIR, or nil on error.
-- @return nil|string    nil on success, error message on failure.
--
-- Example:
--
--   local analyzer = require("coding_adventures.mosaic_analyzer")
--   local ir, err = analyzer.analyze([[
--     component Label {
--       slot text: text;
--       Text { content: @text; }
--     }
--   ]])
--   -- ir.component.name      → "Label"
--   -- ir.component.slots[1]  → { name="text", type={kind="text"}, required=true }
--   -- ir.component.tree.tag  → "Text"
function M.analyze(source)
    local ast, parse_err = mosaic_parser.parse(source)
    if not ast then
        return nil, "parse error: " .. tostring(parse_err)
    end

    local ok, result = pcall(analyze_file, ast)
    if ok then
        return result, nil
    else
        return nil, result
    end
end

--- Analyze a pre-parsed AST (skip re-parsing).
-- Use this when you already have an AST from mosaic_parser.parse().
--
-- @param ast table  Root file AST node.
-- @return table|nil MosaicIR, or nil on error.
-- @return nil|string
function M.analyze_ast(ast)
    local ok, result = pcall(analyze_file, ast)
    if ok then
        return result, nil
    else
        return nil, result
    end
end

return M
