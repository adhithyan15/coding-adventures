# frozen_string_literal: true

require "minitest/autorun"
require "coding_adventures_wasm_runtime"

# ==========================================================================
# Tests for WasiStub Tier 3 --- args, environ, clock, random, sched_yield
# ==========================================================================
#
# Tier 3 adds 8 new WASI functions on top of the existing fd_write and
# proc_exit:
#
#   args_sizes_get    — report argc and total argv buffer size
#   args_get          — fill argv pointers + string data into memory
#   environ_sizes_get — report envcount and total environ buffer size
#   environ_get       — fill environ pointers + "KEY=VALUE\0" data into memory
#   clock_res_get     — write clock resolution (i64 ns) to memory
#   clock_time_get    — write current time (i64 ns) to memory
#   random_get        — fill a memory region with random bytes
#   sched_yield       — no-op success
#
# All tests use injectable fakes (FakeClock, FakeRandom) so:
#   - No wall-clock skew can break assertions.
#   - No OS entropy pool is required for test isolation.
#   - Tests remain deterministic across platforms and time zones.
#
# Test structure:
#   1. FakeClock / FakeRandom definitions
#   2. args_sizes_get
#   3. args_get (pointer array + string bytes)
#   4. environ_sizes_get
#   5. environ_get
#   6. clock_time_get, clock_id 0 (realtime)
#   7. clock_time_get, clock_id 1 (monotonic)
#   8. clock_res_get
#   9. random_get
#  10. sched_yield
#  11. Backward-compatibility: existing square test still works
# ==========================================================================

