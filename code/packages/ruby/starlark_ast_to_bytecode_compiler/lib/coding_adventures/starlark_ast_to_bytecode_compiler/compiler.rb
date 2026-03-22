# frozen_string_literal: true

# ==========================================================================
# Starlark Compiler -- Compiles Starlark AST to Bytecode
# ==========================================================================
#
# This is the heart of the Starlark compilation pipeline. It takes an
# Abstract Syntax Tree (produced by starlark_parser) and emits bytecode
# instructions that the virtual machine can execute.
#
# The compilation pipeline:
#
#   Source Code (String)
#       |
#       v
#   StarlarkLexer.tokenize(source)
#       |
#       v
#   StarlarkParser.parse(source)
#       |
#       v
#   AST (Parser::ASTNode tree)
#       |
#       v
#   THIS FILE: Compiler.compile_starlark(source)
#       |
#       v
#   CodeObject (instructions + constants + names)
#
# ==========================================================================
# How the Compiler Works
# ==========================================================================
#
# The compiler uses the GenericCompiler framework from bytecode_compiler.
# Rather than subclassing, it registers handler methods for each grammar
# rule. When the GenericCompiler encounters an ASTNode with rule_name
# "assign_stmt", it calls the handler registered for "assign_stmt".
#
# For example, compiling "x = 1 + 2":
#
#   1. Visit assign_stmt node
#   2. Visit the RHS expression first (1 + 2)
#      a. Visit arith node
#      b. Visit left atom (1) -> emit LOAD_CONST 0
#      c. Visit right atom (2) -> emit LOAD_CONST 1
#      d. See "+" operator -> emit ADD
#   3. See "=" operator and LHS target "x"
#   4. Emit STORE_NAME 0 (adds "x" to names)
#
# Result:
#   instructions = [LOAD_CONST 0, LOAD_CONST 1, ADD, STORE_NAME 0, HALT]
#   constants    = [1, 2]
#   names        = ["x"]
#
# ==========================================================================
# AST Structure
# ==========================================================================
#
# The Starlark parser produces Parser::ASTNode trees where:
#   - node.rule_name is the grammar rule ("file", "assign_stmt", etc.)
#   - node.children is an array of ASTNode and Lexer::Token objects
#
# Children are in source order. For "x = 1 + 2", assign_stmt's children:
#   [expression_list("x"), assign_op("="), expression_list("1 + 2")]
#
# Tokens have .type (a string like "INT", "NAME", "KEYWORD") and .value
# (the actual text from source code).
# ==========================================================================

require "coding_adventures_bytecode_compiler"
require "coding_adventures_starlark_parser"

