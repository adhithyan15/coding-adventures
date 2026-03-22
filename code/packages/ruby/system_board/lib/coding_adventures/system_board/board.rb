# frozen_string_literal: true

module CodingAdventures
  module SystemBoard
    class Board
      attr_reader :cpu, :display, :interrupt_ctrl, :kernel, :disk_image,
        :trace, :powered, :cycle, :current_phase

      def initialize(config)
        @config = config
        @cpu = nil
        @display = nil
        @interrupt_ctrl = nil
        @kernel = nil
        @disk_image = nil
        @trace = BootTrace.new
        @powered = false
        @cycle = 0
        @current_phase = PHASE_POWER_ON
        @kernel_booted = false
      end

      def power_on
        return if @powered
        rv = RiscvSimulator

        mem_size = 0x10200000
        @cpu = rv::RiscVSimulator.new(memory_size: mem_size)
        @interrupt_ctrl = InterruptHandler::InterruptController.new

        dc = @config.display_config
        display_mem = Array.new(dc.columns * dc.rows * Display::BYTES_PER_CELL, 0)
        @display = Display::DisplayDriver.new(dc, display_mem)

        @kernel = OsKernel::Kernel.new(@config.kernel_config, @interrupt_ctrl, @display)
        @disk_image = Bootloader::DiskImage.new

        user_program = @config.user_program || OsKernel::Programs.generate_hello_world_program(USER_PROCESS_BASE)
        idle_binary = OsKernel::Programs.generate_idle_program

        kernel_stub_size = 16
        total_size = kernel_stub_size + idle_binary.length + user_program.length
        total_size += 4 - (total_size % 4) if total_size % 4 != 0

        bl_config = Bootloader::BootloaderConfig.new(
          entry_address: @config.bootloader_config.entry_address,
          kernel_disk_offset: @config.bootloader_config.kernel_disk_offset,
          kernel_load_address: @config.bootloader_config.kernel_load_address,
          kernel_size: total_size,
          stack_base: @config.bootloader_config.stack_base
        )
        bl = Bootloader::BootloaderGenerator.new(bl_config)
        bootloader_code = bl.generate

        # Write boot protocol
        write_word(BOOT_PROTOCOL_ADDR + 0, Bootloader::BOOT_PROTOCOL_MAGIC)
        write_word(BOOT_PROTOCOL_ADDR + 4, @config.memory_size)
        write_word(BOOT_PROTOCOL_ADDR + 8, bl_config.kernel_disk_offset)
        write_word(BOOT_PROTOCOL_ADDR + 12, bl_config.kernel_size)
        write_word(BOOT_PROTOCOL_ADDR + 16, bl_config.kernel_load_address)
        write_word(BOOT_PROTOCOL_ADDR + 20, bl_config.stack_base)

        bootloader_code.bytes.each_with_index { |b, i| @cpu.cpu.memory.write_byte(BOOTLOADER_BASE + i, b) }

        kernel_disk_data = Array.new(total_size, 0)
        idle_binary.bytes.each_with_index { |b, i| kernel_disk_data[kernel_stub_size + i] = b }
        user_program.bytes.each_with_index { |b, i| kernel_disk_data[kernel_stub_size + idle_binary.length + i] = b }
        @disk_image.load_kernel(kernel_disk_data)

        disk_data = @disk_image.data
        disk_data.each_with_index do |b, i|
          addr = DISK_MAPPED_BASE + i
          @cpu.cpu.memory.write_byte(addr, b) if addr < mem_size
        end

        idle_binary.bytes.each_with_index { |b, i| @cpu.cpu.memory.write_byte(OsKernel::DEFAULT_IDLE_PROCESS_BASE + i, b) }
        user_program.bytes.each_with_index { |b, i| @cpu.cpu.memory.write_byte(OsKernel::DEFAULT_USER_PROCESS_BASE + i, b) }

        @cpu.cpu.pc = BOOTLOADER_BASE
        @cpu.csr.write(rv::CSR_MTVEC, 0xDEAD0000)

        @powered = true
        @current_phase = PHASE_POWER_ON
        @trace.add_event(PHASE_POWER_ON, 0, "System powered on")
        @trace.add_event(PHASE_BIOS, 0, "BIOS phase simulated")
        @current_phase = PHASE_BIOS
      end

      def step
        return unless @powered
        @cycle += 1
        @cpu.step
        detect_phase_transition
        handle_trap
      end

      def run(max_cycles)
        return @trace unless @powered
        max_cycles.times do
          step
          if @kernel_booted && @kernel.idle?
            if @current_phase != PHASE_IDLE
              @current_phase = PHASE_IDLE
              @trace.add_event(PHASE_IDLE, @cycle, "System idle -- all user programs terminated")
            end
            break
          end
          break if @cpu.cpu.halted
        end
        @trace
      end

      def inject_keystroke(char)
        @kernel&.add_keystroke(char)
        @interrupt_ctrl&.raise_interrupt(OsKernel::INTERRUPT_KEYBOARD)
      end

      def display_snapshot
        @display&.snapshot
      end

      def idle?
        @kernel_booted && @kernel&.idle?
      end

      private

      def detect_phase_transition
        pc = @cpu.cpu.pc
        case @current_phase
        when PHASE_BIOS
          if pc >= BOOTLOADER_BASE && pc < BOOTLOADER_BASE + 0x10000
            @current_phase = PHASE_BOOTLOADER
            @trace.add_event(PHASE_BOOTLOADER, @cycle, "Bootloader executing")
          end
        when PHASE_BOOTLOADER
          if pc >= KERNEL_BASE && pc < KERNEL_BASE + 0x10000
            @current_phase = PHASE_KERNEL_INIT
            @trace.add_event(PHASE_KERNEL_INIT, @cycle, "Kernel entry reached")
            initialize_kernel
          end
        when PHASE_KERNEL_INIT
          if pc >= USER_PROCESS_BASE && pc < USER_PROCESS_BASE + 0x10000
            @current_phase = PHASE_USER_PROGRAM
            @trace.add_event(PHASE_USER_PROGRAM, @cycle, "User program executing")
          end
        end
      end

      def initialize_kernel
        return if @kernel_booted
        @kernel.boot
        @kernel_booted = true
        @trace.add_event(PHASE_KERNEL_INIT, @cycle, "Kernel booted: #{@kernel.process_count} processes")
        if @kernel.process_table.length > 1
          pcb = @kernel.process_table[1]
          @cpu.cpu.pc = pcb.saved_pc
          @cpu.cpu.registers.write(OsKernel::REG_SP, pcb.stack_pointer)
        end
      end

      def handle_trap
        pc = @cpu.cpu.pc
        return if pc != 0xDEAD0000

        rv = RiscvSimulator
        if !@kernel_booted
          mepc = @cpu.csr.read(rv::CSR_MEPC)
          @cpu.cpu.pc = mepc + 4
          @cpu.csr.write(rv::CSR_MSTATUS, @cpu.csr.read(rv::CSR_MSTATUS) | rv::MIE)
          return
        end

        syscall_num = @cpu.cpu.registers.read(OsKernel::REG_A7)
        mepc = @cpu.csr.read(rv::CSR_MEPC)

        reg_access = CpuRegAccess.new(@cpu)
        mem_access = CpuMemAccess.new(@cpu)
        @kernel.handle_syscall(syscall_num, reg_access, mem_access)

        pcb = @kernel.current_pcb
        if pcb
          if pcb.state == OsKernel::PROCESS_RUNNING
            @cpu.cpu.pc = mepc + 4
          elsif pcb.state == OsKernel::PROCESS_READY || pcb.state == OsKernel::PROCESS_TERMINATED
            next_pcb = @kernel.current_pcb
            if next_pcb && next_pcb.state == OsKernel::PROCESS_RUNNING
              @cpu.cpu.pc = next_pcb.saved_pc
              @cpu.cpu.registers.write(OsKernel::REG_SP, next_pcb.stack_pointer)
            elsif @kernel.process_table.length > 0
              @cpu.cpu.pc = @kernel.process_table[0].saved_pc
            end
          end
        else
          @cpu.cpu.pc = mepc + 4
        end

        @cpu.csr.write(rv::CSR_MSTATUS, @cpu.csr.read(rv::CSR_MSTATUS) | rv::MIE)

        case syscall_num
        when OsKernel::SYS_WRITE
          @trace.add_event(@current_phase, @cycle, "sys_write: bytes written to display")
        when OsKernel::SYS_EXIT
          @trace.add_event(@current_phase, @cycle, "sys_exit: process terminated")
        when OsKernel::SYS_YIELD
          @trace.add_event(@current_phase, @cycle, "sys_yield: voluntary context switch")
        end
      end

      def write_word(address, value)
        @cpu.cpu.memory.write_byte(address, value & 0xFF)
        @cpu.cpu.memory.write_byte(address + 1, (value >> 8) & 0xFF)
        @cpu.cpu.memory.write_byte(address + 2, (value >> 16) & 0xFF)
        @cpu.cpu.memory.write_byte(address + 3, (value >> 24) & 0xFF)
      end
    end

    class CpuRegAccess
      def initialize(cpu) = @cpu = cpu
      def read_register(index) = @cpu.cpu.registers.read(index)
      def write_register(index, value) = @cpu.cpu.registers.write(index, value)
    end

    class CpuMemAccess
      def initialize(cpu) = @cpu = cpu
      def read_memory_byte(address) = @cpu.cpu.memory.read_byte(address)
    end
  end
end
