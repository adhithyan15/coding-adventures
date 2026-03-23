"""Tests for the S03 Interrupt Handler package.

Covers: IDT, ISR registry, interrupt controller, context save/restore,
priority dispatch, masking, and the full interrupt lifecycle.
"""

from __future__ import annotations

import pytest

from interrupt_handler import (
    INT_INVALID_OPCODE,
    INT_KEYBOARD,
    INT_SYSCALL,
    INT_TIMER,
    IDT_ENTRY_SIZE,
    IDT_SIZE,
    IDTEntry,
    InterruptController,
    InterruptDescriptorTable,
    InterruptFrame,
    ISRRegistry,
    restore_context,
    save_context,
)


# =========================================================================
# IDT Tests
# =========================================================================


class TestNewIDT:
    """Verify that a new IDT has all entries not present."""

    def test_all_entries_not_present(self) -> None:
        idt = InterruptDescriptorTable()
        for i in range(256):
            entry = idt.get_entry(i)
            assert not entry.present
            assert entry.isr_address == 0
            assert entry.privilege_level == 0


class TestIDTSetGetEntry:
    """Verify SetEntry/GetEntry roundtrip."""

    def test_set_and_get_timer(self) -> None:
        idt = InterruptDescriptorTable()
        entry = IDTEntry(isr_address=0x00020100, present=True, privilege_level=0)
        idt.set_entry(INT_TIMER, entry)

        got = idt.get_entry(INT_TIMER)
        assert got.isr_address == 0x00020100
        assert got.present is True
        assert got.privilege_level == 0

    def test_boundary_entries(self) -> None:
        idt = InterruptDescriptorTable()
        idt.set_entry(0, IDTEntry(isr_address=0x1000, present=True))
        idt.set_entry(255, IDTEntry(isr_address=0xFF00, present=True, privilege_level=1))

        assert idt.get_entry(0).isr_address == 0x1000
        assert idt.get_entry(255).isr_address == 0xFF00
        assert idt.get_entry(255).privilege_level == 1

    def test_overwrite(self) -> None:
        idt = InterruptDescriptorTable()
        idt.set_entry(INT_TIMER, IDTEntry(isr_address=0x1000, present=True))
        idt.set_entry(INT_TIMER, IDTEntry(isr_address=0x2000, present=True))
        assert idt.get_entry(INT_TIMER).isr_address == 0x2000

    def test_out_of_range(self) -> None:
        idt = InterruptDescriptorTable()
        with pytest.raises(ValueError):
            idt.set_entry(256, IDTEntry())
        with pytest.raises(ValueError):
            idt.get_entry(-1)


# =========================================================================
# IDT Serialization Tests
# =========================================================================


