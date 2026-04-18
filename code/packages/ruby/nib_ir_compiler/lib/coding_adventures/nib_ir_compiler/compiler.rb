# frozen_string_literal: true

module CodingAdventures
  module NibIrCompiler
    BuildConfig = Data.define(:optimize) do
      def initialize(optimize: true)
        super
      end
    end

    CompileResult = Data.define(:program)

    class Compiler
      include CodingAdventures::CompilerIr

      def initialize
        @ids = IDGenerator.new
        @program = IrProgram.new("_start")
        @registers = {}
        @next_register = 2
        @loop_index = 0
      end

      def compile(typed_ast)
        root = typed_ast.root
        emit_label("_start")
        main = function_nodes(root).find { |node| function_name(node) == "main" }
        emit(IrOp::CALL, IrLabel.new("_fn_main")) unless main.nil?
        emit(IrOp::HALT)

        function_nodes(root).each { |node| compile_function(node) }
        CompileResult.new(program: @program)
      end

      private

      def compile_function(node)
        @registers = {}
        @next_register = 2
        emit_label("_fn_#{function_name(node)}")

        params(node).each_with_index do |(name, _type), index|
          @registers[name] = index + 2
          @next_register = index + 3
        end

        block = child_nodes(node).find { |child| child.rule_name == "block" }
        compile_block(block) unless block.nil?
        emit(IrOp::RET)
      end

      def compile_block(block)
        child_nodes(block).each { |stmt| compile_stmt(stmt) }
      end

      def compile_stmt(stmt)
        inner = stmt.rule_name == "stmt" ? child_nodes(stmt).first : stmt
        return if inner.nil?

        case inner.rule_name
        when "let_stmt"
          name = first_name(inner)
          expr = first_rule(inner, "expr")
          return if name.nil? || expr.nil?

          register = allocate_register(name)
          emit_expr_into(expr, register)
        when "assign_stmt"
          name = first_name(inner)
          expr = first_rule(inner, "expr")
          register = name && @registers[name]
          return if register.nil? || expr.nil?

          emit_expr_into(expr, register)
        when "return_stmt"
          expr = first_rule(inner, "expr")
          emit_expr_into(expr, 1) unless expr.nil?
          emit(IrOp::RET)
        when "expr_stmt"
          expr = first_rule(inner, "expr")
          emit_expr_into(expr, 1) unless expr.nil?
        when "for_stmt"
          compile_for(inner)
        end
      end

      def compile_for(node)
        name = first_name(node)
        exprs = child_nodes(node).select { |child| child.rule_name == "expr" }
        block = child_nodes(node).find { |child| child.rule_name == "block" }
        return if name.nil? || exprs.length < 2 || block.nil?

        loop_register = allocate_register(name)
        emit_expr_into(exprs[0], loop_register)

        end_register = @next_register
        @next_register += 1
        emit_expr_into(exprs[1], end_register)

        cond_register = @next_register
        @next_register += 1

        start_label = "loop_#{@loop_index}_start"
        end_label = "loop_#{@loop_index}_end"
        @loop_index += 1

        emit_label(start_label)
        emit(IrOp::CMP_LT, IrRegister.new(cond_register), IrRegister.new(loop_register), IrRegister.new(end_register))
        emit(IrOp::BRANCH_Z, IrRegister.new(cond_register), IrLabel.new(end_label))
        compile_block(block)
        emit(IrOp::ADD_IMM, IrRegister.new(loop_register), IrRegister.new(loop_register), IrImmediate.new(1))
        emit(IrOp::JUMP, IrLabel.new(start_label))
        emit_label(end_label)
      end

      def emit_expr_into(node, register_index)
        return if node.nil?

        if node.rule_name == "call_expr"
          compile_call(node, register_index)
          return
        end

        if node.rule_name == "add_expr"
          compile_add(node, register_index)
          return
        end

        inner_nodes = child_nodes(node)
        if expression_rule?(node.rule_name) && inner_nodes.length == 1
          emit_expr_into(inner_nodes.first, register_index)
          return
        end

        token = direct_token(node)
        if token
          case token_type(token)
          when "INT_LIT"
            emit(IrOp::LOAD_IMM, IrRegister.new(register_index), IrImmediate.new(token.value.to_i))
            return
          when "HEX_LIT"
            emit(IrOp::LOAD_IMM, IrRegister.new(register_index), IrImmediate.new(token.value.delete_prefix("0x").to_i(16)))
            return
          when "KEYWORD"
            if token.value == "true"
              emit(IrOp::LOAD_IMM, IrRegister.new(register_index), IrImmediate.new(1))
              return
            end

            if token.value == "false"
              emit(IrOp::LOAD_IMM, IrRegister.new(register_index), IrImmediate.new(0))
              return
            end
          when "NAME"
            if @registers.key?(token.value)
              source = @registers.fetch(token.value)
              emit(IrOp::ADD_IMM, IrRegister.new(register_index), IrRegister.new(source), IrImmediate.new(0))
              return
            end
          end
        end

        emit_expr_into(inner_nodes.first, register_index) unless inner_nodes.empty?
      end

      def compile_call(node, register_index)
        name = first_name(node)
        arg_list = child_nodes(node).find { |child| child.rule_name == "arg_list" }
        args = arg_list.nil? ? [] : child_nodes(arg_list).select { |child| child.rule_name == "expr" }

        args.each_with_index do |arg, index|
          emit_expr_into(arg, index + 2)
        end

        emit(IrOp::CALL, IrLabel.new("_fn_#{name}"))
        emit(IrOp::ADD_IMM, IrRegister.new(register_index), IrRegister.new(1), IrImmediate.new(0)) unless register_index == 1
      end

      def compile_add(node, register_index)
        operands = child_nodes(node).select { |child| %w[expr primary call_expr add_expr].include?(child.rule_name) || expression_rule?(child.rule_name) }
        if operands.length < 2
          emit_expr_into(operands.first, register_index)
          return
        end

        left = operands[0]
        right = operands[1]
        emit_expr_into(left, register_index)

        if (value = literal_value(right))
          op = operator_tokens(node).include?("MINUS") ? IrOp::ADD_IMM : IrOp::ADD_IMM
          immediate = operator_tokens(node).include?("MINUS") ? -value : value
          emit(op, IrRegister.new(register_index), IrRegister.new(register_index), IrImmediate.new(immediate))
          return
        end

        scratch = @next_register
        @next_register += 1
        emit_expr_into(right, scratch)

        if operator_tokens(node).include?("MINUS")
          emit(IrOp::SUB, IrRegister.new(register_index), IrRegister.new(register_index), IrRegister.new(scratch))
        else
          emit(IrOp::ADD, IrRegister.new(register_index), IrRegister.new(register_index), IrRegister.new(scratch))
        end
      end

      def emit(opcode, *operands)
        @program.add_instruction(IrInstruction.new(opcode, operands, @ids.next))
      end

      def emit_label(name)
        @program.add_instruction(IrInstruction.new(IrOp::LABEL, [IrLabel.new(name)], -1))
      end

      def allocate_register(name)
        @registers[name] ||= begin
          reg = @next_register
          @next_register += 1
          reg
        end
      end

      def function_nodes(root)
        child_nodes(root).filter_map do |node|
          decl = node.rule_name == "top_decl" ? child_nodes(node).first : node
          decl if decl&.rule_name == "fn_decl"
        end
      end

      def function_name(node)
        first_name(node)
      end

      def params(node)
        list = child_nodes(node).find { |child| child.rule_name == "param_list" }
        return [] if list.nil?

        child_nodes(list).filter_map do |param|
          next unless param.rule_name == "param"

          [first_name(param), first_type_name(param)]
        end
      end

      def child_nodes(node)
        node.children.select { |child| child.is_a?(CodingAdventures::Parser::ASTNode) }
      end

      def first_rule(node, rule_name)
        child_nodes(node).find { |child| child.rule_name == rule_name }
      end

      def first_name(node)
        tokens_in(node).find { |token| token_type(token) == "NAME" }&.value
      end

      def first_type_name(node)
        type_node = child_nodes(node).find { |child| child.rule_name == "type" }
        tokens_in(type_node).first&.value
      end

      def lookup_name(node)
        tokens_in(node).find { |token| token_type(token) == "NAME" }&.value
      end

      def literal_value(node)
        tokens_in(node).each do |token|
          case token_type(token)
          when "INT_LIT" then return token.value.to_i
          when "HEX_LIT" then return token.value.delete_prefix("0x").to_i(16)
          when "true" then return 1
          when "false" then return 0
          end
        end
        nil
      end

      def operator_tokens(node)
        tokens_in(node).map { |token| token_type(token) }
      end

      def direct_token(node)
        tokens = tokens_in(node)
        return nil unless child_nodes(node).empty? && tokens.length == 1

        tokens.first
      end

      def tokens_in(node)
        return [] if node.nil?

        node.children.flat_map do |child|
          if child.is_a?(CodingAdventures::Parser::ASTNode)
            tokens_in(child)
          else
            [child]
          end
        end
      end

      def token_type(token)
        token.respond_to?(:type_name) ? token.type_name : token.type.to_s
      end

      def expression_rule?(name)
        %w[expr or_expr and_expr eq_expr cmp_expr add_expr bitwise_expr unary_expr primary call_expr].include?(name)
      end
    end

    def self.release_config
      BuildConfig.new(optimize: true)
    end

    def self.compile_nib(typed_ast, _config = release_config)
      Compiler.new.compile(typed_ast)
    end
  end
end
