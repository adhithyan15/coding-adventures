use coding_adventures_ge225_simulator::{
    assemble_fixed, assemble_shift, encode_instruction, Simulator,
};

const MASK_20: i32 = (1 << 20) - 1;

fn instruction(opcode: i32, address: i32, modifier: i32) -> i32 {
    encode_instruction(opcode, modifier, address).unwrap()
}

#[test]
fn constructor_rejects_invalid_or_oversized_memory_without_allocating() {
    assert!(Simulator::new(-1).is_err());
    assert!(Simulator::new(0).is_err());
    assert!(Simulator::new(4_095).is_err());
    assert!(Simulator::new(16_385).is_err());
}

#[test]
fn maximum_negative_san_shift_is_defined_and_non_panicking() {
    let mut simulator = Simulator::new(4096).unwrap();
    simulator
        .load_words(
            &[
                assemble_fixed("LMO").unwrap(),
                assemble_shift("SAN", 31).unwrap(),
            ],
            4,
        )
        .unwrap();
    simulator.set_program_counter(4).unwrap();

    simulator.run(2).unwrap();

    let state = simulator.get_state();
    assert_eq!(state.a, MASK_20);
    assert_eq!(state.n, 0o77);
}

#[test]
fn card_reader_input_has_explicit_record_and_queue_bounds() {
    let mut simulator = Simulator::new(4096).unwrap();
    assert!(simulator.queue_card_reader_record(&[0; 28]).is_err());
    for _ in 0..64 {
        simulator.queue_card_reader_record(&[]).unwrap();
    }
    assert!(simulator.queue_card_reader_record(&[]).is_err());
}

#[test]
fn load_words_rejects_the_whole_out_of_range_write() {
    let mut simulator = Simulator::new(4096).unwrap();
    simulator.write_word(4095, 0x12345).unwrap();

    let error = simulator.load_words(&[0xAAAAA, 0xBBBBB], 4095).unwrap_err();

    assert!(error.contains("address range out of range"));
    assert_eq!(simulator.read_word(4095).unwrap(), 0x12345);
}

#[test]
fn address_modification_uses_reserved_core_x_words() {
    let mut simulator = Simulator::new(4096).unwrap();
    simulator.write_word(1, 4).unwrap();
    simulator.write_word(20, 0x12345).unwrap();
    simulator
        .load_words(&[instruction(0o00, 16, 1)], 4)
        .unwrap();
    simulator.set_program_counter(4).unwrap();

    let trace = simulator.step().unwrap();

    assert_eq!(trace.effective_address, Some(20));
    assert_eq!(simulator.get_state().a, 0x12345);
    assert_eq!(simulator.get_state().x_words[1], 4);
    assert_eq!(simulator.get_state().ir, instruction(0o00, 20, 1));
}

#[test]
fn modified_address_outside_installed_memory_fails_instead_of_wrapping() {
    let mut simulator = Simulator::new(4096).unwrap();
    simulator.write_word(1, 10).unwrap();
    simulator
        .load_words(&[instruction(0o00, 4090, 1)], 4)
        .unwrap();
    simulator.set_program_counter(4).unwrap();
    let before = simulator.get_state();

    let error = simulator.step().unwrap_err();

    assert!(error.contains("address out of range: 4100"));
    assert_eq!(simulator.get_state(), before);
}

#[test]
fn index_load_and_store_use_the_selected_core_words() {
    let mut simulator = Simulator::new(4096).unwrap();
    simulator.write_word(20, 0x13579).unwrap();
    simulator
        .load_words(&[instruction(0o06, 20, 2), instruction(0o17, 21, 2)], 4)
        .unwrap();
    simulator.set_program_counter(4).unwrap();

    simulator.step().unwrap();
    assert_eq!(simulator.read_word(2).unwrap(), 0x13579);
    simulator.step().unwrap();

    assert_eq!(simulator.read_word(21).unwrap(), 0x13579);
}

#[test]
fn inx_updates_only_the_documented_fifteen_bit_x_field() {
    let mut simulator = Simulator::new(4096).unwrap();
    simulator.write_word(1, 0o3700001).unwrap();
    simulator.load_words(&[instruction(0o14, 1, 1)], 4).unwrap();
    simulator.set_program_counter(4).unwrap();

    simulator.step().unwrap();

    assert_eq!(simulator.read_word(1).unwrap(), 0o3700002);
}

#[test]
fn spb_stores_its_own_instruction_address() {
    let mut simulator = Simulator::new(4096).unwrap();
    simulator.load_words(&[instruction(0o07, 8, 2)], 4).unwrap();
    simulator.set_program_counter(4).unwrap();

    simulator.step().unwrap();

    let state = simulator.get_state();
    assert_eq!(state.pc, 8);
    assert_eq!(state.memory[2], 4);
    assert_eq!(state.x_words[2], 4);
}

#[test]
fn spb_validates_the_branch_before_overwriting_its_x_word() {
    let mut simulator = Simulator::new(4096).unwrap();
    simulator.write_word(2, 0x12345).unwrap();
    simulator
        .load_words(&[instruction(0o07, 4096, 2)], 4)
        .unwrap();
    simulator.set_program_counter(4).unwrap();

    let error = simulator.step().unwrap_err();

    assert!(error.contains("address out of range: 4096"));
    assert_eq!(simulator.read_word(2).unwrap(), 0x12345);
}

