# frozen_string_literal: true

# === WASM Bytecode Compiler -- Targeting WebAssembly ===
#
# WebAssembly (WASM) is the newest of our three target VMs, standardized in
# 2017 by the W3C. Unlike the JVM (1995) and CLR (2002), which were designed
# for general-purpose application development, WASM was designed specifically
# for the web -- a compact, fast, safe bytecode format that runs in browsers
# alongside JavaScript.
#
# WASM's design philosophy differs from the JVM and CLR in several ways:
#
# 1. **Simplicity over compactness**: WASM uses a uniform encoding for most
#    values. Where the JVM has iconst_0..iconst_5, WASM always uses i32.const
#    followed by a full 4-byte value. Simpler to encode/decode at a slight
#    cost in size.
#
# 2. **No explicit pop**: WASM handles stack cleanup implicitly at function
#    boundaries via the `end` instruction. Expression statements don't need
#    an explicit pop.
#
# 3. **Module-based**: WASM code lives in modules with explicit imports and
#    exports. No global mutable state accessible from outside.

module CodingAdventures
  module BytecodeCompiler
    # Real WASM opcode values from the WebAssembly specification
    WASM_END       = 0x0B
    WASM_LOCAL_GET = 0x20
    WASM_LOCAL_SET = 0x21
    WASM_I32_CONST = 0x41
    WASM_I32_ADD   = 0x6A
    WASM_I32_SUB   = 0x6B
    WASM_I32_MUL   = 0x6C
    WASM_I32_DIV_S = 0x6D

    # Maps source-level operators to WASM bytecode equivalents.
    WASM_OPERATOR_MAP = {
      "+" => WASM_I32_ADD,
      "-" => WASM_I32_SUB,
      "*" => WASM_I32_MUL,
      "/" => WASM_I32_DIV_S
    }.freeze

    # The result of compiling an AST to WASM bytecode.
    #
    # WASM does not need a separate constant pool -- all integer constants
    # are encoded inline using i32.const followed by 4 bytes.
    WASMCodeObject = Data.define(:bytecode, :num_locals, :local_names) do
      def initialize(bytecode:, num_locals: 0, local_names: [])
        super
      end
    end

    # Compiles an AST into WASM bytecode bytes.
    #
    # The WASM compiler is the simplest of our three backends because WASM
    # uses a uniform encoding: every integer is 5 bytes (opcode + 4-byte value),
    # and every local variable access is 2 bytes (opcode + index). No short
    # forms to choose between.
    #
    # Another WASM-specific detail: expression statements don't need an explicit
    # pop. WASM validates the stack at function boundaries, and the `end`
    # instruction handles remaining stack cleanup.
    class WASMCompiler
      def initialize
        @bytecode = []
        @locals = []
      end

      # Compile a full program AST into WASM bytecode.
      # Every WASM function body ends with `end` (0x0B).
      def compile(program)
        program.statements.each { |stmt| compile_statement(stmt) }
        @bytecode << WASM_END

        WASMCodeObject.new(
          bytecode: @bytecode.pack("C*").b.freeze,
          num_locals: @locals.length,
          local_names: @locals.dup
        )
      end

      # ------------------------------------------------------------------
      # Statement compilation
      # ------------------------------------------------------------------

      # WASM: no explicit pop needed for expression statements.
      # The stack is validated at the function boundary.
      def compile_statement(stmt)
        if stmt.is_a?(CodingAdventures::Parser::Assignment)
          compile_assignment(stmt)
        else
          compile_expression(stmt)
        end
      end

      def compile_assignment(node)
        compile_expression(node.value)
        slot = get_local_slot(node.target.name)
        @bytecode << WASM_LOCAL_SET
        @bytecode << slot
      end

      # ------------------------------------------------------------------
      # Expression compilation
      # ------------------------------------------------------------------

      def compile_expression(node)
        case node
        when CodingAdventures::Parser::NumberLiteral
          # WASM always uses i32.const followed by 4-byte little-endian.
          # No short forms, no constant pool -- just the value inline.
          @bytecode << WASM_I32_CONST
          @bytecode.concat([node.value].pack("l<").bytes)

        when CodingAdventures::Parser::StringLiteral
          raise TypeError,
            "WASM compiler does not support string literals yet. " \
            "Got: #{node.value.inspect}"

        when CodingAdventures::Parser::Name
          slot = get_local_slot(node.name)
          @bytecode << WASM_LOCAL_GET
          @bytecode << slot

        when CodingAdventures::Parser::BinaryOp
          compile_expression(node.left)
          compile_expression(node.right)
          @bytecode << WASM_OPERATOR_MAP[node.op]

        else
          raise TypeError,
            "Unknown expression type: #{node.class.name}. " \
            "The WASM compiler doesn't know how to handle this AST node."
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
