use cortex_m3_gatelevel::{
    encoding, gates, CortexM3GateLevel, CortexM3State as GateState, FLIP_FLOP_COUNT, MEMORY_SIZE,
};
use cortex_m3_simulator::{CortexM3Simulator, CortexM3State as FunctionalState};

const ORIGIN: u32 = 0x100;

fn assert_state_eq(gate: &GateState, functional: &FunctionalState, label: &str) {
    assert_eq!(gate.registers, functional.registers, "{label}: registers");
    assert_eq!(gate.msp, functional.msp, "{label}: msp");
    assert_eq!(gate.psp, functional.psp, "{label}: psp");
    assert_eq!(gate.lr, functional.lr, "{label}: lr");
    assert_eq!(gate.pc, functional.pc, "{label}: pc");
    assert_eq!(gate.apsr, functional.apsr, "{label}: apsr");
    assert_eq!(gate.ipsr, functional.ipsr, "{label}: ipsr");
    assert_eq!(gate.itstate, functional.itstate, "{label}: itstate");
    assert_eq!(gate.primask, functional.primask, "{label}: primask");
    assert_eq!(gate.basepri, functional.basepri, "{label}: basepri");
    assert_eq!(gate.faultmask, functional.faultmask, "{label}: faultmask");
    assert_eq!(gate.control, functional.control, "{label}: control");
    assert_eq!(gate.handler_mode, functional.handler_mode, "{label}: mode");
    assert_eq!(gate.memory, functional.memory, "{label}: memory");
    assert_eq!(gate.halted, functional.halted, "{label}: halted");
    assert_eq!(gate.sleeping, functional.sleeping, "{label}: sleeping");
    assert_eq!(
        gate.waiting_for_event, functional.waiting_for_event,
        "{label}: WFE cause"
    );
    assert_eq!(
        gate.event_registered, functional.event_registered,
        "{label}: event"
    );
    assert_eq!(
        gate.exclusive_address, functional.exclusive_address,
        "{label}: exclusive"
    );
    assert_eq!(
        gate.loaded_origin, functional.loaded_origin,
        "{label}: origin"
    );
    assert_eq!(gate.loaded_len, functional.loaded_len, "{label}: length");
}

fn compare_transition(label: &str, words: &[u16], configure: impl Fn(&mut GateState)) {
    let program = encoding::program(words);
    let mut gate = CortexM3GateLevel::new();
    let mut functional = CortexM3Simulator::new();
    gate.load(&program, ORIGIN).unwrap();
    functional.load(&program, ORIGIN).unwrap();

    let mut gate_state = gate.get_state();
    configure(&mut gate_state);
    let functional_state = FunctionalState {
        registers: gate_state.registers,
        msp: gate_state.msp,
        psp: gate_state.psp,
        lr: gate_state.lr,
        pc: gate_state.pc,
        apsr: gate_state.apsr,
        ipsr: gate_state.ipsr,
        itstate: gate_state.itstate,
        primask: gate_state.primask,
        basepri: gate_state.basepri,
        faultmask: gate_state.faultmask,
        control: gate_state.control,
        handler_mode: gate_state.handler_mode,
        memory: gate_state.memory.clone(),
        halted: gate_state.halted,
        sleeping: gate_state.sleeping,
        waiting_for_event: gate_state.waiting_for_event,
        event_registered: gate_state.event_registered,
        exclusive_address: gate_state.exclusive_address,
        loaded_origin: gate_state.loaded_origin,
        loaded_len: gate_state.loaded_len,
    };
    gate.restore(gate_state).unwrap();
    functional.restore(functional_state).unwrap();

    match (gate.step(), functional.step()) {
        (Ok(gate_trace), Ok(functional_trace)) => {
            assert_eq!(gate_trace.pc_before, functional_trace.pc_before, "{label}");
            assert_eq!(gate_trace.pc_after, functional_trace.pc_after, "{label}");
            assert_eq!(gate_trace.raw, functional_trace.raw, "{label}");
            assert_eq!(gate_trace.width, functional_trace.width, "{label}");
            assert_eq!(gate_trace.mnemonic, functional_trace.mnemonic, "{label}");
            assert_state_eq(
                &gate_trace.state_before,
                &functional_trace.state_before,
                label,
            );
            assert_state_eq(
                &gate_trace.state_after,
                &functional_trace.state_after,
                label,
            );
            assert_state_eq(&gate.get_state(), &functional.get_state(), label);
        }
        (Err(gate_error), Err(functional_error)) => {
            assert_eq!(
                format!("{gate_error:?}"),
                format!("{functional_error:?}"),
                "{label}"
            );
            assert_state_eq(&gate.get_state(), &functional.get_state(), label);
        }
        (gate_result, functional_result) => {
            panic!("{label} diverged: gate={gate_result:?}, functional={functional_result:?}")
        }
    }
}

