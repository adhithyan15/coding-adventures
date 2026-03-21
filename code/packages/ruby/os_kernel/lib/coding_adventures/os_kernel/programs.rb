# frozen_string_literal: true

module CodingAdventures
  module OsKernel
    DEFAULT_USER_PROCESS_BASE = 0x00040000

    module Programs
      def self.generate_idle_program
        rv = RiscvSimulator
        instructions = [
          rv.encode_addi(REG_A7, 0, SYS_YIELD),
          rv.encode_ecall,
          rv.encode_jal(0, -8)
        ]
        rv.assemble(instructions)
      end

      def self.generate_hello_world_program(mem_base)
        rv = RiscvSimulator
        data_offset = 0x100
        data_addr = mem_base + data_offset
        message = "Hello World\n".bytes

        instructions = []
        upper = (data_addr >> 12) & 0xFFFFF
        lower = data_addr & 0xFFF
        upper = (upper + 1) & 0xFFFFF if lower >= 0x800

        instructions << rv.encode_lui(REG_A1, upper)
        if lower != 0
          sl = lower >= 0x800 ? lower - 0x1000 : lower
          instructions << rv.encode_addi(REG_A1, REG_A1, sl)
        end
        instructions << rv.encode_addi(REG_A0, 0, 1)
        instructions << rv.encode_addi(REG_A2, 0, message.length)
        instructions << rv.encode_addi(REG_A7, 0, SYS_WRITE)
        instructions << rv.encode_ecall
        instructions << rv.encode_addi(REG_A0, 0, 0)
        instructions << rv.encode_addi(REG_A7, 0, SYS_EXIT)
        instructions << rv.encode_ecall

        code = rv.assemble(instructions)
        binary = Array.new(data_offset + message.length, 0)
        code.bytes.each_with_index { |b, i| binary[i] = b }
        message.each_with_index { |b, i| binary[data_offset + i] = b }
        binary.pack("C*")
      end

      def self.generate_hello_world_binary
        generate_hello_world_program(DEFAULT_USER_PROCESS_BASE)
      end
    end
  end
end
