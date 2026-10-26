//! Independent gate-network architectural transition engine.

use crate::{gates, hfp::GateHfp, StepTrace, System360Error, System360State};

const ADDRESS_MASK: u32 = 0x00ff_ffff;

pub(crate) fn execute(
    before: &System360State,
    trace: &StepTrace,
) -> Result<System360State, System360Error> {
    let mut next = before.clone();
    next.pc = gates::add_24(before.pc, u32::from(trace.width));
    match trace.raw.len() {
        2 => execute_rr(before, &mut next, &trace.raw)?,
        4 => execute_four(before, &mut next, &trace.raw)?,
        6 => execute_ss(before, &mut next, &trace.raw)?,
        _ => return gate_error("gate decoder received an invalid instruction width"),
    }
    Ok(next)
}

fn execute_rr(
    before: &System360State,
    next: &mut System360State,
    raw: &[u8],
) -> Result<(), System360Error> {
    let opcode = raw[0];
    let r1 = usize::from(raw[1] >> 4);
    let r2 = usize::from(raw[1] & 15);
    let a = before.gprs[r1];
    let b = before.gprs[r2];
    match opcode {
        0x0a => next.halted = true,
        0x10 => {
            next.gprs[r1] = if b & 0x8000_0000 == 0 {
                b
            } else {
                gates::negate_32(b).0
            };
            next.condition_code = gates::signed_condition_code(next.gprs[r1]);
        }
        0x11 => {
            next.gprs[r1] = if b & 0x8000_0000 == 0 && b != 0 {
                gates::negate_32(b).0
            } else {
                b
            };
            next.condition_code = gates::signed_condition_code(next.gprs[r1]);
        }
        0x12 | 0x18 => {
            next.gprs[r1] = b;
            if opcode == 0x12 {
                next.condition_code = gates::signed_condition_code(b);
            }
        }
        0x13 => {
            let (value, overflow) = gates::negate_32(b);
            require_zero(
                overflow,
                "gate complement network overflowed an oracle-valid order",
            )?;
            next.gprs[r1] = value;
            next.condition_code = gates::signed_condition_code(value);
        }
        0x14 | 0x16 | 0x17 => {
            let value = match opcode {
                0x14 => gates::bit_and_32(a, b),
                0x16 => gates::bit_or_32(a, b),
                _ => gates::bit_xor_32(a, b),
            };
            next.gprs[r1] = value;
            next.condition_code = gates::logical_condition_code(value);
        }
        0x15 => next.condition_code = gates::unsigned_compare_condition_code(a, b),
        0x19 => next.condition_code = gates::signed_compare_condition_code(a, b),
        0x1a | 0x1b => {
            let (value, _, overflow) = if opcode == 0x1a {
                gates::add_32(a, b, 0)
            } else {
                gates::add_32(a, gates::bit_not_32(b), 1)
            };
            require_zero(overflow, "gate signed ALU overflowed an oracle-valid order")?;
            next.gprs[r1] = value;
            next.condition_code = gates::signed_condition_code(value);
        }
        0x1e | 0x1f => {
            let (value, carry, _) = if opcode == 0x1e {
                gates::add_32(a, b, 0)
            } else {
                gates::add_32(a, gates::bit_not_32(b), 1)
            };
            next.gprs[r1] = value;
            next.condition_code = (carry << 1) | gates::logical_condition_code(value);
        }
        0x1c => {
            let product = gates::multiply_signed_32(before.gprs[r1 + 1], b);
            next.gprs[r1] = (product >> 32) as u32;
            next.gprs[r1 + 1] = product as u32;
        }
        0x1d => {
            let dividend = (u64::from(a) << 32) | u64::from(before.gprs[r1 + 1]);
            let (remainder, quotient) = gates::divide_signed_64_by_32(dividend, b).ok_or(
                System360Error::Specification {
                    reason: "gate divider rejected an oracle-valid order",
                },
            )?;
            next.gprs[r1] = remainder;
            next.gprs[r1 + 1] = quotient;
        }
        0x05 => {
            next.gprs[r1] = next.pc;
            next.pc = gates::select_u32(
                next.pc,
                b & ADDRESS_MASK,
                gates::register_field_nonzero(r2 as u8),
            );
        }
        0x06 => {
            let value = gates::add_32(a, u32::MAX, 0).0;
            next.gprs[r1] = value;
            next.pc = gates::select_u32(
                next.pc,
                b & ADDRESS_MASK,
                gates::count_branch_selected(value, r2 as u8),
            );
        }
        0x07 => {
            next.pc = gates::select_u32(
                next.pc,
                b & ADDRESS_MASK,
                gates::branch_selected(r1 as u8, before.condition_code),
            );
        }
        0x20..=0x3f => execute_float_rr(before, next, opcode, r1, r2)?,
        _ => return gate_error("gate RR decoder accepted an unsupported opcode"),
    }
    Ok(())
}

