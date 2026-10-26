//! Independent settled-wire engine for the five System/370 extension orders.

use std::fmt::Write as _;

use crate::allocation::{self, AllocationSite};
use crate::{gates, System370Error, System370State, MEMORY_BYTES};

const ADDRESS_MASK: u32 = 0x00ff_ffff;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Extension {
    Mvcl,
    Clcl,
    Clm,
    Stcm,
    Icm,
}

#[derive(Debug)]
pub(crate) struct SettledExtension {
    pub(crate) raw: Vec<u8>,
    pub(crate) width: u8,
    pub(crate) mnemonic: String,
    pub(crate) description: String,
    pub(crate) pc: u32,
    pub(crate) gpr_updates: [(usize, u32); 4],
    pub(crate) gpr_count: usize,
    pub(crate) memory_writes: Vec<(usize, u8)>,
    pub(crate) condition_code: Option<u8>,
}

#[must_use]
pub(crate) fn classify_extension(opcode: u8) -> Option<Extension> {
    let lines = gates::decode_extension(opcode);
    match lines {
        [1, 0, 0, 0, 0] => Some(Extension::Mvcl),
        [0, 1, 0, 0, 0] => Some(Extension::Clcl),
        [0, 0, 1, 0, 0] => Some(Extension::Clm),
        [0, 0, 0, 1, 0] => Some(Extension::Stcm),
        [0, 0, 0, 0, 1] => Some(Extension::Icm),
        _ => None,
    }
}

#[must_use]
pub(crate) fn inherited_width(opcode: u8) -> Option<u8> {
    match opcode {
        0x05..=0x07 | 0x0a | 0x10..=0x3f => Some(2),
        0x40..=0x7f | 0x86..=0x98 => Some(4),
        0xd2 | 0xd4..=0xd7 => Some(6),
        _ => None,
    }
}

#[must_use]
pub(crate) fn instruction_width(opcode: u8) -> Option<u8> {
    match classify_extension(opcode) {
        Some(Extension::Mvcl | Extension::Clcl) => Some(2),
        Some(Extension::Clm | Extension::Stcm | Extension::Icm) => Some(4),
        None => inherited_width(opcode),
    }
}

pub(crate) fn settle(
    before: &System370State,
    raw: &[u8],
    extension: Extension,
) -> Result<SettledExtension, System370Error> {
    let mut raw_owned = Vec::new();
    allocation::checkpoint(AllocationSite::RawInstruction)
        .map_err(|()| System370Error::AllocationFailed)?;
    raw_owned
        .try_reserve_exact(raw.len())
        .map_err(|_| System370Error::AllocationFailed)?;
    raw_owned.extend_from_slice(raw);
    let width = raw.len() as u8;
    let mut settled = SettledExtension {
        raw: raw_owned,
        width,
        mnemonic: String::new(),
        description: String::new(),
        pc: gates::add_u24(before.pc, u32::from(width)),
        gpr_updates: [(0, 0); 4],
        gpr_count: 0,
        memory_writes: Vec::new(),
        condition_code: None,
    };
    match extension {
        Extension::Mvcl => settle_mvcl(before, raw, &mut settled)?,
        Extension::Clcl => settle_clcl(before, raw, &mut settled)?,
        Extension::Clm => settle_clm(before, raw, &mut settled)?,
        Extension::Stcm => settle_stcm(before, raw, &mut settled)?,
        Extension::Icm => settle_icm(before, raw, &mut settled)?,
    }
    Ok(settled)
}

fn reserve_description() -> Result<String, System370Error> {
    let mut description = String::new();
    allocation::checkpoint(AllocationSite::Description)
        .map_err(|()| System370Error::AllocationFailed)?;
    description
        .try_reserve_exact(64)
        .map_err(|_| System370Error::AllocationFailed)?;
    Ok(description)
}

fn checked_mnemonic(value: &str) -> Result<String, System370Error> {
    let mut mnemonic = String::new();
    allocation::checkpoint(AllocationSite::Description)
        .map_err(|()| System370Error::AllocationFailed)?;
    mnemonic
        .try_reserve_exact(value.len())
        .map_err(|_| System370Error::AllocationFailed)?;
    mnemonic.push_str(value);
    Ok(mnemonic)
}

fn push_gpr(settled: &mut SettledExtension, register: usize, value: u32) {
    settled.gpr_updates[settled.gpr_count] = (register, value);
    settled.gpr_count += 1;
}

