# frozen_string_literal: true

module CodingAdventures
  module OsKernel
    SYS_EXIT  = 0
    SYS_WRITE = 1
    SYS_READ  = 2
    SYS_YIELD = 3

    REG_A0 = 10
    REG_A1 = 11
    REG_A2 = 12
    REG_A7 = 17
    REG_SP = 2

    module SyscallHandlers
      def self.handle_sys_exit(kernel, regs, mem)
        exit_code = regs.read_register(REG_A0)
        pid = kernel.current_process
        if pid >= 0 && pid < kernel.process_table.length
          kernel.process_table[pid].state = PROCESS_TERMINATED
          kernel.process_table[pid].exit_code = exit_code
        end
        next_pid = kernel.scheduler.schedule
        kernel.scheduler.context_switch(pid, next_pid)
        kernel.current_process = next_pid
        true
      end

      def self.handle_sys_write(kernel, regs, mem)
        fd = regs.read_register(REG_A0)
        buf_addr = regs.read_register(REG_A1)
        length = regs.read_register(REG_A2)
        if fd != 1 || kernel.display.nil?
          regs.write_register(REG_A0, 0)
          return true
        end
        written = 0
        length.times do |i|
          ch = mem.read_memory_byte(buf_addr + i)
          kernel.display.put_char(ch)
          written += 1
        end
        regs.write_register(REG_A0, written)
        true
      end

      def self.handle_sys_read(kernel, regs, _mem)
        fd = regs.read_register(REG_A0)
        length = regs.read_register(REG_A2)
        if fd != 0
          regs.write_register(REG_A0, 0)
          return true
        end
        available = kernel.keyboard_buffer.length
        to_read = [length, available].min
        regs.write_register(REG_A0, to_read)
        kernel.keyboard_buffer.shift(to_read) if to_read > 0
        true
      end

      def self.handle_sys_yield(kernel, regs, _mem)
        pid = kernel.current_process
        if pid >= 0 && pid < kernel.process_table.length
          kernel.process_table[pid].state = PROCESS_READY if kernel.process_table[pid].state == PROCESS_RUNNING
        end
        next_pid = kernel.scheduler.schedule
        kernel.scheduler.context_switch(pid, next_pid)
        kernel.current_process = next_pid
        true
      end

      DEFAULT_TABLE = {
        SYS_EXIT => method(:handle_sys_exit),
        SYS_WRITE => method(:handle_sys_write),
        SYS_READ => method(:handle_sys_read),
        SYS_YIELD => method(:handle_sys_yield)
      }.freeze
    end
  end
end