module CodingAdventures
  module StarlarkAstToBytecodeCompiler
    # The Compiler class provides static methods to compile Starlark source
    # code or ASTs into bytecode CodeObjects.
    #
    # Usage:
    #   code = Compiler.compile_starlark("x = 1 + 2\n")
    #   # => CodeObject with instructions, constants, names
    #
    #   compiler = Compiler.create_starlark_compiler
    #   code = compiler.compile(ast, halt_opcode: Op::HALT)
    class Compiler
      # ================================================================
      # Binary and Comparison Operator Mappings
      # ================================================================
      #
      # These maps connect operator token values to their opcodes.
      # When the compiler sees a "+" token in an arith node, it looks
      # up "+" to find Op::ADD.

      BINARY_OP_MAP = {
        "+" => Op::ADD,
        "-" => Op::SUB,
        "*" => Op::MUL,
        "/" => Op::DIV,
        "//" => Op::FLOOR_DIV,
        "%" => Op::MOD,
        "**" => Op::POWER,
        "<<" => Op::LSHIFT,
        ">>" => Op::RSHIFT,
        "&" => Op::BIT_AND,
        "|" => Op::BIT_OR,
        "^" => Op::BIT_XOR
      }.freeze

      COMPARE_OP_MAP = {
        "==" => Op::CMP_EQ,
        "!=" => Op::CMP_NE,
        "<" => Op::CMP_LT,
        ">" => Op::CMP_GT,
        "<=" => Op::CMP_LE,
        ">=" => Op::CMP_GE,
        "in" => Op::CMP_IN,
        "not in" => Op::CMP_NOT_IN
      }.freeze

      # Augmented assignment operators map to their binary operation.
      # "x += 1" is equivalent to "x = x + 1".
      AUGMENTED_ASSIGN_OP_MAP = {
        "+=" => Op::ADD,
        "-=" => Op::SUB,
        "*=" => Op::MUL,
        "/=" => Op::DIV,
        "//=" => Op::FLOOR_DIV,
        "%=" => Op::MOD,
        "**=" => Op::POWER,
        "<<=" => Op::LSHIFT,
        ">>=" => Op::RSHIFT,
        "&=" => Op::BIT_AND,
        "|=" => Op::BIT_OR,
        "^=" => Op::BIT_XOR
      }.freeze

      # ================================================================
      # Public API
      # ================================================================

      # Compile Starlark source code directly to a CodeObject.
      #
      # This is the one-shot entry point that handles the full pipeline:
      # lex -> parse -> compile. Returns a CodeObject ready for the VM.
      #
      # @param source [String] Starlark source code
      # @return [VirtualMachine::CodeObject]
      def self.compile_starlark(source)
        ast = CodingAdventures::StarlarkParser.parse(source)
        compiler = create_starlark_compiler
        compiler.compile(ast, halt_opcode: Op::HALT)
      end

      # Compile a pre-parsed AST into a CodeObject.
      #
      # Useful when you already have an AST and want to skip re-parsing.
      #
      # @param ast [Parser::ASTNode] the root AST node
      # @return [VirtualMachine::CodeObject]
      def self.compile_ast(ast)
        compiler = create_starlark_compiler
        compiler.compile(ast, halt_opcode: Op::HALT)
      end

      # Create a GenericCompiler configured with all Starlark rule handlers.
      #
      # You can use this if you want lower-level control over compilation
      # (e.g., compiling fragments without the HALT instruction).
      #
      # @return [BytecodeCompiler::GenericCompiler]
      def self.create_starlark_compiler
        compiler = CodingAdventures::BytecodeCompiler::GenericCompiler.new
        register_all_rules(compiler)
        compiler
      end

      # ================================================================
      # Rule Registration
      # ================================================================
      #
      # Each grammar rule gets a handler that knows how to compile nodes
      # of that type. The handler receives (compiler, node) where
      # compiler is the GenericCompiler instance and node is an ASTNode.

      # @api private
      def self.register_all_rules(compiler)
        # -- Top-level and statement containers --
        compiler.register_rule("file", method(:compile_file))
        compiler.register_rule("statement", method(:compile_passthrough))
        compiler.register_rule("simple_stmt", method(:compile_passthrough))
        compiler.register_rule("small_stmt", method(:compile_passthrough))

        # -- Simple statements --
        compiler.register_rule("assign_stmt", method(:compile_assign_stmt))
        compiler.register_rule("return_stmt", method(:compile_return_stmt))
        compiler.register_rule("break_stmt", method(:compile_break_stmt))
        compiler.register_rule("continue_stmt", method(:compile_continue_stmt))
        compiler.register_rule("pass_stmt", method(:compile_pass_stmt))
        compiler.register_rule("load_stmt", method(:compile_load_stmt))

        # -- Compound statements --
        compiler.register_rule("if_stmt", method(:compile_if_stmt))
        compiler.register_rule("for_stmt", method(:compile_for_stmt))
        compiler.register_rule("def_stmt", method(:compile_def_stmt))
        compiler.register_rule("suite", method(:compile_suite))

        # -- Expressions --
        compiler.register_rule("expression", method(:compile_expression))
        compiler.register_rule("expression_list", method(:compile_expression_list))
        compiler.register_rule("or_expr", method(:compile_or_expr))
        compiler.register_rule("and_expr", method(:compile_and_expr))
        compiler.register_rule("not_expr", method(:compile_not_expr))
        compiler.register_rule("comparison", method(:compile_comparison))

        # -- Binary chain operators (each uses the same pattern) --
        compiler.register_rule("bitwise_or", method(:compile_binary_chain_multi_op))
        compiler.register_rule("bitwise_xor", method(:compile_binary_chain_multi_op))
        compiler.register_rule("bitwise_and", method(:compile_binary_chain_multi_op))
        compiler.register_rule("shift", method(:compile_binary_chain_multi_op))
        compiler.register_rule("arith", method(:compile_binary_chain_multi_op))
        compiler.register_rule("term", method(:compile_binary_chain_multi_op))

        # -- Unary and power --
        compiler.register_rule("factor", method(:compile_factor))
        compiler.register_rule("power", method(:compile_power))

        # -- Primary and atom --
        compiler.register_rule("primary", method(:compile_primary))
        compiler.register_rule("atom", method(:compile_atom))

        # -- Collection literals --
        compiler.register_rule("list_expr", method(:compile_list_expr))
        compiler.register_rule("list_body", method(:compile_list_body))
        compiler.register_rule("dict_expr", method(:compile_dict_expr))
        compiler.register_rule("dict_body", method(:compile_dict_body))
        compiler.register_rule("dict_entry", method(:compile_dict_entry))
        compiler.register_rule("paren_expr", method(:compile_paren_expr))
        compiler.register_rule("paren_body", method(:compile_paren_body))

        # -- Lambda --
        compiler.register_rule("lambda_expr", method(:compile_lambda_expr))
      end

      private_class_method :register_all_rules

      # ================================================================
      # AST Traversal Helpers
      # ================================================================
      #
      # The parser produces heterogeneous children (ASTNode or Token).
      # These helpers extract typed children for pattern matching.

      # Extract all ASTNode children from a node.
      # @param node [Parser::ASTNode]
      # @return [Array<Parser::ASTNode>]
      def self.extract_nodes(node)
        node.children.select { |c| c.is_a?(CodingAdventures::Parser::ASTNode) }
      end

      # Extract all Token children from a node.
      # @param node [Parser::ASTNode]
      # @return [Array<Lexer::Token>]
      def self.extract_tokens(node)
        node.children.select { |c| c.is_a?(CodingAdventures::Lexer::Token) }
      end

      # Check if any token child has a specific value.
      # Used to detect operators like "=", "+", "if", etc.
      # @param node [Parser::ASTNode]
      # @param value [String]
      # @return [Boolean]
      def self.has_token?(node, value)
        node.children.any? { |c| c.is_a?(CodingAdventures::Lexer::Token) && c.value == value }
      end

      # Get the effective type name for a token.
      #
      # Grammar-driven lexers produce string types like "INT", "FLOAT",
      # "STRING", "NAME", "KEYWORD". The base lexer uses TokenType constants
      # which are also strings. This method normalizes both.
      #
      # @param tok [Lexer::Token]
      # @return [String]
      def self.token_type_name(tok)
        tok.type.to_s
      end

      # Extract a simple variable name from an expression AST.
      #
      # For a simple name reference like "x", this traverses the expression
      # tree down through precedence-encoding wrapper nodes to find the
      # NAME token at the bottom.
      #
      # AST path: expression_list -> expression -> or_expr -> and_expr ->
      #   not_expr -> comparison -> bitwise_or -> ... -> atom -> NAME token
      #
      # Returns nil if the expression is not a simple name.
      # @param node [Parser::ASTNode]
      # @return [String, nil]
      def self.extract_simple_name(node)
        current = node
        while current
          # Check if this node directly contains a NAME token
          current.children.each do |child|
            if child.is_a?(CodingAdventures::Lexer::Token)
              return child.value if token_type_name(child) == "NAME"
            end
          end
          # If there's exactly one child node, descend into it
          child_nodes = extract_nodes(current)
          if child_nodes.length == 1
            current = child_nodes[0]
          else
            break
          end
        end
        nil
      end

      # Parse a string literal value from the lexer.
      #
      # The Starlark grammar-driven lexer may or may not strip quotes
      # depending on the string type. This method handles both cases:
      #   - If the value starts with a quote or prefix (r, b), strip them
      #   - If already bare content, return as-is
      #
      # Starlark strings can be:
      #   - Single-quoted:  'hello'
      #   - Double-quoted:  "hello"
      #   - Triple-quoted:  '''hello''' or """hello"""
      #   - Raw prefixed:   r"hello\n" (backslashes are literal)
      #   - Byte prefixed:  b"hello"
      #
      # @param s [String]
      # @return [String]
      def self.parse_string_literal(s)
        return s if s.empty?

        first_char = s[0]
        has_prefix = %w[r R b B].include?(first_char)
        has_quotes = ['"', "'"].include?(first_char)

        # If the lexer already stripped quotes, return as-is.
        return s unless has_prefix || has_quotes

        # Strip optional prefix (r, b, rb, br, etc.)
        raw = false
        while s.length > 0 && %w[r R b B].include?(s[0])
          raw = true if s[0] == "r" || s[0] == "R"
          s = s[1..]
        end

        # Strip triple or single quotes
        if s.start_with?('"""') && s.end_with?('"""')
          s = s[3...-3]
        elsif s.start_with?("'''") && s.end_with?("'''")
          s = s[3...-3]
        elsif s.length >= 2 && (s[0] == '"' || s[0] == "'")
          s = s[1...-1]
        end

        return s if raw

        # Process escape sequences
        result = +""
        i = 0
        while i < s.length
          if s[i] == "\\" && i + 1 < s.length
            case s[i + 1]
            when "n" then result << "\n"
            when "t" then result << "\t"
            when "r" then result << "\r"
            when "\\" then result << "\\"
            when "'" then result << "'"
            when '"' then result << '"'
            when "0" then result << "\0"
            else
              result << "\\"
              result << s[i + 1]
            end
            i += 2
          else
            result << s[i]
            i += 1
          end
        end
        result
      end

      # ================================================================
      # Statement Compilation
      # ================================================================

      # Compile the top-level "file" rule.
      # A file is a sequence of statements separated by newlines.
      # Grammar: file = { NEWLINE | statement } ;
      def self.compile_file(compiler, node)
        node.children.each do |child|
          compiler.compile_node(child) if child.is_a?(CodingAdventures::Parser::ASTNode)
        end
      end

      # Compile a pass-through node that just contains child nodes.
      # Used for statement, simple_stmt, small_stmt which are just
      # containers dispatching to their children.
      def self.compile_passthrough(compiler, node)
        node.children.each do |child|
          compiler.compile_node(child) if child.is_a?(CodingAdventures::Parser::ASTNode)
        end
      end

      # Compile an assignment statement, augmented assignment, or expression statement.
      #
      # Grammar: assign_stmt = expression_list [ ( assign_op | augmented_assign_op ) expression_list ] ;
      #
      # Three cases:
      #   1. x = expr          (simple assignment)
      #   2. x += expr         (augmented assignment)
      #   3. expr              (expression statement -- result discarded with POP)
      #
      # For case 1: compile RHS, then emit STORE_NAME/STORE_LOCAL for LHS.
      # For case 2: load current value, compile RHS, apply op, store back.
      # For case 3: compile expression, emit POP to discard result.
      def self.compile_assign_stmt(compiler, node)
        nodes = extract_nodes(node)
        tokens = extract_tokens(node)

        # Case 3: bare expression statement (no operator)
        if nodes.length == 1 && tokens.empty?
          compiler.compile_node(nodes[0])
          compiler.emit(Op::POP)
          return
        end

        if nodes.length >= 2
          # Find the operator token (inside assign_op or augmented_assign_op node)
          op_token = nil
          node.children.each do |child|
            if child.is_a?(CodingAdventures::Parser::ASTNode)
              if child.rule_name == "assign_op" || child.rule_name == "augmented_assign_op"
                op_tokens = extract_tokens(child)
                op_token = op_tokens[0] unless op_tokens.empty?
              end
            end
          end

          if op_token && op_token.value == "="
            # Case 1: simple assignment (x = expr)
            compiler.compile_node(nodes.last)
            compile_store_target(compiler, nodes[0])
            return
          end

          if op_token
            # Case 2: augmented assignment (x += expr)
            bin_op = AUGMENTED_ASSIGN_OP_MAP[op_token.value]
            if bin_op
              name = extract_simple_name(nodes[0])
              if name
                name_idx = compiler.add_name(name)
                if compiler.scope
                  compiler.emit(Op::LOAD_LOCAL, name_idx)
                else
                  compiler.emit(Op::LOAD_NAME, name_idx)
                end
              end
              compiler.compile_node(nodes.last)
              compiler.emit(bin_op)
              compile_store_target(compiler, nodes[0])
              return
            end
          end
        end

        # Fallback: single node with tokens -- treat as expression statement
        if nodes.length == 1
          compiler.compile_node(nodes[0])
          compiler.emit(Op::POP)
        end
      end

      # Emit a STORE instruction for an assignment target.
      #
      # The target can be a simple name (STORE_NAME/STORE_LOCAL),
      # a dotted name (STORE_ATTR), or a subscript (STORE_SUBSCRIPT).
      # For now we handle simple names.
      def self.compile_store_target(compiler, target)
        name = extract_simple_name(target)
        if name
          name_idx = compiler.add_name(name)
          if compiler.scope
            compiler.emit(Op::STORE_LOCAL, name_idx)
          else
            compiler.emit(Op::STORE_NAME, name_idx)
          end
        end
      end

      # Compile a return statement.
      # Grammar: return_stmt = "return" [ expression ] ;
      #
      # If there's no expression, return None. In Starlark, every function
      # implicitly returns None if it falls through.
      def self.compile_return_stmt(compiler, node)
        nodes = extract_nodes(node)
        if nodes.length > 0
          compiler.compile_node(nodes[0])
        else
          compiler.emit(Op::LOAD_NONE)
        end
        compiler.emit(Op::RETURN_VALUE)
      end

      # Compile break -- emit a single BREAK instruction.
      def self.compile_break_stmt(compiler, _node)
        compiler.emit(Op::BREAK)
      end

      # Compile continue -- emit a single CONTINUE instruction.
      def self.compile_continue_stmt(compiler, _node)
        compiler.emit(Op::CONTINUE)
      end

      # Compile pass -- a no-op. Emit nothing.
      def self.compile_pass_stmt(_compiler, _node)
        # pass is intentionally empty
      end

      # Compile a load() statement.
      #
      # Grammar: load_stmt = "load" LPAREN STRING { COMMA load_arg } [ COMMA ] RPAREN ;
      #
      # Starlark's load() imports symbols from another module:
      #   load("module.star", "symbol1", alias = "symbol2")
      #
      # Compilation:
      #   1. LOAD_MODULE (push the module path as a constant)
      #   2. For each imported symbol:
      #      a. DUP the module object
      #      b. IMPORT_FROM (extract the symbol)
      #      c. STORE_NAME (bind to local/alias name)
      #   3. POP the module object
      def self.compile_load_stmt(compiler, node)
        tokens = extract_tokens(node)
        load_arg_nodes = node.children.select do |child|
          child.is_a?(CodingAdventures::Parser::ASTNode) && child.rule_name == "load_arg"
        end

        # First STRING token is the module path
        module_path = nil
        tokens.each do |tok|
          if token_type_name(tok) == "STRING"
            module_path = parse_string_literal(tok.value)
            break
          end
        end

        # Emit LOAD_MODULE
        module_idx = compiler.add_constant(module_path)
        compiler.emit(Op::LOAD_MODULE, module_idx)

        # Process each load_arg
        load_arg_nodes.each do |arg|
          arg_tokens = extract_tokens(arg)
          compiler.emit(Op::DUP)

          if arg_tokens.length >= 3
            # Alias form: local_name = "remote_name"
            local_name = arg_tokens[0].value
            remote_name = parse_string_literal(arg_tokens[2].value)
            remote_idx = compiler.add_name(remote_name)
            compiler.emit(Op::IMPORT_FROM, remote_idx)
            local_idx = compiler.add_name(local_name)
            compiler.emit(Op::STORE_NAME, local_idx)
          elsif arg_tokens.length >= 1
            # Simple form: "symbol_name"
            if token_type_name(arg_tokens[0]) == "STRING"
              symbol_name = parse_string_literal(arg_tokens[0].value)
              sym_idx = compiler.add_name(symbol_name)
              compiler.emit(Op::IMPORT_FROM, sym_idx)
              compiler.emit(Op::STORE_NAME, sym_idx)
            end
          end
        end

        # Pop the module object
        compiler.emit(Op::POP)
      end

      # ================================================================
      # Compound Statement Compilation
      # ================================================================

      # Compile if/elif/else chains.
      #
      # Grammar: if_stmt = "if" expression COLON suite
      #                     { "elif" expression COLON suite }
      #                     [ "else" COLON suite ] ;
      #
      # Bytecode pattern for if/elif/else:
      #
      #   compile condition1
      #   JUMP_IF_FALSE -> elif1 (or else, or end)
      #   compile body1
      #   JUMP -> end
      #   elif1:
      #   compile condition2
      #   JUMP_IF_FALSE -> else (or end)
      #   compile body2
      #   JUMP -> end
      #   else:
      #   compile else_body
      #   end:
      def self.compile_if_stmt(compiler, node)
        # Collect branches: pairs of (condition, body). condition is nil for else.
        branches = []
        current_condition = nil
        expecting_condition = false
        expecting_body = false

        node.children.each do |child|
          if child.is_a?(CodingAdventures::Lexer::Token)
            if child.value == "if" || child.value == "elif"
              expecting_condition = true
              next
            end
            if child.value == "else"
              current_condition = nil
              expecting_body = true
              next
            end
            if child.value == ":"
              expecting_condition = false if expecting_condition
              expecting_body = true
              next
            end
          end

          if child.is_a?(CodingAdventures::Parser::ASTNode)
            if expecting_condition || (current_condition.nil? && !expecting_body && child.rule_name != "suite")
              current_condition = child
              expecting_condition = false
              next
            end
            if expecting_body || child.rule_name == "suite"
              branches << { condition: current_condition, body: child }
              current_condition = nil
              expecting_body = false
              next
            end
          end
        end

        # Compile the branches with forward jumps
        end_jumps = []
        branches.each_with_index do |br, i|
          if br[:condition]
            compiler.compile_node(br[:condition])
            false_jump = compiler.emit_jump(Op::JUMP_IF_FALSE)
            compiler.compile_node(br[:body])
            if i < branches.length - 1
              end_jump = compiler.emit_jump(Op::JUMP)
              end_jumps << end_jump
            end
            compiler.patch_jump(false_jump)
          else
            # else branch -- no condition
            compiler.compile_node(br[:body])
          end
        end

        # Patch all end jumps to point here
        end_jumps.each { |j| compiler.patch_jump(j) }
      end

      # Compile a for loop.
      #
      # Grammar: for_stmt = "for" loop_vars "in" expression COLON suite ;
      #
      # Bytecode pattern:
      #
      #   compile iterable_expression
      #   GET_ITER
      #   loop_top:
      #   FOR_ITER -> loop_exit
      #   STORE_NAME loop_var
      #   compile loop_body
      #   JUMP -> loop_top
      #   loop_exit:
      def self.compile_for_stmt(compiler, node)
        loop_vars_node = nil
        expr_node = nil
        suite_node = nil
        expecting_vars = false
        expecting_expr = false
        expecting_suite = false

        node.children.each do |child|
          if child.is_a?(CodingAdventures::Lexer::Token)
            if child.value == "for"
              expecting_vars = true
              next
            end
            if child.value == "in"
              expecting_vars = false
              expecting_expr = true
              next
            end
            if child.value == ":"
              expecting_expr = false
              expecting_suite = true
              next
            end
          end

          if child.is_a?(CodingAdventures::Parser::ASTNode)
            if expecting_vars && loop_vars_node.nil?
              loop_vars_node = child
              next
            end
            if expecting_expr && expr_node.nil?
              expr_node = child
              next
            end
            if expecting_suite && suite_node.nil?
              suite_node = child
              next
            end
          end
        end

        return if expr_node.nil? || suite_node.nil?

        # Compile the iterable expression
        compiler.compile_node(expr_node)
        compiler.emit(Op::GET_ITER)

        # Loop header
        loop_top = compiler.current_offset
        exit_jump = compiler.emit_jump(Op::FOR_ITER)

        # Store loop variable(s)
        if loop_vars_node
          var_tokens = extract_tokens(loop_vars_node)
          name_tokens = var_tokens.select { |t| token_type_name(t) == "NAME" }

          if name_tokens.length > 1
            # Multiple loop variables: unpack
            compiler.emit(Op::UNPACK_SEQUENCE, name_tokens.length)
            name_tokens.each do |tok|
              name_idx = compiler.add_name(tok.value)
              if compiler.scope
                compiler.emit(Op::STORE_LOCAL, name_idx)
              else
                compiler.emit(Op::STORE_NAME, name_idx)
              end
            end
          elsif name_tokens.length == 1
            name_idx = compiler.add_name(name_tokens[0].value)
            if compiler.scope
              compiler.emit(Op::STORE_LOCAL, name_idx)
            else
              compiler.emit(Op::STORE_NAME, name_idx)
            end
          end
        end

        # Compile loop body
        compiler.compile_node(suite_node)

        # Jump back to loop header
        compiler.emit(Op::JUMP, loop_top)

        # Patch the exit jump
        compiler.patch_jump(exit_jump)
      end

      # Compile a function definition.
      #
      # Grammar: def_stmt = "def" NAME LPAREN [ parameters ] RPAREN COLON suite ;
      #
      # Function definitions compile into TWO parts:
      #   1. The function body -> a separate CodeObject (nested compilation)
      #   2. In the outer scope:
      #      - Compile default argument values
      #      - LOAD_CONST (function info map with CodeObject, params, defaults)
      #      - MAKE_FUNCTION
      #      - STORE_NAME (bind function to its name)
      def self.compile_def_stmt(compiler, node)
        func_name = nil
        params_node = nil
        suite_node = nil

        node.children.each do |child|
          if child.is_a?(CodingAdventures::Lexer::Token)
            if token_type_name(child) == "NAME" && func_name.nil?
              func_name = child.value
            end
          end
          if child.is_a?(CodingAdventures::Parser::ASTNode)
            if child.rule_name == "parameters"
              params_node = child
            elsif child.rule_name == "suite"
              suite_node = child
            end
          end
        end

        return if suite_node.nil?

        # Extract parameter names and compile default values
        param_names = []
        default_count = 0
        if params_node
          params_node.children.each do |child|
            if child.is_a?(CodingAdventures::Parser::ASTNode) && child.rule_name == "parameter"
              param_tokens = extract_tokens(child)
              param_subnodes = extract_nodes(child)
              param_tokens.each do |pt|
                if token_type_name(pt) == "NAME"
                  param_names << pt.value
                  break
                end
              end
              # Check for default value
              if has_token?(child, "=") && param_subnodes.length > 0
                compiler.compile_node(param_subnodes[0])
                default_count += 1
              end
            end
          end
        end

        # Compile the function body into a nested CodeObject.
        # We create a fresh compiler with incremented scope depth.
        body_compiler = create_starlark_compiler
        body_compiler.enter_scope(param_names)
        body_compiler.compile_node(suite_node)
        body_compiler.emit(Op::LOAD_NONE)
        body_compiler.emit(Op::RETURN_VALUE)

        body_code = CodingAdventures::VirtualMachine::CodeObject.new(
          instructions: body_compiler.instructions,
          constants: body_compiler.constants,
          names: body_compiler.names
        )
        body_compiler.exit_scope

        # Store parameter info with the CodeObject as a hash
        func_info = {
          "code" => body_code,
          "params" => param_names,
          "default_count" => default_count
        }

        func_idx = compiler.add_constant(func_info)
        compiler.emit(Op::MAKE_FUNCTION, func_idx)

        # Bind the function to its name
        name_idx = compiler.add_name(func_name)
        if compiler.scope
          compiler.emit(Op::STORE_LOCAL, name_idx)
        else
          compiler.emit(Op::STORE_NAME, name_idx)
        end
      end

      # Compile the body of a compound statement.
      # Grammar: suite = simple_stmt | NEWLINE INDENT { statement } DEDENT ;
      def self.compile_suite(compiler, node)
        node.children.each do |child|
          compiler.compile_node(child) if child.is_a?(CodingAdventures::Parser::ASTNode)
        end
      end

      # ================================================================
      # Expression Compilation
      # ================================================================

      # Compile a top-level expression.
      #
      # Grammar: expression = lambda_expr | or_expr [ "if" or_expr "else" expression ] ;
      #
      # The "if" form is the ternary conditional:
      #   value = x if condition else y
      #
      # Bytecode for ternary:
      #   compile condition (middle expression)
      #   JUMP_IF_FALSE -> else_branch
      #   compile true_value (left expression)
      #   JUMP -> end
      #   else_branch:
      #   compile false_value (right expression)
      #   end:
      def self.compile_expression(compiler, node)
        nodes = extract_nodes(node)

        # Check for lambda
        if nodes.length > 0 && nodes[0].rule_name == "lambda_expr"
          compiler.compile_node(nodes[0])
          return
        end

        # Check for ternary: or_expr "if" or_expr "else" expression
        if nodes.length >= 3 && has_token?(node, "if") && has_token?(node, "else")
          compiler.compile_node(nodes[1]) # condition
          false_jump = compiler.emit_jump(Op::JUMP_IF_FALSE)
          compiler.compile_node(nodes[0]) # true value
          end_jump = compiler.emit_jump(Op::JUMP)
          compiler.patch_jump(false_jump)
          compiler.compile_node(nodes[2]) # false value
          compiler.patch_jump(end_jump)
          return
        end

        # Simple expression
        compiler.compile_node(nodes[0]) if nodes.length >= 1
      end

      # Compile an expression list (for tuples and multi-assignment).
      # Grammar: expression_list = expression { COMMA expression } [ COMMA ] ;
      #
      # If there's only one expression, compile it directly.
      # If there are multiple, compile each and emit BUILD_TUPLE.
      def self.compile_expression_list(compiler, node)
        nodes = extract_nodes(node)
        if nodes.length == 1
          compiler.compile_node(nodes[0])
          return
        end
        nodes.each { |n| compiler.compile_node(n) }
        compiler.emit(Op::BUILD_TUPLE, nodes.length) if nodes.length > 1
      end

      # Compile boolean OR with short-circuit evaluation.
      # Grammar: or_expr = and_expr { "or" and_expr } ;
      #
      # Short-circuit: "a or b" evaluates a; if truthy, returns a
      # without evaluating b.
      #
      # Bytecode:
      #   compile a
      #   JUMP_IF_TRUE_OR_POP -> end
      #   compile b
      #   end:
      def self.compile_or_expr(compiler, node)
        nodes = extract_nodes(node)
        if nodes.length == 1
          compiler.compile_node(nodes[0])
          return
        end

        jumps = []
        nodes.each_with_index do |n, i|
          compiler.compile_node(n)
          jumps << compiler.emit_jump(Op::JUMP_IF_TRUE_OR_POP) if i < nodes.length - 1
        end
        jumps.each { |j| compiler.patch_jump(j) }
      end

      # Compile boolean AND with short-circuit evaluation.
      # Grammar: and_expr = not_expr { "and" not_expr } ;
      #
      # Short-circuit: "a and b" evaluates a; if falsy, returns a
      # without evaluating b.
      def self.compile_and_expr(compiler, node)
        nodes = extract_nodes(node)
        if nodes.length == 1
          compiler.compile_node(nodes[0])
          return
        end

        jumps = []
        nodes.each_with_index do |n, i|
          compiler.compile_node(n)
          jumps << compiler.emit_jump(Op::JUMP_IF_FALSE_OR_POP) if i < nodes.length - 1
        end
        jumps.each { |j| compiler.patch_jump(j) }
      end

      # Compile logical NOT.
      # Grammar: not_expr = "not" not_expr | comparison ;
      def self.compile_not_expr(compiler, node)
        if has_token?(node, "not")
          nodes = extract_nodes(node)
          if nodes.length > 0
            compiler.compile_node(nodes[0])
            compiler.emit(Op::NOT)
          end
          return
        end
        nodes = extract_nodes(node)
        compiler.compile_node(nodes[0]) if nodes.length > 0
      end

      # Compile comparison operators.
      # Grammar: comparison = bitwise_or { comp_op bitwise_or } ;
      #
      # For a single comparison (a < b):
      #   compile a, compile b, CMP_LT
      def self.compile_comparison(compiler, node)
        operands = []
        operators = []

        node.children.each do |child|
          if child.is_a?(CodingAdventures::Parser::ASTNode)
            if child.rule_name == "comp_op"
              op = extract_comp_op(child)
              operators << op
            else
              operands << child
            end
          end
        end

        if operators.empty?
          compiler.compile_node(operands[0]) if operands.length > 0
          return
        end

        compiler.compile_node(operands[0])
        operators.each_with_index do |op, i|
          compiler.compile_node(operands[i + 1])
          opcode = COMPARE_OP_MAP[op]
          compiler.emit(opcode) if opcode
        end
      end

      # Extract the comparison operator string from a comp_op node.
      # Handles the special "not in" two-token operator.
      def self.extract_comp_op(node)
        tokens = extract_tokens(node)
        if tokens.length == 2 && tokens[0].value == "not" && tokens[1].value == "in"
          return "not in"
        end
        tokens.length >= 1 ? tokens[0].value : ""
      end

      # Compile a left-associative binary operator chain with
      # possibly different operators (used for arith, term, shift,
      # bitwise_or, bitwise_xor, bitwise_and).
      #
      # Grammar pattern: rule = subrule { OP subrule } ;
      # Children are interleaved: [node, token, node, token, node, ...]
      def self.compile_binary_chain_multi_op(compiler, node)
        operands = []
        operators = []

        node.children.each do |child|
          if child.is_a?(CodingAdventures::Parser::ASTNode)
            operands << child
          elsif child.is_a?(CodingAdventures::Lexer::Token)
            # Only consider tokens that map to binary operators
            operators << child.value if BINARY_OP_MAP.key?(child.value)
          end
        end

        return if operands.empty?

        compiler.compile_node(operands[0])
        operators.each_with_index do |op, i|
          if i + 1 < operands.length
            compiler.compile_node(operands[i + 1])
            opcode = BINARY_OP_MAP[op]
            compiler.emit(opcode) if opcode
          end
        end
      end

      # Compile unary prefix operators.
      # Grammar: factor = ( PLUS | MINUS | TILDE ) factor | power ;
      #
      # Unary minus (-x) emits NEGATE.
      # Unary plus (+x) is a no-op.
      # Bitwise not (~x) emits BIT_NOT.
      def self.compile_factor(compiler, node)
        nodes = extract_nodes(node)
        tokens = extract_tokens(node)

        if tokens.length > 0 && nodes.length > 0
          op = tokens[0].value
          compiler.compile_node(nodes[0])
          case op
          when "-" then compiler.emit(Op::NEGATE)
          when "~" then compiler.emit(Op::BIT_NOT)
          # "+" is a no-op
          end
          return
        end

        compiler.compile_node(nodes[0]) if nodes.length > 0
      end

      # Compile exponentiation.
      # Grammar: power = primary [ DOUBLE_STAR factor ] ;
      def self.compile_power(compiler, node)
        nodes = extract_nodes(node)
        return if nodes.empty?

        compiler.compile_node(nodes[0])
        if nodes.length > 1 && has_token?(node, "**")
          compiler.compile_node(nodes[1])
          compiler.emit(Op::POWER)
        end
      end

      # Compile a primary expression with optional suffixes.
      # Grammar: primary = atom { suffix } ;
      # suffix = DOT NAME | LBRACKET subscript RBRACKET | LPAREN [ arguments ] RPAREN ;
      def self.compile_primary(compiler, node)
        atom_node = nil
        suffix_nodes = []

        node.children.each do |child|
          if child.is_a?(CodingAdventures::Parser::ASTNode)
            if atom_node.nil? && child.rule_name != "suffix"
              atom_node = child
            else
              suffix_nodes << child
            end
          end
        end

        compiler.compile_node(atom_node) if atom_node
        suffix_nodes.each { |suffix| compile_suffix(compiler, suffix) }
      end

      # Compile a single suffix (dot, subscript, or call).
      def self.compile_suffix(compiler, node)
        tokens = extract_tokens(node)
        nodes = extract_nodes(node)

        if has_token?(node, ".")
          # Attribute access: .NAME
          tokens.each do |tok|
            if token_type_name(tok) == "NAME"
              name_idx = compiler.add_name(tok.value)
              compiler.emit(Op::LOAD_ATTR, name_idx)
              return
            end
          end
          return
        end

        if has_token?(node, "[")
          # Subscript: [expr] or [slice]
          if nodes.length > 0
            subscript_node = nodes[0]
            if subscript_node.rule_name == "subscript"
              compile_subscript(compiler, subscript_node)
            else
              compiler.compile_node(subscript_node)
              compiler.emit(Op::LOAD_SUBSCRIPT)
            end
          end
          return
        end

        if has_token?(node, "(")
          # Function call: (args)
          if nodes.length > 0
            compile_arguments(compiler, nodes[0])
          else
            compiler.emit(Op::CALL_FUNCTION, 0)
          end
        end
      end

      # Compile a subscript expression (index or slice).
      def self.compile_subscript(compiler, node)
        if has_token?(node, ":")
          # Slice syntax
          nodes = extract_nodes(node)
          slice_args = 0
          nodes.each do |n|
            compiler.compile_node(n)
            slice_args += 1
          end
          while slice_args < 2
            compiler.emit(Op::LOAD_NONE)
            slice_args += 1
          end
          compiler.emit(Op::LOAD_SLICE, slice_args)
        else
          # Simple index
          nodes = extract_nodes(node)
          compiler.compile_node(nodes[0]) if nodes.length > 0
          compiler.emit(Op::LOAD_SUBSCRIPT)
        end
      end

      # Compile function call arguments.
      # Grammar: arguments = argument { COMMA argument } [ COMMA ] ;
      def self.compile_arguments(compiler, node)
        arg_nodes = node.children.select do |child|
          child.is_a?(CodingAdventures::Parser::ASTNode) && child.rule_name == "argument"
        end

        positional_count = 0
        kw_count = 0

        arg_nodes.each do |arg|
          compile_argument(compiler, arg, positional_count, kw_count)
          if has_token?(arg, "=") && extract_tokens(arg).length >= 2 && extract_nodes(arg).length >= 1
            kw_count += 1
          else
            positional_count += 1
          end
        end

        if kw_count > 0
          compiler.emit(Op::CALL_FUNCTION_KW, positional_count + kw_count)
        else
          compiler.emit(Op::CALL_FUNCTION, positional_count)
        end
      end

      # Compile a single function call argument.
      # Grammar: argument = NAME EQUALS expression | expression ;
      def self.compile_argument(compiler, node, _positional_count, _kw_count)
        tokens = extract_tokens(node)
        nodes = extract_nodes(node)

        # Check for keyword argument: NAME = expression
        if has_token?(node, "=") && tokens.length >= 2 && nodes.length >= 1
          tokens.each do |tok|
            if token_type_name(tok) == "NAME"
              name_idx = compiler.add_constant(tok.value)
              compiler.emit(Op::LOAD_CONST, name_idx)
              break
            end
          end
          compiler.compile_node(nodes[0])
          return
        end

        # Positional argument
        compiler.compile_node(nodes[0]) if nodes.length > 0
      end

      # ================================================================
      # Atom Compilation -- Leaf values of the expression tree
      # ================================================================

      # Compile atomic expressions (literals, names, collections).
      #
      # Grammar: atom = INT | FLOAT | STRING { STRING } | NAME
      #               | "True" | "False" | "None"
      #               | list_expr | dict_expr | paren_expr ;
      #
      # This is where literal values enter the bytecode. Each literal
      # is added to the constants pool and referenced by LOAD_CONST.
      def self.compile_atom(compiler, node)
        # Check for child AST nodes first (list_expr, dict_expr, paren_expr)
        nodes = extract_nodes(node)
        if nodes.length > 0
          compiler.compile_node(nodes[0])
          return
        end

        # Process token children
        tokens = extract_tokens(node)
        return if tokens.empty?

        # Handle adjacent string concatenation: "a" "b" -> "ab"
        all_strings = tokens.all? { |t| token_type_name(t) == "STRING" }
        if all_strings && tokens.length > 1
          combined = tokens.map { |t| parse_string_literal(t.value) }.join
          idx = compiler.add_constant(combined)
          compiler.emit(Op::LOAD_CONST, idx)
          return
        end

        tok = tokens[0]
        tn = token_type_name(tok)

        case
        when tn == "INT"
          # Parse integer literal (decimal, hex, or octal)
          val = Integer(tok.value, 0)
          idx = compiler.add_constant(val)
          compiler.emit(Op::LOAD_CONST, idx)

        when tn == "FLOAT"
          val = Float(tok.value)
          idx = compiler.add_constant(val)
          compiler.emit(Op::LOAD_CONST, idx)

        when tn == "STRING"
          val = parse_string_literal(tok.value)
          idx = compiler.add_constant(val)
          compiler.emit(Op::LOAD_CONST, idx)

        when tn == "NAME"
          name_idx = compiler.add_name(tok.value)
          if compiler.scope
            compiler.emit(Op::LOAD_LOCAL, name_idx)
          else
            compiler.emit(Op::LOAD_NAME, name_idx)
          end

        when tn == "KEYWORD" && tok.value == "True"
          compiler.emit(Op::LOAD_TRUE)

        when tn == "KEYWORD" && tok.value == "False"
          compiler.emit(Op::LOAD_FALSE)

        when tn == "KEYWORD" && tok.value == "None"
          compiler.emit(Op::LOAD_NONE)

        when tn == "NUMBER"
          # Generic NUMBER from base lexer -- try int first, then float
          if tok.value.include?(".")
            val = Float(tok.value)
            idx = compiler.add_constant(val)
            compiler.emit(Op::LOAD_CONST, idx)
          else
            val = Integer(tok.value, 0)
            idx = compiler.add_constant(val)
            compiler.emit(Op::LOAD_CONST, idx)
          end
        end
      end

      # ================================================================
      # Collection Literal Compilation
      # ================================================================

      # Compile a list literal or list comprehension.
      # Grammar: list_expr = LBRACKET [ list_body ] RBRACKET ;
      def self.compile_list_expr(compiler, node)
        nodes = extract_nodes(node)
        if nodes.empty?
          compiler.emit(Op::BUILD_LIST, 0)
          return
        end
        compiler.compile_node(nodes[0])
      end

      # Compile the contents of a list literal.
      # Grammar: list_body = expression { COMMA expression } [ COMMA ] ;
      def self.compile_list_body(compiler, node)
        nodes = extract_nodes(node)

        # Check for comprehension
        nodes.each do |n|
          if n.rule_name == "comp_clause" || n.rule_name == "comp_for"
            compiler.emit(Op::BUILD_LIST, 0)
            return
          end
        end

        # Regular list literal
        count = 0
        nodes.each do |n|
          compiler.compile_node(n)
          count += 1
        end
        compiler.emit(Op::BUILD_LIST, count)
      end

      # Compile a dict literal or dict comprehension.
      # Grammar: dict_expr = LBRACE [ dict_body ] RBRACE ;
      def self.compile_dict_expr(compiler, node)
        nodes = extract_nodes(node)
        if nodes.empty?
          compiler.emit(Op::BUILD_DICT, 0)
          return
        end
        compiler.compile_node(nodes[0])
      end

      # Compile the contents of a dict literal.
      # Grammar: dict_body = dict_entry { COMMA dict_entry } [ COMMA ] ;
      def self.compile_dict_body(compiler, node)
        nodes = extract_nodes(node)
        count = 0
        nodes.each do |n|
          if n.rule_name == "dict_entry"
            compiler.compile_node(n)
            count += 1
          end
        end
        compiler.emit(Op::BUILD_DICT, count)
      end

      # Compile a single key: value pair in a dict literal.
      # Grammar: dict_entry = expression COLON expression ;
      def self.compile_dict_entry(compiler, node)
        nodes = extract_nodes(node)
        if nodes.length >= 2
          compiler.compile_node(nodes[0]) # key
          compiler.compile_node(nodes[1]) # value
        end
      end

      # Compile a parenthesized expression or tuple.
      # Grammar: paren_expr = LPAREN [ paren_body ] RPAREN ;
      def self.compile_paren_expr(compiler, node)
        nodes = extract_nodes(node)
        if nodes.empty?
          compiler.emit(Op::BUILD_TUPLE, 0)
          return
        end
        compiler.compile_node(nodes[0])
      end

      # Compile the contents of parentheses.
      # Grammar: paren_body = expression COMMA [ expression { COMMA expression } [ COMMA ] ]
      #                     | expression ;
      def self.compile_paren_body(compiler, node)
        nodes = extract_nodes(node)

        if has_token?(node, ",")
          # Tuple: (a, b, c)
          count = 0
          nodes.each do |n|
            compiler.compile_node(n)
            count += 1
          end
          compiler.emit(Op::BUILD_TUPLE, count)
          return
        end

        # Single expression in parens: (expr)
        compiler.compile_node(nodes[0]) if nodes.length > 0
      end

      # ================================================================
      # Lambda Compilation
      # ================================================================

      # Compile a lambda expression.
      # Grammar: lambda_expr = "lambda" [ lambda_params ] COLON expression ;
      #
      # A lambda is like a def but anonymous and with a single expression body.
      def self.compile_lambda_expr(compiler, node)
        nodes = extract_nodes(node)

        body_expr = nil
        params_node = nil

        nodes.each do |n|
          if n.rule_name == "lambda_params"
            params_node = n
          else
            body_expr = n
          end
        end

        # Extract parameter names
        param_names = []
        if params_node
          params_node.children.each do |child|
            if child.is_a?(CodingAdventures::Parser::ASTNode) && child.rule_name == "lambda_param"
              extract_tokens(child).each do |tok|
                if token_type_name(tok) == "NAME"
                  param_names << tok.value
                  break
                end
              end
            end
          end
        end

        # Compile the lambda body
        body_compiler = create_starlark_compiler
        body_compiler.enter_scope(param_names)
        body_compiler.compile_node(body_expr) if body_expr
        body_compiler.emit(Op::RETURN_VALUE)

        body_code = CodingAdventures::VirtualMachine::CodeObject.new(
          instructions: body_compiler.instructions,
          constants: body_compiler.constants,
          names: body_compiler.names
        )
        body_compiler.exit_scope

        func_info = {
          "code" => body_code,
          "params" => param_names,
          "default_count" => 0
        }

        func_idx = compiler.add_constant(func_info)
        compiler.emit(Op::MAKE_FUNCTION, func_idx)
      end

      # ================================================================
      # Disassembly (for debugging)
      # ================================================================

      # Produce a human-readable disassembly of a CodeObject.
      #
      # @param code [VirtualMachine::CodeObject]
      # @return [String]
      def self.disassemble(code)
        lines = []
        code.instructions.each_with_index do |instr, i|
          name = Op::NAMES[instr.opcode] || "UNKNOWN(0x#{instr.opcode.to_s(16)})"
          if instr.operand
            lines << format("%04d  %-25s %s", i, name, instr.operand.inspect)
          else
            lines << format("%04d  %s", i, name)
          end
        end
        lines.join("\n")
      end
    end
  end
end
