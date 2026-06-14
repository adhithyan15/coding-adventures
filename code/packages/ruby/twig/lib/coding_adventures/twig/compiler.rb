# frozen_string_literal: true

require "coding_adventures_interpreter_ir"
require_relative "parser"

module CodingAdventures
  module Twig
    class CompileError < StandardError; end

    class Compiler
      IR = CodingAdventures::InterpreterIr
      BUILTINS = %w[+ - * / = < > cons car cdr null? pair? number? print].freeze

      def self.compile_source(source, module_name: "twig")
        new(module_name: module_name).compile(Parser.parse(source))
      end

      def initialize(module_name:)
        @module_name = module_name
        @functions = []
        @fn_globals = []
        @value_globals = []
        @tmp = 0
        @label = 0
      end

      def compile(forms)
        forms.each do |form|
          next unless define_form?(form)

          if function_define?(form)
            @fn_globals << form[1][0].name
          else
            @value_globals << form[1].name
          end
        end

        main = []
        last = nil
        forms.each do |form|
          if function_define?(form)
            compile_function_define(form)
          elsif define_form?(form)
            value = compile_expr(form[2], main, locals: [])
            key = string_const(form[1].name, main)
            main << IR::IIRInstr.new("call_builtin", nil, ["global_set", key, value], "void")
            last = nil
          else
            last = compile_expr(form, main, locals: [])
          end
        end

        if last
          main << IR::IIRInstr.new("ret", nil, [last], "any")
        else
          main << IR::IIRInstr.new("const", "__nil", [nil], "any")
          main << IR::IIRInstr.new("ret", nil, ["__nil"], "any")
        end
        @functions << IR::IIRFunction.new(
          name: "main",
          return_type: "any",
          instructions: main,
          register_count: 64,
          type_status: IR::FunctionTypeStatus::UNTYPED
        )

        IR::IIRModule.new(name: @module_name, language: "twig", functions: @functions, entry_point: "main")
      end

      private

      def define_form?(form)
        form.is_a?(Array) && sym?(form[0], "define")
      end

      def function_define?(form)
        define_form?(form) && form[1].is_a?(Array)
      end

      def compile_function_define(form)
        signature = form[1]
        name = signature[0].name
        params = signature[1..].map(&:name)
        body = []
        last = nil
        form[2..].each { |expr| last = compile_expr(expr, body, locals: params) }
        body << IR::IIRInstr.new("ret", nil, [last], "any")
        @functions << IR::IIRFunction.new(
          name: name,
          params: params.map { |p| [p, "any"] },
          return_type: "any",
          instructions: body,
          register_count: 64,
          type_status: IR::FunctionTypeStatus::UNTYPED
        )
      end

      def compile_expr(expr, body, locals:)
        case expr
        when Integer, TrueClass, FalseClass, NilClass
          dest = fresh("lit")
          body << IR::IIRInstr.new("const", dest, [expr], "any")
          dest
        when SymbolRef
          compile_symbol(expr.name, body, locals)
        when Array
          compile_list(expr, body, locals)
        else
          raise CompileError, "unknown Twig expression #{expr.inspect}"
        end
      end

      def compile_symbol(name, body, locals)
        return name if locals.include?(name)

        if @value_globals.include?(name)
          key = string_const(name, body)
          dest = fresh("g")
          body << IR::IIRInstr.new("call_builtin", dest, ["global_get", key], "any")
          dest
        else
          raise CompileError, "unbound name #{name.inspect}"
        end
      end

      def compile_list(expr, body, locals)
        head = expr[0]
        if sym?(head, "if")
          compile_if(expr, body, locals)
        elsif sym?(head, "begin")
          last = nil
          expr[1..].each { |e| last = compile_expr(e, body, locals: locals) }
          last
        elsif sym?(head, "let")
          compile_let(expr, body, locals)
        elsif head.is_a?(SymbolRef)
          args = expr[1..].map { |arg| compile_expr(arg, body, locals: locals) }
          dest = fresh("call")
          if BUILTINS.include?(head.name)
            body << IR::IIRInstr.new("call_builtin", dest, [head.name, *args], "any")
          elsif @fn_globals.include?(head.name)
            body << IR::IIRInstr.new("call", dest, [head.name, *args], "any")
          else
            raise CompileError, "unknown function #{head.name.inspect}"
          end
          dest
        else
          raise CompileError, "invalid application #{expr.inspect}"
        end
      end

      def compile_if(expr, body, locals)
        cond = compile_expr(expr[1], body, locals: locals)
        else_label = fresh_label("else")
        end_label = fresh_label("endif")
        result = fresh("if")
        body << IR::IIRInstr.new("jmp_if_false", nil, [cond, else_label], "void")
        then_v = compile_expr(expr[2], body, locals: locals)
        body << IR::IIRInstr.new("call_builtin", result, ["_move", then_v], "any")
        body << IR::IIRInstr.new("jmp", nil, [end_label], "void")
        body << IR::IIRInstr.new("label", nil, [else_label], "void")
        else_v = compile_expr(expr[3], body, locals: locals)
        body << IR::IIRInstr.new("call_builtin", result, ["_move", else_v], "any")
        body << IR::IIRInstr.new("label", nil, [end_label], "void")
        result
      end

      def compile_let(expr, body, locals)
        added = []
        expr[1].each do |binding|
          name = binding[0].name
          value = compile_expr(binding[1], body, locals: locals)
          body << IR::IIRInstr.new("call_builtin", name, ["_move", value], "any")
          added << name
        end
        last = nil
        expr[2..].each { |e| last = compile_expr(e, body, locals: locals + added) }
        last
      end

      def sym?(expr, name)
        expr.is_a?(SymbolRef) && expr.name == name
      end

      def string_const(value, body)
        dest = fresh("s")
        body << IR::IIRInstr.new("const", dest, [value], "str")
        dest
      end

      def fresh(prefix)
        @tmp += 1
        "__#{prefix}#{@tmp}"
      end

      def fresh_label(prefix)
        @label += 1
        "__#{prefix}#{@label}"
      end
    end
  end
end
