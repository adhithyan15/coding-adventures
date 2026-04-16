-- test_wasi_tier3.lua — WASI Tier 3 host function tests
--
-- Tests for the 8 new WASI Tier 3 host functions added to WasiStub:
--   args_sizes_get, args_get, environ_sizes_get, environ_get,
--   clock_res_get, clock_time_get, random_get, sched_yield
--
-- ============================================================================
-- DESIGN NOTES
-- ============================================================================
--
-- We use fake/stub implementations of the WasiClock and WasiRandom interfaces
-- to make tests deterministic. The real SystemClock depends on os.time() which
-- changes every second, making assertions on exact values impossible.
--
-- A FakeClock returns hardcoded values:
--   realtime_ns()     → 1700000000000000001
--   monotonic_ns()    → 42000000000
--   resolution_ns(id) → 1000000
--
-- A FakeRandom always returns 0xAB bytes.
--
-- To verify that i64 values are written correctly to memory, we read them back
-- using LinearMemory:load_i64() from wasm_execution.
--
-- ============================================================================
-- PATH SETUP
-- ============================================================================

package.path = "../src/?.lua;" .. "../src/?/init.lua;" .. package.path
package.path = "../../wasm_execution/src/?.lua;" .. "../../wasm_execution/src/?/init.lua;" .. package.path
package.path = "../../wasm_validator/src/?.lua;" .. "../../wasm_validator/src/?/init.lua;" .. package.path
package.path = "../../wasm_module_parser/src/?.lua;" .. "../../wasm_module_parser/src/?/init.lua;" .. package.path
package.path = "../../wasm_leb128/src/?.lua;" .. "../../wasm_leb128/src/?/init.lua;" .. package.path
package.path = "../../wasm_types/src/?.lua;" .. "../../wasm_types/src/?/init.lua;" .. package.path
package.path = "../../wasm_opcodes/src/?.lua;" .. "../../wasm_opcodes/src/?/init.lua;" .. package.path
package.path = "../../virtual_machine/src/?.lua;" .. "../../virtual_machine/src/?/init.lua;" .. package.path

local wasm_runtime = require("coding_adventures.wasm_runtime")
local wasm_execution = require("coding_adventures.wasm_execution")


-- ============================================================================
-- FAKE CLOCK — deterministic clock for testing
-- ============================================================================
--
-- Implements the WasiClock interface with hardcoded values so tests don't
-- depend on the wall clock. Each test that checks time values uses FakeClock.
--
-- Interface contract:
--   clock:realtime_ns()     → integer (nanoseconds since Unix epoch)
--   clock:monotonic_ns()    → integer (nanoseconds, monotonically increasing)
--   clock:resolution_ns(id) → integer (nanoseconds)

local FakeClock = {}
FakeClock.__index = FakeClock

function FakeClock.new()
    return setmetatable({}, FakeClock)
end

-- A plausible 2023 timestamp: 2023-11-14T22:13:20.000000001 UTC
-- This fits comfortably in a 64-bit signed integer (max ~9.2 × 10^18).
function FakeClock:realtime_ns()
    return 1700000000000000001
end

-- 42 seconds of monotonic time (42 billion nanoseconds).
function FakeClock:monotonic_ns()
    return 42000000000
end

-- 1 millisecond resolution for all clock IDs.
function FakeClock:resolution_ns(_id)
    return 1000000
end


-- ============================================================================
-- FAKE RANDOM — deterministic PRNG for testing
-- ============================================================================
--
-- Implements the WasiRandom interface with a constant byte value (0xAB = 171).
-- This makes random_get tests fully deterministic.
--
-- Interface contract:
--   random:fill_bytes(n) → table of n integers in [0, 255]

local FakeRandom = {}
FakeRandom.__index = FakeRandom

function FakeRandom.new()
    return setmetatable({}, FakeRandom)
end

function FakeRandom:fill_bytes(n)
    local bytes = {}
    for i = 1, n do
        bytes[i] = 0xAB  -- 171 decimal — distinctive, easy to spot in memory
    end
    return bytes
end


-- ============================================================================
-- TEST HELPERS
-- ============================================================================

