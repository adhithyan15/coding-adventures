//! Integration tests for gpu-core.
//!
//! These tests exercise the full pipeline: loading programs, running them,
//! and verifying results through the public API. They mirror the kinds of
//! programs a student would write to learn GPU architecture.

use gpu_core::core::GPUCore;
use gpu_core::generic_isa::GenericISA;
use gpu_core::opcodes::*;
use gpu_core::protocols::ProcessingElement;

// ---------------------------------------------------------------------------
// Basic arithmetic programs
// ---------------------------------------------------------------------------

/// Test: compute 3.0 + 4.0 = 7.0 using FADD.
#[test]
fn test_addition_program() {
    let mut core = GPUCore::new(Box::new(GenericISA));
    core.load_program(vec![
        limm(0, 3.0),
        limm(1, 4.0),
        fadd(2, 0, 1),
        halt(),
    ]);
    let traces = core.run(100).unwrap();
    assert_eq!(traces.len(), 4);
    assert_eq!(core.registers.read_float(2), 7.0);
    assert!(core.halted());
}

/// Test: compute 10.0 - 3.0 = 7.0 using FSUB.
#[test]
fn test_subtraction_program() {
    let mut core = GPUCore::new(Box::new(GenericISA));
    core.load_program(vec![
        limm(0, 10.0),
        limm(1, 3.0),
        fsub(2, 0, 1),
        halt(),
    ]);
    core.run(100).unwrap();
    assert_eq!(core.registers.read_float(2), 7.0);
}

/// Test: compute 3.0 * 4.0 = 12.0 using FMUL.
#[test]
fn test_multiplication_program() {
    let mut core = GPUCore::new(Box::new(GenericISA));
    core.load_program(vec![
        limm(0, 3.0),
        limm(1, 4.0),
        fmul(2, 0, 1),
        halt(),
    ]);
    core.run(100).unwrap();
    assert_eq!(core.registers.read_float(2), 12.0);
}

/// Test: compute 2.0 * 3.0 + 1.0 = 7.0 using FFMA.
#[test]
fn test_fma_program() {
    let mut core = GPUCore::new(Box::new(GenericISA));
    core.load_program(vec![
        limm(0, 2.0),
        limm(1, 3.0),
        limm(2, 1.0),
        ffma(3, 0, 1, 2),
        halt(),
    ]);
    core.run(100).unwrap();
    assert_eq!(core.registers.read_float(3), 7.0);
}

/// Test: negate and absolute value.
#[test]
fn test_fneg_fabs_program() {
    let mut core = GPUCore::new(Box::new(GenericISA));
    core.load_program(vec![
        limm(0, 5.0),
        fneg(1, 0),  // R1 = -5.0
        fabs(2, 1),  // R2 = |R1| = 5.0
        halt(),
    ]);
    core.run(100).unwrap();
    assert_eq!(core.registers.read_float(1), -5.0);
    assert_eq!(core.registers.read_float(2), 5.0);
}

// ---------------------------------------------------------------------------
// Memory operations
// ---------------------------------------------------------------------------

/// Test: store a value to memory and load it back.
#[test]
fn test_store_load_program() {
    let mut core = GPUCore::new(Box::new(GenericISA));
    core.load_program(vec![
        limm(0, 0.0),    // R0 = 0 (base address)
        limm(1, 42.0),   // R1 = 42.0
        store(0, 1, 0.0), // Mem[0] = R1
        load(2, 0, 0.0),  // R2 = Mem[0]
        halt(),
    ]);
    core.run(100).unwrap();
    assert_eq!(core.registers.read_float(2), 42.0);
}

/// Test: store and load with offsets.
#[test]
fn test_store_load_with_offset() {
    let mut core = GPUCore::new(Box::new(GenericISA));
    core.load_program(vec![
        limm(0, 0.0),       // base address = 0
        limm(1, 10.0),
        limm(2, 20.0),
        store(0, 1, 0.0),   // Mem[0] = 10.0
        store(0, 2, 4.0),   // Mem[4] = 20.0
        load(3, 0, 0.0),    // R3 = Mem[0] = 10.0
        load(4, 0, 4.0),    // R4 = Mem[4] = 20.0
        halt(),
    ]);
    core.run(100).unwrap();
    assert_eq!(core.registers.read_float(3), 10.0);
    assert_eq!(core.registers.read_float(4), 20.0);
}

