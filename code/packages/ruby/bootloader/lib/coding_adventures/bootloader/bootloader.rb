# frozen_string_literal: true

# S02 Bootloader -- generates RISC-V machine code that loads the kernel
# from disk into RAM and transfers control to it.

module CodingAdventures
  module Bootloader
    DEFAULT_ENTRY_ADDRESS     = 0x00010000
    DEFAULT_KERNEL_DISK_OFFSET = 0x00080000
    DEFAULT_KERNEL_LOAD_ADDRESS = 0x00020000
    DEFAULT_STACK_BASE        = 0x0006FFF0
    DISK_MEMORY_MAP_BASE      = 0x10000000
    BOOT_PROTOCOL_ADDRESS     = 0x00001000
    BOOT_PROTOCOL_MAGIC       = 0xB007CAFE

    BootloaderConfig = Data.define(
      :entry_address, :kernel_disk_offset, :kernel_load_address,
      :kernel_size, :stack_base
    ) do
      def initialize(
        entry_address: DEFAULT_ENTRY_ADDRESS,
        kernel_disk_offset: DEFAULT_KERNEL_DISK_OFFSET,
        kernel_load_address: DEFAULT_KERNEL_LOAD_ADDRESS,
        kernel_size: 0,
        stack_base: DEFAULT_STACK_BASE
      )
        super
      end
    end

    AnnotatedInstruction = Data.define(:address, :machine_code, :assembly, :comment)

    class BootloaderGenerator
      attr_reader :config

      def initialize(config)
        @config = config
      end

      def generate
        annotated = generate_with_comments
        instructions = annotated.map(&:machine_code)
        RiscvSimulator.assemble(instructions)
      end

      def generate_with_comments
        instructions = []
        address = config.entry_address
        rv = RiscvSimulator

        emit = proc { |code, asm, comment|
          instructions << AnnotatedInstruction.new(address: address, machine_code: code, assembly: asm, comment: comment)
          address += 4
        }

        # Phase 1: Validate Boot Protocol
        emit.call(rv.encode_lui(5, 1), "lui t0, 0x00001", "Phase 1: t0 = 0x00001000")
        emit.call(rv.encode_lw(6, 5, 0), "lw t1, 0(t0)", "Phase 1: t1 = magic")
        emit.call(rv.encode_lui(7, 0xB007D), "lui t2, 0xB007D", "Phase 1: t2 upper = 0xB007D000")
        signed_afe = 0xAFE >= 0x800 ? 0xAFE - 0x1000 : 0xAFE
        emit.call(rv.encode_addi(7, 7, signed_afe), "addi t2, t2, #{signed_afe}", "Phase 1: t2 = 0xB007CAFE")

        halt_branch_index = instructions.length
        emit.call(rv.encode_bne(6, 7, 0), "bne t1, t2, halt", "Phase 1: If magic wrong, halt")

        # Phase 2: Read Boot Parameters
        source = DISK_MEMORY_MAP_BASE + config.kernel_disk_offset
        address = emit_load_imm(instructions, address, 5, source, "Phase 2: t0 = source")
        address = emit_load_imm(instructions, address, 6, config.kernel_load_address, "Phase 2: t1 = dest")
        address = emit_load_imm(instructions, address, 7, config.kernel_size, "Phase 2: t2 = size")

        # Phase 3: Copy kernel
        emit.call(rv.encode_beq(7, 0, 24), "beq t2, x0, +24", "Phase 3: Skip if size 0")
        copy_loop_addr = address
        emit.call(rv.encode_lw(28, 5, 0), "lw t3, 0(t0)", "Phase 3: Load word")
        emit.call(rv.encode_sw(28, 6, 0), "sw t3, 0(t1)", "Phase 3: Store word")
        emit.call(rv.encode_addi(5, 5, 4), "addi t0, t0, 4", "Phase 3: src += 4")
        emit.call(rv.encode_addi(6, 6, 4), "addi t1, t1, 4", "Phase 3: dst += 4")
        emit.call(rv.encode_addi(7, 7, -4), "addi t2, t2, -4", "Phase 3: remaining -= 4")
        loop_offset = copy_loop_addr - address
        emit.call(rv.encode_bne(7, 0, loop_offset), "bne t2, x0, #{loop_offset}", "Phase 3: Loop")

        # Phase 4: Set stack and jump
        address = emit_load_imm(instructions, address, 2, config.stack_base, "Phase 4: sp = stack")
        address = emit_load_imm(instructions, address, 5, config.kernel_load_address, "Phase 4: t0 = kernel")
        emit.call(rv.encode_jalr(0, 5, 0), "jalr x0, t0, 0", "Phase 4: Jump to kernel")

        halt_addr = address
        emit.call(rv.encode_jal(0, 0), "jal x0, 0", "Halt: infinite loop")

        # Patch halt branch
        branch_pc = instructions[halt_branch_index].address
        halt_offset = halt_addr - branch_pc
        instructions[halt_branch_index] = AnnotatedInstruction.new(
          address: branch_pc,
          machine_code: rv.encode_bne(6, 7, halt_offset),
          assembly: "bne t1, t2, +#{halt_offset}",
          comment: instructions[halt_branch_index].comment
        )

        instructions
      end

      def instruction_count = generate_with_comments.length

      def estimate_cycles
        (config.kernel_size / 4) * 6 + 20
      end

      private

      def emit_load_imm(instructions, address, rd, value, comment)
        rv = RiscvSimulator
        upper = (value >> 12) & 0xFFFFF
        lower = value & 0xFFF
        upper = (upper + 1) & 0xFFFFF if lower >= 0x800
        reg_names = {2 => "sp", 5 => "t0", 6 => "t1", 7 => "t2"}
        rn = reg_names[rd] || "x#{rd}"

        if upper != 0
          instructions << AnnotatedInstruction.new(address: address, machine_code: rv.encode_lui(rd, upper), assembly: "lui #{rn}, 0x#{upper.to_s(16).upcase}", comment: comment)
          address += 4
          if lower != 0
            sl = lower >= 0x800 ? lower - 0x1000 : lower
            instructions << AnnotatedInstruction.new(address: address, machine_code: rv.encode_addi(rd, rd, sl), assembly: "addi #{rn}, #{rn}, #{sl}", comment: comment)
            address += 4
          end
        elsif lower != 0
          sl = lower >= 0x800 ? lower - 0x1000 : lower
          instructions << AnnotatedInstruction.new(address: address, machine_code: rv.encode_addi(rd, 0, sl), assembly: "addi #{rn}, x0, #{sl}", comment: comment)
          address += 4
        else
          instructions << AnnotatedInstruction.new(address: address, machine_code: rv.encode_addi(rd, 0, 0), assembly: "addi #{rn}, x0, 0", comment: "#{comment} (value = 0)")
          address += 4
        end
        address
      end
    end
  end
end
