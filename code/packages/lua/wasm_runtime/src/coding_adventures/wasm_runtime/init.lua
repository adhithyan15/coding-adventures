-- wasm_runtime -- Complete WebAssembly 1.0 runtime
--
-- This module is part of the coding-adventures project, an educational
-- computing stack built from logic gates up through interpreters.
--
-- ============================================================================
-- WHAT IS A WASM RUNTIME?
-- ============================================================================
--
-- The runtime is the top-level orchestrator that ties together:
--
--   1. Parser     (wasm_module_parser) -- binary -> structured module
--   2. Validator  (wasm_validator)     -- checks structural correctness
--   3. Execution  (wasm_execution)     -- interprets WASM bytecodes
--
-- It provides a convenient API for the common workflow:
--
--   parse -> validate -> instantiate -> call
--
-- ============================================================================
-- ARCHITECTURE
-- ============================================================================
--
--   ┌───────────────────────────────────────────────────────────────────┐
--   │                        WasmRuntime                                │
--   │                                                                   │
--   │   load(bytes)                                                     │
--   │     └── wasm_module_parser.parse(bytes)                           │
--   │                                                                   │
--   │   validate(module)                                                │
--   │     └── wasm_validator.validate(module)                           │
--   │                                                                   │
--   │   instantiate(module)                                             │
--   │     ├── Resolve imports (host functions, memory, tables, globals) │
--   │     ├── Allocate memory and tables                                │
--   │     ├── Initialize globals from const expressions                 │
--   │     ├── Apply data segments to memory                             │
--   │     ├── Apply element segments to tables                          │
--   │     ├── Build export map                                          │
--   │     └── Return WasmInstance                                       │
--   │                                                                   │
--   │   call(instance, name, args)                                      │
--   │     ├── Look up export by name                                    │
--   │     ├── Convert plain numbers to WasmValues                       │
--   │     ├── Create WasmExecutionEngine                                │
--   │     └── engine:call_function(index, wasm_args)                    │
--   │                                                                   │
--   │   load_and_run(bytes, entry, args)                                │
--   │     └── load -> validate -> instantiate -> call                   │
--   └───────────────────────────────────────────────────────────────────┘
--
-- ============================================================================
-- WASM INSTANCE
-- ============================================================================
--
-- A WasmInstance is the live, executable form of a WASM module. It holds:
--
--   - module:          the original parsed module structure
--   - memory:          the allocated LinearMemory (or nil)
--   - tables:          array of Table objects
--   - globals:         array of WasmValue entries
--   - global_types:    array of {type, mutable} descriptors
--   - func_types:      array of FuncType (params + results)
--   - func_bodies:     array of function bodies (nil for imports)
--   - host_functions:  array of host function callbacks (nil for module funcs)
--   - exports:         map from name -> {kind, index}
--
-- ============================================================================
-- Usage
-- ============================================================================
--
--   local wasm_runtime = require("coding_adventures.wasm_runtime")
--
--   local runtime = wasm_runtime.WasmRuntime.new()
--   local results = runtime:load_and_run(wasm_bytes, "square", {5})
--   -- results == {25}
--
-- ============================================================================

local wasm_module_parser = require("coding_adventures.wasm_module_parser")
local wasm_validator = require("coding_adventures.wasm_validator")
local wasm_execution = require("coding_adventures.wasm_execution")

local M = {}

M.VERSION = "0.1.0"


-- ============================================================================
-- EXTERNAL KIND CONSTANTS
-- ============================================================================
--
-- These match the WebAssembly binary format for import/export descriptors.
-- An import or export can be one of four kinds of entity:
--
--   0 = Function   (a callable function)
--   1 = Table      (an array of function references)
--   2 = Memory     (a linear byte array)
--   3 = Global     (a single typed value)

local EXTERNAL_FUNCTION = 0
local EXTERNAL_TABLE    = 1
local EXTERNAL_MEMORY   = 2
local EXTERNAL_GLOBAL   = 3


-- ============================================================================
-- WASM INSTANCE
-- ============================================================================
--
-- A WasmInstance is a table holding the live runtime state of an instantiated
-- WASM module. It is created by WasmRuntime:instantiate() and consumed by
-- WasmRuntime:call().

local WasmInstance = {}
WasmInstance.__index = WasmInstance
M.WasmInstance = WasmInstance

