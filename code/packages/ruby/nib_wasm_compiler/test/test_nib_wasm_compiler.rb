# frozen_string_literal: true

require_relative "test_helper"

class NibWasmCompilerTest < Minitest::Test
  def runtime
    CodingAdventures::WasmRuntime::Runtime.new
  end

  def test_compile_source_returns_binary
    result = CodingAdventures::NibWasmCompiler.compile_source("fn answer() -> u4 { return 7; }")

    assert result.binary.bytesize.positive?
    assert result.raw_ir.instructions.any?
  end

  def test_pack_source_aliases_compile_source
    compiled = CodingAdventures::NibWasmCompiler.compile_source("fn answer() -> u4 { return 7; }")
    packed = CodingAdventures::NibWasmCompiler.pack_source("fn answer() -> u4 { return 7; }")

    assert_equal compiled.binary, packed.binary
  end

  def test_write_wasm_file_persists_bytes
    Dir.mktmpdir do |dir|
      path = File.join(dir, "program.wasm")
      result = CodingAdventures::NibWasmCompiler.write_wasm_file("fn answer() -> u4 { return 7; }", path)

      assert_equal result.binary, File.binread(path)
      assert_equal path, result.wasm_path
    end
  end

  def test_compiled_program_runs_through_start
    result = CodingAdventures::NibWasmCompiler.compile_source(<<~NIB)
      fn add(a: u4, b: u4) -> u4 { return a +% b; }
      fn main() -> u4 { return add(3, 4); }
    NIB

    assert_equal [7], runtime.load_and_run(result.binary, "_start", [])
  end

  def test_compiled_loop_runs_through_export
    result = CodingAdventures::NibWasmCompiler.compile_source(<<~NIB)
      fn count_to(n: u4) -> u4 {
        let acc: u4 = 0;
        for i: u4 in 0..n {
          acc = acc +% 1;
        }
        return acc;
      }
    NIB

    assert_equal [5], runtime.load_and_run(result.binary, "count_to", [5])
  end

  def test_type_errors_raise_package_error
    error = assert_raises(CodingAdventures::NibWasmCompiler::PackageError) do
      CodingAdventures::NibWasmCompiler.compile_source("fn main() { let x: bool = 1 +% 2; }")
    end

    assert_equal "type-check", error.stage
  end
end
