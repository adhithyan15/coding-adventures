//! Independent IAS transition engine built from gate networks.

use crate::gates;
use ias_simulator::{Half, IasError, IasState, Opcode, StepTrace, MEMORY_WORDS};

const INSTRUCTION_MASK: u32 = (1_u32 << 20) - 1;

pub(crate) fn execute(before: &IasState) -> Result<(IasState, StepTrace), IasError> {
    if before.halted {
        return Err(IasError::Halted);
    }
    let mut state = before.clone();
    let (word_address, half, raw) = if let Some(right) = state.buffered_right.take() {
        let source = state
            .control_counter
            .checked_sub(1)
            .ok_or(IasError::InvalidControlCounter)?;
        (source, Half::Right, right)
    } else {
        let word_address = state.control_counter;
        ensure_fetch(&state, word_address)?;
        let fetched = state.memory[usize::from(word_address)];
        state.memory_address = word_address;
        state.selectron_register = fetched;
        state.buffered_right = Some((fetched & u64::from(INSTRUCTION_MASK)) as u32);
        state.control_counter = word_address + 1;
        (word_address, Half::Left, (fetched >> 20) as u32)
    };

    if raw > INSTRUCTION_MASK {
        return Err(IasError::InvalidInstruction { value: raw });
    }
    let fetched_opcode = (raw >> 12) as u8;
    let opcode_number = gates::decode_opcode(fetched_opcode).ok_or(IasError::InvalidOpcode {
        opcode: fetched_opcode,
    })?;
    let address = gates::decode_address((raw & 0x0fff) as u16);
    state.current_instruction = raw;
    state.function_table = opcode_number;
    state.memory_address = address;
    let accumulator_before = state.accumulator;
    let arithmetic_before = state.arithmetic_register;
    let opcode = opcode(opcode_number)?;

    execute_order(&mut state, opcode, address)?;
    let trace = StepTrace {
        word_address,
        half,
        raw,
        opcode,
        address,
        mnemonic: opcode.mnemonic(),
        accumulator_before,
        accumulator_after: state.accumulator,
        arithmetic_before,
        arithmetic_after: state.arithmetic_register,
        state_after: state.clone(),
    };
    Ok((state, trace))
}

fn opcode(value: u8) -> Result<Opcode, IasError> {
    let opcode = match value {
        0 => Opcode::Halt,
        1 => Opcode::Load,
        2 => Opcode::LoadNegative,
        3 => Opcode::LoadAbsolute,
        4 => Opcode::LoadNegativeAbsolute,
        5 => Opcode::Add,
        6 => Opcode::Subtract,
        7 => Opcode::AddAbsolute,
        8 => Opcode::SubtractAbsolute,
        9 => Opcode::LoadArithmetic,
        10 => Opcode::LoadArithmeticIntoAccumulator,
        11 => Opcode::Multiply,
        12 => Opcode::Divide,
        13 => Opcode::JumpLeft,
        14 => Opcode::JumpRight,
        15 => Opcode::JumpNonNegativeLeft,
        16 => Opcode::JumpNonNegativeRight,
        17 => Opcode::Store,
        18 => Opcode::StoreLeftAddress,
        19 => Opcode::StoreRightAddress,
        20 => Opcode::ShiftLeft,
        21 => Opcode::ShiftRight,
        _ => return Err(IasError::InvalidOpcode { opcode: value }),
    };
    Ok(opcode)
}