--- Create a new WasmInstance.
-- @param config table  All instance fields.
-- @return WasmInstance
function WasmInstance.new(config)
    local self = setmetatable({}, WasmInstance)
    self.module = config.module
    self.memory = config.memory
    self.tables = config.tables or {}
    self.globals = config.globals or {}
    self.global_types = config.global_types or {}
    self.func_types = config.func_types or {}
    self.func_bodies = config.func_bodies or {}
    self.host_functions = config.host_functions or {}
    self.exports = config.exports or {}
    self.host = config.host
    return self
end


-- ============================================================================
-- WASM RUNTIME
-- ============================================================================

local WasmRuntime = {}
WasmRuntime.__index = WasmRuntime
M.WasmRuntime = WasmRuntime

--- Create a new WasmRuntime.
--
-- @param host table|nil  Optional host environment for resolving imports.
--   The host object should implement:
--     host:resolve_function(module_name, name) -> callable or nil
--     host:resolve_memory(module_name, name) -> LinearMemory or nil
--     host:resolve_table(module_name, name) -> Table or nil
--     host:resolve_global(module_name, name) -> {type, value} or nil
--
-- @return WasmRuntime
function WasmRuntime.new(host)
    local self = setmetatable({}, WasmRuntime)
    self._host = host
    return self
end


--- Parse a WASM binary (string of bytes) into a module table.
--
-- The parser reads the binary format and produces a structured Lua table
-- with fields like types, imports, functions, exports, codes, etc.
--
-- @param wasm_bytes string  The raw WASM binary data.
-- @return table  The parsed module.
function WasmRuntime:load(wasm_bytes)
    return wasm_module_parser.parse(wasm_bytes)
end


--- Validate a parsed module.
--
-- Checks structural correctness: type indices, memory limits, export
-- uniqueness, function/code count match, etc.
--
-- @param module table  The parsed module from load().
-- @return boolean, table|string  true + validated info, or false + error msg.
function WasmRuntime:validate(module)
    return wasm_validator.validate(module)
end


