use arm7tdmi_gatelevel::{
    Arm7TdmiGateLevel, Arm7TdmiState as GateState, InstructionSet as GateSet, FLIP_FLOP_COUNT,
};
use arm7tdmi_simulator::{
    Arm7TdmiSimulator, Arm7TdmiState as FunctionalState, InstructionSet as FunctionalSet,
};

fn assert_state_eq(gate: &GateState, functional: &FunctionalState, label: &str) {
    assert_eq!(
        gate.registers.common_r0_r7, functional.registers.common_r0_r7,
        "{label}: r0-r7"
    );
    assert_eq!(
        gate.registers.normal_r8_r12, functional.registers.normal_r8_r12,
        "{label}: normal r8-r12"
    );
    assert_eq!(
        gate.registers.fiq_r8_r14, functional.registers.fiq_r8_r14,
        "{label}: fiq"
    );
    assert_eq!(
        gate.registers.user_r13_r14, functional.registers.user_r13_r14,
        "{label}: user"
    );
    assert_eq!(
        gate.registers.irq_r13_r14, functional.registers.irq_r13_r14,
        "{label}: irq"
    );
    assert_eq!(
        gate.registers.supervisor_r13_r14, functional.registers.supervisor_r13_r14,
        "{label}: svc"
    );
    assert_eq!(
        gate.registers.abort_r13_r14, functional.registers.abort_r13_r14,
        "{label}: abort"
    );
    assert_eq!(
        gate.registers.undefined_r13_r14, functional.registers.undefined_r13_r14,
        "{label}: undefined"
    );
    assert_eq!(gate.registers.pc, functional.registers.pc, "{label}: pc");
    assert_eq!(gate.cpsr, functional.cpsr, "{label}: cpsr");
    assert_eq!(gate.spsr_fiq, functional.spsr_fiq, "{label}: spsr_fiq");
    assert_eq!(gate.spsr_irq, functional.spsr_irq, "{label}: spsr_irq");
    assert_eq!(
        gate.spsr_supervisor, functional.spsr_supervisor,
        "{label}: spsr_svc"
    );
    assert_eq!(
        gate.spsr_abort, functional.spsr_abort,
        "{label}: spsr_abort"
    );
    assert_eq!(
        gate.spsr_undefined, functional.spsr_undefined,
        "{label}: spsr_und"
    );
    assert_eq!(gate.memory, functional.memory, "{label}: memory");
    assert_eq!(gate.halted, functional.halted, "{label}: halt");
    assert_eq!(
        gate.loaded_origin, functional.loaded_origin,
        "{label}: origin"
    );
    assert_eq!(gate.loaded_len, functional.loaded_len, "{label}: length");
}

fn arm_program(raw: u32) -> Vec<u8> {
    [raw, 0xef00_0000]
        .into_iter()
        .flat_map(u32::to_le_bytes)
        .collect()
}

fn thumb_program(raw: u16) -> Vec<u8> {
    [raw, 0xdf00]
        .into_iter()
        .flat_map(u16::to_le_bytes)
        .collect()
}

fn seed_pair_arm(raw: u32) -> (Arm7TdmiGateLevel, Arm7TdmiSimulator) {
    let program = arm_program(raw);
    let mut gate = Arm7TdmiGateLevel::new();
    let mut functional = Arm7TdmiSimulator::new();
    gate.load(&program, 0, GateSet::Arm).unwrap();
    functional.load(&program, 0, FunctionalSet::Arm).unwrap();
    seed_registers(&mut gate, &mut functional);
    (gate, functional)
}

fn seed_pair_thumb(raw: u16) -> (Arm7TdmiGateLevel, Arm7TdmiSimulator) {
    let program = thumb_program(raw);
    let mut gate = Arm7TdmiGateLevel::new();
    let mut functional = Arm7TdmiSimulator::new();
    gate.load(&program, 0, GateSet::Thumb).unwrap();
    functional.load(&program, 0, FunctionalSet::Thumb).unwrap();
    seed_registers(&mut gate, &mut functional);
    (gate, functional)
}

fn seed_registers(gate: &mut Arm7TdmiGateLevel, functional: &mut Arm7TdmiSimulator) {
    let values = [
        0x8000_0001,
        0x400,
        3,
        31,
        0xffff_ffff,
        0x7fff_ffff,
        0x1234_5678,
        0x200,
        8,
        9,
        10,
        11,
        12,
        0x600,
        0x100,
    ];
    for (register, value) in values.into_iter().enumerate() {
        gate.write_register(register, value).unwrap();
        functional.write_register(register, value).unwrap();
    }
    let bytes: Vec<u8> = (0..64).map(|index| index as u8 ^ 0xa5).collect();
    gate.write_memory(0x400, &bytes).unwrap();
    functional.write_memory(0x400, &bytes).unwrap();
}

fn compare_arm(raw: u32, label: &str) {
    let (mut gate, mut functional) = seed_pair_arm(raw);
    let before_gate = gate.get_state();
    let before_functional = functional.get_state();
    assert_state_eq(&before_gate, &before_functional, label);
    match (gate.step(), functional.step()) {
        (Ok(g), Ok(f)) => {
            assert_eq!(g.pc_before, f.pc_before, "{label}: pc before");
            assert_eq!(g.pc_after, f.pc_after, "{label}: pc after");
            assert_eq!(
                format!("{:?}", g.instruction_set),
                format!("{:?}", f.instruction_set)
            );
            assert_eq!(g.width, f.width, "{label}: width");
            assert_eq!(g.fetched, f.fetched, "{label}: fetched");
            assert_eq!(g.raw, f.raw, "{label}: raw");
            assert_eq!(g.mnemonic, f.mnemonic, "{label}: mnemonic");
            assert_eq!(g.condition_met, f.condition_met, "{label}: condition");
            assert_state_eq(&g.state_before, &f.state_before, label);
            assert_state_eq(&g.state_after, &f.state_after, label);
        }
        (Err(g), Err(f)) => assert_eq!(format!("{g:?}"), format!("{f:?}"), "{label}: error"),
        (g, f) => panic!("{label}: differential result mismatch gate={g:?} functional={f:?}"),
    }
    assert_state_eq(&gate.get_state(), &functional.get_state(), label);
}

