use coding_adventures_ge225_simulator::{
    assemble_card_io, assemble_fixed, assemble_shift, CardFormat as FunctionalCardFormat,
    CardStatus as FunctionalCardStatus, Simulator as Functional,
};
use ge225_gatelevel::{
    CardFormat, CardStatus, Ge225GateError, Ge225GateLevel, NRegisterDevice, PaperTapeFrame,
    MIN_MEMORY_WORDS,
};

const PROGRAM: usize = 0o2000;
const SIGN_BIT: i32 = 1 << 19;
const HOPPER_EMPTY_BIT: i32 = 1 << 18;

fn card(format: CardFormat, seed: i32) -> Vec<i32> {
    (0..format.word_count() as i32)
        .map(|offset| seed + offset)
        .collect()
}

fn machine(program: &[i32]) -> Ge225GateLevel {
    let mut gate = Ge225GateLevel::new(MIN_MEMORY_WORDS).unwrap();
    gate.load_words(program, PROGRAM).unwrap();
    gate.set_program_counter(PROGRAM as i32).unwrap();
    gate
}

#[test]
fn decimal_continuous_reader_rotates_four_gate_backed_dma_slots() {
    let mut gate = machine(&[assemble_card_io("RCD", 0o400, 0).unwrap()]);
    for seed in [0o100, 0o200, 0o300, 0o400, 0o500] {
        gate.queue_card_reader_card(
            CardFormat::Decimal,
            &card(CardFormat::Decimal, seed),
            CardStatus::default(),
        )
        .unwrap();
    }

    gate.step().unwrap();
    assert_eq!(gate.read_word(0o400).unwrap(), 0o100);
    assert_eq!(gate.read_word(0o400 + 27).unwrap(), 0o2606077);
    for (seed, slot) in [(0o200, 1), (0o300, 2), (0o400, 3), (0o500, 0)] {
        assert!(gate.advance_card_reader().unwrap());
        assert_eq!(gate.read_word(0o400 + slot * 32).unwrap(), seed);
    }
    let final_sync = gate.read_word(0o400 + 27).unwrap();
    assert_ne!(final_sync & SIGN_BIT, 0);
    assert_ne!(final_sync & HOPPER_EMPTY_BIT, 0);
    assert!(gate.advance_card_reader().is_err());
    assert!(!gate.get_state().card_reader_ready);
}

#[test]
fn halt_reader_restores_ready_and_ready_branch_is_program_visible() {
    let mut gate = machine(&[
        assemble_card_io("RCB", 0o400, 0).unwrap(),
        assemble_fixed("HCR").unwrap(),
        assemble_fixed("BCR").unwrap(),
    ]);
    for seed in [1, 2] {
        gate.queue_card_reader_card(
            CardFormat::Binary10,
            &card(CardFormat::Binary10, seed),
            CardStatus::default(),
        )
        .unwrap();
    }
    gate.step().unwrap();
    assert!(!gate.get_state().card_reader_ready);
    gate.step().unwrap();
    assert!(gate.get_state().card_reader_ready);
    gate.step().unwrap();
    assert_eq!(gate.get_state().pc, PROGRAM as i32 + 3);
}

#[test]
fn full_mixed_and_mismatched_reads_preserve_atomic_alarm_behavior() {
    let mut full = machine(&[
        assemble_card_io("RCF", 0o400, 0).unwrap(),
        assemble_card_io("RCM", 0o600, 0).unwrap(),
    ]);
    let full_card = card(CardFormat::Full12, 0o1000);
    full.queue_card_reader_card(CardFormat::Full12, &full_card, CardStatus::default())
        .unwrap();
    let mixed_card = card(CardFormat::MixedBinary, 0o2000);
    full.queue_card_reader_card(CardFormat::MixedBinary, &mixed_card, CardStatus::default())
        .unwrap();
    full.step().unwrap();
    full.step().unwrap();
    assert_eq!(full.read_word(0o400 + 79).unwrap(), full_card[79]);
    assert_ne!(full.read_word(0o600).unwrap() & SIGN_BIT, 0);

    let mut mismatch = machine(&[assemble_card_io("RCD", 0o400, 0).unwrap()]);
    mismatch
        .queue_card_reader_card(
            CardFormat::Binary10,
            &card(CardFormat::Binary10, 7),
            CardStatus::default(),
        )
        .unwrap();
    mismatch.step().unwrap();
    let state = mismatch.get_state();
    assert!(state.halted && state.card_reader_alarm && state.priority_alarm);
    assert_eq!(state.memory[0o400], 0);
}

