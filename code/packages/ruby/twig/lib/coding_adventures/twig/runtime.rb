# frozen_string_literal: true

require "stringio"
require "coding_adventures_codegen_core"
require "coding_adventures_jit_core"
require "coding_adventures_vm_core"
require_relative "compiler"

module CodingAdventures
  module Twig
    class RuntimeError < StandardError; end

    class Runtime
      attr_reader :last_module, :last_vm, :globals

      def compile(source)
        Compiler.compile_source(source)
      end

      def run(source, jit: false)
        execute_module(compile(source), jit: jit)
      end

      def execute_module(mod, jit: false)
        @last_module = mod
        @last_vm = CodingAdventures::VmCore::VMCore.new(profiler_enabled: true)
        @globals = {}
        @stdout = StringIO.new
        register_builtins(@last_vm)
        value = if jit
          CodingAdventures::JitCore::JITCore.new(@last_vm).execute_with_jit(mod)
        else
          @last_vm.execute(mod)
        end
        [@stdout.string, value]
      end

      def emit(source, target:)
        CodingAdventures::CodegenCore::BackendRegistry.default.compile(compile(source), target: target)
      end

      private

      def register_builtins(vm)
        vm.register_builtin("+") { |args| args.map { |a| integer(a) }.sum }
        vm.register_builtin("-") { |args| args.length == 1 ? -integer(args[0]) : integer(args[0]) - args[1..].map { |a| integer(a) }.sum }
        vm.register_builtin("*") { |args| args.map { |a| integer(a) }.reduce(1, :*) }
        vm.register_builtin("/") { |args| integer(args[0]) / integer(args[1]) }
        vm.register_builtin("=") { |args| args.each_cons(2).all? { |a, b| a == b } }
        vm.register_builtin("<") { |args| integer(args[0]) < integer(args[1]) }
        vm.register_builtin(">") { |args| integer(args[0]) > integer(args[1]) }
        vm.register_builtin("cons") { |args| [:cons, args[0], args[1]] }
        vm.register_builtin("car") { |args| pair(args[0])[1] }
        vm.register_builtin("cdr") { |args| pair(args[0])[2] }
        vm.register_builtin("null?") { |args| args[0].nil? }
        vm.register_builtin("pair?") { |args| args[0].is_a?(Array) && args[0][0] == :cons }
        vm.register_builtin("number?") { |args| args[0].is_a?(Integer) }
        vm.register_builtin("print") do |args|
          @stdout << format_value(args[0]) << "\n"
          nil
        end
        vm.register_builtin("global_get") do |args|
          key = args[0].to_s
          raise RuntimeError, "unbound global #{key.inspect}" unless @globals.key?(key)

          @globals[key]
        end
        vm.register_builtin("global_set") do |args|
          @globals[args[0].to_s] = args[1]
          nil
        end
        vm.register_builtin("_move") { |args| args[0] }
      end

      def integer(value)
        raise RuntimeError, "expected number, got #{value.inspect}" unless value.is_a?(Integer)

        value
      end

      def pair(value)
        raise RuntimeError, "expected pair, got #{value.inspect}" unless value.is_a?(Array) && value[0] == :cons

        value
      end

      def format_value(value)
        if value.nil?
          "nil"
        elsif value == true
          "#t"
        elsif value == false
          "#f"
        elsif value.is_a?(Array) && value[0] == :cons
          "(#{format_value(value[1])} . #{format_value(value[2])})"
        else
          value.to_s
        end
      end
    end
  end
end
