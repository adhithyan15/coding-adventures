# frozen_string_literal: true

require "minitest/autorun"
require "coding_adventures/interrupt_handler"

module CodingAdventures
  module InterruptHandler
    # =====================================================================
    # IDT Tests
    # =====================================================================

    class TestNewIDT < Minitest::Test
      # A new IDT should have all 256 entries not present.
      def test_all_entries_not_present
        idt = IDT.new
        256.times do |i|
          entry = idt.get_entry(i)
          refute entry.present, "entry #{i} should not be present"
          assert_equal 0, entry.isr_address
          assert_equal 0, entry.privilege_level
        end
      end
    end

    class TestIDTSetGetEntry < Minitest::Test
      def test_set_and_get_timer
        idt = IDT.new
        entry = IDTEntry.new(isr_address: 0x00020100, present: true, privilege_level: 0)
        idt.set_entry(INT_TIMER, entry)

        got = idt.get_entry(INT_TIMER)
        assert_equal 0x00020100, got.isr_address
        assert got.present
        assert_equal 0, got.privilege_level
      end

      def test_boundary_entries
        idt = IDT.new
        idt.set_entry(0, IDTEntry.new(isr_address: 0x1000, present: true))
        idt.set_entry(255, IDTEntry.new(isr_address: 0xFF00, present: true, privilege_level: 1))

        assert_equal 0x1000, idt.get_entry(0).isr_address
        assert idt.get_entry(0).present
        assert_equal 0xFF00, idt.get_entry(255).isr_address
        assert_equal 1, idt.get_entry(255).privilege_level
      end

      def test_overwrite
        idt = IDT.new
        idt.set_entry(INT_TIMER, IDTEntry.new(isr_address: 0x1000, present: true))
        idt.set_entry(INT_TIMER, IDTEntry.new(isr_address: 0x2000, present: true))
        assert_equal 0x2000, idt.get_entry(INT_TIMER).isr_address
      end

      def test_out_of_range
        idt = IDT.new
        assert_raises(ArgumentError) { idt.set_entry(256, IDTEntry.new) }
        assert_raises(ArgumentError) { idt.get_entry(-1) }
      end
    end

    # =====================================================================
    # IDT Serialization Tests
    # =====================================================================

    class TestIDTSerialization < Minitest::Test
      def test_write_to_memory
        idt = IDT.new
        idt.set_entry(0, IDTEntry.new(isr_address: 0x00001000, present: true))
        idt.set_entry(INT_TIMER, IDTEntry.new(isr_address: 0x00020100, present: true))
        idt.set_entry(INT_SYSCALL, IDTEntry.new(isr_address: 0xDEADBEEF, present: true, privilege_level: 1))

        memory = Array.new(IDT_SIZE + 100, 0)
        idt.write_to_memory(memory, 0)

        # Entry 0: address 0x00001000 little-endian
        assert_equal [0x00, 0x10, 0x00, 0x00], memory[0..3]
        assert_equal 0x01, memory[4] # present

        # Entry 32 at offset 256
        off = INT_TIMER * IDT_ENTRY_SIZE
        assert_equal [0x00, 0x01, 0x02, 0x00], memory[off..off + 3]

        # Entry 128 at offset 1024
        off = INT_SYSCALL * IDT_ENTRY_SIZE
        assert_equal [0xEF, 0xBE, 0xAD, 0xDE], memory[off..off + 3]
        assert_equal 0x01, memory[off + 5] # privilege level
      end

      def test_load_from_memory
        memory = Array.new(IDT_SIZE, 0)
        off = 5 * IDT_ENTRY_SIZE
        memory[off] = 0xBE
        memory[off + 1] = 0xBA
        memory[off + 2] = 0xFE
        memory[off + 3] = 0xCA
        memory[off + 4] = 0x01 # present

        idt = IDT.new
        idt.load_from_memory(memory, 0)

        got = idt.get_entry(5)
        assert_equal 0xCAFEBABE, got.isr_address
        assert got.present
      end

      def test_roundtrip
        original = IDT.new
        original.set_entry(0, IDTEntry.new(isr_address: 0x1000, present: true))
        original.set_entry(INT_TIMER, IDTEntry.new(isr_address: 0x20100, present: true))
        original.set_entry(INT_SYSCALL, IDTEntry.new(isr_address: 0xDEAD, present: true, privilege_level: 1))
        original.set_entry(255, IDTEntry.new(isr_address: 0xFFFF, present: true, privilege_level: 2))

        memory = Array.new(IDT_SIZE, 0)
        original.write_to_memory(memory, 0)

        loaded = IDT.new
        loaded.load_from_memory(memory, 0)

        256.times do |i|
          assert_equal original.get_entry(i), loaded.get_entry(i), "entry #{i} mismatch"
        end
      end

      def test_endianness
        idt = IDT.new
        idt.set_entry(0, IDTEntry.new(isr_address: 0x04030201, present: true))

        memory = Array.new(IDT_SIZE, 0)
        idt.write_to_memory(memory, 0)

        # Little-endian: least significant byte first
        assert_equal [0x01, 0x02, 0x03, 0x04], memory[0..3]
      end
    end

    # =====================================================================
    # ISR Registry Tests
    # =====================================================================

    class TestISRRegistry < Minitest::Test
      def test_register_and_dispatch
        registry = ISRRegistry.new
        call_count = 0

        registry.register(INT_TIMER, ->(frame, kernel) { call_count += 1 })
        registry.dispatch(INT_TIMER, InterruptFrame.new(mcause: INT_TIMER), nil)

        assert_equal 1, call_count
      end

      def test_handler_receives_frame
        registry = ISRRegistry.new
        received = nil

        registry.register(INT_TIMER, ->(frame, kernel) { received = frame })
        frame = InterruptFrame.new(pc: 0x1000, mcause: INT_TIMER, mstatus: 0x1800)
        frame.registers[1] = 0xAAAA

        registry.dispatch(INT_TIMER, frame, nil)

        refute_nil received
        assert_equal 0x1000, received.pc
        assert_equal 0xAAAA, received.registers[1]
      end

      def test_has_handler
        registry = ISRRegistry.new
        registry.register(INT_TIMER, ->(f, k) {})

        assert registry.has_handler?(INT_TIMER)
        refute registry.has_handler?(INT_KEYBOARD)
      end

      def test_overwrite
        registry = ISRRegistry.new
        first_called = false
        second_called = false

        registry.register(INT_TIMER, ->(_f, _k) { first_called = true })
        registry.register(INT_TIMER, ->(_f, _k) { second_called = true })
        registry.dispatch(INT_TIMER, InterruptFrame.new, nil)

        refute first_called
        assert second_called
      end

      def test_dispatch_missing_raises
        registry = ISRRegistry.new
        assert_raises(KeyError) { registry.dispatch(INT_TIMER, InterruptFrame.new, nil) }
      end
    end

    # =====================================================================
    # Interrupt Controller Tests
    # =====================================================================

    class TestInterruptController < Minitest::Test
      def test_raise_interrupt
        ic = InterruptController.new
        ic.raise_interrupt(INT_TIMER)
        assert_equal 1, ic.pending_count
      end

      def test_has_pending
        ic = InterruptController.new
        refute ic.has_pending?
        ic.raise_interrupt(INT_TIMER)
        assert ic.has_pending?
      end

      def test_next_pending_priority
        ic = InterruptController.new
        ic.raise_interrupt(INT_KEYBOARD) # 33
        ic.raise_interrupt(INT_TIMER) # 32
        assert_equal INT_TIMER, ic.next_pending
      end

      def test_acknowledge
        ic = InterruptController.new
        ic.raise_interrupt(INT_TIMER)
        ic.acknowledge(INT_TIMER)
        assert_equal 0, ic.pending_count
      end

      def test_no_duplicates
        ic = InterruptController.new
        ic.raise_interrupt(INT_TIMER)
        ic.raise_interrupt(INT_TIMER)
        assert_equal 1, ic.pending_count
      end

      def test_mask
        ic = InterruptController.new
        ic.set_mask(INT_INVALID_OPCODE, true)
        ic.raise_interrupt(INT_INVALID_OPCODE)

        assert_equal 1, ic.pending_count
        refute ic.has_pending?
        assert_equal(-1, ic.next_pending)
      end

      def test_unmask
        ic = InterruptController.new
        ic.set_mask(INT_INVALID_OPCODE, true)
        ic.raise_interrupt(INT_INVALID_OPCODE)
        refute ic.has_pending?

        ic.set_mask(INT_INVALID_OPCODE, false)
        assert ic.has_pending?
      end

      def test_is_masked
        ic = InterruptController.new
        refute ic.masked?(5)
        ic.set_mask(5, true)
        assert ic.masked?(5)
        # Interrupts 32+ never masked by mask register
        refute ic.masked?(INT_TIMER)
      end

      def test_global_disable
        ic = InterruptController.new
        ic.disable
        ic.raise_interrupt(INT_TIMER)
        refute ic.has_pending?
        assert_equal(-1, ic.next_pending)
      end

      def test_global_enable
        ic = InterruptController.new
        ic.disable
        ic.raise_interrupt(INT_TIMER)
        ic.enable
        assert ic.has_pending?
      end

      def test_clear_all
        ic = InterruptController.new
        ic.raise_interrupt(INT_TIMER)
        ic.raise_interrupt(INT_KEYBOARD)
        ic.clear_all
        assert_equal 0, ic.pending_count
      end

      def test_mask_high_interrupt_ignored
        ic = InterruptController.new
        ic.set_mask(INT_TIMER, true) # 32 is out of mask range
        ic.raise_interrupt(INT_TIMER)
        assert ic.has_pending?
      end

      def test_next_pending_empty
        ic = InterruptController.new
        assert_equal(-1, ic.next_pending)
      end
    end

    # =====================================================================
    # Context Save/Restore Tests
    # =====================================================================

    class TestContextSaveRestore < Minitest::Test
      def test_roundtrip
        regs = Array.new(32) { |i| i * 100 }
        pc = 0x00080000
        mstatus = 0x00001800

        frame = InterruptHandler.save_context(regs, pc, mstatus, INT_TIMER)
        got_regs, got_pc, got_mstatus = InterruptHandler.restore_context(frame)

        assert_equal pc, got_pc
        assert_equal mstatus, got_mstatus
        32.times { |i| assert_equal regs[i], got_regs[i] }
      end

      def test_all_registers
        regs = Array.new(32) { |i| 0xDEAD0000 + i }
        frame = InterruptHandler.save_context(regs, 0, 0, 0)
        got_regs, = InterruptHandler.restore_context(frame)
        32.times { |i| assert_equal 0xDEAD0000 + i, got_regs[i] }
      end

      def test_mcause
        frame = InterruptHandler.save_context(Array.new(32, 0), 0, 0, INT_TIMER)
        assert_equal INT_TIMER, frame.mcause
      end

      def test_defensive_copy
        regs = Array.new(32, 42)
        frame = InterruptHandler.save_context(regs, 0, 0, 0)
        regs[0] = 999
        assert_equal 42, frame.registers[0]
      end
    end

    # =====================================================================
    # Priority Tests
    # =====================================================================

    class TestPriority < Minitest::Test
      def test_multiple_pending
        ic = InterruptController.new
        ic.raise_interrupt(INT_SYSCALL)       # 128
        ic.raise_interrupt(INT_KEYBOARD)      # 33
        ic.raise_interrupt(INT_INVALID_OPCODE) # 5
        ic.raise_interrupt(INT_TIMER)          # 32

        expected = [INT_INVALID_OPCODE, INT_TIMER, INT_KEYBOARD, INT_SYSCALL]
        expected.each do |want|
          got = ic.next_pending
          assert_equal want, got
          ic.acknowledge(got)
        end

        assert_equal 0, ic.pending_count
      end

      def test_acknowledge_and_next
        ic = InterruptController.new
        ic.raise_interrupt(INT_INVALID_OPCODE) # 5
        ic.raise_interrupt(INT_TIMER) # 32

        assert_equal INT_INVALID_OPCODE, ic.next_pending
        ic.acknowledge(INT_INVALID_OPCODE)
        assert_equal INT_TIMER, ic.next_pending
      end
    end

    # =====================================================================
    # Full Lifecycle Test
    # =====================================================================

    class TestFullLifecycle < Minitest::Test
      def test_complete_cycle
        ic = InterruptController.new

        # Install timer ISR
        ic.idt.set_entry(INT_TIMER, IDTEntry.new(isr_address: 0x00020100, present: true))

        handler_called = false
        handler_frame = nil

        ic.registry.register(INT_TIMER, lambda { |frame, kernel|
          handler_called = true
          handler_frame = frame
        })

        # Set up CPU state
        cpu_regs = Array.new(32, 0)
        cpu_regs[1] = 0x10000  # ra
        cpu_regs[2] = 0x7FFF0  # sp
        cpu_regs[10] = 42      # a0
        cpu_pc = 0x80000
        cpu_mstatus = 0x1800

        # Timer fires
        ic.raise_interrupt(INT_TIMER)
        assert ic.has_pending?

        int_num = ic.next_pending
        assert_equal INT_TIMER, int_num

        # Save context
        frame = InterruptHandler.save_context(cpu_regs, cpu_pc, cpu_mstatus, int_num)

        # Disable interrupts
        ic.disable

        # Look up IDT
        idt_entry = ic.idt.get_entry(int_num)
        assert idt_entry.present
        assert_equal 0x00020100, idt_entry.isr_address

        # Dispatch ISR
        ic.registry.dispatch(int_num, frame, nil)
        assert handler_called
        assert_equal INT_TIMER, handler_frame.mcause

        # Acknowledge
        ic.acknowledge(int_num)
        assert_equal 0, ic.pending_count

        # Restore context
        restored_regs, restored_pc, restored_mstatus = InterruptHandler.restore_context(frame)
        ic.enable

        # Verify
        assert_equal cpu_pc, restored_pc
        assert_equal cpu_mstatus, restored_mstatus
        assert_equal 0x10000, restored_regs[1]
        assert_equal 0x7FFF0, restored_regs[2]
        assert_equal 42, restored_regs[10]
      end
    end
  end
end