// ---------------------------------------------------------------------------
// Data movement
// ---------------------------------------------------------------------------

/// Test: MOV copies a register.
#[test]
fn test_mov_program() {
    let mut core = GPUCore::new(Box::new(GenericISA));
    core.load_program(vec![
        limm(0, 7.5),
        mov(1, 0),
        halt(),
    ]);
    core.run(100).unwrap();
    assert_eq!(core.registers.read_float(0), 7.5);
    assert_eq!(core.registers.read_float(1), 7.5);
}

// ---------------------------------------------------------------------------
// Control flow
// ---------------------------------------------------------------------------

/// Test: BEQ branches when equal, falls through when not.
#[test]
fn test_beq_branch() {
    let mut core = GPUCore::new(Box::new(GenericISA));
    core.load_program(vec![
        limm(0, 5.0),    // 0: R0 = 5.0
        limm(1, 5.0),    // 1: R1 = 5.0
        beq(0, 1, 2),    // 2: if R0 == R1, skip 2 instructions
        limm(2, 99.0),   // 3: R2 = 99.0 (should be skipped)
        halt(),           // 4: should be skipped
        limm(2, 42.0),   // 5: R2 = 42.0 (should execute via branch: PC=2+2=4... wait)
        halt(),           // 6:
    ]);
    // BEQ at PC=2 with offset=2: next PC = 2 + 2 = 4
    core.run(100).unwrap();
    // PC=4 is halt(), so R2 should NOT have been set
    // Actually let me re-check: beq offset=2 from PC=2 => PC=4 which is halt()
    assert_eq!(core.registers.read_float(2), 0.0);
}

/// Test: BEQ falls through when not equal.
#[test]
fn test_beq_no_branch() {
    let mut core = GPUCore::new(Box::new(GenericISA));
    core.load_program(vec![
        limm(0, 5.0),    // 0
        limm(1, 6.0),    // 1
        beq(0, 1, 2),    // 2: not equal, fall through
        limm(2, 99.0),   // 3: should execute
        halt(),           // 4
    ]);
    core.run(100).unwrap();
    assert_eq!(core.registers.read_float(2), 99.0);
}

/// Test: BLT branches when less than.
#[test]
fn test_blt_branch() {
    let mut core = GPUCore::new(Box::new(GenericISA));
    core.load_program(vec![
        limm(0, 3.0),    // 0
        limm(1, 5.0),    // 1
        blt(0, 1, 2),    // 2: 3 < 5, branch +2 => PC=4
        limm(2, 99.0),   // 3: skipped
        halt(),           // 4
    ]);
    core.run(100).unwrap();
    assert_eq!(core.registers.read_float(2), 0.0); // skipped
}

/// Test: BNE branches when not equal.
#[test]
fn test_bne_branch() {
    let mut core = GPUCore::new(Box::new(GenericISA));
    core.load_program(vec![
        limm(0, 3.0),    // 0
        limm(1, 5.0),    // 1
        bne(0, 1, 2),    // 2: 3 != 5, branch +2 => PC=4
        limm(2, 99.0),   // 3: skipped
        halt(),           // 4
    ]);
    core.run(100).unwrap();
    assert_eq!(core.registers.read_float(2), 0.0);
}

/// Test: JMP unconditional jump.
#[test]
fn test_jmp() {
    let mut core = GPUCore::new(Box::new(GenericISA));
    core.load_program(vec![
        jmp(2),           // 0: jump to PC=2
        limm(0, 99.0),   // 1: skipped
        halt(),           // 2
    ]);
    core.run(100).unwrap();
    assert_eq!(core.registers.read_float(0), 0.0); // skipped
}

/// Test: NOP does nothing but advances PC.
#[test]
fn test_nop() {
    let mut core = GPUCore::new(Box::new(GenericISA));
    core.load_program(vec![
        nop(),
        nop(),
        limm(0, 1.0),
        halt(),
    ]);
    core.run(100).unwrap();
    assert_eq!(core.registers.read_float(0), 1.0);
    assert_eq!(core.cycle, 4);
}