class TestIDTSerialization:
    """Verify binary write/load of IDT to/from memory."""

    def test_write_to_memory(self) -> None:
        idt = InterruptDescriptorTable()
        idt.set_entry(0, IDTEntry(isr_address=0x00001000, present=True))
        idt.set_entry(INT_TIMER, IDTEntry(isr_address=0x00020100, present=True))
        idt.set_entry(
            INT_SYSCALL,
            IDTEntry(isr_address=0xDEADBEEF, present=True, privilege_level=1),
        )

        memory = bytearray(IDT_SIZE + 100)
        idt.write_to_memory(memory, 0)

        # Entry 0: address 0x00001000 little-endian
        assert memory[0:4] == b"\x00\x10\x00\x00"
        assert memory[4] == 0x01  # present

        # Entry 32 at offset 256
        off = INT_TIMER * IDT_ENTRY_SIZE
        assert memory[off : off + 4] == b"\x00\x01\x02\x00"

        # Entry 128 at offset 1024
        off = INT_SYSCALL * IDT_ENTRY_SIZE
        assert memory[off : off + 4] == b"\xEF\xBE\xAD\xDE"
        assert memory[off + 5] == 0x01  # privilege level

    def test_load_from_memory(self) -> None:
        memory = bytearray(IDT_SIZE)
        off = 5 * IDT_ENTRY_SIZE
        memory[off : off + 4] = b"\xBE\xBA\xFE\xCA"  # 0xCAFEBABE LE
        memory[off + 4] = 0x01  # present

        idt = InterruptDescriptorTable()
        idt.load_from_memory(memory, 0)

        got = idt.get_entry(5)
        assert got.isr_address == 0xCAFEBABE
        assert got.present is True

    def test_roundtrip(self) -> None:
        original = InterruptDescriptorTable()
        original.set_entry(0, IDTEntry(isr_address=0x1000, present=True))
        original.set_entry(INT_TIMER, IDTEntry(isr_address=0x20100, present=True))
        original.set_entry(
            INT_SYSCALL, IDTEntry(isr_address=0xDEAD, present=True, privilege_level=1)
        )
        original.set_entry(
            255, IDTEntry(isr_address=0xFFFF, present=True, privilege_level=2)
        )

        memory = bytearray(IDT_SIZE)
        original.write_to_memory(memory, 0)

        loaded = InterruptDescriptorTable()
        loaded.load_from_memory(memory, 0)

        for i in range(256):
            orig = original.get_entry(i)
            got = loaded.get_entry(i)
            assert orig.isr_address == got.isr_address, f"entry {i} isr_address"
            assert orig.present == got.present, f"entry {i} present"
            assert orig.privilege_level == got.privilege_level, f"entry {i} priv"

    def test_endianness(self) -> None:
        idt = InterruptDescriptorTable()
        idt.set_entry(0, IDTEntry(isr_address=0x04030201, present=True))

        memory = bytearray(IDT_SIZE)
        idt.write_to_memory(memory, 0)

        # Little-endian: least significant byte first
        assert memory[0:4] == b"\x01\x02\x03\x04"


# =========================================================================
# ISR Registry Tests
# =========================================================================


class TestISRRegistry:
    """Verify handler registration and dispatch."""

    def test_register_and_dispatch(self) -> None:
        registry = ISRRegistry()
        call_count = 0

        def handler(frame: InterruptFrame, kernel: object) -> None:
            nonlocal call_count
            call_count += 1

        registry.register(INT_TIMER, handler)
        registry.dispatch(INT_TIMER, InterruptFrame(mcause=INT_TIMER), None)
        assert call_count == 1

    def test_handler_receives_frame(self) -> None:
        registry = ISRRegistry()
        received: list[InterruptFrame] = []

        def handler(frame: InterruptFrame, kernel: object) -> None:
            received.append(frame)

        registry.register(INT_TIMER, handler)
        frame = InterruptFrame(pc=0x1000, mcause=INT_TIMER, mstatus=0x1800)
        frame.registers[1] = 0xAAAA

        registry.dispatch(INT_TIMER, frame, None)

        assert len(received) == 1
        assert received[0].pc == 0x1000
        assert received[0].registers[1] == 0xAAAA

    def test_has_handler(self) -> None:
        registry = ISRRegistry()
        registry.register(INT_TIMER, lambda f, k: None)

        assert registry.has_handler(INT_TIMER)
        assert not registry.has_handler(INT_KEYBOARD)

    def test_overwrite(self) -> None:
        registry = ISRRegistry()
        first_called = False
        second_called = False

        def first(frame: InterruptFrame, kernel: object) -> None:
            nonlocal first_called
            first_called = True

        def second(frame: InterruptFrame, kernel: object) -> None:
            nonlocal second_called
            second_called = True

        registry.register(INT_TIMER, first)
        registry.register(INT_TIMER, second)
        registry.dispatch(INT_TIMER, InterruptFrame(), None)

        assert not first_called
        assert second_called

    def test_dispatch_missing_raises(self) -> None:
        registry = ISRRegistry()
        with pytest.raises(KeyError):
            registry.dispatch(INT_TIMER, InterruptFrame(), None)


