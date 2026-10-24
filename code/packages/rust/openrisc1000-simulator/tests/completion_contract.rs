use openrisc1000_simulator::{
    primary_opcode_is_implemented, ExecutionResult, OpenRisc1000Error, OpenRisc1000Simulator,
    OpenRisc1000State, MEMORY_SIZE, SR_CY, SR_F, SR_FO, SR_OV, SR_SM,
};

fn words(words: &[u32]) -> Vec<u8> {
    words.iter().flat_map(|word| word.to_be_bytes()).collect()
}

const fn imm(op: u32, rd: u32, ra: u32, value: u16) -> u32 {
    (op << 26) | (rd << 21) | (ra << 16) | value as u32
}

const fn reg(rd: u32, ra: u32, rb: u32, function: u32) -> u32 {
    (0x38 << 26) | (rd << 21) | (ra << 16) | (rb << 11) | function
}

const fn compare(condition: u32, ra: u32, rb: u32) -> u32 {
    (0x39 << 26) | (condition << 21) | (ra << 16) | (rb << 11)
}

const fn compare_imm(condition: u32, ra: u32, value: u16) -> u32 {
    (0x2f << 26) | (condition << 21) | (ra << 16) | value as u32
}

const fn store(op: u32, ra: u32, rb: u32, offset: u16) -> u32 {
    (op << 26)
        | (((offset as u32) >> 11) << 21)
        | (ra << 16)
        | (rb << 11)
        | ((offset as u32) & 0x7ff)
}

const fn mtspr(ra: u32, rb: u32, address: u16) -> u32 {
    (0x30 << 26)
        | ((((address as u32) >> 11) & 0x1f) << 21)
        | (ra << 16)
        | (rb << 11)
        | ((address as u32) & 0x7ff)
}

const NOP: u32 = 0x1500_0000;
const HALT: u32 = 0x1500_0001;

fn loaded(program: &[u32]) -> OpenRisc1000Simulator {
    let mut simulator = OpenRisc1000Simulator::new();
    simulator.load(&words(program), 0x100).unwrap();
    simulator
}

#[test]
fn primary_opcode_table_matches_the_orbis32_boundary() {
    let implemented = [
        0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x08, 0x09, 0x11, 0x12, 0x13, 0x1b, 0x21, 0x22,
        0x23, 0x24, 0x25, 0x26, 0x27, 0x28, 0x29, 0x2a, 0x2b, 0x2c, 0x2d, 0x2e, 0x2f, 0x30, 0x31,
        0x33, 0x35, 0x36, 0x37, 0x38, 0x39,
    ];
    for opcode in 0_u8..64 {
        assert_eq!(
            primary_opcode_is_implemented(opcode),
            implemented.contains(&opcode),
            "classification mismatch for primary opcode {opcode:#04x}"
        );
    }
}

#[test]
fn default_and_load_expose_complete_big_endian_state() {
    let simulator = OpenRisc1000Simulator::new();
    let state = simulator.get_state();
    assert_eq!(state.gpr, [0; 32]);
    assert_eq!(state.memory.len(), MEMORY_SIZE);
    assert_eq!(state.sr, SR_FO | 1);
    assert_eq!(state.loaded_len, 0);

    let mut simulator = simulator;
    simulator.load(&words(&[NOP, HALT]), 0x120).unwrap();
    let state = simulator.get_state();
    assert_eq!(state.pc, 0x120);
    assert_eq!(state.ppc, 0x120);
    assert_eq!(&state.memory[0x120..0x128], &words(&[NOP, HALT]));
    assert_eq!(state.loaded_origin, 0x120);
    assert_eq!(state.loaded_len, 8);
}

#[test]
fn load_validation_is_atomic() {
    let mut simulator = loaded(&[HALT]);
    let before = simulator.get_state();
    assert_eq!(simulator.load(&[], 0), Err(OpenRisc1000Error::EmptyProgram));
    assert_eq!(simulator.get_state(), before);
    assert!(matches!(
        simulator.load(&[0; 3], 0),
        Err(OpenRisc1000Error::MisalignedProgram { .. })
    ));
    assert_eq!(simulator.get_state(), before);
    assert!(matches!(
        simulator.load(&words(&[NOP]), 0xffff),
        Err(OpenRisc1000Error::MisalignedProgram { .. })
    ));
    assert_eq!(simulator.get_state(), before);
}