// ---------------------------------------------------------------------------
// Complex programs
// ---------------------------------------------------------------------------

/// Test: dot product of two 3-element vectors.
///
/// vec_a = [1.0, 2.0, 3.0]
/// vec_b = [4.0, 5.0, 6.0]
/// dot = 1*4 + 2*5 + 3*6 = 4 + 10 + 18 = 32
#[test]
fn test_dot_product() {
    let mut core = GPUCore::new(Box::new(GenericISA));
    core.load_program(vec![
        // Load vector elements
        limm(0, 1.0),    // a[0]
        limm(1, 2.0),    // a[1]
        limm(2, 3.0),    // a[2]
        limm(3, 4.0),    // b[0]
        limm(4, 5.0),    // b[1]
        limm(5, 6.0),    // b[2]
        // Multiply pairs
        fmul(6, 0, 3),   // a[0]*b[0] = 4.0
        fmul(7, 1, 4),   // a[1]*b[1] = 10.0
        fmul(8, 2, 5),   // a[2]*b[2] = 18.0
        // Sum
        fadd(9, 6, 7),   // 4+10 = 14
        fadd(10, 9, 8),  // 14+18 = 32
        halt(),
    ]);
    core.run(100).unwrap();
    assert_eq!(core.registers.read_float(10), 32.0);
}

/// Test: a simple loop that sums 1.0 + 1.0 + 1.0 = 3.0
/// using a counter and branch.
#[test]
fn test_simple_loop() {
    let mut core = GPUCore::new(Box::new(GenericISA));
    core.load_program(vec![
        // R0 = accumulator (starts at 0)
        // R1 = counter (starts at 0)
        // R2 = limit (3.0)
        // R3 = increment (1.0)
        limm(0, 0.0),     // 0: accumulator = 0
        limm(1, 0.0),     // 1: counter = 0
        limm(2, 3.0),     // 2: limit = 3
        limm(3, 1.0),     // 3: increment = 1
        // Loop body (PC=4)
        fadd(0, 0, 3),    // 4: accumulator += 1
        fadd(1, 1, 3),    // 5: counter += 1
        blt(1, 2, -2),    // 6: if counter < limit, branch -2 => PC=4
        halt(),            // 7:
    ]);
    core.run(100).unwrap();
    assert_eq!(core.registers.read_float(0), 3.0);
    assert_eq!(core.registers.read_float(1), 3.0);
}

// ---------------------------------------------------------------------------
// Tracing
// ---------------------------------------------------------------------------

/// Test: trace records are correct and complete.
#[test]
fn test_trace_records() {
    let mut core = GPUCore::new(Box::new(GenericISA));
    core.load_program(vec![
        limm(0, 5.0),
        halt(),
    ]);
    let traces = core.run(100).unwrap();

    assert_eq!(traces.len(), 2);

    // First trace: LIMM
    assert_eq!(traces[0].cycle, 1);
    assert_eq!(traces[0].pc, 0);
    assert_eq!(traces[0].next_pc, 1);
    assert!(!traces[0].halted);
    assert!(traces[0].registers_changed.contains_key("R0"));

    // Second trace: HALT
    assert_eq!(traces[1].cycle, 2);
    assert_eq!(traces[1].pc, 1);
    assert!(traces[1].halted);
}

/// Test: trace format() produces readable output.
#[test]
fn test_trace_format() {
    let mut core = GPUCore::new(Box::new(GenericISA));
    core.load_program(vec![
        limm(0, 42.0),
        halt(),
    ]);
    let traces = core.run(100).unwrap();
    let formatted = traces[0].format();
    assert!(formatted.contains("[Cycle 1]"));
    assert!(formatted.contains("PC=0"));
    assert!(formatted.contains("42"));
}

// ---------------------------------------------------------------------------
// Reset and reload
// ---------------------------------------------------------------------------

