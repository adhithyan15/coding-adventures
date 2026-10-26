use cdp1802_gatelevel::{
    Cdp1802Error, Cdp1802GateSimulator, Cdp1802State, ExecutionError, StepTrace, MEMORY_SIZE,
};
use cdp1802_simulator::Cdp1802Simulator as FunctionalSimulator;

fn assert_trace_eq(gate: &StepTrace, functional: &cdp1802_simulator::StepTrace, label: &str) {
    assert_eq!(gate.pc_before, functional.pc_before, "{label}: pc_before");
    assert_eq!(gate.pc_after, functional.pc_after, "{label}: pc_after");
    assert_eq!(gate.p_before, functional.p_before, "{label}: p_before");
    assert_eq!(gate.p_after, functional.p_after, "{label}: p_after");
    assert_eq!(gate.x_before, functional.x_before, "{label}: x_before");
    assert_eq!(gate.x_after, functional.x_after, "{label}: x_after");
    assert_eq!(gate.raw, functional.raw, "{label}: raw");
    assert_eq!(gate.width, functional.width, "{label}: width");
    assert_eq!(gate.mnemonic, functional.mnemonic, "{label}: mnemonic");
    assert_eq!(gate.d_before, functional.d_before, "{label}: d_before");
    assert_eq!(gate.d_after, functional.d_after, "{label}: d_after");
    assert_eq!(gate.df_before, functional.df_before, "{label}: df_before");
    assert_eq!(gate.df_after, functional.df_after, "{label}: df_after");
    assert_eq!(gate.q_before, functional.q_before, "{label}: q_before");
    assert_eq!(gate.q_after, functional.q_after, "{label}: q_after");
    assert_eq!(gate.ie_before, functional.ie_before, "{label}: ie_before");
    assert_eq!(gate.ie_after, functional.ie_after, "{label}: ie_after");
    assert_eq!(
        gate.memory_access,
        convert_access(functional.memory_access),
        "{label}: memory access"
    );
    assert_eq!(
        gate.port_transaction,
        convert_port(functional.port_transaction),
        "{label}: port transaction"
    );
    assert_eq!(
        gate.branch_taken, functional.branch_taken,
        "{label}: branch"
    );
    assert_eq!(gate.cycles, functional.cycles, "{label}: cycles");
    assert_eq!(
        gate.idle_before, functional.idle_before,
        "{label}: idle_before"
    );
    assert_eq!(
        gate.idle_after, functional.idle_after,
        "{label}: idle_after"
    );
    assert_eq!(
        gate.state_before,
        convert_state(functional.state_before.clone()),
        "{label}: state_before"
    );
    assert_eq!(
        gate.state_after,
        convert_state(functional.state_after.clone()),
        "{label}: state_after"
    );
}

fn restored_pair(state: Cdp1802State) -> (Cdp1802GateSimulator, FunctionalSimulator) {
    let mut gate = Cdp1802GateSimulator::new();
    let mut functional = FunctionalSimulator::new();
    gate.restore(state.clone()).unwrap();
    functional.restore(convert_to_functional(state)).unwrap();
    (gate, functional)
}

fn compare_one(state: Cdp1802State, label: &str) {
    let (mut gate, mut functional) = restored_pair(state);
    let gate_trace = gate.step().unwrap();
    let functional_trace = functional.step().unwrap();
    assert_trace_eq(&gate_trace, &functional_trace, label);
    assert_eq!(
        gate.get_state(),
        convert_state(functional.get_state()),
        "{label}: committed state"
    );
}