fn settle_mvcl(
    before: &System370State,
    raw: &[u8],
    settled: &mut SettledExtension,
) -> Result<(), System370Error> {
    let r1 = usize::from(raw[1] >> 4);
    let r2 = usize::from(raw[1] & 15);
    require_pairs(r1, r2)?;
    let destination = before.gprs[r1] & ADDRESS_MASK;
    let destination_len = before.gprs[r1 + 1] & ADDRESS_MASK;
    let source = before.gprs[r2] & ADDRESS_MASK;
    let source_len = before.gprs[r2 + 1] & ADDRESS_MASK;
    let pad = (before.gprs[r2 + 1] >> 24) as u8;
    let copied = gates::select_u24(
        destination_len,
        source_len,
        gates::compare_u24(source_len, destination_len) & 1,
    );
    let overlap = gates::destructive_overlap(destination, source, copied);
    settled.mnemonic = checked_mnemonic("MVCL")?;
    let mut description = reserve_description()?;
    if overlap == 1 {
        write!(description, "MVCL R{r1},R{r2} (destructive overlap)")
            .expect("writing to String cannot fail");
        push_gpr(settled, r1, destination);
        push_gpr(settled, r2, source);
        settled.condition_code = Some(3);
        settled.description = description;
        return Ok(());
    }
    checked_range(destination, destination_len as usize)?;
    checked_range(source, copied as usize)?;
    allocation::checkpoint(AllocationSite::MemoryWrites)
        .map_err(|()| System370Error::AllocationFailed)?;
    settled
        .memory_writes
        .try_reserve_exact(destination_len as usize)
        .map_err(|_| System370Error::AllocationFailed)?;
    let mut offset = 0;
    while gates::nonzero_u24(gates::sub_u24(destination_len, offset)) == 1 {
        let use_source = gates::compare_u24(offset, source_len) & 1;
        let source_byte = before.memory[gates::add_u24(source, offset) as usize];
        let byte = gates::select_u8(pad, source_byte, use_source);
        settled
            .memory_writes
            .push((gates::add_u24(destination, offset) as usize, byte));
        offset = gates::add_u24(offset, 1);
    }
    push_gpr(settled, r1, gates::add_u24(destination, destination_len));
    push_gpr(settled, r1 + 1, before.gprs[r1 + 1] & 0xff00_0000);
    push_gpr(settled, r2, gates::add_u24(source, copied));
    push_gpr(
        settled,
        r2 + 1,
        (before.gprs[r2 + 1] & 0xff00_0000) | gates::sub_u24(source_len, copied),
    );
    settled.condition_code = Some(gates::compare_u24(destination_len, source_len));
    write!(description, "MVCL R{r1},R{r2}").expect("writing to String cannot fail");
    settled.description = description;
    Ok(())
}

fn settle_clcl(
    before: &System370State,
    raw: &[u8],
    settled: &mut SettledExtension,
) -> Result<(), System370Error> {
    let r1 = usize::from(raw[1] >> 4);
    let r2 = usize::from(raw[1] & 15);
    require_pairs(r1, r2)?;
    let mut first_address = before.gprs[r1] & ADDRESS_MASK;
    let mut first_len = before.gprs[r1 + 1] & ADDRESS_MASK;
    let mut second_address = before.gprs[r2] & ADDRESS_MASK;
    let mut second_len = before.gprs[r2 + 1] & ADDRESS_MASK;
    let pad = (before.gprs[r2 + 1] >> 24) as u8;
    let mut condition_code = 0;
    while gates::nonzero_u24(first_len) | gates::nonzero_u24(second_len) == 1 {
        if gates::nonzero_u24(first_len) == 1 {
            checked_range(first_address, 1)?;
        }
        if gates::nonzero_u24(second_len) == 1 {
            checked_range(second_address, 1)?;
        }
        let first_present = gates::nonzero_u24(first_len);
        let first_value = if first_present == 1 {
            before.memory[first_address as usize]
        } else {
            0
        };
        let first = gates::select_u8(pad, first_value, first_present);
        let second_present = gates::nonzero_u24(second_len);
        let second_value = if second_present == 1 {
            before.memory[second_address as usize]
        } else {
            0
        };
        let second = gates::select_u8(pad, second_value, second_present);
        condition_code = gates::compare_byte(first, second);
        if condition_code != 0 {
            break;
        }
        first_address = gates::select_u24(
            first_address,
            gates::add_u24(first_address, 1),
            first_present,
        );
        first_len = gates::select_u24(first_len, gates::sub_u24(first_len, 1), first_present);
        second_address = gates::select_u24(
            second_address,
            gates::add_u24(second_address, 1),
            second_present,
        );
        second_len = gates::select_u24(second_len, gates::sub_u24(second_len, 1), second_present);
    }
    push_gpr(settled, r1, first_address);
    push_gpr(
        settled,
        r1 + 1,
        (before.gprs[r1 + 1] & 0xff00_0000) | first_len,
    );
    push_gpr(settled, r2, second_address);
    push_gpr(
        settled,
        r2 + 1,
        (before.gprs[r2 + 1] & 0xff00_0000) | second_len,
    );
    settled.condition_code = Some(condition_code);
    settled.mnemonic = checked_mnemonic("CLCL")?;
    let mut description = reserve_description()?;
    write!(description, "CLCL R{r1},R{r2}").expect("writing to String cannot fail");
    settled.description = description;
    Ok(())
}