fn compare_sequence(label: &str, words: &[u16], steps: usize, configure: impl Fn(&mut GateState)) {
    let program = encoding::program(words);
    let mut gate = CortexM3GateLevel::new();
    let mut functional = CortexM3Simulator::new();
    gate.load(&program, ORIGIN).unwrap();
    functional.load(&program, ORIGIN).unwrap();

    let mut gate_state = gate.get_state();
    configure(&mut gate_state);
    let functional_state = FunctionalState {
        registers: gate_state.registers,
        msp: gate_state.msp,
        psp: gate_state.psp,
        lr: gate_state.lr,
        pc: gate_state.pc,
        apsr: gate_state.apsr,
        ipsr: gate_state.ipsr,
        itstate: gate_state.itstate,
        primask: gate_state.primask,
        basepri: gate_state.basepri,
        faultmask: gate_state.faultmask,
        control: gate_state.control,
        handler_mode: gate_state.handler_mode,
        memory: gate_state.memory.clone(),
        halted: gate_state.halted,
        sleeping: gate_state.sleeping,
        waiting_for_event: gate_state.waiting_for_event,
        event_registered: gate_state.event_registered,
        exclusive_address: gate_state.exclusive_address,
        loaded_origin: gate_state.loaded_origin,
        loaded_len: gate_state.loaded_len,
    };
    gate.restore(gate_state).unwrap();
    functional.restore(functional_state).unwrap();

    for step in 0..steps {
        let step_label = format!("{label}-step-{step}");
        let gate_trace = gate.step().unwrap();
        let functional_trace = functional.step().unwrap();
        assert_eq!(
            gate_trace.pc_before, functional_trace.pc_before,
            "{step_label}"
        );
        assert_eq!(
            gate_trace.pc_after, functional_trace.pc_after,
            "{step_label}"
        );
        assert_eq!(gate_trace.raw, functional_trace.raw, "{step_label}");
        assert_eq!(gate_trace.width, functional_trace.width, "{step_label}");
        assert_eq!(
            gate_trace.mnemonic, functional_trace.mnemonic,
            "{step_label}"
        );
        assert_state_eq(
            &gate_trace.state_before,
            &functional_trace.state_before,
            &step_label,
        );
        assert_state_eq(
            &gate_trace.state_after,
            &functional_trace.state_after,
            &step_label,
        );
        assert_state_eq(&gate.get_state(), &functional.get_state(), &step_label);
    }
}

