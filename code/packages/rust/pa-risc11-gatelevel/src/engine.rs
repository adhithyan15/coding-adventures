//! Settled-decoder execution through gate-derived datapath and control networks.

use crate::decoder::{self, Kind};
use crate::gates;
use logic_gates::gates::{and_gate, not_gate, or_gate, xor_gate};
use pa_risc11_simulator::{PaRisc11Error, PaRisc11State, MEMORY_SIZE};

fn read_gpr(state: &PaRisc11State, index: u8) -> u32 {
    gates::register_mux(&state.gpr, index)
}

fn write_gpr(state: &mut PaRisc11State, index: u8, value: u32) {
    let enables = gates::register_enables(index, 1);
    for (register, enable) in enables.into_iter().enumerate() {
        state.gpr[register] = gates::mux32(state.gpr[register], value, enable);
    }
}

fn advance_queue(state: &mut PaRisc11State) {
    state.pc = state.next_pc;
    state.next_pc = gates::add32(state.next_pc, 4, 0).0;
    state.in_delay_slot = false;
}

fn data_range(address: u32, width: usize) -> Result<std::ops::Range<usize>, PaRisc11Error> {
    let width_mask = (width - 1) as u32;
    let aligned = gates::selected(gates::and32(address, width_mask), 0, 2);
    if width > 1 && aligned == 0 {
        return Err(PaRisc11Error::MisalignedData { address, width });
    }
    let (end, carry, _, _) = gates::add32(address, width as u32, 0);
    let address_high = gates::u32_bits(address)[16..]
        .iter()
        .copied()
        .fold(0, or_gate);
    let end_high = gates::u32_bits(end)[17..].iter().copied().fold(0, or_gate);
    let end_is_limit = gates::selected(end, MEMORY_SIZE as u32, 17);
    let valid_end = or_gate(not_gate(gates::u32_bits(end)[16]), end_is_limit);
    if or_gate(
        or_gate(carry, address_high),
        or_gate(end_high, not_gate(valid_end)),
    ) == 1
    {
        return Err(PaRisc11Error::DataOutOfRange { address, width });
    }
    Ok(address as usize..end as usize)
}

fn load_data(state: &PaRisc11State, address: u32, width: usize) -> Result<u32, PaRisc11Error> {
    let range = data_range(address, width)?;
    let mut value = 0;
    for byte in &state.memory[range] {
        value = gates::or32(gates::shift_left(value, 8), u32::from(*byte));
    }
    Ok(value)
}

fn store_data(
    state: &mut PaRisc11State,
    address: u32,
    width: usize,
    value: u32,
) -> Result<(), PaRisc11Error> {
    let range = data_range(address, width)?;
    for (offset, byte) in state.memory[range].iter_mut().enumerate() {
        let shift = (width - offset - 1) * 8;
        *byte = gates::shift_right(value, shift as u32) as u8;
    }
    Ok(())
}

fn install_conditional_branch(
    state: &mut PaRisc11State,
    taken: u8,
    displacement: u32,
    nullify: bool,
) {
    let old_pc = state.pc;
    let old_next = state.next_pc;
    let sequential = gates::add32(old_next, 4, 0).0;
    let target = gates::add32(gates::add32(old_pc, 8, 0).0, displacement, 0).0;
    state.pc = old_next;
    state.next_pc = gates::mux32(sequential, target, taken);
    let backward = gates::u32_bits(displacement)[31];
    let direction_outcome = or_gate(
        and_gate(not_gate(backward), taken),
        and_gate(backward, not_gate(taken)),
    );
    state.nullify_next = and_gate(u8::from(nullify), direction_outcome) == 1;
    state.in_delay_slot = true;
}