class TestWasiTier3 < Minitest::Test
  WE = CodingAdventures::WasmExecution
  WT = CodingAdventures::WasmTypes
  WR = CodingAdventures::WasmRuntime

  # ── Test doubles ──────────────────────────────────────────────────────────

  # FakeClock returns fixed nanosecond values so tests are deterministic.
  #
  # realtime_ns  = 1_700_000_000_000_000_001
  #   → 2023-11-14 22:13:20.000000001 UTC  (just past a round number)
  #
  # monotonic_ns = 42_000_000_000
  #   → 42 seconds since monotonic start; easy to spot in memory dumps
  #
  # resolution_ns = 1_000_000  (1 ms — same as SystemClock default)
  class FakeClock
    def realtime_ns = 1_700_000_000_000_000_001
    def monotonic_ns = 42_000_000_000
    def resolution_ns(_id) = 1_000_000
  end

  # FakeRandom always returns 0xAB bytes.
  # This makes the test assertion trivial: every byte in the filled
  # region must equal 0xAB.
  class FakeRandom
    def fill_bytes(n) = Array.new(n, 0xAB)
  end

  # Build a fresh 1-page LinearMemory (64 KiB) for each test.
  def fresh_memory
    WE::LinearMemory.new(1)
  end

  # Build a WasiStub with given args/env and the fake clock+random.
  def make_stub(args: [], env: {})
    WR::WasiStub.new(
      args: args,
      env: env,
      clock: FakeClock.new,
      random: FakeRandom.new
    )
  end

  # ── 1. args_sizes_get ────────────────────────────────────────────────────

  def test_args_sizes_get_returns_argc_and_buf_size
    # args = ["myapp", "hello"]
    # null-terminated:  "myapp\0" = 6 bytes, "hello\0" = 6 bytes → total 12
    stub = make_stub(args: ["myapp", "hello"])
    mem = fresh_memory
    stub.set_memory(mem)

    fn = stub.resolve_function("wasi_snapshot_preview1", "args_sizes_get")
    result = fn.call([WE.i32(0), WE.i32(4)])  # argc_ptr=0, buf_size_ptr=4

    assert_equal 0, result[0].value, "args_sizes_get should return ESUCCESS"

    argc = mem.load_i32(0)
    buf_size = mem.load_i32(4)

    assert_equal 2, argc, "argc must be 2 for two arguments"
    assert_equal 12, buf_size, "buf_size = 6 + 6 = 12 (null-terminated)"
  end

  def test_args_sizes_get_empty_args
    stub = make_stub(args: [])
    mem = fresh_memory
    stub.set_memory(mem)

    fn = stub.resolve_function("wasi_snapshot_preview1", "args_sizes_get")
    result = fn.call([WE.i32(0), WE.i32(4)])

    assert_equal 0, result[0].value
    assert_equal 0, mem.load_i32(0), "argc = 0 for empty args"
    assert_equal 0, mem.load_i32(4), "buf_size = 0 for empty args"
  end

  # ── 2. args_get ──────────────────────────────────────────────────────────

  def test_args_get_writes_pointers_and_strings
    # Memory layout after args_get(argv_ptr=0, argv_buf_ptr=100):
    #   mem[0..3]   = i32 pointer to "myapp\0"  → 100
    #   mem[4..7]   = i32 pointer to "hello\0"  → 106
    #   mem[100..105] = 'm','y','a','p','p',0
    #   mem[106..111] = 'h','e','l','l','o',0
    stub = make_stub(args: ["myapp", "hello"])
    mem = fresh_memory
    stub.set_memory(mem)

    fn = stub.resolve_function("wasi_snapshot_preview1", "args_get")
    result = fn.call([WE.i32(0), WE.i32(100)])  # argv_ptr=0, argv_buf_ptr=100

    assert_equal 0, result[0].value, "args_get should return ESUCCESS"

    # Verify pointer array
    ptr0 = mem.load_i32(0)
    ptr1 = mem.load_i32(4)
    assert_equal 100, ptr0, "argv[0] should point to offset 100"
    assert_equal 106, ptr1, "argv[1] should point to offset 106 (100 + 6)"

    # Verify string bytes (including null terminator)
    myapp_bytes = 6.times.map { |i| mem.load_i32_8u(100 + i) }
    assert_equal "myapp\0".bytes, myapp_bytes, "argv[0] string must be 'myapp\\0'"

    hello_bytes = 6.times.map { |i| mem.load_i32_8u(106 + i) }
    assert_equal "hello\0".bytes, hello_bytes, "argv[1] string must be 'hello\\0'"
  end

  def test_args_get_single_arg
    stub = make_stub(args: ["prog"])
    mem = fresh_memory
    stub.set_memory(mem)

    fn = stub.resolve_function("wasi_snapshot_preview1", "args_get")
    result = fn.call([WE.i32(0), WE.i32(50)])

    assert_equal 0, result[0].value
    assert_equal 50, mem.load_i32(0), "argv[0] ptr = 50"

    bytes = 5.times.map { |i| mem.load_i32_8u(50 + i) }
    assert_equal "prog\0".bytes, bytes
  end

  # ── 3. environ_sizes_get ─────────────────────────────────────────────────

  def test_environ_sizes_get_single_var
    # env = {"HOME" => "/home/user"}
    # Serialised as "HOME=/home/user\0" which is 16 bytes (15 chars + 1 null).
    stub = make_stub(env: {"HOME" => "/home/user"})
    mem = fresh_memory
    stub.set_memory(mem)

    fn = stub.resolve_function("wasi_snapshot_preview1", "environ_sizes_get")
    result = fn.call([WE.i32(0), WE.i32(4)])

    assert_equal 0, result[0].value
    assert_equal 1, mem.load_i32(0), "count = 1 variable"
    assert_equal 16, mem.load_i32(4), "'HOME=/home/user\\0' = 16 bytes (15 chars + null)"
  end

  def test_environ_sizes_get_empty_env
    stub = make_stub(env: {})
    mem = fresh_memory
    stub.set_memory(mem)

    fn = stub.resolve_function("wasi_snapshot_preview1", "environ_sizes_get")
    result = fn.call([WE.i32(0), WE.i32(4)])

    assert_equal 0, result[0].value
    assert_equal 0, mem.load_i32(0)
    assert_equal 0, mem.load_i32(4)
  end

  def test_environ_sizes_get_multiple_vars
    stub = make_stub(env: {"A" => "1", "BB" => "22"})
    mem = fresh_memory
    stub.set_memory(mem)

    fn = stub.resolve_function("wasi_snapshot_preview1", "environ_sizes_get")
    result = fn.call([WE.i32(0), WE.i32(4)])

    # "A=1\0"   = 4 bytes  (3 chars + null)
    # "BB=22\0" = 6 bytes  (5 chars + null)
    # total     = 10 bytes
    assert_equal 0, result[0].value
    assert_equal 2, mem.load_i32(0), "count = 2"
    assert_equal 10, mem.load_i32(4), "4 + 6 = 10 bytes"
  end

  # ── 4. environ_get ───────────────────────────────────────────────────────

  def test_environ_get_writes_pointer_and_string
    stub = make_stub(env: {"HOME" => "/home/user"})
    mem = fresh_memory
    stub.set_memory(mem)

    fn = stub.resolve_function("wasi_snapshot_preview1", "environ_get")
    result = fn.call([WE.i32(0), WE.i32(50)])  # environ_ptr=0, buf_ptr=50

    assert_equal 0, result[0].value

    # environ[0] pointer must point to the buffer start
    assert_equal 50, mem.load_i32(0), "environ[0] ptr = 50"

    # "HOME=/home/user\0" = 16 bytes at offset 50 (15 chars + 1 null)
    expected = "HOME=/home/user\0".bytes
    actual = 16.times.map { |i| mem.load_i32_8u(50 + i) }
    assert_equal expected, actual, "environ[0] string must be 'HOME=/home/user\\0'"
  end

  def test_environ_get_multiple_vars
    stub = make_stub(env: {"A" => "1", "BB" => "22"})
    mem = fresh_memory
    stub.set_memory(mem)

    fn = stub.resolve_function("wasi_snapshot_preview1", "environ_get")
    result = fn.call([WE.i32(0), WE.i32(100)])

    assert_equal 0, result[0].value

    ptr0 = mem.load_i32(0)
    ptr1 = mem.load_i32(4)
    assert_equal 100, ptr0, "first var at 100"
    # "A=1\0" is 4 bytes, so second var starts at 100 + 4 = 104
    assert_equal 104, ptr1, "second var at 104 (100 + 4 bytes for 'A=1\\0')"

    # "A=1\0" = 4 bytes
    s0 = 4.times.map { |i| mem.load_i32_8u(100 + i) }
    assert_equal "A=1\0".bytes, s0

    # "BB=22\0" = 6 bytes (not 7)
    s1 = 6.times.map { |i| mem.load_i32_8u(104 + i) }
    assert_equal "BB=22\0".bytes, s1
  end

  # ── 5. clock_time_get, clock_id 0 (realtime) ─────────────────────────────

  def test_clock_time_get_realtime
    # FakeClock.realtime_ns = 1_700_000_000_000_000_001
    # That value must appear as a little-endian i64 at time_ptr.
    stub = make_stub
    mem = fresh_memory
    stub.set_memory(mem)

    fn = stub.resolve_function("wasi_snapshot_preview1", "clock_time_get")
    # clock_time_get(id=0, precision=0 [i64], time_ptr=100)
    result = fn.call([WE.i32(0), WE.i64(0), WE.i32(100)])

    assert_equal 0, result[0].value, "clock_time_get should return ESUCCESS"

    ts = mem.load_i64(100)
    assert_equal 1_700_000_000_000_000_001, ts,
      "realtime timestamp must match FakeClock.realtime_ns"
  end

  # ── 6. clock_time_get, clock_id 1 (monotonic) ────────────────────────────

  def test_clock_time_get_monotonic
    # FakeClock.monotonic_ns = 42_000_000_000
    stub = make_stub
    mem = fresh_memory
    stub.set_memory(mem)

    fn = stub.resolve_function("wasi_snapshot_preview1", "clock_time_get")
    result = fn.call([WE.i32(1), WE.i64(0), WE.i32(200)])

    assert_equal 0, result[0].value
    assert_equal 42_000_000_000, mem.load_i64(200),
      "monotonic timestamp must match FakeClock.monotonic_ns"
  end

  def test_clock_time_get_process_cpu_maps_to_realtime
    # clock_id 2 (PROCESS_CPUTIME_ID) → realtime
    stub = make_stub
    mem = fresh_memory
    stub.set_memory(mem)

    fn = stub.resolve_function("wasi_snapshot_preview1", "clock_time_get")
    result = fn.call([WE.i32(2), WE.i64(0), WE.i32(300)])

    assert_equal 0, result[0].value
    assert_equal 1_700_000_000_000_000_001, mem.load_i64(300)
  end

  def test_clock_time_get_thread_cpu_maps_to_realtime
    # clock_id 3 (THREAD_CPUTIME_ID) → realtime
    stub = make_stub
    mem = fresh_memory
    stub.set_memory(mem)

    fn = stub.resolve_function("wasi_snapshot_preview1", "clock_time_get")
    result = fn.call([WE.i32(3), WE.i64(0), WE.i32(400)])

    assert_equal 0, result[0].value
    assert_equal 1_700_000_000_000_000_001, mem.load_i64(400)
  end

  def test_clock_time_get_unknown_id_returns_einval
    # An unknown clock ID should return EINVAL (28), not ENOSYS.
    stub = make_stub
    mem = fresh_memory
    stub.set_memory(mem)

    fn = stub.resolve_function("wasi_snapshot_preview1", "clock_time_get")
    result = fn.call([WE.i32(99), WE.i64(0), WE.i32(500)])

    assert_equal 28, result[0].value, "unknown clock id → EINVAL (28)"
  end

  # ── 7. clock_res_get ─────────────────────────────────────────────────────

  def test_clock_res_get_writes_resolution
    # FakeClock.resolution_ns = 1_000_000 (1 ms)
    stub = make_stub
    mem = fresh_memory
    stub.set_memory(mem)

    fn = stub.resolve_function("wasi_snapshot_preview1", "clock_res_get")
    result = fn.call([WE.i32(0), WE.i32(600)])  # id=0, resolution_ptr=600

    assert_equal 0, result[0].value

    res = mem.load_i64(600)
    assert_equal 1_000_000, res, "resolution must be 1_000_000 ns (1 ms)"
  end

  def test_clock_res_get_monotonic
    stub = make_stub
    mem = fresh_memory
    stub.set_memory(mem)

    fn = stub.resolve_function("wasi_snapshot_preview1", "clock_res_get")
    result = fn.call([WE.i32(1), WE.i32(700)])

    assert_equal 0, result[0].value
    assert_equal 1_000_000, mem.load_i64(700)
  end

  # ── 8. random_get ────────────────────────────────────────────────────────

  def test_random_get_fills_bytes
    # FakeRandom always returns 0xAB bytes, so every byte at buf_ptr..buf_ptr+3
    # should equal 0xAB after the call.
    stub = make_stub
    mem = fresh_memory
    stub.set_memory(mem)

    fn = stub.resolve_function("wasi_snapshot_preview1", "random_get")
    result = fn.call([WE.i32(800), WE.i32(4)])  # buf_ptr=800, buf_len=4

    assert_equal 0, result[0].value

    bytes = 4.times.map { |i| mem.load_i32_8u(800 + i) }
    assert_equal [0xAB, 0xAB, 0xAB, 0xAB], bytes,
      "all bytes must be 0xAB (FakeRandom)"
  end

  def test_random_get_zero_length
    # Requesting 0 bytes should succeed and not write anything.
    stub = make_stub
    mem = fresh_memory
    mem.store_i64_8(900, 0x55) # sentinel
    stub.set_memory(mem)

    fn = stub.resolve_function("wasi_snapshot_preview1", "random_get")
    result = fn.call([WE.i32(900), WE.i32(0)])

    assert_equal 0, result[0].value
    assert_equal 0x55, mem.load_i32_8u(900), "sentinel byte must be unchanged"
  end

  # ── 9. sched_yield ───────────────────────────────────────────────────────

  def test_sched_yield_returns_success
    stub = make_stub
    fn = stub.resolve_function("wasi_snapshot_preview1", "sched_yield")
    result = fn.call([])

    assert_equal 1, result.length, "sched_yield returns one value"
    assert_equal 0, result[0].value, "sched_yield returns ESUCCESS (0)"
    assert_equal WT::VALUE_TYPE[:i32], result[0].type,
      "return type must be i32"
  end

  # ── 10. Backward-compatibility — existing constructor keywords ─────────────

  def test_legacy_stdout_callback_keyword
    # The old API used stdout_callback: / stderr_callback: keywords.
    # These must still work after the constructor change.
    captured = []
    wasi = WR::WasiStub.new(stdout_callback: ->(t) { captured << t })
    mem = fresh_memory

    mem.write_bytes(100, "hi".b)
    mem.store_i32(0, 100)
    mem.store_i32(4, 2)
    wasi.set_memory(mem)

    fn = wasi.resolve_function("wasi_snapshot_preview1", "fd_write")
    fn.call([WE.i32(1), WE.i32(0), WE.i32(1), WE.i32(20)])

    assert_equal ["hi"], captured, "legacy stdout_callback: keyword must still work"
  end

  def test_new_stdout_keyword
    captured = []
    wasi = WR::WasiStub.new(stdout: ->(t) { captured << t })
    mem = fresh_memory

    mem.write_bytes(100, "ok".b)
    mem.store_i32(0, 100)
    mem.store_i32(4, 2)
    wasi.set_memory(mem)

    fn = wasi.resolve_function("wasi_snapshot_preview1", "fd_write")
    fn.call([WE.i32(1), WE.i32(0), WE.i32(1), WE.i32(20)])

    assert_equal ["ok"], captured, "new stdout: keyword must work"
  end

  # ── 11. Existing square WASM test still passes ───────────────────────────

  # Build square.wasm using the same LEB128-encoded helper pattern as test_runtime.rb.
  # (module
  #   (func (export "square") (param i32) (result i32)
  #     local.get 0
  #     local.get 0
  #     i32.mul))
  def build_square_wasm
    leb = CodingAdventures::WasmLeb128
    enc = ->(n) { leb.encode_unsigned(n).bytes }
    parts = []

    parts.push(0x00, 0x61, 0x73, 0x6D)  # magic
    parts.push(0x01, 0x00, 0x00, 0x00)  # version

    # Type section: (i32) -> (i32)
    type_payload = [0x01, 0x60, 0x01, 0x7F, 0x01, 0x7F]
    parts.push(0x01)
    parts.concat(enc.call(type_payload.length))
    parts.concat(type_payload)

    # Function section
    func_payload = [0x01, 0x00]
    parts.push(0x03)
    parts.concat(enc.call(func_payload.length))
    parts.concat(func_payload)

    # Export section
    name_bytes = "square".bytes
    export_payload = [0x01, *enc.call(name_bytes.length), *name_bytes, 0x00, 0x00]
    parts.push(0x07)
    parts.concat(enc.call(export_payload.length))
    parts.concat(export_payload)

    # Code section: local.get 0, local.get 0, i32.mul, end
    body_code = [0x20, 0x00, 0x20, 0x00, 0x6C, 0x0B]
    body_payload = [0x00, *body_code]
    func_body = [*enc.call(body_payload.length), *body_payload]
    code_payload = [0x01, *func_body]
    parts.push(0x0A)
    parts.concat(enc.call(code_payload.length))
    parts.concat(code_payload)

    parts.pack("C*")
  end

  def test_existing_square_wasm_still_works
    # This test proves that adding Tier 3 WASI functions does not break
    # the existing execution path. A pure-computation WASM module (no WASI
    # imports) must continue to work without modification.
    runtime = WR::Runtime.new
    result = runtime.load_and_run(build_square_wasm, "square", [5])
    assert_equal [25], result, "square(5) must still return 25"
  end
end
