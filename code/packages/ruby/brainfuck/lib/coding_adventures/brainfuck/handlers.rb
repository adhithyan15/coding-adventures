# frozen_string_literal: true

# ==========================================================================
# Brainfuck Opcode Handlers — Teaching the GenericVM a New Language
# ==========================================================================
#
# How Handlers Plug Into the GenericVM
# ==========================================================================
#
# The GenericVM is a blank slate — it knows how to fetch-decode-execute
# instructions, but it doesn't know what any opcode means. That's where
# handlers come in.
#
# Each handler is a lambda/proc with the signature:
#
#   ->(vm, instruction, code) { ... }
#
# The handler receives:
#
# - vm          — The GenericVM instance. We use vm.tape, vm.dp,
#                 vm.output, and vm.advance_pc / vm.jump_to.
# - instruction — The current instruction (opcode + optional operand).
# - code        — The CodeObject (unused by most BF handlers).
#
# The handler returns a string if it produces output (the . command),
# otherwise nil.
#
# ==========================================================================
# Brainfuck's Extra State
# ==========================================================================
#
# The GenericVM provides a stack, variables, and locals — none of which
# Brainfuck uses. Instead, Brainfuck needs:
#
# - tape         — An array of 30,000 byte cells, initialized to 0.
# - dp           — Data pointer (index into the tape), starts at 0.
# - input_buffer — String to read from (simulates stdin).
# - input_pos    — Current position in the input buffer.
#
# These are attached as attributes on the GenericVM instance in the
# factory function (create_brainfuck_vm). Ruby's open classes and
# instance_variable_set make this easy.
#
# ==========================================================================
# Cell Wrapping
# ==========================================================================
#
# Brainfuck cells are unsigned bytes: values 0–255. Incrementing 255
# wraps to 0; decrementing 0 wraps to 255. This is modular arithmetic:
#
#   cell = (cell + 1) % 256   # INC
#   cell = (cell - 1) % 256   # DEC
#
# Ruby's % operator handles negative numbers correctly:
# (-1) % 256 == 255. This matches the spec perfectly.
# ==========================================================================

module CodingAdventures
  module Brainfuck
    # The number of cells on the Brainfuck tape.
    #
    # The original Brainfuck specification uses 30,000 cells. Some
    # implementations use more (or even dynamically grow), but 30,000
    # is the classic size.
    TAPE_SIZE = 30_000

    # Runtime error during Brainfuck execution.
    class BrainfuckError < StandardError; end

    # =====================================================================
    # Handler lambdas
    # =====================================================================
    #
    # Each handler is a lambda that takes (vm, instruction, code) and
    # returns a String (for output) or nil.

    # > — Move the data pointer one cell to the right.
    #
    # If the pointer is already at the last cell, this raises an error.
    HANDLE_RIGHT = ->(vm, _instr, _code) {
      vm.dp += 1
      if vm.dp >= TAPE_SIZE
        raise BrainfuckError,
          "Data pointer moved past end of tape (position #{vm.dp}). " \
          "The tape has #{TAPE_SIZE} cells (indices 0–#{TAPE_SIZE - 1})."
      end
      vm.advance_pc
      nil
    }

    # < — Move the data pointer one cell to the left.
    #
    # If the pointer is already at cell 0, this raises an error.
    HANDLE_LEFT = ->(vm, _instr, _code) {
      vm.dp -= 1
      if vm.dp < 0
        raise BrainfuckError,
          "Data pointer moved before start of tape (position -1). " \
          "The tape starts at index 0."
      end
      vm.advance_pc
      nil
    }

    # + — Increment the byte at the data pointer.
    #
    # Wraps from 255 to 0 (unsigned byte arithmetic).
    HANDLE_INC = ->(vm, _instr, _code) {
      vm.tape[vm.dp] = (vm.tape[vm.dp] + 1) % 256
      vm.advance_pc
      nil
    }

    # - — Decrement the byte at the data pointer.
    #
    # Wraps from 0 to 255 (unsigned byte arithmetic).
    HANDLE_DEC = ->(vm, _instr, _code) {
      vm.tape[vm.dp] = (vm.tape[vm.dp] - 1) % 256
      vm.advance_pc
      nil
    }

    # . — Output the current cell's value as an ASCII character.
    HANDLE_OUTPUT = ->(vm, _instr, _code) {
      char = vm.tape[vm.dp].chr
      vm.output << char
      vm.advance_pc
      char
    }

    # , — Read one byte of input into the current cell.
    #
    # If the input is exhausted (EOF), the cell is set to 0.
    HANDLE_INPUT = ->(vm, _instr, _code) {
      if vm.input_pos < vm.input_buffer.length
        vm.tape[vm.dp] = vm.input_buffer[vm.input_pos].ord
        vm.input_pos += 1
      else
        vm.tape[vm.dp] = 0
      end
      vm.advance_pc
      nil
    }

    # [ — Jump forward past the matching ] if the current cell is zero.
    #
    # If the cell is nonzero, execution continues to the next instruction
    # (entering the loop body). If the cell is zero, the VM jumps to the
    # instruction index stored in the operand (one past the matching ]),
    # effectively skipping the loop entirely.
    HANDLE_LOOP_START = ->(vm, instr, _code) {
      if vm.tape[vm.dp] == 0
        vm.jump_to(instr.operand)
      else
        vm.advance_pc
      end
      nil
    }

    # ] — Jump backward to the matching [ if the current cell is nonzero.
    #
    # If the cell is nonzero, jump back to the matching [ (which will
    # re-test the condition). If the cell is zero, fall through to the
    # next instruction (exiting the loop).
    HANDLE_LOOP_END = ->(vm, instr, _code) {
      if vm.tape[vm.dp] != 0
        vm.jump_to(instr.operand)
      else
        vm.advance_pc
      end
      nil
    }

    # Stop the VM.
    HANDLE_HALT = ->(vm, _instr, _code) {
      vm.halted = true
      nil
    }

    # All Brainfuck opcode handlers, keyed by opcode number.
    # Used by create_brainfuck_vm to register all handlers at once.
    HANDLERS = {
      Op::RIGHT => HANDLE_RIGHT,
      Op::LEFT => HANDLE_LEFT,
      Op::INC => HANDLE_INC,
      Op::DEC => HANDLE_DEC,
      Op::OUTPUT => HANDLE_OUTPUT,
      Op::INPUT => HANDLE_INPUT,
      Op::LOOP_START => HANDLE_LOOP_START,
      Op::LOOP_END => HANDLE_LOOP_END,
      Op::HALT => HANDLE_HALT
    }.freeze
  end
end
