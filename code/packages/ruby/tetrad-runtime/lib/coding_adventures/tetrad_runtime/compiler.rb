# frozen_string_literal: true

require "coding_adventures_interpreter_ir"
require_relative "ast"
require_relative "parser"

module CodingAdventures
  module TetradRuntime
    class Compiler
      IR = CodingAdventures::InterpreterIr
      OPS = { "+" => "add", "-" => "sub", "*" => "mul", "/" => "div", "%" => "mod" }.freeze

      def self.compile_source(source, module_name: "tetrad")
        new(module_name: module_name).compile(Parser.parse(source))
      end

      def initialize(module_name:)
        @module_name = module_name
        @tmp = 0
      end

      def compile(program)
        functions = []
        main_body = []
        program.forms.each do |form|
          if form.is_a?(FunctionDef)
            functions << compile_function(form)
          else
            compile_statement(form, main_body)
          end
        end
        unless functions.any? { |fn| fn.name == "main" }
          main_body << IR::IIRInstr.new("ret_void", nil, [], "void") unless main_body.any? { |i| i.op.start_with?("ret") }
          functions.unshift(IR::IIRFunction.new(name: "main", return_type: "void", instructions: main_body, register_count: 32))
        end
        IR::IIRModule.new(name: @module_name, language: "tetrad", functions: functions, entry_point: "main")
      end

      private

      def compile_function(fn_def)
        body = []
        fn_def.body.each { |stmt| compile_statement(stmt, body) }
        body << IR::IIRInstr.new("ret_void", nil, [], "void") unless body.any? { |i| i.op.start_with?("ret") }
        IR::IIRFunction.new(
          name: fn_def.name,
          params: fn_def.params.map { |p| [p, "u8"] },
          return_type: "u8",
          instructions: body,
          register_count: 32,
          type_status: IR::FunctionTypeStatus::FULLY_TYPED
        )
      end

      def compile_statement(stmt, body)
        case stmt
        when LetStmt
          value = compile_expr(stmt.expr, body)
          body << IR::IIRInstr.new("tetrad.move", stmt.name, [value], "u8")
        when ReturnStmt
          value = compile_expr(stmt.expr, body)
          body << IR::IIRInstr.new("ret", nil, [value], "u8")
        when ExprStmt
          compile_expr(stmt.expr, body)
        end
      end

      def compile_expr(expr, body)
        case expr
        when NumberLit
          dest = fresh("n")
          body << IR::IIRInstr.new("const", dest, [expr.value & 0xFF], "u8")
          dest
        when VarRef
          expr.name
        when Binary
          left = compile_expr(expr.left, body)
          right = compile_expr(expr.right, body)
          dest = fresh("v")
          body << IR::IIRInstr.new(OPS.fetch(expr.op), dest, [left, right], "u8")
          dest
        when Call
          args = expr.args.map { |arg| compile_expr(arg, body) }
          dest = fresh("call")
          body << IR::IIRInstr.new("call", dest, [expr.name, *args], "u8")
          dest
        else
          raise ArgumentError, "unknown Tetrad expression #{expr.inspect}"
        end
      end

      def fresh(prefix)
        @tmp += 1
        "__#{prefix}#{@tmp}"
      end
    end
  end
end