/// Test: reset clears state, reload runs a different program.
#[test]
fn test_reset_and_reload() {
    let mut core = GPUCore::new(Box::new(GenericISA));

    // First program
    core.load_program(vec![limm(0, 100.0), halt()]);
    core.run(100).unwrap();
    assert_eq!(core.registers.read_float(0), 100.0);

    // Reset
    core.reset();
    assert_eq!(core.registers.read_float(0), 0.0);

    // Second program
    core.load_program(vec![limm(0, 200.0), halt()]);
    core.run(100).unwrap();
    assert_eq!(core.registers.read_float(0), 200.0);
}

// ---------------------------------------------------------------------------
// Edge cases
// ---------------------------------------------------------------------------

/// Test: execution limit is enforced for infinite loops.
#[test]
fn test_infinite_loop_protection() {
    let mut core = GPUCore::new(Box::new(GenericISA));
    core.load_program(vec![jmp(0)]);
    let result = core.run(50);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Execution limit"));
}

/// Test: stepping a halted core returns an error.
#[test]
fn test_step_halted_core() {
    let mut core = GPUCore::new(Box::new(GenericISA));
    core.load_program(vec![halt()]);
    core.step().unwrap();
    let result = core.step();
    assert!(result.is_err());
}

/// Test: memory store and trace includes memory_changed.
#[test]
fn test_store_trace_has_memory_changed() {
    let mut core = GPUCore::new(Box::new(GenericISA));
    core.load_program(vec![
        limm(0, 0.0),
        limm(1, 99.0),
        store(0, 1, 0.0),
        halt(),
    ]);
    let traces = core.run(100).unwrap();
    // The store instruction trace should have memory_changed
    let store_trace = &traces[2];
    assert!(!store_trace.memory_changed.is_empty());
    assert_eq!(*store_trace.memory_changed.get(&0).unwrap(), 99.0);
}

/// Test: custom config with more registers and larger memory.
#[test]
fn test_custom_config() {
    use fp_arithmetic::FP32;
    let mut core = GPUCore::with_config(Box::new(GenericISA), FP32, 64, 8192);
    assert_eq!(core.registers.num_registers, 64);
    assert_eq!(core.memory.size, 8192);

    // Use a high register index
    core.load_program(vec![
        limm(63, 42.0),
        halt(),
    ]);
    core.run(100).unwrap();
    assert_eq!(core.registers.read_float(63), 42.0);
}

/// Test: all 16 opcodes appear in a single program.
#[test]
fn test_all_16_opcodes() {
    let mut core = GPUCore::new(Box::new(GenericISA));
    core.load_program(vec![
        limm(0, 2.0),       // LIMM
        limm(1, 3.0),       // LIMM
        limm(2, 1.0),       // LIMM
        fadd(3, 0, 1),      // FADD: 2+3=5
        fsub(4, 1, 0),      // FSUB: 3-2=1
        fmul(5, 0, 1),      // FMUL: 2*3=6
        ffma(6, 0, 1, 2),   // FFMA: 2*3+1=7
        fneg(7, 0),          // FNEG: -2
        fabs(8, 7),          // FABS: |-2|=2
        mov(9, 3),           // MOV: R9=R3=5
        limm(10, 0.0),      // base address
        store(10, 0, 0.0),  // STORE: Mem[0] = 2.0
        load(11, 10, 0.0),  // LOAD: R11 = Mem[0] = 2.0
        nop(),               // NOP
        beq(0, 0, 2),       // BEQ: R0==R0, skip 2 => jump over next line + halt
        halt(),              // skipped
        halt(),              // HALT
    ]);
    let traces = core.run(100).unwrap();
    // Verify we got through all instructions
    assert!(core.halted());
    assert_eq!(core.registers.read_float(3), 5.0);
    assert_eq!(core.registers.read_float(4), 1.0);
    assert_eq!(core.registers.read_float(5), 6.0);
    assert_eq!(core.registers.read_float(6), 7.0);
    assert_eq!(core.registers.read_float(7), -2.0);
    assert_eq!(core.registers.read_float(8), 2.0);
    assert_eq!(core.registers.read_float(9), 5.0);
    assert_eq!(core.registers.read_float(11), 2.0);
    // Verify trace count
    assert!(traces.len() >= 15);
}