--- Create a WasiStub with FakeClock, FakeRandom, and the given config.
-- @param extra table|nil  Extra config fields (args, env, stdout, stderr).
-- @return WasiStub, LinearMemory  The stub and a 1-page (64 KiB) memory.
local function make_stub_and_memory(extra)
    extra = extra or {}
    local config = {
        args   = extra.args   or {},
        env    = extra.env    or {},
        stdin  = extra.stdin  or function(_n) return "" end,
        stdout = extra.stdout or function(_t) end,
        stderr = extra.stderr or function(_t) end,
        clock  = FakeClock.new(),
        random = FakeRandom.new(),
    }
    local stub = wasm_runtime.WasiStub.new(config)
    -- Allocate 1 page (64 KiB) of linear memory and inject it.
    local memory = wasm_execution.LinearMemory.new(1)
    stub:set_memory(memory)
    return stub, memory
end

--- Call a WASI host function by name with the given argument values.
--
-- Wraps the i32/i64 conversion so tests can pass plain Lua numbers.
-- All args are treated as i32 unless overridden by `types`.
--
-- @param stub WasiStub   The stub to query.
-- @param name string     WASI function name (e.g. "args_sizes_get").
-- @param arg_vals table  Array of raw Lua numbers.
-- @param types table|nil  Array of "i32" or "i64" per argument (default: all i32).
-- @return table  Array of WasmValue results.
local function call_wasi(stub, name, arg_vals, types)
    types = types or {}
    local host_fn = stub:resolve_function("wasi_snapshot_preview1", name)
    assert(host_fn, "WASI function not found: " .. name)

    local args = {}
    for i, v in ipairs(arg_vals) do
        local t = types[i] or "i32"
        if t == "i64" then
            args[i] = wasm_execution.i64(v)
        else
            args[i] = wasm_execution.i32(v)
        end
    end

    return host_fn.call(args)
end


-- ============================================================================
-- WASM BINARY ASSEMBLY HELPERS (for the existing square test)
-- ============================================================================

