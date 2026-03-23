//! Tests for the S03 Interrupt Handler crate.

use crate::*;

// =========================================================================
// IDT Tests
// =========================================================================

#[test]
fn test_new_idt_all_not_present() {
    let idt = InterruptDescriptorTable::new();
    for i in 0..256 {
        let entry = idt.get_entry(i);
        assert!(!entry.present, "entry {i} should not be present");
        assert_eq!(entry.isr_address, 0);
        assert_eq!(entry.privilege_level, 0);
    }
}

#[test]
fn test_idt_set_get_timer() {
    let mut idt = InterruptDescriptorTable::new();
    let entry = IDTEntry {
        isr_address: 0x00020100,
        present: true,
        privilege_level: 0,
    };
    idt.set_entry(INT_TIMER, entry);

    let got = idt.get_entry(INT_TIMER);
    assert_eq!(got.isr_address, 0x00020100);
    assert!(got.present);
    assert_eq!(got.privilege_level, 0);
}

#[test]
fn test_idt_boundary_entries() {
    let mut idt = InterruptDescriptorTable::new();
    idt.set_entry(0, IDTEntry { isr_address: 0x1000, present: true, privilege_level: 0 });
    idt.set_entry(255, IDTEntry { isr_address: 0xFF00, present: true, privilege_level: 1 });

    assert_eq!(idt.get_entry(0).isr_address, 0x1000);
    assert!(idt.get_entry(0).present);
    assert_eq!(idt.get_entry(255).isr_address, 0xFF00);
    assert_eq!(idt.get_entry(255).privilege_level, 1);
}

#[test]
fn test_idt_overwrite() {
    let mut idt = InterruptDescriptorTable::new();
    idt.set_entry(INT_TIMER, IDTEntry { isr_address: 0x1000, present: true, privilege_level: 0 });
    idt.set_entry(INT_TIMER, IDTEntry { isr_address: 0x2000, present: true, privilege_level: 0 });
    assert_eq!(idt.get_entry(INT_TIMER).isr_address, 0x2000);
}

// =========================================================================
// IDT Serialization Tests
// =========================================================================

#[test]
fn test_idt_write_to_memory() {
    let mut idt = InterruptDescriptorTable::new();
    idt.set_entry(0, IDTEntry { isr_address: 0x00001000, present: true, privilege_level: 0 });
    idt.set_entry(INT_TIMER, IDTEntry { isr_address: 0x00020100, present: true, privilege_level: 0 });
    idt.set_entry(INT_SYSCALL, IDTEntry { isr_address: 0xDEADBEEF, present: true, privilege_level: 1 });

    let mut memory = vec![0u8; IDT_SIZE + 100];
    idt.write_to_memory(&mut memory, 0);

    // Entry 0: address 0x00001000 little-endian
    assert_eq!(&memory[0..4], &[0x00, 0x10, 0x00, 0x00]);
    assert_eq!(memory[4], 0x01); // present

    // Entry 32 at offset 256
    let off = INT_TIMER * IDT_ENTRY_SIZE;
    assert_eq!(&memory[off..off + 4], &[0x00, 0x01, 0x02, 0x00]);

    // Entry 128 at offset 1024
    let off = INT_SYSCALL * IDT_ENTRY_SIZE;
    assert_eq!(&memory[off..off + 4], &[0xEF, 0xBE, 0xAD, 0xDE]);
    assert_eq!(memory[off + 5], 0x01); // privilege level
}

#[test]
fn test_idt_load_from_memory() {
    let mut memory = vec![0u8; IDT_SIZE];
    let off = 5 * IDT_ENTRY_SIZE;
    memory[off] = 0xBE;
    memory[off + 1] = 0xBA;
    memory[off + 2] = 0xFE;
    memory[off + 3] = 0xCA;
    memory[off + 4] = 0x01; // present

    let mut idt = InterruptDescriptorTable::new();
    idt.load_from_memory(&memory, 0);

    let got = idt.get_entry(5);
    assert_eq!(got.isr_address, 0xCAFEBABE);
    assert!(got.present);
}

