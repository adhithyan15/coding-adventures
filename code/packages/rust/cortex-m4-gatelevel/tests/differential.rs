use cortex_m3_gatelevel::encoding as m3_encoding;
use cortex_m3_gatelevel::CortexM3State as GateM3State;
use cortex_m4_gatelevel::{
    classify, encoding, CortexM3InstructionClass, CortexM4GateLevel, CortexM4State,
    InstructionClass, FLIP_FLOP_COUNT, MEMORY_SIZE,
};
use cortex_m4_simulator::{
    CortexM3State as OracleM3State, CortexM4Simulator, CortexM4State as OracleM4State,
};

const ORIGIN: u32 = 0x100;

fn image(words: &[u16]) -> Vec<u8> {
    words.iter().flat_map(|word| word.to_le_bytes()).collect()
}

fn core_to_oracle(state: &GateM3State) -> OracleM3State {
    OracleM3State {
        registers: state.registers,
        msp: state.msp,
        psp: state.psp,
        lr: state.lr,
        pc: state.pc,
        apsr: state.apsr,
        ipsr: state.ipsr,
        itstate: state.itstate,
        primask: state.primask,
        basepri: state.basepri,
        faultmask: state.faultmask,
        control: state.control,
        handler_mode: state.handler_mode,
        memory: state.memory.clone(),
        halted: state.halted,
        sleeping: state.sleeping,
        waiting_for_event: state.waiting_for_event,
        event_registered: state.event_registered,
        exclusive_address: state.exclusive_address,
        loaded_origin: state.loaded_origin,
        loaded_len: state.loaded_len,
    }
}

fn state_to_oracle(state: &CortexM4State) -> OracleM4State {
    OracleM4State {
        core: core_to_oracle(&state.core),
        ge: state.ge,
        fpu_enabled: state.fpu_enabled,
        s_registers: state.s_registers,
        fpscr: state.fpscr,
    }
}

fn paired(words: &[u16], fpu: bool) -> (CortexM4GateLevel, CortexM4Simulator) {
    let bytes = image(words);
    let mut gate = CortexM4GateLevel::new();
    let mut oracle = CortexM4Simulator::new();
    gate.set_fpu_enabled(fpu);
    oracle.set_fpu_enabled(fpu);
    gate.load(&bytes, ORIGIN).unwrap();
    oracle.load(&bytes, ORIGIN).unwrap();
    (gate, oracle)
}

fn compare_step(gate: &mut CortexM4GateLevel, oracle: &mut CortexM4Simulator) {
    let actual = gate.step().unwrap();
    let expected = oracle.step().unwrap();
    assert_eq!(actual.pc_before, expected.pc_before);
    assert_eq!(actual.pc_after, expected.pc_after);
    assert_eq!(actual.raw, expected.raw);
    assert_eq!(actual.width, expected.width);
    assert_eq!(actual.mnemonic, expected.mnemonic);
    assert_eq!(state_to_oracle(&actual.state_before), expected.state_before);
    assert_eq!(state_to_oracle(&actual.state_after), expected.state_after);
    assert_eq!(state_to_oracle(&gate.get_state()), oracle.get_state());
}

fn seeded_core(state: &mut GateM3State) {
    state.registers = [0; 13];
    state.registers[0] = 0x8000_0001;
    state.registers[1] = 0x201;
    state.registers[2] = 4;
    state.registers[3] = 0xffff_ff80;
    state.registers[4] = 0x200;
    state.msp = 0x400;
    state.psp = 0x500;
    state.lr = 0x105;
    state.apsr = 0x3000_0000;
    state.memory[0x200..0x208].copy_from_slice(&[0x80, 0xff, 0x34, 0x12, 7, 0, 0, 0]);
}

fn configure_inherited_pair(
    words: &[u16],
    configure: impl Fn(&mut GateM3State),
) -> (CortexM4GateLevel, CortexM4Simulator) {
    let (mut gate, mut oracle) = paired(words, true);
    let mut gate_state = gate.get_state();
    seeded_core(&mut gate_state.core);
    configure(&mut gate_state.core);
    gate_state.ge = 0b1010;
    gate_state.s_registers[31] = 0x7fc0_1234;
    gate_state.fpscr = 0x2000_0011;
    let oracle_state = state_to_oracle(&gate_state);
    gate.restore(gate_state).unwrap();
    oracle.restore(oracle_state).unwrap();
    (gate, oracle)
}

