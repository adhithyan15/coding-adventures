defmodule CodingAdventures.WasmRuntime.WasiConfig do
  @moduledoc """
  Configuration for the WASI host environment.

  A `WasiConfig` struct bundles all the parameters that a WASM module
  can observe through WASI system calls:

  - **args** — command-line arguments, starting with the program name.
    Exposed via `args_sizes_get` and `args_get`.

  - **env** — environment variables as a `%{key => value}` map.
    Exposed via `environ_sizes_get` and `environ_get`.

  - **stdout / stderr** — currently reserved for future use (e.g., piping
    WASM output to an Elixir IO device). Stubs ignore them for now.

  - **clock** — any module implementing `WasiClock`. Defaults to
    `SystemClock` (real OS time). Swap in a `FakeClock` for deterministic
    tests.

  - **random** — any module implementing `WasiRandom`. Defaults to
    `SystemRandom` (`:crypto` CSPRNG). Swap in `FakeRandom` for tests.

  ## Example

      config = %WasiConfig{
        args: ["myapp", "--flag"],
        env: %{"HOME" => "/home/user", "PATH" => "/usr/bin"},
        clock: FakeClock,
        random: FakeRandom
      }

      host_fns = WasiStub.host_functions(config)
  """

  defstruct args: [],
            env: %{},
            stdin: nil,
            stdout: nil,
            stderr: nil,
            clock: CodingAdventures.WasmRuntime.SystemClock,
            random: CodingAdventures.WasmRuntime.SystemRandom
end

defmodule CodingAdventures.WasmRuntime.WasiHost do
  @moduledoc """
  Preferred alias for the WASI host helper surface.

  `WasiStub` remains available for backwards compatibility, but new code
  should use `WasiHost` to reflect that this module now exposes real host
  behavior rather than a pure placeholder.
  """

  alias CodingAdventures.WasmRuntime.WasiStub

  @spec host_functions() :: map()
  defdelegate host_functions(), to: WasiStub

  @spec host_functions(CodingAdventures.WasmRuntime.WasiConfig.t()) :: map()
  defdelegate host_functions(config), to: WasiStub

  @spec call_with_memory(map(), String.t(), [CodingAdventures.WasmExecution.Values.wasm_value()], CodingAdventures.WasmExecution.LinearMemory.t()) ::
          {[CodingAdventures.WasmExecution.Values.wasm_value()], CodingAdventures.WasmExecution.LinearMemory.t()}
  defdelegate call_with_memory(host_fns, func_name, wasm_args, memory), to: WasiStub
end