#[test]
fn immediate_register_and_flag_operations_execute() {
    let mut simulator = loaded(&[
        imm(0x06, 1, 0, 0x1234),
        imm(0x2a, 1, 1, 0x5678),
        imm(0x27, 2, 1, 1),
        imm(0x28, 3, 2, 0xffff),
        imm(0x29, 4, 3, 0x00ff),
        imm(0x2b, 5, 4, 0xffff),
        reg(6, 5, 4, 0x005),
        HALT,
    ]);
    let result = simulator.run(8).unwrap();
    let state = result.final_state;
    assert_eq!(state.gpr[1], 0x1234_5678);
    assert_eq!(state.gpr[2], 0x1234_5679);
    assert_eq!(state.gpr[3], 0x1234_5678);
    assert_eq!(state.gpr[4], 0x78);
    assert_eq!(state.gpr[5], 0xffff_ff87);
    assert_eq!(state.gpr[6], 0xffff_ffff);
    assert_ne!(state.sr & SR_CY, 0);
    assert!(result.halted);
}

#[test]
fn arithmetic_reports_carry_and_signed_overflow() {
    let mut simulator = loaded(&[
        imm(0x2a, 1, 0, 0xffff),
        reg(2, 1, 1, 0x000),
        imm(0x06, 3, 0, 0x7fff),
        imm(0x2a, 3, 3, 0xffff),
        imm(0x27, 4, 3, 1),
        HALT,
    ]);
    simulator.run(2).unwrap_err();
    // A failed bounded run rolls back the whole execution.
    assert_eq!(simulator.read_register(1).unwrap(), 0);
    let result = simulator.run(6).unwrap();
    assert_eq!(result.final_state.gpr[2], 0x1fffe);
    assert_eq!(result.final_state.gpr[4], 0x8000_0000);
    assert_ne!(result.traces[4].state_after.sr & SR_OV, 0);
}

#[test]
fn multiply_immediate_and_division_refresh_their_status_flags() {
    let mut multiply = loaded(&[imm(0x2c, 2, 1, 2), imm(0x2c, 3, 4, 2), HALT]);
    multiply.write_register(1, i32::MAX as u32).unwrap();
    multiply.write_register(4, 2).unwrap();
    assert_ne!(multiply.step().unwrap().state_after.sr & SR_OV, 0);
    assert_eq!(multiply.step().unwrap().state_after.sr & SR_OV, 0);

    let mut divide = loaded(&[reg(3, 1, 2, 0x309), reg(4, 1, 2, 0x30a), HALT]);
    let mut state = divide.get_state();
    state.gpr[1] = 12;
    state.gpr[2] = 3;
    state.sr |= SR_OV | SR_CY;
    divide.restore(state).unwrap();
    let signed = divide.step().unwrap();
    assert_eq!(signed.state_after.sr & SR_OV, 0);
    assert_ne!(signed.state_after.sr & SR_CY, 0);
    let unsigned = divide.step().unwrap();
    assert_eq!(unsigned.state_after.sr & (SR_OV | SR_CY), 0);
}

#[test]
fn shifts_rotates_extensions_and_bit_search_execute() {
    let mut simulator = loaded(&[
        imm(0x2a, 1, 0, 0x8001),
        imm(0x2e, 2, 1, 1),
        imm(0x2e, 3, 1, 0x41),
        imm(0x2e, 4, 1, 0x81),
        imm(0x2e, 5, 1, 0xc1),
        reg(6, 1, 0, 0x00c),
        reg(7, 1, 0, 0x00f),
        reg(8, 1, 0, 0x04f),
        HALT,
    ]);
    let state = simulator.run(9).unwrap().final_state;
    assert_eq!(state.gpr[2], 0x1_0002);
    assert_eq!(state.gpr[3], 0x4000);
    assert_eq!(state.gpr[4], 0x4000);
    assert_eq!(state.gpr[5], 0x8000_4000);
    assert_eq!(state.gpr[6], 0xffff_8001);
    assert_eq!(state.gpr[7], 1);
    assert_eq!(state.gpr[8], 16);
}