fn execute_order(state: &mut IasState, opcode: Opcode, address: u16) -> Result<(), IasError> {
    match opcode {
        Opcode::Halt => {
            state.halted = true;
            state.buffered_right = None;
        }
        Opcode::Load => state.accumulator = read_operand(state, address),
        Opcode::LoadNegative => state.accumulator = gates::negate40(read_operand(state, address)),
        Opcode::LoadAbsolute => state.accumulator = gates::absolute40(read_operand(state, address)),
        Opcode::LoadNegativeAbsolute => {
            let operand = read_operand(state, address);
            state.accumulator = gates::negate40(gates::absolute40(operand));
        }
        Opcode::Add => {
            let operand = read_operand(state, address);
            state.accumulator = gates::add40(state.accumulator, operand);
        }
        Opcode::Subtract => {
            let operand = read_operand(state, address);
            state.accumulator = gates::add40(state.accumulator, gates::negate40(operand));
        }
        Opcode::AddAbsolute => {
            let operand = read_operand(state, address);
            state.accumulator = gates::add40(state.accumulator, gates::absolute40(operand));
        }
        Opcode::SubtractAbsolute => {
            let operand = read_operand(state, address);
            state.accumulator = gates::add40(
                state.accumulator,
                gates::negate40(gates::absolute40(operand)),
            );
        }
        Opcode::LoadArithmetic => state.arithmetic_register = read_operand(state, address),
        Opcode::LoadArithmeticIntoAccumulator => state.accumulator = state.arithmetic_register,
        Opcode::Multiply => {
            let operand = read_operand(state, address);
            (state.accumulator, state.arithmetic_register) =
                gates::multiply40(state.arithmetic_register, operand);
        }
        Opcode::Divide => {
            let operand = read_operand(state, address);
            (state.accumulator, state.arithmetic_register) =
                gates::divide40(state.accumulator, operand)?;
        }
        Opcode::JumpLeft => jump_left(state, address)?,
        Opcode::JumpRight => jump_right(state, address)?,
        Opcode::JumpNonNegativeLeft => {
            if !gates::is_negative40(state.accumulator) {
                jump_left(state, address)?;
            }
        }
        Opcode::JumpNonNegativeRight => {
            if !gates::is_negative40(state.accumulator) {
                jump_right(state, address)?;
            }
        }
        Opcode::Store => {
            state.memory[usize::from(address)] = state.accumulator;
            state.selectron_register = state.accumulator;
        }
        Opcode::StoreLeftAddress => store_address(state, address, true),
        Opcode::StoreRightAddress => store_address(state, address, false),
        Opcode::ShiftLeft => state.accumulator = gates::shift_left40(state.accumulator),
        Opcode::ShiftRight => state.accumulator = gates::shift_right40(state.accumulator),
    }
    Ok(())
}

fn read_operand(state: &mut IasState, address: u16) -> u64 {
    let value = state.memory[usize::from(address)];
    state.selectron_register = value;
    value
}

fn jump_left(state: &mut IasState, address: u16) -> Result<(), IasError> {
    ensure_fetch(state, address)?;
    state.control_counter = address;
    state.buffered_right = None;
    Ok(())
}

fn jump_right(state: &mut IasState, address: u16) -> Result<(), IasError> {
    ensure_fetch(state, address)?;
    let fetched = state.memory[usize::from(address)];
    state.selectron_register = fetched;
    state.control_counter = address + 1;
    state.buffered_right = Some((fetched & u64::from(INSTRUCTION_MASK)) as u32);
    Ok(())
}

fn store_address(state: &mut IasState, address: u16, left: bool) {
    let changed = gates::replace_address(
        state.memory[usize::from(address)],
        state.accumulator & 0x0fff,
        left,
    );
    state.memory[usize::from(address)] = changed;
    state.selectron_register = changed;
}

fn ensure_fetch(state: &IasState, word: u16) -> Result<(), IasError> {
    let start = usize::from(state.loaded_origin);
    let end = start + state.loaded_words;
    let address = usize::from(word);
    if state.loaded_words == 0 || address >= MEMORY_WORDS || !(start..end).contains(&address) {
        Err(IasError::FetchOutsideProgram { word })
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ias_simulator::{encode_instruction, pack_instructions};

    #[test]
    fn invalid_opcode_and_unloaded_fetch_fail() {
        assert_eq!(opcode(22), Err(IasError::InvalidOpcode { opcode: 22 }));
        let state = IasState::default();
        assert_eq!(
            execute(&state),
            Err(IasError::FetchOutsideProgram { word: 0 })
        );
    }

    #[test]
    fn gate_transition_executes_a_paired_word() {
        let mut state = IasState {
            loaded_words: 1,
            ..IasState::default()
        };
        let load = encode_instruction(Opcode::Load, 8).unwrap();
        state.memory[0] = pack_instructions(load, 0).unwrap();
        state.memory[8] = 7;
        let (next, trace) = execute(&state).unwrap();
        assert_eq!(trace.opcode, Opcode::Load);
        assert_eq!(next.accumulator, 7);
        assert!(next.buffered_right.is_some());
    }
}
