# frozen_string_literal: true

require "fileutils"

module CodingAdventures
  module NibWasmCompiler
    PackageResult = Data.define(
      :source,
      :ast,
      :typed_ast,
      :raw_ir,
      :optimized_ir,
      :module,
      :validated_module,
      :binary,
      :wasm_path
    ) do
      def initialize(source:, ast:, typed_ast:, raw_ir:, optimized_ir:, module:, validated_module:, binary:, wasm_path: nil)
        super
      end
    end

    class PackageError < StandardError
      attr_reader :stage, :cause

      def initialize(stage, message, cause = nil)
        @stage = stage
        @cause = cause
        super("[#{stage}] #{message}")
      end
    end

    class Compiler
      def compile_source(source)
        ast = CodingAdventures::NibParser.parse_nib(source)
        type_result = CodingAdventures::NibTypeChecker.check(ast)
        raise PackageError.new("type-check", type_result.errors.map(&:message).join("\n")) unless type_result.ok

        ir_result = CodingAdventures::NibIrCompiler.compile_nib(type_result.typed_ast)
        signatures = extract_signatures(type_result.typed_ast.root)
        lowering_errors = CodingAdventures::IrToWasmValidator.validate(ir_result.program, signatures)
        unless lowering_errors.empty?
          raise PackageError.new("validate-ir", lowering_errors.first.message)
        end

        wasm_module = CodingAdventures::IrToWasmCompiler.compile(ir_result.program, signatures)
        validated_module = CodingAdventures::WasmValidator.validate(wasm_module)
        binary = CodingAdventures::WasmModuleEncoder.encode_module(wasm_module)

        PackageResult.new(
          source: source,
          ast: ast,
          typed_ast: type_result.typed_ast,
          raw_ir: ir_result.program,
          optimized_ir: ir_result.program,
          module: wasm_module,
          validated_module: validated_module,
          binary: binary
        )
      rescue PackageError
        raise
      rescue StandardError => e
        raise PackageError.new("compile", e.message, e)
      end

      def write_wasm_file(source, output_path)
        result = compile_source(source)
        FileUtils.mkdir_p(File.dirname(output_path))
        File.binwrite(output_path, result.binary)
        PackageResult.new(**result.to_h.merge(wasm_path: output_path))
      rescue PackageError
        raise
      rescue StandardError => e
        raise PackageError.new("write", e.message, e)
      end

      private

      def extract_signatures(root)
        signatures = [
          CodingAdventures::IrToWasmCompiler::FunctionSignature.new(
            label: "_start",
            param_count: 0,
            export_name: "_start"
          )
        ]

        child_nodes(root).each do |node|
          decl = node.rule_name == "top_decl" ? child_nodes(node).first : node
          next unless decl&.rule_name == "fn_decl"

          name = first_name(decl)
          param_list = child_nodes(decl).find { |child| child.rule_name == "param_list" }
          param_count = param_list.nil? ? 0 : child_nodes(param_list).count { |child| child.rule_name == "param" }
          signatures << CodingAdventures::IrToWasmCompiler::FunctionSignature.new(
            label: "_fn_#{name}",
            param_count: param_count,
            export_name: name
          )
        end

        signatures
      end

      def child_nodes(node)
        node.children.select { |child| child.is_a?(CodingAdventures::Parser::ASTNode) }
      end

      def first_name(node)
        node.children.each do |child|
          if child.is_a?(CodingAdventures::Parser::ASTNode)
            name = first_name(child)
            return name unless name.nil?
          elsif child.respond_to?(:type_name) && child.type_name == "NAME"
            return child.value
          end
        end
        nil
      end
    end

    def self.compile_source(source)
      Compiler.new.compile_source(source)
    end

    def self.pack_source(source)
      compile_source(source)
    end

    def self.write_wasm_file(source, output_path)
      Compiler.new.write_wasm_file(source, output_path)
    end
  end
end
