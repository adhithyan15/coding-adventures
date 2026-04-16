defmodule FakeClock do
  @moduledoc """
  Deterministic clock for testing WASI Tier 3 functions.

  Returns fixed timestamps so tests are reproducible regardless of
  wall-clock time. The values are chosen to be large (close to real
  Unix time) so they exercise the full 64-bit i64 range.

      realtime_ns  = 1_700_000_000_000_000_001  (year ~2023 in ns)
      monotonic_ns = 42_000_000_000              (42 seconds of uptime)
      resolution   = 1_000_000                   (1 ms)
  """

  @behaviour CodingAdventures.WasmRuntime.WasiClock

  @impl true
  def realtime_ns(), do: 1_700_000_000_000_000_001

  @impl true
  def monotonic_ns(), do: 42_000_000_000

  @impl true
  def resolution_ns(_id), do: 1_000_000
end

defmodule FakeRandom do
  @moduledoc """
  Deterministic random byte generator for testing.

  Always returns a binary of the given length filled with `0xAB`.
  This lets tests verify that the correct number of bytes were written
  to the correct memory location without relying on unpredictable output.
  """

  @behaviour CodingAdventures.WasmRuntime.WasiRandom

  @impl true
  def fill_bytes(n), do: :binary.list_to_bin(List.duplicate(0xAB, n))
end

