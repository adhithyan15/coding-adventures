use coding_adventures_intel8051_gatelevel::{Cpu8051, Intel8051State};
use intel8051_simulator::Intel8051Simulator;

fn seeded_state(opcode: u8) -> Intel8051State {
    let mut iram = [0u8; 256];
    for (index, byte) in iram.iter_mut().enumerate() {
        *byte = (index as u8).wrapping_mul(37).wrapping_add(opcode ^ 0x5A);
    }
    // Use register bank zero and valid base-8051 indirect addresses.
    iram[0xD0] &= !0x18;
    for register in iram.iter_mut().take(2) {
        *register &= 0x7F;
    }
    iram[0x81] = 0x40;
    iram[0xF0] |= 1; // exercise ordinary DIV rather than its special zero path

    let mut code = vec![0u8; 65_536];
    for (index, byte) in code.iter_mut().enumerate() {
        *byte = (index as u8).wrapping_mul(13).wrapping_add(0x31);
    }
    code[0x2000] = opcode;
    code[0x2001] = 0x34;
    code[0x2002] = 0xFE;

    let mut xdata = vec![0u8; 65_536];
    for (index, byte) in xdata.iter_mut().enumerate() {
        *byte = (index as u8).wrapping_mul(17).wrapping_add(0x63);
    }

    Intel8051State {
        pc: 0x2000,
        iram,
        xdata,
        code,
        halted: false,
        loaded_origin: 0,
        loaded_len: 65_536,
    }
}

#[test]
fn every_opcode_matches_the_functional_full_state_transition() {
    let mut gate = Cpu8051::new();
    let mut functional = Intel8051Simulator::new();
    for opcode in 0u8..=u8::MAX {
        let state = seeded_state(opcode);
        gate.restore(&state).unwrap();
        functional.restore(&state).unwrap();
        let gate_trace = gate
            .step_checked()
            .unwrap_or_else(|error| panic!("gate opcode {opcode:#04x}: {error}"));
        let functional_trace = functional
            .step_checked()
            .unwrap_or_else(|error| panic!("functional opcode {opcode:#04x}: {error}"));
        assert_eq!(gate_trace, functional_trace, "opcode {opcode:#04x}");
    }
}