fn execute_four(
    before: &System360State,
    next: &mut System360State,
    raw: &[u8],
) -> Result<(), System360Error> {
    match raw[0] {
        0x91..=0x97 => execute_si(before, next, raw),
        0x86..=0x90 | 0x98 => execute_rs(before, next, raw),
        _ => execute_rx(before, next, raw),
    }
}

fn execute_rx(
    before: &System360State,
    next: &mut System360State,
    raw: &[u8],
) -> Result<(), System360Error> {
    let opcode = raw[0];
    let register = usize::from(raw[1] >> 4);
    let address = effective_address_rx(raw, before) as usize;
    let a = before.gprs[register];
    if matches!(opcode, 0x60..=0x7f) {
        return execute_float_rx(before, next, opcode, register, address);
    }
    let word = || read_be32(&before.memory, address);
    let half = || i32::from(read_be16(&before.memory, address) as i16) as u32;
    match opcode {
        0x40 => write_bytes(&mut next.memory, address, &(a as u16).to_be_bytes()),
        0x42 => next.memory[address] = a as u8,
        0x50 => write_bytes(&mut next.memory, address, &a.to_be_bytes()),
        0x41 => next.gprs[register] = address as u32,
        0x43 => next.gprs[register] = (a & 0xffff_ff00) | u32::from(before.memory[address]),
        0x48 => next.gprs[register] = half(),
        0x58 => next.gprs[register] = word(),
        0x4a | 0x4b | 0x5a | 0x5b => {
            let b = if matches!(opcode, 0x4a | 0x4b) {
                half()
            } else {
                word()
            };
            let (value, _, overflow) = if matches!(opcode, 0x4a | 0x5a) {
                gates::add_32(a, b, 0)
            } else {
                gates::add_32(a, gates::bit_not_32(b), 1)
            };
            require_zero(overflow, "gate RX ALU overflowed an oracle-valid order")?;
            next.gprs[register] = value;
            next.condition_code = gates::signed_condition_code(value);
        }
        0x49 => next.condition_code = gates::signed_compare_condition_code(a, half()),
        0x59 => next.condition_code = gates::signed_compare_condition_code(a, word()),
        0x55 => next.condition_code = gates::unsigned_compare_condition_code(a, word()),
        0x54 | 0x56 | 0x57 => {
            let value = match opcode {
                0x54 => gates::bit_and_32(a, word()),
                0x56 => gates::bit_or_32(a, word()),
                _ => gates::bit_xor_32(a, word()),
            };
            next.gprs[register] = value;
            next.condition_code = gates::logical_condition_code(value);
        }
        0x5e | 0x5f => {
            let b = word();
            let (value, carry, _) = if opcode == 0x5e {
                gates::add_32(a, b, 0)
            } else {
                gates::add_32(a, gates::bit_not_32(b), 1)
            };
            next.gprs[register] = value;
            next.condition_code = (carry << 1) | gates::logical_condition_code(value);
        }
        0x5c => {
            let product = gates::multiply_signed_32(before.gprs[register + 1], word());
            next.gprs[register] = (product >> 32) as u32;
            next.gprs[register + 1] = product as u32;
        }
        0x4c => next.gprs[register] = gates::multiply_signed_32(a, half()) as u32,
        0x5d => {
            let dividend = (u64::from(a) << 32) | u64::from(before.gprs[register + 1]);
            let (remainder, quotient) = gates::divide_signed_64_by_32(dividend, word()).ok_or(
                System360Error::Specification {
                    reason: "gate RX divider rejected an oracle-valid order",
                },
            )?;
            next.gprs[register] = remainder;
            next.gprs[register + 1] = quotient;
        }
        0x45 => {
            next.gprs[register] = next.pc;
            next.pc = address as u32;
        }
        0x46 => {
            let decremented = gates::add_32(a, u32::MAX, 0).0;
            next.gprs[register] = decremented;
            next.pc = gates::select_u32(next.pc, address as u32, gates::nonzero_32(decremented));
        }
        0x47 => {
            next.pc = gates::select_u32(
                next.pc,
                address as u32,
                gates::branch_selected(register as u8, before.condition_code),
            );
        }
        _ => return gate_error("gate RX decoder accepted an unsupported opcode"),
    }
    Ok(())
}

