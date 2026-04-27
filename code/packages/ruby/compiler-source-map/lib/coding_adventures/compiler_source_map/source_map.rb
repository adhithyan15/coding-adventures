# frozen_string_literal: true

# ==========================================================================
# Source Map Chain — Pipeline Sidecar for AOT Compiler Debugging
# ==========================================================================
#
# A source map chain answers the question "where in the source did THIS
# machine code instruction come from?" at every stage of the pipeline.
#
# Why a "chain" and not a flat table?
# ─────────────────────────────────────────────────────────────────────────
# A flat table (machine-code offset → source position) works for the final
# consumer (a debugger, profiler, or error reporter). But it doesn't help
# when you need to debug the *compiler itself*:
#
#   "Why did the optimiser delete instruction #42?"
#     → Look at the IrToIr segment for that pass.
#
#   "Which AST node produced this IR instruction?"
#     → Look at AstToIr.
#
#   "The machine code for this instruction seems wrong — what IR produced it?"
#     → Look at IrToMachineCode in reverse.
#
# The chain makes the compiler pipeline transparent at every stage.
#
# ── Segment overview ────────────────────────────────────────────────────────
#
#   Segment 1: SourceToAst    — source text position  → AST node ID
#   Segment 2: AstToIr        — AST node ID           → IR instruction IDs
#   Segment 3: IrToIr         — IR instruction ID     → optimised IR IDs
#                               (one segment per optimiser pass)
#   Segment 4: IrToMachineCode — IR instruction ID    → machine code byte offset + length
#
#   Composite:  source position → machine code offset  (forward)
#               machine code offset → source position   (reverse)
# ==========================================================================