fn compare_inherited_transition(label: &str, words: &[u16], configure: impl Fn(&mut GateM3State)) {
    let (mut gate, mut oracle) = configure_inherited_pair(words, configure);
    match (gate.step(), oracle.step()) {
        (Ok(actual), Ok(expected)) => {
            assert_eq!(actual.pc_before, expected.pc_before, "{label}: pc before");
            assert_eq!(actual.pc_after, expected.pc_after, "{label}: pc after");
            assert_eq!(actual.raw, expected.raw, "{label}: raw");
            assert_eq!(actual.width, expected.width, "{label}: width");
            assert_eq!(actual.mnemonic, expected.mnemonic, "{label}: mnemonic");
            assert_eq!(
                state_to_oracle(&actual.state_before),
                expected.state_before,
                "{label}: complete state before"
            );
            assert_eq!(
                state_to_oracle(&actual.state_after),
                expected.state_after,
                "{label}: complete state after"
            );
        }
        (Err(actual), Err(expected)) => {
            assert_eq!(
                format!("{actual:?}"),
                format!("{expected:?}"),
                "{label}: typed error"
            );
        }
        (actual, expected) => panic!("{label}: gate={actual:?}, oracle={expected:?}"),
    }
    assert_eq!(
        state_to_oracle(&gate.get_state()),
        oracle.get_state(),
        "{label}: committed complete state"
    );
}

fn compare_inherited_sequence(
    label: &str,
    words: &[u16],
    steps: usize,
    configure: impl Fn(&mut GateM3State),
) {
    let (mut gate, mut oracle) = configure_inherited_pair(words, configure);
    for step in 0..steps {
        compare_step(&mut gate, &mut oracle);
        assert_eq!(
            state_to_oracle(&gate.get_state()),
            oracle.get_state(),
            "{label}: complete state after step {step}"
        );
    }
}

#[test]
fn topology_and_complete_reset_state_are_exact() {
    let mut gate = CortexM4GateLevel::new();
    assert_eq!(FLIP_FLOP_COUNT, 526_013);
    assert_eq!(gate.flip_flop_count(), 526_013);
    assert!(gate.gate_count() > gate.flip_flop_count() * 6);
    gate.set_fpu_enabled(true);
    gate.write_s_register(31, 0xdead_beef).unwrap();
    gate.write_register(3, 7).unwrap();
    gate.write_memory(8, 4, 0xaabb_ccdd).unwrap();
    gate.reset();
    assert_eq!(gate.get_state(), CortexM4State::default());
    assert!(gate.read_s_register(32).is_err());
}

#[test]
fn inherited_executor_matches_m3_backed_oracle() {
    // movs r0,#7; adds r0,#1; bkpt #0
    let (mut gate, mut oracle) = paired(&[0x2007, 0x3001, 0xbe00], false);
    compare_step(&mut gate, &mut oracle);
    compare_step(&mut gate, &mut oracle);
    compare_step(&mut gate, &mut oracle);
    assert!(gate.step().is_err());
}

#[test]
fn inherited_primary_families_execute_with_exact_full_trace_and_state() {
    let cases: Vec<(&str, Vec<u16>)> = vec![
        ("shift-add-subtract", vec![0x0008]),
        ("immediate", vec![0x2001]),
        ("data-processing", vec![0x4008]),
        ("special-data", vec![0x4408]),
        ("literal-load", vec![0x4800]),
        ("register-memory", vec![0x5020]),
        ("immediate-memory", vec![0x6020]),
        ("stack-memory", vec![0x9400]),
        ("address-generation", vec![0xa000]),
        ("miscellaneous", vec![0xb240]),
        ("multiple-memory", vec![0xc011]),
        ("conditional-branch", vec![0xd000]),
        ("unconditional-branch", vec![0xe000]),
        ("branch-link", m3_encoding::bl(0).to_vec()),
        ("barrier", vec![0xf3bf, 0x8f5f]),
        ("special-register", m3_encoding::mrs(0, 16).to_vec()),
        ("thumb2-data", m3_encoding::movw(0, 0x1234).to_vec()),
        ("thumb2-memory", vec![0xf8c4, 0x0004]),
        ("exclusive", m3_encoding::ldrex(0, 4, 0).to_vec()),
        ("table-branch", m3_encoding::tbb(4, 2).to_vec()),
    ];
    for (label, words) in cases {
        compare_inherited_transition(label, &words, |_| {});
    }
}

