use super::*;

#[test]
fn topology_inventory_is_derived_from_authoritative_containers() {
    let machine = Cdp1802GateSimulator::new();
    assert_eq!(machine.flip_flop_count(), 524_667);
    assert_eq!(machine.storage_gate_count(), 3_148_002);
}

#[test]
fn reset_values_are_clocked_into_q() {
    let mut machine = Cdp1802GateSimulator::new();
    machine.write_register(7, 0xa55a).unwrap();
    machine.write_memory(0x1234, 0x5a);
    machine.reset();
    assert_eq!(machine.get_state(), Cdp1802State::default());
}