fn execute_rs(
    before: &System360State,
    next: &mut System360State,
    raw: &[u8],
) -> Result<(), System360Error> {
    let opcode = raw[0];
    let r1 = usize::from(raw[1] >> 4);
    let r3 = usize::from(raw[1] & 15);
    let address = effective_address_base(raw, 2, before) as usize;
    let count = (address as u32) & 63;
    let source = before.gprs[r1];
    match opcode {
        0x90 => {
            for offset in 0..register_span(r1, r3) {
                write_bytes(
                    &mut next.memory,
                    address + offset * 4,
                    &before.gprs[(r1 + offset) & 15].to_be_bytes(),
                );
            }
        }
        0x98 => {
            for offset in 0..register_span(r1, r3) {
                next.gprs[(r1 + offset) & 15] = read_be32(&before.memory, address + offset * 4);
            }
        }
        0x88 => next.gprs[r1] = gates::shift_right_32(source, count),
        0x89 => next.gprs[r1] = gates::shift_left_32(source, count),
        0x8a => {
            let value = gates::arithmetic_right_32(source, count);
            next.gprs[r1] = value;
            next.condition_code = gates::signed_condition_code(value);
        }
        0x8b => {
            let value =
                (source & 0x8000_0000) | (gates::shift_left_32(source, count) & 0x7fff_ffff);
            next.gprs[r1] = value;
            next.condition_code = gates::signed_condition_code(value);
        }
        0x8c..=0x8f => {
            let pair = (u64::from(source) << 32) | u64::from(before.gprs[r1 + 1]);
            let value = match opcode {
                0x8c => gates::shift_right_64(pair, count),
                0x8d => gates::shift_left_64(pair, count),
                0x8e => gates::arithmetic_right_64(pair, count),
                _ => {
                    (pair & 0x8000_0000_0000_0000)
                        | (gates::shift_left_64(pair, count) & 0x7fff_ffff_ffff_ffff)
                }
            };
            next.gprs[r1] = (value >> 32) as u32;
            next.gprs[r1 + 1] = value as u32;
            if matches!(opcode, 0x8e | 0x8f) {
                next.condition_code = gates::signed_condition_code_64(value);
            }
        }
        0x86 | 0x87 => {
            let compare = if r3.is_multiple_of(2) { r3 + 1 } else { r3 };
            let incremented = gates::add_32(source, before.gprs[r3], 0).0;
            next.gprs[r1] = incremented;
            let comparison =
                gates::signed_compare_condition_code(incremented, before.gprs[compare]);
            next.pc = gates::select_u32(
                next.pc,
                address as u32,
                gates::index_branch_selected(opcode, comparison),
            );
        }
        _ => return gate_error("gate RS decoder accepted an unsupported opcode"),
    }
    Ok(())
}

fn execute_si(
    before: &System360State,
    next: &mut System360State,
    raw: &[u8],
) -> Result<(), System360Error> {
    let opcode = raw[0];
    let immediate = raw[1];
    let address = effective_address_base(raw, 2, before) as usize;
    let a = u32::from(before.memory[address]);
    let b = u32::from(immediate);
    match opcode {
        0x91 => {
            let value = gates::bit_and_32(a, b) as u8;
            next.condition_code = if value == 0 {
                0
            } else if value == immediate {
                3
            } else {
                1
            };
        }
        0x92 => next.memory[address] = immediate,
        0x94 | 0x96 | 0x97 => {
            let value = match opcode {
                0x94 => gates::bit_and_32(a, b),
                0x96 => gates::bit_or_32(a, b),
                _ => gates::bit_xor_32(a, b),
            } as u8;
            next.memory[address] = value;
            next.condition_code = gates::logical_condition_code(u32::from(value));
        }
        0x95 => next.condition_code = gates::unsigned_compare_condition_code(a, b),
        _ => return gate_error("gate SI decoder accepted an unsupported opcode"),
    }
    Ok(())
}

