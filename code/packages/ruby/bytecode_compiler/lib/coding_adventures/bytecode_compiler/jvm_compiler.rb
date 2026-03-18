# frozen_string_literal: true

# === JVM Bytecode Compiler -- Targeting the Java Virtual Machine ===
#
# The Java Virtual Machine (JVM) is one of the most successful virtual machines
# ever built. Created by James Gosling at Sun Microsystems in 1995, it has
# become the runtime for Java, Kotlin, Scala, Clojure, and many other languages.
#
# This module compiles our AST into *real* JVM bytecode bytes -- the same format
# that `javac` produces when it compiles .java files into .class files.
#
# === How JVM bytecode works ===
#
# The JVM is a **stack machine**, just like our custom VM. But where our VM uses
# high-level instructions like LOAD_CONST 0 (opcode + index), the JVM uses
# compact byte-level encodings designed to minimize class file size. This was a
# deliberate design choice in 1995 when bandwidth was expensive -- Java applets
# needed to download quickly over dial-up connections.
#
# The JVM has several clever encoding tricks:
#
# 1. **Short-form instructions**: Instead of always using `bipush N` (2 bytes)
#    for small numbers, the JVM has dedicated single-byte opcodes for the most
#    common values: iconst_0 through iconst_5.
#
# 2. **Local variable slots**: Similarly, instead of always using `istore N`
#    (2 bytes), there are single-byte forms istore_0 through istore_3 for the
#    first four local variables.
#
# 3. **Constant pool**: For values too large for bipush (-128 to 127), the JVM
#    stores them in a constant pool and uses `ldc` to reference them by index.

module CodingAdventures
  module BytecodeCompiler
    # Real JVM opcode values from the JVM specification
    ICONST_0  = 0x03
    ICONST_1  = 0x04
    ICONST_2  = 0x05
    ICONST_3  = 0x06
    ICONST_4  = 0x07
    ICONST_5  = 0x08
    BIPUSH    = 0x10
    LDC       = 0x12
    ILOAD     = 0x15
    ILOAD_0   = 0x1A
    ILOAD_1   = 0x1B
    ILOAD_2   = 0x1C
    ILOAD_3   = 0x1D
    ISTORE    = 0x36
    ISTORE_0  = 0x3B
    ISTORE_1  = 0x3C
    ISTORE_2  = 0x3D
    ISTORE_3  = 0x3E
    JVM_POP   = 0x57
    IADD      = 0x60
    ISUB      = 0x64
    IMUL      = 0x68
    IDIV      = 0x6C
    JVM_RETURN = 0xB1

    # Maps source-level operators to JVM bytecode equivalents.
    JVM_OPERATOR_MAP = {
      "+" => IADD,
      "-" => ISUB,
      "*" => IMUL,
      "/" => IDIV
    }.freeze

    # The result of compiling an AST to JVM bytecode.
    #
    # Contains raw bytes (the method body), a constant pool for values too
    # large to encode inline, and local variable metadata.
    JVMCodeObject = Data.define(:bytecode, :constants, :num_locals, :local_names) do
      def initialize(bytecode:, constants: [], num_locals: 0, local_names: [])
        super
      end
    end

    # Compiles an AST into JVM bytecode bytes.
    #
    # Walks the same AST that our custom Compiler uses, but emits raw bytes
    # using real JVM opcode values. Uses the JVM's compact tiered encoding:
    #
    # - Small integers (0-5) use dedicated single-byte iconst_N instructions
    # - Medium integers (-128 to 127) use two-byte bipush N
    # - Larger integers use ldc with a constant pool reference
    # - First four local variables use single-byte istore_N / iload_N
    # - Additional locals use two-byte istore N / iload N
    class JVMCompiler
      def initialize
        @bytecode = []
        @constants = []
        @locals = []
      end

      # Compile a full program AST into JVM bytecode.
      # Every JVM method must end with a return instruction (0xB1).
      def compile(program)
        program.statements.each { |stmt| compile_statement(stmt) }
        @bytecode << JVM_RETURN

        JVMCodeObject.new(
          bytecode: @bytecode.pack("C*").b.freeze,
          constants: @constants,
          num_locals: @locals.length,
          local_names: @locals.dup
        )
      end

      # ------------------------------------------------------------------
      # Statement compilation
      # ------------------------------------------------------------------

      def compile_statement(stmt)
        if stmt.is_a?(CodingAdventures::Parser::Assignment)
          compile_assignment(stmt)
        else
          compile_expression(stmt)
          @bytecode << JVM_POP
        end
      end

      def compile_assignment(node)
        compile_expression(node.value)
        slot = get_local_slot(node.target.name)
        emit_istore(slot)
      end

      # ------------------------------------------------------------------
      # Expression compilation -- the recursive heart
      # ------------------------------------------------------------------

      def compile_expression(node)
        case node
        when CodingAdventures::Parser::NumberLiteral
          emit_number(node.value)

        when CodingAdventures::Parser::StringLiteral
          const_index = add_constant(node.value)
          @bytecode << LDC
          @bytecode << const_index

        when CodingAdventures::Parser::Name
          slot = get_local_slot(node.name)
          emit_iload(slot)

        when CodingAdventures::Parser::BinaryOp
          compile_expression(node.left)
          compile_expression(node.right)
          @bytecode << JVM_OPERATOR_MAP[node.op]

        else
          raise TypeError,
            "Unknown expression type: #{node.class.name}. " \
            "The JVM compiler doesn't know how to handle this AST node."
        end
      end

      # ------------------------------------------------------------------
      # Number encoding -- the JVM's tiered approach
      # ------------------------------------------------------------------

      # Emit the most compact bytecode to push an integer onto the stack.
      #
      # Tier 1: iconst_N (1 byte) for values 0-5
      # Tier 2: bipush N (2 bytes) for values -128 to 127
      # Tier 3: ldc index (2 bytes) for everything else
      def emit_number(value)
        if value >= 0 && value <= 5
          @bytecode << (ICONST_0 + value)
        elsif value >= -128 && value <= 127
          @bytecode << BIPUSH
          @bytecode << (value & 0xFF)
        else
          const_index = add_constant(value)
          @bytecode << LDC
          @bytecode << const_index
        end
      end

      # ------------------------------------------------------------------
      # Local variable encoding -- another tiered approach
      # ------------------------------------------------------------------

      # Emit istore: short form (1 byte) for slots 0-3, long form (2 bytes) otherwise.
      def emit_istore(slot)
        if slot <= 3
          @bytecode << (ISTORE_0 + slot)
        else
          @bytecode << ISTORE
          @bytecode << slot
        end
      end

      # Emit iload: short form (1 byte) for slots 0-3, long form (2 bytes) otherwise.
      def emit_iload(slot)
        if slot <= 3
          @bytecode << (ILOAD_0 + slot)
        else
          @bytecode << ILOAD
          @bytecode << slot
        end
      end

      # ------------------------------------------------------------------
      # Pool management
      # ------------------------------------------------------------------

      def add_constant(value)
        idx = @constants.index(value)
        return idx if idx
        @constants << value
        @constants.length - 1
      end

      def get_local_slot(name)
        idx = @locals.index(name)
        return idx if idx
        @locals << name
        @locals.length - 1
      end
    end
  end
end
