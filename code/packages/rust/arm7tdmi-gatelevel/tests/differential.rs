use arm7tdmi_gatelevel::{
    Arm7TdmiError as GateError, Arm7TdmiGateLevel, Arm7TdmiState as GateState,
    InstructionSet as GateSet, Mode as GateMode, CPSR_T as GATE_CPSR_T, FLIP_FLOP_COUNT,
    MAX_RETAINED_STEPS,
};
use arm7tdmi_simulator::{
    Arm7TdmiError as FunctionalError, Arm7TdmiSimulator, Arm7TdmiState as FunctionalState,
    InstructionSet as FunctionalSet, Mode as FunctionalMode, CPSR_T as FUNCTIONAL_CPSR_T,
};
use std::fmt::Debug;

fn assert_same_error<G: Debug, F: Debug, GE: Debug, FE: Debug>(
    gate: Result<G, GE>,
    functional: Result<F, FE>,
    label: &str,
) {
    match (gate, functional) {
        (Err(gate), Err(functional)) => {
            assert_eq!(format!("{gate:?}"), format!("{functional:?}"), "{label}")
        }
        (Ok(_), Ok(_)) => panic!("{label}: both implementations unexpectedly succeeded"),
        (Ok(_), Err(_)) => panic!("{label}: gate unexpectedly succeeded"),
        (Err(_), Ok(_)) => panic!("{label}: functional oracle unexpectedly succeeded"),
    }
}

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
        (Err(g), Err(f)) => {
            assert_eq!(format!("{g:?}"), format!("{f:?}"), "{label}: error");
            assert_eq!(gate.get_state(), before_gate, "{label}: gate atomicity");
            assert_eq!(
                functional.get_state(),
                before_functional,
                "{label}: functional atomicity"
            );
        }
        (g, f) => panic!("{label}: differential result mismatch gate={g:?} functional={f:?}"),
    }
    assert_state_eq(&gate.get_state(), &functional.get_state(), label);
}