fn execute_ss(
    before: &System360State,
    next: &mut System360State,
    raw: &[u8],
) -> Result<(), System360Error> {
    let opcode = raw[0];
    let length = usize::from(raw[1]) + 1;
    let destination = effective_address_base(raw, 2, before) as usize;
    let source = effective_address_base(raw, 4, before) as usize;
    match opcode {
        0xd2 => {
            for offset in 0..length {
                next.memory[destination + offset] = next.memory[source + offset];
            }
        }
        0xd5 => {
            next.condition_code = compare_bytes(
                &before.memory[destination..destination + length],
                &before.memory[source..source + length],
            );
        }
        0xd4 | 0xd6 | 0xd7 => {
            let mut any = 0_u8;
            for offset in 0..length {
                let a = u32::from(next.memory[destination + offset]);
                let b = u32::from(next.memory[source + offset]);
                let value = match opcode {
                    0xd4 => gates::bit_and_32(a, b),
                    0xd6 => gates::bit_or_32(a, b),
                    _ => gates::bit_xor_32(a, b),
                } as u8;
                next.memory[destination + offset] = value;
                any |= value;
            }
            next.condition_code = gates::logical_condition_code(u32::from(any));
        }
        _ => return gate_error("gate SS decoder accepted an unsupported opcode"),
    }
    Ok(())
}

fn execute_float_rr(
    before: &System360State,
    next: &mut System360State,
    opcode: u8,
    r1: usize,
    r2: usize,
) -> Result<(), System360Error> {
    let slot1 = r1 / 2;
    let slot2 = r2 / 2;
    let long = opcode < 0x30;
    let digits = if long { 14 } else { 6 };
    let raw_a = before.fprs[slot1];
    let raw_b = before.fprs[slot2];
    let a = GateHfp::decode(raw_a, long);
    let b = GateHfp::decode(raw_b, long);
    match opcode {
        0x28 | 0x38 => store_float(
            next,
            slot1,
            gates::hfp_with_sign(raw_b, long, raw_b >> 63 != 0),
            long,
        ),
        0x22 | 0x32 => {
            let value = gates::hfp_with_sign(raw_b, long, raw_b >> 63 != 0);
            store_float(next, slot1, value, long);
            next.condition_code = gates::hfp_condition_code(value, long);
        }
        0x23 | 0x33 => {
            let value = gates::hfp_with_sign(raw_b, long, raw_b >> 63 == 0);
            store_float(next, slot1, value, long);
            next.condition_code = gates::hfp_condition_code(value, long);
        }
        0x20 | 0x30 => {
            let value = gates::hfp_with_sign(raw_b, long, false);
            store_float(next, slot1, value, long);
            next.condition_code = gates::hfp_condition_code(value, long);
        }
        0x21 | 0x31 => {
            let value =
                gates::hfp_with_sign(raw_b, long, gates::hfp_fraction_nonzero(raw_b, long) == 1);
            store_float(next, slot1, value, long);
            next.condition_code = gates::hfp_condition_code(value, long);
        }
        0x2a | 0x3a | 0x2b | 0x3b | 0x24 | 0x34 | 0x2c | 0x3c | 0x2d | 0x3d => {
            let value = match opcode {
                0x2a | 0x3a => a.add(b, digits),
                0x2b | 0x3b => a.add(b.with_complemented_sign(), digits),
                0x24 | 0x34 => a.halve(digits),
                0x2c | 0x3c => a.multiply(b, digits),
                _ => a.divide(b, digits),
            }
            .and_then(|value| value.encode(long))
            .ok_or(System360Error::Specification {
                reason: "gate HFP network rejected an oracle-valid RR order",
            })?;
            store_float(next, slot1, value, long);
            if matches!(opcode, 0x2a | 0x3a | 0x2b | 0x3b) {
                next.condition_code = gates::hfp_condition_code(value, long);
            }
        }
        0x29 | 0x39 => next.condition_code = a.compare(b),
        _ => return gate_error("gate floating RR decoder accepted an unsupported opcode"),
    }
    Ok(())
}

