use coding_adventures_ge225_simulator::{
    assemble_card_io, assemble_fixed, assemble_shift, CardFormat, CardStatus, NRegisterDevice,
    PaperTapeFrame, Simulator,
};

const SIGN_BIT: i32 = 1 << 19;
const HOPPER_EMPTY_BIT: i32 = 1 << 18;

fn card(format: CardFormat, seed: i32) -> Vec<i32> {
    (0..match format {
        CardFormat::Decimal => 27,
        CardFormat::Binary10 => 40,
        CardFormat::Full12 | CardFormat::MixedDecimal | CardFormat::MixedBinary => 80,
    })
        .map(|offset| seed + offset)
        .collect()
}

#[test]
fn card_instruction_encoding_requires_hardware_alignment() {
    let sim = Simulator::new(4096).unwrap();
    for (mnemonic, mode) in [
        ("RCD", 0o00),
        ("RCB", 0o01),
        ("WCD", 0o02),
        ("WCB", 0o03),
        ("RCF", 0o10),
        ("RCM", 0o12),
        ("WCF", 0o17),
    ] {
        let word = assemble_card_io(mnemonic, 0o400, 0).unwrap();
        assert_eq!(word, 0o2500400 | mode);
        assert_eq!(
            sim.disassemble_word(word).unwrap(),
            format!("{mnemonic} 0x100,X0")
        );
    }
    assert!(assemble_card_io("RCD", 127, 0).is_err());
    assert!(assemble_card_io("RCD", 2048, 0).is_err());
    assert!(assemble_card_io("BAD", 128, 0).is_err());
    assert!(sim.disassemble_word(0o2500070).is_err());
}

#[test]
fn decimal_continuous_read_rotates_four_areas_and_sets_sync_status() {
    let mut sim = Simulator::new(4096).unwrap();
    for seed in [0o100, 0o200, 0o300, 0o400, 0o500] {
        sim.queue_card_reader_card(
            CardFormat::Decimal,
            &card(CardFormat::Decimal, seed),
            CardStatus::default(),
        )
        .unwrap();
    }
    sim.load_words(&[assemble_card_io("RCD", 0o400, 0).unwrap()], 0o2000)
        .unwrap();
    sim.set_program_counter(0o2000).unwrap();
    sim.step().unwrap();
    assert_eq!(sim.read_word(0o400).unwrap(), 0o100);
    assert_eq!(sim.read_word(0o400 + 27).unwrap(), 0o2606077);

    for expected in [0o200, 0o300, 0o400, 0o500] {
        assert!(sim.advance_card_reader().unwrap());
        let slot = match expected {
            0o200 => 1,
            0o300 => 2,
            0o400 => 3,
            0o500 => 0,
            _ => unreachable!(),
        };
        assert_eq!(sim.read_word(0o400 + slot * 32).unwrap(), expected);
    }
    let final_sync = sim.read_word(0o400 + 27).unwrap();
    assert_ne!(final_sync & SIGN_BIT, 0);
    assert_ne!(final_sync & HOPPER_EMPTY_BIT, 0);
    assert!(sim.advance_card_reader().is_err());
    assert!(!sim.get_state().card_reader_ready);
}

#[test]
fn halt_reader_stops_continuous_dma_and_restores_ready_state() {
    let mut sim = Simulator::new(4096).unwrap();
    for seed in [1, 2] {
        sim.queue_card_reader_card(
            CardFormat::Binary10,
            &card(CardFormat::Binary10, seed),
            CardStatus::default(),
        )
        .unwrap();
    }
    sim.load_words(
        &[
            assemble_card_io("RCB", 0o400, 0).unwrap(),
            assemble_fixed("HCR").unwrap(),
            assemble_fixed("BCR").unwrap(),
        ],
        0o2000,
    )
    .unwrap();
    sim.set_program_counter(0o2000).unwrap();
    sim.step().unwrap();
    assert!(!sim.get_state().card_reader_ready);
    sim.step().unwrap();
    assert!(sim.get_state().card_reader_ready);
    sim.step().unwrap();
    assert_eq!(sim.get_state().pc, 0o2003);
    assert!(sim.advance_card_reader().is_err());
}