--- Instantiate a parsed WASM module into a live WasmInstance.
--
-- This is where the module comes to life:
--   1. Resolve imports from the host environment
--   2. Allocate linear memory
--   3. Allocate tables
--   4. Initialize globals from constant expressions
--   5. Apply data segments to memory
--   6. Apply element segments to tables
--   7. Build the export name -> {kind, index} map
--
-- @param module table  The parsed module from load().
-- @return WasmInstance  The live instance ready for function calls.
function WasmRuntime:instantiate(module)
    local func_types = {}
    local func_bodies = {}
    local host_functions = {}
    local global_types = {}
    local globals_list = {}
    local memory = nil
    local tables = {}

    -- ── Step 1: Resolve imports ─────────────────────────────────────
    --
    -- Imports come first in each index space. We process them to build
    -- the initial func_types, func_bodies (nil for imports), and
    -- host_functions arrays.

    -- The parser stores imports in one of two formats:
    --   1. { mod = "env", name = "foo", desc = { kind = "func", type_idx = N } }
    --   2. { kind = 0, type_index = N, module_name = "env", name = "foo" }
    -- We normalize here.

    for _, imp in ipairs(module.imports or {}) do
        local imp_kind, imp_mod, imp_name
        if imp.desc then
            -- Parser format
            imp_kind = imp.desc.kind
            imp_mod = imp.mod
            imp_name = imp.name
        else
            -- Direct format
            local NUMERIC_KIND_MAP = { [0] = "func", [1] = "table", [2] = "mem", [3] = "global" }
            imp_kind = NUMERIC_KIND_MAP[imp.kind] or imp.kind
            imp_mod = imp.module_name
            imp_name = imp.name
        end

        if imp_kind == "func" then
            local type_idx = (imp.desc and imp.desc.type_idx) or imp.type_index or imp.typeInfo or 0
            func_types[#func_types + 1] = module.types[type_idx + 1]
            func_bodies[#func_bodies + 1] = nil
            local host_func = nil
            if self._host and self._host.resolve_function then
                host_func = self._host:resolve_function(imp_mod, imp_name)
            end
            host_functions[#host_functions + 1] = host_func

        elseif imp_kind == "mem" then
            if self._host and self._host.resolve_memory then
                local imported_mem = self._host:resolve_memory(imp_mod, imp_name)
                if imported_mem then
                    memory = imported_mem
                end
            end

        elseif imp_kind == "table" then
            if self._host and self._host.resolve_table then
                local imported_table = self._host:resolve_table(imp_mod, imp_name)
                if imported_table then
                    tables[#tables + 1] = imported_table
                end
            end

        elseif imp_kind == "global" then
            if self._host and self._host.resolve_global then
                local imported_global = self._host:resolve_global(imp_mod, imp_name)
                if imported_global then
                    global_types[#global_types + 1] = imported_global.type_info
                    globals_list[#globals_list + 1] = imported_global.value
                end
            end
        end
    end

    -- ── Step 2: Add module-defined functions ─────────────────────────
    --
    -- Each entry in module.functions is a type index. The corresponding
    -- body is in module.codes at the same position.

    for i, type_idx in ipairs(module.functions or {}) do
        func_types[#func_types + 1] = module.types[type_idx + 1]
        local code_entry = (module.codes or {})[i]
        func_bodies[#func_bodies + 1] = code_entry
        host_functions[#host_functions + 1] = nil
    end

    -- ── Step 3: Allocate memory ──────────────────────────────────────
    --
    -- If no memory was imported and the module defines one, allocate it.
    -- WASM 1.0 allows at most one memory.

    if memory == nil and #(module.memories or {}) > 0 then
        local mem_type = module.memories[1]
        local limits = mem_type.limits or mem_type
        local min_pages = limits.min or 0
        local max_pages = limits.max
        memory = wasm_execution.LinearMemory.new(min_pages, max_pages)
    end

    -- ── Step 4: Allocate tables ──────────────────────────────────────

    for _, table_type in ipairs(module.tables or {}) do
        local limits = table_type.limits or table_type
        tables[#tables + 1] = wasm_execution.Table.new(
            limits.min or 0,
            limits.max
        )
    end

    -- ── Step 5: Initialize globals ───────────────────────────────────
    --
    -- Each global has a type and an initializer expression (a constant
    -- expression that may reference previously defined globals).

    for _, global_def in ipairs(module.globals or {}) do
        global_types[#global_types + 1] = global_def.global_type or global_def.type_info
        local init_expr = global_def.init_expr or global_def.init or {}
        local value = wasm_execution.evaluate_const_expr(init_expr, globals_list)
        globals_list[#globals_list + 1] = value
    end

    -- ── Step 6: Apply data segments ──────────────────────────────────
    --
    -- Data segments initialize regions of linear memory with bytes.
    -- Each segment has an offset expression and a byte array.

    if memory ~= nil then
        for _, seg in ipairs(module.data or {}) do
            if type(seg) == "table" and seg.offset_expr then
                local offset = wasm_execution.evaluate_const_expr(
                    seg.offset_expr, globals_list)
                memory:write_bytes(offset.value, seg.data)
            end
        end
    end

    -- ── Step 7: Apply element segments ───────────────────────────────
    --
    -- Element segments initialize table entries with function indices.

    for _, elem in ipairs(module.elements or {}) do
        if type(elem) == "table" and elem.table_index then
            local tbl_idx = elem.table_index
            if tables[tbl_idx + 1] then
                local tbl = tables[tbl_idx + 1]
                local offset = wasm_execution.evaluate_const_expr(
                    elem.offset_expr or {}, globals_list)
                local offset_num = offset.value
                for j, func_idx in ipairs(elem.function_indices or {}) do
                    tbl:set(offset_num + j - 1, func_idx)
                end
            end
        end
    end

    -- ── Step 8: Build export map ─────────────────────────────────────
    --
    -- The export map provides O(1) lookup from export name to the kind
    -- and index of the exported entity.
    --
    -- The parser stores exports in one of two formats:
    --   1. { name, desc = { kind = "func"|"table"|..., idx = N } }
    --   2. { name, kind = 0|1|2|3, index = N }
    -- We normalize to { kind = numeric, index = N }.

    local KIND_MAP = { func = 0, table = 1, mem = 2, global = 3 }

    local exports = {}
    for _, exp in ipairs(module.exports or {}) do
        local kind, index
        if exp.desc then
            -- Parser format: { name, desc = { kind = "func", idx = N } }
            kind = KIND_MAP[exp.desc.kind] or exp.desc.kind
            index = exp.desc.idx
        else
            -- Direct format: { name, kind = N, index = N }
            kind = exp.kind
            index = exp.index
        end
        exports[exp.name] = { kind = kind, index = index }
    end

    -- ── Step 9: Construct the instance ───────────────────────────────

    local instance = WasmInstance.new({
        module = module,
        memory = memory,
        tables = tables,
        globals = globals_list,
        global_types = global_types,
        func_types = func_types,
        func_bodies = func_bodies,
        host_functions = host_functions,
        exports = exports,
        host = self._host,
    })

    -- Set memory on the host if it supports it (e.g., WASI stubs).
    if self._host and self._host.set_memory and memory then
        self._host:set_memory(memory)
    end

    -- ── Step 10: Call start function ─────────────────────────────────
    --
    -- If the module declares a start function, it is automatically
    -- called during instantiation (before any exports can be called).

    if module.start ~= nil then
        local engine = wasm_execution.WasmExecutionEngine.new({
            memory = instance.memory,
            tables = instance.tables,
            globals = instance.globals,
            global_types = instance.global_types,
            func_types = instance.func_types,
            func_bodies = instance.func_bodies,
            host_functions = instance.host_functions,
        })
        engine:call_function(module.start, {})
    end

    return instance
end


--- Call an exported function by name on an instance.
--
-- Looks up the export, converts plain Lua numbers to WasmValues based
-- on the function's parameter types, invokes the execution engine, and
-- returns the result values as plain Lua numbers.
--
-- @param instance WasmInstance  The instantiated module.
-- @param name string            The export name (e.g., "square").
-- @param args table             Array of plain Lua numbers.
-- @return table  Array of plain Lua number results.
function WasmRuntime:call(instance, name, args)
    args = args or {}
    local exp = instance.exports[name]
    if not exp then
        error("TrapError: export \"" .. name .. "\" not found")
    end
    if exp.kind ~= EXTERNAL_FUNCTION then
        error("TrapError: export \"" .. name .. "\" is not a function")
    end

    local func_type = instance.func_types[exp.index + 1]
    if not func_type then
        error("TrapError: no type for function index " .. exp.index)
    end

    -- Convert plain numbers to WasmValues using the function's parameter types.
    local wasm_args = {}
    for i, arg in ipairs(args) do
        local param_type = func_type.params[i] or 0x7F  -- default to i32
        if param_type == 0x7F then
            wasm_args[i] = wasm_execution.i32(math.floor(arg))
        elseif param_type == 0x7E then
            wasm_args[i] = wasm_execution.i64(math.floor(arg))
        elseif param_type == 0x7D then
            wasm_args[i] = wasm_execution.f32(arg + 0.0)
        elseif param_type == 0x7C then
            wasm_args[i] = wasm_execution.f64(arg + 0.0)
        else
            wasm_args[i] = wasm_execution.i32(math.floor(arg))
        end
    end

    -- Create a fresh execution engine for this call.
    local engine = wasm_execution.WasmExecutionEngine.new({
        memory = instance.memory,
        tables = instance.tables,
        globals = instance.globals,
        global_types = instance.global_types,
        func_types = instance.func_types,
        func_bodies = instance.func_bodies,
        host_functions = instance.host_functions,
    })

    local results = engine:call_function(exp.index, wasm_args)

    -- Convert WasmValues back to plain Lua numbers.
    local plain_results = {}
    for i, r in ipairs(results) do
        plain_results[i] = r.value
    end
    return plain_results
end


--- Parse, validate, instantiate, and call in one step.
--
-- This is the most convenient entry point for simple use cases.
--
-- @param wasm_bytes string      The raw WASM binary data.
-- @param entry string           The export name to call (default: "_start").
-- @param args table             Array of plain Lua numbers (default: {}).
-- @return table  Array of plain Lua number results.
function WasmRuntime:load_and_run(wasm_bytes, entry, args)
    entry = entry or "_start"
    args = args or {}

    local module = self:load(wasm_bytes)
    local ok, err = self:validate(module)
    if not ok then
        error("ValidationError: " .. tostring(err))
    end

    local instance = self:instantiate(module)
    return self:call(instance, entry, args)
end


-- ============================================================================
-- RE-EXPORTS
-- ============================================================================
--
-- For convenience, re-export commonly used types from wasm_execution so
-- callers don't need to require both modules.

M.LinearMemory = wasm_execution.LinearMemory
M.Table = wasm_execution.Table
M.i32 = wasm_execution.i32
M.i64 = wasm_execution.i64
M.f32 = wasm_execution.f32
M.f64 = wasm_execution.f64
M.trap = wasm_execution.trap


return M