defmodule CodingAdventures.WasmRuntime.WasiTier3Test do
  use ExUnit.Case, async: true

  @moduledoc """
  Tests for WASI Tier 3 host functions: args, environ, clock, random, sched_yield.

  ## Test strategy

  Each test:
  1. Creates a `WasiConfig` with known args, env, clock, or random.
  2. Builds a `WasiConfig` and calls `WasiStub.host_functions/1`.
  3. Creates a `LinearMemory` large enough to hold the written data.
  4. Calls `WasiStub.call_with_memory/4` which passes `{wasm_args, memory}`
     to the handler and returns `{results, updated_memory}`.
  5. Asserts on the returned results and reads back values from memory.

  ## Memory layout

  We start writing at offset 100 to leave room for pointer arrays.
  Pointer arrays (argv, environ) are written at offset 0.
  Data buffers (strings) are written at offset 100.
  i64 values (clock) are written at offset 200.
  Random bytes are written at offset 300.

  This avoids overlap between different regions of memory in the tests.
  """

  alias CodingAdventures.WasmRuntime.{WasiHost, WasiStub, WasiConfig}
  alias CodingAdventures.WasmExecution.{LinearMemory, Values}

  # Create a 1-page (64 KiB) linear memory, zero-initialized.
  defp fresh_memory(), do: LinearMemory.new(1)

  # Build host functions with a config that uses fake clock + random.
  defp test_config(args \\ [], env \\ %{}, stdin \\ nil) do
    %WasiConfig{
      args: args,
      env: env,
      stdin: stdin,
      clock: FakeClock,
      random: FakeRandom
    }
  end

  # ===========================================================================
  # 1. args_sizes_get
  # ===========================================================================

  test "WasiHost delegates to WasiStub" do
    config = test_config(["myapp"], %{})
    assert WasiHost.host_functions(config) == WasiStub.host_functions(config)
  end

  test "args_sizes_get with [\"myapp\", \"hello\"] writes argc=2 and buf_size=12" do
    # "myapp\0" = 6 bytes, "hello\0" = 6 bytes, total = 12
    config = test_config(["myapp", "hello"])
    host_fns = WasiStub.host_functions(config)
    memory = fresh_memory()

    argc_ptr = 0
    buf_size_ptr = 4

    wasm_args = [Values.i32(argc_ptr), Values.i32(buf_size_ptr)]
    {results, mem2} = WasiStub.call_with_memory(host_fns, "args_sizes_get", wasm_args, memory)

    assert results == [Values.i32(0)], "Expected success (errno 0)"
    assert LinearMemory.load_i32(mem2, argc_ptr) == 2, "Expected argc = 2"
    assert LinearMemory.load_i32(mem2, buf_size_ptr) == 12, "Expected buf_size = 12"
  end

  test "args_sizes_get with no args writes argc=0 and buf_size=0" do
    config = test_config([])
    host_fns = WasiStub.host_functions(config)
    memory = fresh_memory()

    wasm_args = [Values.i32(0), Values.i32(4)]
    {results, mem2} = WasiStub.call_with_memory(host_fns, "args_sizes_get", wasm_args, memory)

    assert results == [Values.i32(0)]
    assert LinearMemory.load_i32(mem2, 0) == 0
    assert LinearMemory.load_i32(mem2, 4) == 0
  end

  # ===========================================================================
  # 2. args_get
  # ===========================================================================

  test "args_get with [\"myapp\", \"hello\"] writes pointer array and null-terminated strings" do
    config = test_config(["myapp", "hello"])
    host_fns = WasiStub.host_functions(config)
    memory = fresh_memory()

    # argv pointer array at offset 0 (2 pointers × 4 bytes = 8 bytes)
    # buf area at offset 100
    argv_ptr = 0
    buf_ptr = 100

    wasm_args = [Values.i32(argv_ptr), Values.i32(buf_ptr)]
    {results, mem2} = WasiStub.call_with_memory(host_fns, "args_get", wasm_args, memory)

    assert results == [Values.i32(0)], "Expected success"

    # argv[0] should point to buf_ptr (100), argv[1] to buf_ptr + 6 (106)
    assert LinearMemory.load_i32(mem2, 0) == 100, "argv[0] should point to buf_ptr"
    assert LinearMemory.load_i32(mem2, 4) == 106, "argv[1] should point to buf_ptr + 6"

    # Read "myapp\0" from buf_ptr
    myapp_bytes = for i <- 0..5, do: LinearMemory.load_i32_8u(mem2, 100 + i)
    assert myapp_bytes == [?m, ?y, ?a, ?p, ?p, 0], "Expected 'myapp\\0' in buf"

    # Read "hello\0" from buf_ptr + 6
    hello_bytes = for i <- 0..5, do: LinearMemory.load_i32_8u(mem2, 106 + i)
    assert hello_bytes == [?h, ?e, ?l, ?l, ?o, 0], "Expected 'hello\\0' in buf"
  end

  test "fd_write writes stdout text and nwritten" do
    parent = self()

    config =
      %WasiConfig{
        stdout: fn text -> send(parent, {:stdout, text}) end,
        clock: FakeClock,
        random: FakeRandom
      }

    host_fns = WasiStub.host_functions(config)
    memory = fresh_memory()

    memory =
      memory
      |> LinearMemory.store_i32(0, 100)
      |> LinearMemory.store_i32(4, 2)
      |> LinearMemory.store_i32_8(100, ?H)
      |> LinearMemory.store_i32_8(101, ?i)

    wasm_args = [Values.i32(1), Values.i32(0), Values.i32(1), Values.i32(8)]
    {results, mem2} = WasiStub.call_with_memory(host_fns, "fd_write", wasm_args, memory)

    assert results == [Values.i32(0)]
    assert_receive {:stdout, "Hi"}
    assert LinearMemory.load_i32(mem2, 8) == 2
  end

  test "fd_read reads bytes from stdin and writes nread" do
    config = test_config([], %{}, "ok")
    host_fns = WasiStub.host_functions(config)
    memory = fresh_memory()

    memory =
      memory
      |> LinearMemory.store_i32(0, 100)
      |> LinearMemory.store_i32(4, 2)

    wasm_args = [Values.i32(0), Values.i32(0), Values.i32(1), Values.i32(8)]
    {results, mem2} = WasiStub.call_with_memory(host_fns, "fd_read", wasm_args, memory)

    assert results == [Values.i32(0)]
    assert LinearMemory.load_i32(mem2, 8) == 2
    assert LinearMemory.load_i32_8u(mem2, 100) == ?o
    assert LinearMemory.load_i32_8u(mem2, 101) == ?k
  end

  # ===========================================================================
  # 3. environ_sizes_get
  # ===========================================================================

  test "environ_sizes_get with %{HOME => /home/user} writes count=1 and buf_size=16" do
    # "HOME=/home/user\0" = 4 + 1 + 10 + 1 = 16 bytes
    # "HOME" (4) + "=" (1) + "/home/user" (10) + "\0" (1) = 16
    config = test_config([], %{"HOME" => "/home/user"})
    host_fns = WasiStub.host_functions(config)
    memory = fresh_memory()

    count_ptr = 0
    buf_size_ptr = 4

    wasm_args = [Values.i32(count_ptr), Values.i32(buf_size_ptr)]
    {results, mem2} = WasiStub.call_with_memory(host_fns, "environ_sizes_get", wasm_args, memory)

    assert results == [Values.i32(0)]
    assert LinearMemory.load_i32(mem2, count_ptr) == 1, "Expected env count = 1"
    assert LinearMemory.load_i32(mem2, buf_size_ptr) == 16, "Expected buf_size = 16"
  end

  test "environ_sizes_get with empty env writes count=0 and buf_size=0" do
    config = test_config([], %{})
    host_fns = WasiStub.host_functions(config)
    memory = fresh_memory()

    wasm_args = [Values.i32(0), Values.i32(4)]
    {results, mem2} = WasiStub.call_with_memory(host_fns, "environ_sizes_get", wasm_args, memory)

    assert results == [Values.i32(0)]
    assert LinearMemory.load_i32(mem2, 0) == 0
    assert LinearMemory.load_i32(mem2, 4) == 0
  end

  # ===========================================================================
  # 4. environ_get
  # ===========================================================================

  test "environ_get with %{HOME => /home/user} writes pointer and KEY=VALUE string" do
    config = test_config([], %{"HOME" => "/home/user"})
    host_fns = WasiStub.host_functions(config)
    memory = fresh_memory()

    # environ pointer array at offset 0 (1 pointer × 4 bytes = 4 bytes)
    # buf area at offset 100
    environ_ptr = 0
    buf_ptr = 100

    wasm_args = [Values.i32(environ_ptr), Values.i32(buf_ptr)]
    {results, mem2} = WasiStub.call_with_memory(host_fns, "environ_get", wasm_args, memory)

    assert results == [Values.i32(0)], "Expected success"

    # environ[0] should point to buf_ptr (100)
    assert LinearMemory.load_i32(mem2, 0) == 100, "environ[0] should point to buf_ptr"

    # Read "HOME=/home/user\0" from buf_ptr
    # "HOME=/home/user\0" = 4 + 1 + 10 + 1 = 16 bytes
    expected = :binary.bin_to_list("HOME=/home/user" <> <<0>>)
    actual = for i <- 0..15, do: LinearMemory.load_i32_8u(mem2, 100 + i)
    assert actual == expected, "Expected 'HOME=/home/user\\0' in buf"
  end

  # ===========================================================================
  # 5. clock_time_get (REALTIME, clock ID 0)
  # ===========================================================================

  test "clock_time_get(id=0) with FakeClock writes realtime_ns as i64" do
    config = test_config()
    host_fns = WasiStub.host_functions(config)
    memory = fresh_memory()

    # clock_time_get(id: i32=0, precision: i64=0, time_ptr: i32=200)
    # In our VM args are typed values: [i32(0), i64(0), i32(200)]
    time_ptr = 200
    wasm_args = [Values.i32(0), Values.i64(0), Values.i32(time_ptr)]
    {results, mem2} = WasiStub.call_with_memory(host_fns, "clock_time_get", wasm_args, memory)

    assert results == [Values.i32(0)], "Expected success"

    ns = LinearMemory.load_i64(mem2, time_ptr)

    assert ns == 1_700_000_000_000_000_001,
           "Expected FakeClock.realtime_ns() = 1_700_000_000_000_000_001, got #{ns}"
  end

  # ===========================================================================
  # 6. clock_time_get (MONOTONIC, clock ID 1)
  # ===========================================================================

  test "clock_time_get(id=1) with FakeClock writes monotonic_ns as i64" do
    config = test_config()
    host_fns = WasiStub.host_functions(config)
    memory = fresh_memory()

    time_ptr = 200
    wasm_args = [Values.i32(1), Values.i64(0), Values.i32(time_ptr)]
    {results, mem2} = WasiStub.call_with_memory(host_fns, "clock_time_get", wasm_args, memory)

    assert results == [Values.i32(0)]

    ns = LinearMemory.load_i64(mem2, time_ptr)

    assert ns == 42_000_000_000,
           "Expected FakeClock.monotonic_ns() = 42_000_000_000, got #{ns}"
  end

  # ===========================================================================
  # 7. clock_res_get
  # ===========================================================================

  test "clock_res_get(id=0) with FakeClock writes resolution_ns as i64" do
    config = test_config()
    host_fns = WasiStub.host_functions(config)
    memory = fresh_memory()

    resolution_ptr = 200
    wasm_args = [Values.i32(0), Values.i32(resolution_ptr)]
    {results, mem2} = WasiStub.call_with_memory(host_fns, "clock_res_get", wasm_args, memory)

    assert results == [Values.i32(0)]

    ns = LinearMemory.load_i64(mem2, resolution_ptr)
    assert ns == 1_000_000, "Expected resolution = 1_000_000 ns (1 ms), got #{ns}"
  end

  # ===========================================================================
  # 8. random_get
  # ===========================================================================

  test "random_get(buf_ptr=300, buf_len=4) with FakeRandom writes 4 bytes of 0xAB" do
    config = test_config()
    host_fns = WasiStub.host_functions(config)
    memory = fresh_memory()

    buf_ptr = 300
    buf_len = 4

    wasm_args = [Values.i32(buf_ptr), Values.i32(buf_len)]
    {results, mem2} = WasiStub.call_with_memory(host_fns, "random_get", wasm_args, memory)

    assert results == [Values.i32(0)]

    bytes = for i <- 0..3, do: LinearMemory.load_i32_8u(mem2, buf_ptr + i)

    assert bytes == [0xAB, 0xAB, 0xAB, 0xAB],
           "Expected 4 bytes of 0xAB from FakeRandom, got #{inspect(bytes)}"
  end

  # ===========================================================================
  # 9. sched_yield
  # ===========================================================================

  test "sched_yield returns i32(0) immediately" do
    config = test_config()
    host_fns = WasiStub.host_functions(config)
    key = {"wasi_snapshot_preview1", "sched_yield"}
    handler = Map.fetch!(host_fns, key)

    result = handler.([])
    assert result == [Values.i32(0)], "sched_yield should return success (errno 0)"
  end

  # ===========================================================================
  # 10. Regression: existing square test still passes via Runtime
  # ===========================================================================

  test "square function still executes correctly via Runtime (regression)" do
    # Build the same minimal WASM module as SquareTest
    wasm_bytes = build_square_wasm()

    {:ok, instance} = CodingAdventures.WasmRuntime.Runtime.instantiate_bytes(wasm_bytes)
    results = CodingAdventures.WasmRuntime.Runtime.call(instance, "square", [Values.i32(5)])

    assert [%{value: 25}] = results, "square(5) should still equal 25"
  end

  # Minimal WASM module binary for the square function.
  # (module (type (func (param i32) (result i32))) (func (type 0) local.get 0 local.get 0 i32.mul) (export "square" (func 0)))
  defp build_square_wasm do
    header = <<0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00>>

    type_entry = <<0x60, 0x01, 0x7F, 0x01, 0x7F>>
    type_section_body = <<0x01>> <> type_entry
    type_section = <<0x01>> <> <<byte_size(type_section_body)>> <> type_section_body

    func_section_body = <<0x01, 0x00>>
    func_section = <<0x03>> <> <<byte_size(func_section_body)>> <> func_section_body

    export_name = "square"
    export_entry = <<byte_size(export_name)>> <> export_name <> <<0x00, 0x00>>
    export_section_body = <<0x01>> <> export_entry
    export_section = <<0x07>> <> <<byte_size(export_section_body)>> <> export_section_body

    func_code = <<0x20, 0x00, 0x20, 0x00, 0x6C, 0x0B>>
    func_body = <<0x00>> <> func_code
    func_body_with_size = <<byte_size(func_body)>> <> func_body
    code_section_body = <<0x01>> <> func_body_with_size
    code_section = <<0x0A>> <> <<byte_size(code_section_body)>> <> code_section_body

    header <> type_section <> func_section <> export_section <> code_section
  end
end