#[test]
fn test_idt_roundtrip() {
    let mut original = InterruptDescriptorTable::new();
    original.set_entry(0, IDTEntry { isr_address: 0x1000, present: true, privilege_level: 0 });
    original.set_entry(INT_TIMER, IDTEntry { isr_address: 0x20100, present: true, privilege_level: 0 });
    original.set_entry(INT_SYSCALL, IDTEntry { isr_address: 0xDEAD, present: true, privilege_level: 1 });
    original.set_entry(255, IDTEntry { isr_address: 0xFFFF, present: true, privilege_level: 2 });

    let mut memory = vec![0u8; IDT_SIZE];
    original.write_to_memory(&mut memory, 0);

    let mut loaded = InterruptDescriptorTable::new();
    loaded.load_from_memory(&memory, 0);

    for i in 0..256 {
        assert_eq!(original.get_entry(i), loaded.get_entry(i), "entry {i} mismatch");
    }
}

#[test]
fn test_idt_endianness() {
    let mut idt = InterruptDescriptorTable::new();
    idt.set_entry(0, IDTEntry { isr_address: 0x04030201, present: true, privilege_level: 0 });

    let mut memory = vec![0u8; IDT_SIZE];
    idt.write_to_memory(&mut memory, 0);

    // Little-endian: least significant byte first
    assert_eq!(&memory[0..4], &[0x01, 0x02, 0x03, 0x04]);
}

// =========================================================================
// ISR Registry Tests
// =========================================================================

#[test]
fn test_isr_register_and_dispatch() {
    use std::cell::RefCell;
    use std::rc::Rc;

    let mut registry = ISRRegistry::new();
    let call_count = Rc::new(RefCell::new(0));
    let cc = call_count.clone();

    registry.register(INT_TIMER, Box::new(move |_frame| {
        *cc.borrow_mut() += 1;
    }));

    let mut frame = InterruptFrame { mcause: INT_TIMER as u32, ..Default::default() };
    registry.dispatch(INT_TIMER, &mut frame);

    assert_eq!(*call_count.borrow(), 1);
}

#[test]
fn test_isr_handler_receives_frame() {
    use std::cell::RefCell;
    use std::rc::Rc;

    let mut registry = ISRRegistry::new();
    let received_pc = Rc::new(RefCell::new(0u32));
    let rp = received_pc.clone();

    registry.register(INT_TIMER, Box::new(move |frame| {
        *rp.borrow_mut() = frame.pc;
    }));

    let mut frame = InterruptFrame {
        pc: 0x1000,
        mcause: INT_TIMER as u32,
        mstatus: 0x1800,
        ..Default::default()
    };
    frame.registers[1] = 0xAAAA;

    registry.dispatch(INT_TIMER, &mut frame);
    assert_eq!(*received_pc.borrow(), 0x1000);
}

#[test]
fn test_isr_has_handler() {
    let mut registry = ISRRegistry::new();
    registry.register(INT_TIMER, Box::new(|_| {}));

    assert!(registry.has_handler(INT_TIMER));
    assert!(!registry.has_handler(INT_KEYBOARD));
}

#[test]
fn test_isr_overwrite() {
    use std::cell::RefCell;
    use std::rc::Rc;

    let mut registry = ISRRegistry::new();
    let first_called = Rc::new(RefCell::new(false));
    let second_called = Rc::new(RefCell::new(false));

    let fc = first_called.clone();
    registry.register(INT_TIMER, Box::new(move |_| { *fc.borrow_mut() = true; }));
    let sc = second_called.clone();
    registry.register(INT_TIMER, Box::new(move |_| { *sc.borrow_mut() = true; }));

    let mut frame = InterruptFrame::default();
    registry.dispatch(INT_TIMER, &mut frame);

    assert!(!*first_called.borrow());
    assert!(*second_called.borrow());
}

#[test]
#[should_panic(expected = "No ISR handler registered")]
fn test_isr_dispatch_panics_on_missing() {
    let mut registry = ISRRegistry::new();
    let mut frame = InterruptFrame::default();
    registry.dispatch(INT_TIMER, &mut frame);
}

// =========================================================================
// Interrupt Controller Tests
// =========================================================================