fn settle_clm(
    before: &System370State,
    raw: &[u8],
    settled: &mut SettledExtension,
) -> Result<(), System370Error> {
    let register = usize::from(raw[1] >> 4);
    let mask = raw[1] & 15;
    let address = effective_address(before, raw);
    if mask == 0 {
        checked_range(address, 1)?;
    }
    let mut cc = 0;
    for byte_index in 0..4 {
        let enable = gates::mask_enabled(mask, byte_index);
        if enable == 0 {
            continue;
        }
        let storage = gates::add_u24(address, u32::from(gates::mask_prefix(mask, byte_index)));
        checked_range(storage, 1)?;
        let register_byte = (before.gprs[register] >> (24 - byte_index * 8)) as u8;
        cc = gates::compare_byte(register_byte, before.memory[storage as usize]);
        if cc != 0 {
            break;
        }
    }
    settled.condition_code = Some(cc);
    settled.mnemonic = checked_mnemonic("CLM")?;
    let mut description = reserve_description()?;
    write!(description, "CLM R{register},{mask:#x},{address:#08x}")
        .expect("writing to String cannot fail");
    settled.description = description;
    Ok(())
}

fn settle_stcm(
    before: &System370State,
    raw: &[u8],
    settled: &mut SettledExtension,
) -> Result<(), System370Error> {
    let register = usize::from(raw[1] >> 4);
    let mask = raw[1] & 15;
    let address = effective_address(before, raw);
    allocation::checkpoint(AllocationSite::MemoryWrites)
        .map_err(|()| System370Error::AllocationFailed)?;
    settled
        .memory_writes
        .try_reserve_exact(mask.count_ones() as usize)
        .map_err(|_| System370Error::AllocationFailed)?;
    for byte_index in 0..4 {
        let enable = gates::mask_enabled(mask, byte_index);
        if enable == 0 {
            continue;
        }
        let storage = gates::add_u24(address, u32::from(gates::mask_prefix(mask, byte_index)));
        checked_range(storage, 1)?;
        settled.memory_writes.push((
            storage as usize,
            (before.gprs[register] >> (24 - byte_index * 8)) as u8,
        ));
    }
    settled.mnemonic = checked_mnemonic("STCM")?;
    let mut description = reserve_description()?;
    write!(description, "STCM R{register},{mask:#x},{address:#08x}")
        .expect("writing to String cannot fail");
    settled.description = description;
    Ok(())
}

fn settle_icm(
    before: &System370State,
    raw: &[u8],
    settled: &mut SettledExtension,
) -> Result<(), System370Error> {
    let register = usize::from(raw[1] >> 4);
    let mask = raw[1] & 15;
    let address = effective_address(before, raw);
    if mask == 0 {
        checked_range(address, 1)?;
    }
    let mut bytes = [0; 4];
    let mut value = before.gprs[register];
    for (byte_index, byte) in bytes.iter_mut().enumerate() {
        let enable = gates::mask_enabled(mask, byte_index);
        if enable == 0 {
            continue;
        }
        let storage = gates::add_u24(address, u32::from(gates::mask_prefix(mask, byte_index)));
        checked_range(storage, 1)?;
        *byte = before.memory[storage as usize];
        value = gates::merge_register_byte(value, *byte, byte_index, enable);
    }
    if mask != 0 {
        push_gpr(settled, register, value);
    }
    settled.condition_code = Some(gates::icm_condition_code(mask, bytes));
    settled.mnemonic = checked_mnemonic("ICM")?;
    let mut description = reserve_description()?;
    if mask == 0 {
        write!(description, "ICM R{register},0,{address:#08x}")
            .expect("writing to String cannot fail");
    } else {
        write!(description, "ICM R{register},{mask:#x},{address:#08x}")
            .expect("writing to String cannot fail");
    }
    settled.description = description;
    Ok(())
}

fn require_pairs(r1: usize, r2: usize) -> Result<(), System370Error> {
    let valid = logic_gates::gates::and_gate(
        logic_gates::gates::not_gate((r1 & 1) as u8),
        logic_gates::gates::not_gate((r2 & 1) as u8),
    );
    if valid == 0 {
        Err(System370Error::Specification {
            reason: "long instruction register pairs must start at even registers",
        })
    } else {
        Ok(())
    }
}

fn effective_address(before: &System370State, raw: &[u8]) -> u32 {
    let base = usize::from(raw[2] >> 4);
    let displacement = (u32::from(raw[2] & 15) << 8) | u32::from(raw[3]);
    let base_present = ibm_system360_gatelevel::gates::register_field_nonzero(base as u8);
    let selected = gates::select_u24(0, before.gprs[base], base_present);
    gates::add_u24(selected, displacement)
}

fn checked_range(address: u32, bytes: usize) -> Result<std::ops::Range<usize>, System370Error> {
    let end = (address as usize)
        .checked_add(bytes)
        .ok_or(System370Error::MemoryOutOfRange { address, bytes })?;
    if address > ADDRESS_MASK || end > MEMORY_BYTES {
        return Err(System370Error::MemoryOutOfRange { address, bytes });
    }
    Ok(address as usize..end)
}