#[test]
fn signed_and_unsigned_compares_control_branches() {
    let mut simulator = loaded(&[
        imm(0x2b, 1, 0, 0xffff),
        compare_imm(0x0c, 1, 0),
        (0x04 << 26) | 2,
        imm(0x2a, 2, 0, 1),
        compare(0x04, 2, 1),
        (0x03 << 26) | 2,
        imm(0x2a, 3, 0, 7),
        HALT,
    ]);
    let result = simulator.run(7).unwrap();
    assert_eq!(result.final_state.gpr[2], 0);
    assert_eq!(result.final_state.gpr[3], 7);
    assert_ne!(result.traces[3].state_after.sr & SR_F, 0);
}

#[test]
fn unsigned_immediate_compares_sign_extend_before_comparison() {
    for immediate in [0x8000, 0xffff] {
        let mut simulator = loaded(&[compare_imm(0x04, 1, immediate), HALT]);
        simulator
            .write_register(1, u32::from(immediate) - 1)
            .unwrap();
        simulator.step().unwrap();
        assert_ne!(
            simulator.get_state().sr & SR_F,
            0,
            "immediate {immediate:#06x}"
        );
    }
}

#[test]
fn jumps_link_without_a_delay_slot() {
    let mut simulator = loaded(&[
        (0x01 << 26) | 2,
        imm(0x2a, 1, 0, 1),
        imm(0x2a, 2, 0, 2),
        (0x12 << 26) | (3 << 11),
        HALT,
    ]);
    simulator.write_register(3, 0x110).unwrap();
    let result = simulator.run(4).unwrap();
    assert_eq!(result.final_state.gpr[1], 0);
    assert_eq!(result.final_state.gpr[2], 2);
    assert_eq!(result.final_state.gpr[9], 0x110);
    assert_eq!(result.final_state.pc, 0x114);
}

#[test]
fn jalr_rejects_link_register_as_its_target_atomically() {
    let word = (0x12 << 26) | (9 << 11);
    let mut simulator = loaded(&[word, HALT]);
    simulator.write_register(9, 0x104).unwrap();
    let before = simulator.get_state();
    assert_eq!(
        simulator.step(),
        Err(OpenRisc1000Error::UnknownInstruction { word, pc: 0x100 })
    );
    assert_eq!(simulator.get_state(), before);
}

#[test]
fn big_endian_loads_stores_and_sign_extension_execute() {
    let mut simulator = loaded(&[
        imm(0x2a, 1, 0, 0x0200),
        imm(0x06, 2, 0, 0x80ff),
        imm(0x2a, 2, 2, 0x7e01),
        store(0x35, 1, 2, 0),
        imm(0x21, 3, 1, 0),
        imm(0x23, 4, 1, 0),
        imm(0x24, 5, 1, 0),
        imm(0x25, 6, 1, 0),
        imm(0x26, 7, 1, 0),
        store(0x36, 1, 2, 4),
        store(0x37, 1, 2, 6),
        HALT,
    ]);
    let state = simulator.run(12).unwrap().final_state;
    assert_eq!(&state.memory[0x200..0x204], &[0x80, 0xff, 0x7e, 0x01]);
    assert_eq!(state.gpr[3], 0x80ff_7e01);
    assert_eq!(state.gpr[4], 0x80);
    assert_eq!(state.gpr[5], 0xffff_ff80);
    assert_eq!(state.gpr[6], 0x80ff);
    assert_eq!(state.gpr[7], 0xffff_80ff);
    assert_eq!(state.memory[0x204], 1);
    assert_eq!(&state.memory[0x206..0x208], &[0x7e, 0x01]);
}

