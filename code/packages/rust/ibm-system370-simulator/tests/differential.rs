use ibm_system360_simulator::System360Simulator;
use ibm_system370_simulator::System370Simulator;

#[test]
fn every_non_extension_first_byte_delegates_exactly_to_system360() {
    for opcode in 0_u8..=u8::MAX {
        if matches!(opcode, 0x0e | 0x0f | 0xbd..=0xbf) {
            continue;
        }
        let program = [opcode, 0, 0, 0, 0, 0];
        let mut inherited = System360Simulator::new();
        let mut successor = System370Simulator::new();
        inherited.load(&program, 0).unwrap();
        successor.load(&program, 0).unwrap();
        let inherited_result = inherited.step();
        let successor_result = successor.step();
        match (inherited_result, successor_result) {
            (Ok(left), Ok(right)) => {
                assert_eq!(right.pc_before, left.pc_before, "opcode {opcode:#04x}");
                assert_eq!(right.pc_after, left.pc_after, "opcode {opcode:#04x}");
                assert_eq!(right.raw, left.raw, "opcode {opcode:#04x}");
                assert_eq!(right.width, left.width, "opcode {opcode:#04x}");
                assert_eq!(right.mnemonic, left.mnemonic, "opcode {opcode:#04x}");
                assert_eq!(right.description, left.description, "opcode {opcode:#04x}");
                let left = inherited.get_state();
                let right = successor.get_state();
                assert_eq!(right.gprs, left.gprs, "opcode {opcode:#04x}");
                assert_eq!(right.fprs, left.fprs, "opcode {opcode:#04x}");
                assert_eq!(right.pc, left.pc, "opcode {opcode:#04x}");
                assert_eq!(right.condition_code, left.condition_code, "opcode {opcode:#04x}");
                assert_eq!(right.memory, left.memory, "opcode {opcode:#04x}");
                assert_eq!(right.halted, left.halted, "opcode {opcode:#04x}");
                assert_eq!(right.loaded_origin, left.loaded_origin, "opcode {opcode:#04x}");
                assert_eq!(right.loaded_len, left.loaded_len, "opcode {opcode:#04x}");
            }
            (Err(left), Err(right)) => {
                assert_eq!(format!("{right:?}"), format!("{left:?}"), "opcode {opcode:#04x}");
                assert_eq!(successor.get_state().pc, inherited.get_state().pc);
            }
            (left, right) => panic!(
                "opcode {opcode:#04x} classification differs: inherited={left:?}, successor={right:?}"
            ),
        }
    }
}

#[test]
fn inherited_instruction_preserves_complete_nontrivial_state_and_trace() {
    let program = [0x58, 0x20, 0x01, 0x00]; // L R2,0x100
    let mut inherited = System360Simulator::new();
    let mut successor = System370Simulator::new();
    inherited.load(&program, 0).unwrap();
    successor.load(&program, 0).unwrap();
    inherited
        .write_memory(0x100, &0x89ab_cdef_u32.to_be_bytes())
        .unwrap();
    successor
        .write_memory(0x100, &0x89ab_cdef_u32.to_be_bytes())
        .unwrap();
    inherited.write_fpr(6, 0x1234_5678_9abc_def0).unwrap();
    successor.write_fpr(6, 0x1234_5678_9abc_def0).unwrap();
    let left = inherited.step().unwrap();
    let right = successor.step().unwrap();
    assert_eq!(right.mnemonic, left.mnemonic);
    assert_eq!(successor.read_gpr(2).unwrap(), 0x89ab_cdef);
    assert_eq!(successor.read_fpr(6).unwrap(), 0x1234_5678_9abc_def0);
}
