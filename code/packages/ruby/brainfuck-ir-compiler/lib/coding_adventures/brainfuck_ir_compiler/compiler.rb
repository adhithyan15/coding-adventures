# frozen_string_literal: true

# ==========================================================================
# BrainfuckIrCompiler — Translates a Brainfuck AST into IR
# ==========================================================================
#
# This is the Brainfuck-specific frontend of the AOT compiler pipeline. It
# knows Brainfuck semantics (tape, cells, pointer, loops, I/O) and translates
# them into target-independent IR instructions. It does NOT know about RISC-V,
# ARM, ELF, or any specific machine target.
#
# The compiler produces two outputs:
#   1. An IrProgram containing the compiled IR instructions
#   2. A SourceMapChain with SourceToAst and AstToIr segments filled in
#
# ── Register allocation ──────────────────────────────────────────────────────
#
# Brainfuck needs very few registers:
#
#   v0 = tape base address (pointer to the start of the tape)
#   v1 = tape pointer offset (current cell index, 0-based)
#   v2 = temporary (cell value for loads/stores)
#   v3 = temporary (for bounds checks)
#   v4 = temporary (for syscall arguments)
#   v5 = max pointer value (tape_size - 1, for bounds checks)
#   v6 = zero constant (for bounds checks)
#
# This fixed allocation maps directly to a small set of physical registers
# in any ISA. Future languages that need more registers will use a register
# allocator in the backend.
#
# ── Compilation mapping ─────────────────────────────────────────────────────
#
# ┌──────────────────┬────────────────────────────────────────────────────┐
# │ Command          │ IR Output                                          │
# ├──────────────────┼────────────────────────────────────────────────────┤
# │ > (RIGHT)        │ ADD_IMM v1, v1, 1                                  │
# │ < (LEFT)         │ ADD_IMM v1, v1, -1                                 │
# │ + (INC)          │ LOAD_BYTE v2, v0, v1; ADD_IMM v2, v2, 1;          │
# │                  │ AND_IMM v2, v2, 255; STORE_BYTE v2, v0, v1        │
# │ - (DEC)          │ LOAD_BYTE v2, v0, v1; ADD_IMM v2, v2, -1;         │
# │                  │ AND_IMM v2, v2, 255; STORE_BYTE v2, v0, v1        │
# │ . (OUTPUT)       │ LOAD_BYTE v2, v0, v1; ADD_IMM v4, v2, 0; SYSCALL 1 │
# │ , (INPUT)        │ SYSCALL 2; STORE_BYTE v4, v0, v1                  │
# └──────────────────┴────────────────────────────────────────────────────┘
#
# The AND_IMM v2, v2, 255 step ("byte masking") keeps cells in the 0-255 range
# per the Brainfuck specification. It can be disabled via BuildConfig.
# ==========================================================================

require "coding_adventures_compiler_ir"
require "coding_adventures_compiler_source_map"
require "coding_adventures_lexer"
require "coding_adventures_parser"

