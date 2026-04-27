# frozen_string_literal: true

# ==========================================================================
# IrPrinter — IrProgram → Human-Readable Text
# ==========================================================================
#
# The printer converts an IrProgram into its canonical text format.
# This format serves three purposes:
#
#   1. Debugging — humans can read the IR to understand what the compiler did
#   2. Golden-file tests — expected IR output is committed as .ir text files
#   3. Roundtrip — parse(print(program)) == program is a testable invariant
#
# ── Text Format ─────────────────────────────────────────────────────────────
#
#   .version 1
#
#   .data tape 30000 0
#
#   .entry _start
#
#   _start:
#     LOAD_ADDR   v0, tape          ; #0
#     LOAD_IMM    v1, 0             ; #1
#     HALT                          ; #2
#
# Key rules:
#   - .version N is always the first non-comment line
#   - .data declarations come before .entry
#   - Labels are on their own unindented line with a trailing colon
#   - Instructions are indented with two spaces
#   - ; #N comments show instruction IDs (informational only)
#   - COMMENT instructions emit as "; <text>" on their own indented line
#
# The opcode field is left-padded to 11 characters so that operands align
# into a readable column. This matches the Go printer's "%-11s" format.
# ==========================================================================

module CodingAdventures
  module CompilerIr
    module IrPrinter
      # print(program) → String
      #
      # Converts an IrProgram to its canonical text representation.
      # The returned string contains the full .ir file content including
      # the version directive, data declarations, entry point, and all
      # instructions with their ID comments.
      #
      # @param program [IrProgram] the program to print
      # @return [String] the canonical IR text
      def self.print(program)
        parts = []

        # Version directive — always the first line.
        parts << ".version #{program.version}"

        # Data declarations — one per line, blank line before each.
        program.data.each do |decl|
          parts << ""
          parts << ".data #{decl.label} #{decl.size} #{decl.init}"
        end

        # Entry point — blank line before.
        parts << ""
        parts << ".entry #{program.entry_label}"

        # Instructions — each on its own line.
        program.instructions.each do |instr|
          case instr.opcode
          when IrOp::LABEL
            # Labels get their own unindented line with a trailing colon.
            # A blank line before each label visually separates basic blocks.
            parts << ""
            parts << "#{instr.operands[0]}:"

          when IrOp::COMMENT
            # COMMENT instructions emit as "  ; <text>" — indented like
            # regular instructions but without an opcode mnemonic.
            text = instr.operands[0]&.to_s || ""
            parts << "  ; #{text}"

          else
            # Regular instruction: "  OPCODE       operands  ; #ID"
            #
            # The opcode mnemonic is left-aligned in an 11-character field
            # so that operands form a readable column. The %-11s format in
            # Ruby's sprintf matches Go's fmt.Sprintf("%-11s", ...).
            mnemonic = IrOp.op_name(instr.opcode)
            operand_str = instr.operands.map(&:to_s).join(", ")
            line = format("  %-11s", mnemonic)
            line += operand_str unless operand_str.empty?
            line += "  ; ##{instr.id}"
            parts << line
          end
        end

        # Join all parts with newlines and add a trailing newline.
        parts.join("\n") + "\n"
      end
    end
  end
end