#[test]
fn inherited_privilege_it_memory_exclusive_and_branch_sequences_are_exact() {
    let mut privilege = Vec::new();
    privilege.extend(m3_encoding::msr(0, 16));
    privilege.extend(m3_encoding::mrs(1, 16));
    compare_inherited_sequence("privilege", &privilege, 2, |_| {});

    let it = [m3_encoding::it(0, 8), m3_encoding::movs(0, 9)];
    for apsr in [0, 0x4000_0000] {
        compare_inherited_sequence("it-execute-skip", &it, 2, |state| state.apsr = apsr);
    }

    let memory = [m3_encoding::str_imm(0, 4, 0), m3_encoding::ldr_imm(2, 4, 0)];
    compare_inherited_sequence("memory-store-load", &memory, 2, |_| {});

    let mut exclusive = Vec::new();
    exclusive.extend(m3_encoding::ldrex(0, 4, 0));
    exclusive.extend(m3_encoding::strex(1, 0, 4, 0));
    compare_inherited_sequence("exclusive-success", &exclusive, 2, |_| {});

    for apsr in [0, 0x4000_0000] {
        compare_inherited_transition("conditional-branch-outcome", &[0xd000], |state| {
            state.apsr = apsr;
        });
    }
}

#[test]
fn inherited_power_signals_and_faults_match_with_extension_state_intact() {
    for instruction in [0xbf20, 0xbf30] {
        let (mut gate, mut oracle) = configure_inherited_pair(&[instruction], |_| {});
        compare_step(&mut gate, &mut oracle);
        if instruction == 0xbf20 {
            gate.signal_event();
            oracle.signal_event();
        } else {
            gate.signal_interrupt();
            oracle.signal_interrupt();
        }
        assert_eq!(state_to_oracle(&gate.get_state()), oracle.get_state());
    }

    compare_inherited_transition("unknown-32", &[0xf000, 0], |_| {});
    compare_inherited_transition("misaligned-data", &[m3_encoding::ldr_imm(0, 1, 0)], |_| {});
    compare_inherited_transition("branch-outside-program", &[0x4708], |_| {});
}

#[test]
fn every_dsp_family_is_gate_differential() {
    let cases = [
        encoding::qadd(0, 1, 2),
        encoding::qsub(0, 1, 2),
        encoding::ssat(0, 8, 1),
        encoding::usat(0, 8, 1),
        encoding::sadd8(0, 1, 2),
        encoding::uadd8(0, 1, 2),
        encoding::smuad(0, 1, 2),
        encoding::smlad(0, 1, 2, 3),
    ];
    for words in cases {
        let (mut gate, mut oracle) = paired(&words, false);
        for (index, value) in [0x7fff_ffff, 0x0001_ffff, 0x7fff_7fff, 0x4000_0000]
            .into_iter()
            .enumerate()
        {
            gate.write_register(index as u8, value).unwrap();
            oracle.write_register(index as u8, value).unwrap();
        }
        compare_step(&mut gate, &mut oracle);
    }
}

#[test]
fn binary_fp_network_matches_oracle_edges() {
    let operations = [
        encoding::vadd_f32(0, 1, 2),
        encoding::vsub_f32(0, 1, 2),
        encoding::vmul_f32(0, 1, 2),
        encoding::vdiv_f32(0, 1, 2),
    ];
    let pairs = [
        (1.5_f32.to_bits(), 2.25_f32.to_bits()),
        (0x0000_0001, 1.0_f32.to_bits()),
        (f32::MAX.to_bits(), 2.0_f32.to_bits()),
        (f32::INFINITY.to_bits(), f32::NEG_INFINITY.to_bits()),
        (0x7f80_0001, 1.0_f32.to_bits()),
    ];
    for words in operations {
        for (left, right) in pairs {
            let (mut gate, mut oracle) = paired(&words, true);
            gate.write_s_register(1, left).unwrap();
            gate.write_s_register(2, right).unwrap();
            oracle.write_s_register(1, left).unwrap();
            oracle.write_s_register(2, right).unwrap();
            compare_step(&mut gate, &mut oracle);
        }
    }
}

