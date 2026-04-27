# frozen_string_literal: true

require_relative "test_helper"

class TestBrainfuckWasmCompiler < Minitest::Test
  def run_binary(binary, stdin_text = "")
    output = []
    offset = 0
    host = WR::WasiHost.new(
      stdin: lambda { |count|
        return "".b if offset >= stdin_text.bytesize

        chunk = stdin_text.byteslice(offset, count) || "".b
        offset += chunk.bytesize
        chunk
      },
      stdout: ->(text) { output << text }
    )

    runtime = WR::Runtime.new(host)
    [runtime.load_and_run(binary, "_start", []), output]
  end

  def test_compile_source_returns_pipeline_artifacts
    result = BWC.compile_source("+.")

    assert_equal "program.bf", result.filename
    assert_operator result.raw_ir.instructions.length, :>, 0
    assert_operator result.optimized_ir.instructions.length, :>, 0
    assert_operator result.binary.bytesize, :>, 0
    assert_includes result.module.exports.map(&:name), "_start"
  end

  def test_pack_source_aliases_compile_source
    compiled = BWC.compile_source("+.")
    packed = BWC.pack_source("+.")

    assert_equal compiled.binary, packed.binary
  end

  def test_write_wasm_file_writes_binary_to_disk
    Dir.mktmpdir do |dir|
      path = File.join(dir, "program.wasm")
      result = BWC.write_wasm_file("+.", path)

      assert_equal result.binary, File.binread(path)
      assert_equal path, result.wasm_path
    end
  end

  def test_runs_compiled_output_programs_in_runtime
    result = BWC.compile_source("+" * 65 + ".")
    execution_result, output = run_binary(result.binary)

    assert_equal [0], execution_result
    assert_equal ["A"], output
  end

  def test_runs_compiled_input_programs_in_runtime
    result = BWC.compile_source(",.")
    execution_result, output = run_binary(result.binary, "Z")

    assert_equal [0], execution_result
    assert_equal ["Z"], output
  end

  def test_runs_compiled_cat_programs_in_runtime
    result = BWC.compile_source(",[.,]")
    execution_result, output = run_binary(result.binary, "Hi")

    assert_equal [0], execution_result
    assert_equal ["H", "i"], output
  end

  def test_honors_custom_filename
    compiler = BWC::Compiler.new(filename: "hello.bf")
    result = compiler.compile_source("+")

    assert_equal "hello.bf", result.filename
  end

  def test_wraps_parse_errors_with_stage_information
    error = assert_raises(BWC::PackageError) do
      BWC.compile_source("[")
    end

    assert_equal "parse", error.stage
    assert_match(/\[parse\]/, error.to_s)
  end

  def test_wraps_write_errors_with_stage_information
    compiler = BWC::Compiler.new

    Dir.mktmpdir do |dir|
      error = assert_raises(BWC::PackageError) do
        compiler.write_wasm_file("+", dir)
      end

      assert_equal "write", error.stage
      refute_nil error.cause
    end
  end
end
