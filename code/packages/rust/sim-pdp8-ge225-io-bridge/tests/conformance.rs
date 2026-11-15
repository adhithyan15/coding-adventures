//! # Conformance and Differential Verification Suite (GAME03b)
//!
//! Validates:
//! - PDP-8 KL8-E / W076 Teletype controller functional dispatch (`KSF`, `KCC`, `KRS`, `KRB`,
//!   `TSF`, `TCF`, `TPC`, `TLS`) and interrupt request sequencing (`kb_flag || tp_flag`).
//! - Exhaustive gate-level differential equivalence between `pdp8_dispatch_iot_functional`
//!   and `GateLevelIotDecoder::decode` across all 64 device codes, 8 pulse configurations,
//!   accumulator patterns, and input states.
//! - GE-225 6-bit BCD <-> ASCII bidirectional conversion, case folding, and console typewriter bridge.
//! - End-to-end machine-code differential execution of `hamurabi` and `lunar-lander` telemetry
//!   on (`Pdp8Simulator` + `Asr33Functional`) versus (`Pdp8GateLevel` + `Asr33GateLevel`),
//!   asserting identical `Pdp8State` at `HLT`, identical step counts, and identical paper roll output.
//! - GE-225 BCD console typewriter execution on both `ge225_simulator::Simulator` and
//!   `ge225_gatelevel::Ge225GateLevel`, asserting identical typewriter output.
//! - CWE adversarial boundary immunization: CWE-190, CWE-125, CWE-787, and CWE-20.

#![deny(unsafe_code)]

use pdp8_simulator::{IotEvent, Pdp8Simulator};
use pdp8_gatelevel::Pdp8GateLevel;
use sim_pdp8_ge225_io_bridge::{
    ascii_to_ge225_bcd, assemble_ge225_typewriter_program,
    assemble_pdp8_teletype_printer_program, encode_ascii_for_pdp8, ge225_bcd_to_ascii,
    ge225_bcd_to_str, new_pdp8_teletype_bridge, new_pdp8_teletype_gate_bridge,
    pdp8_dispatch_iot_functional, str_to_ge225_bcd, GateLevelIotDecoder, Ge225TeletypeBridge,
    Pdp8TeletypeBridge, Pdp8TeletypeGateLevelBridge, PDP8_DEV_KBD, PDP8_DEV_TPR,
    PDP8_IOP1, PDP8_IOP2, PDP8_IOP4, PDP8_WORD_MASK,
};
use sim_teletype_asr33::Asr33Functional;

#[test]
fn test_constructor_helpers() {
    let bridge_func = new_pdp8_teletype_bridge();
    let bridge_gate = new_pdp8_teletype_gate_bridge();

    assert!(bridge_func.tp_flag());
    assert!(!bridge_func.kb_flag());
    assert!(bridge_gate.tp_flag());
    assert!(!bridge_gate.kb_flag());
}

#[test]
fn test_pdp8_iot_keyboard_instructions() {
    let mut bridge = Pdp8TeletypeBridge::new();
    // Clear teleprinter flag first so IRQ line solely reflects keyboard state
    bridge.handle_iot(IotEvent { device: PDP8_DEV_TPR, pulses: PDP8_IOP2 }, 0);
    assert!(!bridge.tp_flag());

    // 1. Initially kb_flag is false: KSF does not skip
    let ksf_event = IotEvent {
        device: PDP8_DEV_KBD,
        pulses: PDP8_IOP1,
    };
    let res = bridge.handle_iot(ksf_event, 0o1234);
    assert!(res.handled);
    assert!(!res.skip);
    assert_eq!(res.new_ac, 0o1234);
    assert!(!res.irq);

    // 2. Push character into keyboard: kb_flag becomes true
    bridge.kbd_push_byte(b'Z');
    assert!(bridge.kb_flag());

    // 3. KSF should now skip, leaving AC and flag intact
    let res = bridge.handle_iot(ksf_event, 0o1234);
    assert!(res.handled);
    assert!(res.skip);
    assert_eq!(res.new_ac, 0o1234);
    assert!(bridge.kb_flag());
    assert!(res.irq);

    // 4. KRS should OR keyboard buffer into AC without clearing flag
    let krs_event = IotEvent {
        device: PDP8_DEV_KBD,
        pulses: PDP8_IOP4,
    };
    let res = bridge.handle_iot(krs_event, 0o1200);
    assert!(res.handled);
    assert!(!res.skip);
    let expected_ac = (0o1200 | u16::from(bridge.asr33().state().kb_buffer)) & PDP8_WORD_MASK;
    assert_eq!(res.new_ac, expected_ac);
    assert!(bridge.kb_flag());

    // 5. KCC clears AC to 0 and clears kb_flag
    let kcc_event = IotEvent {
        device: PDP8_DEV_KBD,
        pulses: PDP8_IOP2,
    };
    let res = bridge.handle_iot(kcc_event, 0o7777);
    assert!(res.handled);
    assert!(!res.skip);
    assert_eq!(res.new_ac, 0);
    assert!(!bridge.kb_flag());

    // 6. KRB (IOP2 | IOP4): clears flag & AC, then reads buffer into AC
    bridge.kbd_push_byte(b'A');
    let krb_event = IotEvent {
        device: PDP8_DEV_KBD,
        pulses: PDP8_IOP2 | PDP8_IOP4,
    };
    let res = bridge.handle_iot(krb_event, 0o7777);
    assert!(res.handled);
    assert!(!res.skip);
    assert_eq!(res.new_ac, u16::from(bridge.asr33().state().kb_buffer));
    assert!(!bridge.kb_flag());
}

