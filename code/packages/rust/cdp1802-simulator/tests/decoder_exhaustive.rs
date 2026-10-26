use cdp1802_simulator::{decode, Cdp1802Error, InstructionClass};

fn expected_width(opcode: u8) -> u8 {
    match opcode {
        0x30..=0x3f | 0x7c | 0x7d | 0x7f | 0xf8..=0xfd | 0xff => 2,
        0xc0..=0xc3 | 0xc9..=0xcb => 3,
        _ => 1,
    }
}

fn expected_class(opcode: u8) -> InstructionClass {
    match opcode >> 4 {
        0 if opcode == 0 => InstructionClass::Idle,
        0 => InstructionClass::LoadN,
        1 => InstructionClass::Increment,
        2 => InstructionClass::Decrement,
        3 => InstructionClass::ShortControl,
        4 => InstructionClass::LoadAdvance,
        5 => InstructionClass::Store,
        6 => InstructionClass::Io,
        7 => InstructionClass::Control,
        8 => InstructionClass::GetLow,
        9 => InstructionClass::GetHigh,
        10 => InstructionClass::PutLow,
        11 => InstructionClass::PutHigh,
        12 => InstructionClass::LongControl,
        13 => InstructionClass::SetProgram,
        14 => InstructionClass::SetIndex,
        15 => InstructionClass::Alu,
        _ => unreachable!(),
    }
}

fn expected_mnemonic(opcode: u8) -> &'static str {
    const SHORT: [&str; 16] = [
        "BR", "BQ", "BZ", "BDF", "B1", "B2", "B3", "B4", "SKP", "BNQ", "BNZ", "BNF", "BN1", "BN2",
        "BN3", "BN4",
    ];
    const SIX: [&str; 16] = [
        "IRX", "OUT 1", "OUT 2", "OUT 3", "OUT 4", "OUT 5", "OUT 6", "OUT 7", "", "INP 1", "INP 2",
        "INP 3", "INP 4", "INP 5", "INP 6", "INP 7",
    ];
    const SEVEN: [&str; 16] = [
        "RET", "DIS", "LDXA", "STXD", "ADC", "SDB", "SHRC", "SMB", "SAV", "MARK", "REQ", "SEQ",
        "ADCI", "SDBI", "SHLC", "SMBI",
    ];
    const LONG: [&str; 16] = [
        "LBR", "LBQ", "LBZ", "LBDF", "NOP", "LSNQ", "LSNZ", "LSNF", "LSKP", "LBNQ", "LBNZ", "LBNF",
        "LSIE", "LSQ", "LSZ", "LSDF",
    ];
    const ALU: [&str; 16] = [
        "LDX", "OR", "AND", "XOR", "ADD", "SD", "SHR", "SM", "LDI", "ORI", "ANI", "XRI", "ADI",
        "SDI", "SHL", "SMI",
    ];
    let low = usize::from(opcode & 0x0f);
    match opcode >> 4 {
        0 if opcode == 0 => "IDL",
        0 => "LDN",
        1 => "INC",
        2 => "DEC",
        3 => SHORT[low],
        4 => "LDA",
        5 => "STR",
        6 => SIX[low],
        7 => SEVEN[low],
        8 => "GLO",
        9 => "GHI",
        10 => "PLO",
        11 => "PHI",
        12 => LONG[low],
        13 => "SEP",
        14 => "SEX",
        15 => ALU[low],
        _ => unreachable!(),
    }
}

#[test]
fn every_first_byte_has_independently_derived_metadata() {
    for raw in 0u8..=u8::MAX {
        if raw == 0x68 {
            assert_eq!(
                decode(raw),
                Err(Cdp1802Error::UnknownInstruction { raw: 0x68 })
            );
            continue;
        }
        let decoded = decode(raw).unwrap();
        assert_eq!(decoded.opcode, raw, "raw {raw:#04x}");
        assert_eq!(decoded.class, expected_class(raw), "raw {raw:#04x}");
        assert_eq!(decoded.width, expected_width(raw), "raw {raw:#04x}");
        assert_eq!(decoded.cycles, if raw >> 4 == 0xc { 3 } else { 2 });
        assert_eq!(decoded.mnemonic, expected_mnemonic(raw), "raw {raw:#04x}");
        let register_family = matches!(raw >> 4, 0..=2 | 4..=5 | 8..=0xb | 0xd..=0xe) && raw != 0;
        assert_eq!(
            decoded.register,
            register_family.then_some(raw & 0x0f),
            "raw {raw:#04x}"
        );
    }
}

#[test]
fn canonical_alias_names_are_stable() {
    assert_eq!(decode(0x76).unwrap().mnemonic, "SHRC");
    assert_eq!(decode(0x7e).unwrap().mnemonic, "SHLC");
    assert_eq!(decode(0x38).unwrap().mnemonic, "SKP");
    assert_eq!(decode(0xc8).unwrap().mnemonic, "LSKP");
}
