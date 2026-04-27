# frozen_string_literal: true

# ==========================================================================
# IrParser — Text → IrProgram
# ==========================================================================
#
# The parser reads the canonical IR text format (produced by IrPrinter) and
# reconstructs an IrProgram. This enables:
#
#   1. Golden-file testing — load an expected .ir file, parse it, compare
#   2. Roundtrip verification — parse(print(program)) == program
#   3. Manual IR authoring — write IR by hand for testing backends
#
# ── Parsing strategy ────────────────────────────────────────────────────────
#
# The parser processes the text line by line:
#
#   1. Lines starting with ".version" set the program version
#   2. Lines starting with ".data" add a data declaration
#   3. Lines starting with ".entry" set the entry label
#   4. Lines ending with ":" define a label
#   5. Lines starting with whitespace are instructions
#   6. Lines starting with ";" are standalone comments
#   7. Blank lines are skipped
#
# Each instruction line is split into: opcode, operands, and optional
# "; #N" ID comment. Operands are parsed as registers (v0, v1, ...),
# immediates (42, -1), or labels (any other identifier).
#
# ── Safety limits ───────────────────────────────────────────────────────────
#
# The parser enforces limits to prevent denial-of-service from large inputs:
#   MAX_LINES            = 1_000_000
#   MAX_OPERANDS_PER_INSTR = 16
#   MAX_REGISTER_INDEX   = 65_535
# ==========================================================================

module CodingAdventures
  module CompilerIr
    module IrParser
      # Maximum number of lines in an IR text file.
      MAX_LINES = 1_000_000

      # Maximum number of operands per instruction.
      MAX_OPERANDS_PER_INSTR = 16

      # Maximum virtual register index (v0 .. v65535).
      MAX_REGISTER_INDEX = 65_535

      # parse(text) → IrProgram
      #
      # Converts IR text into an IrProgram.
      # Raises RuntimeError if the text is malformed.
      #
      # @param text [String] canonical IR text produced by IrPrinter
      # @return [IrProgram] the reconstructed program
      # @raise [RuntimeError] if the text is malformed
      def self.parse(text)
        program = IrProgram.new("")
        program.version = 1

        lines = text.split("\n", -1)
        raise "input too large: #{lines.length} lines (max #{MAX_LINES})" if lines.length > MAX_LINES

        lines.each_with_index do |line, idx|
          line_num = idx + 1
          trimmed = line.strip

          # Skip blank lines
          next if trimmed.empty?

          # Version directive: ".version N"
          if trimmed.start_with?(".version")
            parts = trimmed.split
            raise "line #{line_num}: invalid .version directive: #{line.inspect}" unless parts.length == 2

            v = Integer(parts[1])
            program.version = v
            next
          end

          # Data declaration: ".data label size init"
          if trimmed.start_with?(".data")
            parts = trimmed.split
            raise "line #{line_num}: invalid .data directive: #{line.inspect}" unless parts.length == 4

            size = Integer(parts[2])
            init = Integer(parts[3])
            program.add_data(IrDataDecl.new(parts[1], size, init))
            next
          end

          # Entry point: ".entry label"
          if trimmed.start_with?(".entry")
            parts = trimmed.split
            raise "line #{line_num}: invalid .entry directive: #{line.inspect}" unless parts.length == 2

            program.entry_label = parts[1]
            next
          end

          # Label definition: ends with ":" and does not start with ";"
          if trimmed.end_with?(":") && !trimmed.start_with?(";")
            label_name = trimmed.chomp(":")
            program.add_instruction(IrInstruction.new(IrOp::LABEL, [IrLabel.new(label_name)], -1))
            next
          end

          # Standalone comment line: starts with ";"
          if trimmed.start_with?(";")
            comment_text = trimmed[1..].strip
            # Only treat as a COMMENT instruction if it's not just an ID comment
            unless comment_text.start_with?("#")
              program.add_instruction(IrInstruction.new(IrOp::COMMENT, [IrLabel.new(comment_text)], -1))
            end
            next
          end

          # Instruction line (starts with whitespace in canonical format,
          # but we work with the stripped version here)
          instr = parse_instruction_line(trimmed, line_num)
          program.add_instruction(instr)
        end

        program
      end

      # parse_instruction_line(line, line_num) → IrInstruction
      #
      # Parses a single (already-stripped) instruction line like:
      #   "LOAD_IMM   v0, 42  ; #3"
      #
      # @param line [String] the trimmed instruction text
      # @param line_num [Integer] 1-based line number for error messages
      # @return [IrInstruction] the parsed instruction
      def self.parse_instruction_line(line, line_num)
        # Split off the "; #N" ID comment if present.
        # We scan from the right so that operand text like "; comment_label"
        # is not confused with the ID comment.
        id = -1
        instruction_part = line
        if (idx = line.rindex("; #"))
          id_str = line[(idx + 3)..].strip
          parsed_id = Integer(id_str, exception: false)
          if parsed_id
            id = parsed_id
            instruction_part = line[...idx].strip
          end
        end

        # Split into opcode and operand text
        fields = instruction_part.split
        raise "line #{line_num}: empty instruction" if fields.empty?

        opcode_name = fields[0]
        opcode = IrOp.parse_op(opcode_name)
        raise "line #{line_num}: unknown opcode #{opcode_name.inspect}" unless opcode

        # Parse operands (everything after the opcode, comma-separated).
        operands = []
        if fields.length > 1
          # Rejoin remaining fields and split by comma to handle "v0, v1, 42" format
          operand_str = fields[1..].join(" ")
          parts = operand_str.split(",")
          if parts.length > MAX_OPERANDS_PER_INSTR
            raise "line #{line_num}: too many operands (#{parts.length}, max #{MAX_OPERANDS_PER_INSTR})"
          end

          parts.each do |part|
            part = part.strip
            next if part.empty?

            operands << parse_operand(part, line_num)
          end
        end

        IrInstruction.new(opcode, operands, id)
      end

      # parse_operand(s, line_num) → IrRegister | IrImmediate | IrLabel
      #
      # Parses a single operand string into an operand object.
      #
      # Parsing rules (checked in order):
      #   1. Starts with "v" followed by only digits → IrRegister{index: N}
      #   2. Parseable as a decimal integer (with optional leading "-") → IrImmediate{value: N}
      #   3. Anything else → IrLabel{name: str}
      #
      # @param s [String] the operand text
      # @param line_num [Integer] 1-based line number for error messages
      # @return [IrRegister | IrImmediate | IrLabel] the parsed operand
      def self.parse_operand(s, line_num)
        # Register: "v" followed by one or more digits
        if s.length > 1 && s[0] == "v" && s[1..] =~ /\A\d+\z/
          idx = s[1..].to_i
          if idx < 0 || idx > MAX_REGISTER_INDEX
            raise "line #{line_num}: register index #{idx} out of range (max #{MAX_REGISTER_INDEX})"
          end

          return IrRegister.new(idx)
        end

        # Immediate: an optional leading "-" followed by digits
        int_val = Integer(s, exception: false)
        return IrImmediate.new(int_val) unless int_val.nil?

        # Label: any other identifier
        IrLabel.new(s)
      end

    end
  end
end
