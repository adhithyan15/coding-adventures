# frozen_string_literal: true

# ==========================================================================
# Generic Compiler — A Pluggable AST-to-Bytecode Compiler Framework
# ==========================================================================
#
# The original Compiler class (in compiler.rb) compiles a specific AST
# structure — Program, Assignment, BinaryOp — produced by the hand-written
# parser. But the grammar-driven parser produces a different AST: ASTNode
# objects where each node's rule_name tells you which grammar rule created it.
#
# GenericCompiler solves this: languages register their AST rule handlers
# via register_rule(). The compiler walks the tree, dispatching on each
# node's rule_name. If no handler exists for a single-child node, it
# passes through (for precedence-encoding wrappers). Multi-child nodes
# without a handler raise an error.
#
# Think of it like the GenericVM:
#   - The chassis (tree walking, emission, pools)  = GenericCompiler
#   - The engine (what to do for assign_stmt, etc.) = language-specific plugin
#
# ==========================================================================
# How Compilation Works
# ==========================================================================
#
# Compilation is a tree walk. The compiler starts at the root ASTNode
# (typically rule_name="file"), looks up the handler for that rule name,
# and calls it. The handler processes the node's children — some are tokens
# (leaf values like 42 or "x"), others are sub-nodes (like "expression"
# or "assign_stmt"). For sub-nodes, the handler calls compile_node()
# recursively.
#
# The result is a flat list of Instruction objects — the bytecode — plus
# constant and name pools. Together these form a CodeObject that the VM
# can execute.
#
# ==========================================================================
# Jump Patching
# ==========================================================================
#
# Control flow (if/else, for loops) requires jumps — instructions that
# change the program counter. But when you emit a jump, you don't yet know
# the target address. The solution is backpatching:
#
#   1. Emit a placeholder jump with operand 0.
#   2. Record the instruction's index.
#   3. Compile the body.
#   4. Patch the placeholder with the real target.
#
# The emit_jump() and patch_jump() methods handle this pattern.
# ==========================================================================