#[test]
fn unmodified_upper_bank_branches_preserve_the_instruction_bank() {
    let mut bru = Simulator::new(16_384).unwrap();
    bru.load_words(&[instruction(0o26, 12, 0)], 8_200).unwrap();
    bru.set_program_counter(8_200).unwrap();
    let trace = bru.step().unwrap();
    assert_eq!(trace.effective_address, Some(8_204));
    assert_eq!(bru.get_state().pc, 8_204);

    let mut spb = Simulator::new(16_384).unwrap();
    spb.load_words(&[instruction(0o07, 12, 2)], 8_200).unwrap();
    spb.set_program_counter(8_200).unwrap();
    spb.step().unwrap();
    assert_eq!(spb.get_state().pc, 8_204);
    assert_eq!(spb.read_word(2).unwrap(), 8_200);
}

#[test]
fn branch_bank_selection_matches_instruction_timing_at_the_8191_boundary() {
    let mut bru = Simulator::new(16_384).unwrap();
    bru.load_words(&[instruction(0o26, 12, 0)], 8_191).unwrap();
    bru.set_program_counter(8_191).unwrap();
    let trace = bru.step().unwrap();
    assert_eq!(trace.effective_address, Some(8_204));
    assert_eq!(bru.get_state().pc, 8_204);

    let mut spb = Simulator::new(16_384).unwrap();
    spb.load_words(&[instruction(0o07, 12, 2)], 8_191).unwrap();
    spb.set_program_counter(8_191).unwrap();
    spb.step().unwrap();
    assert_eq!(spb.get_state().pc, 12);
    assert_eq!(spb.read_word(2).unwrap(), 8_191);
}

#[test]
fn modified_bru_can_select_a_different_bank() {
    let mut simulator = Simulator::new(16_384).unwrap();
    simulator.write_word(1, 50).unwrap();
    simulator
        .load_words(&[instruction(0o26, 50, 1)], 8_200)
        .unwrap();
    simulator.set_program_counter(8_200).unwrap();

    let trace = simulator.step().unwrap();

    assert_eq!(trace.effective_address, Some(100));
    assert_eq!(simulator.get_state().pc, 100);
}

#[test]
fn even_double_word_at_memory_end_fails_without_wrapping() {
    let mut simulator = Simulator::new(4097).unwrap();
    simulator.write_word(4096, 0x12345).unwrap();
    simulator
        .load_words(&[instruction(0o10, 4096, 0)], 4)
        .unwrap();
    simulator.set_program_counter(4).unwrap();

    let error = simulator.step().unwrap_err();

    assert!(error.contains("address out of range: 4097"));
    assert_eq!(simulator.get_state().a, 0);
    assert_eq!(simulator.get_state().q, 0);
}

#[test]
fn card_read_is_atomic_and_keeps_the_record_after_a_range_error() {
    let mut simulator = Simulator::new(4096).unwrap();
    simulator
        .queue_card_reader_record(&[0x11111, 0x22222])
        .unwrap();
    simulator
        .load_words(&[instruction(0o25, 4095, 0)], 4)
        .unwrap();
    simulator.set_program_counter(4).unwrap();

    let error = simulator.step().unwrap_err();
    assert!(error.contains("address range out of range"));
    assert_eq!(simulator.read_word(4095).unwrap(), 0);

    simulator
        .load_words(&[instruction(0o25, 10, 0)], 4)
        .unwrap();
    simulator.set_program_counter(4).unwrap();
    simulator.step().unwrap();
    assert_eq!(simulator.read_word(10).unwrap(), 0x11111);
    assert_eq!(simulator.read_word(11).unwrap(), 0x22222);
}

#[test]
fn overlapping_mov_reads_the_complete_source_before_writing() {
    let mut simulator = Simulator::new(4096).unwrap();
    simulator.write_word(20, 0x11111).unwrap();
    simulator.write_word(21, 0x22222).unwrap();
    simulator.write_word(30, 21).unwrap();
    simulator.write_word(31, MASK_20 - 1).unwrap();
    simulator
        .load_words(
            &[
                instruction(0o00, 30, 0),
                0o2504004,
                instruction(0o00, 31, 0),
                0o2504005,
                instruction(0o24, 20, 0),
            ],
            4,
        )
        .unwrap();
    simulator.set_program_counter(4).unwrap();

    simulator.run(5).unwrap();

    assert_eq!(simulator.read_word(21).unwrap(), 0x11111);
    assert_eq!(simulator.read_word(22).unwrap(), 0x22222);
}

#[test]
fn branch_target_outside_installed_memory_fails_closed() {
    let mut simulator = Simulator::new(4096).unwrap();
    simulator
        .load_words(&[instruction(0o26, 4096, 0)], 4)
        .unwrap();
    simulator.set_program_counter(4).unwrap();
    let before = simulator.get_state();

    let error = simulator.step().unwrap_err();

    assert!(error.contains("address out of range: 4096"));
    assert_eq!(simulator.get_state(), before);
}

#[test]
fn final_installed_word_cannot_leave_a_successful_out_of_range_pc() {
    let mut simulator = Simulator::new(4096).unwrap();
    simulator
        .load_words(&[assemble_fixed("NOP").unwrap()], 4095)
        .unwrap();
    simulator.set_program_counter(4095).unwrap();

    let error = simulator.step().unwrap_err();

    assert!(error.contains("sequential P counter leaves installed memory"));
    assert_eq!(simulator.get_state().pc, 4095);

    simulator
        .load_words(&[instruction(0o26, 4, 0)], 4095)
        .unwrap();
    let trace = simulator.step().unwrap();
    assert_eq!(trace.address, 4095);
    assert_eq!(simulator.get_state().pc, 4);
}
