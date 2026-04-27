# frozen_string_literal: true

module CodingAdventures
  module BrainfuckWasmCompiler
    PackageResult = Struct.new(
      :source,
      :filename,
      :ast,
      :raw_ir,
      :optimized_ir,
      :module,
      :validated_module,
      :binary,
      :wasm_path,
      keyword_init: true
    )

    class PackageError < StandardError
      attr_reader :stage, :cause

      def initialize(stage, message, cause = nil)
        @stage = stage
        @cause = cause
        @detail_message = message
        super(message)
      end

      def to_s
        "[#{stage}] #{@detail_message}"
      end
    end

    class Compiler
      def initialize(filename: "program.bf", build_config: nil)
        @filename = filename
        @build_config = build_config
      end

      def compile_source(source, filename: @filename, build_config: @build_config)
        config = build_config || CodingAdventures::BrainfuckIrCompiler::BuildConfig.release_config
        signatures = [
          CodingAdventures::IrToWasmCompiler::FunctionSignature.new(
            label: "_start",
            param_count: 0,
            export_name: "_start"
          )
        ]

        ast = CodingAdventures::Brainfuck::Parser.parse(source)
        ir_result = CodingAdventures::BrainfuckIrCompiler.compile(ast, filename, config)

        lowering_errors = CodingAdventures::IrToWasmValidator.validate(ir_result.program, signatures)
        unless lowering_errors.empty?
          raise PackageError.new("validate-ir", lowering_errors.first.message)
        end

        wasm_module = CodingAdventures::IrToWasmCompiler.compile(ir_result.program, signatures)
        validated_module = CodingAdventures::WasmValidator.validate(wasm_module)
        binary = CodingAdventures::WasmModuleEncoder.encode_module(wasm_module)

        PackageResult.new(
          source: source,
          filename: filename,
          ast: ast,
          raw_ir: ir_result.program,
          optimized_ir: ir_result.program,
          module: wasm_module,
          validated_module: validated_module,
          binary: binary
        )
      rescue PackageError
        raise
      rescue StandardError => error
        raise self.class.wrap_stage(infer_stage(error), error)
      end

      def write_wasm_file(source, output_path, filename: @filename, build_config: @build_config)
        result = compile_source(source, filename:, build_config:)
        File.binwrite(output_path, result.binary)
        result.wasm_path = output_path
        result
      rescue PackageError
        raise
      rescue StandardError => error
        raise self.class.wrap_stage("write", error)
      end

      def self.wrap_stage(stage, error)
        return error if error.is_a?(PackageError)

        PackageError.new(stage, error.message, error)
      end

      private

      def infer_stage(error)
        case error
        when CodingAdventures::IrToWasmCompiler::WasmLoweringError
          "lower"
        when CodingAdventures::WasmValidator::ValidationError
          "validate-wasm"
        when CodingAdventures::WasmModuleEncoder::WasmEncodeError
          "encode"
        else
          "parse"
        end
      end
    end

    module_function

    def compile_source(source, filename: "program.bf", build_config: nil)
      Compiler.new(filename:, build_config:).compile_source(source, filename:, build_config:)
    end

    def pack_source(source, filename: "program.bf", build_config: nil)
      compile_source(source, filename:, build_config:)
    end

    def write_wasm_file(source, output_path, filename: "program.bf", build_config: nil)
      Compiler.new(filename:, build_config:).write_wasm_file(source, output_path, filename:, build_config:)
    end
  end
end
