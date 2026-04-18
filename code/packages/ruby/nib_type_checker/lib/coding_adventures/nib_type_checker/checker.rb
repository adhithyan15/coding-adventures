# frozen_string_literal: true

module CodingAdventures
  module NibTypeChecker
    TypedAst = Data.define(:root, :types) do
      def type_of(node)
        types[node.object_id]
      end
    end

    class Checker
      include Types

      def check(ast)
        @errors = []
        @types = {}
        scope = ScopeChain.new

        collect_program(ast, scope)
        check_program(ast, scope)

        CodingAdventures::TypeCheckerProtocol::TypeCheckResult.new(
          typed_ast: TypedAst.new(root: ast, types: @types.dup),
          errors: @errors.dup,
          ok: @errors.empty?
        )
      end

      private

      def collect_program(ast, scope)
        child_nodes(ast).each do |top_decl|
          decl = unwrap_top_decl(top_decl)
          next if decl.nil?

          case decl.rule_name
          when "const_decl"
            collect_const_or_static(decl, scope, is_const: true)
          when "static_decl"
            collect_const_or_static(decl, scope, is_const: false)
          when "fn_decl"
            collect_function_signature(decl, scope)
          end
        end
      end

      def check_program(ast, scope)
        child_nodes(ast).each do |top_decl|
          decl = unwrap_top_decl(top_decl)
          next unless decl&.rule_name == "fn_decl"

          check_function_body(decl, scope)
        end
      end

      def collect_const_or_static(node, scope, is_const:)
        name = first_name_token(node)
        type = type_node(node)
        return if name.nil? || type.nil?

        nib_type = resolve_type(type)
        return if nib_type.nil?

        scope.define_global(
          name.value,
          SymbolRecord.new(
            name: name.value,
            nib_type: nib_type,
            is_const: is_const,
            is_static: !is_const
          )
        )
      end

      def collect_function_signature(node, scope)
        name = first_name_token(node)
        return if name.nil?

        params = extract_params(node)
        return_type = child_nodes(node).filter_map { |child| resolve_type(child) if child.rule_name == "type" }.last

        scope.define_global(
          name.value,
          SymbolRecord.new(
            name: name.value,
            is_fn: true,
            fn_params: params,
            fn_return_type: return_type || VOID,
            nib_type: return_type || VOID
          )
        )
      end

      def check_function_body(node, outer_scope)
        function_name = first_name_token(node)&.value
        function_symbol = function_name && outer_scope.lookup(function_name)
        return if function_symbol.nil?

        block = child_nodes(node).find { |child| child.rule_name == "block" }
        return if block.nil?

        outer_scope.push
        function_symbol.fn_params.each do |param_name, param_type|
          outer_scope.define_local(
            param_name,
            SymbolRecord.new(name: param_name, nib_type: param_type)
          )
        end

        check_block(block, outer_scope, function_symbol.fn_return_type || VOID)
      ensure
        outer_scope.pop
      end

      def check_block(block, scope, return_type)
        child_nodes(block).each do |stmt|
          check_stmt(stmt, scope, return_type)
        end
      end

      def check_stmt(stmt, scope, return_type)
        inner = stmt.rule_name == "stmt" ? child_nodes(stmt).first : stmt
        return if inner.nil?

        case inner.rule_name
        when "let_stmt"
          name = first_name_token(inner)
          declared = resolve_type(type_node(inner))
          expr = first_rule(inner, "expr")
          return if name.nil? || declared.nil? || expr.nil?

          actual = check_expr(expr, scope)
          error("let `#{name.value}` expects #{declared}, got #{actual}", expr) unless Types.compatible?(declared, actual)
          scope.define_local(name.value, SymbolRecord.new(name: name.value, nib_type: declared))
        when "assign_stmt"
          name = first_name_token(inner)
          expr = first_rule(inner, "expr")
          return if name.nil? || expr.nil?

          symbol = scope.lookup(name.value)
          if symbol.nil?
            error("unknown variable `#{name.value}`", name)
            return
          end

          actual = check_expr(expr, scope)
          error("assignment to `#{name.value}` expects #{symbol.nib_type}, got #{actual}", expr) unless Types.compatible?(symbol.nib_type, actual)
        when "return_stmt"
          expr = first_rule(inner, "expr")
          return if expr.nil?

          actual = check_expr(expr, scope)
          error("return expects #{return_type}, got #{actual}", expr) unless Types.compatible?(return_type, actual)
        when "for_stmt"
          check_for_stmt(inner, scope, return_type)
        when "expr_stmt"
          expr = first_rule(inner, "expr")
          check_expr(expr, scope) unless expr.nil?
        end
      end

      def check_for_stmt(node, scope, return_type)
        name = first_name_token(node)
        declared = resolve_type(type_node(node))
        exprs = child_nodes(node).select { |child| child.rule_name == "expr" }
        block = child_nodes(node).find { |child| child.rule_name == "block" }
        return if name.nil? || declared.nil? || exprs.length < 2 || block.nil?

        lower_type = check_expr(exprs[0], scope)
        upper_type = check_expr(exprs[1], scope)

        unless numericish?(lower_type) && numericish?(upper_type)
          error("for loop bounds must be numeric", node)
        end

        scope.push
        scope.define_local(name.value, SymbolRecord.new(name: name.value, nib_type: declared))
        check_block(block, scope, return_type)
      ensure
        scope.pop if block
      end

      def check_expr(node, scope)
        return nil if node.nil?

        result =
          case node.rule_name
          when "expr", "primary"
            first_expr_child = expression_children(node).first || child_nodes(node).first
            if first_expr_child.nil?
              infer_primary(node, scope)
            else
              check_expr(first_expr_child, scope)
            end
          when "add_expr"
            check_add_expr(node, scope)
          when "call_expr"
            check_call_expr(node, scope)
          when "or_expr", "and_expr", "eq_expr", "cmp_expr", "bitwise_expr", "unary_expr"
            first_expr_child = expression_children(node).first || child_nodes(node).first
            check_expr(first_expr_child, scope)
          else
            first_expr_child = expression_children(node).first || child_nodes(node).first
            first_expr_child ? check_expr(first_expr_child, scope) : infer_primary(node, scope)
          end

        @types[node.object_id] = result unless result.nil?
        result
      end

      def check_add_expr(node, scope)
        operands = expression_children(node)
        return check_expr(operands.first, scope) if operands.length < 2

        left = check_expr(operands[0], scope)
        right = check_expr(operands[1], scope)

        return right if left == LITERAL && Types.numeric?(right)
        return left if right == LITERAL && Types.numeric?(left)
        return LITERAL if left == LITERAL && right == LITERAL
        return left if left == right && Types.numeric?(left)

        error("binary expression type mismatch: #{left} vs #{right}", node)
        nil
      end

      def check_call_expr(node, scope)
        name = first_name_token(node)
        return nil if name.nil?

        symbol = scope.lookup(name.value)
        if symbol.nil? || !symbol.is_fn
          error("unknown function `#{name.value}`", name)
          return nil
        end

        args = []
        arg_list = child_nodes(node).find { |child| child.rule_name == "arg_list" }
        if arg_list
          args = child_nodes(arg_list).select { |child| child.rule_name == "expr" }
        end

        if args.length != symbol.fn_params.length
          error("function `#{name.value}` expects #{symbol.fn_params.length} args, got #{args.length}", node)
          return symbol.fn_return_type
        end

        symbol.fn_params.zip(args).each do |(param_name, param_type), arg|
          actual = check_expr(arg, scope)
          next if Types.compatible?(param_type, actual)

          error("argument `#{param_name}` expects #{param_type}, got #{actual}", arg)
        end

        symbol.fn_return_type
      end

      def infer_primary(node, scope)
        tokens = tokens_in(node)
        token = tokens.first
        return nil if token.nil?

        case token_type(token)
        when "INT_LIT", "HEX_LIT" then LITERAL
        when "KEYWORD"
          return BOOL if %w[true false].include?(token.value)

          nil
        when "NAME"
          symbol = scope.lookup(token.value)
          if symbol.nil?
            error("unknown name `#{token.value}`", token)
            nil
          else
            symbol.nib_type
          end
        else
          nil
        end
      end

      def resolve_type(node)
        return nil if node.nil?

        token = tokens_in(node).first
        token && Types.parse_type_name(token.value)
      end

      def extract_params(node)
        param_list = child_nodes(node).find { |child| child.rule_name == "param_list" }
        return [] if param_list.nil?

        child_nodes(param_list).filter_map do |child|
          next unless child.rule_name == "param"

          name = first_name_token(child)
          type = resolve_type(type_node(child))
          [name.value, type] unless name.nil? || type.nil?
        end
      end

      def child_nodes(node)
        node.children.select { |child| child.is_a?(CodingAdventures::Parser::ASTNode) }
      end

      def expression_children(node)
        child_nodes(node).select { |child| expression_rule?(child.rule_name) }
      end

      def expression_rule?(name)
        %w[expr or_expr and_expr eq_expr cmp_expr add_expr bitwise_expr unary_expr primary call_expr].include?(name)
      end

      def unwrap_top_decl(node)
        child_nodes(node).first
      end

      def first_name_token(node)
        tokens_in(node).find { |token| token_type(token) == "NAME" }
      end

      def first_rule(node, rule_name)
        child_nodes(node).find { |child| child.rule_name == rule_name }
      end

      def type_node(node)
        child_nodes(node).find { |child| child.rule_name == "type" }
      end

      def tokens_in(node)
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

      def numericish?(type)
        Types.numeric?(type) || type == LITERAL
      end

      def error(message, subject)
        line, column = locate(subject)
        @errors << CodingAdventures::TypeCheckerProtocol::TypeErrorDiagnostic.new(
          message: message,
          line: line,
          column: column
        )
      end

      def locate(subject)
        token = first_token(subject)
        return [1, 1] if token.nil?

        [token.line, token.column]
      end

      def first_token(subject)
        if subject.is_a?(CodingAdventures::Parser::ASTNode)
          subject.children.each do |child|
            found = first_token(child)
            return found unless found.nil?
          end
          nil
        else
          subject
        end
      end
    end

    def self.check(ast)
      Checker.new.check(ast)
    end

    def self.check_source(source)
      ast = CodingAdventures::NibParser.parse_nib(source)
      check(ast)
    rescue StandardError => e
      CodingAdventures::TypeCheckerProtocol::TypeCheckResult.new(
        typed_ast: TypedAst.new(root: nil, types: {}),
        errors: [
          CodingAdventures::TypeCheckerProtocol::TypeErrorDiagnostic.new(
            message: e.message,
            line: 1,
            column: 1
          )
        ],
        ok: false
      )
    end
  end
end
