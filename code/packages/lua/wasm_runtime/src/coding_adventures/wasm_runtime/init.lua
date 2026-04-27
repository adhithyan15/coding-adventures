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

    local next_func_index = 1

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
            func_types[next_func_index] = module.types[type_idx + 1]
            local host_func = nil
            if self._host and self._host.resolve_function then
                host_func = self._host:resolve_function(imp_mod, imp_name)
            end
            host_functions[next_func_index] = host_func
            next_func_index = next_func_index + 1

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
        func_types[next_func_index] = module.types[type_idx + 1]
        local code_entry = (module.codes or {})[i]
        func_bodies[next_func_index] = code_entry
        next_func_index = next_func_index + 1
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
        global_types[#global_types + 1] = global_def.global_type
            or global_def.type_info
            or { value_type = global_def.val_type, mutable = global_def.mutable }
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


-- ============================================================================
-- CLOCK AND RANDOM INTERFACES
-- ============================================================================
--
-- WASM programs that import WASI clock or random functions need host-provided
-- implementations. We model these as injectable Lua table objects ("duck-typed
-- interfaces") so they can be swapped out for deterministic testing or custom
-- PRNG/clock implementations.
--
-- This follows the Dependency Injection pattern: the WasiStub receives a clock
-- object and a random object at construction time, rather than hardcoding a
-- particular clock or PRNG strategy.
--
-- The interfaces are defined by their method signatures (duck-typed):
--
--   WasiClock interface:
--     clock:realtime_ns()        → integer  (nanoseconds since Unix epoch)
--     clock:monotonic_ns()       → integer  (nanoseconds, monotonically increasing)
--     clock:resolution_ns(id)    → integer  (nanoseconds — precision of the clock)
--
--   WasiRandom interface:
--     random:fill_bytes(n)       → table of n integers in range 0-255

-- ── SystemClock ─────────────────────────────────────────────────────────────
--
-- The default clock implementation that delegates to Lua's standard library.
--
-- Caveat: Lua's standard library only provides second-precision time.
--   os.time()  → integer seconds since Unix epoch
--   os.clock() → CPU time in seconds (good for monotonic ordering)
--
-- For production use requiring sub-second resolution, swap in a clock that
-- calls a C extension or reads /proc/timer_list. For testing, swap in a
-- FakeClock that returns deterministic values.

local SystemClock = {}
SystemClock.__index = SystemClock
M.SystemClock = SystemClock

--- Create a new SystemClock.
-- @return SystemClock
function SystemClock.new()
    return setmetatable({}, SystemClock)
end

--- Return wall-clock time as nanoseconds since the Unix epoch.
--
-- os.time() returns seconds as an integer (UTC). We multiply by 1e9 to
-- convert to nanoseconds. Because Lua 5.3+ integers are 64-bit signed,
-- values up to ~9.2 × 10^18 fit without overflow. The year 2262 is
-- approximately 9.2 × 10^18 ns, so we are safe for centuries.
--
-- @return integer  Nanoseconds since Unix epoch.
function SystemClock:realtime_ns()
    -- math.floor(x * 1e9) works even when os.time() returns an integer,
    -- because Lua will promote to float for the multiply, then floor
    -- truncates back to an integer.
    return math.floor(os.time() * 1e9)
end

--- Return monotonic time as nanoseconds.
--
-- os.clock() returns CPU time consumed by the process. It is monotonically
-- non-decreasing (never goes backward), making it suitable for the WASI
-- monotonic clock. The resolution is platform-dependent (often microseconds
-- or milliseconds). We multiply by 1e9 for nanoseconds.
--
-- @return integer  Nanoseconds of CPU time.
function SystemClock:monotonic_ns()
    return math.floor(os.clock() * 1e9)
end

--- Return the resolution of the given clock in nanoseconds.
--
-- For os.time(), the resolution is 1 second = 1,000,000,000 ns.
-- All WASI clock IDs map to the same 1-second resolution here since
-- Lua's standard library doesn't expose higher-resolution clocks.
--
-- @param _id integer  WASI clock ID (0=realtime, 1=monotonic, 2=process, 3=thread)
-- @return integer  Resolution in nanoseconds.
function SystemClock:resolution_ns(_id)
    return 1000000000  -- 1 second (os.time resolution)
end


-- ── SystemRandom ─────────────────────────────────────────────────────────────
--
-- The default random implementation using Lua's built-in math.random.
--
-- WARNING: math.random is NOT cryptographically secure. It is a simple PRNG
-- seeded at startup. For security-sensitive applications, replace this with
-- a CSPRNG (e.g., via a LuaJIT FFI call to getrandom(2) on Linux).
--
-- For testing, swap in a FakeRandom that returns deterministic bytes.

local SystemRandom = {}
SystemRandom.__index = SystemRandom
M.SystemRandom = SystemRandom

--- Create a new SystemRandom.
-- @return SystemRandom
function SystemRandom.new()
    return setmetatable({}, SystemRandom)
end

--- Generate n pseudo-random bytes.
--
-- Returns a table of n integers, each in range [0, 255].
-- The bytes are generated using math.random(0, 255).
--
-- Because math.random is a PRNG (not a CSPRNG), this is suitable for
-- simulations and tests but NOT for cryptographic key generation.
--
-- @param n integer  Number of bytes to generate.
-- @return table  Array of n integers in [0, 255].
function SystemRandom:fill_bytes(n)
    local bytes = {}
    for i = 1, n do
        bytes[i] = math.random(0, 255)
    end
    return bytes
end


-- ============================================================================
-- WASI STUB
-- ============================================================================
--
-- The WasiStub is a host environment that provides WASI (WebAssembly System
-- Interface) host functions to WASM modules. It implements the
-- "wasi_snapshot_preview1" module, which is the standard WASI preview 1 API.
--
-- ── What is WASI? ────────────────────────────────────────────────────────────
--
-- WASI is a portable syscall interface for WASM. When a Rust or C program is
-- compiled to WASM with WASI support, the compiler generates imports like:
--
--   (import "wasi_snapshot_preview1" "fd_write" (func ...))
--   (import "wasi_snapshot_preview1" "proc_exit" (func ...))
--   (import "wasi_snapshot_preview1" "clock_time_get" (func ...))
--
-- The runtime must resolve these imports before the module can be instantiated.
-- WasiStub provides these resolutions.
--
-- ── Tiers ────────────────────────────────────────────────────────────────────
--
-- We implement WASI in tiers of increasing completeness:
--
--   Tier 1 (Tier 1):  fd_write + proc_exit (Hello World programs)
--   Tier 2 (Tier 2):  args + environ (command-line argument programs)  ← TODO
--   Tier 3 (this):    clock + random + sched_yield (time-aware programs)
--
-- ── Host function format ─────────────────────────────────────────────────────
--
-- The execution engine expects host functions as tables with a `call` field:
--
--   { call = function(args) ... return {Values.i32(0)} end }
--
-- The `args` parameter is an array of WasmValues (from wasm_execution).
-- Each WasmValue is {type = I32|I64|..., value = number}.
--
-- ── WASI errno values ────────────────────────────────────────────────────────
--
-- WASI uses errno codes for error returns. Key values:
--   0  = ESUCCESS  (no error)
--   28 = EINVAL    (invalid argument)
--   52 = ENOSYS    (function not implemented)

-- WASI errno constants.
local ESUCCESS = 0   -- No error.
local EBADF    = 8   -- Bad file descriptor.
local EINVAL   = 28  -- Invalid argument.
local ENOSYS   = 52  -- Function not implemented.

local WasiStub = {}
WasiStub.__index = WasiStub
M.WasiStub = WasiStub

--- Create a new WasiStub.
--
-- Configuration table fields:
--   args    table of strings  Command-line arguments (including argv[0]).
--   env     table            Environment variables as {"KEY=VALUE", ...} strings.
--   stdout  function(text)   Callback for stdout output. Defaults to no-op.
--   stderr  function(text)   Callback for stderr output. Defaults to no-op.
--   clock   WasiClock        Clock object implementing the WasiClock interface.
--                            Defaults to SystemClock.new().
--   random  WasiRandom       Random object implementing the WasiRandom interface.
--                            Defaults to SystemRandom.new().
--
-- @param config table|nil  Optional configuration (all fields optional).
-- @return WasiStub
function WasiStub.new(config)
    config = config or {}
    local self = setmetatable({}, WasiStub)

    -- Command-line arguments: a list of strings.
    -- argv[0] is conventionally the program name.
    self.args = config.args or {}

    -- Environment variables: a list of "KEY=VALUE" strings.
    self.env = config.env or {}

    -- Input callback for fd_read. Defaults to EOF.
    self.stdin_cb = config.stdin or function(_n) return "" end

    -- Output callbacks for fd_write.
    self.stdout_cb = config.stdout or function(_t) end
    self.stderr_cb = config.stderr or function(_t) end

    -- Clock and random: use injected implementations or defaults.
    self.clock  = config.clock  or SystemClock.new()
    self.random = config.random or SystemRandom.new()

    -- Linear memory reference (set by the runtime after instantiation).
    self.memory = nil

    return self
end


--- Called by WasmRuntime after instantiation to give us the linear memory.
--
-- We need memory access to implement fd_write (reading iov buffers),
-- args_get/environ_get (writing strings and pointers), and
-- clock/random (writing results).
--
-- @param memory LinearMemory  The instance's linear memory.
function WasiStub:set_memory(memory)
    self.memory = memory
end


--- Resolve a WASI import function by module and name.
--
-- Called by WasmRuntime:instantiate() for each import whose module is
-- "wasi_snapshot_preview1". Returns a host function table { call = fn }
-- or nil if the function is unknown.
--
-- @param module_name string  The import module name.
-- @param name string         The import function name.
-- @return table|nil  Host function object or nil.
function WasiStub:resolve_function(module_name, name)
    if module_name ~= "wasi_snapshot_preview1" then
        return nil
    end

    -- Dispatch to specific implementations.
    if name == "fd_write"            then return self:_make_fd_write()
    elseif name == "fd_read"         then return self:_make_fd_read()
    elseif name == "proc_exit"       then return self:_make_proc_exit()
    elseif name == "args_sizes_get"  then return self:_make_args_sizes_get()
    elseif name == "args_get"        then return self:_make_args_get()
    elseif name == "environ_sizes_get" then return self:_make_environ_sizes_get()
    elseif name == "environ_get"     then return self:_make_environ_get()
    elseif name == "clock_res_get"   then return self:_make_clock_res_get()
    elseif name == "clock_time_get"  then return self:_make_clock_time_get()
    elseif name == "random_get"      then return self:_make_random_get()
    elseif name == "sched_yield"     then return self:_make_sched_yield()
    else
        -- Unknown WASI function: return a stub that says ENOSYS.
        -- This allows modules that import uncommon WASI functions to still
        -- load; they'll get ENOSYS (function not implemented) at call time.
        return self:_make_enosys_stub(name)
    end
end

--- Resolve a WASI memory import (none provided by this stub).
function WasiStub:resolve_memory(_module_name, _name)
    return nil
end

--- Resolve a WASI table import (none provided by this stub).
function WasiStub:resolve_table(_module_name, _name)
    return nil
end

--- Resolve a WASI global import (none provided by this stub).
function WasiStub:resolve_global(_module_name, _name)
    return nil
end


-- ── fd_write ──────────────────────────────────────────────────────────────────
--
-- fd_write(fd: i32, iovs_ptr: i32, iovs_len: i32, nwritten_ptr: i32) → i32
--
-- The POSIX writev() syscall. Writes from a scatter-gather array of I/O
-- vectors to a file descriptor.
--
-- An iov (I/O vector) is a pair of (pointer, length) stored in memory:
--   iov.buf     = memory[iovs_ptr + i*8 ..+ 4]  (pointer to data)
--   iov.buf_len = memory[iovs_ptr + i*8 + 4 ..+ 4] (number of bytes)
--
-- We support fd=1 (stdout) and fd=2 (stderr); others are silently ignored.
-- The total bytes written is stored at nwritten_ptr.
--
-- Diagram:
--   Memory layout of iovs:
--   ┌──────────────────────┬──────────────────────┬──── ...
--   │ buf_ptr (4 bytes LE) │ buf_len (4 bytes LE) │ (next iov)
--   └──────────────────────┴──────────────────────┴──── ...
--   iovs_ptr^

function WasiStub:_make_fd_write()
    local self_ref = self
    return {
        call = function(args)
            local fd         = args[1].value
            local iovs_ptr   = args[2].value
            local iovs_len   = args[3].value
            local nwritten_ptr = args[4].value

            local memory = self_ref.memory
            if not memory then
                return { wasm_execution.i32(ENOSYS) }
            end

            local total_written = 0

            -- Process each I/O vector in the scatter-gather array.
            -- Each iov occupies 8 bytes: 4 for buf_ptr, 4 for buf_len.
            for i = 0, iovs_len - 1 do
                local buf_ptr = memory:load_i32(iovs_ptr + i * 8)
                -- Treat as unsigned: mask off sign bit artifacts.
                buf_ptr = buf_ptr & 0xFFFFFFFF
                local buf_len = memory:load_i32(iovs_ptr + i * 8 + 4)
                buf_len = buf_len & 0xFFFFFFFF

                -- Read buf_len bytes from memory and assemble a Lua string.
                local chars = {}
                for j = 0, buf_len - 1 do
                    chars[#chars + 1] = string.char(memory:load_i32_8u(buf_ptr + j))
                end
                local text = table.concat(chars)
                total_written = total_written + buf_len

                -- Route output to the appropriate callback.
                if fd == 1 then
                    self_ref.stdout_cb(text)
                elseif fd == 2 then
                    self_ref.stderr_cb(text)
                end
                -- Other fds are silently dropped.
            end

            -- Write total bytes written back to the WASM program.
            memory:store_i32(nwritten_ptr, total_written)

            return { wasm_execution.i32(ESUCCESS) }
        end
    }
end


-- ── fd_read ───────────────────────────────────────────────────────────────────
--
-- fd_read(fd: i32, iovs_ptr: i32, iovs_len: i32, nread_ptr: i32) → i32
--
-- Reads bytes from stdin into guest buffers. Only fd=0 (stdin) is supported.

function WasiStub:_make_fd_read()
    local self_ref = self
    return {
        call = function(args)
            local fd         = args[1].value
            local iovs_ptr   = args[2].value
            local iovs_len   = args[3].value
            local nread_ptr  = args[4].value

            local memory = self_ref.memory
            if not memory then
                return { wasm_execution.i32(ENOSYS) }
            end
            if fd ~= 0 then
                return { wasm_execution.i32(EBADF) }
            end

            local total_read = 0

            for i = 0, iovs_len - 1 do
                local buf_ptr = memory:load_i32(iovs_ptr + i * 8) & 0xFFFFFFFF
                local buf_len = memory:load_i32(iovs_ptr + i * 8 + 4) & 0xFFFFFFFF

                local chunk = self_ref.stdin_cb(buf_len)
                if type(chunk) == "table" then
                    local chars = {}
                    for _, byte in ipairs(chunk) do
                        chars[#chars + 1] = string.char(byte)
                    end
                    chunk = table.concat(chars)
                elseif chunk == nil then
                    chunk = ""
                end

                chunk = string.sub(chunk, 1, buf_len)
                for j = 1, #chunk do
                    memory:store_i32_8(buf_ptr + (j - 1), string.byte(chunk, j))
                end

                total_read = total_read + #chunk
                if #chunk < buf_len then
                    break
                end
            end

            memory:store_i32(nread_ptr, total_read)
            return { wasm_execution.i32(ESUCCESS) }
        end
    }
end


-- ── proc_exit ─────────────────────────────────────────────────────────────────
--
-- proc_exit(code: i32) → (never returns)
--
-- The WASM program requests graceful termination with an exit code.
-- We signal this by throwing a Lua error with a special prefix so the
-- runtime can catch and handle it.

function WasiStub:_make_proc_exit()
    return {
        call = function(args)
            local code = args[1].value
            error("ProcExit:" .. tostring(code))
        end
    }
end


-- ── args_sizes_get ───────────────────────────────────────────────────────────
--
-- args_sizes_get(argc_ptr: i32, argv_buf_size_ptr: i32) → i32 (errno)
--
-- Query the number of command-line arguments and the total buffer size
-- needed to hold all argument strings (each null-terminated).
--
-- This is a two-step WASI pattern: first call args_sizes_get to find out
-- how much memory to allocate, then call args_get to actually fill it.
--
-- Example with args = {"myapp", "hello"}:
--   argc          = 2
--   argv_buf_size = len("myapp") + 1 + len("hello") + 1 = 6 + 6 = 12
--
-- The WASM program reads both output pointers and uses them for malloc.

function WasiStub:_make_args_sizes_get()
    local self_ref = self
    return {
        call = function(args)
            local argc_ptr         = args[1].value
            local argv_buf_size_ptr = args[2].value

            local memory = self_ref.memory
            if not memory then return { wasm_execution.i32(ENOSYS) } end

            -- Count arguments and total buffer space.
            local argc = #self_ref.args
            local buf_size = 0
            for _, arg in ipairs(self_ref.args) do
                buf_size = buf_size + #arg + 1  -- +1 for null terminator '\0'
            end

            memory:store_i32(argc_ptr, argc)
            memory:store_i32(argv_buf_size_ptr, buf_size)

            return { wasm_execution.i32(ESUCCESS) }
        end
    }
end


-- ── args_get ─────────────────────────────────────────────────────────────────
--
-- args_get(argv_ptr: i32, argv_buf_ptr: i32) → i32 (errno)
--
-- Populate the argument pointer array and argument string buffer.
--
-- Memory layout after args_get with args = {"myapp", "hello"}:
--
--   argv_ptr:       points to an array of i32 pointers
--   argv_buf_ptr:   the raw string buffer
--
--   At argv_ptr + 0*4: argv_buf_ptr + 0   (points to "myapp\0")
--   At argv_ptr + 1*4: argv_buf_ptr + 6   (points to "hello\0")
--
--   At argv_buf_ptr + 0: 'm','y','a','p','p','\0'
--   At argv_buf_ptr + 6: 'h','e','l','l','o','\0'
--
-- The WASM program treats argv_ptr as char** (C-style argv array).

function WasiStub:_make_args_get()
    local self_ref = self
    return {
        call = function(args)
            local argv_ptr     = args[1].value
            local argv_buf_ptr = args[2].value

            local memory = self_ref.memory
            if not memory then return { wasm_execution.i32(ENOSYS) } end

            -- `offset` tracks our current write position in the string buffer.
            local offset = argv_buf_ptr
            for i, arg in ipairs(self_ref.args) do
                -- Write the pointer to this arg string into the argv array.
                -- argv[i-1] is at argv_ptr + (i-1)*4 (each pointer is 4 bytes).
                memory:store_i32(argv_ptr + (i - 1) * 4, offset)

                -- Write the argument string bytes, then a null terminator.
                for j = 1, #arg do
                    memory:store_i32_8(offset + j - 1, string.byte(arg, j))
                end
                memory:store_i32_8(offset + #arg, 0)  -- null terminator

                -- Advance offset past this string (including the null byte).
                offset = offset + #arg + 1
            end

            return { wasm_execution.i32(ESUCCESS) }
        end
    }
end


-- ── environ_sizes_get ────────────────────────────────────────────────────────
--
-- environ_sizes_get(count_ptr: i32, buf_size_ptr: i32) → i32 (errno)
--
-- Same two-step pattern as args_sizes_get but for environment variables.
-- Environment variables are stored as "KEY=VALUE" strings.
--
-- Example with env = {"HOME=/home/user"}:
--   count    = 1
--   buf_size = len("HOME=/home/user") + 1 = 16

function WasiStub:_make_environ_sizes_get()
    local self_ref = self
    return {
        call = function(args)
            local count_ptr    = args[1].value
            local buf_size_ptr = args[2].value

            local memory = self_ref.memory
            if not memory then return { wasm_execution.i32(ENOSYS) } end

            local count = #self_ref.env
            local buf_size = 0
            for _, e in ipairs(self_ref.env) do
                buf_size = buf_size + #e + 1  -- +1 for null terminator
            end

            memory:store_i32(count_ptr, count)
            memory:store_i32(buf_size_ptr, buf_size)

            return { wasm_execution.i32(ESUCCESS) }
        end
    }
end


-- ── environ_get ──────────────────────────────────────────────────────────────
--
-- environ_get(environ_ptr: i32, environ_buf_ptr: i32) → i32 (errno)
--
-- Populate the environ pointer array and environ string buffer.
-- Exactly mirrors args_get but reads from self.env instead of self.args.
--
-- Memory layout after environ_get with env = {"HOME=/home/user"}:
--
--   At environ_ptr + 0*4: environ_buf_ptr + 0   (→ "HOME=/home/user\0")
--   At environ_buf_ptr: 'H','O','M','E','=','/','h','o','m','e','/','u','s','e','r','\0'

function WasiStub:_make_environ_get()
    local self_ref = self
    return {
        call = function(args)
            local environ_ptr     = args[1].value
            local environ_buf_ptr = args[2].value

            local memory = self_ref.memory
            if not memory then return { wasm_execution.i32(ENOSYS) } end

            local offset = environ_buf_ptr
            for i, e in ipairs(self_ref.env) do
                -- Write pointer to this "KEY=VALUE" string into the environ array.
                memory:store_i32(environ_ptr + (i - 1) * 4, offset)

                -- Write the "KEY=VALUE" bytes followed by null terminator.
                for j = 1, #e do
                    memory:store_i32_8(offset + j - 1, string.byte(e, j))
                end
                memory:store_i32_8(offset + #e, 0)

                offset = offset + #e + 1
            end

            return { wasm_execution.i32(ESUCCESS) }
        end
    }
end


-- ── clock_res_get ────────────────────────────────────────────────────────────
--
-- clock_res_get(id: i32, resolution_ptr: i32) → i32 (errno)
--
-- Query the resolution of a WASI clock. The result is written as a
-- 64-bit unsigned integer (i64) in little-endian byte order to
-- resolution_ptr in linear memory.
--
-- WASI clock IDs:
--   0 = CLOCK_REALTIME    (wall clock)
--   1 = CLOCK_MONOTONIC   (monotonic)
--   2 = CLOCK_PROCESS_CPUTIME_ID
--   3 = CLOCK_THREAD_CPUTIME_ID
--
-- Writing i64 as two i32s in little-endian:
--   Low 32 bits  → memory[resolution_ptr + 0 .. +3]
--   High 32 bits → memory[resolution_ptr + 4 .. +7]
--
-- Lua 5.3+ supports 64-bit integer arithmetic natively, so we can use
-- bitwise shifts and masks without loss of precision.

function WasiStub:_make_clock_res_get()
    local self_ref = self
    return {
        call = function(args)
            local id             = args[1].value
            local resolution_ptr = args[2].value

            local memory = self_ref.memory
            if not memory then return { wasm_execution.i32(ENOSYS) } end

            local ns = self_ref.clock:resolution_ns(id)

            -- Store as 64-bit little-endian: low word first, then high word.
            memory:store_i64(resolution_ptr, ns)

            return { wasm_execution.i32(ESUCCESS) }
        end
    }
end


-- ── clock_time_get ───────────────────────────────────────────────────────────
--
-- clock_time_get(id: i32, precision: i64, time_ptr: i32) → i32 (errno)
--
-- Read the current time of the given clock. The result is a 64-bit unsigned
-- integer in nanoseconds, written to time_ptr in linear memory (little-endian).
--
-- The `precision` argument is a "desired precision" hint in nanoseconds.
-- Most WASI implementations ignore it (we do too).
--
-- Clock ID dispatch:
--   0 (REALTIME)   → clock:realtime_ns()
--   1 (MONOTONIC)  → clock:monotonic_ns()
--   2 (PROCESS)    → clock:realtime_ns() (best approximation)
--   3 (THREAD)     → clock:realtime_ns() (best approximation)
--   other          → EINVAL (invalid argument)
--
-- Note: precision is an i64 param. In the wasm_execution engine, i64 values
-- have value type 0x7E. The args array still contains a WasmValue table.

function WasiStub:_make_clock_time_get()
    local self_ref = self
    return {
        call = function(args)
            local id        = args[1].value
            -- args[2] is precision (i64) — we intentionally ignore it.
            local time_ptr  = args[3].value

            local memory = self_ref.memory
            if not memory then return { wasm_execution.i32(ENOSYS) } end

            local ns
            if id == 0 or id == 2 or id == 3 then
                -- Realtime, process, and thread clocks all map to wall time.
                ns = self_ref.clock:realtime_ns()
            elseif id == 1 then
                -- Monotonic clock.
                ns = self_ref.clock:monotonic_ns()
            else
                -- Unknown clock ID → EINVAL.
                return { wasm_execution.i32(EINVAL) }
            end

            -- Write the 64-bit nanosecond timestamp to memory.
            memory:store_i64(time_ptr, ns)

            return { wasm_execution.i32(ESUCCESS) }
        end
    }
end


-- ── random_get ───────────────────────────────────────────────────────────────
--
-- random_get(buf_ptr: i32, buf_len: i32) → i32 (errno)
--
-- Fill a region of linear memory with random bytes.
--
-- The WASM program provides a pointer and length. We ask self.random for
-- buf_len bytes and write them one by one to memory.
--
-- Example with buf_len=4 and a PRNG that returns [0xAB, 0xAB, 0xAB, 0xAB]:
--   memory[buf_ptr + 0] = 0xAB
--   memory[buf_ptr + 1] = 0xAB
--   memory[buf_ptr + 2] = 0xAB
--   memory[buf_ptr + 3] = 0xAB

function WasiStub:_make_random_get()
    local self_ref = self
    return {
        call = function(args)
            local buf_ptr = args[1].value
            local buf_len = args[2].value

            local memory = self_ref.memory
            if not memory then return { wasm_execution.i32(ENOSYS) } end

            -- Ask the random provider for buf_len bytes.
            local bytes = self_ref.random:fill_bytes(buf_len)

            -- Write each byte into linear memory.
            for i, b in ipairs(bytes) do
                memory:store_i32_8(buf_ptr + i - 1, b)
            end

            return { wasm_execution.i32(ESUCCESS) }
        end
    }
end


-- ── sched_yield ──────────────────────────────────────────────────────────────
--
-- sched_yield() → i32 (errno)
--
-- Voluntarily yield the CPU to other threads or processes.
--
-- In a single-threaded Lua interpreter there is nothing to yield to.
-- We return ESUCCESS immediately, which is the correct behavior per the
-- WASI spec (yielding is an optimization hint, not a blocking call).

function WasiStub:_make_sched_yield()
    return {
        call = function(_args)
            return { wasm_execution.i32(ESUCCESS) }
        end
    }
end


-- ── ENOSYS stub ───────────────────────────────────────────────────────────────
--
-- Fallback for any WASI function we haven't implemented yet.
-- Returns ENOSYS so the calling program can handle the missing feature
-- gracefully instead of crashing on a missing import.

function WasiStub:_make_enosys_stub(_name)
    return {
        call = function(_args)
            return { wasm_execution.i32(ENOSYS) }
        end
    }
end

M.WasiHost = WasiStub


return M