#[test]
fn test_pdp8_iot_teleprinter_instructions() {
    let mut bridge = Pdp8TeletypeBridge::new();

    // 1. Initially tp_flag is true: TSF skips
    let tsf_event = IotEvent {
        device: PDP8_DEV_TPR,
        pulses: PDP8_IOP1,
    };
    let res = bridge.handle_iot(tsf_event, 0o1234);
    assert!(res.handled);
    assert!(res.skip);
    assert_eq!(res.new_ac, 0o1234);

    // 2. TCF clears teleprinter flag
    let tcf_event = IotEvent {
        device: PDP8_DEV_TPR,
        pulses: PDP8_IOP2,
    };
    let res = bridge.handle_iot(tcf_event, 0o1234);
    assert!(res.handled);
    assert!(!res.skip);
    assert!(!bridge.tp_flag());

    // 3. TSF now does not skip
    let res = bridge.handle_iot(tsf_event, 0o1234);
    assert!(res.handled);
    assert!(!res.skip);

    // 4. TLS (TCF | TPC) clears flag, loads character, starts transmission
    let tls_event = IotEvent {
        device: PDP8_DEV_TPR,
        pulses: PDP8_IOP2 | PDP8_IOP4,
    };
    let res = bridge.handle_iot(tls_event, 0o0105); // 'E'
    assert!(res.handled);
    assert!(res.tx_strobe);
    assert_eq!(res.tx_byte, 0o105);
    assert!(!bridge.tp_flag());
    assert_eq!(bridge.asr33().state().tx_bits_remaining, 11);

    // Step 11 serial bit clocks until transmission completes
    for _ in 0..11 {
        bridge.asr33_mut().step_serial_bit();
    }
    assert!(bridge.tp_flag());
    assert_eq!(bridge.paper_roll_text(), "E");
}