fn assert_same_error<G: core::fmt::Debug, F: core::fmt::Debug>(
    gate: Result<G, Cdp1802Error>,
    functional: Result<F, cdp1802_simulator::Cdp1802Error>,
    label: &str,
) {
    match (gate, functional) {
        (Err(gate), Err(functional)) => {
            assert_eq!(format!("{gate:?}"), format!("{functional:?}"), "{label}");
            assert_eq!(gate.to_string(), functional.to_string(), "{label}");
        }
        (Ok(_), Ok(_)) => panic!("{label}: both machines unexpectedly succeeded"),
        (Ok(_), Err(_)) => panic!("{label}: gate unexpectedly succeeded"),
        (Err(_), Ok(_)) => panic!("{label}: functional unexpectedly succeeded"),
    }
}

fn assert_same_execution_error(
    gate: Result<cdp1802_gatelevel::ExecutionResult, ExecutionError>,
    functional: Result<cdp1802_simulator::ExecutionResult, cdp1802_simulator::ExecutionError>,
    label: &str,
) {
    match (gate, functional) {
        (Err(gate), Err(functional)) => {
            assert_eq!(
                format!("{:?}", gate.source),
                format!("{:?}", functional.source),
                "{label}"
            );
            assert_eq!(
                gate.source.to_string(),
                functional.source.to_string(),
                "{label}"
            );
        }
        (Ok(_), Ok(_)) => panic!("{label}: both machines unexpectedly succeeded"),
        (Ok(_), Err(_)) => panic!("{label}: gate unexpectedly succeeded"),
        (Err(_), Ok(_)) => panic!("{label}: functional unexpectedly succeeded"),
    }
}

fn fixture(opcode: u8) -> Cdp1802State {
    let registers = std::array::from_fn(|index| match index {
        0 => 0,
        1 => 0x0100,
        _ => 0x0100 + index as u16 * 3,
    });
    let mut memory = vec![0; MEMORY_SIZE];
    memory[0] = opcode;
    memory[1] = 2;
    memory[2] = 0;
    memory[0x0100] = 0xa5;
    Cdp1802State {
        origin: Some(0),
        length: 3,
        registers,
        p: 0,
        x: 1,
        d: 0x5a,
        df: true,
        ie: true,
        q: true,
        ef: [false, true, false, true],
        input_ports: [0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77],
        memory,
        ..Cdp1802State::default()
    }
}

#[test]
fn every_accepted_first_byte_matches_the_functional_oracle() {
    let mut witnessed = 0;
    for opcode in u8::MIN..=u8::MAX {
        if opcode == 0x68 {
            continue;
        }
        let state = fixture(opcode);
        let mut gate = Cdp1802GateSimulator::new();
        let mut functional = FunctionalSimulator::new();
        gate.restore(state.clone()).unwrap();
        functional.restore(convert_to_functional(state)).unwrap();

        let gate_trace = gate.step().unwrap_or_else(|error| {
            panic!("gate opcode {opcode:#04x} unexpectedly failed: {error}")
        });
        let functional_trace = functional.step().unwrap_or_else(|error| {
            panic!("functional opcode {opcode:#04x} unexpectedly failed: {error}")
        });

        assert_trace_eq(
            &gate_trace,
            &functional_trace,
            &format!("opcode {opcode:#04x}"),
        );
        assert_eq!(gate.get_state(), convert_state(functional.get_state()));
        witnessed += 1;
    }
    assert_eq!(witnessed, 255);
}