#[test]
fn full_and_mixed_reads_use_eighty_words_and_are_atomic_on_mode_error() {
    let mut sim = Simulator::new(4096).unwrap();
    let full = card(CardFormat::Full12, 0o1000);
    sim.queue_card_reader_card(CardFormat::Full12, &full, CardStatus::default())
        .unwrap();
    sim.load_words(&[assemble_card_io("RCF", 0o400, 0).unwrap()], 0o2000)
        .unwrap();
    sim.set_program_counter(0o2000).unwrap();
    sim.step().unwrap();
    assert_eq!(sim.read_word(0o400 + 79).unwrap(), full[79]);
    assert_eq!(sim.read_word(0o400 + 83).unwrap() & SIGN_BIT, SIGN_BIT);

    let mixed = card(CardFormat::MixedBinary, 0o2000);
    sim.queue_card_reader_card(CardFormat::MixedBinary, &mixed, CardStatus::default())
        .unwrap();
    sim.load_words(&[assemble_card_io("RCM", 0o600, 0).unwrap()], 0o2001)
        .unwrap();
    sim.set_program_counter(0o2001).unwrap();
    sim.step().unwrap();
    assert_ne!(sim.read_word(0o600).unwrap() & SIGN_BIT, 0);

    let mut mismatch = Simulator::new(4096).unwrap();
    mismatch
        .queue_card_reader_card(
            CardFormat::Binary10,
            &card(CardFormat::Binary10, 7),
            CardStatus::default(),
        )
        .unwrap();
    mismatch
        .load_words(&[assemble_card_io("RCD", 0o400, 0).unwrap()], 0o2000)
        .unwrap();
    mismatch.set_program_counter(0o2000).unwrap();
    mismatch.step().unwrap();
    let state = mismatch.get_state();
    assert!(state.halted);
    assert!(state.card_reader_alarm);
    assert!(state.priority_alarm);
    assert_eq!(state.memory[0o400], 0);
}

#[test]
fn card_punch_captures_all_three_formats_and_not_ready_halts() {
    let mut sim = Simulator::new(4096).unwrap();
    let source: Vec<i32> = (0..128).map(|word| 0o1000 + word).collect();
    sim.load_words(&source, 0o400).unwrap();
    sim.load_words(
        &[
            assemble_card_io("WCD", 0o400, 0).unwrap(),
            assemble_card_io("WCB", 0o400, 0).unwrap(),
            assemble_card_io("WCF", 0o400, 0).unwrap(),
        ],
        0o2000,
    )
    .unwrap();
    sim.set_program_counter(0o2000).unwrap();
    sim.run(3).unwrap();
    let output = sim.card_punch_output();
    assert_eq!(output.len(), 3);
    assert_eq!(output[0].format, CardFormat::Decimal);
    assert_eq!(output[0].words.len(), 27);
    assert_eq!(output[1].format, CardFormat::Binary10);
    assert_eq!(output[1].words.len(), 40);
    assert_eq!(output[2].format, CardFormat::Full12);
    assert_eq!(output[2].words.len(), 80);

    let mut offline = Simulator::new(4096).unwrap();
    offline.set_card_punch_online(false);
    offline
        .load_words(&[assemble_card_io("WCD", 0o400, 0).unwrap()], 0o2000)
        .unwrap();
    offline.set_program_counter(0o2000).unwrap();
    offline.step().unwrap();
    let state = offline.get_state();
    assert!(state.halted);
    assert!(state.card_punch_alarm);
    assert!(state.priority_alarm);
    assert!(offline.card_punch_output().is_empty());
    offline.set_card_punch_online(true);
    offline.clear_direct_io_alarms();
    offline.set_program_counter(0o2000).unwrap();
    offline.step().unwrap();
    assert!(!offline.get_state().halted);
    assert_eq!(offline.card_punch_output().len(), 1);
}

#[test]
fn card_automatic_modification_preserves_mode_in_i_register() {
    let mut sim = Simulator::new(4096).unwrap();
    sim.write_word(1, 0o200).unwrap();
    sim.queue_card_reader_card(
        CardFormat::Binary10,
        &card(CardFormat::Binary10, 0o700),
        CardStatus::default(),
    )
    .unwrap();
    sim.load_words(&[assemble_card_io("RCB", 0o200, 1).unwrap()], 0o2000)
        .unwrap();
    sim.set_program_counter(0o2000).unwrap();
    sim.step().unwrap();
    assert_eq!(sim.read_word(0o400).unwrap(), 0o700);
    assert_eq!(
        sim.get_state().ir,
        assemble_card_io("RCB", 0o400, 1).unwrap()
    );
}