#[test]
fn misaligned_and_out_of_range_memory_faults_are_atomic() {
    let mut simulator = loaded(&[imm(0x21, 2, 1, 0), HALT]);
    simulator.write_register(1, 3).unwrap();
    let before = simulator.get_state();
    assert!(matches!(
        simulator.step(),
        Err(OpenRisc1000Error::MisalignedData {
            address: 3,
            width: 4
        })
    ));
    assert_eq!(simulator.get_state(), before);

    simulator.write_register(1, 0xffff).unwrap();
    let before = simulator.get_state();
    assert!(matches!(
        simulator.step(),
        Err(OpenRisc1000Error::MisalignedData { .. })
            | Err(OpenRisc1000Error::DataOutOfRange { .. })
    ));
    assert_eq!(simulator.get_state(), before);
}

#[test]
fn multiply_divide_and_mac_families_execute() {
    let mut simulator = loaded(&[
        imm(0x27, 1, 0, 7),
        imm(0x27, 2, 0, 3),
        reg(3, 1, 2, 0x306),
        reg(4, 1, 2, 0x309),
        reg(5, 1, 2, 0x30a),
        (0x31 << 26) | (1 << 16) | (2 << 11) | 1,
        (0x06 << 26) | (6 << 21) | 0x1_0000,
        HALT,
    ]);
    let state = simulator.run(8).unwrap().final_state;
    assert_eq!(state.gpr[3], 21);
    assert_eq!(state.gpr[4], 2);
    assert_eq!(state.gpr[5], 2);
    assert_eq!(state.gpr[6], 21);
    assert_eq!(state.maclo, 0);
    assert_eq!(state.machi, 0);
}

#[test]
fn divide_faults_preserve_complete_state() {
    let mut simulator = loaded(&[reg(3, 1, 2, 0x309), HALT]);
    simulator.write_register(1, 7).unwrap();
    let before = simulator.get_state();
    assert_eq!(simulator.step(), Err(OpenRisc1000Error::DivideByZero));
    assert_eq!(simulator.get_state(), before);
}

#[test]
fn sprs_gprs_and_mac_registers_round_trip() {
    let mtspr_sr = (0x30 << 26) | (1 << 16) | (2 << 11) | 17;
    let mfspr_sr = imm(0x2d, 3, 1, 17);
    let mtspr_maclo = (0x30 << 26) | (5 << 21) | (1 << 16) | (2 << 11) | 1;
    let mut simulator = loaded(&[mtspr_sr, mfspr_sr, mtspr_maclo, HALT]);
    simulator.write_register(1, 0).unwrap();
    simulator
        .write_register(2, SR_FO | SR_SM | SR_F | SR_CY)
        .unwrap();
    let state = simulator.run(4).unwrap().final_state;
    assert_eq!(state.gpr[3] & (SR_FO | SR_F | SR_CY), SR_FO | SR_F | SR_CY);
    assert_eq!(state.sr & SR_FO, SR_FO);
    assert_eq!(state.maclo, SR_FO | SR_SM | SR_F | SR_CY);
}

#[test]
fn system_exception_and_rfe_save_and_restore_control_state() {
    let mut simulator = loaded(&[(0x08 << 26) | 7, HALT]);
    let mut enabled = simulator.get_state();
    enabled.sr |= (1 << 1) | (1 << 2) | (1 << 5) | (1 << 6);
    simulator.restore(enabled).unwrap();
    simulator.step().unwrap();
    let state = simulator.get_state();
    assert_eq!(state.epcr0, 0x104);
    assert_eq!(state.pc, 0x0c00);
    assert_eq!(state.esr0 & SR_FO, SR_FO);
    assert_eq!(state.sr & ((1 << 1) | (1 << 2) | (1 << 5) | (1 << 6)), 0);

    let mut restored = state;
    restored.memory[0x0c00..0x0c04].copy_from_slice(&(0x09_u32 << 26).to_be_bytes());
    restored.loaded_origin = 0;
    restored.loaded_len = 0x0c04;
    simulator.restore(restored).unwrap();
    simulator.step().unwrap();
    assert_eq!(simulator.get_state().pc, 0x104);
}