#[test]
fn focused_alias_wrap_self_modifying_branch_skip_immediate_and_port_traces_match() {
    let mut alias = fixture(0x40); // LDA R(P): fetch changes the aliased source first.
    alias.memory[1] = 0x6a;
    compare_one(alias, "P=N alias");

    let mut pxn = fixture(0x79); // MARK with P=X=2 exercises pre-state aliases.
    pxn.p = 2;
    pxn.x = 2;
    pxn.registers[2] = 0;
    pxn.memory[0] = 0x79;
    compare_one(pxn, "P=X=N MARK alias");

    let mut wrap = fixture(0x11);
    wrap.origin = Some(0);
    wrap.length = MEMORY_SIZE as u32;
    wrap.p = 1;
    wrap.registers[1] = u16::MAX;
    wrap.memory[u16::MAX as usize] = 0x11;
    compare_one(wrap, "fetch and INC alias wrap");

    let mut self_modifying = fixture(0x50);
    self_modifying.d = 0xaa;
    compare_one(self_modifying, "self-modifying STR keeps raw");

    for (opcode, q, label) in [
        (0x31, true, "short branch taken"),
        (0x39, true, "short branch not taken"),
        (0xc8, false, "unconditional long skip"),
        (0xcd, true, "conditional long skip"),
    ] {
        let mut state = fixture(opcode);
        state.q = q;
        state.memory[1] = 0x12;
        state.memory[2] = 0x34;
        compare_one(state, label);
    }

    let mut immediate = fixture(0xf8);
    immediate.memory[1] = 0xa5;
    compare_one(immediate, "immediate operand");

    for port in 1u8..=7 {
        let mut output = fixture(0x60 | port);
        output.x = 1;
        output.registers[1] = 0x0100;
        output.memory[0x0100] = 0x80 | port;
        compare_one(output, &format!("OUT {port}"));

        let mut input = fixture(0x68 | port);
        input.x = 1;
        input.registers[1] = 0x0100;
        input.input_ports[usize::from(port - 1)] = 0x40 | port;
        compare_one(input, &format!("INP {port}"));
    }
}

