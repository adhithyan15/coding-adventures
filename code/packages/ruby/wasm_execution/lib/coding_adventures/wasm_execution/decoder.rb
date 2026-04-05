# frozen_string_literal: true

# ==========================================================================
# WASM Bytecode Decoder --- Bridging Variable-Length to Fixed-Format
# ==========================================================================
#
# WebAssembly bytecodes are *variable-length*: the opcode is 1 byte,
# but immediates can be 1 to 10+ bytes depending on LEB128 encoding.
# GenericVM expects fixed-format Instruction objects with a single
# opcode and optional operand.
#
# The decoder bridges this gap by pre-scanning the entire function body
# and producing an array of fixed-format instructions. Each decoded
# instruction has an opcode, a decoded operand, and metadata about its
# position in the byte stream.
#
# The decoder also builds the *control flow map* --- a lookup table
# mapping each block/loop/if instruction to its matching end (and else).
# ==========================================================================

module CodingAdventures
  module WasmExecution
    # A decoded WASM instruction with its byte offset and size.
    DecodedInstruction = Struct.new(:opcode, :operand, :offset, :size, keyword_init: true)

    # A control flow map entry: records where a block/loop/if ends.
    ControlTarget = Struct.new(:end_pc, :else_pc, keyword_init: true)

    module Decoder
      module_function

      # Decode all instructions in a function body's bytecodes.
      #
      # @param body [FunctionBody] the function body with raw bytecodes
      # @return [Array<DecodedInstruction>]
      def decode_function_body(body)
        code = body.code
        # Ensure we have an array of bytes
        bytes = code.is_a?(String) ? code.bytes : code
        instructions = []
        offset = 0

        while offset < bytes.length
          start_offset = offset
          opcode_byte = bytes[offset]
          offset += 1

          # Look up the opcode metadata to determine immediates.
          info = CodingAdventures::WasmOpcodes.get_opcode(opcode_byte)
          operand = nil

          if info
            operand, consumed = decode_immediates(bytes, offset, info[:immediates])
            offset += consumed
          end

          instructions << DecodedInstruction.new(
            opcode: opcode_byte,
            operand: operand,
            offset: start_offset,
            size: offset - start_offset
          )
        end

        instructions
      end

      # Build the control flow map for a function's decoded instructions.
      #
      # Scans through all instructions and maps each block/loop/if start
      # to its matching end (and else for if instructions). Uses a stack
      # to track nesting.
      #
      # @param instructions [Array<DecodedInstruction>]
      # @return [Hash<Integer, ControlTarget>]
      def build_control_flow_map(instructions)
        map = {}
        stack = [] # Array of { index:, opcode:, else_pc: }

        instructions.each_with_index do |instr, i|
          case instr.opcode
          when 0x02, 0x03, 0x04 # block, loop, if
            stack.push({index: i, opcode: instr.opcode, else_pc: nil})
          when 0x05 # else
            stack.last[:else_pc] = i unless stack.empty?
          when 0x0B # end
            unless stack.empty?
              opener = stack.pop
              map[opener[:index]] = ControlTarget.new(
                end_pc: i,
                else_pc: opener[:else_pc]
              )
            end
          end
        end

        map
      end

      # Convert decoded instructions to GenericVM's Instruction format.
      #
      # @param decoded [Array<DecodedInstruction>]
      # @return [Array<Instruction>]
      def to_vm_instructions(decoded)
        decoded.map do |d|
          CodingAdventures::VirtualMachine::Instruction.new(
            opcode: d.opcode,
            operand: d.operand
          )
        end
      end

      # ── Private Helpers ──────────────────────────────────────────────

      # Decode immediate operands from bytecodes based on the opcode's
      # immediate specification.
      #
      # @param bytes [Array<Integer>] the raw bytes
      # @param offset [Integer] current position after the opcode byte
      # @param immediates [Array<String>] immediate type specs
      # @return [Array(Object, Integer)] [decoded_operand, bytes_consumed]
      def decode_immediates(bytes, offset, immediates)
        return [nil, 0] if immediates.nil? || immediates.empty?

        if immediates.length == 1
          decode_single_immediate(bytes, offset, immediates[0])
        else
          # Multiple immediates — return as a Hash.
          result = {}
          pos = offset
          immediates.each do |imm|
            value, size = decode_single_immediate(bytes, pos, imm)
            result[imm.to_sym] = value
            pos += size
          end
          [result, pos - offset]
        end
      end

      # Decode a single immediate value.
      # @return [Array(Object, Integer)] [value, bytes_consumed]
      def decode_single_immediate(bytes, offset, type)
        case type
        when "i32"
          CodingAdventures::WasmLeb128.decode_signed(bytes, offset)
        when "labelidx", "funcidx", "typeidx", "localidx", "globalidx", "tableidx", "memidx"
          CodingAdventures::WasmLeb128.decode_unsigned(bytes, offset)
        when "i64"
          decode_signed_64(bytes, offset)
        when "f32"
          val = bytes[offset, 4].pack("C*").unpack1("e")
          [val, 4]
        when "f64"
          val = bytes[offset, 8].pack("C*").unpack1("E")
          [val, 8]
        when "blocktype"
          byte = bytes[offset]
          if byte == 0x40 || [0x7F, 0x7E, 0x7D, 0x7C].include?(byte)
            [byte, 1]
          else
            CodingAdventures::WasmLeb128.decode_signed(bytes, offset)
          end
        when "memarg"
          align, align_size = CodingAdventures::WasmLeb128.decode_unsigned(bytes, offset)
          mem_offset, offset_size = CodingAdventures::WasmLeb128.decode_unsigned(bytes, offset + align_size)
          [{align: align, offset: mem_offset}, align_size + offset_size]
        when "vec_labelidx"
          count, count_size = CodingAdventures::WasmLeb128.decode_unsigned(bytes, offset)
          pos = offset + count_size
          labels = []
          count.times do
            label, label_size = CodingAdventures::WasmLeb128.decode_unsigned(bytes, pos)
            labels << label
            pos += label_size
          end
          default_label, default_size = CodingAdventures::WasmLeb128.decode_unsigned(bytes, pos)
          pos += default_size
          [{labels: labels, default_label: default_label}, pos - offset]
        else
          [nil, 0]
        end
      end

      # Decode a signed 64-bit LEB128 value.
      # Returns [value, bytes_consumed].
      def decode_signed_64(data, offset)
        result = 0
        shift = 0
        bytes_consumed = 0

        loop do
          raise TrapError, "unterminated LEB128 sequence" if offset + bytes_consumed >= data.length

          byte = data[offset + bytes_consumed]
          bytes_consumed += 1

          result |= (byte & 0x7F) << shift
          shift += 7

          if (byte & 0x80) == 0
            # Sign extension
            if shift < 64 && (byte & 0x40) != 0
              result |= -(1 << shift)
            end
            break
          end

          raise TrapError, "LEB128 sequence too long for i64" if bytes_consumed >= 10
        end

        # Wrap to signed 64-bit range
        wrapped = ((result + I64_SIGN) % I64_MOD) - I64_SIGN
        [wrapped, bytes_consumed]
      end
    end
  end
end