#[test]
fn trap_rfe_returns_to_the_trapping_instruction() {
    let trap = (0x08 << 26) | (1 << 24) | 9;
    let mut simulator = loaded(&[trap, HALT]);
    simulator.step().unwrap();
    let mut state = simulator.get_state();
    assert_eq!(state.epcr0, 0x100);
    assert_eq!(state.pc, 0x0e00);
    state.memory[0x0e00..0x0e04].copy_from_slice(&(0x09_u32 << 26).to_be_bytes());
    state.loaded_origin = 0;
    state.loaded_len = 0x0e04;
    simulator.restore(state).unwrap();
    simulator.step().unwrap();
    assert_eq!(simulator.get_state().pc, 0x100);
}

#[test]
fn atomic_load_store_requires_a_matching_reservation() {
    let mut simulator = loaded(&[
        imm(0x1b, 2, 1, 0),
        store(0x33, 1, 3, 0),
        store(0x33, 1, 3, 0),
        HALT,
    ]);
    simulator.write_register(1, 0x200).unwrap();
    simulator.write_register(3, 0x1122_3344).unwrap();
    simulator
        .write_memory(0x200, &[0xaa, 0xbb, 0xcc, 0xdd])
        .unwrap();
    simulator.step().unwrap();
    assert_eq!(simulator.read_register(2).unwrap(), 0xaabb_ccdd);
    simulator.step().unwrap();
    assert_ne!(simulator.get_state().sr & SR_F, 0);
    assert_eq!(
        simulator.read_memory(0x200, 4).unwrap(),
        vec![0x11, 0x22, 0x33, 0x44]
    );
    simulator.step().unwrap();
    assert_eq!(simulator.get_state().sr & SR_F, 0);
}

#[test]
fn only_overlapping_stores_cancel_an_atomic_reservation() {
    let mut simulator = loaded(&[
        imm(0x1b, 2, 1, 0),
        store(0x35, 5, 4, 0),
        store(0x33, 1, 3, 0),
        HALT,
    ]);
    simulator.write_register(1, 0x200).unwrap();
    simulator.write_register(3, 0x1122_3344).unwrap();
    simulator.write_register(4, 0x5566_7788).unwrap();
    simulator.write_register(5, 0x300).unwrap();
    simulator
        .write_memory(0x200, &[0xaa, 0xbb, 0xcc, 0xdd])
        .unwrap();
    simulator.step().unwrap();
    simulator.step().unwrap();
    assert_eq!(simulator.get_state().reservation, Some(0x200));
    simulator.step().unwrap();
    assert_ne!(simulator.get_state().sr & SR_F, 0);

    let mut same = loaded(&[imm(0x1b, 2, 1, 0), HALT]);
    same.write_register(1, 0x200).unwrap();
    same.step().unwrap();
    same.write_memory(0x200, &[0]).unwrap();
    assert_eq!(same.get_state().reservation, None);

    let mut partial = loaded(&[imm(0x1b, 2, 1, 0), HALT]);
    partial.write_register(1, 0x200).unwrap();
    partial.step().unwrap();
    partial.write_memory(0x203, &[0]).unwrap();
    assert_eq!(partial.get_state().reservation, None);
}

#[test]
fn traces_preserve_exact_transport_and_mnemonics() {
    let mut simulator = loaded(&[imm(0x27, 1, 0, 42), HALT]);
    let trace = simulator.step().unwrap();
    assert_eq!(trace.pc_before, 0x100);
    assert_eq!(trace.pc_after, 0x104);
    assert_eq!(trace.raw_word, imm(0x27, 1, 0, 42));
    assert_eq!(trace.raw, imm(0x27, 1, 0, 42).to_be_bytes());
    assert_eq!(trace.mnemonic, "l.addi");
    assert_eq!(trace.state_after.gpr[1], 42);
}

#[test]
fn retention_budget_and_halt_are_checked_before_mutation() {
    let mut simulator = loaded(&[HALT]);
    let before = simulator.get_state();
    assert!(matches!(
        simulator.run(1_025),
        Err(OpenRisc1000Error::RetentionLimitExceeded { .. })
    ));
    assert_eq!(simulator.get_state(), before);
    simulator.step().unwrap();
    let halted = simulator.get_state();
    assert_eq!(simulator.step(), Err(OpenRisc1000Error::Halted));
    assert_eq!(simulator.get_state(), halted);
}