#[test]
fn lifecycle_dma_interrupt_fault_run_and_execute_surfaces_match() {
    let mut gate = Cdp1802GateSimulator::new();
    let mut functional = FunctionalSimulator::new();
    assert_eq!(gate.get_state(), convert_state(functional.get_state()));
    gate.load(&[0xf8, 0x2a, 0], 0x0100).unwrap();
    functional.load(&[0xf8, 0x2a, 0], 0x0100).unwrap();
    gate.write_register(1, u16::MAX).unwrap();
    functional.write_register(1, u16::MAX).unwrap();
    gate.write_memory(0x2000, 0x5a);
    functional.write_memory(0x2000, 0x5a);
    gate.set_external_flag(3, true).unwrap();
    functional.set_external_flag(3, true).unwrap();
    gate.set_input_port(7, 0xa7).unwrap();
    functional.set_input_port(7, 0xa7).unwrap();
    assert_eq!(gate.get_state(), convert_state(functional.get_state()));

    let mut dma_state = gate.get_state();
    dma_state.registers[0] = u16::MAX;
    dma_state.idle = true;
    gate.restore(dma_state.clone()).unwrap();
    functional
        .restore(convert_to_functional(dma_state))
        .unwrap();
    assert_eq!(
        format!("{:?}", gate.dma_in(0xcc).unwrap()),
        format!("{:?}", functional.dma_in(0xcc).unwrap())
    );
    assert_eq!(gate.get_state(), convert_state(functional.get_state()));
    assert_eq!(
        format!("{:?}", gate.dma_out().unwrap()),
        format!("{:?}", functional.dma_out().unwrap())
    );
    assert_eq!(gate.get_state(), convert_state(functional.get_state()));

    let mut inhibited = gate.get_state();
    inhibited.first_instruction_pending = true;
    inhibited.ie = false;
    gate.restore(inhibited.clone()).unwrap();
    functional
        .restore(convert_to_functional(inhibited))
        .unwrap();
    assert_same_error(
        gate.interrupt(),
        functional.interrupt(),
        "interrupt inhibited",
    );
    let mut masked = gate.get_state();
    masked.first_instruction_pending = false;
    gate.restore(masked.clone()).unwrap();
    functional.restore(convert_to_functional(masked)).unwrap();
    assert_same_error(gate.interrupt(), functional.interrupt(), "interrupt masked");
    let mut eligible = gate.get_state();
    eligible.ie = true;
    eligible.first_instruction_pending = false;
    eligible.p = 5;
    eligible.x = 10;
    eligible.idle = true;
    gate.restore(eligible.clone()).unwrap();
    functional.restore(convert_to_functional(eligible)).unwrap();
    assert_eq!(
        format!("{:?}", gate.interrupt().unwrap()),
        format!("{:?}", functional.interrupt().unwrap())
    );
    assert_eq!(gate.get_state(), convert_state(functional.get_state()));

    let mut gate = Cdp1802GateSimulator::new();
    let mut functional = FunctionalSimulator::new();
    assert_same_error(gate.step(), functional.step(), "no program");
    gate.load(&[0xf8], 0).unwrap();
    functional.load(&[0xf8], 0).unwrap();
    assert_same_error(gate.step(), functional.step(), "truncated immediate");
    gate.load(&[0x68], 0).unwrap();
    functional.load(&[0x68], 0).unwrap();
    assert_same_error(gate.step(), functional.step(), "unknown 0x68");
    let mut outside = gate.get_state();
    outside.registers[0] = 1;
    gate.restore(outside.clone()).unwrap();
    functional.restore(convert_to_functional(outside)).unwrap();
    assert_same_error(gate.step(), functional.step(), "fetch outside image");
    assert_same_error(
        gate.set_external_flag(0, true),
        functional.set_external_flag(0, true),
        "invalid EF",
    );
    assert_same_error(
        gate.set_input_port(8, 0),
        functional.set_input_port(8, 0),
        "invalid port",
    );

    let mut gate = Cdp1802GateSimulator::new();
    let mut functional = FunctionalSimulator::new();
    gate.load(&[0xf8, 0x2a, 0], 0x100).unwrap();
    functional.load(&[0xf8, 0x2a, 0], 0x100).unwrap();
    let gate_run = gate.run(2).unwrap();
    let functional_run = functional.run(2).unwrap();
    assert_eq!(gate_run.stopped, functional_run.stopped);
    assert_eq!(gate_run.steps, functional_run.steps);
    for (index, (gate_trace, functional_trace)) in gate_run
        .traces
        .iter()
        .zip(&functional_run.traces)
        .enumerate()
    {
        assert_trace_eq(gate_trace, functional_trace, &format!("run trace {index}"));
    }
    assert_eq!(
        gate_run.final_state,
        convert_state(functional_run.final_state)
    );

    let gate_execute = gate.execute(&[0xf8, 0x66, 0], 0x200, 2).unwrap();
    let functional_execute = functional.execute(&[0xf8, 0x66, 0], 0x200, 2).unwrap();
    for (index, (gate_trace, functional_trace)) in gate_execute
        .traces
        .iter()
        .zip(&functional_execute.traces)
        .enumerate()
    {
        assert_trace_eq(
            gate_trace,
            functional_trace,
            &format!("execute trace {index}"),
        );
    }
    assert_eq!(
        gate_execute.final_state,
        convert_state(functional_execute.final_state)
    );
}

