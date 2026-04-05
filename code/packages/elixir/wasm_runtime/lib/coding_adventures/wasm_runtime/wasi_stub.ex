defmodule CodingAdventures.WasmRuntime.WasiStub do
  @moduledoc """
  Stub implementation of WASI (WebAssembly System Interface).

  WASI provides system-level capabilities (file I/O, clocks, random numbers)
  to WASM modules. This stub provides minimal implementations that allow
  WASI-using modules to link without trapping on import resolution.

  Most functions are no-ops or return "not supported" error codes.
  This is sufficient for running simple computational WASM modules
  that were compiled with WASI support but do not actually use I/O.

  ## WASI Functions Stubbed

      +------------------------------+------------------------------------------+
      | Function                     | Behavior                                 |
      +------------------------------+------------------------------------------+
      | proc_exit                    | Returns [] (no-op)                       |
      | fd_write                     | Returns [errno=8] (EBADF)                |
      | fd_read                      | Returns [errno=8] (EBADF)                |
      | fd_close                     | Returns [errno=8] (EBADF)                |
      | fd_seek                      | Returns [errno=8] (EBADF)                |
      | environ_sizes_get            | Returns [errno=0] (empty env)            |
      | environ_get                  | Returns [errno=0] (empty env)            |
      | args_sizes_get               | Returns [errno=0] (no args)              |
      | args_get                     | Returns [errno=0] (no args)              |
      +------------------------------+------------------------------------------+
  """

  alias CodingAdventures.WasmExecution.Values

  @ebadf 8   # Bad file descriptor errno

  @doc """
  Return a map of WASI host function handlers.

  Each entry maps `{"wasi_snapshot_preview1", func_name}` to a handler
  function that accepts a list of WasmValue arguments and returns a list
  of WasmValue results.
  """
  @spec host_functions() :: map()
  def host_functions do
    %{
      {"wasi_snapshot_preview1", "proc_exit"} => fn _args -> [] end,
      {"wasi_snapshot_preview1", "fd_write"} => fn _args -> [Values.i32(@ebadf)] end,
      {"wasi_snapshot_preview1", "fd_read"} => fn _args -> [Values.i32(@ebadf)] end,
      {"wasi_snapshot_preview1", "fd_close"} => fn _args -> [Values.i32(@ebadf)] end,
      {"wasi_snapshot_preview1", "fd_seek"} => fn _args -> [Values.i32(@ebadf)] end,
      {"wasi_snapshot_preview1", "environ_sizes_get"} => fn _args -> [Values.i32(0)] end,
      {"wasi_snapshot_preview1", "environ_get"} => fn _args -> [Values.i32(0)] end,
      {"wasi_snapshot_preview1", "args_sizes_get"} => fn _args -> [Values.i32(0)] end,
      {"wasi_snapshot_preview1", "args_get"} => fn _args -> [Values.i32(0)] end
    }
  end
end