fn compare_thumb(raw: u16, label: &str) {
    let (mut gate, mut functional) = seed_pair_thumb(raw);
    let before_gate = gate.get_state();
    let before_functional = functional.get_state();
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
        (Err(g), Err(f)) => {
            assert_eq!(format!("{g:?}"), format!("{f:?}"), "{label}: error");
            assert_eq!(gate.get_state(), before_gate, "{label}: gate atomicity");
            assert_eq!(
                functional.get_state(),
                before_functional,
                "{label}: functional atomicity"
            );
        }
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

#[test]
fn every_reachable_typed_fault_is_atomic_and_matches_the_oracle() {
    let mut gate = Arm7TdmiGateLevel::new();
    let mut functional = Arm7TdmiSimulator::new();
    for (program, origin, gate_set, functional_set, label) in [
        (
            &[][..],
            0,
            GateSet::Arm,
            FunctionalSet::Arm,
            "empty program",
        ),
        (
            &[0; 4][..],
            2,
            GateSet::Arm,
            FunctionalSet::Arm,
            "misaligned program",
        ),
        (
            &[0; 8][..],
            0xfffc,
            GateSet::Arm,
            FunctionalSet::Arm,
            "program out of range",
        ),
    ] {
        let gate_before = gate.get_state();
        let functional_before = functional.get_state();
        assert_same_error(
            gate.load(program, origin, gate_set),
            functional.load(program, origin, functional_set),
            label,
        );
        assert_state_eq(&gate.get_state(), &functional.get_state(), label);
        assert_eq!(gate.get_state(), gate_before, "{label}: gate atomicity");
        assert_eq!(
            functional.get_state(),
            functional_before,
            "{label}: oracle atomicity"
        );
    }

    let mut gate_invalid = gate.get_state();
    let mut functional_invalid = functional.get_state();
    gate_invalid.memory.pop();
    functional_invalid.memory.pop();
    let gate_before = gate.get_state();
    let functional_before = functional.get_state();
    assert_same_error(
        gate.restore(gate_invalid),
        functional.restore(functional_invalid),
        "invalid state",
    );
    assert_eq!(
        gate.get_state(),
        gate_before,
        "invalid state: gate atomicity"
    );
    assert_eq!(
        functional.get_state(),
        functional_before,
        "invalid state: functional atomicity"
    );
    assert_same_error(
        gate.read_register(16),
        functional.read_register(16),
        "invalid register",
    );
    assert_eq!(
        gate.get_state(),
        gate_before,
        "invalid register: gate state"
    );
    assert_eq!(
        functional.get_state(),
        functional_before,
        "invalid register: functional state"
    );

    let mut gate_reserved = gate.get_state();
    let mut functional_reserved = functional.get_state();
    gate_reserved.cpsr = 0;
    functional_reserved.cpsr = 0;
    assert_same_error(
        gate.restore(gate_reserved),
        functional.restore(functional_reserved),
        "reserved mode",
    );
    assert_eq!(
        gate.get_state(),
        gate_before,
        "reserved mode: gate atomicity"
    );
    assert_eq!(
        functional.get_state(),
        functional_before,
        "reserved mode: functional atomicity"
    );

    // ADD HIGH with neither high-register selector set is architecturally
    // unpredictable in this bounded profile and must roll back atomically.
    compare_thumb(0x4408, "unpredictable Thumb high-register form");

    let (mut gate, mut functional) = seed_pair_arm(0xe14f_0000); // MRS r0,SPSR in User mode.
    let mut gate_user = gate.get_state();
    let mut functional_user = functional.get_state();
    gate_user.cpsr = GateMode::User as u32;
    functional_user.cpsr = FunctionalMode::User as u32;
    gate.restore(gate_user).unwrap();
    functional.restore(functional_user).unwrap();
    let gate_before = gate.get_state();
    let functional_before = functional.get_state();
    assert_same_error(gate.step(), functional.step(), "unavailable SPSR");
    assert_state_eq(
        &gate.get_state(),
        &functional.get_state(),
        "unavailable SPSR",
    );
    assert_eq!(gate.get_state(), gate_before);
    assert_eq!(functional.get_state(), functional_before);

    for (raw, label) in [
        (0xffff_ffff, "unknown instruction"),
        (0xee00_0010, "unsupported instruction"),
        (0xef00_0001, "unsupported exception"),
    ] {
        compare_arm(raw, label);
    }

    for (register_zero, label) in [(2, "misaligned target"), (0x1_0000, "target out of range")] {
        let (mut gate, mut functional) = seed_pair_arm(0xe12f_ff10); // BX r0.
        gate.write_register(0, register_zero).unwrap();
        functional.write_register(0, register_zero).unwrap();
        let gate_before = gate.get_state();
        let functional_before = functional.get_state();
        assert_same_error(gate.step(), functional.step(), label);
        assert_state_eq(&gate.get_state(), &functional.get_state(), label);
        assert_eq!(gate.get_state(), gate_before, "{label}: gate atomicity");
        assert_eq!(
            functional.get_state(),
            functional_before,
            "{label}: oracle atomicity"
        );
    }

    for (address, label) in [(1, "misaligned data"), (0x1_0000, "data out of range")] {
        let (mut gate, mut functional) = seed_pair_arm(0xe591_0000); // LDR r0,[r1].
        gate.write_register(1, address).unwrap();
        functional.write_register(1, address).unwrap();
        let gate_before = gate.get_state();
        let functional_before = functional.get_state();
        assert_same_error(gate.step(), functional.step(), label);
        assert_state_eq(&gate.get_state(), &functional.get_state(), label);
        assert_eq!(gate.get_state(), gate_before, "{label}: gate atomicity");
        assert_eq!(
            functional.get_state(),
            functional_before,
            "{label}: oracle atomicity"
        );
    }

    let mut gate = Arm7TdmiGateLevel::new();
    let mut functional = Arm7TdmiSimulator::new();
    gate.load(&0xe1a0_0000u32.to_le_bytes(), 0, GateSet::Arm)
        .unwrap();
    functional
        .load(&0xe1a0_0000u32.to_le_bytes(), 0, FunctionalSet::Arm)
        .unwrap();
    gate.step().unwrap();
    functional.step().unwrap();
    let gate_before = gate.get_state();
    let functional_before = functional.get_state();
    assert_same_error(gate.step(), functional.step(), "fetch outside program");
    assert_eq!(gate.get_state(), gate_before, "fetch: gate atomicity");
    assert_eq!(
        functional.get_state(),
        functional_before,
        "fetch: functional atomicity"
    );

    let (mut gate, mut functional) = seed_pair_arm(0xef00_0000);
    gate.step().unwrap();
    functional.step().unwrap();
    let gate_before = gate.get_state();
    let functional_before = functional.get_state();
    assert_same_error(gate.step(), functional.step(), "halted");
    assert_eq!(gate.get_state(), gate_before, "halted: gate atomicity");
    assert_eq!(
        functional.get_state(),
        functional_before,
        "halted: functional atomicity"
    );

    let (mut gate, mut functional) = seed_pair_arm(0xe1a0_0000);
    let gate_before = gate.get_state();
    let functional_before = functional.get_state();
    assert_same_error(gate.run(1), functional.run(1), "step limit");
    assert_eq!(gate.get_state(), gate_before, "step limit: gate atomicity");
    assert_eq!(
        functional.get_state(),
        functional_before,
        "step limit: functional atomicity"
    );
    assert_same_error(
        gate.run(MAX_RETAINED_STEPS + 1),
        functional.run(MAX_RETAINED_STEPS + 1),
        "retention limit",
    );
    assert_eq!(gate.get_state(), gate_before, "retention: gate atomicity");
    assert_eq!(
        functional.get_state(),
        functional_before,
        "retention: functional atomicity"
    );
}

#[test]
fn compatibility_only_faults_are_narrowed_and_shape_locked() {
    // Every publicly reachable functional fault is executed above.  These two
    // variants remain for API parity but cannot escape a public gate call:
    // PrivilegedInstruction has no production constructor in this bounded
    // profile, while public restore rejects a misaligned PC as InvalidState
    // before the private fetch boundary can observe it.
    let privileged_gate = GateError::PrivilegedInstruction { raw: 0, pc: 0 };
    let privileged_functional = FunctionalError::PrivilegedInstruction { raw: 0, pc: 0 };
    assert_eq!(
        format!("{privileged_gate:?}"),
        format!("{privileged_functional:?}")
    );
    assert_eq!(
        privileged_gate.to_string(),
        privileged_functional.to_string()
    );

    let fetch_gate = GateError::MisalignedFetch {
        pc: 2,
        alignment: 4,
    };
    let fetch_functional = FunctionalError::MisalignedFetch {
        pc: 2,
        alignment: 4,
    };
    assert_eq!(format!("{fetch_gate:?}"), format!("{fetch_functional:?}"));
    assert_eq!(fetch_gate.to_string(), fetch_functional.to_string());

    let (mut gate, mut functional) = seed_pair_arm(0xe1a0_0000);
    let gate_before = gate.get_state();
    let functional_before = functional.get_state();
    let mut bad_gate = gate_before.clone();
    let mut bad_functional = functional_before.clone();
    bad_gate.registers.pc = 2;
    bad_functional.registers.pc = 2;
    assert_same_error(
        gate.restore(bad_gate),
        functional.restore(bad_functional),
        "misaligned public restore is InvalidState, not MisalignedFetch",
    );
    assert_eq!(gate.get_state(), gate_before);
    assert_eq!(functional.get_state(), functional_before);

    assert!(!GateError::CommitFailed.to_string().is_empty());
}

#[test]
fn functional_completion_matrix_covers_modes_exchange_and_lifecycle() {
    // Every bank/mode path is exercised through the public API and compared
    // after a transition, complementing the family/opcode matrix above.
    for (gate_mode, functional_mode) in [
        (GateMode::User, FunctionalMode::User),
        (GateMode::Fiq, FunctionalMode::Fiq),
        (GateMode::Irq, FunctionalMode::Irq),
        (GateMode::Supervisor, FunctionalMode::Supervisor),
        (GateMode::Abort, FunctionalMode::Abort),
        (GateMode::Undefined, FunctionalMode::Undefined),
        (GateMode::System, FunctionalMode::System),
    ] {
        let (mut gate, mut functional) = seed_pair_arm(0xe1a0_0000);
        let mut gate_state = gate.get_state();
        let mut functional_state = functional.get_state();
        gate_state.cpsr = gate_mode as u32;
        functional_state.cpsr = functional_mode as u32;
        gate.restore(gate_state).unwrap();
        functional.restore(functional_state).unwrap();
        for register in 0..15 {
            let value = 0x1000 + register as u32;
            gate.write_register(register, value).unwrap();
            functional.write_register(register, value).unwrap();
        }
        let gate_trace = gate.step().unwrap();
        let functional_trace = functional.step().unwrap();
        assert_eq!(gate_trace.mnemonic, functional_trace.mnemonic);
        assert_state_eq(&gate.get_state(), &functional.get_state(), "mode matrix");
    }

    // ARM -> Thumb exchange and mixed-width continuation.
    let mut gate = Arm7TdmiGateLevel::new();
    let mut functional = Arm7TdmiSimulator::new();
    let mut program = 0xe12f_ff10u32.to_le_bytes().to_vec(); // BX r0.
    program.extend_from_slice(&0x2001u16.to_le_bytes()); // MOVS r0,#1.
    program.extend_from_slice(&[0; 2]); // ARM image transport remains word-sized.
    gate.load(&program, 0, GateSet::Arm).unwrap();
    functional.load(&program, 0, FunctionalSet::Arm).unwrap();
    gate.write_register(0, 5).unwrap();
    functional.write_register(0, 5).unwrap();
    for label in ["exchange", "thumb continuation"] {
        let gate_trace = gate.step().unwrap();
        let functional_trace = functional.step().unwrap();
        assert_eq!(gate_trace.mnemonic, functional_trace.mnemonic, "{label}");
        assert_state_eq(&gate.get_state(), &functional.get_state(), label);
    }
    assert_eq!(gate.get_state().cpsr & GATE_CPSR_T, GATE_CPSR_T);
    assert_eq!(
        functional.get_state().cpsr & FUNCTIONAL_CPSR_T,
        FUNCTIONAL_CPSR_T
    );

    let halt = 0xef00_0000u32.to_le_bytes();
    let mut gate = Arm7TdmiGateLevel::new();
    let mut functional = Arm7TdmiSimulator::new();
    let gate_result = gate.execute(&halt, 0, GateSet::Arm, 1).unwrap();
    let functional_result = functional.execute(&halt, 0, FunctionalSet::Arm, 1).unwrap();
    assert_eq!(gate_result.steps, functional_result.steps);
    assert_eq!(gate_result.halted, functional_result.halted);
    assert_state_eq(
        &gate.get_state(),
        &functional.get_state(),
        "execute lifecycle",
    );
}