defmodule CodingAdventures.WasmRuntime.WasiStub do
  @moduledoc """
  WASI Tier 3 host function implementations.

  WASI (WebAssembly System Interface) is a set of standard system calls
  that WASM modules can import. This module implements two tiers:

  ## Tier 1 — No-ops and error stubs

  Functions that WASM modules may import but that we choose not to support.
  They return WASI errno codes like EBADF (8) so the WASM module knows the
  operation failed.

      +------------------------------+------------------------------------------+
      | Function                     | Behavior                                 |
      +------------------------------+------------------------------------------+
      | proc_exit                    | Returns [] (no-op)                       |
      | fd_write                     | Returns [errno=8] (EBADF)                |
      | fd_read                      | Returns [errno=8] (EBADF)                |
      | fd_close                     | Returns [errno=8] (EBADF)                |
      | fd_seek                      | Returns [errno=8] (EBADF)                |
      +------------------------------+------------------------------------------+

  ## Tier 3 — Full implementations

  Functions with real implementations that write data into WASM linear memory.

      +------------------------------+------------------------------------------+
      | Function                     | Behavior                                 |
      +------------------------------+------------------------------------------+
      | args_sizes_get               | Writes argc + total buf size to memory   |
      | args_get                     | Writes null-terminated args to memory    |
      | environ_sizes_get            | Writes env count + total buf size        |
      | environ_get                  | Writes KEY=VALUE\\0 strings to memory     |
      | clock_res_get                | Writes clock resolution (i64) to memory |
      | clock_time_get               | Writes current time (i64) to memory      |
      | random_get                   | Fills buffer with random bytes           |
      | sched_yield                  | Returns success immediately              |
      +------------------------------+------------------------------------------+

  ## How WASM memory writing works

  WASM functions that write results to memory receive *pointer* arguments —
  i32 values that are byte offsets into the WASM module's linear memory.

  The host function must:
  1. Receive the pointer as an argument
  2. Fetch the LinearMemory from the execution context
  3. Write the value at the pointer offset
  4. Return errno 0 (success)

  In Elixir, `LinearMemory` is an immutable struct. Each `store_*` call
  returns a *new* struct. Since the wasi_stub handlers are closures that
  close over the config, but memory is threaded through the execution
  context by the engine, we use a different approach:

  The Tier 3 handlers accept an explicit `memory` argument from a wrapper
  that extracts it from the execution context. See `host_functions/1` for
  how handlers are structured.

  ## Memory-aware host functions

  Handlers that need to write to memory accept args as:

      [arg0, arg1, ..., memory: memory_ref]

  But since the GenericVM passes args as a plain list of WasmValues, we
  use a two-level approach:

  The handler closures close over `config` and look up memory via the
  `memory` key injected by the engine. However, the current engine design
  passes host functions as `fn args -> result` where `args` is a list of
  WasmValues. To integrate memory-writing WASI functions, we use a
  **convention**: we pass memory as the last argument (as a tagged value)
  via a specialized engine callback, OR we rely on the WasiStub being
  called with a separate `call_with_memory/4` API.

  For simplicity in this implementation, memory-writing functions use
  `WasiStub.call_with_memory/4` which wraps the handler and passes memory
  explicitly. The test file demonstrates this pattern.

  ## EINVAL error code

  WASI errno 28 = EINVAL (invalid argument). Returned when an unsupported
  clock ID is requested.
  """

  alias CodingAdventures.WasmExecution.{Values, LinearMemory}
  alias CodingAdventures.WasmRuntime.WasiConfig

  # errno 8  = EBADF (bad file descriptor)
  @ebadf_fd 8
  # errno 28 = EINVAL (invalid argument)
  @einval 28
  # errno 0  = success
  @esuccess 0

  # ===========================================================================
  # Public API
  # ===========================================================================

  @doc """
  Return a map of WASI host function handlers with default configuration.

  Each handler in the map is a function `fn wasm_args -> [WasmValue]`.
  Handlers that only return error codes (Tier 1) ignore their arguments.

  For Tier 3 handlers that write to memory, use `host_functions_with_memory/2`
  instead, which returns handlers that accept `{wasm_args, memory}` and
  return `{[WasmValue], updated_memory}`.
  """
  @spec host_functions() :: map()
  def host_functions() do
    host_functions(%WasiConfig{})
  end

  @doc """
  Return a map of WASI host function handlers configured with `config`.

  ## Parameters

  - `config` — a `WasiConfig` struct with args, env, clock, and random.

  Tier 1 handlers (no-ops and EBADF stubs) ignore the config.
  Tier 3 handlers use `config.args`, `config.env`, `config.clock`, and
  `config.random`.

  ## Memory-writing handlers

  Handlers like `args_sizes_get`, `args_get`, `environ_sizes_get`,
  `environ_get`, `clock_res_get`, `clock_time_get`, and `random_get`
  need to write into WASM linear memory. Because the host function
  signature is `fn wasm_args -> [WasmValue]` (no memory access), these
  handlers use a workaround:

  They return `{:memory_op, fn memory -> {[result], updated_memory} end}`
  so the caller can thread memory through. See `call_with_memory/4` for
  the pattern used in tests.

  For production use, the engine should be extended to pass memory to
  host functions. The current architecture stores this in ctx.memory.
  """
  @spec host_functions(WasiConfig.t()) :: map()
  def host_functions(%WasiConfig{} = config) do
    stdin_reader = build_stdin_reader(config.stdin)

    %{
      # --- Tier 1: Unconditional stubs ---
      {"wasi_snapshot_preview1", "proc_exit"} => fn _args -> [] end,
      {"wasi_snapshot_preview1", "fd_write"} => fn args -> handle_fd_write(args, config) end,
      {"wasi_snapshot_preview1", "fd_read"} => fn args ->
        handle_fd_read(args, config, stdin_reader)
      end,
      {"wasi_snapshot_preview1", "fd_close"} => fn _args -> [Values.i32(@ebadf_fd)] end,
      {"wasi_snapshot_preview1", "fd_seek"} => fn _args -> [Values.i32(@ebadf_fd)] end,

      # --- Tier 3: Memory-writing functions ---
      # These are called via call_with_memory/4 in tests.
      {"wasi_snapshot_preview1", "args_sizes_get"} => fn args ->
        handle_args_sizes_get(args, config)
      end,
      {"wasi_snapshot_preview1", "args_get"} => fn args -> handle_args_get(args, config) end,
      {"wasi_snapshot_preview1", "environ_sizes_get"} => fn args ->
        handle_environ_sizes_get(args, config)
      end,
      {"wasi_snapshot_preview1", "environ_get"} => fn args -> handle_environ_get(args, config) end,
      {"wasi_snapshot_preview1", "clock_res_get"} => fn args ->
        handle_clock_res_get(args, config)
      end,
      {"wasi_snapshot_preview1", "clock_time_get"} => fn args ->
        handle_clock_time_get(args, config)
      end,
      {"wasi_snapshot_preview1", "random_get"} => fn args -> handle_random_get(args, config) end,
      {"wasi_snapshot_preview1", "sched_yield"} => fn _args -> [Values.i32(@esuccess)] end
    }
  end

  @doc """
  Call a WASI host function that may need to write to memory.

  This is the primary API for Tier 3 memory-writing functions. It:
  1. Looks up the handler in `host_fns`
  2. Calls the handler with `{wasm_args, memory}`
  3. Returns `{results, updated_memory}`

  ## Why this extra layer?

  The GenericVM's host function signature is `fn args -> results`.
  That design works for pure functions (no side effects) but fails for
  WASI functions that must write into linear memory.

  Rather than change the engine today, tests call this helper directly,
  and it can be adapted later when the engine supports memory-passing
  host functions.

  ## Parameters

  - `host_fns` — the map from `host_functions/1`
  - `func_name` — the WASI function name (e.g., `"args_sizes_get"`)
  - `wasm_args` — list of WasmValues (the i32 pointer arguments)
  - `memory` — the LinearMemory struct to read/write

  ## Returns

  `{[WasmValue], updated_memory}` where the first element is the
  list of return values and the second is the possibly-updated memory.
  """
  @spec call_with_memory(map(), String.t(), [Values.wasm_value()], LinearMemory.t()) ::
          {[Values.wasm_value()], LinearMemory.t()}
  def call_with_memory(host_fns, func_name, wasm_args, memory) do
    key = {"wasi_snapshot_preview1", func_name}
    handler = Map.fetch!(host_fns, key)
    # Pass memory as part of args using a tagged tuple convention
    handler.({wasm_args, memory})
  end

  # ===========================================================================
  # Tier 3: args_sizes_get
  # ===========================================================================

  # handle_args_sizes_get(argc_ptr: i32, argv_buf_size_ptr: i32) → errno
  # Writes argc and total buffer size to memory.
  # Called with {[argc_ptr_val, buf_size_ptr_val], memory}.
  defp handle_args_sizes_get({wasm_args, memory}, %WasiConfig{args: args}) do
    [argc_ptr_val, buf_size_ptr_val] = wasm_args
    argc_ptr = Values.as_i32(argc_ptr_val)
    buf_size_ptr = Values.as_i32(buf_size_ptr_val)

    argc = length(args)
    # Each arg gets a null terminator, so buf size = sum of (len + 1)
    buf_size =
      Enum.reduce(args, 0, fn arg, total ->
        total + byte_size(arg) + 1
      end)

    mem1 = LinearMemory.store_i32(memory, argc_ptr, argc)
    mem2 = LinearMemory.store_i32(mem1, buf_size_ptr, buf_size)

    {[Values.i32(@esuccess)], mem2}
  end

  defp handle_args_sizes_get(plain_args, config) when is_list(plain_args) do
    # Fallback for non-memory-passing context — return success with zero counts
    _ = config
    [Values.i32(@esuccess)]
  end

  # ===========================================================================
  # Tier 3: args_get
  # ===========================================================================

  # handle_args_get(argv_ptr: i32, argv_buf_ptr: i32) → errno
  # Writes null-terminated args and pointer array to memory.
  # argv area at argv_ptr (4 bytes per pointer), buf area at argv_buf_ptr.
  defp handle_args_get({wasm_args, memory}, %WasiConfig{args: args}) do
    [argv_ptr_val, argv_buf_ptr_val] = wasm_args
    argv_ptr = Values.as_i32(argv_ptr_val)
    buf_start = Values.as_i32(argv_buf_ptr_val)

    # Write strings into buf area and build the pointer array
    {final_mem, _final_offset} =
      args
      |> Enum.with_index()
      |> Enum.reduce({memory, buf_start}, fn {arg, idx}, {mem, offset} ->
        # The pointer for this arg goes into the argv array
        ptr_location = argv_ptr + idx * 4
        mem2 = LinearMemory.store_i32(mem, ptr_location, offset)

        # Write the null-terminated string into the buf area
        null_terminated = arg <> <<0>>
        bytes = :binary.bin_to_list(null_terminated)

        mem3 =
          Enum.reduce(Enum.with_index(bytes), mem2, fn {byte_val, i}, m ->
            LinearMemory.store_i32_8(m, offset + i, byte_val)
          end)

        {mem3, offset + byte_size(null_terminated)}
      end)

    {[Values.i32(@esuccess)], final_mem}
  end

  defp handle_args_get(plain_args, _config) when is_list(plain_args) do
    [Values.i32(@esuccess)]
  end

  # ===========================================================================
  # Tier 3: environ_sizes_get
  # ===========================================================================

  # handle_environ_sizes_get(environ_count_ptr: i32, environ_buf_size_ptr: i32) → errno
  # Writes env count and total buffer size (sum of "KEY=VALUE\0" lengths) to memory.
  defp handle_environ_sizes_get({wasm_args, memory}, %WasiConfig{env: env}) do
    [count_ptr_val, buf_size_ptr_val] = wasm_args
    count_ptr = Values.as_i32(count_ptr_val)
    buf_size_ptr = Values.as_i32(buf_size_ptr_val)

    env_count = map_size(env)
    # Each env var is encoded as "KEY=VALUE\0"
    buf_size =
      Enum.reduce(env, 0, fn {key, value}, total ->
        total + byte_size(key) + 1 + byte_size(value) + 1
        # "KEY" + "=" + "VALUE" + "\0"
      end)

    mem1 = LinearMemory.store_i32(memory, count_ptr, env_count)
    mem2 = LinearMemory.store_i32(mem1, buf_size_ptr, buf_size)

    {[Values.i32(@esuccess)], mem2}
  end

  defp handle_environ_sizes_get(plain_args, _config) when is_list(plain_args) do
    [Values.i32(@esuccess)]
  end

  # ===========================================================================
  # Tier 3: environ_get
  # ===========================================================================

  # handle_environ_get(environ_ptr: i32, environ_buf_ptr: i32) → errno
  # Writes "KEY=VALUE\0" strings and pointer array to memory.
  # environ area at environ_ptr (4 bytes per pointer), buf at environ_buf_ptr.
  defp handle_environ_get({wasm_args, memory}, %WasiConfig{env: env}) do
    [environ_ptr_val, environ_buf_ptr_val] = wasm_args
    environ_ptr = Values.as_i32(environ_ptr_val)
    buf_start = Values.as_i32(environ_buf_ptr_val)

    env_list = Enum.to_list(env)

    {final_mem, _final_offset} =
      env_list
      |> Enum.with_index()
      |> Enum.reduce({memory, buf_start}, fn {{key, value}, idx}, {mem, offset} ->
        ptr_location = environ_ptr + idx * 4
        mem2 = LinearMemory.store_i32(mem, ptr_location, offset)

        # "KEY=VALUE\0"
        entry = key <> "=" <> value <> <<0>>
        bytes = :binary.bin_to_list(entry)

        mem3 =
          Enum.reduce(Enum.with_index(bytes), mem2, fn {byte_val, i}, m ->
            LinearMemory.store_i32_8(m, offset + i, byte_val)
          end)

        {mem3, offset + byte_size(entry)}
      end)

    {[Values.i32(@esuccess)], final_mem}
  end

  defp handle_environ_get(plain_args, _config) when is_list(plain_args) do
    [Values.i32(@esuccess)]
  end

  # ===========================================================================
  # Tier 3: clock_res_get
  # ===========================================================================

  # handle_clock_res_get(id: i32, resolution_ptr: i32) → errno
  # Writes clock resolution as i64 (little-endian, nanoseconds) at resolution_ptr.
  defp handle_clock_res_get({wasm_args, memory}, %WasiConfig{clock: clock_mod}) do
    [id_val, resolution_ptr_val] = wasm_args
    clock_id = Values.as_i32(id_val)
    resolution_ptr = Values.as_i32(resolution_ptr_val)

    ns = clock_mod.resolution_ns(clock_id)
    mem2 = LinearMemory.store_i64(memory, resolution_ptr, ns)

    {[Values.i32(@esuccess)], mem2}
  end

  defp handle_clock_res_get(plain_args, _config) when is_list(plain_args) do
    [Values.i32(@esuccess)]
  end

  # ===========================================================================
  # Tier 3: clock_time_get
  # ===========================================================================

  # handle_clock_time_get(id: i32, precision: i64, time_ptr: i32) → errno
  # Clock IDs: 0,2,3 → realtime_ns(), 1 → monotonic_ns(), other → EINVAL (28).
  # Writes ns timestamp as i64 (8 bytes, little-endian) at time_ptr.
  defp handle_clock_time_get({wasm_args, memory}, %WasiConfig{clock: clock_mod}) do
    # clock_time_get has 3 WASM args: id (i32), precision (i64), time_ptr (i32)
    # The i64 precision takes TWO i32 arg slots in WASM's calling convention,
    # but in our VM each WasmValue is typed, so it's one value.
    # We extract from the list positionally.
    case wasm_args do
      [id_val | rest] ->
        # time_ptr is the last argument
        time_ptr_val = List.last(rest)
        clock_id = Values.as_i32(id_val)
        time_ptr = Values.as_i32(time_ptr_val)

        ns_result =
          case clock_id do
            id when id in [0, 2, 3] -> clock_mod.realtime_ns()
            1 -> clock_mod.monotonic_ns()
            _ -> :einval
          end

        case ns_result do
          :einval ->
            {[Values.i32(@einval)], memory}

          ns ->
            mem2 = LinearMemory.store_i64(memory, time_ptr, ns)
            {[Values.i32(@esuccess)], mem2}
        end

      _ ->
        {[Values.i32(@einval)], memory}
    end
  end

  defp handle_clock_time_get(plain_args, _config) when is_list(plain_args) do
    [Values.i32(@esuccess)]
  end

  # ===========================================================================
  # Tier 3: random_get
  # ===========================================================================

  # handle_random_get(buf_ptr: i32, buf_len: i32) → errno
  # Fills buf_len bytes at buf_ptr with random data from config.random.fill_bytes/1.
  defp handle_random_get({wasm_args, memory}, %WasiConfig{random: random_mod}) do
    [buf_ptr_val, buf_len_val] = wasm_args
    buf_ptr = Values.as_i32(buf_ptr_val)
    buf_len = Values.as_i32(buf_len_val)

    rand_bytes = random_mod.fill_bytes(buf_len)
    bytes = :binary.bin_to_list(rand_bytes)

    final_mem =
      Enum.reduce(Enum.with_index(bytes), memory, fn {byte_val, i}, mem ->
        LinearMemory.store_i32_8(mem, buf_ptr + i, byte_val)
      end)

    {[Values.i32(@esuccess)], final_mem}
  end

  defp handle_random_get(plain_args, _config) when is_list(plain_args) do
    [Values.i32(@esuccess)]
  end

  # ===========================================================================
  # Runtime fd_write / fd_read support
  # ===========================================================================

  defp handle_fd_write({wasm_args, memory}, %WasiConfig{} = config) do
    [fd_val, iovs_ptr_val, iovs_len_val, nwritten_ptr_val] = wasm_args
    fd = Values.as_i32(fd_val)
    iovs_ptr = Values.as_i32(iovs_ptr_val)
    iovs_len = Values.as_i32(iovs_len_val)
    nwritten_ptr = Values.as_i32(nwritten_ptr_val)

    {text, total_written} =
      if iovs_len > 0 do
        Enum.reduce(0..(iovs_len - 1), {"", 0}, fn i, {acc, total} ->
          buf_ptr = LinearMemory.load_i32(memory, iovs_ptr + i * 8) |> Bitwise.band(0xFFFFFFFF)

          buf_len =
            LinearMemory.load_i32(memory, iovs_ptr + i * 8 + 4) |> Bitwise.band(0xFFFFFFFF)

          bytes =
            if buf_len > 0 do
              for j <- 0..(buf_len - 1), do: LinearMemory.load_i32_8u(memory, buf_ptr + j)
            else
              []
            end

          chunk = :binary.list_to_bin(bytes)
          {acc <> chunk, total + buf_len}
        end)
      else
        {"", 0}
      end

    case fd do
      1 -> if is_function(config.stdout, 1), do: config.stdout.(text)
      2 -> if is_function(config.stderr, 1), do: config.stderr.(text)
      _ -> :ok
    end

    memory = LinearMemory.store_i32(memory, nwritten_ptr, total_written)
    errno = if fd in [1, 2], do: @esuccess, else: @ebadf_fd
    {[Values.i32(errno)], memory}
  end

  defp handle_fd_write(plain_args, _config) when is_list(plain_args) do
    [Values.i32(@ebadf_fd)]
  end

  defp handle_fd_read({wasm_args, memory}, _config, stdin_reader) do
    [fd_val, iovs_ptr_val, iovs_len_val, nread_ptr_val] = wasm_args
    fd = Values.as_i32(fd_val)
    iovs_ptr = Values.as_i32(iovs_ptr_val)
    iovs_len = Values.as_i32(iovs_len_val)
    nread_ptr = Values.as_i32(nread_ptr_val)

    if fd != 0 do
      {[Values.i32(@ebadf_fd)], memory}
    else
      {memory, total_read, _done} =
        if iovs_len > 0 do
          Enum.reduce(0..(iovs_len - 1), {memory, 0, false}, fn i, {mem, total, done?} ->
            if done? do
              {mem, total, true}
            else
              buf_ptr = LinearMemory.load_i32(mem, iovs_ptr + i * 8) |> Bitwise.band(0xFFFFFFFF)

              buf_len =
                LinearMemory.load_i32(mem, iovs_ptr + i * 8 + 4) |> Bitwise.band(0xFFFFFFFF)

              chunk = stdin_reader.(buf_len)
              bytes = :binary.bin_to_list(chunk)

              updated_mem =
                Enum.reduce(Enum.with_index(bytes), mem, fn {byte_val, j}, acc ->
                  LinearMemory.store_i32_8(acc, buf_ptr + j, byte_val)
                end)

              chunk_len = byte_size(chunk)
              {updated_mem, total + chunk_len, chunk_len < buf_len}
            end
          end)
        else
          {memory, 0, true}
        end

      memory = LinearMemory.store_i32(memory, nread_ptr, total_read)
      {[Values.i32(@esuccess)], memory}
    end
  end

  defp handle_fd_read(plain_args, _config, _stdin_reader) when is_list(plain_args) do
    [Values.i32(@ebadf_fd)]
  end

  defp build_stdin_reader(nil), do: fn _max_bytes -> <<>> end

  defp build_stdin_reader(stdin) when is_binary(stdin) do
    {:ok, pid} = Agent.start_link(fn -> 0 end)

    fn max_bytes ->
      Agent.get_and_update(pid, fn offset ->
        if offset >= byte_size(stdin) do
          {<<>>, offset}
        else
          size = min(max_bytes, byte_size(stdin) - offset)
          chunk = binary_part(stdin, offset, size)
          {chunk, offset + size}
        end
      end)
    end
  end

  defp build_stdin_reader(stdin) when is_function(stdin, 1), do: stdin
end