fn compare_thumb(raw: u16, label: &str) {
    let (mut gate, mut functional) = seed_pair_thumb(raw);
    match (gate.step(), functional.step()) {
        (Ok(g), Ok(f)) => {
            assert_eq!(g.pc_before, f.pc_before, "{label}: pc before");
            assert_eq!(g.pc_after, f.pc_after, "{label}: pc after");
            assert_eq!(g.width, f.width, "{label}: width");
            assert_eq!(g.fetched, f.fetched, "{label}: fetched");
            assert_eq!(g.raw, f.raw, "{label}: raw");
            assert_eq!(g.mnemonic, f.mnemonic, "{label}: mnemonic");
            assert_eq!(g.condition_met, f.condition_met, "{label}: condition");
            assert_state_eq(&g.state_before, &f.state_before, label);
            assert_state_eq(&g.state_after, &f.state_after, label);
        }
        (Err(g), Err(f)) => assert_eq!(format!("{g:?}"), format!("{f:?}"), "{label}: error"),
        (g, f) => panic!("{label}: differential result mismatch gate={g:?} functional={f:?}"),
    }
    assert_state_eq(&gate.get_state(), &functional.get_state(), label);
}

#[test]
fn exact_storage_inventory_and_production_manifest() {
    assert_eq!(FLIP_FLOP_COUNT, 525_522);
    let manifest = include_str!("../Cargo.toml");
    let dependencies = manifest
        .split("[dependencies]")
        .nth(1)
        .unwrap()
        .split("[dev-dependencies]")
        .next()
        .unwrap();
    assert!(!dependencies.contains("arm7tdmi-simulator"));
}

#[test]
fn every_arm_family_has_a_full_state_and_trace_differential() {
    let mut cases = vec![
        (0xe1a0_0000, "mov-register"),
        (0xe3a0_0001, "mov-immediate"),
        (0xe080_2001, "add"),
        (0xe040_2001, "sub"),
        (0xe000_2001, "and"),
        (0xe020_2001, "eor"),
        (0xe180_2001, "orr"),
        (0xe1c0_2001, "bic"),
        (0xe1e0_2001, "mvn"),
        (0xe150_0001, "cmp"),
        (0xe110_0001, "tst"),
        (0xe000_0291, "mul"),
        (0xe023_4291, "mla"),
        (0xe086_5291, "umull"),
        (0xe0c8_7291, "smull"),
        (0xe591_2000, "ldr"),
        (0xe581_2000, "str"),
        (0xe5d1_2000, "ldrb"),
        (0xe5c1_2000, "strb"),
        (0xe1d1_20b0, "ldrh"),
        (0xe1c1_20b0, "strh"),
        (0xe1d1_20d0, "ldrsb"),
        (0xe1d1_20f0, "ldrsh"),
        (0xe891_0005, "ldmia"),
        (0xe881_0005, "stmia"),
        (0xea00_0000, "branch"),
        (0xeb00_0000, "branch-link"),
        (0xe12f_ff10, "bx"),
        (0xe10f_2000, "mrs"),
        (0xe121_f002, "msr"),
        (0xe101_2093, "swp"),
        (0xef00_0000, "halt"),
        (0xef00_0001, "unsupported-swi"),
        (0xee00_0010, "coprocessor"),
        (0xffff_ffff, "reserved-nv"),
    ];
    for condition in 0..16u32 {
        cases.push((condition << 28 | 0x03a0_0001, "condition-mov"));
    }
    for opcode in 0..16u32 {
        cases.push((0xe200_0001 | opcode << 21, "data-processing-opcode"));
    }
    for (raw, label) in cases {
        compare_arm(raw, label);
    }
}

#[test]
fn every_thumb_family_has_a_full_state_and_trace_differential() {
    let cases = [
        0x0048, 0x0848, 0x1048, 0x1888, 0x1e88, 0x202a, 0x282a, 0x3001, 0x3801, 0x4008, 0x4048,
        0x4088, 0x40c8, 0x4108, 0x4148, 0x4188, 0x41c8, 0x4208, 0x4248, 0x4288, 0x42c8, 0x4308,
        0x4348, 0x4388, 0x43c8, 0x4408, 0x4588, 0x4688, 0x4700, 0x4801, 0x500a, 0x520a, 0x540a,
        0x560a, 0x580a, 0x5a0a, 0x5c0a, 0x5e0a, 0x6008, 0x680a, 0x7008, 0x780b, 0x8008, 0x880c,
        0x9001, 0x9800, 0xa101, 0xa901, 0xb001, 0xb081, 0xb503, 0xbc0c, 0xbd00, 0xc106, 0xd100,
        0xde00, 0xdf00, 0xe000, 0xf000, 0xf801, 0xbe00, 0xffff,
    ];
    for raw in cases {
        compare_thumb(raw, "thumb-family");
    }
}
