defmodule CodingAdventures.NibWasmCompilerTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.NibWasmCompiler
  alias CodingAdventures.WasmExecution.Values
  alias CodingAdventures.WasmRuntime.Runtime

  test "compile_source returns pipeline artifacts" do
    assert {:ok, result} = NibWasmCompiler.compile_source("fn answer() -> u4 { return 7; }")
    assert byte_size(result.binary) > 0
    assert length(result.raw_ir.instructions) > 0
  end

  test "pack_source aliases compile_source" do
    assert {:ok, compiled} = NibWasmCompiler.compile_source("fn answer() -> u4 { return 7; }")
    assert {:ok, packed} = NibWasmCompiler.pack_source("fn answer() -> u4 { return 7; }")
    assert packed.binary == compiled.binary
  end

  test "write_wasm_file writes bytes" do
    output_path = Path.join(__DIR__, "tmp_nib_program.wasm")
    File.rm(output_path)

    assert {:ok, result} = NibWasmCompiler.write_wasm_file("fn answer() -> u4 { return 7; }", output_path)
    assert result.wasm_path == output_path
    assert {:ok, bytes} = File.read(output_path)
    assert bytes == result.binary

    File.rm(output_path)
  end

  test "compiled _start path runs in the wasm runtime" do
    assert {:ok, result} =
             NibWasmCompiler.compile_source("""
             fn add(a: u4, b: u4) -> u4 { return a +% b; }
             fn main() -> u4 { return add(3, 4); }
             """)

    assert {:ok, instance} = Runtime.instantiate_bytes(result.binary, %{})
    assert Runtime.call(instance, "_start", []) == [Values.i32(7)]
  end

  test "compiled exported loop runs in the wasm runtime" do
    assert {:ok, result} =
             NibWasmCompiler.compile_source("""
             fn count_to(n: u4) -> u4 {
               let acc: u4 = 0;
               for i: u4 in 0..n {
                 acc = acc +% 1;
               }
               return acc;
             }
             """)

    assert {:ok, instance} = Runtime.instantiate_bytes(result.binary, %{})
    assert Runtime.call(instance, "count_to", [Values.i32(5)]) == [Values.i32(5)]
  end

  test "type errors come back as package errors" do
    assert {:error, error} = NibWasmCompiler.compile_source("fn main() { let x: bool = 1 +% 2; }")
    assert error.stage == "type-check"
  end
end
