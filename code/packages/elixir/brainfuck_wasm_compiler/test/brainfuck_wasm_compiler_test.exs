defmodule CodingAdventures.BrainfuckWasmCompilerTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.BrainfuckWasmCompiler
  alias CodingAdventures.WasmRuntime.{Runtime, WasiConfig, WasiStub}

  defp run_program(binary, opts \\ []) do
    {:ok, stdout_agent} = Agent.start_link(fn -> [] end)
    {:ok, stderr_agent} = Agent.start_link(fn -> [] end)

    config = %WasiConfig{
      stdin: Keyword.get(opts, :stdin, ""),
      stdout: fn text -> Agent.update(stdout_agent, &(&1 ++ [text])) end,
      stderr: fn text -> Agent.update(stderr_agent, &(&1 ++ [text])) end
    }

    host_functions = WasiStub.host_functions(config)
    assert {:ok, instance} = Runtime.instantiate_bytes(binary, host_functions)
    results = Runtime.call(instance, "_start", [])

    stdout = Agent.get(stdout_agent, &Enum.join/1)
    stderr = Agent.get(stderr_agent, &Enum.join/1)
    Agent.stop(stdout_agent)
    Agent.stop(stderr_agent)

    %{stdout: stdout, stderr: stderr, results: results}
  end

  test "compiles a Brainfuck source string to WASM" do
    assert {:ok, result} = BrainfuckWasmCompiler.compile_source("+++++.")
    executed = run_program(result.binary)
    assert executed.stdout == <<5>>
  end

  test "supports stdin via WASI fd_read" do
    assert {:ok, result} = BrainfuckWasmCompiler.compile_source(",[.,]")
    executed = run_program(result.binary, stdin: "echo")
    assert executed.stdout == "echo"
  end

  test "writes a wasm file to disk" do
    output_path = Path.join(__DIR__, "tmp_brainfuck_program.wasm")
    File.rm(output_path)

    assert {:ok, result} = BrainfuckWasmCompiler.write_wasm_file("+.", output_path)
    assert result.wasm_path == output_path
    assert {:ok, bytes} = File.read(output_path)
    assert byte_size(bytes) > 8

    File.rm(output_path)
  end
end