#[test]
fn punch_formats_and_offline_alarm_recovery_are_gate_backed() {
    let mut gate = machine(&[
        assemble_card_io("WCD", 0o400, 0).unwrap(),
        assemble_card_io("WCB", 0o400, 0).unwrap(),
        assemble_card_io("WCF", 0o400, 0).unwrap(),
    ]);
    let source: Vec<i32> = (0..128).map(|word| 0o1000 + word).collect();
    gate.load_words(&source, 0o400).unwrap();
    gate.run(3).unwrap();
    assert_eq!(gate.card_punch_output()[0].words.len(), 27);
    assert_eq!(gate.card_punch_output()[1].words.len(), 40);
    assert_eq!(gate.card_punch_output()[2].words.len(), 80);

    let mut offline = machine(&[assemble_card_io("WCD", 0o400, 0).unwrap()]);
    offline.set_card_punch_online(false);
    offline.step().unwrap();
    assert!(offline.get_state().card_punch_alarm);
    assert!(offline.card_punch_output().is_empty());
    offline.set_card_punch_online(true);
    offline.clear_direct_io_alarms();
    offline.set_program_counter(PROGRAM as i32).unwrap();
    offline.step().unwrap();
    assert_eq!(offline.card_punch_output().len(), 1);
}

#[test]
fn card_modification_preserves_mode_in_the_gate_i_register() {
    let mut gate = machine(&[assemble_card_io("RCB", 0o200, 1).unwrap()]);
    gate.write_word(1, 0o200).unwrap();
    gate.queue_card_reader_card(
        CardFormat::Binary10,
        &card(CardFormat::Binary10, 0o700),
        CardStatus::default(),
    )
    .unwrap();
    gate.step().unwrap();
    assert_eq!(gate.read_word(0o400).unwrap(), 0o700);
    assert_eq!(
        gate.get_state().ir,
        assemble_card_io("RCB", 0o400, 1).unwrap()
    );
}

#[test]
fn shared_n_command_routes_reader_punch_and_typewriter_streams() {
    let mut gate = machine(&[
        assemble_fixed("RON").unwrap(),
        assemble_fixed("RPT").unwrap(),
        assemble_shift("SNA", 6).unwrap(),
        assemble_fixed("HPT").unwrap(),
        assemble_fixed("PON").unwrap(),
        assemble_fixed("WPT").unwrap(),
        assemble_fixed("TON").unwrap(),
        assemble_fixed("TYP").unwrap(),
    ]);
    gate.queue_paper_tape_input(&[0o21, 0o22, 0o23]).unwrap();
    gate.step().unwrap();
    assert_eq!(gate.get_state().n_device, NRegisterDevice::PaperTapeReader);
    gate.step().unwrap();
    assert_eq!(gate.get_state().n, 0o21);
    gate.step().unwrap();
    assert!(gate.advance_paper_tape_reader().unwrap());
    assert!(gate.advance_paper_tape_reader().unwrap());
    assert!(gate.get_state().n_overrun);
    gate.step().unwrap();
    gate.step().unwrap();
    gate.step().unwrap();
    assert_eq!(gate.paper_tape_output(), &[0o23]);
    gate.step().unwrap();
    gate.step().unwrap();
    assert_eq!(gate.get_typewriter_output(), "C");
}

#[test]
fn keyboard_input_and_parity_stop_follow_gate_latches() {
    let mut keyboard = machine(&[
        assemble_fixed("TON").unwrap(),
        assemble_fixed("HPT").unwrap(),
        assemble_shift("SNA", 6).unwrap(),
    ]);
    keyboard.queue_typewriter_input(&[0o21, 0o22]).unwrap();
    keyboard.step().unwrap();
    keyboard.step().unwrap();
    assert!(keyboard.advance_typewriter_input().unwrap());
    keyboard.step().unwrap();
    assert!(keyboard.advance_typewriter_input().unwrap());
    assert_eq!(keyboard.get_state().n, 0o22);

    let mut tape = machine(&[
        assemble_fixed("RON").unwrap(),
        assemble_fixed("RPT").unwrap(),
    ]);
    tape.set_stop_on_parity_alarm(true);
    tape.queue_paper_tape_frames(&[PaperTapeFrame {
        data: 0o22,
        parity_error: true,
    }])
    .unwrap();
    tape.run(2).unwrap();
    let state = tape.get_state();
    assert!(state.halted && state.parity_error && state.priority_alarm);
    assert!(!state.paper_tape_reader_running);
}