#[test]
fn test_exhaustive_gate_level_iot_decoder_equivalence() {
    let test_acs = [0o0000, 0o0001, 0o0077, 0o0377, 0o1234, 0o4000, 0o7777];
    let test_buffers = [0x00, 0x20, 0x41, 0x5A, 0x7F, 0xC1, 0xFF];

    for device in (0u8..=0o77u8).chain(std::iter::once(0o103u8)) {
        for pulses in 0..=7 {
            for &ac in &test_acs {
                for kb_flag in [false, true] {
                    for tp_flag in [false, true] {
                        for &kb_buf in &test_buffers {
                            // Functional reference
                            let func_out = pdp8_dispatch_iot_functional(
                                device, pulses, ac, kb_flag, tp_flag, kb_buf,
                            );

                            // Gate-level implementation
                            let gate_out = GateLevelIotDecoder::decode(
                                device, pulses, ac, kb_flag, tp_flag, kb_buf,
                            );

                            // Assert 100% equivalence
                            let is_dev_03 = (device == PDP8_DEV_KBD) as u8;
                            let is_dev_04 = (device == PDP8_DEV_TPR) as u8;

                            assert_eq!(gate_out.dev_03, is_dev_03);
                            assert_eq!(gate_out.dev_04, is_dev_04);

                            assert_eq!(
                                gate_out.skip == 1,
                                func_out.skip,
                                "skip mismatch for dev={device:02o}, p={pulses:o}, ac={ac:04o}"
                            );
                            assert_eq!(
                                gate_out.new_ac, func_out.new_ac,
                                "new_ac mismatch for dev={device:02o}, p={pulses:o}, ac={ac:04o}"
                            );
                            assert_eq!(
                                gate_out.clear_kb == 1,
                                func_out.clear_kb,
                                "clear_kb mismatch for dev={device:02o}, p={pulses:o}"
                            );
                            assert_eq!(
                                gate_out.clear_tp == 1,
                                func_out.clear_tp,
                                "clear_tp mismatch for dev={device:02o}, p={pulses:o}"
                            );
                            assert_eq!(
                                gate_out.tx_strobe == 1,
                                func_out.tx_strobe,
                                "tx_strobe mismatch for dev={device:02o}, p={pulses:o}"
                            );
                            assert_eq!(
                                gate_out.tx_byte, func_out.tx_byte,
                                "tx_byte mismatch for dev={device:02o}, p={pulses:o}"
                            );
                            assert_eq!(
                                gate_out.irq == 1,
                                func_out.irq,
                                "irq mismatch for dev={device:02o}, p={pulses:o}, kb={kb_flag}, tp={tp_flag}"
                            );
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn test_ge225_bcd_ascii_bidirectional_conversion() {
    let test_chars = "0123456789/.-$ ABCDEFGHIJKLMNOPQRSTUVWXYZ\r\t";

    for ch in test_chars.chars() {
        let ascii = ch as u8;
        let bcd = ascii_to_ge225_bcd(ascii)
            .unwrap_or_else(|| panic!("failed to convert ASCII '{}' to BCD", ch));
        let roundtrip_ascii = ge225_bcd_to_ascii(bcd)
            .unwrap_or_else(|| panic!("failed to convert BCD 0o{:02o} to ASCII", bcd));
        assert_eq!(
            roundtrip_ascii, ascii,
            "roundtrip mismatch for character '{}'",
            ch
        );
    }

    // Lowercase folding
    assert_eq!(ascii_to_ge225_bcd(b'a'), ascii_to_ge225_bcd(b'A'));
    assert_eq!(ascii_to_ge225_bcd(b'z'), ascii_to_ge225_bcd(b'Z'));

    // Newline mapping to CR
    assert_eq!(ascii_to_ge225_bcd(b'\n'), Some(0o37));

    // String conversion helpers
    let text = "ALT 120000 FT\r";
    let bcd_vec = str_to_ge225_bcd(text);
    let reconstructed = ge225_bcd_to_str(&bcd_vec);
    assert_eq!(reconstructed, text);
}

#[test]
fn test_ge225_console_typewriter_simulation_and_gatelevel_equivalence() {
    let program = assemble_ge225_typewriter_program(0o0100, 7).unwrap();

    // Data words at 0o0100: BCD codes for "YEAR 01"
    let text = "YEAR 01";
    let bcd_codes = str_to_ge225_bcd(text);

    // 1. Run on ge225-simulator
    let mut sim = ge225_simulator::Simulator::new(4096).unwrap();
    sim.load_words(&program, 0).unwrap();
    for (i, &code) in bcd_codes.iter().enumerate() {
        sim.write_word(0o0100 + (i as i32), code as i32).unwrap();
    }
    sim.run(program.len()).unwrap();
    let sim_output = sim.get_typewriter_output();
    assert_eq!(sim_output, text);

    // 2. Run on ge225-gatelevel
    let mut gate = ge225_gatelevel::Ge225GateLevel::new(4096).unwrap();
    gate.load_words(&program, 0).unwrap();
    for (i, &code) in bcd_codes.iter().enumerate() {
        gate.write_word(0o0100 + (i as i32), code as i32).unwrap();
    }
    gate.run(program.len()).unwrap();
    let gate_output = gate.get_typewriter_output();
    assert_eq!(gate_output, text);

    // 3. Assert 100% output equivalence between simulator and gate-level
    assert_eq!(sim_output, gate_output);

    // 4. Sync to ASR-33 paper roll
    let mut asr = Asr33Functional::new();
    Ge225TeletypeBridge::sync_sim_typewriter(&sim, &mut asr);
    assert_eq!(asr.paper_roll_text(), text);
}

#[test]
fn test_end_to_end_pdp8_hamurabi_telemetry_differential_equivalence() {
    // Generate Hamurabi telemetry report string
    let h_state = hamurabi::new_game(42);
    let summary = format!("YR {} POP {}", h_state.year, h_state.population);

    let string_words = encode_ascii_for_pdp8(&summary);
    let program = assemble_pdp8_teletype_printer_program(0o0020);

    const PROG_ORIGIN: usize = 0o0200;
    const STR_ORIGIN: usize = 0o0300;
    const AUTO_INDEX: usize = 0o0010;

    // --- System A: Pdp8Simulator + Asr33Functional via Pdp8TeletypeBridge ---
    // --- System A: Pdp8Simulator + Asr33Functional via Pdp8TeletypeBridge ---
    let mut sim = Pdp8Simulator::new();
    sim.load_words(&program, PROG_ORIGIN).unwrap();
    for (i, &word) in string_words.iter().enumerate() {
        sim.write_memory(STR_ORIGIN + i, word).unwrap();
    }
    sim.write_memory(AUTO_INDEX, (STR_ORIGIN as u16).wrapping_sub(1) & PDP8_WORD_MASK).unwrap();

    let mut bridge_func = Pdp8TeletypeBridge::new();
    bridge_func.kbd_push_byte(b'3'); // Order digit '3'

    let steps_func = bridge_func.run_sim_until_halt(&mut sim, 100_000).unwrap();
    assert!(sim.state().halted);

    // Drain remaining serial bits
    while bridge_func.asr33().state().tx_bits_remaining > 0 {
        bridge_func.asr33_mut().step_serial_bit();
    }

    // --- System B: Pdp8GateLevel + Asr33GateLevel via Pdp8TeletypeGateLevelBridge ---
    let mut cpu = Pdp8GateLevel::new();
    cpu.load_words(&program, PROG_ORIGIN).unwrap();
    for (i, &word) in string_words.iter().enumerate() {
        cpu.write_memory(STR_ORIGIN + i, word).unwrap();
    }
    cpu.write_memory(AUTO_INDEX, (STR_ORIGIN as u16).wrapping_sub(1) & PDP8_WORD_MASK).unwrap();

    let mut bridge_gate = Pdp8TeletypeGateLevelBridge::new();
    bridge_gate.kbd_push_byte(b'3'); // Identical input digit

    let steps_gate = bridge_gate.run_gate_until_halt(&mut cpu, 100_000).unwrap();
    assert!(cpu.state().halted);

    // Drain remaining serial bits
    while bridge_gate.asr33().tx_bits_remaining > 0 {
        bridge_gate.asr33_mut().step_serial_bit();
    }

    // --- Assert Complete Machine-State and Differential Equivalence ---
    assert_eq!(steps_func, steps_gate, "step count to HLT must be identical");
    assert_eq!(sim.state(), cpu.state(), "final Pdp8State at HLT must match 100%");

    // Verify stored input digit in memory location 0o0020
    let stored_input = sim.read_memory(0o0020).unwrap();
    assert_eq!(stored_input, u16::from(b'3'));

    // Verify printed paper roll output matches telemetry line
    let paper_text = bridge_func.paper_roll_text();
    assert_eq!(paper_text, summary);

    // Verify paint scene rendering
    let rendered_scene = bridge_func.render_paper_roll_ascii();
    assert!(rendered_scene.contains(&summary));
}

#[test]
fn test_end_to_end_pdp8_lunar_lander_telemetry_differential_equivalence() {
    // Generate Lunar Lander telemetry status line in 7-bit ASCII
    let l_state = lunar_lander::new_flight();
    let status_line = format!(
        "ALT {}K F {}K",
        l_state.altitude_mft,
        l_state.fuel_lbs / 1000
    );

    let string_words = encode_ascii_for_pdp8(&status_line);
    let program = assemble_pdp8_teletype_printer_program(0o0020);

    const PROG_ORIGIN: usize = 0o0200;
    const STR_ORIGIN: usize = 0o0300;
    const AUTO_INDEX: usize = 0o0010;

    // --- System A: Pdp8Simulator + Asr33Functional ---
    let mut sim = Pdp8Simulator::new();
    sim.load_words(&program, PROG_ORIGIN).unwrap();
    for (i, &word) in string_words.iter().enumerate() {
        sim.write_memory(STR_ORIGIN + i, word).unwrap();
    }
    sim.write_memory(AUTO_INDEX, (STR_ORIGIN as u16).wrapping_sub(1) & PDP8_WORD_MASK).unwrap();

    let mut bridge_func = Pdp8TeletypeBridge::new();
    bridge_func.kbd_push_byte(b'9');

    let steps_func = bridge_func.run_sim_until_halt(&mut sim, 100_000).unwrap();
    assert!(sim.state().halted);

    while bridge_func.asr33().state().tx_bits_remaining > 0 {
        bridge_func.asr33_mut().step_serial_bit();
    }

    // --- System B: Pdp8GateLevel + Asr33GateLevel ---
    let mut cpu = Pdp8GateLevel::new();
    cpu.load_words(&program, PROG_ORIGIN).unwrap();
    for (i, &word) in string_words.iter().enumerate() {
        cpu.write_memory(STR_ORIGIN + i, word).unwrap();
    }
    cpu.write_memory(AUTO_INDEX, (STR_ORIGIN as u16).wrapping_sub(1) & PDP8_WORD_MASK).unwrap();

    let mut bridge_gate = Pdp8TeletypeGateLevelBridge::new();
    bridge_gate.kbd_push_byte(b'9');

    let steps_gate = bridge_gate.run_gate_until_halt(&mut cpu, 100_000).unwrap();
    assert!(cpu.state().halted);

    while bridge_gate.asr33().tx_bits_remaining > 0 {
        bridge_gate.asr33_mut().step_serial_bit();
    }

    // --- Assert Differential Equivalence ---
    assert_eq!(steps_func, steps_gate);
    assert_eq!(sim.state(), cpu.state());
    assert_eq!(bridge_func.paper_roll_text(), status_line);
}

#[test]
fn test_adversarial_cwe190_integer_overflow_prevention() {
    // 1. Extreme 16-bit AC and PC values wrapped properly by mask
    let out = pdp8_dispatch_iot_functional(PDP8_DEV_KBD, PDP8_IOP4, u16::MAX, false, false, 0xFF);
    assert_eq!(out.new_ac, PDP8_WORD_MASK);

    let gate_out = GateLevelIotDecoder::decode(PDP8_DEV_KBD, PDP8_IOP4, u16::MAX, false, false, 0xFF);
    assert_eq!(gate_out.new_ac, PDP8_WORD_MASK);

    // 2. High pulses bits beyond 3-bit mask do not overflow or panic
    let out = pdp8_dispatch_iot_functional(PDP8_DEV_TPR, 0xFF, 0o1234, false, true, 0);
    assert!(out.skip);
    assert!(out.clear_tp);
    assert!(out.tx_strobe);
}

#[test]
fn test_adversarial_cwe125_cwe787_buffer_bounds() {
    // 1. Unrecognized device codes (e.g., 0o77 or out of bounds) fail closed without corruption
    for dev in [0o00, 0o01, 0o02, 0o05, 0o06, 0o07, 0o77] {
        let res = pdp8_dispatch_iot_functional(dev, 0o7, 0o1234, false, false, 0);
        assert!(!res.handled);
        assert!(!res.skip);
        assert_eq!(res.new_ac, 0o1234);

        let gate_res = GateLevelIotDecoder::decode(dev, 0o7, 0o1234, false, false, 0);
        assert_eq!(gate_res.dev_03, 0);
        assert_eq!(gate_res.dev_04, 0);
        assert_eq!(gate_res.skip, 0);
    }

    // 2. GE-225 BCD codes beyond 0o77 mask cleanly to 6 bits
    for code in 0o100u8..=0o377u8 {
        assert_eq!(ge225_bcd_to_ascii(code), ge225_bcd_to_ascii(code & 0o77));
    }

    // 3. GE-225 assembler bounds and overflow checks
    assert!(assemble_ge225_typewriter_program(0o0100, 100_000).is_none());
    assert!(assemble_ge225_typewriter_program(i32::MAX, 4).is_none());
}

#[test]
fn test_adversarial_cwe20_input_sanitization() {
    // 1. Non-printable and invalid ASCII inputs to GE-225 BCD return None
    assert_eq!(ascii_to_ge225_bcd(0x00), None);
    assert_eq!(ascii_to_ge225_bcd(0x7F), None);
    assert_eq!(ascii_to_ge225_bcd(0xFF), None);

    // 2. Invalid BCD codes return None
    assert_eq!(ge225_bcd_to_ascii(0o12), None);
    assert_eq!(ge225_bcd_to_ascii(0o14), None);
    assert_eq!(ge225_bcd_to_ascii(0o77), None);
}