module CodingAdventures
  module BytecodeCompiler
    # Base class for all compilation errors.
    class CompilerError < StandardError; end

    # Raised when no handler is registered for an AST rule.
    #
    # This happens when the compiler encounters a rule_name it doesn't know
    # how to compile. Unlike pass-through rules (which have exactly one child),
    # multi-child rules without a handler indicate a missing compilation rule.
    class UnhandledRuleError < CompilerError; end

    # Tracks local variable names within a function scope.
    #
    # When compiling a function body, local variables are assigned numbered
    # slots for fast access. The scope tracks which names have been assigned
    # which slots, and provides the total count of locals needed.
    #
    # This mirrors what CPython does: each function has a co_varnames tuple
    # listing its local variable names, and LOAD_FAST/STORE_FAST reference
    # them by index.
    class CompilerScope
      attr_reader :locals, :parent

      def initialize(parent: nil)
        @locals = {}
        @parent = parent
      end

      # Register a local variable and return its slot index.
      # If already registered, returns the existing index.
      def add_local(name)
        return @locals[name] if @locals.key?(name)

        index = @locals.length
        @locals[name] = index
        index
      end

      # Look up a local variable's slot index. Returns nil if not found.
      def get_local(name)
        @locals[name]
      end

      # Total number of local variables in this scope.
      def num_locals
        @locals.length
      end
    end

    # A pluggable AST-to-bytecode compiler framework.
    #
    # Languages register their AST rule handlers via register_rule() to teach
    # the compiler how to handle language-specific constructs. The compiler
    # provides universal helpers: emit instructions, manage constant and name
    # pools, patch jumps, track scopes, and compile nested CodeObjects.
    #
    # == Usage
    #
    #   compiler = GenericCompiler.new
    #   compiler.register_rule("file", method(:compile_file))
    #   compiler.register_rule("assign_stmt", method(:compile_assign))
    #   code = compiler.compile(ast)
    #
    class GenericCompiler
      attr_reader :instructions, :constants, :names
      attr_accessor :scope

      def initialize
        # -- Bytecode output --------------------------------------------------
        @instructions = []
        @constants = []
        @names = []

        # -- Plugin registry --------------------------------------------------
        @dispatch = {}

        # -- Scope tracking ---------------------------------------------------
        @scope = nil

        # -- Nested code objects ----------------------------------------------
        @code_objects = []
      end

      # ====================================================================
      # Plugin Registration
      # ====================================================================

      # Register a compilation handler for a grammar rule.
      #
      # The handler must be a callable that accepts (compiler, node) and
      # emits instructions by calling compiler.emit().
      #
      #   compiler.register_rule("assign_stmt", method(:compile_assign))
      #   compiler.register_rule("arith", ->(c, node) { ... })
      #
      def register_rule(rule_name, handler)
        @dispatch[rule_name] = handler
      end

      # ====================================================================
      # Instruction Emission
      # ====================================================================

      # Emit a single bytecode instruction.
      #
      # Returns the index of the emitted instruction (useful for jump patching).
      def emit(opcode, operand = nil)
        index = @instructions.length
        @instructions << CodingAdventures::VirtualMachine::Instruction.new(
          opcode: opcode, operand: operand
        )
        index
      end

      # Emit a jump instruction with a placeholder target of 0.
      #
      # The returned index is used later with patch_jump() to fill in
      # the real target once you know it.
      def emit_jump(opcode)
        emit(opcode, 0)
      end

      # Patch a previously emitted jump with its real target.
      #
      # If target is nil, patches to the current instruction offset
      # (the most common case — "jump to here").
      def patch_jump(index, target = nil)
        target = current_offset if target.nil?
        old = @instructions[index]
        @instructions[index] = CodingAdventures::VirtualMachine::Instruction.new(
          opcode: old.opcode, operand: target
        )
      end

      # The index where the next emitted instruction will go.
      def current_offset
        @instructions.length
      end

      # ====================================================================
      # Constant and Name Pool Management
      # ====================================================================

      # Add a value to the constant pool, returning its index.
      # Deduplicates: if the value already exists, returns the existing index.
      def add_constant(value)
        idx = @constants.index(value)
        return idx if idx

        @constants << value
        @constants.length - 1
      end

      # Add a variable/function name to the name pool, returning its index.
      # Deduplicates just like constants.
      def add_name(name)
        idx = @names.index(name)
        return idx if idx

        @names << name
        @names.length - 1
      end

      # ====================================================================
      # Scope Management
      # ====================================================================

      # Enter a new local scope (for function bodies).
      #
      # If parameter names are provided, they're registered as the first
      # local slots.
      def enter_scope(params = nil)
        new_scope = CompilerScope.new(parent: @scope)
        if params
          params.each { |name| new_scope.add_local(name) }
        end
        @scope = new_scope
        new_scope
      end

      # Exit the current scope and return to the parent.
      #
      # Returns the scope that was exited (in case you need its locals info).
      def exit_scope
        raise CompilerError, "Cannot exit scope — not in any scope" if @scope.nil?

        old = @scope
        @scope = @scope.parent
        old
      end

      # ====================================================================
      # Nested Code Object Compilation
      # ====================================================================

      # Compile a sub-tree into a separate CodeObject (for function bodies).
      #
      # Saves the current compiler state, compiles the node into a fresh
      # instruction list, builds a CodeObject, and restores the original state.
      def compile_nested(node)
        saved_instructions = @instructions
        saved_constants = @constants
        saved_names = @names

        @instructions = []
        @constants = []
        @names = []

        compile_node(node)

        nested = CodingAdventures::VirtualMachine::CodeObject.new(
          instructions: @instructions,
          constants: @constants,
          names: @names
        )
        @code_objects << nested

        @instructions = saved_instructions
        @constants = saved_constants
        @names = saved_names

        nested
      end

      # ====================================================================
      # AST Dispatch — the recursive core
      # ====================================================================

      # Compile an AST node by dispatching to the registered handler.
      #
      # For each node:
      # 1. If it has a registered handler, call that handler.
      # 2. If it has exactly one child and no handler, pass through to
      #    the child (precedence-encoding nodes).
      # 3. Otherwise, raise UnhandledRuleError.
      #
      # Tokens (leaf nodes) are passed to compile_token(), which is a
      # no-op by default — structural tokens (NEWLINE, INDENT) are ignored,
      # and meaningful tokens are handled by their parent rule's handler.
      def compile_node(node)
        # If it's a token (leaf node), handle it separately.
        if node.respond_to?(:type) && !node.respond_to?(:rule_name)
          compile_token(node)
          return
        end

        handler = @dispatch[node.rule_name]
        if handler
          handler.call(self, node)
        elsif node.children.length == 1
          compile_node(node.children[0])
        else
          raise UnhandledRuleError,
            "No handler registered for rule '#{node.rule_name}' " \
            "and it has #{node.children.length} children (not a pass-through). " \
            "Register a handler with compiler.register_rule('#{node.rule_name}', handler)."
        end
      end

      # Compile a bare token. No-op by default — structural tokens are
      # ignored, meaningful tokens are handled by their parent rule's handler.
      def compile_token(_token)
        # No-op — override in subclass or handle in rule handlers.
      end

      # ====================================================================
      # Top-Level Compile API
      # ====================================================================

      # Compile an AST into a CodeObject.
      #
      # This is the main entry point. Compiles the root AST node into
      # bytecode instructions, appends a HALT instruction, and returns
      # a CodeObject ready for the VM to execute.
      def compile(ast, halt_opcode: 0xFF)
        compile_node(ast)
        emit(halt_opcode)

        CodingAdventures::VirtualMachine::CodeObject.new(
          instructions: @instructions,
          constants: @constants,
          names: @names
        )
      end
    end
  end
end
