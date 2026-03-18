# frozen_string_literal: true

# === Bytecode Compiler -- The Bridge Between Parsing and Execution ===
#
# In the previous layers we built a lexer (Layer 2) that turns source code into
# tokens, and a parser (Layer 3) that arranges those tokens into an Abstract
# Syntax Tree (AST). Now we face the next question: how do we turn a *tree*
# into something a machine can actually execute?
#
# The answer is **compilation** -- walking the tree and emitting a flat sequence
# of stack-machine instructions. This is exactly what real compilers do:
#
#   javac   : Java source   -->  JVM bytecode  (.class files)
#   csc     : C# source     -->  CLR IL        (.dll files)
#   cpython : Python source  -->  Python bytecode (.pyc files)
#   Ours    : AST            -->  CodeObject    (for our VM)
#
# The key insight is that a tree-structured program can always be "flattened"
# into a sequence of stack operations. Consider the expression `1 + 2 * 3`.
# The AST looks like this:
#
#         +
#        / \
#       1   *
#          / \
#         2   3
#
# To evaluate this on a stack machine, we do a **post-order traversal** (visit
# children before the parent):
#
#   1. Visit the left child of `+`:  emit LOAD_CONST 1
#   2. Visit the right child of `+` (which is `*`):
#      a. Visit left child of `*`:   emit LOAD_CONST 2
#      b. Visit right child of `*`:  emit LOAD_CONST 3
#      c. Visit `*` itself:          emit MUL
#   3. Visit `+` itself:             emit ADD
#
# The result is: LOAD_CONST 1, LOAD_CONST 2, LOAD_CONST 3, MUL, ADD
#
# This is called **Reverse Polish Notation** (RPN), and it is the natural
# output format for a stack-machine compiler.
#
# === Terminology ===
#
# - **Emit**: Append an instruction to the output list.
# - **Constant pool**: A list of literal values (numbers, strings) that
#   instructions reference by index.
# - **Name pool**: A list of variable names, similarly referenced by index.
# - **CodeObject**: The final compiled artifact -- instructions + pools.