module CodingAdventures
  module CompilerSourceMap
    # ──────────────────────────────────────────────────────────────────────────
    # SourcePosition — a span of characters in a source file
    #
    # Think of this as a "highlighter pen" marking a region of source code.
    # The (line, column) pair marks the start; length tells you how many
    # characters are highlighted.
    #
    # For Brainfuck, every command is exactly one character (length = 1).
    # For BASIC, a keyword like "PRINT" would have length = 5.
    #
    # All positions are 1-based (line 1 = first line, column 1 = first char).
    # ──────────────────────────────────────────────────────────────────────────
    SourcePosition = Struct.new(:file, :line, :column, :length, keyword_init: true) do
      # Returns a human-readable string like "hello.bf:1:3 (len=1)".
      def to_s
        "#{file}:#{line}:#{column} (len=#{length})"
      end
    end

    # ──────────────────────────────────────────────────────────────────────────
    # SourceToAstEntry — one mapping from a source position to an AST node ID
    #
    # Example: The "+" character at line 1, column 3 of "hello.bf" maps
    # to AST node #42 (a command(INC) node in the parse tree).
    # ──────────────────────────────────────────────────────────────────────────
    SourceToAstEntry = Struct.new(:pos, :ast_node_id, keyword_init: true)

    # ──────────────────────────────────────────────────────────────────────────
    # SourceToAst — Segment 1: source text positions → AST node IDs
    #
    # Produced by the parser or language-specific frontend. Maps every
    # meaningful source position to the AST node that represents it.
    # ──────────────────────────────────────────────────────────────────────────
    class SourceToAst
      attr_reader :entries

      def initialize
        @entries = []
      end

      # add(pos, ast_node_id) records a mapping from a source position to
      # an AST node ID.
      #
      # @param pos [SourcePosition] the source location
      # @param ast_node_id [Integer] the AST node identifier
      def add(pos, ast_node_id)
        @entries << SourceToAstEntry.new(pos: pos, ast_node_id: ast_node_id)
      end

      # lookup_by_node_id(id) → SourcePosition or nil
      #
      # Returns the source position for the given AST node ID, or nil if
      # not found. This is used for reverse lookups when tracing back from
      # IR to source.
      #
      # @param ast_node_id [Integer] the AST node to look up
      # @return [SourcePosition, nil]
      def lookup_by_node_id(ast_node_id)
        entry = @entries.find { |e| e.ast_node_id == ast_node_id }
        entry&.pos
      end
    end

    # ──────────────────────────────────────────────────────────────────────────
    # AstToIrEntry — one mapping from an AST node to the IR instructions it produced
    #
    # A single AST node often produces multiple IR instructions. For example,
    # a Brainfuck "+" command produces four instructions:
    #   LOAD_BYTE, ADD_IMM, AND_IMM, STORE_BYTE
    # So the mapping is one-to-many: ast_node_42 → [ir_7, ir_8, ir_9, ir_10].
    # ──────────────────────────────────────────────────────────────────────────
    AstToIrEntry = Struct.new(:ast_node_id, :ir_ids, keyword_init: true)

    # ──────────────────────────────────────────────────────────────────────────
    # AstToIr — Segment 2: AST node IDs → IR instruction IDs
    # ──────────────────────────────────────────────────────────────────────────
    class AstToIr
      attr_reader :entries

      def initialize
        @entries = []
      end

      # add(ast_node_id, ir_ids) records that the given AST node produced
      # the given IR instruction IDs.
      #
      # @param ast_node_id [Integer] the AST node identifier
      # @param ir_ids [Array<Integer>] the IR instruction IDs produced
      def add(ast_node_id, ir_ids)
        @entries << AstToIrEntry.new(ast_node_id: ast_node_id, ir_ids: ir_ids)
      end

      # lookup_by_ast_node_id(id) → Array<Integer> or nil
      #
      # Returns the IR instruction IDs for the given AST node, or nil if
      # not found.
      #
      # @param ast_node_id [Integer]
      # @return [Array<Integer>, nil]
      def lookup_by_ast_node_id(ast_node_id)
        entry = @entries.find { |e| e.ast_node_id == ast_node_id }
        entry&.ir_ids
      end

      # lookup_by_ir_id(ir_id) → Integer
      #
      # Returns the AST node ID that produced the given IR instruction,
      # or -1 if not found. Used for reverse lookups during debugging.
      #
      # @param ir_id [Integer]
      # @return [Integer] the AST node ID, or -1
      def lookup_by_ir_id(ir_id)
        @entries.each do |entry|
          return entry.ast_node_id if entry.ir_ids.include?(ir_id)
        end
        -1
      end
    end

    # ──────────────────────────────────────────────────────────────────────────
    # IrToIrEntry — one mapping from an original IR instruction to its
    # replacement(s) after an optimiser pass
    #
    # Three cases:
    #   1. Preserved:  original_id → [same_id]        (instruction unchanged)
    #   2. Replaced:   original_id → [new_id_1, ...]  (instruction split/transformed)
    #   3. Deleted:    original_id is in deleted set   (instruction optimised away)
    #
    # Example: A contraction pass folds three ADD_IMM 1 instructions
    # (IDs 7, 8, 9) into one ADD_IMM 3 (ID 100):
    #   7 → [100], 8 → [100], 9 → [100]
    # ──────────────────────────────────────────────────────────────────────────
    IrToIrEntry = Struct.new(:original_id, :new_ids, keyword_init: true)

    # ──────────────────────────────────────────────────────────────────────────
    # IrToIr — Segment 3: IR instruction IDs → optimised IR instruction IDs
    #
    # One segment is produced per optimiser pass. The pass_name attribute
    # identifies which pass produced this mapping (e.g., "identity",
    # "contraction", "clear_loop", "dead_store").
    # ──────────────────────────────────────────────────────────────────────────
    class IrToIr
      attr_reader :entries, :deleted, :pass_name

      def initialize(pass_name)
        @pass_name = pass_name
        @entries = []
        @deleted = {}
      end

      # add_mapping(original_id, new_ids) records that the original instruction
      # was replaced by the new ones.
      #
      # @param original_id [Integer]
      # @param new_ids [Array<Integer>]
      def add_mapping(original_id, new_ids)
        @entries << IrToIrEntry.new(original_id: original_id, new_ids: new_ids)
      end

      # add_deletion(original_id) records that the original instruction was
      # deleted by this optimiser pass.
      #
      # @param original_id [Integer]
      def add_deletion(original_id)
        @deleted[original_id] = true
        @entries << IrToIrEntry.new(original_id: original_id, new_ids: nil)
      end

      # lookup_by_original_id(id) → Array<Integer> or nil
      #
      # Returns the new IDs for the given original ID, or nil if deleted or
      # not found.
      #
      # @param original_id [Integer]
      # @return [Array<Integer>, nil]
      def lookup_by_original_id(original_id)
        return nil if @deleted[original_id]

        entry = @entries.find { |e| e.original_id == original_id }
        entry&.new_ids
      end

      # lookup_by_new_id(new_id) → Integer
      #
      # Returns the original ID that produced the given new ID, or -1 if
      # not found. When multiple originals map to the same new ID (contraction),
      # returns the first one found.
      #
      # @param new_id [Integer]
      # @return [Integer] the original ID, or -1
      def lookup_by_new_id(new_id)
        @entries.each do |entry|
          next if entry.new_ids.nil?

          return entry.original_id if entry.new_ids.include?(new_id)
        end
        -1
      end
    end

    # ──────────────────────────────────────────────────────────────────────────
    # IrToMachineCodeEntry — one mapping from an IR instruction to the machine
    # code bytes it produced
    #
    # Each entry is a triple: (ir_id, mc_offset, mc_length).
    # For example, a LOAD_BYTE IR instruction might produce 8 bytes of RISC-V
    # machine code starting at offset 0x14 in the .text section.
    # ──────────────────────────────────────────────────────────────────────────
    IrToMachineCodeEntry = Struct.new(:ir_id, :mc_offset, :mc_length, keyword_init: true)

    # ──────────────────────────────────────────────────────────────────────────
    # IrToMachineCode — Segment 4: IR instruction IDs → machine code byte offsets
    # ──────────────────────────────────────────────────────────────────────────
    class IrToMachineCode
      attr_reader :entries

      def initialize
        @entries = []
      end

      # add(ir_id, mc_offset, mc_length) records that the given IR instruction
      # produced machine code at the given offset with the given byte length.
      #
      # @param ir_id [Integer]
      # @param mc_offset [Integer] byte offset in the .text section
      # @param mc_length [Integer] number of bytes of machine code emitted
      def add(ir_id, mc_offset, mc_length)
        @entries << IrToMachineCodeEntry.new(ir_id: ir_id, mc_offset: mc_offset, mc_length: mc_length)
      end

      # lookup_by_ir_id(ir_id) → [offset, length] or [-1, 0]
      #
      # Returns [offset, length] for the given IR instruction ID, or [-1, 0]
      # if not found.
      #
      # @param ir_id [Integer]
      # @return [Array(Integer, Integer)]
      def lookup_by_ir_id(ir_id)
        entry = @entries.find { |e| e.ir_id == ir_id }
        entry ? [entry.mc_offset, entry.mc_length] : [-1, 0]
      end

      # lookup_by_mc_offset(offset) → Integer
      #
      # Returns the IR instruction ID whose machine code contains the given
      # byte offset, or -1 if not found.
      #
      # An entry "contains" an offset if:
      #   entry.mc_offset <= offset < entry.mc_offset + entry.mc_length
      #
      # @param offset [Integer] byte offset in the .text section
      # @return [Integer] the IR instruction ID, or -1
      def lookup_by_mc_offset(offset)
        entry = @entries.find do |e|
          offset >= e.mc_offset && offset < e.mc_offset + e.mc_length
        end
        entry ? entry.ir_id : -1
      end
    end

    # ──────────────────────────────────────────────────────────────────────────
    # SourceMapChain — the full pipeline sidecar
    #
    # This is the central data structure that flows through every stage of the
    # compiler pipeline. Each stage reads the existing segments and appends its
    # own:
    #
    #   1. Frontend (brainfuck-ir-compiler) → fills source_to_ast + ast_to_ir
    #   2. Optimiser (compiler-ir-optimizer) → appends ir_to_ir segments
    #   3. Backend (codegen-riscv) → fills ir_to_machine_code
    # ──────────────────────────────────────────────────────────────────────────
    class SourceMapChain
      attr_accessor :source_to_ast, :ast_to_ir, :ir_to_ir, :ir_to_machine_code

      def initialize
        @source_to_ast = SourceToAst.new
        @ast_to_ir = AstToIr.new
        @ir_to_ir = []
        @ir_to_machine_code = nil
      end

      # add_optimizer_pass(segment) appends an IrToIr segment from an optimiser
      # pass. Optimiser passes are applied in order; the chain preserves them.
      #
      # @param segment [IrToIr] the segment produced by one optimiser pass
      def add_optimizer_pass(segment)
        @ir_to_ir << segment
      end

      # source_to_mc(pos) → Array<IrToMachineCodeEntry>
      #
      # Composite forward query: source position → machine code offsets.
      # Returns an empty array if the chain is incomplete or no mapping exists.
      #
      # Algorithm:
      #   1. SourceToAst:   source position → AST node ID
      #   2. AstToIr:       AST node ID → IR instruction IDs
      #   3. IrToIr (each): follow IR IDs through each optimiser pass
      #   4. IrToMachineCode: final IR IDs → machine code offsets
      #
      # @param pos [SourcePosition]
      # @return [Array<IrToMachineCodeEntry>]
      def source_to_mc(pos)
        return [] if @ir_to_machine_code.nil?

        # Step 1: source → AST node
        ast_node_id = -1
        @source_to_ast.entries.each do |entry|
          if entry.pos.file == pos.file &&
             entry.pos.line == pos.line &&
             entry.pos.column == pos.column
            ast_node_id = entry.ast_node_id
            break
          end
        end
        return [] if ast_node_id == -1

        # Step 2: AST node → IR IDs
        ir_ids = @ast_to_ir.lookup_by_ast_node_id(ast_node_id)
        return [] if ir_ids.nil?

        # Step 3: follow through optimiser passes
        current_ids = ir_ids.dup
        @ir_to_ir.each do |pass|
          next_ids = []
          current_ids.each do |id|
            next if pass.deleted[id]

            new_ids = pass.lookup_by_original_id(id)
            next_ids.concat(new_ids) if new_ids
          end
          current_ids = next_ids
        end

        return [] if current_ids.empty?

        # Step 4: IR IDs → machine code entries
        results = []
        current_ids.each do |id|
          offset, length = @ir_to_machine_code.lookup_by_ir_id(id)
          if offset >= 0
            results << IrToMachineCodeEntry.new(ir_id: id, mc_offset: offset, mc_length: length)
          end
        end
        results
      end

      # mc_to_source(mc_offset) → SourcePosition or nil
      #
      # Composite reverse query: machine code offset → source position.
      # Returns nil if the chain is incomplete or no mapping exists.
      #
      # Algorithm (reverse of source_to_mc):
      #   1. IrToMachineCode: MC offset → IR instruction ID
      #   2. IrToIr (each pass, in reverse): trace IR ID back through passes
      #   3. AstToIr: IR ID → AST node ID
      #   4. SourceToAst: AST node ID → source position
      #
      # @param mc_offset [Integer] byte offset in the .text section
      # @return [SourcePosition, nil]
      def mc_to_source(mc_offset)
        return nil if @ir_to_machine_code.nil?

        # Step 1: MC offset → IR ID
        ir_id = @ir_to_machine_code.lookup_by_mc_offset(mc_offset)
        return nil if ir_id == -1

        # Step 2: trace back through optimiser passes (reverse order)
        current_id = ir_id
        @ir_to_ir.reverse_each do |pass|
          original_id = pass.lookup_by_new_id(current_id)
          return nil if original_id == -1

          current_id = original_id
        end

        # Step 3: IR ID → AST node ID
        ast_node_id = @ast_to_ir.lookup_by_ir_id(current_id)
        return nil if ast_node_id == -1

        # Step 4: AST node ID → source position
        @source_to_ast.lookup_by_node_id(ast_node_id)
      end
    end
  end
end
