use super::*;
use std::collections::BTreeMap;

const C: u8 = 0x01;
const DC: u8 = 0x02;
const Z: u8 = 0x04;
const PD: u8 = 0x08;
const TO: u8 = 0x10;
const RP0: u8 = 0x20;

fn byte(base: u16, destination_file: bool, file: u8) -> u16 {
    base | (u16::from(destination_file) << 7) | u16::from(file & 0x7f)
}

fn bit(base: u16, selected_bit: u8, file: u8) -> u16 {
    base | (u16::from(selected_bit & 7) << 7) | u16::from(file & 0x7f)
}

fn installed(words: &[u16], origin: u16) -> Pic16f84aSimulator {
    let mut cpu = Pic16f84aSimulator::new();
    cpu.load(words, origin).unwrap();
    cpu
}

fn one_word_state(raw: u16) -> Pic16f84aState {
    installed(&[raw], 0).snapshot()
}

fn is_peripheral(address: u8) -> bool {
    matches!(
        address,
        0x01 | 0x05 | 0x06 | 0x08 | 0x09 | 0x81 | 0x85 | 0x86 | 0x88 | 0x89
    )
}

fn gpr_index(address: u8) -> Option<usize> {
    let low = address & 0x7f;
    (0x0c..=0x4f)
        .contains(&low)
        .then(|| usize::from(low - 0x0c))
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum OracleDecode {
    Instruction(Instruction),
    Legacy,
    Unknown,
}

fn decode_oracle(raw: u16) -> OracleDecode {
    use Instruction::*;
    let exact = [
        (0x0001, Halt),
        (0x0008, Return),
        (0x0009, Retfie),
        (0x0063, Sleep),
        (0x0064, Clrwdt),
    ];
    if let Some((_, instruction)) = exact.iter().find(|(word, _)| *word == raw) {
        return OracleDecode::Instruction(*instruction);
    }
    if matches!(raw, 0x0062 | 0x0065..=0x0067) {
        return OracleDecode::Legacy;
    }
    let patterns = [
        (0x3f00, 0x0700, Addwf),
        (0x3f00, 0x0500, Andwf),
        (0x3f00, 0x0900, Comf),
        (0x3f00, 0x0300, Decf),
        (0x3f00, 0x0b00, Decfsz),
        (0x3f00, 0x0a00, Incf),
        (0x3f00, 0x0f00, Incfsz),
        (0x3f00, 0x0400, Iorwf),
        (0x3f00, 0x0800, Movf),
        (0x3f00, 0x0d00, Rlf),
        (0x3f00, 0x0c00, Rrf),
        (0x3f00, 0x0200, Subwf),
        (0x3f00, 0x0e00, Swapf),
        (0x3f00, 0x0600, Xorwf),
        (0x3f80, 0x0180, Clrf),
        (0x3f80, 0x0100, Clrw),
        (0x3f80, 0x0080, Movwf),
        (0x3f9f, 0x0000, Nop),
        (0x3c00, 0x1000, Bcf),
        (0x3c00, 0x1400, Bsf),
        (0x3c00, 0x1800, Btfsc),
        (0x3c00, 0x1c00, Btfss),
        (0x3e00, 0x3e00, Addlw),
        (0x3f00, 0x3900, Andlw),
        (0x3f00, 0x3800, Iorlw),
        (0x3c00, 0x3000, Movlw),
        (0x3e00, 0x3c00, Sublw),
        (0x3f00, 0x3a00, Xorlw),
        (0x3c00, 0x3400, Retlw),
        (0x3800, 0x2000, Call),
        (0x3800, 0x2800, Goto),
    ];
    patterns
        .iter()
        .find_map(|&(mask, value, instruction)| {
            (raw & mask == value).then_some(OracleDecode::Instruction(instruction))
        })
        .unwrap_or(OracleDecode::Unknown)
}

#[test]
fn exhaustive_decoder_matches_independent_table() {
    let mut counts = BTreeMap::<&'static str, usize>::new();
    let mut architectural = 0;
    let mut legacy = 0;
    let mut unknown = 0;
    for raw in 0..=0x3fff {
        let actual = classify(raw);
        match decode_oracle(raw) {
            OracleDecode::Instruction(expected) => {
                assert_eq!(actual, Ok(expected), "raw {raw:#06x}");
                *counts.entry(expected.mnemonic()).or_default() += 1;
                if expected != Instruction::Halt {
                    architectural += 1;
                }
            }
            OracleDecode::Legacy => {
                assert!(matches!(
                    actual,
                    Err(Pic16f84aError::UnsupportedLegacyInstruction { raw: got, .. }) if got == raw
                ));
                legacy += 1;
            }
            OracleDecode::Unknown => {
                assert!(matches!(
                    actual,
                    Err(Pic16f84aError::UnknownInstruction { raw: got, .. }) if got == raw
                ));
                unknown += 1;
            }
        }
    }
    assert_eq!(architectural, 16_008);
    assert_eq!(legacy, 4);
    assert_eq!(unknown, 371);
    assert_eq!(counts.len(), 36);
    assert_eq!(counts["HALT"], 1);
    assert_eq!(counts["NOP"], 4);
    assert_eq!(counts["MOVLW"], 1_024);
    assert!(matches!(
        classify(0x4000),
        Err(Pic16f84aError::UnknownInstruction { raw: 0x4000, .. })
    ));
}

#[test]
fn direct_file_write_matrix_covers_banks_mirrors_gprs_and_boundaries() {
    for bank in 0..=1_u8 {
        for file in 0..=0x7f_u8 {
            let mut cpu = installed(&[byte(0x0080, false, file)], 0);
            let mut state = cpu.snapshot();
            state.w = 0xa5;
            state.status = TO | PD | (bank * RP0);
            state.fsr = 0x4f;
            state.pclath = 0x1b;
            cpu.restore(state.clone()).unwrap();
            let effective = if file == 0 {
                0x4f
            } else {
                (bank * 0x80) | file
            };
            let result = cpu.step();
            if is_peripheral(effective) {
                assert!(matches!(
                    result,
                    Err(Pic16f84aError::UnsupportedPeripheralRegister { address }) if address == effective
                ));
                assert_eq!(cpu.snapshot(), state);
                continue;
            }
            let trace = result.unwrap();
            assert_eq!(trace.file_address, Some(effective));
            assert_eq!(trace.indirect, file == 0);
            let after = cpu.snapshot();
            if let Some(index) = gpr_index(effective) {
                assert_eq!(after.gpr[index], 0xa5);
            } else {
                match effective & 0x7f {
                    0x02 => {
                        assert_eq!(after.pc, 0x1ba5);
                        assert_eq!(trace.cycles, 2);
                        assert!(trace.pipeline_discarded && trace.branch_taken);
                    }
                    0x03 => assert_eq!(after.status, TO | PD | RP0 | Z | C),
                    0x04 => assert_eq!(after.fsr, 0xa5),
                    0x0a => assert_eq!(after.pclath, 0x05),
                    0x0b => assert_eq!(after.intcon, 0xa5),
                    _ => {}
                }
            }
        }
    }
}

fn expected_file_read(state: &Pic16f84aState, address: u8) -> u8 {
    if address == 0 || address == 0x80 {
        return 0;
    }
    if let Some(index) = gpr_index(address) {
        return state.gpr[index];
    }
    match address & 0x7f {
        0x02 => 1,
        0x03 => state.status,
        0x04 => state.fsr,
        0x0a => state.pclath,
        0x0b => state.intcon,
        _ => 0,
    }
}

#[test]
fn all_fsr_values_have_exact_indirect_read_and_write_semantics() {
    for address in 0..=u8::MAX {
        let mut read_cpu = installed(&[byte(0x0800, false, 0)], 0);
        let mut state = read_cpu.snapshot();
        state.status = TO | PD;
        state.fsr = address;
        state.pclath = 0x1d;
        state.intcon = 0x5a;
        for (index, value) in state.gpr.iter_mut().enumerate() {
            *value = index as u8 + 1;
        }
        read_cpu.restore(state.clone()).unwrap();
        let read = read_cpu.step();
        if is_peripheral(address) {
            assert!(matches!(
                read,
                Err(Pic16f84aError::UnsupportedPeripheralRegister { address: got }) if got == address
            ));
            assert_eq!(read_cpu.snapshot(), state);
        } else {
            let expected = expected_file_read(&state, address);
            let trace = read.unwrap();
            assert_eq!(trace.file_address, Some(address));
            assert!(trace.indirect);
            assert_eq!(read_cpu.snapshot().w, expected);
            assert_eq!(
                read_cpu.snapshot().status & Z,
                if expected == 0 { Z } else { 0 }
            );
        }

        let mut write_cpu = installed(&[byte(0x0080, false, 0)], 0);
        let mut state = write_cpu.snapshot();
        state.status = TO | PD;
        state.fsr = address;
        state.pclath = 0x1d;
        state.w = 0xa6;
        write_cpu.restore(state.clone()).unwrap();
        let write = write_cpu.step();
        if is_peripheral(address) {
            assert!(matches!(
                write,
                Err(Pic16f84aError::UnsupportedPeripheralRegister { address: got }) if got == address
            ));
            assert_eq!(write_cpu.snapshot(), state);
        } else {
            let trace = write.unwrap();
            assert_eq!(trace.file_address, Some(address));
            assert!(trace.indirect);
            let after = write_cpu.snapshot();
            if address == 0 || address == 0x80 {
                assert_eq!(after, {
                    let mut expected = state;
                    expected.pc = 1;
                    expected
                });
            } else if let Some(index) = gpr_index(address) {
                assert_eq!(after.gpr[index], 0xa6);
            } else {
                match address & 0x7f {
                    0x02 => assert_eq!(after.pc, 0x1da6),
                    0x03 => assert_eq!(after.status, TO | PD | RP0 | Z | DC),
                    0x04 => assert_eq!(after.fsr, 0xa6),
                    0x0a => assert_eq!(after.pclath, 0x06),
                    0x0b => assert_eq!(after.intcon, 0xa6),
                    _ => {}
                }
            }
        }
    }
}

#[test]
fn lifecycle_load_restore_reset_and_basic_trace_are_complete_and_atomic() {
    let mut cpu = Pic16f84aSimulator::new();
    let reset = cpu.snapshot();
    assert_eq!(reset.status, TO | PD);
    assert_eq!(cpu.load(&[], 0), Err(Pic16f84aError::EmptyProgram));
    assert_eq!(cpu.snapshot(), reset);
    assert!(matches!(
        cpu.load(&[0x4000], 0),
        Err(Pic16f84aError::ProgramWordOutOfRange { .. })
    ));
    assert!(matches!(
        cpu.load(&[0; 2], 0x03ff),
        Err(Pic16f84aError::ImageOutOfRange { .. })
    ));
    assert_eq!(cpu.snapshot(), reset);

    cpu.load(&[encoding::movlw(0x2a), encoding::halt()], 0x20)
        .unwrap();
    let first = cpu.step().unwrap();
    assert_eq!(first.pc_before, 0x20);
    assert_eq!(first.pc_after, 0x21);
    assert_eq!(first.fetch_index, 0x20);
    assert_eq!(first.raw_word, encoding::movlw(0x2a));
    assert_eq!(first.mnemonic, "MOVLW");
    assert_eq!(first.cycles, 1);
    assert!(!first.pipeline_discarded && !first.skip_taken && !first.branch_taken);
    assert_eq!(first.file_address, None);
    assert_eq!(first.post_state, cpu.snapshot());
    let halt = cpu.step().unwrap();
    assert_eq!(halt.mnemonic, "HALT");
    assert!(halt.post_state.halted);
    let halted = cpu.snapshot();
    assert_eq!(cpu.step(), Err(Pic16f84aError::Halted));
    assert_eq!(cpu.snapshot(), halted);

    cpu.reset();
    assert_eq!(cpu.snapshot(), reset);
}

#[test]
fn restore_rejects_every_noncanonical_field_without_mutation() {
    let mut cpu = installed(&[encoding::halt()], 0);
    let entry = cpu.snapshot();
    let mut cases = Vec::new();
    let mut bad = entry.clone();
    bad.pc = 0x2000;
    cases.push(bad);
    let mut bad = entry.clone();
    bad.status = 0x40;
    cases.push(bad);
    let mut bad = entry.clone();
    bad.pclath = 0x20;
    cases.push(bad);
    let mut bad = entry.clone();
    bad.stack_pointer = 8;
    cases.push(bad);
    let mut bad = entry.clone();
    bad.return_stack[3] = 0x2000;
    cases.push(bad);
    let mut bad = entry.clone();
    bad.program[3] = 0x4000;
    cases.push(bad);
    let mut bad = entry.clone();
    bad.origin = None;
    cases.push(bad);
    let mut bad = entry.clone();
    bad.length = 0;
    cases.push(bad);
    let mut bad = entry.clone();
    bad.origin = Some(0x03ff);
    bad.length = 2;
    cases.push(bad);
    let mut bad = entry.clone();
    bad.sleeping = true;
    bad.halted = true;
    cases.push(bad);
    for bad in cases {
        assert!(matches!(
            cpu.restore(bad),
            Err(Pic16f84aError::InvalidState { .. })
        ));
        assert_eq!(cpu.snapshot(), entry);
    }
    cpu.restore(entry.clone()).unwrap();
    assert_eq!(cpu.snapshot(), entry);
}

fn arithmetic_state(raw: u16, w: u8, source: u8) -> Pic16f84aState {
    let mut state = one_word_state(raw);
    state.w = w;
    state.status = TO | PD | RP0 | C | DC | Z;
    state.gpr[0] = source;
    state
}

fn expected_add_flags(left: u8, right: u8, result: u8) -> u8 {
    (u8::from(u16::from(left) + u16::from(right) > 0xff) * C)
        | (u8::from((left & 0x0f) + (right & 0x0f) > 0x0f) * DC)
        | (u8::from(result == 0) * Z)
}

fn expected_sub_flags(left: u8, right: u8, result: u8) -> u8 {
    (u8::from(left >= right) * C)
        | (u8::from((left & 0x0f) >= (right & 0x0f)) * DC)
        | (u8::from(result == 0) * Z)
}

#[test]
fn exhaustive_add_sub_and_rotate_flags_use_independent_oracles() {
    for left in 0..=u8::MAX {
        for right in 0..=u8::MAX {
            for (raw, result, flags) in [
                (
                    0x3e00 | u16::from(right),
                    left.wrapping_add(right),
                    expected_add_flags(left, right, left.wrapping_add(right)),
                ),
                (
                    0x3c00 | u16::from(right),
                    right.wrapping_sub(left),
                    expected_sub_flags(right, left, right.wrapping_sub(left)),
                ),
            ] {
                let mut cpu = installed(&[raw], 0);
                let mut state = cpu.snapshot();
                state.w = left;
                state.status = TO | PD | RP0 | C | DC | Z;
                cpu.restore(state).unwrap();
                cpu.step().unwrap();
                let after = cpu.snapshot();
                assert_eq!(after.w, result, "raw={raw:#06x} W={left:#04x}");
                assert_eq!(after.status, TO | PD | RP0 | flags);
            }

            for (raw, result, flags) in [
                (
                    byte(0x0700, false, 0x0c),
                    left.wrapping_add(right),
                    expected_add_flags(left, right, left.wrapping_add(right)),
                ),
                (
                    byte(0x0200, false, 0x0c),
                    left.wrapping_sub(right),
                    expected_sub_flags(left, right, left.wrapping_sub(right)),
                ),
            ] {
                let mut cpu = installed(&[raw], 0);
                let state = arithmetic_state(raw, right, left);
                cpu.restore(state).unwrap();
                cpu.step().unwrap();
                let after = cpu.snapshot();
                assert_eq!(
                    after.w, result,
                    "raw={raw:#06x} f={left:#04x} W={right:#04x}"
                );
                assert_eq!(after.status, TO | PD | RP0 | flags);
            }
        }
    }

    for source in 0..=u8::MAX {
        for carry in 0..=1_u8 {
            for (base, expected, expected_carry) in [
                (0x0d00, (source << 1) | carry, source >> 7),
                (0x0c00, (source >> 1) | (carry << 7), source & 1),
            ] {
                let raw = byte(base, false, 0x0c);
                let mut cpu = installed(&[raw], 0);
                let mut state = cpu.snapshot();
                state.status = TO | PD | RP0 | DC | Z | carry;
                state.gpr[0] = source;
                cpu.restore(state).unwrap();
                cpu.step().unwrap();
                let after = cpu.snapshot();
                assert_eq!(after.w, expected);
                assert_eq!(after.status, TO | PD | RP0 | DC | Z | expected_carry);
            }
        }
    }
}

#[test]
fn status_destination_merges_write_inhibition_and_alu_flags() {
    let families = [
        0x0700, 0x0500, 0x0900, 0x0300, 0x0b00, 0x0a00, 0x0f00, 0x0400, 0x0800, 0x0d00, 0x0c00,
        0x0200, 0x0e00, 0x0600,
    ];
    for base in families {
        for source_status in 0..=0x3f_u8 {
            let raw = byte(base, true, 0x03);
            let mut cpu = installed(&[raw], 0);
            let mut state = cpu.snapshot();
            state.status = source_status;
            state.w = 0x5a;
            cpu.restore(state).unwrap();
            cpu.step().unwrap();
            let after = cpu.snapshot();
            assert_eq!(after.status & (TO | PD), source_status & (TO | PD));
            assert_eq!(after.status & 0xc0, 0);
        }
    }

    for raw in [0x0183, 0x0083, bit(0x1000, 0, 0x03), bit(0x1400, 5, 0x03)] {
        let mut cpu = installed(&[raw], 0);
        let mut state = cpu.snapshot();
        state.status = TO | PD | RP0 | C | DC;
        state.w = 0xff;
        cpu.restore(state.clone()).unwrap();
        cpu.step().unwrap();
        let after = cpu.snapshot();
        assert_eq!(after.status & (TO | PD), state.status & (TO | PD));
        assert_eq!(after.status & 0xc0, 0);
    }

    let mut cpu = installed(&[byte(0x0700, true, 0x03)], 0);
    let mut state = cpu.snapshot();
    state.status = TO | PD | RP0 | C;
    state.w = 0xff;
    cpu.restore(state).unwrap();
    cpu.step().unwrap();
    assert_eq!(cpu.snapshot().status, TO | PD | RP0 | C | DC);
}

fn full_program(word: u16) -> Pic16f84aState {
    let mut cpu = Pic16f84aSimulator::new();
    cpu.load(&vec![word; PROGRAM_WORDS], 0).unwrap();
    cpu.snapshot()
}

#[test]
fn every_pcl_writer_uses_fallthrough_and_all_pclath_bits() {
    let cases: [(u16, u8, u8, u8); 18] = [
        (byte(0x0700, true, 0x02), 3, C, 4),
        (byte(0x0500, true, 0x02), 3, C, 1),
        (byte(0x0900, true, 0x02), 3, C, 0xfe),
        (byte(0x0300, true, 0x02), 3, C, 0),
        (byte(0x0b00, true, 0x02), 3, C, 0),
        (byte(0x0a00, true, 0x02), 3, C, 2),
        (byte(0x0f00, true, 0x02), 3, C, 2),
        (byte(0x0400, true, 0x02), 3, C, 3),
        (byte(0x0800, true, 0x02), 3, C, 1),
        (byte(0x0d00, true, 0x02), 3, C, 3),
        (byte(0x0c00, true, 0x02), 3, C, 0x80),
        (byte(0x0200, true, 0x02), 3, C, 0xfe),
        (byte(0x0e00, true, 0x02), 3, C, 0x10),
        (byte(0x0600, true, 0x02), 3, C, 2),
        (0x0182, 3, C, 0),
        (0x0082, 3, C, 3),
        (bit(0x1000, 0, 0x02), 3, C, 0),
        (bit(0x1400, 7, 0x02), 3, C, 0x81),
    ];
    for pclath in 0..=0x1f_u8 {
        for &(raw, w, status, low) in &cases {
            let mut cpu = installed(&[raw], 0);
            let mut state = cpu.snapshot();
            state.w = w;
            state.status = TO | PD | status;
            state.pclath = pclath;
            cpu.restore(state).unwrap();
            let trace = cpu.step().unwrap();
            assert_eq!(trace.pc_after, u16::from(pclath) << 8 | u16::from(low));
            assert_eq!(trace.cycles, 2);
            assert!(trace.pipeline_discarded && trace.branch_taken);
        }
    }

    for &(raw, expected) in &[
        (byte(0x0800, false, 0x02), 0x00),
        (byte(0x0a00, false, 0x02), 0x01),
        (byte(0x0300, false, 0x02), 0xff),
    ] {
        let mut cpu = installed(&[raw], 0xff);
        cpu.step().unwrap();
        assert_eq!(cpu.snapshot().w, expected);
    }

    let mut cpu = Pic16f84aSimulator::new();
    cpu.load(&[encoding::halt()], 0).unwrap();
    let mut aliased = cpu.snapshot();
    aliased.pc = 0x0400;
    cpu.restore(aliased).unwrap();
    let trace = cpu.step().unwrap();
    assert_eq!(trace.fetch_index, 0);
    assert!(trace.post_state.halted);
}

#[test]
fn call_and_goto_targets_are_exhaustive_and_stack_fallthrough_is_exact() {
    for pclath in 0..=0x1f_u8 {
        for literal in 0..=0x07ff_u16 {
            let expected = (u16::from(pclath & 0x18) << 8) | literal;
            for call in [false, true] {
                let raw = if call {
                    0x2000 | literal
                } else {
                    0x2800 | literal
                };
                let mut cpu = installed(&[raw], 0);
                let mut state = cpu.snapshot();
                state.pclath = pclath;
                state.return_stack = [0x1555; 8];
                state.stack_pointer = 7;
                cpu.restore(state).unwrap();
                let trace = cpu.step().unwrap();
                let after = cpu.snapshot();
                assert_eq!(after.pc, expected);
                assert_eq!(trace.cycles, 2);
                assert!(trace.pipeline_discarded && trace.branch_taken);
                if call {
                    assert_eq!(after.return_stack[7], 1);
                    assert_eq!(after.stack_pointer, 0);
                } else {
                    assert_eq!(after.return_stack, [0x1555; 8]);
                    assert_eq!(after.stack_pointer, 7);
                }
            }
        }
    }
}

#[test]
fn circular_stack_depths_wraps_and_return_variants_are_exact() {
    for depth in [0_usize, 1, 7, 8, 9, 10, 16, 24] {
        let mut state = full_program(encoding::call(0));
        let mut cpu = Pic16f84aSimulator::new();
        for index in 0..depth {
            state.pc = (index as u16) & 0x03ff;
            cpu.restore(state).unwrap();
            cpu.step().unwrap();
            state = cpu.snapshot();
        }
        assert_eq!(state.stack_pointer, (depth & 7) as u8);
        for slot in 0..8 {
            let last = (slot..depth).step_by(8).next_back();
            assert_eq!(state.return_stack[slot], last.map_or(0, |i| i as u16 + 1));
        }

        for (raw, literal, gie) in [
            (0x0008, None, false),
            (0x3400 | 0xa5, Some(0xa5), false),
            (0x0009, None, true),
        ] {
            let mut pop_state = state.clone();
            pop_state.pc = 0;
            pop_state.program[0] = raw;
            pop_state.intcon = 0;
            let expected_pointer = pop_state.stack_pointer.wrapping_sub(1) & 7;
            let expected_pc = pop_state.return_stack[usize::from(expected_pointer)];
            cpu.restore(pop_state).unwrap();
            let trace = cpu.step().unwrap();
            let after = cpu.snapshot();
            assert_eq!(after.stack_pointer, expected_pointer);
            assert_eq!(after.pc, expected_pc);
            assert_eq!(after.intcon & 0x80 != 0, gie);
            if let Some(value) = literal {
                assert_eq!(after.w, value);
            }
            assert_eq!(trace.cycles, 2);
        }
    }
}

#[test]
fn skip_boundaries_and_pcl_overlap_have_exact_pipeline_behavior() {
    let skip_cases = [
        (byte(0x0b00, true, 0x0c), 1_u8, true),
        (byte(0x0b00, true, 0x0c), 2, false),
        (byte(0x0f00, true, 0x0c), 0xff, true),
        (byte(0x0f00, true, 0x0c), 0xfe, false),
        (bit(0x1800, 3, 0x0c), 0, true),
        (bit(0x1800, 3, 0x0c), 8, false),
        (bit(0x1c00, 3, 0x0c), 8, true),
        (bit(0x1c00, 3, 0x0c), 0, false),
    ];
    for pc in [0x00ff_u16, 0x03ff, 0x1fff] {
        for &(raw, source, taken) in &skip_cases {
            let mut state = full_program(0);
            state.pc = pc;
            state.program[usize::from(pc & 0x03ff)] = raw;
            state.gpr[0] = source;
            let mut cpu = Pic16f84aSimulator::new();
            cpu.restore(state).unwrap();
            let trace = cpu.step().unwrap();
            let fallthrough = pc.wrapping_add(1) & 0x1fff;
            assert_eq!(
                trace.pc_after,
                if taken {
                    fallthrough.wrapping_add(1) & 0x1fff
                } else {
                    fallthrough
                }
            );
            assert_eq!(trace.skip_taken, taken);
            assert_eq!(trace.pipeline_discarded, taken);
            assert_eq!(trace.cycles, if taken { 2 } else { 1 });
        }
    }

    for pclath in 0..=0x1f_u8 {
        for &(base, pc, taken) in &[
            (0x0b00, 0_u16, true),
            (0x0b00, 1, false),
            (0x0f00, 0xfe, true),
            (0x0f00, 0xfd, false),
        ] {
            let raw = byte(base, true, 0x02);
            let mut cpu = installed(&[raw], pc);
            let mut state = cpu.snapshot();
            state.pclath = pclath;
            cpu.restore(state).unwrap();
            let trace = cpu.step().unwrap();
            let source = pc.wrapping_add(1) as u8;
            let low = if base == 0x0b00 {
                source.wrapping_sub(1)
            } else {
                source.wrapping_add(1)
            };
            assert_eq!(trace.pc_after, u16::from(pclath) << 8 | u16::from(low));
            assert_eq!(trace.skip_taken, taken);
            assert_eq!(trace.cycles, 2);
        }
    }
}

#[test]
fn sleep_watchdog_halt_run_execute_and_faults_are_atomic() {
    let mut cpu = installed(&[0x0064, 0x0063], 0);
    let mut state = cpu.snapshot();
    state.status = C;
    cpu.restore(state).unwrap();
    cpu.step().unwrap();
    assert_eq!(cpu.snapshot().status, TO | PD | C);
    let sleep = cpu.step().unwrap();
    assert_eq!(sleep.mnemonic, "SLEEP");
    assert!(sleep.post_state.sleeping);
    assert_eq!(sleep.post_state.status, TO | C);
    let asleep = cpu.snapshot();
    assert_eq!(cpu.step(), Err(Pic16f84aError::Sleeping));
    assert_eq!(cpu.snapshot(), asleep);

    let original = installed(&[encoding::movlw(7)], 0x20).snapshot();
    for (program, max_steps) in [
        (vec![0x0000], 1),
        (vec![0x0063], 2),
        (vec![0x0002], 1),
        (vec![0x0062], 1),
        (vec![byte(0x0800, false, 0x05)], 1),
    ] {
        let mut cpu = Pic16f84aSimulator::new();
        cpu.restore(original.clone()).unwrap();
        assert!(cpu.execute(&program, 0, max_steps).is_err());
        assert_eq!(cpu.snapshot(), original);
    }

    let mut cpu = installed(&[encoding::movlw(1), 0x0000], 0);
    let before = cpu.snapshot();
    let error = cpu.run(2).unwrap_err();
    assert!(
        matches!(error, Pic16f84aError::ExecutionFailed { step: 2, source, .. } if matches!(*source, Pic16f84aError::StepLimitExceeded { limit: 2 }))
    );
    assert_eq!(cpu.snapshot(), before);
    assert!(matches!(
        cpu.run(MAX_RETAINED_STEPS + 1),
        Err(Pic16f84aError::TraceLimitExceeded { .. })
    ));
    assert_eq!(cpu.snapshot(), before);

    let result = cpu
        .execute(&[encoding::movlw(0x5a), encoding::halt()], 0, 2)
        .unwrap();
    assert!(result.halted);
    assert_eq!(result.steps, 2);
    assert_eq!(result.traces[0].mnemonic, "MOVLW");
    assert_eq!(result.traces[1].mnemonic, "HALT");
    assert_eq!(result.final_state, cpu.snapshot());
}

#[test]
fn remaining_families_and_exact_trace_contracts_are_pinned() {
    let program = [
        0x30f0, // MOVLW f0
        0x39aa, // ANDLW aa => a0
        0x3805, // IORLW 05 => a5
        0x3a0f, // XORLW 0f => aa
        0x0100, // CLRW
        0x0020, // NOP don't-care alias
        encoding::halt(),
    ];
    let mut cpu = installed(&program, 0);
    let result = cpu.run(program.len()).unwrap();
    assert_eq!(result.steps, program.len());
    assert_eq!(
        result.traces.iter().map(|t| t.mnemonic).collect::<Vec<_>>(),
        ["MOVLW", "ANDLW", "IORLW", "XORLW", "CLRW", "NOP", "HALT"]
    );
    assert_eq!(result.final_state.w, 0);
    assert_ne!(result.final_state.status & Z, 0);

    let mut cpu = installed(&[byte(0x0800, false, 0)], 0);
    let mut state = cpu.snapshot();
    state.fsr = 0x0c;
    state.gpr[0] = 0x42;
    cpu.restore(state).unwrap();
    let indirect = cpu.step().unwrap();
    assert_eq!(
        (indirect.pc_before, indirect.pc_after, indirect.fetch_index),
        (0, 1, 0)
    );
    assert_eq!(
        (indirect.raw_word, indirect.mnemonic, indirect.cycles),
        (0x0800, "MOVF", 1)
    );
    assert_eq!(
        (indirect.file_address, indirect.indirect),
        (Some(0x0c), true)
    );
    assert!(!indirect.pipeline_discarded && !indirect.skip_taken && !indirect.branch_taken);

    let mut cpu = installed(&[bit(0x1800, 0, 0x0c)], 0);
    let skip = cpu.step().unwrap();
    assert_eq!((skip.pc_after, skip.cycles), (2, 2));
    assert!(skip.pipeline_discarded && skip.skip_taken && !skip.branch_taken);

    let mut cpu = installed(&[byte(0x0700, true, 0x02)], 0);
    let mut state = cpu.snapshot();
    state.w = 4;
    state.pclath = 0x12;
    cpu.restore(state).unwrap();
    let computed = cpu.step().unwrap();
    assert_eq!((computed.pc_after, computed.cycles), (0x1205, 2));
    assert!(computed.pipeline_discarded && computed.branch_taken && !computed.skip_taken);

    let mut cpu = installed(&[encoding::call(0x345)], 0);
    let call = cpu.step().unwrap();
    assert_eq!(
        (call.raw_word, call.mnemonic, call.pc_after),
        (0x2345, "CALL", 0x345)
    );
    assert_eq!(call.post_state.return_stack[0], 1);

    let mut return_state = full_program(0x0008);
    return_state.return_stack[7] = 0x1234;
    return_state.stack_pointer = 0;
    cpu.restore(return_state).unwrap();
    let returned = cpu.step().unwrap();
    assert_eq!((returned.mnemonic, returned.pc_after), ("RETURN", 0x1234));
    assert!(returned.pipeline_discarded && returned.branch_taken);

    let sleep = installed(&[0x0063], 0).step().unwrap();
    assert_eq!((sleep.mnemonic, sleep.cycles), ("SLEEP", 1));
    assert!(sleep.post_state.sleeping && !sleep.post_state.halted);
    let halt = installed(&[encoding::halt()], 0).step().unwrap();
    assert_eq!((halt.mnemonic, halt.cycles), ("HALT", 1));
    assert!(halt.post_state.halted && !halt.post_state.sleeping);

    assert_eq!(encoding::movwf(0xff), 0x00ff);
    assert_eq!(encoding::addwf(0xff, true), 0x07ff);
    assert_eq!(encoding::goto(0xffff), 0x2fff);
}

#[test]
fn direct_step_faults_preserve_complete_state_and_errors_chain() {
    let cases = [0x0002, 0x0062, byte(0x0800, false, 0x05)];
    for raw in cases {
        let mut cpu = installed(&[raw], 0);
        let before = cpu.snapshot();
        assert!(cpu.step().is_err());
        assert_eq!(cpu.snapshot(), before);
    }

    let mut cpu = installed(&[0], 0);
    let mut outside = cpu.snapshot();
    outside.pc = 1;
    cpu.restore(outside.clone()).unwrap();
    assert!(matches!(
        cpu.step(),
        Err(Pic16f84aError::FetchOutsideProgram { pc: 1, physical: 1 })
    ));
    assert_eq!(cpu.snapshot(), outside);

    let error = cpu.run(1).unwrap_err();
    assert!(std::error::Error::source(&error).is_some());
    assert!(error.to_string().contains("execution failed at step 0"));
    let messages = [
        Pic16f84aError::EmptyProgram.to_string(),
        Pic16f84aError::ProgramWordOutOfRange {
            index: 2,
            word: 0x4000,
        }
        .to_string(),
        Pic16f84aError::ImageOutOfRange {
            origin: 0x03ff,
            length: 2,
        }
        .to_string(),
        Pic16f84aError::UnknownInstruction { raw: 2, pc: 0 }.to_string(),
        Pic16f84aError::UnsupportedLegacyInstruction { raw: 0x62, pc: 0 }.to_string(),
        Pic16f84aError::UnsupportedPeripheralRegister { address: 5 }.to_string(),
        Pic16f84aError::InvalidState { reason: "witness" }.to_string(),
        Pic16f84aError::Sleeping.to_string(),
        Pic16f84aError::Halted.to_string(),
        Pic16f84aError::TraceLimitExceeded {
            requested: 65,
            limit: 64,
        }
        .to_string(),
        Pic16f84aError::StepLimitExceeded { limit: 1 }.to_string(),
        Pic16f84aError::AllocationFailed.to_string(),
    ];
    assert!(messages.iter().all(|message| !message.is_empty()));
}