fn execute_float_rx(
    before: &System360State,
    next: &mut System360State,
    opcode: u8,
    register: usize,
    address: usize,
) -> Result<(), System360Error> {
    let slot = register / 2;
    let long = opcode < 0x70;
    let digits = if long { 14 } else { 6 };
    let memory_raw = if long {
        read_be64(&before.memory, address)
    } else {
        u64::from(read_be32(&before.memory, address)) << 32
    };
    let a = GateHfp::decode(before.fprs[slot], long);
    let b = GateHfp::decode(memory_raw, long);
    match opcode {
        0x68 | 0x78 => store_float(
            next,
            slot,
            gates::hfp_with_sign(memory_raw, long, memory_raw >> 63 != 0),
            long,
        ),
        0x60 => write_bytes(&mut next.memory, address, &before.fprs[slot].to_be_bytes()),
        0x70 => write_bytes(
            &mut next.memory,
            address,
            &((before.fprs[slot] >> 32) as u32).to_be_bytes(),
        ),
        0x6a | 0x7a | 0x6b | 0x7b | 0x6c | 0x7c | 0x6d | 0x7d => {
            let value = match opcode {
                0x6a | 0x7a => a.add(b, digits),
                0x6b | 0x7b => a.add(b.with_complemented_sign(), digits),
                0x6c | 0x7c => a.multiply(b, digits),
                _ => a.divide(b, digits),
            }
            .and_then(|value| value.encode(long))
            .ok_or(System360Error::Specification {
                reason: "gate HFP network rejected an oracle-valid RX order",
            })?;
            store_float(next, slot, value, long);
            if matches!(opcode, 0x6a | 0x7a | 0x6b | 0x7b) {
                next.condition_code = gates::hfp_condition_code(value, long);
            }
        }
        0x69 | 0x79 => next.condition_code = a.compare(b),
        _ => return gate_error("gate floating RX decoder accepted an unsupported opcode"),
    }
    Ok(())
}

fn store_float(state: &mut System360State, slot: usize, value: u64, long: bool) {
    state.fprs[slot] = if long {
        value
    } else {
        (value & 0xffff_ffff_0000_0000) | (state.fprs[slot] & 0xffff_ffff)
    };
}

fn effective_address_base(raw: &[u8], offset: usize, state: &System360State) -> u32 {
    let base = usize::from(raw[offset] >> 4);
    let displacement = (u32::from(raw[offset] & 15) << 8) | u32::from(raw[offset + 1]);
    gates::add_24(if base == 0 { 0 } else { state.gprs[base] }, displacement)
}

fn effective_address_rx(raw: &[u8], state: &System360State) -> u32 {
    let index = usize::from(raw[1] & 15);
    gates::add_24(
        if index == 0 { 0 } else { state.gprs[index] },
        effective_address_base(raw, 2, state),
    )
}

fn read_be16(memory: &[u8], address: usize) -> u16 {
    u16::from_be_bytes([memory[address], memory[address + 1]])
}

fn read_be32(memory: &[u8], address: usize) -> u32 {
    u32::from_be_bytes(
        memory[address..address + 4]
            .try_into()
            .expect("checked word"),
    )
}

fn read_be64(memory: &[u8], address: usize) -> u64 {
    u64::from_be_bytes(
        memory[address..address + 8]
            .try_into()
            .expect("checked doubleword"),
    )
}

fn write_bytes(memory: &mut [u8], address: usize, value: &[u8]) {
    memory[address..address + value.len()].copy_from_slice(value);
}

fn register_span(first: usize, last: usize) -> usize {
    if last >= first {
        last - first + 1
    } else {
        17 - first + last
    }
}

fn compare_bytes(a: &[u8], b: &[u8]) -> u8 {
    for (&left, &right) in a.iter().zip(b) {
        let condition = gates::unsigned_compare_condition_code(u32::from(left), u32::from(right));
        if condition != 0 {
            return condition;
        }
    }
    0
}

fn require_zero(value: u8, reason: &'static str) -> Result<(), System360Error> {
    if value == 0 {
        Ok(())
    } else {
        gate_error(reason)
    }
}

fn gate_error<T>(reason: &'static str) -> Result<T, System360Error> {
    Err(System360Error::Specification { reason })
}