#[test]
fn full_fault_and_lifecycle_rollback_matrix_matches() {
    let mut gate = Cdp1802GateSimulator::new();
    let mut functional = FunctionalSimulator::new();
    gate.load(&[0xf8, 1, 0], 0x100).unwrap();
    functional.load(&[0xf8, 1, 0], 0x100).unwrap();
    gate.step().unwrap();
    functional.step().unwrap();
    gate.reset();
    functional.reset();
    assert_eq!(gate.get_state(), convert_state(functional.get_state()));

    gate.load(&[0xf8, 1, 0], 0x100).unwrap();
    functional.load(&[0xf8, 1, 0], 0x100).unwrap();
    for (mut gate_bad, mut functional_bad, label) in [
        (
            {
                let mut state = gate.get_state();
                state.p = 16;
                state
            },
            {
                let mut state = functional.get_state();
                state.p = 16;
                state
            },
            "invalid selector state",
        ),
        (
            {
                let mut state = gate.get_state();
                state.memory.pop();
                state
            },
            {
                let mut state = functional.get_state();
                state.memory.pop();
                state
            },
            "invalid memory state",
        ),
    ] {
        let gate_before = gate.get_state();
        let functional_before = functional.get_state();
        assert_same_error(
            gate.restore(gate_bad.clone()),
            functional.restore(functional_bad.clone()),
            label,
        );
        // Keep ownership explicitly consumed so this matrix cannot silently
        // degrade into testing only one implementation's fixture.
        gate_bad.memory.clear();
        functional_bad.memory.clear();
        assert_eq!(gate.get_state(), gate_before, "{label}: gate rollback");
        assert_eq!(
            functional.get_state(),
            functional_before,
            "{label}: oracle rollback"
        );
    }
    assert_same_error(
        gate.read_register(16),
        functional.read_register(16),
        "invalid register",
    );

    let gate_before = gate.get_state();
    let functional_before = functional.get_state();
    assert_same_error(gate.run(1), functional.run(1), "step limit");
    assert_eq!(gate.get_state(), gate_before);
    assert_eq!(functional.get_state(), functional_before);
    assert_same_error(
        gate.run(cdp1802_gatelevel::MAX_RETAINED_STEPS + 1),
        functional.run(cdp1802_simulator::MAX_RETAINED_STEPS + 1),
        "retention limit",
    );

    gate.load(&[0], 0).unwrap();
    functional.load(&[0], 0).unwrap();
    let gate_trace = gate.step().unwrap();
    let functional_trace = functional.step().unwrap();
    assert_trace_eq(&gate_trace, &functional_trace, "IDL completion");
    assert_same_error(gate.step(), functional.step(), "post-IDL fault");

    let gate_before = gate.get_state();
    let functional_before = functional.get_state();
    assert_same_execution_error(
        gate.execute(&[0x68], 0x200, 1),
        functional.execute(&[0x68], 0x200, 1),
        "execute unknown rollback",
    );
    assert_eq!(gate.get_state(), gate_before);
    assert_eq!(functional.get_state(), functional_before);
    assert_same_execution_error(
        gate.execute(&[], 0, 1),
        functional.execute(&[], 0, 1),
        "execute empty rollback",
    );
}