#[test]
fn test_controller_raise_interrupt() {
    let mut ic = InterruptController::new();
    ic.raise_interrupt(INT_TIMER);
    assert_eq!(ic.pending_count(), 1);
}

#[test]
fn test_controller_has_pending() {
    let mut ic = InterruptController::new();
    assert!(!ic.has_pending());
    ic.raise_interrupt(INT_TIMER);
    assert!(ic.has_pending());
}

#[test]
fn test_controller_next_pending_priority() {
    let mut ic = InterruptController::new();
    ic.raise_interrupt(INT_KEYBOARD); // 33
    ic.raise_interrupt(INT_TIMER); // 32
    assert_eq!(ic.next_pending(), Some(INT_TIMER));
}

#[test]
fn test_controller_acknowledge() {
    let mut ic = InterruptController::new();
    ic.raise_interrupt(INT_TIMER);
    ic.acknowledge(INT_TIMER);
    assert_eq!(ic.pending_count(), 0);
}

#[test]
fn test_controller_no_duplicates() {
    let mut ic = InterruptController::new();
    ic.raise_interrupt(INT_TIMER);
    ic.raise_interrupt(INT_TIMER);
    assert_eq!(ic.pending_count(), 1);
}

#[test]
fn test_controller_mask() {
    let mut ic = InterruptController::new();
    ic.set_mask(INT_INVALID_OPCODE, true);
    ic.raise_interrupt(INT_INVALID_OPCODE);

    assert_eq!(ic.pending_count(), 1);
    assert!(!ic.has_pending());
    assert_eq!(ic.next_pending(), None);
}

#[test]
fn test_controller_unmask() {
    let mut ic = InterruptController::new();
    ic.set_mask(INT_INVALID_OPCODE, true);
    ic.raise_interrupt(INT_INVALID_OPCODE);
    assert!(!ic.has_pending());

    ic.set_mask(INT_INVALID_OPCODE, false);
    assert!(ic.has_pending());
}

#[test]
fn test_controller_is_masked() {
    let mut ic = InterruptController::new();
    assert!(!ic.is_masked(5));
    ic.set_mask(5, true);
    assert!(ic.is_masked(5));
    // Interrupts 32+ never masked
    assert!(!ic.is_masked(INT_TIMER));
}

#[test]
fn test_controller_global_disable() {
    let mut ic = InterruptController::new();
    ic.disable();
    ic.raise_interrupt(INT_TIMER);
    assert!(!ic.has_pending());
    assert_eq!(ic.next_pending(), None);
}

#[test]
fn test_controller_global_enable() {
    let mut ic = InterruptController::new();
    ic.disable();
    ic.raise_interrupt(INT_TIMER);
    ic.enable();
    assert!(ic.has_pending());
}

#[test]
fn test_controller_clear_all() {
    let mut ic = InterruptController::new();
    ic.raise_interrupt(INT_TIMER);
    ic.raise_interrupt(INT_KEYBOARD);
    ic.clear_all();
    assert_eq!(ic.pending_count(), 0);
}

#[test]
fn test_controller_mask_high_interrupt_ignored() {
    let mut ic = InterruptController::new();
    ic.set_mask(INT_TIMER, true); // 32 out of mask range
    ic.raise_interrupt(INT_TIMER);
    assert!(ic.has_pending());
}

#[test]
fn test_controller_next_pending_empty() {
    let ic = InterruptController::new();
    assert_eq!(ic.next_pending(), None);
}

// =========================================================================
// Context Save/Restore Tests
// =========================================================================

#[test]
fn test_context_roundtrip() {
    let mut regs = [0u32; 32];
    for i in 0..32 {
        regs[i] = (i as u32) * 100;
    }
    let pc = 0x00080000u32;
    let mstatus = 0x00001800u32;

    let frame = save_context(regs, pc, mstatus, INT_TIMER as u32);
    let (got_regs, got_pc, got_mstatus) = restore_context(&frame);

    assert_eq!(got_pc, pc);
    assert_eq!(got_mstatus, mstatus);
    for i in 0..32 {
        assert_eq!(got_regs[i], regs[i]);
    }
}