# =========================================================================
# Interrupt Controller Tests
# =========================================================================


class TestInterruptController:
    """Verify controller behavior: raise, pending, acknowledge, mask, enable."""

    def test_raise_interrupt(self) -> None:
        ic = InterruptController()
        ic.raise_interrupt(INT_TIMER)
        assert ic.pending_count() == 1

    def test_has_pending(self) -> None:
        ic = InterruptController()
        assert not ic.has_pending()
        ic.raise_interrupt(INT_TIMER)
        assert ic.has_pending()

    def test_next_pending_priority(self) -> None:
        ic = InterruptController()
        ic.raise_interrupt(INT_KEYBOARD)  # 33
        ic.raise_interrupt(INT_TIMER)  # 32
        assert ic.next_pending() == INT_TIMER

    def test_acknowledge(self) -> None:
        ic = InterruptController()
        ic.raise_interrupt(INT_TIMER)
        ic.acknowledge(INT_TIMER)
        assert ic.pending_count() == 0

    def test_no_duplicates(self) -> None:
        ic = InterruptController()
        ic.raise_interrupt(INT_TIMER)
        ic.raise_interrupt(INT_TIMER)
        assert ic.pending_count() == 1

    def test_mask(self) -> None:
        ic = InterruptController()
        ic.set_mask(INT_INVALID_OPCODE, True)
        ic.raise_interrupt(INT_INVALID_OPCODE)

        assert ic.pending_count() == 1
        assert not ic.has_pending()
        assert ic.next_pending() == -1

    def test_unmask(self) -> None:
        ic = InterruptController()
        ic.set_mask(INT_INVALID_OPCODE, True)
        ic.raise_interrupt(INT_INVALID_OPCODE)
        assert not ic.has_pending()

        ic.set_mask(INT_INVALID_OPCODE, False)
        assert ic.has_pending()

    def test_is_masked(self) -> None:
        ic = InterruptController()
        assert not ic.is_masked(5)
        ic.set_mask(5, True)
        assert ic.is_masked(5)
        # Interrupts 32+ are never masked by mask register
        assert not ic.is_masked(INT_TIMER)

    def test_global_disable(self) -> None:
        ic = InterruptController()
        ic.disable()
        ic.raise_interrupt(INT_TIMER)
        assert not ic.has_pending()
        assert ic.next_pending() == -1

    def test_global_enable(self) -> None:
        ic = InterruptController()
        ic.disable()
        ic.raise_interrupt(INT_TIMER)
        ic.enable()
        assert ic.has_pending()

    def test_clear_all(self) -> None:
        ic = InterruptController()
        ic.raise_interrupt(INT_TIMER)
        ic.raise_interrupt(INT_KEYBOARD)
        ic.clear_all()
        assert ic.pending_count() == 0

    def test_mask_high_interrupt_ignored(self) -> None:
        ic = InterruptController()
        ic.set_mask(INT_TIMER, True)  # 32 is out of mask range
        ic.raise_interrupt(INT_TIMER)
        assert ic.has_pending()

    def test_next_pending_empty(self) -> None:
        ic = InterruptController()
        assert ic.next_pending() == -1


# =========================================================================
# Context Save/Restore Tests
# =========================================================================


