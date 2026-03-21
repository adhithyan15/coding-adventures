# frozen_string_literal: true

module CodingAdventures
  module OsKernel
    DEFAULT_KERNEL_BASE       = 0x00020000
    DEFAULT_IDLE_PROCESS_BASE = 0x00030000
    DEFAULT_IDLE_PROCESS_SIZE = 0x00010000
    DEFAULT_USER_PROCESS_SIZE = 0x00010000
    DEFAULT_KERNEL_STACK_BASE = 0x00060000
    DEFAULT_KERNEL_STACK_SIZE = 0x00010000
    INTERRUPT_TIMER    = 32
    INTERRUPT_KEYBOARD = 33
    INTERRUPT_SYSCALL  = 128

    KernelConfig = Data.define(:timer_interval, :max_processes, :memory_layout) do
      def initialize(timer_interval: 100, max_processes: 16, memory_layout: [])
        super
      end
    end

    def self.default_kernel_config
      KernelConfig.new(
        timer_interval: 100,
        max_processes: 16,
        memory_layout: [
          MemoryRegion.new(base: 0, size: 0x1000, permissions: PERM_READ, owner: -1, name: "IDT"),
          MemoryRegion.new(base: 0x1000, size: 0x1000, permissions: PERM_READ | PERM_WRITE, owner: -1, name: "Boot Protocol"),
          MemoryRegion.new(base: DEFAULT_KERNEL_BASE, size: 0x10000, permissions: PERM_READ | PERM_WRITE | PERM_EXECUTE, owner: -1, name: "Kernel Code"),
          MemoryRegion.new(base: DEFAULT_IDLE_PROCESS_BASE, size: DEFAULT_IDLE_PROCESS_SIZE, permissions: PERM_READ | PERM_WRITE | PERM_EXECUTE, owner: 0, name: "Idle Process"),
          MemoryRegion.new(base: DEFAULT_USER_PROCESS_BASE, size: DEFAULT_USER_PROCESS_SIZE, permissions: PERM_READ | PERM_WRITE | PERM_EXECUTE, owner: 1, name: "User Process"),
          MemoryRegion.new(base: DEFAULT_KERNEL_STACK_BASE, size: DEFAULT_KERNEL_STACK_SIZE, permissions: PERM_READ | PERM_WRITE, owner: -1, name: "Kernel Stack")
        ]
      )
    end

    class Kernel
      attr_accessor :process_table, :current_process, :scheduler, :memory_manager,
        :keyboard_buffer, :booted, :display, :interrupt_ctrl, :syscall_table

      def initialize(config, interrupt_ctrl = nil, display = nil)
        @config = config
        @interrupt_ctrl = interrupt_ctrl
        @display = display
        @syscall_table = SyscallHandlers::DEFAULT_TABLE.dup
        @process_table = []
        @current_process = 0
        @scheduler = nil
        @memory_manager = nil
        @keyboard_buffer = []
        @booted = false
        @next_pid = 0
      end

      def boot
        @memory_manager = MemoryManager.new(@config.memory_layout)

        if @interrupt_ctrl
          @interrupt_ctrl.registry.register(INTERRUPT_TIMER) { |frame, _| handle_timer(frame) }
          @interrupt_ctrl.registry.register(INTERRUPT_KEYBOARD) { |frame, _| handle_keyboard(frame) }
          @interrupt_ctrl.registry.register(INTERRUPT_SYSCALL) { |frame, _| handle_syscall_frame(frame) }
        end

        idle_binary = Programs.generate_idle_program
        create_process("idle", idle_binary, DEFAULT_IDLE_PROCESS_BASE, DEFAULT_IDLE_PROCESS_SIZE)

        hw_binary = Programs.generate_hello_world_program(DEFAULT_USER_PROCESS_BASE)
        create_process("hello-world", hw_binary, DEFAULT_USER_PROCESS_BASE, DEFAULT_USER_PROCESS_SIZE)

        @scheduler = Scheduler.new(@process_table)

        if @process_table.length > 1
          @process_table[1].state = PROCESS_RUNNING
          @current_process = 1
          @scheduler.current = 1
        end

        @booted = true
      end

      def create_process(name, _binary, mem_base, mem_size)
        return -1 if @process_table.length >= @config.max_processes
        pid = @next_pid
        @next_pid += 1
        pcb = ProcessControlBlock.new(pid: pid, name: name, memory_base: mem_base, memory_size: mem_size)
        pcb.saved_registers[REG_SP] = pcb.stack_pointer
        @process_table << pcb
        pid
      end

      def handle_syscall(syscall_num, regs, mem)
        handler = @syscall_table[syscall_num]
        if handler.nil?
          pid = @current_process
          if pid >= 0 && pid < @process_table.length
            @process_table[pid].state = PROCESS_TERMINATED
            @process_table[pid].exit_code = -1
          end
          return false
        end
        handler.call(self, regs, mem)
      end

      def handle_syscall_frame(_frame) = nil
      def handle_keyboard(_frame) = nil

      def handle_timer(frame)
        return if @scheduler.nil?
        pid = @current_process
        if pid >= 0 && pid < @process_table.length
          pcb = @process_table[pid]
          if pcb.state == PROCESS_RUNNING
            pcb.state = PROCESS_READY
            pcb.saved_registers = frame.registers.dup
            pcb.saved_pc = frame.pc
          end
        end
        next_pid = @scheduler.schedule
        @scheduler.context_switch(pid, next_pid)
        @current_process = next_pid
        if next_pid >= 0 && next_pid < @process_table.length
          np = @process_table[next_pid]
          frame.registers = np.saved_registers.dup
          frame.pc = np.saved_pc
        end
      end

      def idle?
        @process_table.each do |pcb|
          next if pcb.pid == 0
          return false if pcb.state != PROCESS_TERMINATED
        end
        true
      end

      def process_info(pid)
        return ProcessInfo.new if pid < 0 || pid >= @process_table.length
        pcb = @process_table[pid]
        ProcessInfo.new(pid: pcb.pid, name: pcb.name, state: pcb.state, pc: pcb.saved_pc)
      end

      def process_count = @process_table.length

      def current_pcb
        return nil unless @current_process >= 0 && @current_process < @process_table.length
        @process_table[@current_process]
      end

      def add_keystroke(ch) = @keyboard_buffer << ch
    end
  end
end
