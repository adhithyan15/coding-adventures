# frozen_string_literal: true

# === CLR IL Compiler -- Targeting the Common Language Runtime ===
#
# The Common Language Runtime (CLR) is Microsoft's virtual machine, introduced
# in 2002 as part of the .NET Framework. Like the JVM, it is a stack-based VM
# that runs bytecode -- but Microsoft calls it "Intermediate Language" (IL).
#
# The CLR was designed *after* the JVM, and its designers learned from both the
# JVM's strengths and limitations. Some notable differences:
#
# - **Wider short-form range**: The CLR has dedicated opcodes for constants 0
#   through 8 (the JVM only goes to 5).
# - **Signed byte encoding**: ldc.i4.s uses a signed byte for -128 to 127.
# - **Full 32-bit inline encoding**: For larger values, ldc.i4 embeds a full
#   4-byte little-endian integer directly in the bytecode stream (5 bytes total).
#   The JVM instead references a constant pool entry.

module CodingAdventures
  module BytecodeCompiler
    # Real CLR IL opcode values from the ECMA-335 specification
    CLR_LDLOC_0  = 0x06
    CLR_LDLOC_1  = 0x07
    CLR_LDLOC_2  = 0x08
    CLR_LDLOC_3  = 0x09
    CLR_STLOC_0  = 0x0A
    CLR_STLOC_1  = 0x0B
    CLR_STLOC_2  = 0x0C
    CLR_STLOC_3  = 0x0D
    CLR_LDLOC_S  = 0x11
    CLR_STLOC_S  = 0x13
    CLR_LDC_I4_0 = 0x16
    CLR_LDC_I4_1 = 0x17
    CLR_LDC_I4_2 = 0x18
    CLR_LDC_I4_3 = 0x19
    CLR_LDC_I4_4 = 0x1A
    CLR_LDC_I4_5 = 0x1B
    CLR_LDC_I4_6 = 0x1C
    CLR_LDC_I4_7 = 0x1D
    CLR_LDC_I4_8 = 0x1E
    CLR_LDC_I4_S = 0x1F
    CLR_LDC_I4   = 0x20
    CLR_POP      = 0x26
    CLR_RET      = 0x2A
    CLR_ADD      = 0x58
    CLR_SUB      = 0x59
    CLR_MUL      = 0x5A
    CLR_DIV      = 0x5B

    # Maps source-level operators to CLR IL bytecode equivalents.
    CLR_OPERATOR_MAP = {
      "+" => CLR_ADD,
      "-" => CLR_SUB,
      "*" => CLR_MUL,
      "/" => CLR_DIV
    }.freeze

    # The result of compiling an AST to CLR IL bytecode.
    #
    # Unlike the JVM, the CLR does not need a separate constant pool for
    # integers -- it embeds them directly in the bytecode stream using ldc.i4.
    CLRCodeObject = Data.define(:bytecode, :num_locals, :local_names) do
      def initialize(bytecode:, num_locals: 0, local_names: [])
        super
      end
    end

    # Compiles an AST into CLR IL bytecode bytes.
    #
    # Follows the same pattern as the JVM compiler: walk the AST in post-order,
    # emitting stack-machine instructions. Differences are in encoding details:
    #
    # - Wider short-form range: constants 0-8 have dedicated single-byte opcodes
    # - Inline integers: large constants are embedded directly as 4-byte LE values
    # - Different opcode values: add is 0x58 (vs JVM's 0x60)
    class CLRCompiler
      def initialize
        @bytecode = []
        @locals = []
      end

      # Compile a full program AST into CLR IL bytecode.
      # Every CLR method body must end with a ret instruction (0x2A).
      def compile(program)
        program.statements.each { |stmt| compile_statement(stmt) }
        @bytecode << CLR_RET

        CLRCodeObject.new(
          bytecode: @bytecode.pack("C*").b.freeze,
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
          @bytecode << CLR_POP
        end
      end

      def compile_assignment(node)
        compile_expression(node.value)
        slot = get_local_slot(node.target.name)
        emit_stloc(slot)
      end

      # ------------------------------------------------------------------
      # Expression compilation
      # ------------------------------------------------------------------

      def compile_expression(node)
        case node
        when CodingAdventures::Parser::NumberLiteral
          emit_number(node.value)

        when CodingAdventures::Parser::StringLiteral
          raise TypeError,
            "CLR compiler does not support string literals yet. " \
            "Got: #{node.value.inspect}"

        when CodingAdventures::Parser::Name
          slot = get_local_slot(node.name)
          emit_ldloc(slot)

        when CodingAdventures::Parser::BinaryOp
          compile_expression(node.left)
          compile_expression(node.right)
          @bytecode << CLR_OPERATOR_MAP[node.op]

        else
          raise TypeError,
            "Unknown expression type: #{node.class.name}. " \
            "The CLR compiler doesn't know how to handle this AST node."
        end
      end

      # ------------------------------------------------------------------
      # Number encoding -- the CLR's three tiers
      # ------------------------------------------------------------------

      # Emit the most compact IL to push an integer onto the stack.
      #
      # Tier 1: ldc.i4.N (1 byte) for values 0-8
      # Tier 2: ldc.i4.s N (2 bytes) for values -128 to 127
      # Tier 3: ldc.i4 N (5 bytes) for everything else
      def emit_number(value)
        if value >= 0 && value <= 8
          @bytecode << (CLR_LDC_I4_0 + value)
        elsif value >= -128 && value <= 127
          @bytecode << CLR_LDC_I4_S
          @bytecode << (value & 0xFF)
        else
          @bytecode << CLR_LDC_I4
          # 4-byte little-endian signed int32
          @bytecode.concat([value].pack("l<").bytes)
        end
      end

      # ------------------------------------------------------------------
      # Local variable encoding
      # ------------------------------------------------------------------

      # Emit stloc: short form (1 byte) for slots 0-3, stloc.s (2 bytes) otherwise.
      def emit_stloc(slot)
        if slot <= 3
          @bytecode << (CLR_STLOC_0 + slot)
        else
          @bytecode << CLR_STLOC_S
          @bytecode << slot
        end
      end

      # Emit ldloc: short form (1 byte) for slots 0-3, ldloc.s (2 bytes) otherwise.
      def emit_ldloc(slot)
        if slot <= 3
          @bytecode << (CLR_LDLOC_0 + slot)
        else
          @bytecode << CLR_LDLOC_S
          @bytecode << slot
        end
      end

      # ------------------------------------------------------------------
      # Local slot management
      # ------------------------------------------------------------------

      def get_local_slot(name)
        idx = @locals.index(name)
        return idx if idx
        @locals << name
        @locals.length - 1
      end
    end
  end
end