#[test]
fn shared_n_command_obeys_power_selection_and_stream_readiness() {
    let mut sim = Simulator::new(4096).unwrap();
    sim.queue_paper_tape_input(&[0o21, 0o22, 0o23]).unwrap();
    sim.load_words(
        &[
            assemble_fixed("RON").unwrap(),
            assemble_fixed("RPT").unwrap(),
            assemble_shift("SNA", 6).unwrap(),
            assemble_fixed("HPT").unwrap(),
            assemble_fixed("PON").unwrap(),
            assemble_fixed("WPT").unwrap(),
            assemble_fixed("TON").unwrap(),
            assemble_fixed("TYP").unwrap(),
        ],
        0o2000,
    )
    .unwrap();
    sim.set_program_counter(0o2000).unwrap();
    sim.step().unwrap();
    assert_eq!(sim.get_state().n_device, NRegisterDevice::PaperTapeReader);
    assert_eq!(sim.disassemble_word(0o2500006).unwrap(), "RPT");
    sim.step().unwrap();
    assert_eq!(sim.get_state().n, 0o21);
    assert!(sim.get_state().n_ready);
    sim.step().unwrap();
    assert!(!sim.get_state().n_ready);
    assert!(sim.advance_paper_tape_reader().unwrap());
    assert_eq!(sim.get_state().n, 0o22);
    assert!(sim.advance_paper_tape_reader().unwrap());
    assert!(sim.get_state().n_overrun);
    sim.step().unwrap();
    assert!(!sim.get_state().paper_tape_reader_running);
    sim.step().unwrap();
    assert_eq!(sim.disassemble_word(0o2500006).unwrap(), "WPT");
    sim.step().unwrap();
    assert_eq!(sim.paper_tape_output(), &[0o23]);
    sim.step().unwrap();
    assert_eq!(sim.disassemble_word(0o2500006).unwrap(), "TYP");
    sim.step().unwrap();
    assert_eq!(sim.get_typewriter_output(), "C");
}

#[test]
fn hpt_enables_deterministic_typewriter_keyboard_input() {
    let mut sim = Simulator::new(4096).unwrap();
    sim.queue_typewriter_input(&[0o21, 0o22]).unwrap();
    sim.load_words(
        &[
            assemble_fixed("TON").unwrap(),
            assemble_fixed("HPT").unwrap(),
            assemble_shift("SNA", 6).unwrap(),
        ],
        0o2000,
    )
    .unwrap();
    sim.set_program_counter(0o2000).unwrap();
    sim.step().unwrap();
    sim.step().unwrap();
    assert!(sim.get_state().typewriter_keyboard_enabled);
    assert!(!sim.get_state().n_ready);
    assert!(sim.advance_typewriter_input().unwrap());
    assert_eq!(sim.get_state().n, 0o21);
    sim.step().unwrap();
    assert!(!sim.get_state().n_ready);
    assert!(sim.advance_typewriter_input().unwrap());
    assert_eq!(sim.get_state().n, 0o22);
}

#[test]
fn direct_ready_branches_sync_errors_and_tape_parity_are_program_visible() {
    let mut reader = Simulator::new(4096).unwrap();
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
    reader
        .load_words(
            &[
                assemble_fixed("BCR").unwrap(),
                assemble_card_io("RCD", 0o400, 0).unwrap(),
            ],
            0o2000,
        )
        .unwrap();
    reader.set_program_counter(0o2000).unwrap();
    reader.step().unwrap();
    assert_eq!(reader.get_state().pc, 0o2001);
    reader.step().unwrap();
    let sync = reader.read_word(0o400 + 27).unwrap();
    assert_ne!(sync & SIGN_BIT, 0);
    assert_ne!(sync & HOPPER_EMPTY_BIT, 0);
    assert_eq!(sync & 0o17, 0);

    let mut punch = Simulator::new(4096).unwrap();
    punch.set_card_punch_online(false);
    punch
        .load_words(&[assemble_fixed("BPN").unwrap()], 0o2000)
        .unwrap();
    punch.set_program_counter(0o2000).unwrap();
    punch.step().unwrap();
    assert_eq!(punch.get_state().pc, 0o2001);

    let mut tape = Simulator::new(4096).unwrap();
    tape.queue_paper_tape_frames(&[PaperTapeFrame {
        data: 0o21,
        parity_error: true,
    }])
    .unwrap();
    tape.load_words(
        &[
            assemble_fixed("RON").unwrap(),
            assemble_fixed("RPT").unwrap(),
            assemble_fixed("BPE").unwrap(),
        ],
        0o2000,
    )
    .unwrap();
    tape.set_program_counter(0o2000).unwrap();
    tape.run(3).unwrap();
    assert_eq!(tape.get_state().pc, 0o2003);
    assert!(!tape.get_state().parity_error);

    let mut stopping = Simulator::new(4096).unwrap();
    stopping.set_stop_on_parity_alarm(true);
    stopping
        .queue_paper_tape_frames(&[PaperTapeFrame {
            data: 0o22,
            parity_error: true,
        }])
        .unwrap();
    stopping
        .load_words(
            &[
                assemble_fixed("RON").unwrap(),
                assemble_fixed("RPT").unwrap(),
            ],
            0o2000,
        )
        .unwrap();
    stopping.set_program_counter(0o2000).unwrap();
    stopping.run(2).unwrap();
    let state = stopping.get_state();
    assert!(state.halted);
    assert!(state.parity_error);
    assert!(state.priority_alarm);
    assert!(!state.paper_tape_reader_running);
}
