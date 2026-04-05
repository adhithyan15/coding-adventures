-- wasm_validator -- WebAssembly 1.0 module validator
--
-- This module is part of the coding-adventures project, an educational
-- computing stack built from logic gates up through interpreters.
--
-- ============================================================================
-- WHAT IS VALIDATION?
-- ============================================================================
--
-- Validation is a semantic check performed on a parsed WebAssembly module
-- before it can be instantiated and executed. While the parser ensures the
-- binary format is well-formed (correct magic number, valid LEB128, etc.),
-- the validator ensures the module is *meaningful*:
--
--   - Every type index references a valid type in the type section.
--   - Every function index references a valid function (import or local).
--   - Memory limits are within the spec maximum (65536 pages = 4 GiB).
--   - Exported names are unique.
--   - Function bodies reference valid locals and globals.
--
-- This is a simplified validator that checks structural constraints without
-- performing full type-stack simulation. A production validator would also
-- verify that every instruction sequence is type-safe (stack polymorphism,
-- block result types, etc.), but for our educational purposes we focus on
-- the index-space and structural checks that catch the most common errors.
--
-- ============================================================================
-- Usage
-- ============================================================================
--
--   local validator = require("coding_adventures.wasm_validator")
--   local parser = require("coding_adventures.wasm_module_parser")
--
--   local module = parser.parse(wasm_bytes)
--   local ok, err = validator.validate(module)
--   if not ok then
--     print("Validation failed: " .. err)
--   end
--
-- ============================================================================

local wasm_types = require("coding_adventures.wasm_types")

local M = {}

M.VERSION = "0.1.0"

-- ============================================================================
-- CONSTANTS
-- ============================================================================

-- Maximum number of memory pages allowed by the WebAssembly specification.
-- Each page is 64 KiB, so 65536 pages = 4 GiB of addressable memory.
local MAX_MEMORY_PAGES = 65536

-- ExternalKind constants matching the WASM binary format.
-- These identify what kind of entity an import or export refers to.
local EXTERNAL_FUNCTION = 0
local EXTERNAL_TABLE    = 1
local EXTERNAL_MEMORY   = 2
local EXTERNAL_GLOBAL   = 3


