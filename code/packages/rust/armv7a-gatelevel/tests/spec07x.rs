use armv7a_gatelevel::Armv7AGateLevel;
use armv7a_simulator::{encoding, Armv7AState, LR, PC, SP};

fn raw16(value: u16) -> encoding::Instruction {
    encoding::Instruction::Thumb16(value)
}

fn execute(instructions: &[encoding::Instruction]) -> Armv7AState {
    let mut all = instructions.to_vec();
    all.push(encoding::halt());
    let mut cpu = Armv7AGateLevel::new();
    cpu.load_checked(&encoding::program(&all)).unwrap();
    cpu.run_checked(all.len() + 8).unwrap().final_state
}

#[test]
fn arithmetic_flags_cover_unsigned_and_signed_edges() {
    let state = execute(&[
        encoding::mov_imm8(0, 0xff),
        raw16(0x0200), // LSLS R0,R0,#8 = 0xff00
        raw16(0x0200), // = 0xff0000
        raw16(0x0200), // = 0xff000000
        encoding::add_imm8(0, 1),
    ]);
    assert_eq!(state.registers[0], 0xff00_0001);
    assert!(state.cpsr & (1 << 31) != 0);
    assert!(state.cpsr & (1 << 30) == 0);

    let mut cpu = Armv7AGateLevel::new();
    let bytes = encoding::program(&[encoding::add_imm8(0, 1), encoding::halt()]);
    cpu.load_checked(&bytes).unwrap();
    let mut seeded = cpu.get_state();
    seeded.registers[0] = u32::MAX;
    cpu.restore(&seeded).unwrap();
    cpu.step_checked().unwrap();
    let state = cpu.get_state();
    assert_eq!(state.registers[0], 0);
    assert!(state.cpsr & (1 << 30) != 0);
    assert!(state.cpsr & (1 << 29) != 0);
}

#[test]
fn all_fourteen_branch_conditions_take_and_skip() {
    let flags = [
        1 << 30,
        0,
        1 << 29,
        0,
        1 << 31,
        0,
        1 << 28,
        0,
        1 << 29,
        1 << 30,
        0,
        1 << 31,
        0,
        1 << 30,
    ];
    for (condition, required) in flags.into_iter().enumerate() {
        let mut cpu = Armv7AGateLevel::new();
        let bytes = encoding::program(&[
            encoding::branch_cond(condition as u8, 1),
            encoding::mov_imm8(0, 1),
            encoding::halt(),
        ]);
        cpu.load_checked(&bytes).unwrap();
        let mut state = cpu.get_state();
        state.cpsr = (1 << 5) | required;
        cpu.restore(&state).unwrap();
        cpu.run_checked(4).unwrap();
        assert_eq!(cpu.read_register(0), Ok(0), "condition {condition:x}");

        let inverse = condition ^ 1;
        let mut cpu = Armv7AGateLevel::new();
        let bytes = encoding::program(&[
            encoding::branch_cond(inverse as u8, 1),
            encoding::mov_imm8(0, 1),
            encoding::halt(),
        ]);
        cpu.load_checked(&bytes).unwrap();
        let mut state = cpu.get_state();
        state.cpsr = (1 << 5) | required;
        cpu.restore(&state).unwrap();
        cpu.run_checked(4).unwrap();
        assert_eq!(cpu.read_register(0), Ok(1), "inverse {inverse:x}");
    }
}

#[test]
fn wrapping_load_store_and_signed_loads_are_exact() {
    let mut cpu = Armv7AGateLevel::new();
    let bytes = encoding::program(&[
        raw16(0x5011), // STR R1,[R2,R0]
        raw16(0x5c13), // LDRB R3,[R2,R0]
        raw16(0x5614), // LDRSB R4,[R2,R0]
        encoding::halt(),
    ]);
    cpu.load_at_checked(&bytes, 0x100).unwrap();
    let mut state = cpu.get_state();
    state.registers[0] = 1;
    state.registers[1] = 0x1234_5680;
    state.registers[2] = 0xffff_fffe;
    cpu.restore(&state).unwrap();
    cpu.run_checked(8).unwrap();
    let state = cpu.get_state();
    assert_eq!(state.memory[0xffff], 0x80);
    assert_eq!(state.memory[0], 0x56);
    assert_eq!(state.memory[1], 0x34);
    assert_eq!(state.memory[2], 0x12);
    assert_eq!(state.registers[3], 0x80);
    assert_eq!(state.registers[4], 0xffff_ff80);
}

#[test]
fn stack_multiple_and_sp_relative_forms_round_trip() {
    let mut cpu = Armv7AGateLevel::new();
    let bytes = encoding::program(&[
        raw16(0xb503), // PUSH {R0,R1,LR}
        encoding::mov_imm8(0, 0),
        encoding::mov_imm8(1, 0),
        raw16(0xbd03), // POP {R0,R1,PC}
        encoding::halt(),
    ]);
    cpu.load_checked(&bytes).unwrap();
    let mut state = cpu.get_state();
    state.registers[0] = 11;
    state.registers[1] = 22;
    state.registers[LR] = 9;
    cpu.restore(&state).unwrap();
    cpu.run_checked(8).unwrap();
    let state = cpu.get_state();
    assert_eq!(state.registers[0], 11);
    assert_eq!(state.registers[1], 22);
    assert_eq!(state.registers[SP], 0xfff8);
    assert!(state.halted);

    let state = execute(&[raw16(0xa801)]); // ADD R0,SP,#4, manual-correct oracle defect
    assert_eq!(state.registers[0], 0xfffc);
}

#[test]
fn pc_reads_high_register_branch_and_link_are_coherent() {
    let mut cpu = Armv7AGateLevel::new();
    let bytes = encoding::program(&[
        encoding::branch_link(2),
        encoding::halt(),
        encoding::mov_imm8(0, 42),
        raw16(0x4770), // BX LR
    ]);
    cpu.load_checked(&bytes).unwrap();
    let result = cpu.run_checked(8).unwrap();
    assert_eq!(result.final_state.registers[0], 42);
    assert_eq!(result.final_state.registers[LR], 5);
    assert_eq!(result.final_state.registers[PC], result.final_state.pc);
}

#[test]
fn wide_moves_immediates_and_memory_execute() {
    let state = execute(&[
        encoding::movw(0, 0x1234),
        encoding::movt(0, 0x5678),
        encoding::raw32(0xf102, 0x0111), // ADD.W R1,R2,#0x11
        encoding::raw32(0xf8c2, 0x1003), // STR.W R1,[R2,#3]
        encoding::raw32(0xf8d2, 0x3003), // LDR.W R3,[R2,#3]
    ]);
    assert_eq!(state.registers[0], 0x5678_1234);
    assert_eq!(state.registers[1], 0x11);
    assert_eq!(state.registers[3], 0x11);
}