module CodingAdventures
  module BytecodeCompiler
    # Maps source-level operator symbols to their corresponding VM opcodes.
    # Each arithmetic operator has a direct counterpart in the VM instruction
    # set. Using a hash separates data (the mapping) from logic (compilation).
    OPERATOR_MAP = {
      "+" => CodingAdventures::VirtualMachine::OpCode::ADD,
      "-" => CodingAdventures::VirtualMachine::OpCode::SUB,
      "*" => CodingAdventures::VirtualMachine::OpCode::MUL,
      "/" => CodingAdventures::VirtualMachine::OpCode::DIV
    }.freeze

    # Compiles an AST into a CodeObject for the virtual machine.
    #
    # This is the bridge between the parser (which understands language syntax)
    # and the VM (which executes instructions). The compiler's job is to
    # translate tree-structured code into a flat sequence of stack operations.
    #
    # The compiler maintains three pieces of state as it walks the AST:
    #
    # 1. **instructions** -- The growing list of bytecode instructions.
    # 2. **constants** -- The constant pool for literal values.
    # 3. **names** -- The name pool for variable names.
    #
    # Example walkthrough for `x = 1 + 2`:
    #
    #   AST:
    #     Assignment(target=Name("x"), value=BinaryOp(1, "+", 2))
    #
    #   Step 1: compile_assignment is called
    #   Step 2: It calls compile_expression on the BinaryOp
    #   Step 3: compile_expression recurses:
    #     - Left:  NumberLiteral(1) -> adds 1 to constants[0], emits LOAD_CONST 0
    #     - Right: NumberLiteral(2) -> adds 2 to constants[1], emits LOAD_CONST 1
    #     - Op "+":                 -> emits ADD
    #   Step 4: Back in compile_assignment:
    #     - Adds "x" to names[0], emits STORE_NAME 0
    #
    #   Result:
    #     instructions = [LOAD_CONST 0, LOAD_CONST 1, ADD, STORE_NAME 0, HALT]
    #     constants    = [1, 2]
    #     names        = ["x"]
    class Compiler
      attr_reader :instructions, :constants, :names

      def initialize
        @instructions = []
        @constants = []
        @names = []
      end

      # Compile a full program AST into a CodeObject.
      #
      # Iterates over every statement in the program, compiles each one,
      # then appends a final HALT instruction to tell the VM execution is
      # complete. Returns a CodeObject ready for the VM to execute.
      def compile(program)
        program.statements.each { |stmt| compile_statement(stmt) }

        # Every program ends with HALT so the VM knows to stop -- just like
        # a CPU needs a HLT instruction.
        @instructions << CodingAdventures::VirtualMachine::Instruction.new(CodingAdventures::VirtualMachine::OpCode::HALT)

        CodingAdventures::VirtualMachine::CodeObject.new(
          instructions: @instructions,
          constants: @constants,
          names: @names
        )
      end

      # ------------------------------------------------------------------
      # Statement compilation
      # ------------------------------------------------------------------

      # Compile a single statement. There are two kinds:
      #
      # 1. Assignment (`x = expr`) -- evaluate the expression, store the result.
      # 2. Expression statement (just `expr`) -- evaluate for side effects,
      #    then emit POP to discard the result and keep the stack clean.
      def compile_statement(stmt)
        if stmt.is_a?(CodingAdventures::Parser::Assignment)
          compile_assignment(stmt)
        else
          compile_expression(stmt)
          @instructions << CodingAdventures::VirtualMachine::Instruction.new(CodingAdventures::VirtualMachine::OpCode::POP)
        end
      end

      # Compile `name = expression`. Strategy:
      # 1. Compile the RHS expression (pushes value onto stack)
      # 2. Emit STORE_NAME to pop the value and bind it to the name
      def compile_assignment(node)
        compile_expression(node.value)
        name_index = add_name(node.target.name)
        @instructions << CodingAdventures::VirtualMachine::Instruction.new(CodingAdventures::VirtualMachine::OpCode::STORE_NAME, name_index)
      end

      # ------------------------------------------------------------------
      # Expression compilation -- the recursive heart
      # ------------------------------------------------------------------

      # Compile an expression, leaving exactly one value on the stack.
      # This is the fundamental contract: after compilation, the stack has
      # N + 1 items (the expression's value on top).
      def compile_expression(node)
        case node
        when CodingAdventures::Parser::NumberLiteral
          const_index = add_constant(node.value)
          @instructions << CodingAdventures::VirtualMachine::Instruction.new(CodingAdventures::VirtualMachine::OpCode::LOAD_CONST, const_index)

        when CodingAdventures::Parser::StringLiteral
          const_index = add_constant(node.value)
          @instructions << CodingAdventures::VirtualMachine::Instruction.new(CodingAdventures::VirtualMachine::OpCode::LOAD_CONST, const_index)

        when CodingAdventures::Parser::Name
          name_index = add_name(node.name)
          @instructions << CodingAdventures::VirtualMachine::Instruction.new(CodingAdventures::VirtualMachine::OpCode::LOAD_NAME, name_index)

        when CodingAdventures::Parser::BinaryOp
          # Post-order traversal: left, right, operator.
          # This naturally produces Reverse Polish Notation (RPN).
          compile_expression(node.left)
          compile_expression(node.right)
          opcode = OPERATOR_MAP[node.op]
          @instructions << CodingAdventures::VirtualMachine::Instruction.new(opcode)

        else
          raise TypeError,
            "Unknown expression type: #{node.class.name}. " \
            "The compiler doesn't know how to handle this AST node."
        end
      end

      # ------------------------------------------------------------------
      # Pool management -- constants and names
      # ------------------------------------------------------------------

      # Add a constant to the pool, returning its index. Deduplicates:
      # if the value already exists, returns the existing index.
      def add_constant(value)
        idx = @constants.index(value)
        return idx if idx
        @constants << value
        @constants.length - 1
      end

      # Add a variable name to the name pool, returning its index.
      # Deduplicates just like constants.
      def add_name(name)
        idx = @names.index(name)
        return idx if idx
        @names << name
        @names.length - 1
      end
    end

    # ------------------------------------------------------------------
    # Convenience function for end-to-end compilation
    # ------------------------------------------------------------------

    # Source code string -> CodeObject in one call.
    #
    # Chains the entire front-end pipeline:
    #   Source -> Lexer -> Tokens -> Parser -> AST -> Compiler -> CodeObject
    #
    # This is the simplest way to go from human-readable code to VM-executable
    # bytecode. Under the hood it creates a fresh Lexer, Parser, and Compiler
    # for each call.
    def self.compile_source(source, keywords: nil)
      kw = keywords || []
      tokens = CodingAdventures::Lexer::Tokenizer.new(source, keywords: kw).tokenize
      ast = CodingAdventures::Parser::RecursiveDescentParser.new(tokens).parse
      Compiler.new.compile(ast)
    end
  end
end