#[test]
fn allocation_failures_cover_preflight_and_after_one_committed_private_step() {
    // The gate wrapper has one projection checkpoint before the engine's ten
    // two-step run preflights. Checkpoint 11 therefore fires only after the
    // first private instruction commit; run must restore its call-entry Q.
    let mut gate = Cdp1802GateSimulator::new();
    gate.load(&[0xf8, 0x2a, 0], 0x100).unwrap();
    let before = gate.get_state();
    gate.fail_allocation_after(Some(11));
    assert_eq!(gate.run(2), Err(Cdp1802Error::AllocationFailed));
    assert_eq!(gate.get_state(), before);

    // Execute adds three prepared outer memories. Checkpoint 14 reaches the
    // same post-first-step seam after the replacement image was installed and
    // must restore the original machine rather than the loaded image.
    gate.fail_allocation_after(Some(14));
    assert_eq!(
        gate.execute(&[0xf8, 0x66, 0], 0x200, 2),
        Err(ExecutionError {
            source: Cdp1802Error::AllocationFailed
        })
    );
    assert_eq!(gate.get_state(), before);

    // Both packages also expose typed, atomic preflight failures. Their
    // internal checkpoint counts are intentionally implementation-private.
    let mut gate_witnessed = false;
    let mut functional_witnessed = false;
    for checkpoint in 0..20 {
        let mut candidate = Cdp1802GateSimulator::new();
        candidate.load(&[0], 0).unwrap();
        let entry = candidate.get_state();
        candidate.fail_allocation_after(Some(checkpoint));
        if candidate.run(1) == Err(Cdp1802Error::AllocationFailed) {
            assert_eq!(candidate.get_state(), entry);
            gate_witnessed = true;
        }

        let mut candidate = FunctionalSimulator::new();
        candidate.load(&[0], 0).unwrap();
        let entry = candidate.get_state();
        candidate.fail_allocation_after(Some(checkpoint));
        if candidate.run(1) == Err(cdp1802_simulator::Cdp1802Error::AllocationFailed) {
            assert_eq!(candidate.get_state(), entry);
            functional_witnessed = true;
        }
    }
    assert!(gate_witnessed && functional_witnessed);

    let mut gate_execute_witnessed = false;
    let mut functional_execute_witnessed = false;
    for checkpoint in 0..24 {
        let mut candidate = Cdp1802GateSimulator::new();
        candidate.load(&[0], 0).unwrap();
        let entry = candidate.get_state();
        candidate.fail_allocation_after(Some(checkpoint));
        if candidate.execute(&[0], 0x100, 1)
            == Err(ExecutionError {
                source: Cdp1802Error::AllocationFailed,
            })
        {
            assert_eq!(candidate.get_state(), entry);
            gate_execute_witnessed = true;
        }

        let mut candidate = FunctionalSimulator::new();
        candidate.load(&[0], 0).unwrap();
        let entry = candidate.get_state();
        candidate.fail_allocation_after(Some(checkpoint));
        if candidate.execute(&[0], 0x100, 1)
            == Err(cdp1802_simulator::ExecutionError {
                source: cdp1802_simulator::Cdp1802Error::AllocationFailed,
            })
        {
            assert_eq!(candidate.get_state(), entry);
            functional_execute_witnessed = true;
        }
    }
    assert!(gate_execute_witnessed && functional_execute_witnessed);
}

fn convert_port(
    source: Option<cdp1802_simulator::PortTransaction>,
) -> Option<cdp1802_gatelevel::PortTransaction> {
    source.map(|transaction| match transaction {
        cdp1802_simulator::PortTransaction::Write { port, value } => {
            cdp1802_gatelevel::PortTransaction::Write { port, value }
        }
        cdp1802_simulator::PortTransaction::Read { port, value } => {
            cdp1802_gatelevel::PortTransaction::Read { port, value }
        }
    })
}

fn convert_state(source: cdp1802_simulator::Cdp1802State) -> Cdp1802State {
    Cdp1802State {
        registers: source.registers,
        p: source.p,
        x: source.x,
        d: source.d,
        df: source.df,
        ie: source.ie,
        q: source.q,
        t: source.t,
        ef: source.ef,
        input_ports: source.input_ports,
        memory: source.memory,
        origin: source.origin,
        length: source.length,
        first_instruction_pending: source.first_instruction_pending,
        idle: source.idle,
    }
}

fn convert_to_functional(source: Cdp1802State) -> cdp1802_simulator::Cdp1802State {
    cdp1802_simulator::Cdp1802State {
        registers: source.registers,
        p: source.p,
        x: source.x,
        d: source.d,
        df: source.df,
        ie: source.ie,
        q: source.q,
        t: source.t,
        ef: source.ef,
        input_ports: source.input_ports,
        memory: source.memory,
        origin: source.origin,
        length: source.length,
        first_instruction_pending: source.first_instruction_pending,
        idle: source.idle,
    }
}

fn convert_access(
    source: Option<cdp1802_simulator::MemoryAccess>,
) -> Option<cdp1802_gatelevel::MemoryAccess> {
    source.map(|access| cdp1802_gatelevel::MemoryAccess {
        kind: match access.kind {
            cdp1802_simulator::MemoryAccessKind::Read => cdp1802_gatelevel::MemoryAccessKind::Read,
            cdp1802_simulator::MemoryAccessKind::Write => {
                cdp1802_gatelevel::MemoryAccessKind::Write
            }
        },
        address: access.address,
        value: access.value,
    })
}
