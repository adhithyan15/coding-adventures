use cdp1802_gatelevel::{Cdp1802GateSimulator, Cdp1802State, MEMORY_SIZE};
use cdp1802_simulator::Cdp1802Simulator as FunctionalSimulator;

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

        assert_eq!(gate_trace.raw, functional_trace.raw, "opcode {opcode:#04x}");
        assert_eq!(
            gate_trace.width, functional_trace.width,
            "opcode {opcode:#04x}"
        );
        assert_eq!(
            gate_trace.mnemonic, functional_trace.mnemonic,
            "opcode {opcode:#04x}"
        );
        assert_eq!(
            gate_trace.cycles, functional_trace.cycles,
            "opcode {opcode:#04x}"
        );
        assert_eq!(
            gate_trace.memory_access,
            convert_access(functional_trace.memory_access)
        );
        assert_eq!(
            gate_trace.port_transaction,
            convert_port(functional_trace.port_transaction)
        );
        assert_eq!(gate_trace.branch_taken, functional_trace.branch_taken);
        assert_eq!(gate.get_state(), convert_state(functional.get_state()));
        witnessed += 1;
    }
    assert_eq!(witnessed, 255);
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