#[test]
fn bounded_run_and_execute_are_transactional() {
    let mut simulator = loaded(&[NOP, NOP]);
    let before = simulator.get_state();
    assert_eq!(
        simulator.run(2),
        Err(OpenRisc1000Error::StepLimitExceeded { limit: 2 })
    );
    assert_eq!(simulator.get_state(), before);

    let result: ExecutionResult = simulator.execute(&words(&[NOP, HALT]), 0x80, 2).unwrap();
    assert!(result.halted);
    assert_eq!(result.steps, 2);
    assert_eq!(result.final_state.pc, 0x88);
}

#[test]
fn restore_rejects_incomplete_or_inconsistent_state_atomically() {
    let mut simulator = loaded(&[HALT]);
    let before = simulator.get_state();
    let mut invalid = OpenRisc1000State::default();
    invalid.memory.pop();
    assert!(matches!(
        simulator.restore(invalid),
        Err(OpenRisc1000Error::InvalidState { .. })
    ));
    assert_eq!(simulator.get_state(), before);
}

#[test]
fn malformed_words_never_panic_and_failed_steps_rollback() {
    for opcode in 0_u32..64 {
        for low in [0_u32, 1, 0x7ff, 0xffff, 0x03ff_ffff] {
            let mut simulator = loaded(&[(opcode << 26) | low, HALT]);
            let before = simulator.get_state();
            let result =
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| simulator.step()));
            assert!(result.is_ok(), "opcode {opcode:#x} panicked");
            if result.unwrap().is_err() {
                assert_eq!(simulator.get_state(), before);
            }
        }
    }
}

#[test]
fn every_error_has_stable_nonempty_display_text() {
    let errors = [
        OpenRisc1000Error::EmptyProgram,
        OpenRisc1000Error::Halted,
        OpenRisc1000Error::DivideByZero,
        OpenRisc1000Error::StepLimitExceeded { limit: 3 },
        OpenRisc1000Error::UnknownInstruction { word: 0, pc: 4 },
        OpenRisc1000Error::UnsupportedInstruction { word: 1, pc: 8 },
    ];
    for error in errors {
        assert!(!error.to_string().is_empty());
    }
}

#[test]
fn every_register_alu_subfamily_decodes_and_executes() {
    let families = [
        (0x000, "l.add"),
        (0x001, "l.addc"),
        (0x002, "l.sub"),
        (0x003, "l.and"),
        (0x004, "l.or"),
        (0x005, "l.xor"),
        (0x008, "l.sll"),
        (0x048, "l.srl"),
        (0x088, "l.sra"),
        (0x0c8, "l.ror"),
        (0x00e, "l.cmov"),
        (0x00f, "l.ff1"),
        (0x04f, "l.fl1"),
        (0x00c, "l.exths"),
        (0x00d, "l.extws"),
        (0x04c, "l.extbs"),
        (0x04d, "l.extwz"),
        (0x08c, "l.exthz"),
        (0x0cc, "l.extbz"),
        (0x306, "l.mul"),
        (0x307, "l.muld"),
        (0x309, "l.div"),
        (0x30a, "l.divu"),
        (0x30b, "l.mulu"),
        (0x30c, "l.muldu"),
    ];
    for (function, mnemonic) in families {
        let mut simulator = loaded(&[reg(3, 1, 2, function), HALT]);
        simulator.write_register(1, 0x8001_8001).unwrap();
        simulator.write_register(2, 3).unwrap();
        let trace = simulator.step().unwrap();
        assert_eq!(trace.mnemonic, mnemonic, "function {function:#05x}");
    }

    let mut simulator = loaded(&[reg(3, 1, 2, 0x309), HALT]);
    simulator.write_register(1, i32::MIN as u32).unwrap();
    simulator.write_register(2, u32::MAX).unwrap();
    assert_eq!(
        simulator.step(),
        Err(OpenRisc1000Error::SignedDivisionOverflow)
    );
}