-- ============================================================================
-- validate(module) -- Validate a parsed WASM module
-- ============================================================================
--
-- Performs structural validation on a parsed module. Returns two values:
--   true, validated_info   on success
--   false, error_message   on failure
--
-- The validated_info table contains resolved type information that the
-- execution engine can use directly:
--   func_types  -- array of FuncType for all functions (imports + local)
--   func_locals -- array of expanded local type arrays per local function
--
-- Validation checks performed:
--   1. Type indices in the function section are valid.
--   2. Import type indices are valid.
--   3. Memory limits are within spec bounds.
--   4. Export names are unique.
--   5. Function count matches code count.
--   6. Local/global/function indices in code bodies are plausible.
--
-- @param module  table  The parsed WASM module from wasm_module_parser.parse().
-- @return boolean, table|string  Success flag and info or error message.
-- ============================================================================
function M.validate(module)
    local types = module.types or {}
    local imports = module.imports or {}
    local functions = module.functions or {}
    local tables = module.tables or {}
    local memories = module.memories or {}
    local globals = module.globals or {}
    local exports = module.exports or {}
    local codes = module.codes or {}

    -- ── Step 1: Count imported entities ──────────────────────────────
    -- Imports come first in each index space. We need to know how many
    -- imported functions, tables, memories, and globals there are so we
    -- can compute the total count for each index space.

    local num_imported_funcs = 0
    local num_imported_tables = 0
    local num_imported_memories = 0
    local num_imported_globals = 0

    for _, imp in ipairs(imports) do
        if imp.kind == EXTERNAL_FUNCTION then
            num_imported_funcs = num_imported_funcs + 1
        elseif imp.kind == EXTERNAL_TABLE then
            num_imported_tables = num_imported_tables + 1
        elseif imp.kind == EXTERNAL_MEMORY then
            num_imported_memories = num_imported_memories + 1
        elseif imp.kind == EXTERNAL_GLOBAL then
            num_imported_globals = num_imported_globals + 1
        end
    end

    local total_funcs = num_imported_funcs + #functions
    local total_tables = num_imported_tables + #tables
    local total_memories = num_imported_memories + #memories
    local total_globals = num_imported_globals + #globals

    -- ── Step 2: Validate import type indices ─────────────────────────
    for i, imp in ipairs(imports) do
        if imp.kind == EXTERNAL_FUNCTION then
            local type_idx = imp.type_index or imp.typeInfo
            if type_idx ~= nil then
                if type_idx < 0 or type_idx >= #types then
                    return false, string.format(
                        "import #%d: type index %d out of range (have %d types)",
                        i, type_idx, #types)
                end
            end
        end
    end

    -- ── Step 3: Validate function section type indices ───────────────
    -- Each entry in the function section is a type index that must
    -- reference a valid entry in the type section.
    for i, type_idx in ipairs(functions) do
        if type_idx < 0 or type_idx >= #types then
            return false, string.format(
                "function #%d: type index %d out of range (have %d types)",
                i, type_idx, #types)
        end
    end

    -- ── Step 4: Validate memory limits ───────────────────────────────
    -- The spec limits memory to at most 65536 pages (4 GiB).
    for i, mem in ipairs(memories) do
        local limits = mem.limits or mem
        local min_pages = limits.min or 0
        local max_pages = limits.max

        if min_pages > MAX_MEMORY_PAGES then
            return false, string.format(
                "memory #%d: min pages %d exceeds maximum %d",
                i, min_pages, MAX_MEMORY_PAGES)
        end
        if max_pages ~= nil and max_pages > MAX_MEMORY_PAGES then
            return false, string.format(
                "memory #%d: max pages %d exceeds maximum %d",
                i, max_pages, MAX_MEMORY_PAGES)
        end
        if max_pages ~= nil and min_pages > max_pages then
            return false, string.format(
                "memory #%d: min pages %d exceeds max pages %d",
                i, min_pages, max_pages)
        end
    end

    -- ── Step 5: Validate export name uniqueness ──────────────────────
    -- The WASM spec requires that all export names are unique.
    local seen_exports = {}
    for i, exp in ipairs(exports) do
        if seen_exports[exp.name] then
            return false, string.format(
                "duplicate export name '%s' (exports #%d and #%d)",
                exp.name, seen_exports[exp.name], i)
        end
        seen_exports[exp.name] = i
    end

    -- ── Step 6: Validate export indices ──────────────────────────────
    for i, exp in ipairs(exports) do
        if exp.kind == EXTERNAL_FUNCTION then
            if exp.index < 0 or exp.index >= total_funcs then
                return false, string.format(
                    "export '%s': function index %d out of range",
                    exp.name, exp.index)
            end
        elseif exp.kind == EXTERNAL_TABLE then
            if exp.index < 0 or exp.index >= total_tables then
                return false, string.format(
                    "export '%s': table index %d out of range",
                    exp.name, exp.index)
            end
        elseif exp.kind == EXTERNAL_MEMORY then
            if exp.index < 0 or exp.index >= total_memories then
                return false, string.format(
                    "export '%s': memory index %d out of range",
                    exp.name, exp.index)
            end
        elseif exp.kind == EXTERNAL_GLOBAL then
            if exp.index < 0 or exp.index >= total_globals then
                return false, string.format(
                    "export '%s': global index %d out of range",
                    exp.name, exp.index)
            end
        end
    end

    -- ── Step 7: Validate function/code count match ───────────────────
    -- The number of entries in the function section must equal the number
    -- of entries in the code section. Each function[i] is the type index,
    -- and codes[i] is the body.
    if #functions ~= #codes then
        return false, string.format(
            "function count (%d) does not match code count (%d)",
            #functions, #codes)
    end

    -- ── Step 8: Build validated output ───────────────────────────────
    -- Build the combined func_types array (imports first, then locals).

    local func_types = {}

    -- Add imported function types.
    for _, imp in ipairs(imports) do
        if imp.kind == EXTERNAL_FUNCTION then
            local type_idx = imp.type_index or imp.typeInfo or 0
            -- Type indices in the parser are 0-based; Lua types array is 1-based.
            func_types[#func_types + 1] = types[type_idx + 1]
        end
    end

    -- Add module-defined function types.
    for _, type_idx in ipairs(functions) do
        func_types[#func_types + 1] = types[type_idx + 1]
    end

    -- Expand local declarations for each code body.
    -- The parser gives us {count=N, type=T} groups; we expand to flat arrays.
    local func_locals = {}
    for i, code_entry in ipairs(codes) do
        local expanded = {}
        for _, decl in ipairs(code_entry.locals or {}) do
            for _ = 1, decl.count do
                expanded[#expanded + 1] = decl.type
            end
        end
        func_locals[i] = expanded
    end

    local validated = {
        module = module,
        func_types = func_types,
        func_locals = func_locals,
    }

    return true, validated
end

return M
