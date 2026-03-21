# frozen_string_literal: true

# === BIOS Firmware Generator ===
#
# Generates RISC-V machine code for the BIOS power-on sequence:
# 1. Memory probe (or use configured size)
# 2. IDT initialization (256 entries)
# 3. HardwareInfo write at 0x00001000
# 4. Jump to bootloader at 0x00010000

require "coding_adventures/riscv_simulator/encoding"
require "coding_adventures/riscv_simulator/opcodes"

module CodingAdventures
  module RomBios
    # Well-known addresses
    IDT_BASE = 0x00000000
    IDT_ENTRY_COUNT = 256
    IDT_ENTRY_SIZE = 8
    ISR_STUB_BASE = 0x00000800
    DEFAULT_FAULT_HANDLER = 0x00000800
    TIMER_ISR = 0x00000808
    KEYBOARD_ISR = 0x00000810
    SYSCALL_ISR = 0x00000818
    PROBE_START = 0x00100000
    PROBE_STEP = 0x00100000
    PROBE_LIMIT = 0xFFFB0000
    DEFAULT_BOOTLOADER_ENTRY = 0x00010000
    DEFAULT_FRAMEBUFFER_BASE = 0xFFFB0000
    HARDWARE_INFO_ADDR = 0x00001000

    # BIOS configuration.
    BIOSConfig = Data.define(
      :memory_size,
      :display_columns,
      :display_rows,
      :framebuffer_base,
      :bootloader_entry
    ) do
      def initialize(
        memory_size: 0,
        display_columns: 80,
        display_rows: 25,
        framebuffer_base: DEFAULT_FRAMEBUFFER_BASE,
        bootloader_entry: DEFAULT_BOOTLOADER_ENTRY
      )
        super
      end
    end

    # Annotated instruction with address, machine code, assembly, and comment.
    AnnotatedInstruction = Data.define(:address, :machine_code, :assembly, :comment)

    # Generates BIOS firmware as RISC-V machine code.
    class BIOSFirmware
      attr_reader :config

      def initialize(config)
        @config = config
      end

      # Return firmware as raw bytes (little-endian RISC-V machine code).
      def generate
        annotated = generate_with_comments
        words = annotated.map(&:machine_code)
        # Convert to little-endian bytes
        words.flat_map { |w| [w & 0xFF, (w >> 8) & 0xFF, (w >> 16) & 0xFF, (w >> 24) & 0xFF] }
      end

      # Return firmware as array of AnnotatedInstruction.
      def generate_with_comments # rubocop:disable Metrics/AbcSize, Metrics/CyclomaticComplexity, Metrics/MethodLength, Metrics/PerceivedComplexity
        instructions = []
        address = DEFAULT_ROM_BASE
        enc = CodingAdventures::RiscvSimulator

        emit = lambda { |code, asm, comment|
          instructions << AnnotatedInstruction.new(
            address: address,
            machine_code: code & 0xFFFFFFFF,
            assembly: asm,
            comment: comment
          )
          address += 4
        }

        # === Step 1: Memory Probe ===
        if @config.memory_size > 0
          upper = (@config.memory_size >> 12) & 0xFFFFF
          lower = @config.memory_size & 0xFFF
          emit.call(enc.encode_lui(8, upper),
            "lui x8, 0x#{format("%05X", upper)}",
            "Step 1: Load configured memory size (#{@config.memory_size} bytes)")
          if lower != 0
            emit.call(enc.encode_addi(8, 8, sign_extend_12(lower)),
              "addi x8, x8, 0x#{format("%03X", lower)}",
              "Step 1: Add lower 12 bits of memory size")
          end
        else
          emit.call(enc.encode_lui(5, PROBE_START >> 12),
            "lui x5, 0x#{format("%05X", PROBE_START >> 12)}",
            "Step 1: x5 = 0x00100000 (probe start)")
          emit.call(enc.encode_lui(6, 0xDEADC),
            "lui x6, 0xDEADC",
            "Step 1: x6 upper = 0xDEADC000 (compensated)")
          emit.call(enc.encode_addi(6, 6, sign_extend_12(0xEEF)),
            "addi x6, x6, #{sign_extend_12(0xEEF)}",
            "Step 1: x6 = 0xDEADBEEF (test pattern)")
          emit.call(enc.encode_lui(9, PROBE_LIMIT >> 12),
            "lui x9, 0x#{format("%05X", PROBE_LIMIT >> 12)}",
            "Step 1: x9 = 0xFFFB0000 (probe limit)")
          emit.call(enc.encode_lui(10, PROBE_STEP >> 12),
            "lui x10, 0x#{format("%05X", PROBE_STEP >> 12)}",
            "Step 1: x10 = 0x00100000 (1 MB step)")
          emit.call(enc.encode_sw(6, 5, 0), "sw x6, 0(x5)",
            "Step 1: Write test pattern")
          emit.call(enc.encode_lw(7, 5, 0), "lw x7, 0(x5)",
            "Step 1: Read it back")
          emit.call(enc.encode_bne(6, 7, 12), "bne x6, x7, +12",
            "Step 1: If mismatch, memory ends here")
          emit.call(enc.encode_add(5, 5, 10), "add x5, x5, x10",
            "Step 1: Advance by 1 MB")
          emit.call(enc.encode_blt(5, 9, -16), "blt x5, x9, -16",
            "Step 1: Loop if below limit")
          emit.call(enc.encode_add(8, 5, 0), "add x8, x5, x0",
            "Step 1: x8 = detected memory size")
        end

        # === Step 2: IDT Initialization ===
        emit.call(enc.encode_lui(11, ISR_STUB_BASE >> 12),
          "lui x11, 0x#{format("%05X", ISR_STUB_BASE >> 12)}",
          "Step 2a: x11 = ISR stub base")
        if (ISR_STUB_BASE & 0xFFF) != 0
          emit.call(enc.encode_addi(11, 11, ISR_STUB_BASE & 0xFFF),
            "addi x11, x11, #{ISR_STUB_BASE & 0xFFF}",
            "Step 2a: Add lower bits")
        end

        fault_instr = enc.encode_jal(0, 0)
        upper_f = li_upper(fault_instr)
        emit.call(enc.encode_lui(12, upper_f),
          "lui x12, 0x#{format("%05X", upper_f)}",
          "Step 2a: Load fault handler instruction")
        if (fault_instr & 0xFFF) != 0
          emit.call(enc.encode_addi(12, 12, sign_extend_12(fault_instr & 0xFFF)),
            "addi x12, x12, #{sign_extend_12(fault_instr & 0xFFF)}",
            "Step 2a: Load fault handler lower bits")
        end
        emit.call(enc.encode_sw(12, 11, 0), "sw x12, 0(x11)",
          "Step 2a: Store fault handler at 0x800")
        emit.call(enc.encode_sw(0, 11, 4), "sw x0, 4(x11)",
          "Step 2a: Store NOP at 0x804")

        mret_instr = enc.encode_mret
        upper_m = li_upper(mret_instr)
        emit.call(enc.encode_lui(12, upper_m),
          "lui x12, 0x#{format("%05X", upper_m)}",
          "Step 2a: Load mret instruction")
        if (mret_instr & 0xFFF) != 0
          emit.call(enc.encode_addi(12, 12, sign_extend_12(mret_instr & 0xFFF)),
            "addi x12, x12, #{sign_extend_12(mret_instr & 0xFFF)}",
            "Step 2a: Load mret lower bits")
        end
        emit.call(enc.encode_sw(12, 11, 8), "sw x12, 8(x11)",
          "Step 2a: Store timer_isr at 0x808")
        emit.call(enc.encode_sw(12, 11, 16), "sw x12, 16(x11)",
          "Step 2a: Store keyboard_isr at 0x810")
        emit.call(enc.encode_sw(12, 11, 24), "sw x12, 24(x11)",
          "Step 2a: Store syscall_isr at 0x818")

        # IDT entries setup
        emit.call(enc.encode_addi(13, 0, 0), "addi x13, x0, 0", "Step 2b: x13 = IDT base")
        emit.call(enc.encode_lui(14, 1), "lui x14, 0x00001", "Step 2b: x14 = 0x1000")
        emit.call(enc.encode_addi(14, 14, -2048), "addi x14, x14, -2048", "Step 2b: x14 = 0x800")
        emit.call(enc.encode_lui(16, 1), "lui x16, 0x00001", "Step 2b: x16 = 0x1000")
        emit.call(enc.encode_addi(16, 16, -2048), "addi x16, x16, -2048", "Step 2b: x16 = 0x800 (IDT end)")
        emit.call(enc.encode_addi(17, 0, 1), "addi x17, x0, 1", "Step 2b: x17 = 1 (flags)")

        emit.call(enc.encode_lui(18, 1), "lui x18, 0x00001", "Step 2b: x18 = 0x1000")
        emit.call(enc.encode_addi(18, 18, -2040), "addi x18, x18, -2040", "Step 2b: x18 = 0x808")
        emit.call(enc.encode_lui(19, 1), "lui x19, 0x00001", "Step 2b: x19 = 0x1000")
        emit.call(enc.encode_addi(19, 19, -2032), "addi x19, x19, -2032", "Step 2b: x19 = 0x810")
        emit.call(enc.encode_lui(20, 1), "lui x20, 0x00001", "Step 2b: x20 = 0x1000")
        emit.call(enc.encode_addi(20, 20, -2024), "addi x20, x20, -2024", "Step 2b: x20 = 0x818")

        emit.call(enc.encode_addi(21, 0, 256), "addi x21, x0, 256", "Step 2b: x21 = 256 (timer)")
        emit.call(enc.encode_addi(22, 0, 264), "addi x22, x0, 264", "Step 2b: x22 = 264 (keyboard)")
        emit.call(enc.encode_addi(23, 0, 1024), "addi x23, x0, 1024", "Step 2b: x23 = 1024 (syscall)")

        loop_start = address
        emit.call(enc.encode_beq(13, 21, 20), "beq x13, x21, +20", "Step 2b: Timer entry?")
        emit.call(enc.encode_beq(13, 22, 24), "beq x13, x22, +24", "Step 2b: Keyboard entry?")
        emit.call(enc.encode_beq(13, 23, 28), "beq x13, x23, +28", "Step 2b: Syscall entry?")
        emit.call(enc.encode_sw(14, 13, 0), "sw x14, 0(x13)", "Step 2b: Store default handler")
        emit.call(enc.encode_jal(0, 24), "jal x0, +24", "Step 2b: Skip special stores")
        emit.call(enc.encode_sw(18, 13, 0), "sw x18, 0(x13)", "Step 2b: Store timer ISR")
        emit.call(enc.encode_jal(0, 16), "jal x0, +16", "Step 2b: Skip to flags")
        emit.call(enc.encode_sw(19, 13, 0), "sw x19, 0(x13)", "Step 2b: Store keyboard ISR")
        emit.call(enc.encode_jal(0, 8), "jal x0, +8", "Step 2b: Skip to flags")
        emit.call(enc.encode_sw(20, 13, 0), "sw x20, 0(x13)", "Step 2b: Store syscall ISR")
        emit.call(enc.encode_sw(17, 13, 4), "sw x17, 4(x13)", "Step 2b: Store flags")
        emit.call(enc.encode_addi(13, 13, 8), "addi x13, x13, 8", "Step 2b: Next entry")
        loop_offset = loop_start - address
        emit.call(enc.encode_blt(13, 16, loop_offset), "blt x13, x16, #{loop_offset}", "Step 2b: Loop")

        # === Step 3: HardwareInfo ===
        emit.call(enc.encode_lui(5, HARDWARE_INFO_ADDR >> 12),
          "lui x5, 0x#{format("%05X", HARDWARE_INFO_ADDR >> 12)}",
          "Step 3: x5 = HardwareInfo base")
        emit.call(enc.encode_sw(8, 5, 0), "sw x8, 0(x5)", "Step 3: MemorySize")
        emit.call(enc.encode_addi(6, 0, @config.display_columns),
          "addi x6, x0, #{@config.display_columns}", "Step 3: DisplayColumns")
        emit.call(enc.encode_sw(6, 5, 4), "sw x6, 4(x5)", "Step 3: Store DisplayColumns")
        emit.call(enc.encode_addi(6, 0, @config.display_rows),
          "addi x6, x0, #{@config.display_rows}", "Step 3: DisplayRows")
        emit.call(enc.encode_sw(6, 5, 8), "sw x6, 8(x5)", "Step 3: Store DisplayRows")

        fb_upper = @config.framebuffer_base >> 12
        fb_lower = @config.framebuffer_base & 0xFFF
        emit.call(enc.encode_lui(6, fb_upper),
          "lui x6, 0x#{format("%05X", fb_upper)}", "Step 3: FramebufferBase upper")
        if fb_lower != 0
          emit.call(enc.encode_addi(6, 6, sign_extend_12(fb_lower)),
            "addi x6, x6, #{sign_extend_12(fb_lower)}", "Step 3: FramebufferBase lower")
        end
        emit.call(enc.encode_sw(6, 5, 12), "sw x6, 12(x5)", "Step 3: Store FramebufferBase")
        emit.call(enc.encode_sw(0, 5, 16), "sw x0, 16(x5)", "Step 3: IDTBase = 0")
        emit.call(enc.encode_addi(6, 0, 256), "addi x6, x0, 256", "Step 3: IDTEntries")
        emit.call(enc.encode_sw(6, 5, 20), "sw x6, 20(x5)", "Step 3: Store IDTEntries")

        bl_upper = @config.bootloader_entry >> 12
        bl_lower = @config.bootloader_entry & 0xFFF
        emit.call(enc.encode_lui(6, bl_upper),
          "lui x6, 0x#{format("%05X", bl_upper)}", "Step 3: BootloaderEntry upper")
        if bl_lower != 0
          emit.call(enc.encode_addi(6, 6, sign_extend_12(bl_lower)),
            "addi x6, x6, #{sign_extend_12(bl_lower)}", "Step 3: BootloaderEntry lower")
        end
        emit.call(enc.encode_sw(6, 5, 24), "sw x6, 24(x5)", "Step 3: Store BootloaderEntry")

        # === Step 4: Jump to Bootloader ===
        emit.call(enc.encode_lui(6, @config.bootloader_entry >> 12),
          "lui x6, 0x#{format("%05X", @config.bootloader_entry >> 12)}",
          "Step 4: Bootloader entry upper")
        if (@config.bootloader_entry & 0xFFF) != 0
          emit.call(enc.encode_addi(6, 6, sign_extend_12(@config.bootloader_entry & 0xFFF)),
            "addi x6, x6, #{sign_extend_12(@config.bootloader_entry & 0xFFF)}",
            "Step 4: Bootloader entry lower")
        end
        emit.call(enc.encode_jalr(0, 6, 0), "jalr x0, x6, 0",
          "Step 4: Jump to bootloader at 0x#{format("%08X", @config.bootloader_entry)}")

        instructions
      end

      private

      def sign_extend_12(val)
        val = val & 0xFFF
        val >= 0x800 ? val - 0x1000 : val
      end

      def li_upper(value)
        upper = (value >> 12) & 0xFFFFF
        upper = (upper + 1) & 0xFFFFF if (value & 0x800) != 0
        upper
      end
    end
  end
end