#[test]
fn every_compare_subfamily_decodes_and_sets_flag() {
    let immediate_families = [
        (0x00, "l.sfeqi"),
        (0x01, "l.sfnei"),
        (0x02, "l.sfgtui"),
        (0x03, "l.sfgeui"),
        (0x04, "l.sfltui"),
        (0x05, "l.sfleui"),
        (0x0a, "l.sfgtsi"),
        (0x0b, "l.sfgesi"),
        (0x0c, "l.sfltsi"),
        (0x0d, "l.sflesi"),
    ];
    for (condition, mnemonic) in immediate_families {
        let mut simulator = loaded(&[compare_imm(condition, 1, 3), HALT]);
        simulator.write_register(1, 4).unwrap();
        assert_eq!(simulator.step().unwrap().mnemonic, mnemonic);
    }

    let register_families = [
        (0x00, "l.sfeq"),
        (0x01, "l.sfne"),
        (0x02, "l.sfgtu"),
        (0x03, "l.sfgeu"),
        (0x04, "l.sfltu"),
        (0x05, "l.sfleu"),
        (0x0a, "l.sfgts"),
        (0x0b, "l.sfges"),
        (0x0c, "l.sflts"),
        (0x0d, "l.sfles"),
    ];
    for (condition, mnemonic) in register_families {
        let mut simulator = loaded(&[compare(condition, 1, 2), HALT]);
        simulator.write_register(1, 4).unwrap();
        simulator.write_register(2, 3).unwrap();
        assert_eq!(simulator.step().unwrap().mnemonic, mnemonic);
    }
}

#[test]
fn synchronization_mac_and_trap_subfamilies_execute() {
    for (word, mnemonic) in [
        (0x2200_0000, "l.msync"),
        (0x2280_0000, "l.psync"),
        (0x2300_0000, "l.csync"),
    ] {
        let mut simulator = loaded(&[word, HALT]);
        assert_eq!(simulator.step().unwrap().mnemonic, mnemonic);
    }

    for (function, mnemonic) in [(1, "l.mac"), (2, "l.msb"), (3, "l.macu"), (4, "l.msbu")] {
        let word = (0x31 << 26) | (1 << 16) | (2 << 11) | function;
        let mut simulator = loaded(&[word, HALT]);
        simulator.write_register(1, 7).unwrap();
        simulator.write_register(2, 3).unwrap();
        assert_eq!(simulator.step().unwrap().mnemonic, mnemonic);
    }

    let mut simulator = loaded(&[(0x08 << 26) | (1 << 24) | 9, HALT]);
    assert_eq!(simulator.step().unwrap().mnemonic, "l.trap");
    assert_eq!(simulator.get_state().pc, 0x0e00);
}

#[test]
fn special_register_fixed_and_mapped_surfaces_are_visible() {
    let readable = [
        0_u16, 1, 2, 3, 9, 10, 16, 17, 18, 32, 48, 64, 128, 129, 1024, 1055, 0x2801, 0x2802,
    ];
    for address in readable {
        let mut simulator = loaded(&[imm(0x2d, 2, 1, address), HALT]);
        simulator.write_register(1, 0).unwrap();
        assert_eq!(simulator.step().unwrap().mnemonic, "l.mfspr");
    }

    let mut simulator = loaded(&[imm(0x2d, 2, 1, 0x222), mtspr(1, 3, 0x222), HALT]);
    simulator.write_register(3, 0xfeed_face).unwrap();
    assert_eq!(simulator.step().unwrap().mnemonic, "l.mfspr");
    assert_eq!(simulator.read_register(2).unwrap(), 0);
    assert_eq!(simulator.step().unwrap().mnemonic, "l.mtspr");

    let mut invalid_read = loaded(&[imm(0x2d, 2, 1, 0x222), HALT]);
    invalid_read.write_register(1, 0x1_0000).unwrap();
    assert!(matches!(
        invalid_read.step(),
        Err(OpenRisc1000Error::InvalidSprRead { address: 0x1_0222 })
    ));
    let read_only_write = (0x30 << 26) | (1 << 16) | (2 << 11);
    let mut simulator = loaded(&[read_only_write, HALT]);
    assert!(matches!(
        simulator.step(),
        Err(OpenRisc1000Error::InvalidSprWrite { address: 0 })
    ));
}