#[test]
fn unary_compare_and_status_fp_paths_match() {
    for words in [
        encoding::vmov_f32(0, 1),
        encoding::vabs_f32(0, 1),
        encoding::vneg_f32(0, 1),
        encoding::vsqrt_f32(0, 1),
    ] {
        let (mut gate, mut oracle) = paired(&words, true);
        gate.write_s_register(1, 2.0_f32.to_bits()).unwrap();
        oracle.write_s_register(1, 2.0_f32.to_bits()).unwrap();
        compare_step(&mut gate, &mut oracle);
    }

    let (mut gate, mut oracle) = paired(&encoding::vcmp_f32(1, 2), true);
    gate.write_s_register(1, (-1.0_f32).to_bits()).unwrap();
    gate.write_s_register(2, 1.0_f32.to_bits()).unwrap();
    oracle.write_s_register(1, (-1.0_f32).to_bits()).unwrap();
    oracle.write_s_register(2, 1.0_f32.to_bits()).unwrap();
    compare_step(&mut gate, &mut oracle);

    let (mut gate, mut oracle) = paired(&encoding::vmsr(0), true);
    gate.write_register(0, 0x6000_0013).unwrap();
    oracle.write_register(0, 0x6000_0013).unwrap();
    compare_step(&mut gate, &mut oracle);
    let state = gate.get_state();
    let (mut reader, mut oracle_reader) = paired(&encoding::vmrs(1), true);
    let mut reader_state = reader.get_state();
    reader_state.fpscr = state.fpscr;
    reader.restore(reader_state).unwrap();
    let mut oracle_state = oracle_reader.get_state();
    oracle_state.fpscr = state.fpscr;
    oracle_reader.restore(oracle_state).unwrap();
    compare_step(&mut reader, &mut oracle_reader);
}

#[test]
fn typed_faults_are_atomic_and_configuration_is_restorable() {
    let words = encoding::vadd_f32(0, 1, 2);
    let (mut gate, _) = paired(&words, false);
    let before = gate.get_state();
    assert!(gate.step().is_err());
    assert_eq!(gate.get_state(), before);

    let mut invalid = before.clone();
    invalid.ge = 0x10;
    assert!(gate.restore(invalid).is_err());
    assert_eq!(gate.get_state(), before);
    let mut invalid = before.clone();
    invalid.fpscr = 0x100;
    assert!(gate.restore(invalid).is_err());
    assert_eq!(gate.get_state(), before);
}

#[test]
fn run_execute_and_step_limit_are_transactional() {
    let program = image(&[0x2001, 0xbe00]);
    let mut gate = CortexM4GateLevel::new();
    let result = gate.execute(&program, ORIGIN, 2).unwrap();
    assert!(result.stopped);
    assert_eq!(result.steps, 2);
    let before = gate.get_state();
    assert!(gate.execute(&program, ORIGIN, 1).is_err());
    assert_eq!(gate.get_state(), before);
    assert!(gate.run(65).is_err());
    assert_eq!(gate.get_state(), before);
}

#[test]
fn extension_decoder_accepts_families_and_refuses_reserved_neighbours() {
    for (words, class) in [
        (encoding::qadd(0, 1, 2), InstructionClass::Saturating),
        (encoding::sadd8(0, 1, 2), InstructionClass::ParallelBytes),
        (encoding::smuad(0, 1, 2), InstructionClass::DspMultiply),
        (
            encoding::vadd_f32(0, 1, 2),
            InstructionClass::FloatingPointBinary,
        ),
        (
            encoding::vabs_f32(0, 1),
            InstructionClass::FloatingPointUnary,
        ),
        (
            encoding::vcmp_f32(0, 1),
            InstructionClass::FloatingPointCompare,
        ),
        (encoding::vmrs(0), InstructionClass::FloatingPointStatus),
    ] {
        assert_eq!(classify(words[0], Some(words[1])).unwrap(), class);
    }
    assert!(classify(0xfa80, Some(0xf0f0)).is_err());
}

#[test]
fn every_inherited_primary_family_remains_explicit() {
    let bl = cortex_m3_gatelevel::encoding::bl(0);
    let cases = [
        (0x0000, None, CortexM3InstructionClass::ShiftAddSubtract),
        (0x2000, None, CortexM3InstructionClass::Immediate),
        (0x4000, None, CortexM3InstructionClass::DataProcessing),
        (0x4400, None, CortexM3InstructionClass::SpecialData),
        (0x4800, None, CortexM3InstructionClass::LiteralLoad),
        (0x5000, None, CortexM3InstructionClass::RegisterMemory),
        (0x6000, None, CortexM3InstructionClass::ImmediateMemory),
        (0x9000, None, CortexM3InstructionClass::StackMemory),
        (0xa000, None, CortexM3InstructionClass::AddressGeneration),
        (0xb000, None, CortexM3InstructionClass::Miscellaneous),
        (0xc000, None, CortexM3InstructionClass::MultipleMemory),
        (0xd000, None, CortexM3InstructionClass::ConditionalBranch),
        (0xe000, None, CortexM3InstructionClass::UnconditionalBranch),
        (bl[0], Some(bl[1]), CortexM3InstructionClass::BranchLink),
        (0xf3bf, Some(0x8f5f), CortexM3InstructionClass::Barrier),
        (
            0xf3ef,
            Some(0x8000),
            CortexM3InstructionClass::SpecialRegister,
        ),
        (0xf240, Some(0), CortexM3InstructionClass::Thumb2Data),
        (0xf8c0, Some(0), CortexM3InstructionClass::Thumb2Memory),
        (0xe850, Some(0x0f00), CortexM3InstructionClass::Exclusive),
        (0xe8d0, Some(0xf000), CortexM3InstructionClass::TableBranch),
    ];
    for (first, second, expected) in cases {
        assert_eq!(
            classify(first, second),
            Ok(InstructionClass::Core(expected))
        );
    }
}