#[test]
fn test_context_all_registers() {
    let mut regs = [0u32; 32];
    for i in 0..32 {
        regs[i] = 0xDEAD0000 + i as u32;
    }

    let frame = save_context(regs, 0, 0, 0);
    let (got_regs, _, _) = restore_context(&frame);

    for i in 0..32 {
        assert_eq!(got_regs[i], 0xDEAD0000 + i as u32);
    }
}

#[test]
fn test_context_mcause() {
    let frame = save_context([0; 32], 0, 0, INT_TIMER as u32);
    assert_eq!(frame.mcause, INT_TIMER as u32);
}

// =========================================================================
// Priority Tests
// =========================================================================

#[test]
fn test_priority_multiple_pending() {
    let mut ic = InterruptController::new();
    ic.raise_interrupt(INT_SYSCALL);       // 128
    ic.raise_interrupt(INT_KEYBOARD);      // 33
    ic.raise_interrupt(INT_INVALID_OPCODE); // 5
    ic.raise_interrupt(INT_TIMER);          // 32

    let expected = [INT_INVALID_OPCODE, INT_TIMER, INT_KEYBOARD, INT_SYSCALL];
    for &want in &expected {
        let got = ic.next_pending().unwrap();
        assert_eq!(got, want);
        ic.acknowledge(got);
    }

    assert_eq!(ic.pending_count(), 0);
}

#[test]
fn test_priority_acknowledge_and_next() {
    let mut ic = InterruptController::new();
    ic.raise_interrupt(INT_INVALID_OPCODE); // 5
    ic.raise_interrupt(INT_TIMER); // 32

    assert_eq!(ic.next_pending(), Some(INT_INVALID_OPCODE));
    ic.acknowledge(INT_INVALID_OPCODE);
    assert_eq!(ic.next_pending(), Some(INT_TIMER));
}

// =========================================================================
// Full Lifecycle Test
// =========================================================================

#[test]
fn test_full_lifecycle() {
    use std::cell::RefCell;
    use std::rc::Rc;

    let mut ic = InterruptController::new();

    // Install timer ISR
    ic.idt.set_entry(INT_TIMER, IDTEntry {
        isr_address: 0x00020100,
        present: true,
        privilege_level: 0,
    });

    let handler_called = Rc::new(RefCell::new(false));
    let handler_mcause = Rc::new(RefCell::new(0u32));
    let hc = handler_called.clone();
    let hm = handler_mcause.clone();

    ic.registry.register(INT_TIMER, Box::new(move |frame| {
        *hc.borrow_mut() = true;
        *hm.borrow_mut() = frame.mcause;
    }));

    // Set up CPU state
    let mut cpu_regs = [0u32; 32];
    cpu_regs[1] = 0x10000;  // ra
    cpu_regs[2] = 0x7FFF0;  // sp
    cpu_regs[10] = 42;      // a0
    let cpu_pc = 0x80000u32;
    let cpu_mstatus = 0x1800u32;

    // Timer fires
    ic.raise_interrupt(INT_TIMER);
    assert!(ic.has_pending());

    let int_num = ic.next_pending().unwrap();
    assert_eq!(int_num, INT_TIMER);

    // Save context
    let mut frame = save_context(cpu_regs, cpu_pc, cpu_mstatus, int_num as u32);

    // Disable interrupts
    ic.disable();

    // Look up IDT
    let idt_entry = ic.idt.get_entry(int_num);
    assert!(idt_entry.present);
    assert_eq!(idt_entry.isr_address, 0x00020100);

    // Dispatch ISR
    ic.registry.dispatch(int_num, &mut frame);
    assert!(*handler_called.borrow());
    assert_eq!(*handler_mcause.borrow(), INT_TIMER as u32);

    // Acknowledge
    ic.acknowledge(int_num);
    assert_eq!(ic.pending_count(), 0);

    // Restore context
    let (restored_regs, restored_pc, restored_mstatus) = restore_context(&frame);
    ic.enable();

    // Verify
    assert_eq!(restored_pc, cpu_pc);
    assert_eq!(restored_mstatus, cpu_mstatus);
    assert_eq!(restored_regs[1], 0x10000);
    assert_eq!(restored_regs[2], 0x7FFF0);
    assert_eq!(restored_regs[10], 42);
}