#[test]
fn card_status_and_direct_ready_branches_are_visible_to_programs() {
    let mut reader = machine(&[
        assemble_fixed("BCR").unwrap(),
        assemble_card_io("RCD", 0o400, 0).unwrap(),
    ]);
    reader
        .queue_card_reader_card(
            CardFormat::Decimal,
            &card(CardFormat::Decimal, 1),
            CardStatus {
                invalid_character: true,
                output_stacker_full: true,
                reader_malfunction: true,
                end_of_file: true,
            },
        )
        .unwrap();
    reader.step().unwrap();
    assert_eq!(reader.get_state().pc, PROGRAM as i32 + 1);
    reader.step().unwrap();
    let sync = reader.read_word(0o400 + 27).unwrap();
    assert_ne!(sync & SIGN_BIT, 0);
    assert_ne!(sync & HOPPER_EMPTY_BIT, 0);
    assert_eq!(sync & 0o17, 0);

    let mut punch = machine(&[assemble_fixed("BPN").unwrap()]);
    punch.set_card_punch_online(false);
    punch.step().unwrap();
    assert_eq!(punch.get_state().pc, PROGRAM as i32 + 1);
}

#[test]
fn direct_io_instruction_sequence_runs_in_functional_lockstep() {
    let program = [
        assemble_fixed("BCR").unwrap(),
        assemble_card_io("RCD", 0o400, 0).unwrap(),
        assemble_fixed("HCR").unwrap(),
        assemble_fixed("RON").unwrap(),
        assemble_fixed("RPT").unwrap(),
        assemble_shift("SNA", 6).unwrap(),
        assemble_fixed("PON").unwrap(),
        assemble_fixed("WPT").unwrap(),
    ];
    let mut gate = machine(&program);
    let mut functional = Functional::new(MIN_MEMORY_WORDS as i32).unwrap();
    functional.load_words(&program, PROGRAM as i32).unwrap();
    functional.set_program_counter(PROGRAM as i32).unwrap();
    for seed in [0o100, 0o200] {
        let words = card(CardFormat::Decimal, seed);
        gate.queue_card_reader_card(CardFormat::Decimal, &words, CardStatus::default())
            .unwrap();
        functional
            .queue_card_reader_card(
                FunctionalCardFormat::Decimal,
                &words,
                FunctionalCardStatus::default(),
            )
            .unwrap();
    }
    gate.queue_paper_tape_input(&[0o21]).unwrap();
    functional.queue_paper_tape_input(&[0o21]).unwrap();

    for _ in 0..program.len() {
        let gate_trace = gate.step().unwrap();
        let functional_trace = functional.step().unwrap();
        assert_eq!(
            gate_trace.mnemonic,
            functional_trace.mnemonic.split_whitespace().next().unwrap()
        );
        let gate_state = gate.get_state();
        let functional_state = functional.get_state();
        assert_eq!(gate_state.a, functional_state.a);
        assert_eq!(gate_state.n, functional_state.n);
        assert_eq!(gate_state.pc, functional_state.pc);
        assert_eq!(gate_state.ir, functional_state.ir);
        assert_eq!(gate_state.parity_error, functional_state.parity_error);
        assert_eq!(gate_state.n_ready, functional_state.n_ready);
        assert_eq!(
            gate_state.card_reader_ready,
            functional_state.card_reader_ready
        );
        assert_eq!(
            gate_state.card_punch_ready,
            functional_state.card_punch_ready
        );
        assert_eq!(
            gate_state.card_reader_alarm,
            functional_state.card_reader_alarm
        );
        assert_eq!(
            gate_state.card_punch_alarm,
            functional_state.card_punch_alarm
        );
        assert_eq!(gate_state.priority_alarm, functional_state.priority_alarm);
        assert_eq!(gate_state.memory, functional_state.memory);
    }
    assert_eq!(gate.paper_tape_output(), functional.paper_tape_output());
}

#[test]
fn invalid_modified_card_base_fails_before_clocking_gate_state() {
    let mut gate = machine(&[assemble_card_io("RCB", 0o200, 1).unwrap()]);
    gate.write_word(1, 1).unwrap();
    gate.queue_card_reader_card(
        CardFormat::Binary10,
        &card(CardFormat::Binary10, 0o700),
        CardStatus::default(),
    )
    .unwrap();
    let before = gate.get_state();

    assert_eq!(
        gate.step(),
        Err(Ge225GateError::InvalidCardAddress { address: 0o201 })
    );
    assert_eq!(gate.get_state(), before);
    assert!(gate.card_punch_output().is_empty());
}