class TestContextSaveRestore:
    """Verify InterruptFrame roundtrip."""

    def test_roundtrip(self) -> None:
        regs = [i * 100 for i in range(32)]
        pc = 0x00080000
        mstatus = 0x00001800
        mcause = INT_TIMER

        frame = save_context(regs, pc, mstatus, mcause)
        got_regs, got_pc, got_mstatus = restore_context(frame)

        assert got_pc == pc
        assert got_mstatus == mstatus
        for i in range(32):
            assert got_regs[i] == regs[i]

    def test_all_registers(self) -> None:
        regs = [0xDEAD0000 + i for i in range(32)]
        frame = save_context(regs, 0, 0, 0)
        got_regs, _, _ = restore_context(frame)
        for i in range(32):
            assert got_regs[i] == 0xDEAD0000 + i

    def test_mcause(self) -> None:
        frame = save_context([0] * 32, 0, 0, INT_TIMER)
        assert frame.mcause == INT_TIMER

    def test_defensive_copy(self) -> None:
        """Modifying original registers should not affect the frame."""
        regs = [42] * 32
        frame = save_context(regs, 0, 0, 0)
        regs[0] = 999
        assert frame.registers[0] == 42


# =========================================================================
# Priority Tests
# =========================================================================


class TestPriority:
    """Verify interrupt dispatch priority ordering."""

    def test_multiple_pending(self) -> None:
        ic = InterruptController()
        ic.raise_interrupt(INT_SYSCALL)  # 128
        ic.raise_interrupt(INT_KEYBOARD)  # 33
        ic.raise_interrupt(INT_INVALID_OPCODE)  # 5
        ic.raise_interrupt(INT_TIMER)  # 32

        expected = [INT_INVALID_OPCODE, INT_TIMER, INT_KEYBOARD, INT_SYSCALL]
        for want in expected:
            got = ic.next_pending()
            assert got == want
            ic.acknowledge(got)

        assert ic.pending_count() == 0

    def test_acknowledge_and_next(self) -> None:
        ic = InterruptController()
        ic.raise_interrupt(INT_INVALID_OPCODE)  # 5
        ic.raise_interrupt(INT_TIMER)  # 32

        assert ic.next_pending() == INT_INVALID_OPCODE
        ic.acknowledge(INT_INVALID_OPCODE)
        assert ic.next_pending() == INT_TIMER


# =========================================================================
# Full Lifecycle Test
# =========================================================================


class TestFullLifecycle:
    """Simulate the complete interrupt lifecycle end-to-end."""

    def test_complete_cycle(self) -> None:
        ic = InterruptController()

        # Install timer ISR
        ic.idt.set_entry(
            INT_TIMER,
            IDTEntry(isr_address=0x00020100, present=True, privilege_level=0),
        )

        handler_called = False
        handler_frames: list[InterruptFrame] = []

        def timer_handler(frame: InterruptFrame, kernel: object) -> None:
            nonlocal handler_called
            handler_called = True
            handler_frames.append(frame)

        ic.registry.register(INT_TIMER, timer_handler)

        # Set up CPU state
        cpu_regs = [0] * 32
        cpu_regs[1] = 0x10000  # ra
        cpu_regs[2] = 0x7FFF0  # sp
        cpu_regs[10] = 42  # a0
        cpu_pc = 0x80000
        cpu_mstatus = 0x1800

        # Timer fires
        ic.raise_interrupt(INT_TIMER)
        assert ic.has_pending()

        int_num = ic.next_pending()
        assert int_num == INT_TIMER

        # Save context
        frame = save_context(cpu_regs, cpu_pc, cpu_mstatus, int_num)

        # Disable interrupts
        ic.disable()

        # Look up IDT
        idt_entry = ic.idt.get_entry(int_num)
        assert idt_entry.present
        assert idt_entry.isr_address == 0x00020100

        # Dispatch ISR
        ic.registry.dispatch(int_num, frame, None)
        assert handler_called
        assert handler_frames[0].mcause == INT_TIMER

        # Acknowledge
        ic.acknowledge(int_num)
        assert ic.pending_count() == 0

        # Restore context
        restored_regs, restored_pc, restored_mstatus = restore_context(frame)
        ic.enable()

        # Verify
        assert restored_pc == cpu_pc
        assert restored_mstatus == cpu_mstatus
        assert restored_regs[1] == 0x10000
        assert restored_regs[2] == 0x7FFF0
        assert restored_regs[10] == 42
