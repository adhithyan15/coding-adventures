# frozen_string_literal: true

# ==========================================================================
# Brainfuck Translator — Source Code to Bytecode in One Pass
# ==========================================================================
#
# Why "Translator" and not "Compiler"?
# ==========================================================================
#
# A compiler transforms a high-level structured representation (an AST)
# into lower-level instructions. It handles scoping, type checking, operator
# precedence, and all the complexity that comes with real languages.
#
# Brainfuck doesn't have any of that. There's no AST, no scoping, no types.
# Each source character maps directly to one instruction. The only non-trivial
# step is bracket matching — connecting [ to its matching ] so the VM knows
# where to jump.
#
# So we call this a "translator" rather than a "compiler": it translates
# characters to opcodes, with bracket matching as the sole transformation.
#
# ==========================================================================
# How Bracket Matching Works
# ==========================================================================
#
# Bracket matching is a classic stack problem:
#
# 1. Scan the source left to right.
# 2. When we see [, emit a LOOP_START with a placeholder target (0),
#    and push its instruction index onto a stack.
# 3. When we see ], pop the matching [ index from the stack.
#    - Patch the [ instruction to jump to one past the current ].
#    - Emit a LOOP_END that jumps back to the [.
# 4. After scanning, if the stack isn't empty, we have unmatched [.
#
# ==========================================================================
# Example
# ==========================================================================
#
# Source: ++[>+<-]
#
# Translation:
#
#   Index  Opcode       Operand   Source
#   ─────────────────────────────────────
#   0      INC          —         +
#   1      INC          —         +
#   2      LOOP_START   8         [  (jump to 8 if cell==0)
#   3      RIGHT        —         >
#   4      INC          —         +
#   5      LEFT         —         <
#   6      DEC          —         -
#   7      LOOP_END     2         ]  (jump to 2 if cell!=0)
#   8      HALT         —         (end)
# ==========================================================================

module CodingAdventures
  module Brainfuck
    # Raised when the Brainfuck source has mismatched brackets.
    class TranslationError < StandardError; end

    # Translate Brainfuck source code into a CodeObject.
    #
    # Each BF character maps to one instruction. Non-BF characters are
    # ignored (they're comments). Bracket matching resolves jump targets.
    #
    # @param source [String] the Brainfuck program
    # @return [CodingAdventures::VirtualMachine::CodeObject]
    # @raise [TranslationError] if brackets are mismatched
    #
    # @example
    #   code = CodingAdventures::Brainfuck.translate("+++.")
    #   code.instructions.length  #=> 5  (3 INCs + 1 OUTPUT + 1 HALT)
    #   code.constants             #=> []
    #   code.names                 #=> []
    #
    def self.translate(source)
      instructions = []
      bracket_stack = []

      source.each_char do |char|
        op = CHAR_TO_OP[char]
        next if op.nil?  # Not a BF command — skip (it's a comment)

        if op == Op::LOOP_START
          # Emit LOOP_START with placeholder operand (will be patched)
          index = instructions.length
          instructions << VirtualMachine::Instruction.new(opcode: Op::LOOP_START, operand: 0)
          bracket_stack.push(index)

        elsif op == Op::LOOP_END
          if bracket_stack.empty?
            raise TranslationError, "Unmatched ']' — no matching '[' found"
          end

          # Pop the matching [ index
          start_index = bracket_stack.pop

          # The LOOP_END instruction index
          end_index = instructions.length

          # Patch LOOP_START to jump past LOOP_END (end_index + 1)
          instructions[start_index] = VirtualMachine::Instruction.new(
            opcode: Op::LOOP_START, operand: end_index + 1
          )

          # Emit LOOP_END that jumps back to LOOP_START
          instructions << VirtualMachine::Instruction.new(
            opcode: Op::LOOP_END, operand: start_index
          )

        else
          # Simple command — no operand needed
          instructions << VirtualMachine::Instruction.new(opcode: op)
        end
      end

      unless bracket_stack.empty?
        if bracket_stack.length == 1
          raise TranslationError, "Unmatched '[' — no matching ']' found"
        else
          raise TranslationError,
            "Unmatched '[' — #{bracket_stack.length} unclosed bracket(s)"
        end
      end

      # Append HALT
      instructions << VirtualMachine::Instruction.new(opcode: Op::HALT)

      VirtualMachine::CodeObject.new(
        instructions: instructions,
        constants: [],
        names: []
      )
    end
  end
end