fn seeded(state: &mut GateState) {
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

#[test]
fn topology_and_combinational_networks_are_exact() {
    let cpu = CortexM3GateLevel::new();
    assert_eq!(FLIP_FLOP_COUNT, 524_952);
    assert_eq!(cpu.flip_flop_count(), 524_952);
    assert!(cpu.gate_count() > FLIP_FLOP_COUNT * 6);
    assert_eq!(cpu.get_state().memory.len(), MEMORY_SIZE);

    for left in [0, 1, 0x7fff_ffff, 0x8000_0000, u32::MAX] {
        for right in [0, 1, 31, 0x5555_aaaa, u32::MAX] {
            assert_eq!(gates::add_32(left, right, 0).0, left.wrapping_add(right));
            assert_eq!(gates::sub_32(left, right, 0).0, left.wrapping_sub(right));
            assert_eq!(gates::bit_and_32(left, right), left & right);
            assert_eq!(gates::bit_or_32(left, right), left | right);
            assert_eq!(gates::bit_xor_32(left, right), left ^ right);
            assert_eq!(gates::equal_32(left, right), u8::from(left == right));
            assert_eq!(gates::shift_left_32(left, right & 31), left << (right & 31));
            assert_eq!(
                gates::shift_right_32(left, right & 31),
                left >> (right & 31)
            );
            assert_eq!(
                gates::shift_arithmetic_right_32(left, right & 31),
                ((left as i32) >> (right & 31)) as u32
            );
            assert_eq!(
                gates::rotate_right_32(left, right & 31),
                left.rotate_right(right & 31)
            );
            assert_eq!(gates::multiply_32(left, right), left.wrapping_mul(right));
            if right != 0 {
                assert_eq!(gates::divide_unsigned_32(left, right), left / right);
                assert_eq!(
                    gates::divide_signed_32(left, right),
                    (left as i32).wrapping_div(right as i32) as u32
                );
            }
        }
    }
    assert_eq!(gates::sign_extend_8(0x80), 0xffff_ff80);
    assert_eq!(gates::sign_extend_16(0x8000), 0xffff_8000);
    assert_eq!(gates::bits_to_u8(&[1; 64]), u8::MAX);
    assert_eq!(gates::bits_to_u16(&[1; 64]), u16::MAX);
    assert_eq!(gates::bits_to_u32(&[1; 64]), u32::MAX);
    assert_eq!(gates::assemble_le_u16([0x34, 0x12]), 0x1234);
    assert_eq!(
        gates::assemble_le_u32([0x78, 0x56, 0x34, 0x12]),
        0x1234_5678
    );
    assert_eq!(gates::split_le_u16(0x1234), [0x34, 0x12]);
    assert_eq!(gates::split_le_u32(0x1234_5678), [0x78, 0x56, 0x34, 0x12]);
    assert_eq!(gates::reverse_bits_32(1), 0x8000_0000);
    assert_eq!(gates::leading_zeros_32(0x0010_0000), 11);
    assert_eq!(gates::reverse_bytes(0x1122_3344), 0x4433_2211);
    assert_eq!(gates::reverse_halfwords(0x1122_3344), 0x2211_4433);
}

#[test]
fn every_sixteen_bit_subfamily_matches_the_functional_oracle() {
    let mut cases: Vec<(String, u16)> = Vec::new();
    for raw in [0x0008, 0x0808, 0x1008, 0x1808, 0x1a08, 0x1c08, 0x1e08] {
        cases.push((format!("shift-add-sub-{raw:04x}"), raw));
    }
    for raw in [0x2001, 0x2801, 0x3001, 0x3801] {
        cases.push((format!("immediate-{raw:04x}"), raw));
    }
    for operation in 0_u16..16 {
        cases.push((
            format!("data-processing-{operation}"),
            0x4000 | (operation << 6) | 8,
        ));
    }
    for raw in [0x4408, 0x4508, 0x4608, 0x4708, 0x4780, 0x4487, 0x4687] {
        cases.push((format!("special-{raw:04x}"), raw));
    }
    cases.push(("literal-load".into(), 0x4800));
    for operation in 0_u16..8 {
        cases.push((
            format!("register-memory-{operation}"),
            0x5000 | (operation << 9) | (4 << 3),
        ));
    }
    for raw in [0x6020, 0x6820, 0x7020, 0x7820, 0x8020, 0x8820] {
        cases.push((format!("immediate-memory-{raw:04x}"), raw));
    }
    for raw in [0x9400, 0x9c00, 0xa000, 0xa800] {
        cases.push((format!("stack-address-{raw:04x}"), raw));
    }
    for raw in [
        0xb001, 0xb081, 0xb240, 0xb248, 0xb280, 0xb2c0, 0xb401, 0xbc01, 0xba00, 0xba40, 0xbac0,
        0xb662, 0xb672, 0xbe00, 0xbe01, 0xbf00, 0xbf10, 0xbf20, 0xbf30, 0xbf40,
    ] {
        cases.push((format!("misc-{raw:04x}"), raw));
    }
    for raw in [0xc011, 0xc811] {
        cases.push((format!("multiple-{raw:04x}"), raw));
    }
    for condition in 0_u16..16 {
        cases.push((
            format!("conditional-{condition}"),
            0xd000 | (condition << 8),
        ));
    }
    cases.push(("unconditional".into(), 0xe000));
    for raw in [0xb100, 0xb600, 0xde00] {
        cases.push((format!("reserved-{raw:04x}"), raw));
    }

    for (label, raw) in cases {
        compare_transition(&label, &[raw], seeded);
    }
}

#[test]
fn every_thirty_two_bit_subfamily_matches_the_functional_oracle() {
    let bl = encoding::bl(0);
    let mut cases = vec![
        ("bl", bl),
        ("dsb", [0xf3bf, 0x8f4f]),
        ("dmb", [0xf3bf, 0x8f5f]),
        ("isb", [0xf3bf, 0x8f6f]),
        ("clrex", encoding::clrex()),
        ("movw", encoding::movw(0, 0x1234)),
        ("movt", encoding::movt(0, 0x5678)),
        ("sdiv", encoding::sdiv(0, 3, 2)),
        ("udiv", encoding::udiv(0, 3, 2)),
        ("ldrex", encoding::ldrex(0, 4, 0)),
        ("strex", encoding::strex(1, 0, 4, 0)),
        ("tbb", encoding::tbb(4, 2)),
        ("tbh", encoding::tbh(4, 2)),
        ("rbit", [0xfa93, 0xf0a3]),
        ("clz", [0xfab3, 0xf083]),
        ("strb-wide", [0xf884, 0x0001]),
        ("ldrb-wide", [0xf894, 0x0001]),
        ("strh-wide", [0xf8a4, 0x0002]),
        ("ldrh-wide", [0xf8b4, 0x0002]),
        ("str-wide", [0xf8c4, 0x0004]),
        ("ldr-wide", [0xf8d4, 0x0004]),
        ("ldrsb-wide", [0xf994, 0x0000]),
        ("ldrsh-wide", [0xf9b4, 0x0000]),
    ];
    for sysm in [0_u8, 1, 2, 3, 5, 6, 7, 8, 9, 16, 17, 18, 19, 20, 31] {
        cases.push(("mrs", encoding::mrs(0, sysm)));
        cases.push(("msr", encoding::msr(0, sysm)));
    }
    for sysm in 0_u8..=3 {
        cases.push(("apsr-msr-mask-zero", [0xf380, 0x8000 | u16::from(sysm)]));
        cases.push((
            "apsr-msr-mask-ge",
            encoding::msr_masked(0, sysm, encoding::ApsrMask::Ge),
        ));
        cases.push((
            "apsr-msr-mask-nzcvq",
            encoding::msr_masked(0, sysm, encoding::ApsrMask::Nzcvq),
        ));
        cases.push((
            "apsr-msr-mask-nzcvqge",
            encoding::msr_masked(0, sysm, encoding::ApsrMask::NzcvqGe),
        ));
    }
    for mask in [encoding::ApsrMask::Ge, encoding::ApsrMask::NzcvqGe] {
        cases.push(("reserved-non-apsr-mask", encoding::msr_masked(0, 20, mask)));
    }
    cases.push(("unknown", [0xf000, 0]));
    for (label, words) in cases {
        compare_transition(&format!("{label}-{words:04x?}"), &words, seeded);
    }
}

#[test]
fn branch_outcomes_privilege_and_process_stack_match() {
    for apsr in [0, 0x4000_0000] {
        compare_transition("branch-outcome", &[0xd000], |state| {
            seeded(state);
            state.apsr = apsr;
        });
    }
    for (control, handler_mode, ipsr) in
        [(1, false, 0), (2, false, 0), (3, false, 0), (1, true, 11)]
    {
        compare_transition("privilege-stack", &[0xb401], |state| {
            seeded(state);
            state.control = control;
            state.handler_mode = handler_mode;
            state.ipsr = ipsr;
        });
    }
}

#[test]
fn stateful_thumb2_lifecycles_match_the_functional_oracle() {
    let mut exclusive = Vec::new();
    exclusive.extend(encoding::ldrex(0, 4, 0));
    exclusive.extend(encoding::strex(1, 0, 4, 0));
    compare_sequence("exclusive-success", &exclusive, 2, seeded);

    let mut wide_move = Vec::new();
    wide_move.extend(encoding::movw(0, 0x1234));
    wide_move.extend(encoding::movt(0, 0xabcd));
    compare_sequence("wide-move-compose", &wide_move, 2, seeded);

    let it_program = [encoding::it(0, 8), encoding::movs(0, 9)];
    for apsr in [0, 0x4000_0000] {
        compare_sequence("it-execute-or-skip", &it_program, 2, |state| {
            seeded(state);
            state.apsr = apsr;
        });
    }

    compare_transition("udiv-by-zero", &encoding::udiv(0, 3, 2), |state| {
        seeded(state);
        state.registers[2] = 0;
    });
    compare_transition("sdiv-min-overflow", &encoding::sdiv(0, 3, 2), |state| {
        seeded(state);
        state.registers[3] = i32::MIN as u32;
        state.registers[2] = u32::MAX;
    });
}

#[test]
fn event_and_interrupt_wakeup_state_matches_the_functional_oracle() {
    let program = encoding::program(&[0xbf20]);
    let mut gate = CortexM3GateLevel::new();
    let mut functional = CortexM3Simulator::new();
    gate.load(&program, ORIGIN).unwrap();
    functional.load(&program, ORIGIN).unwrap();

    let gate_trace = gate.step().unwrap();
    let functional_trace = functional.step().unwrap();
    assert_eq!(gate_trace.mnemonic, functional_trace.mnemonic);
    assert_state_eq(&gate.get_state(), &functional.get_state(), "wfe-sleep");

    gate.signal_event();
    functional.signal_event();
    assert_state_eq(&gate.get_state(), &functional.get_state(), "event-wakeup");

    gate.signal_interrupt();
    functional.signal_interrupt();
    assert_state_eq(
        &gate.get_state(),
        &functional.get_state(),
        "interrupt-wakeup",
    );
}

#[test]
fn production_manifest_keeps_the_oracle_test_only() {
    let manifest =
        std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml")).unwrap();
    let dependency_section = manifest.split("[dev-dependencies]").next().unwrap();
    assert!(!dependency_section.contains("cortex-m3-simulator"));
}