#[test]
fn supervisor_special_registers_reject_user_mode_atomically() {
    for word in [imm(0x2d, 2, 1, 17), mtspr(1, 2, 17)] {
        let mut simulator = loaded(&[word, HALT]);
        let mut state = simulator.get_state();
        state.sr &= !SR_SM;
        state.gpr[2] = 0xfeed_face;
        simulator.restore(state).unwrap();
        let before = simulator.get_state();
        assert_eq!(
            simulator.step(),
            Err(OpenRisc1000Error::PrivilegedInstruction { word, pc: 0x100 })
        );
        assert_eq!(simulator.get_state(), before);
    }
}

#[test]
fn mac_special_registers_remain_available_in_user_mode() {
    let mut simulator = loaded(&[imm(0x2d, 3, 0, 0x2801), mtspr(0, 2, 0x2802), HALT]);
    let mut state = simulator.get_state();
    state.sr &= !SR_SM;
    state.maclo = 0x1122_3344;
    state.gpr[2] = 0xaabb_ccdd;
    simulator.restore(state).unwrap();
    assert_eq!(simulator.step().unwrap().mnemonic, "l.mfspr");
    assert_eq!(simulator.read_register(3).unwrap(), 0x1122_3344);
    assert_eq!(simulator.step().unwrap().mnemonic, "l.mtspr");
    assert_eq!(simulator.get_state().machi, 0xaabb_ccdd);
}

#[test]
fn public_validation_and_reset_cover_all_boundaries() {
    let mut simulator = loaded(&[HALT]);
    assert!(matches!(
        simulator.load(&words(&[NOP]), 0x1_0000),
        Err(OpenRisc1000Error::ProgramOutOfRange { .. })
    ));
    assert!(matches!(
        simulator.read_memory(0x1_0000, 1),
        Err(OpenRisc1000Error::DataOutOfRange { .. })
    ));
    assert_eq!(
        simulator.read_register(32),
        Err(OpenRisc1000Error::InvalidRegister { register: 32 })
    );
    assert_eq!(
        simulator.write_register(32, 1),
        Err(OpenRisc1000Error::InvalidRegister { register: 32 })
    );

    let mut state = simulator.get_state();
    state.pc = 3;
    assert!(matches!(
        simulator.restore(state),
        Err(OpenRisc1000Error::InvalidState { .. })
    ));
    let mut invalid_reservation = simulator.get_state();
    invalid_reservation.reservation = Some(0x1_0000);
    assert!(matches!(
        simulator.restore(invalid_reservation),
        Err(OpenRisc1000Error::InvalidState { .. })
    ));
    simulator.reset();
    assert_eq!(simulator.get_state(), OpenRisc1000State::default());
}

#[test]
fn unsupported_and_malformed_subfamilies_are_typed() {
    for word in [
        0x0a_u32 << 26,
        0x1a_u32 << 26,
        0x20_u32 << 26,
        0x32_u32 << 26,
        0x1c_u32 << 26,
        0x3f_u32 << 26,
    ] {
        let mut simulator = loaded(&[word, HALT]);
        assert!(matches!(
            simulator.step(),
            Err(OpenRisc1000Error::UnsupportedInstruction { .. })
        ));
    }

    for word in [
        (0x2e_u32 << 26) | 0x100,
        (0x2f_u32 << 26) | (6 << 21),
        (0x31_u32 << 26) | 5,
        (0x38_u32 << 26) | 0x3ff,
        (0x39_u32 << 26) | (6 << 21),
    ] {
        let mut simulator = loaded(&[word, HALT]);
        assert!(matches!(
            simulator.step(),
            Err(OpenRisc1000Error::UnknownInstruction { .. })
        ));
    }
}