pub(crate) fn execute_word(
    state: &mut PaRisc11State,
    word: u32,
) -> Result<&'static str, PaRisc11Error> {
    let decoded = decoder::decode(word, state.pc, state.in_delay_slot)?;
    match decoded.kind {
        Kind::Break => {
            advance_queue(state);
            state.halted = true;
            Ok("BREAK")
        }
        Kind::Andcm | Kind::And | Kind::Or | Kind::Xor | Kind::Sub | Kind::Add => {
            let right = read_gpr(state, decoded.right);
            let left = read_gpr(state, decoded.left);
            let (mnemonic, value, carries) = match decoded.kind {
                Kind::Andcm => ("ANDCM", gates::and32(left, gates::not32(right)), None),
                Kind::And => ("AND", gates::and32(left, right), None),
                Kind::Or => ("OR", gates::or32(left, right), None),
                Kind::Xor => ("XOR", gates::xor32(left, right), None),
                Kind::Sub => {
                    let (value, _, _, carries) = gates::sub32(left, right);
                    ("SUB", value, Some(carries))
                }
                Kind::Add => {
                    let (value, _, _, carries) = gates::add32(left, right, 0);
                    ("ADD", value, Some(carries))
                }
                _ => unreachable!("settled ALU family"),
            };
            write_gpr(state, decoded.target, value);
            if let Some(carries) = carries {
                state.carry_borrow = carries.map(|line| line == 1);
            }
            advance_queue(state);
            Ok(mnemonic)
        }
        Kind::Ldil => {
            let value = gates::shift_left(gates::assemble21(word & 0x1f_ffff), 11);
            write_gpr(state, decoded.right, value);
            advance_queue(state);
            Ok("LDIL")
        }
        Kind::Addil => {
            let immediate = gates::shift_left(gates::assemble21(word & 0x1f_ffff), 11);
            let value = gates::add32(read_gpr(state, decoded.right), immediate, 0).0;
            write_gpr(state, 1, value);
            advance_queue(state);
            Ok("ADDIL")
        }
        Kind::Ldo => {
            let displacement = gates::sign_extend_low(word & 0x3fff, 14);
            let value = gates::add32(read_gpr(state, decoded.right), displacement, 0).0;
            write_gpr(state, decoded.left, value);
            advance_queue(state);
            Ok("LDO")
        }
        Kind::Load { width } => {
            let address = gates::add32(
                read_gpr(state, decoded.right),
                gates::sign_extend_low(word & 0x3fff, 14),
                0,
            )
            .0;
            let value = load_data(state, address, width)?;
            write_gpr(state, decoded.left, value);
            advance_queue(state);
            Ok(match width {
                1 => "LDB",
                2 => "LDH",
                _ => "LDW",
            })
        }
        Kind::Store { width } => {
            let address = gates::add32(
                read_gpr(state, decoded.right),
                gates::sign_extend_low(word & 0x3fff, 14),
                0,
            )
            .0;
            store_data(state, address, width, read_gpr(state, decoded.left))?;
            advance_queue(state);
            Ok(match width {
                1 => "STB",
                2 => "STH",
                _ => "STW",
            })
        }
        Kind::Shd => {
            let count = gates::and32(gates::not32((word >> 5) & 31), 31);
            let value = gates::shd(
                read_gpr(state, decoded.left),
                read_gpr(state, decoded.right),
                count,
            );
            write_gpr(state, decoded.target, value);
            advance_queue(state);
            Ok("SHD")
        }
        Kind::Conditional {
            add,
            immediate,
            branch_false,
        } => {
            let left = if immediate {
                gates::sign_extend_low(u32::from(decoded.left), 5)
            } else {
                read_gpr(state, decoded.left)
            };
            let right = read_gpr(state, decoded.right);
            let predicate = if add {
                let (result, carry, overflow, _) = gates::add32(left, right, 0);
                write_gpr(state, decoded.right, result);
                gates::add_condition(decoded.condition, result, carry, overflow)
            } else {
                gates::compare_condition(decoded.condition, left, right)
            };
            let taken = xor_gate(predicate, u8::from(branch_false));
            install_conditional_branch(state, taken, gates::branch_disp12(word), decoded.nullify);
            Ok(match (add, immediate, branch_false) {
                (false, false, false) => "COMBT",
                (false, false, true) => "COMBF",
                (false, true, false) => "COMIBT",
                (false, true, true) => "COMIBF",
                (true, false, false) => "ADDBT",
                (true, false, true) => "ADDBF",
                (true, true, false) => "ADDIBT",
                (true, true, true) => "ADDIBF",
            })
        }
        Kind::Bl | Kind::Blr | Kind::Bv => {
            let old_pc = state.pc;
            let old_next = state.next_pc;
            let link = gates::add32(old_next, 4, 0).0;
            let target = match decoded.kind {
                Kind::Bl => {
                    write_gpr(state, decoded.right, link);
                    gates::add32(gates::add32(old_pc, 8, 0).0, gates::branch_disp17(word), 0).0
                }
                Kind::Blr => {
                    write_gpr(state, decoded.right, link);
                    let index = gates::shift_left(read_gpr(state, decoded.left), 3);
                    gates::add32(gates::add32(old_pc, 8, 0).0, index, 0).0
                }
                Kind::Bv => {
                    let index = gates::shift_left(read_gpr(state, decoded.left), 3);
                    gates::and32(gates::add32(read_gpr(state, decoded.right), index, 0).0, !3)
                }
                _ => unreachable!("settled branch family"),
            };
            state.pc = old_next;
            state.next_pc = target;
            state.nullify_next = decoded.nullify;
            state.in_delay_slot = true;
            Ok(match decoded.kind {
                Kind::Bl => "BL",
                Kind::Blr => "BLR",
                _ => "BV",
            })
        }
    }
}