local function leb128(n)
    local result = {}
    while true do
        local byte = n & 0x7F
        n = n >> 7
        if n > 0 then byte = byte | 0x80 end
        result[#result + 1] = byte
        if n == 0 then break end
    end
    return result
end

local function build_section(id, payload)
    local size = leb128(#payload)
    local result = { id }
    for _, b in ipairs(size) do result[#result + 1] = b end
    for _, b in ipairs(payload) do result[#result + 1] = b end
    return result
end

local function concat_bytes(...)
    local result = {}
    for _, arr in ipairs({...}) do
        for _, b in ipairs(arr) do result[#result + 1] = b end
    end
    return result
end

local function bytes_to_string(bytes)
    local chars = {}
    for _, b in ipairs(bytes) do chars[#chars + 1] = string.char(b) end
    return table.concat(chars)
end

local function build_square_wasm()
    local header = { 0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00 }
    local type_payload = concat_bytes(leb128(1), { 0x60, 0x01, 0x7F, 0x01, 0x7F })
    local type_section = build_section(1, type_payload)
    local func_payload = concat_bytes(leb128(1), leb128(0))
    local func_section = build_section(3, func_payload)
    local export_name  = { 0x73, 0x71, 0x75, 0x61, 0x72, 0x65 }
    local export_payload = concat_bytes(leb128(1), leb128(#export_name), export_name, { 0x00 }, leb128(0))
    local export_section = build_section(7, export_payload)
    local body_code = { 0x20, 0x00, 0x20, 0x00, 0x6C, 0x0B }
    local body = concat_bytes(leb128(0), body_code)
    local code_payload = concat_bytes(leb128(1), leb128(#body), body)
    local code_section = build_section(10, code_payload)
    return bytes_to_string(concat_bytes(header, type_section, func_section, export_section, code_section))
end


-- ============================================================================
-- TESTS
-- ============================================================================

describe("WasiStub Tier 3", function()

    -- ── Module structure ────────────────────────────────────────────────────

    describe("exports", function()
        it("exports WasiStub", function()
            assert.is_not_nil(wasm_runtime.WasiStub)
        end)

        it("exports WasiHost as an alias", function()
            assert.are.equal(wasm_runtime.WasiStub, wasm_runtime.WasiHost)
        end)

        it("exports SystemClock", function()
            assert.is_not_nil(wasm_runtime.SystemClock)
        end)

        it("exports SystemRandom", function()
            assert.is_not_nil(wasm_runtime.SystemRandom)
        end)
    end)


    -- ── WasiStub constructor ────────────────────────────────────────────────

    describe("WasiStub.new", function()
        it("creates a stub with default empty args and env", function()
            local stub = wasm_runtime.WasiStub.new()
            assert.is_not_nil(stub)
            assert.equals(0, #stub.args)
            assert.equals(0, #stub.env)
        end)

        it("stores injected args", function()
            local stub = wasm_runtime.WasiStub.new({ args = {"myapp", "hello"} })
            assert.equals("myapp", stub.args[1])
            assert.equals("hello", stub.args[2])
        end)

        it("stores injected env", function()
            local stub = wasm_runtime.WasiStub.new({ env = {"HOME=/home/user"} })
            assert.equals("HOME=/home/user", stub.env[1])
        end)

        it("defaults to SystemClock when no clock given", function()
            local stub = wasm_runtime.WasiStub.new()
            -- Should have a clock with the SystemClock methods.
            assert.is_not_nil(stub.clock)
            assert.is_function(stub.clock.realtime_ns)
        end)

        it("defaults to SystemRandom when no random given", function()
            local stub = wasm_runtime.WasiStub.new()
            assert.is_not_nil(stub.random)
            assert.is_function(stub.random.fill_bytes)
        end)

        it("accepts injected FakeClock", function()
            local fc = FakeClock.new()
            local stub = wasm_runtime.WasiStub.new({ clock = fc })
            assert.equals(fc, stub.clock)
        end)

        it("accepts injected FakeRandom", function()
            local fr = FakeRandom.new()
            local stub = wasm_runtime.WasiStub.new({ random = fr })
            assert.equals(fr, stub.random)
        end)
    end)


    -- ── resolve_function ────────────────────────────────────────────────────

    describe("resolve_function", function()
        it("returns nil for non-WASI modules", function()
            local stub, _ = make_stub_and_memory()
            local fn = stub:resolve_function("env", "fd_write")
            assert.is_nil(fn)
        end)

        it("returns a host function for known WASI names", function()
            local stub, _ = make_stub_and_memory()
            local fn = stub:resolve_function("wasi_snapshot_preview1", "args_sizes_get")
            assert.is_not_nil(fn)
            assert.is_function(fn.call)
        end)

        it("resolves fd_read", function()
            local stub, _ = make_stub_and_memory()
            local fn = stub:resolve_function("wasi_snapshot_preview1", "fd_read")
            assert.is_not_nil(fn)
            assert.is_function(fn.call)
        end)

        it("returns an ENOSYS stub for unknown WASI names", function()
            local stub, _ = make_stub_and_memory()
            local fn = stub:resolve_function("wasi_snapshot_preview1", "unknown_func_xyz")
            assert.is_not_nil(fn)
            local results = fn.call({})
            assert.equals(52, results[1].value)  -- ENOSYS = 52
        end)
    end)

    describe("fd_read", function()
        it("copies stdin bytes into guest memory", function()
            local stub, memory = make_stub_and_memory({
                stdin = function() return "hi" end,
            })

            memory:store_i32(0, 200)
            memory:store_i32(4, 2)

            local results = call_wasi(stub, "fd_read", { 0, 0, 1, 100 })
            assert.equals(0, results[1].value)
            assert.equals(2, memory:load_i32(100))
            assert.equals(string.byte("h"), memory:load_i32_8u(200))
            assert.equals(string.byte("i"), memory:load_i32_8u(201))
        end)
    end)


    -- ── args_sizes_get ──────────────────────────────────────────────────────
    --
    -- Tests the two-step WASI pattern: query sizes, then fill buffers.
    -- With args = {"myapp", "hello"}:
    --   argc          = 2
    --   argv_buf_size = 6 ("myapp\0") + 6 ("hello\0") = 12

    describe("args_sizes_get", function()
        it("returns argc=2, buf_size=12 for {myapp, hello}", function()
            local stub, memory = make_stub_and_memory({ args = {"myapp", "hello"} })

            -- Store outputs at memory addresses 100 and 104.
            local argc_ptr         = 100
            local argv_buf_size_ptr = 104

            local results = call_wasi(stub, "args_sizes_get",
                { argc_ptr, argv_buf_size_ptr })

            -- errno = 0 (ESUCCESS)
            assert.equals(0, results[1].value)

            -- argc should be 2.
            assert.equals(2, memory:load_i32(argc_ptr))

            -- buf_size = 6 + 6 = 12 (each string + null terminator).
            assert.equals(12, memory:load_i32(argv_buf_size_ptr))
        end)

        it("returns argc=0, buf_size=0 for empty args", function()
            local stub, memory = make_stub_and_memory({ args = {} })
            call_wasi(stub, "args_sizes_get", { 0, 4 })
            assert.equals(0, memory:load_i32(0))
            assert.equals(0, memory:load_i32(4))
        end)

        it("returns errno=0 on success", function()
            local stub, _ = make_stub_and_memory()
            local results = call_wasi(stub, "args_sizes_get", { 0, 4 })
            assert.equals(0, results[1].value)
        end)
    end)


    -- ── args_get ────────────────────────────────────────────────────────────
    --
    -- Tests that args_get writes:
    --   - An argv pointer array at argv_ptr
    --   - Null-terminated arg strings at argv_buf_ptr
    --
    -- Memory layout for args = {"myapp", "hello"}, argv_ptr=200, argv_buf_ptr=300:
    --   memory[200] = 300        (pointer to "myapp\0")
    --   memory[204] = 306        (pointer to "hello\0")
    --   memory[300..305] = "myapp\0"
    --   memory[306..311] = "hello\0"

    describe("args_get", function()
        it("writes pointer array and strings to memory", function()
            local stub, memory = make_stub_and_memory({ args = {"myapp", "hello"} })

            local argv_ptr     = 200
            local argv_buf_ptr = 300

            local results = call_wasi(stub, "args_get", { argv_ptr, argv_buf_ptr })
            assert.equals(0, results[1].value)

            -- argv[0] should point to argv_buf_ptr + 0 = 300.
            local ptr0 = memory:load_i32(argv_ptr)
            assert.equals(argv_buf_ptr, ptr0)

            -- argv[1] should point to argv_buf_ptr + 6 = 306.
            local ptr1 = memory:load_i32(argv_ptr + 4)
            assert.equals(argv_buf_ptr + 6, ptr1)

            -- "myapp" in memory at ptr0.
            assert.equals(string.byte("m"), memory:load_i32_8u(ptr0 + 0))
            assert.equals(string.byte("y"), memory:load_i32_8u(ptr0 + 1))
            assert.equals(string.byte("a"), memory:load_i32_8u(ptr0 + 2))
            assert.equals(string.byte("p"), memory:load_i32_8u(ptr0 + 3))
            assert.equals(string.byte("p"), memory:load_i32_8u(ptr0 + 4))
            assert.equals(0,                memory:load_i32_8u(ptr0 + 5))  -- null

            -- "hello" in memory at ptr1.
            assert.equals(string.byte("h"), memory:load_i32_8u(ptr1 + 0))
            assert.equals(string.byte("e"), memory:load_i32_8u(ptr1 + 1))
            assert.equals(string.byte("l"), memory:load_i32_8u(ptr1 + 2))
            assert.equals(string.byte("l"), memory:load_i32_8u(ptr1 + 3))
            assert.equals(string.byte("o"), memory:load_i32_8u(ptr1 + 4))
            assert.equals(0,                memory:load_i32_8u(ptr1 + 5))  -- null
        end)

        it("handles single argument", function()
            local stub, memory = make_stub_and_memory({ args = {"prog"} })
            call_wasi(stub, "args_get", { 0, 100 })
            -- Pointer at offset 0 should be 100.
            assert.equals(100, memory:load_i32(0))
            -- "prog" at 100.
            assert.equals(string.byte("p"), memory:load_i32_8u(100))
            assert.equals(0, memory:load_i32_8u(104))  -- null after "prog"
        end)

        it("handles empty args list gracefully", function()
            local stub, _ = make_stub_and_memory({ args = {} })
            local results = call_wasi(stub, "args_get", { 0, 100 })
            assert.equals(0, results[1].value)
        end)
    end)


    -- ── environ_sizes_get ───────────────────────────────────────────────────
    --
    -- With env = {"HOME=/home/user"}:
    --   count    = 1
    --   buf_size = len("HOME=/home/user") + 1 = 15 + 1 = 16

    describe("environ_sizes_get", function()
        it("returns count=1, buf_size=16 for {HOME=/home/user}", function()
            local stub, memory = make_stub_and_memory({ env = {"HOME=/home/user"} })

            local count_ptr    = 50
            local buf_size_ptr = 54

            local results = call_wasi(stub, "environ_sizes_get",
                { count_ptr, buf_size_ptr })
            assert.equals(0, results[1].value)

            assert.equals(1, memory:load_i32(count_ptr))
            -- "HOME=/home/user" = 15 chars + 1 null = 16
            assert.equals(16, memory:load_i32(buf_size_ptr))
        end)

        it("returns count=0, buf_size=0 for empty env", function()
            local stub, memory = make_stub_and_memory({ env = {} })
            call_wasi(stub, "environ_sizes_get", { 0, 4 })
            assert.equals(0, memory:load_i32(0))
            assert.equals(0, memory:load_i32(4))
        end)

        it("counts multiple env vars", function()
            local stub, memory = make_stub_and_memory({
                env = {"A=1", "B=2", "C=3"}
            })
            call_wasi(stub, "environ_sizes_get", { 0, 4 })
            assert.equals(3, memory:load_i32(0))
            -- "A=1\0" + "B=2\0" + "C=3\0" = 4 + 4 + 4 = 12
            assert.equals(12, memory:load_i32(4))
        end)
    end)


    -- ── environ_get ─────────────────────────────────────────────────────────
    --
    -- Tests that environ_get writes pointer array and "KEY=VALUE\0" strings.

    describe("environ_get", function()
        it("writes pointer and string for single env var", function()
            local stub, memory = make_stub_and_memory({ env = {"HOME=/home/user"} })

            local environ_ptr     = 400
            local environ_buf_ptr = 500

            local results = call_wasi(stub, "environ_get",
                { environ_ptr, environ_buf_ptr })
            assert.equals(0, results[1].value)

            -- environ[0] should point to environ_buf_ptr + 0 = 500.
            assert.equals(environ_buf_ptr, memory:load_i32(environ_ptr))

            -- "HOME=/home/user" at 500.
            assert.equals(string.byte("H"), memory:load_i32_8u(environ_buf_ptr + 0))
            assert.equals(string.byte("O"), memory:load_i32_8u(environ_buf_ptr + 1))
            assert.equals(string.byte("M"), memory:load_i32_8u(environ_buf_ptr + 2))
            assert.equals(string.byte("E"), memory:load_i32_8u(environ_buf_ptr + 3))
            assert.equals(string.byte("="), memory:load_i32_8u(environ_buf_ptr + 4))
            -- Skip ahead to the null terminator.
            assert.equals(0, memory:load_i32_8u(environ_buf_ptr + 15))
        end)

        it("writes consecutive strings for multiple env vars", function()
            local stub, memory = make_stub_and_memory({ env = {"A=1", "B=2"} })

            call_wasi(stub, "environ_get", { 0, 100 })

            -- environ[0] → 100 (pointer to "A=1\0")
            assert.equals(100, memory:load_i32(0))
            -- environ[1] → 104 (100 + len("A=1") + 1 = 104)
            assert.equals(104, memory:load_i32(4))

            -- "A=1\0" at 100
            assert.equals(string.byte("A"), memory:load_i32_8u(100))
            assert.equals(string.byte("="), memory:load_i32_8u(101))
            assert.equals(string.byte("1"), memory:load_i32_8u(102))
            assert.equals(0,                memory:load_i32_8u(103))

            -- "B=2\0" at 104
            assert.equals(string.byte("B"), memory:load_i32_8u(104))
            assert.equals(0,                memory:load_i32_8u(107))
        end)
    end)


    -- ── clock_time_get ──────────────────────────────────────────────────────
    --
    -- Tests that clock_time_get writes the correct 64-bit value to memory.
    --
    -- FakeClock:
    --   realtime_ns()  → 1700000000000000001
    --   monotonic_ns() → 42000000000
    --
    -- We read the value back with load_i64 to verify.

    describe("clock_time_get", function()
        it("returns realtime for clock_id=0", function()
            local stub, memory = make_stub_and_memory()
            local time_ptr = 1000

            -- precision is i64 (arg 2), id and time_ptr are i32.
            local results = call_wasi(stub, "clock_time_get",
                { 0, 0, time_ptr }, { "i32", "i64", "i32" })

            assert.equals(0, results[1].value)  -- ESUCCESS

            -- FakeClock returns 1700000000000000001 for realtime.
            local ts = memory:load_i64(time_ptr)
            assert.equals(1700000000000000001, ts)
        end)

        it("returns monotonic for clock_id=1", function()
            local stub, memory = make_stub_and_memory()
            local time_ptr = 1000

            local results = call_wasi(stub, "clock_time_get",
                { 1, 0, time_ptr }, { "i32", "i64", "i32" })

            assert.equals(0, results[1].value)

            -- FakeClock returns 42000000000 for monotonic.
            local ts = memory:load_i64(time_ptr)
            assert.equals(42000000000, ts)
        end)

        it("returns realtime for clock_id=2 (process clock)", function()
            local stub, memory = make_stub_and_memory()
            local time_ptr = 1000

            call_wasi(stub, "clock_time_get",
                { 2, 0, time_ptr }, { "i32", "i64", "i32" })

            local ts = memory:load_i64(time_ptr)
            assert.equals(1700000000000000001, ts)
        end)

        it("returns realtime for clock_id=3 (thread clock)", function()
            local stub, memory = make_stub_and_memory()
            local time_ptr = 1000

            call_wasi(stub, "clock_time_get",
                { 3, 0, time_ptr }, { "i32", "i64", "i32" })

            local ts = memory:load_i64(time_ptr)
            assert.equals(1700000000000000001, ts)
        end)

        it("returns EINVAL for unknown clock_id=99", function()
            local stub, _ = make_stub_and_memory()

            local results = call_wasi(stub, "clock_time_get",
                { 99, 0, 1000 }, { "i32", "i64", "i32" })

            assert.equals(28, results[1].value)  -- EINVAL = 28
        end)
    end)


    -- ── clock_res_get ───────────────────────────────────────────────────────
    --
    -- FakeClock:resolution_ns() → 1000000 (1 millisecond)
    -- Written as i64 to memory.

    describe("clock_res_get", function()
        it("writes 1000000 ns resolution to memory for clock_id=0", function()
            local stub, memory = make_stub_and_memory()
            local res_ptr = 2000

            local results = call_wasi(stub, "clock_res_get", { 0, res_ptr })
            assert.equals(0, results[1].value)

            local res = memory:load_i64(res_ptr)
            assert.equals(1000000, res)
        end)

        it("writes resolution for clock_id=1 (monotonic)", function()
            local stub, memory = make_stub_and_memory()
            local res_ptr = 2000

            call_wasi(stub, "clock_res_get", { 1, res_ptr })

            local res = memory:load_i64(res_ptr)
            assert.equals(1000000, res)
        end)
    end)


    -- ── random_get ──────────────────────────────────────────────────────────
    --
    -- FakeRandom always returns 0xAB bytes.
    -- We request 4 bytes at buf_ptr=3000 and verify each byte is 0xAB.

    describe("random_get", function()
        it("fills 4 bytes with 0xAB", function()
            local stub, memory = make_stub_and_memory()
            local buf_ptr = 3000
            local buf_len = 4

            local results = call_wasi(stub, "random_get", { buf_ptr, buf_len })
            assert.equals(0, results[1].value)

            -- All 4 bytes should be 0xAB = 171.
            for i = 0, buf_len - 1 do
                assert.equals(0xAB, memory:load_i32_8u(buf_ptr + i))
            end
        end)

        it("fills 0 bytes without error", function()
            local stub, _ = make_stub_and_memory()
            local results = call_wasi(stub, "random_get", { 3000, 0 })
            assert.equals(0, results[1].value)
        end)

        it("fills 1 byte correctly", function()
            local stub, memory = make_stub_and_memory()
            call_wasi(stub, "random_get", { 5000, 1 })
            assert.equals(0xAB, memory:load_i32_8u(5000))
        end)

        it("does not write beyond buf_len", function()
            local stub, memory = make_stub_and_memory()
            local buf_ptr = 4000
            -- Pre-fill the byte after the buffer with a sentinel.
            memory:store_i32_8(buf_ptr + 2, 0xFF)

            call_wasi(stub, "random_get", { buf_ptr, 2 })

            -- Sentinel byte should be unchanged.
            assert.equals(0xFF, memory:load_i32_8u(buf_ptr + 2))
        end)
    end)


    -- ── sched_yield ─────────────────────────────────────────────────────────
    --
    -- sched_yield() always returns ESUCCESS in a single-threaded environment.

    describe("sched_yield", function()
        it("returns errno=0 (ESUCCESS)", function()
            local stub, _ = make_stub_and_memory()
            local results = call_wasi(stub, "sched_yield", {})
            assert.equals(0, results[1].value)
        end)

        it("can be called multiple times", function()
            local stub, _ = make_stub_and_memory()
            for _ = 1, 5 do
                local results = call_wasi(stub, "sched_yield", {})
                assert.equals(0, results[1].value)
            end
        end)
    end)


    -- ── SystemClock ─────────────────────────────────────────────────────────

    describe("SystemClock", function()
        it("creates a SystemClock", function()
            local clock = wasm_runtime.SystemClock.new()
            assert.is_not_nil(clock)
        end)

        it("returns a non-negative realtime_ns", function()
            local clock = wasm_runtime.SystemClock.new()
            local ns = clock:realtime_ns()
            assert.is_true(ns > 0)
        end)

        it("returns a non-negative monotonic_ns", function()
            local clock = wasm_runtime.SystemClock.new()
            local ns = clock:monotonic_ns()
            -- os.clock() can be 0 at start, so we just check it's non-negative.
            assert.is_true(ns >= 0)
        end)

        it("returns 1000000000 (1 second) as resolution", function()
            local clock = wasm_runtime.SystemClock.new()
            assert.equals(1000000000, clock:resolution_ns(0))
            assert.equals(1000000000, clock:resolution_ns(1))
        end)
    end)


    -- ── SystemRandom ────────────────────────────────────────────────────────

    describe("SystemRandom", function()
        it("creates a SystemRandom", function()
            local rng = wasm_runtime.SystemRandom.new()
            assert.is_not_nil(rng)
        end)

        it("fill_bytes(4) returns a table of 4 integers", function()
            local rng = wasm_runtime.SystemRandom.new()
            local bytes = rng:fill_bytes(4)
            assert.equals(4, #bytes)
        end)

        it("fill_bytes values are in range [0, 255]", function()
            local rng = wasm_runtime.SystemRandom.new()
            local bytes = rng:fill_bytes(100)
            for _, b in ipairs(bytes) do
                assert.is_true(b >= 0 and b <= 255)
            end
        end)

        it("fill_bytes(0) returns an empty table", function()
            local rng = wasm_runtime.SystemRandom.new()
            local bytes = rng:fill_bytes(0)
            assert.equals(0, #bytes)
        end)
    end)


    -- ── Existing square test still passes ───────────────────────────────────
    --
    -- Regression check: none of the new code breaks the existing runtime.

    describe("regression: square function", function()
        it("square(5) = 25 through load_and_run", function()
            local runtime = wasm_runtime.WasmRuntime.new()
            local results = runtime:load_and_run(build_square_wasm(), "square", { 5 })
            assert.equals(1, #results)
            assert.equals(25, results[1])
        end)

        it("square(0) = 0", function()
            local runtime = wasm_runtime.WasmRuntime.new()
            local results = runtime:load_and_run(build_square_wasm(), "square", { 0 })
            assert.equals(0, results[1])
        end)
    end)

end)