module CodingAdventures
  module BrainfuckIrCompiler
    IR = CodingAdventures::CompilerIr
    SM = CodingAdventures::CompilerSourceMap

    # CompileResult holds the two outputs of compilation.
    #
    # Attributes:
    #   program    [IR::IrProgram]     the compiled IR instructions
    #   source_map [SM::SourceMapChain] the source map chain (segments 1+2 filled)
    CompileResult = Struct.new(:program, :source_map, keyword_init: true)

    # compile(ast, filename, config) → CompileResult
    #
    # Takes a Brainfuck AST (from CodingAdventures::Brainfuck::Parser.parse)
    # and a source filename, and produces an IrProgram plus source map segments.
    #
    # The filename is used in source map entries to identify which file the
    # source positions refer to.
    #
    # @param ast      [CodingAdventures::Parser::ASTNode] the root "program" node
    # @param filename [String] the source file name (e.g., "hello.bf")
    # @param config   [BuildConfig] compilation flags
    # @return [CompileResult]
    # @raise [RuntimeError] if the AST root is not "program", or tape_size <= 0
    def self.compile(ast, filename, config)
      Compiler.new(ast, filename, config).compile
    end

    # ──────────────────────────────────────────────────────────────────────────
    # Internal Compiler class
    #
    # Holds all mutable state for one compilation run. Not exposed as part of
    # the public API — callers use the module-level compile/3 method.
    # ──────────────────────────────────────────────────────────────────────────
    class Compiler
      # Virtual register indices — fixed allocation for Brainfuck
      REG_TAPE_BASE = 0 # v0: base address of the tape
      REG_TAPE_PTR  = 1 # v1: current cell offset (0-based index)
      REG_TEMP      = 2 # v2: temporary for cell values
      REG_TEMP2     = 3 # v3: temporary for bounds checks
      REG_SYS_ARG   = 4 # v4: syscall argument register
      REG_MAX_PTR   = 5 # v5: tape_size - 1 (upper bound for bounds checks)
      REG_ZERO      = 6 # v6: constant 0 (lower bound for bounds checks)

      # Syscall numbers (match the RISC-V simulator's ecall dispatch)
      SYSCALL_WRITE = 1  # write byte in a0 to stdout
      SYSCALL_READ  = 2  # read byte from stdin into a0
      SYSCALL_EXIT  = 10 # halt with exit code in a0

      def initialize(ast, filename, config)
        @ast = ast
        @filename = filename
        @config = config
        @id_gen = IR::IDGenerator.new
        @node_id_gen = 0
        @program = IR::IrProgram.new("_start")
        @source_map = SM::SourceMapChain.new
        @loop_count = 0
      end

      def compile
        unless @ast.rule_name == "program"
          raise "expected 'program' AST node, got #{@ast.rule_name.inspect}"
        end

        if @config.tape_size <= 0
          raise "invalid tape_size #{@config.tape_size}: must be positive"
        end

        # Add tape data declaration — the only global memory for Brainfuck
        @program.add_data(IR::IrDataDecl.new("tape", @config.tape_size, 0))

        emit_prologue
        compile_program(@ast)
        emit_epilogue

        CompileResult.new(program: @program, source_map: @source_map)
      end

      private

      # ── ID management ─────────────────────────────────────────────────────

      def next_node_id
        id = @node_id_gen
        @node_id_gen += 1
        id
      end

      # emit(opcode, *operands) → Integer
      #
      # Appends one instruction to the program and returns its unique ID.
      # All real instructions (not labels) flow through this method so that
      # the ID counter stays consistent.
      def emit(opcode, *operands)
        id = @id_gen.next
        @program.add_instruction(IR::IrInstruction.new(opcode, operands, id))
        id
      end

      # emit_label(name) appends a LABEL instruction.
      #
      # Labels have ID -1 because they produce no machine code and therefore
      # have no meaningful position in the output stream.
      def emit_label(name)
        @program.add_instruction(
          IR::IrInstruction.new(IR::IrOp::LABEL, [IR::IrLabel.new(name)], -1)
        )
      end

      # ── Prologue ──────────────────────────────────────────────────────────
      #
      # The prologue sets up the execution environment before any Brainfuck
      # commands run:
      #
      #   LABEL    _start
      #   LOAD_ADDR v0, tape       ← v0 = base address of tape array
      #   LOAD_IMM  v1, 0          ← v1 = tape pointer starts at cell 0
      #
      # In debug builds, two more registers are initialised for bounds checks:
      #
      #   LOAD_IMM  v5, tape_size-1 ← v5 = max valid pointer (upper bound)
      #   LOAD_IMM  v6, 0           ← v6 = 0 (lower bound for left checks)

      def emit_prologue
        emit_label("_start")

        # v0 = &tape (base address of the tape array in memory)
        emit(IR::IrOp::LOAD_ADDR, IR::IrRegister.new(REG_TAPE_BASE), IR::IrLabel.new("tape"))

        # v1 = 0 (tape pointer starts at cell 0)
        emit(IR::IrOp::LOAD_IMM, IR::IrRegister.new(REG_TAPE_PTR), IR::IrImmediate.new(0))

        return unless @config.insert_bounds_checks

        # v5 = tape_size - 1 (the highest valid cell index)
        emit(IR::IrOp::LOAD_IMM,
             IR::IrRegister.new(REG_MAX_PTR),
             IR::IrImmediate.new(@config.tape_size - 1))

        # v6 = 0 (for the lower bound check: pointer must be >= 0)
        emit(IR::IrOp::LOAD_IMM,
             IR::IrRegister.new(REG_ZERO),
             IR::IrImmediate.new(0))
      end

      # ── Epilogue ──────────────────────────────────────────────────────────
      #
      # The epilogue terminates the program cleanly:
      #
      #   HALT
      #
      # In debug builds, an out-of-bounds trap handler is appended after HALT.
      # The CPU never falls through to it from normal execution — it is only
      # reached via BRANCH_NZ from a failed bounds check.
      #
      #   LABEL    __trap_oob
      #   LOAD_IMM  v4, 1
      #   SYSCALL   10              ← exit with code 1

      def emit_epilogue
        emit(IR::IrOp::HALT)

        return unless @config.insert_bounds_checks

        emit_label("__trap_oob")
        emit(IR::IrOp::LOAD_IMM, IR::IrRegister.new(REG_SYS_ARG), IR::IrImmediate.new(1))
        emit(IR::IrOp::SYSCALL, IR::IrImmediate.new(SYSCALL_EXIT))
      end

      # ── AST walking ───────────────────────────────────────────────────────
      #
      # The Brainfuck AST has this structure (from brainfuck.grammar):
      #
      #   program     → { instruction }
      #   instruction → loop | command
      #   loop        → LOOP_START { instruction } LOOP_END
      #   command     → RIGHT | LEFT | INC | DEC | OUTPUT | INPUT
      #
      # The compiler walks this tree recursively, emitting IR for each node.

      def compile_program(node)
        node.children.each do |child|
          next unless child.is_a?(CodingAdventures::Parser::ASTNode)

          compile_node(child)
        end
      end

      def compile_node(node)
        case node.rule_name
        when "instruction"
          # An instruction node wraps either a loop or a command — just recurse
          node.children.each do |child|
            next unless child.is_a?(CodingAdventures::Parser::ASTNode)

            compile_node(child)
          end

        when "command"
          compile_command(node)

        when "loop"
          compile_loop(node)

        else
          raise "unexpected AST node type: #{node.rule_name.inspect}"
        end
      end

      # ── Command compilation ───────────────────────────────────────────────
      #
      # Each Brainfuck command maps to a fixed sequence of IR instructions.
      # The mapping is documented in the file header table above.
      #
      # Every command also creates two source map entries:
      #   - SourceToAst: source position (file, line, col) → AST node ID
      #   - AstToIr:     AST node ID → [list of IR instruction IDs emitted]

      def compile_command(node)
        tok = extract_token(node)
        raise "command node has no token" if tok.nil?

        ast_node_id = next_node_id
        @source_map.source_to_ast.add(
          SM::SourcePosition.new(
            file: @filename, line: tok.line, column: tok.column, length: 1
          ),
          ast_node_id
        )

        ir_ids = []

        case tok.value
        when ">"
          # RIGHT: move tape pointer one cell to the right
          # Bounds check first (if enabled): trap if ptr >= tape_size - 1
          ir_ids.concat(emit_bounds_check_right) if @config.insert_bounds_checks
          ir_ids << emit(IR::IrOp::ADD_IMM,
                         IR::IrRegister.new(REG_TAPE_PTR),
                         IR::IrRegister.new(REG_TAPE_PTR),
                         IR::IrImmediate.new(1))

        when "<"
          # LEFT: move tape pointer one cell to the left
          # Bounds check first (if enabled): trap if ptr < 0
          ir_ids.concat(emit_bounds_check_left) if @config.insert_bounds_checks
          ir_ids << emit(IR::IrOp::ADD_IMM,
                         IR::IrRegister.new(REG_TAPE_PTR),
                         IR::IrRegister.new(REG_TAPE_PTR),
                         IR::IrImmediate.new(-1))

        when "+"
          # INC: increment current cell by 1
          ir_ids.concat(emit_cell_mutation(1))

        when "-"
          # DEC: decrement current cell by 1
          ir_ids.concat(emit_cell_mutation(-1))

        when "."
          # OUTPUT: write current cell to stdout via syscall
          # Step 1: load current cell value into v2
          id1 = emit(IR::IrOp::LOAD_BYTE,
                     IR::IrRegister.new(REG_TEMP),
                     IR::IrRegister.new(REG_TAPE_BASE),
                     IR::IrRegister.new(REG_TAPE_PTR))
          ir_ids << id1
          # Step 2: copy to the syscall argument register without v6.
          id2 = emit(IR::IrOp::ADD_IMM,
                     IR::IrRegister.new(REG_SYS_ARG),
                     IR::IrRegister.new(REG_TEMP),
                     IR::IrImmediate.new(0))
          ir_ids << id2
          # Step 3: syscall 1 = write byte
          id3 = emit(IR::IrOp::SYSCALL, IR::IrImmediate.new(SYSCALL_WRITE))
          ir_ids << id3

        when ","
          # INPUT: read byte from stdin into current cell
          # Step 1: syscall 2 = read byte (result lands in syscall arg register)
          id1 = emit(IR::IrOp::SYSCALL, IR::IrImmediate.new(SYSCALL_READ))
          ir_ids << id1
          # Step 2: store syscall result into current cell
          id2 = emit(IR::IrOp::STORE_BYTE,
                     IR::IrRegister.new(REG_SYS_ARG),
                     IR::IrRegister.new(REG_TAPE_BASE),
                     IR::IrRegister.new(REG_TAPE_PTR))
          ir_ids << id2

        else
          raise "unknown command token: #{tok.value.inspect}"
        end

        @source_map.ast_to_ir.add(ast_node_id, ir_ids)
      end

      # emit_cell_mutation(delta) → Array<Integer>
      #
      # Emits the IR for incrementing or decrementing the current cell.
      # Returns the IDs of the emitted instructions.
      #
      # Sequence:
      #   LOAD_BYTE  v2, v0, v1        ← load current cell
      #   ADD_IMM    v2, v2, delta      ← apply delta (+1 or -1)
      #   AND_IMM    v2, v2, 255        ← mask to byte range (if enabled)
      #   STORE_BYTE v2, v0, v1        ← write back to cell
      def emit_cell_mutation(delta)
        ids = []

        ids << emit(IR::IrOp::LOAD_BYTE,
                    IR::IrRegister.new(REG_TEMP),
                    IR::IrRegister.new(REG_TAPE_BASE),
                    IR::IrRegister.new(REG_TAPE_PTR))

        ids << emit(IR::IrOp::ADD_IMM,
                    IR::IrRegister.new(REG_TEMP),
                    IR::IrRegister.new(REG_TEMP),
                    IR::IrImmediate.new(delta))

        if @config.mask_byte_arithmetic
          ids << emit(IR::IrOp::AND_IMM,
                      IR::IrRegister.new(REG_TEMP),
                      IR::IrRegister.new(REG_TEMP),
                      IR::IrImmediate.new(255))
        end

        ids << emit(IR::IrOp::STORE_BYTE,
                    IR::IrRegister.new(REG_TEMP),
                    IR::IrRegister.new(REG_TAPE_BASE),
                    IR::IrRegister.new(REG_TAPE_PTR))

        ids
      end

      # ── Bounds checking ───────────────────────────────────────────────────
      #
      # In debug builds, the compiler inserts range checks before every
      # pointer move. If the pointer goes out of bounds, the program jumps to
      # the __trap_oob label (which calls exit(1)).
      #
      # RIGHT (>):
      #   CMP_GT    v3, v1, v5        ← is ptr > max_ptr (tape_size-1)?
      #   BRANCH_NZ v3, __trap_oob    ← if so, trap
      #
      # LEFT (<):
      #   CMP_LT    v1, v1, v6        ← is ptr < 0?
      #   BRANCH_NZ v1, __trap_oob    ← if so, trap
      #
      # Note: The left check uses v1 (tape ptr) as the comparison destination
      # register, matching the Go implementation exactly.

      def emit_bounds_check_right
        ids = []
        ids << emit(IR::IrOp::CMP_GT,
                    IR::IrRegister.new(REG_TEMP2),
                    IR::IrRegister.new(REG_TAPE_PTR),
                    IR::IrRegister.new(REG_MAX_PTR))
        ids << emit(IR::IrOp::BRANCH_NZ,
                    IR::IrRegister.new(REG_TEMP2),
                    IR::IrLabel.new("__trap_oob"))
        ids
      end

      def emit_bounds_check_left
        ids = []
        ids << emit(IR::IrOp::CMP_LT,
                    IR::IrRegister.new(REG_TAPE_PTR),
                    IR::IrRegister.new(REG_TAPE_PTR),
                    IR::IrRegister.new(REG_ZERO))
        ids << emit(IR::IrOp::BRANCH_NZ,
                    IR::IrRegister.new(REG_TAPE_PTR),
                    IR::IrLabel.new("__trap_oob"))
        ids
      end

      # ── Loop compilation ──────────────────────────────────────────────────
      #
      # A Brainfuck loop [body] compiles to:
      #
      #   LABEL      loop_N_start
      #   LOAD_BYTE  v2, v0, v1          ← load current cell
      #   BRANCH_Z   v2, loop_N_end      ← skip body if cell == 0
      #   ...compiled body...
      #   JUMP       loop_N_start        ← repeat
      #   LABEL      loop_N_end
      #
      # Loops nest arbitrarily deep. Each loop gets a unique counter N
      # (from @loop_count) to make its labels globally unique.

      def compile_loop(node)
        loop_num = @loop_count
        @loop_count += 1
        start_label = "loop_#{loop_num}_start"
        end_label   = "loop_#{loop_num}_end"

        # Source map entry for the loop construct (maps to [ position)
        ast_node_id = next_node_id
        if node.start_line
          @source_map.source_to_ast.add(
            SM::SourcePosition.new(
              file: @filename,
              line: node.start_line,
              column: node.start_column,
              length: 1
            ),
            ast_node_id
          )
        end

        ir_ids = []

        emit_label(start_label)

        # Load the current cell; branch if zero → skip loop body
        id = emit(IR::IrOp::LOAD_BYTE,
                  IR::IrRegister.new(REG_TEMP),
                  IR::IrRegister.new(REG_TAPE_BASE),
                  IR::IrRegister.new(REG_TAPE_PTR))
        ir_ids << id

        id = emit(IR::IrOp::BRANCH_Z,
                  IR::IrRegister.new(REG_TEMP),
                  IR::IrLabel.new(end_label))
        ir_ids << id

        # Compile the loop body — skip bracket tokens, recurse on instruction nodes
        node.children.each do |child|
          next unless child.is_a?(CodingAdventures::Parser::ASTNode)

          compile_node(child)
        end

        # Jump back to check the loop condition again
        id = emit(IR::IrOp::JUMP, IR::IrLabel.new(start_label))
        ir_ids << id

        emit_label(end_label)

        @source_map.ast_to_ir.add(ast_node_id, ir_ids)
      end

      # ── Token extraction ─────────────────────────────────────────────────
      #
      # The "command" AST node wraps a single leaf token. This helper digs
      # through the AST to find it, handling both direct leaf nodes and
      # nodes that have a single token child.

      def extract_token(node)
        # Direct leaf: node with one child that is a Token
        if node.leaf?
          return node.token
        end

        # Search children for a Token or a leaf ASTNode
        node.children.each do |child|
          case child
          when CodingAdventures::Lexer::Token
            return child
          when CodingAdventures::Parser::ASTNode
            tok = extract_token(child)
            return tok if tok
          end
        end

        nil
      end
    end
  end
end
