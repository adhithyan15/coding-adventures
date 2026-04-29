# frozen_string_literal: true

require "coding_adventures_interpreter_ir"

module CodingAdventures
  module DartmouthBasicIrCompiler
    class CompileError < StandardError; end

    CompileResult = Data.define(:module, :var_names)

    class Compiler
      IR = CodingAdventures::InterpreterIr
      OPS = { "+" => "add", "-" => "sub", "*" => "mul", "/" => "div" }.freeze
      CMPS = { "=" => "cmp_eq", "<>" => "cmp_ne", "<" => "cmp_lt", "<=" => "cmp_le", ">" => "cmp_gt", ">=" => "cmp_ge" }.freeze

      def self.compile_source(source, module_name: "dartmouth-basic")
        new(module_name: module_name).compile_lines(source.lines)
      end

      def initialize(module_name:)
        @module_name = module_name
        @instrs = []
        @tmp = 0
        @for_stack = []
        @var_names = {}
      end

      def compile_lines(lines)
        @instrs << IR::IIRInstr.new("label", nil, ["_start"], "void")
        parsed = lines.filter_map { |line| parse_line(line) }
        parsed.each { |line_no, stmt| compile_line(line_no, stmt) }
        @instrs << IR::IIRInstr.new("ret_void", nil, [], "void")
        fn = IR::IIRFunction.new(
          name: "main",
          return_type: "void",
          instructions: @instrs,
          register_count: 384,
          type_status: IR::FunctionTypeStatus::PARTIALLY_TYPED
        )
        mod = IR::IIRModule.new(name: @module_name, language: "dartmouth-basic", functions: [fn], entry_point: "main")
        CompileResult.new(mod, @var_names.keys.sort)
      end

      private

      def parse_line(line)
        stripped = line.strip
        return nil if stripped.empty?

        match = stripped.match(/\A(\d+)\s*(.*)\z/)
        raise CompileError, "expected line number: #{line.inspect}" unless match

        [match[1].to_i, match[2].strip]
      end

      def compile_line(line_no, stmt)
        @instrs << IR::IIRInstr.new("label", nil, ["_line_#{line_no}"], "void")
        upper = stmt.upcase
        case upper
        when /\AREM\b/, ""
          nil
        when /\A(?:LET\s+)?([A-Z][A-Z0-9]?)\s*=\s*(.+)\z/
          name = Regexp.last_match(1)
          value = compile_expr(Regexp.last_match(2))
          @var_names[name] = true
          @instrs << IR::IIRInstr.new("move", name, [value], "u64")
        when /\APRINT(?:\s+(.*))?\z/
          compile_print(Regexp.last_match(1))
        when /\AGOTO\s+(\d+)\z/
          @instrs << IR::IIRInstr.new("jmp", nil, ["_line_#{Regexp.last_match(1)}"], "void")
        when /\AIF\s+(.+?)\s*(<=|>=|<>|=|<|>)\s*(.+?)\s+THEN\s+(\d+)\z/
          left = compile_expr(Regexp.last_match(1))
          cmp = Regexp.last_match(2)
          right = compile_expr(Regexp.last_match(3))
          dest = fresh("cmp")
          @instrs << IR::IIRInstr.new(CMPS.fetch(cmp), dest, [left, right], "bool")
          @instrs << IR::IIRInstr.new("jmp_if_true", nil, [dest, "_line_#{Regexp.last_match(4)}"], "void")
        when /\AFOR\s+([A-Z][A-Z0-9]?)\s*=\s*(.+?)\s+TO\s+(.+?)(?:\s+STEP\s+(.+))?\z/
          compile_for(Regexp.last_match(1), Regexp.last_match(2), Regexp.last_match(3), Regexp.last_match(4))
        when /\ANEXT(?:\s+([A-Z][A-Z0-9]?))?\z/
          compile_next(Regexp.last_match(1))
        when /\A(?:END|STOP)\z/
          @instrs << IR::IIRInstr.new("ret_void", nil, [], "void")
        else
          raise CompileError, "unsupported BASIC statement: #{stmt.inspect}"
        end
      end

      def compile_print(arg)
        if arg.nil? || arg.empty?
          emit_print_const("")
        elsif arg.start_with?("\"") && arg.end_with?("\"")
          emit_print_const(arg[1...-1])
        else
          value = compile_expr(arg)
          @instrs << IR::IIRInstr.new("call_builtin", nil, ["__basic_print", value], "void")
        end
      end

      def emit_print_const(value)
        dest = fresh("s")
        @instrs << IR::IIRInstr.new("const", dest, [value], "str")
        @instrs << IR::IIRInstr.new("call_builtin", nil, ["__basic_print", dest], "void")
      end

      def compile_for(name, start_expr, limit_expr, step_expr)
        start = compile_expr(start_expr)
        limit = compile_expr(limit_expr)
        step = compile_expr(step_expr || "1")
        check = "__for#{@for_stack.length}_check"
        done = "__for#{@for_stack.length}_done"
        @var_names[name] = true
        @instrs << IR::IIRInstr.new("move", name, [start], "u64")
        @instrs << IR::IIRInstr.new("label", nil, [check], "void")
        cmp = fresh("forcmp")
        @instrs << IR::IIRInstr.new("cmp_gt", cmp, [name, limit], "bool")
        @instrs << IR::IIRInstr.new("jmp_if_true", nil, [cmp, done], "void")
        @for_stack << [name, step, check, done]
      end

      def compile_next(name)
        raise CompileError, "NEXT without matching FOR" if @for_stack.empty?

        expected, step, check, done = @for_stack.pop
        raise CompileError, "NEXT #{name} does not match FOR #{expected}" if name && name != expected

        @instrs << IR::IIRInstr.new("add", expected, [expected, step], "u64")
        @instrs << IR::IIRInstr.new("jmp", nil, [check], "void")
        @instrs << IR::IIRInstr.new("label", nil, [done], "void")
      end

      def compile_expr(text)
        tokens = text.scan(/[A-Z][A-Z0-9]?|\d+|[()+\-*\/]/i)
        @expr_tokens = tokens
        @expr_pos = 0
        parse_add
      end

      def parse_add
        left = parse_mul
        while peek_expr == "+" || peek_expr == "-"
          op = advance_expr
          right = parse_mul
          dest = fresh("v")
          @instrs << IR::IIRInstr.new(OPS.fetch(op), dest, [left, right], "u64")
          left = dest
        end
        left
      end

      def parse_mul
        left = parse_primary
        while peek_expr == "*" || peek_expr == "/"
          op = advance_expr
          right = parse_primary
          dest = fresh("v")
          @instrs << IR::IIRInstr.new(OPS.fetch(op), dest, [left, right], "u64")
          left = dest
        end
        left
      end

      def parse_primary
        tok = advance_expr
        if tok == "("
          expr = parse_add
          raise CompileError, "expected ')'" unless advance_expr == ")"

          expr
        elsif tok =~ /\A\d+\z/
          dest = fresh("n")
          @instrs << IR::IIRInstr.new("const", dest, [tok.to_i], "u64")
          dest
        elsif tok =~ /\A[A-Z][A-Z0-9]?\z/i
          name = tok.upcase
          @var_names[name] = true
          name
        else
          raise CompileError, "unexpected expression token #{tok.inspect}"
        end
      end

      def peek_expr
        @expr_tokens[@expr_pos]
      end

      def advance_expr
        tok = peek_expr
        @expr_pos += 1
        tok
      end

      def fresh(prefix)
        @tmp += 1
        "__#{prefix}#{@tmp}"
      end
    end
  end
end