#[test]
fn inherited_execution_preserves_every_extension_q_bit() {
    let (mut gate, mut oracle) = paired(&[0x2007, 0x3001, 0xbe00], true);
    let mut gate_state = gate.get_state();
    gate_state.ge = 0b1010;
    gate_state.s_registers[31] = 0x7fc0_0000;
    gate_state.fpscr = 0x2000_0011;
    gate.restore(gate_state).unwrap();
    let mut oracle_state = oracle.get_state();
    oracle_state.ge = 0b1010;
    oracle_state.s_registers[31] = 0x7fc0_0000;
    oracle_state.fpscr = 0x2000_0011;
    oracle.restore(oracle_state).unwrap();
    for _ in 0..3 {
        compare_step(&mut gate, &mut oracle);
    }
}

#[test]
fn checked_lifecycle_faults_preserve_the_complete_machine() {
    let mut gate = CortexM4GateLevel::new();
    gate.set_fpu_enabled(true);
    gate.write_s_register(31, 0xdead_beef).unwrap();
    let pristine = gate.get_state();
    for result in [gate.load(&[], ORIGIN), gate.load(&[0, 0], ORIGIN + 1)] {
        assert!(result.is_err());
        assert_eq!(gate.get_state(), pristine);
    }
    assert!(gate.load(&[0; 4], MEMORY_SIZE as u32 - 2).is_err());
    assert_eq!(gate.get_state(), pristine);
    assert!(gate.write_s_register(32, 0).is_err());
    assert!(gate.write_memory(MEMORY_SIZE as u32, 4, 0).is_err());
    assert_eq!(gate.get_state(), pristine);

    let (mut fpscr, _) = paired(&encoding::vmsr(0), true);
    fpscr.write_register(0, 0x0000_0100).unwrap();
    let before = fpscr.get_state();
    assert!(fpscr.step().is_err());
    assert_eq!(fpscr.get_state(), before);

    let (mut truncated, _) = paired(&[encoding::vadd_f32(0, 1, 2)[0]], true);
    let before = truncated.get_state();
    assert!(truncated.step().is_err());
    assert_eq!(truncated.get_state(), before);

    let (mut halted, _) = paired(&[0xbe00], false);
    halted.step().unwrap();
    let before = halted.get_state();
    assert!(halted.step().is_err());
    assert_eq!(halted.get_state(), before);
}

#[test]
fn deterministic_boundary_sweep_matches_the_functional_oracle() {
    let mut seed = 0x6d34_5e71_u32;
    let operations = [
        encoding::qadd(0, 1, 2),
        encoding::qsub(0, 1, 2),
        encoding::sadd8(0, 1, 2),
        encoding::uadd8(0, 1, 2),
        encoding::smuad(0, 1, 2),
        encoding::smlad(0, 1, 2, 3),
        encoding::vadd_f32(0, 1, 2),
        encoding::vsub_f32(0, 1, 2),
        encoding::vmul_f32(0, 1, 2),
        encoding::vdiv_f32(0, 1, 2),
    ];
    for words in operations {
        for iteration in 0..32 {
            let (mut gate, mut oracle) = paired(&words, true);
            for register in 0..4 {
                seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                gate.write_register(register, seed).unwrap();
                oracle.write_register(register, seed).unwrap();
                gate.write_s_register(register, seed).unwrap();
                oracle.write_s_register(register, seed).unwrap();
            }
            let actual = gate.step().unwrap();
            let expected = oracle.step().unwrap();
            assert_eq!(
                actual.state_after.core.registers[0], expected.state_after.core.registers[0],
                "integer result for {words:?} iteration {iteration}"
            );
            assert_eq!(
                actual.state_after.s_registers[0], expected.state_after.s_registers[0],
                "floating result for {words:?} iteration {iteration}"
            );
            assert_eq!(
                state_to_oracle(&actual.state_after),
                expected.state_after,
                "complete state for {words:?} iteration {iteration}"
            );
        }
    }
}
