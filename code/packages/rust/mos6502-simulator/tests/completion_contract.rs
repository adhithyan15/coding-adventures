use mos6502_simulator::{opcodes, Mos6502Error, Mos6502Simulator, Mos6502State};

const PYTHON_ORACLE: [u64; 256] = [
    0xB6AA70397C0DA472,
    0x2813A1AAA6064938,
    0,
    0,
    0,
    0x0342EB3E52C9D812,
    0x25BF3E35CCF77D4D,
    0,
    0xBA3F98A4B1C4D719,
    0xEDDF00B3D54E5E82,
    0x2D6ED46801E4ADD0,
    0,
    0,
    0xB25813DE74D1F225,
    0xEE6E784966C06DDF,
    0,
    0x3B86C165B41A9BEA,
    0x97960BD007EC8408,
    0,
    0,
    0,
    0xBED82A0EF0F09526,
    0xFC41BD31EEA3B751,
    0,
    0x00FC36D79B8B4E9C,
    0x440AB948AFF3CD1B,
    0,
    0,
    0,
    0x1085C5DB4DC89C77,
    0x2097FC159848E983,
    0,
    0x3C0F531F7532725C,
    0x32BAB05CCE61806A,
    0,
    0,
    0x7DFDB7E9D1D49567,
    0x9AFA1C7B29987BA7,
    0xDB29660C68C6F134,
    0,
    0xC541F4CA5634DAE6,
    0x0A2A4E1987024B71,
    0x59026DAEE9B02739,
    0,
    0x06E721DDFE861707,
    0xEF7353B557555887,
    0x416ADE677F31233E,
    0,
    0x5703296B261652EA,
    0x9D1CFDE8E7CEF2FA,
    0,
    0,
    0,
    0x8323A8E95A0AD7CD,
    0x80E719C6E8169210,
    0,
    0x88A695DDB315D5FD,
    0xF951D28A7CDD400C,
    0,
    0,
    0,
    0x6C1DB3DEEF696208,
    0x69404FD13A908FC1,
    0,
    0x200B542534F34863,
    0xB569B69D6746D088,
    0,
    0,
    0,
    0x984B2EDCC01CEF43,
    0x478335A91724B7A7,
    0,
    0x02B6E4A80A6B5D61,
    0x0678AE4BA840CA42,
    0x4ADED1587CE92932,
    0,
    0x241854B922588A18,
    0xBA3F8AC94F16E675,
    0xF81C6F0E2F1181CF,
    0,
    0xAFA72B922F8D838A,
    0xEF3BC493A96193B8,
    0,
    0,
    0,
    0xE1B4C849BF7E29E6,
    0x7F3F328F9F9D2D81,
    0,
    0xBEC459BC4F509A9D,
    0xBF19129D2A80E25B,
    0,
    0,
    0,
    0x8B941F2FC855B1B7,
    0xA589A2C11DBBD443,
    0,
    0x25B1D8869ECC3B04,
    0xF103914E8F69689A,
    0,
    0,
    0,
    0x920300CE0F5EA6FA,
    0x9D0C79607B799886,
    0,
    0x95EDFE99B9348B40,
    0xC57097F6E9042C02,
    0xB18EEE7310D27443,
    0,
    0x1DFCF1CC9BCFA956,
    0xA844AB3613E30753,
    0xD1044FC7601EDC66,
    0,
    0x942AC38CBD91CC8A,
    0xFEED3EB62346668A,
    0,
    0,
    0,
    0x9669C15204A662A6,
    0x64B0EE3D983AC048,
    0,
    0x306297E34BE61CC4,
    0xB533F0B741945CF6,
    0,
    0,
    0,
    0xBB0B128A27952F1A,
    0x3381B3A041715D41,
    0,
    0,
    0xE21C5BCAC998C380,
    0,
    0,
    0x01FC109C87745FEE,
    0x56287B4286092392,
    0x52B1B84EF49EEC52,
    0,
    0xD373AB076ACE4D2E,
    0,
    0x7FC45E4F6C600960,
    0,
    0x7080C32A9C5F4BC9,
    0x1E801D4E4DB4B03D,
    0x29DB1E9F88A79581,
    0,
    0x292F51B15A1A1A4A,
    0xF0A349952CA7AFF0,
    0,
    0,
    0xA67FC9ED49CBFBE2,
    0x5B444CF8610BE75E,
    0x45846DDE063BCFFA,
    0,
    0x9D3DB24AF8238A20,
    0xA8BE6705BE0E8A0B,
    0xCE401B14ABB01076,
    0,
    0,
    0x35A674C7B658AE6F,
    0,
    0,
    0x81F68551CFB02A7E,
    0x064F4E0261EFFBC8,
    0x10737E89853BA9CA,
    0,
    0xE2C59F128A18AC2F,
    0xBF0AD4FAF43BB0F3,
    0x5C76521DF3A4A77B,
    0,
    0xDE183F59CC8C8865,
    0x8995252B9C5447C2,
    0x9BA1610B33DEA88B,
    0,
    0x2D653AF75E886569,
    0x8D898726491C5E45,
    0x2C3642B5637AF241,
    0,
    0x0DB2E9ABE81E634A,
    0x40215BF8A40ABEF8,
    0,
    0,
    0xEB42B48E84A16832,
    0x64D13F29B391A766,
    0x43C4C09FC846F115,
    0,
    0x4DF0C31ADE441392,
    0xF7CB9F552CDCC18C,
    0x090759113FAA8517,
    0,
    0xED38B1A3381BB6E0,
    0x6A9780A99F68E388,
    0x397E4085FE3260F8,
    0,
    0xA0374AF54BB0071A,
    0x98DBCA5B3B7032CB,
    0,
    0,
    0x11B094FD5E3368BF,
    0x87B3ED924C9035CF,
    0x46D1DA675EE7FB3A,
    0,
    0x57181252EB261F48,
    0x755AB8C5481F8853,
    0x7F17C49003F8B31A,
    0,
    0x3995E548BA15D6CF,
    0x68A00FBD5F6DA9F6,
    0xBBE7AF447033DC0A,
    0,
    0xBEF234DBE98D082A,
    0x0E6DA9BB91E8A17B,
    0,
    0,
    0,
    0x3891922AA3467E57,
    0x50423AA43D73280F,
    0,
    0x50D52BA51BF5578A,
    0xC09F45D27A8160DB,
    0,
    0,
    0,
    0xE3C5DA4974553237,
    0x405634A79A87FDB7,
    0,
    0x420CD2937119C562,
    0x195A64F088DECF8B,
    0,
    0,
    0x995C46B468E6075F,
    0xDB9F019C233045F6,
    0xFD95651A8DC60520,
    0,
    0xC7344D5C2C95F0BE,
    0xD46EE227F0BD97D3,
    0x93BA34471772569B,
    0,
    0xD2E31945EE70B6EF,
    0xE91EAA52ACDC46B6,
    0xBDA734F56729DFE8,
    0,
    0xDA6E9CE15B88BF2A,
    0x8EEC4450DF573E3B,
    0,
    0,
    0,
    0x97A5BB8D4BE48DD7,
    0xE4F676CCEDA6FFD1,
    0,
    0x03B4EF322DA2EB3D,
    0xD608AE23374383FC,
    0,
    0,
    0,
    0x5D2EF29705D987F8,
    0x27096F0DDEBAF040,
    0,
];

fn initialized() -> Mos6502Simulator {
    let mut sim = Mos6502Simulator::new(65_536);
    sim.a = 0x91;
    sim.x = 0x12;
    sim.y = 0x34;
    sim.s = 0xFD;
    sim.flag_v = true;
    sim.flag_d = true;
    sim.flag_i = false;
    sim.flag_c = true;
    sim.mem.write_byte(0x20, 0xEC);
    sim.mem.write_byte(0x21, 0x1F);
    sim.mem.write_byte(0x32, 0x20);
    sim.mem.write_byte(0x33, 0x20);
    sim.mem.write_byte(0x2020, 0x5A);
    sim.mem.write_byte(0x2021, 0x20);
    sim.mem.write_byte(0x01FE, 0x65);
    sim.mem.write_byte(0x01FF, 0x34);
    sim.mem.write_byte(0x0100, 0x12);
    sim
}

fn state_hash(state: &Mos6502State) -> u64 {
    let mut hash = 0xCBF29CE484222325u64;
    let mut feed = |byte: u8| {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x100000001B3);
    };
    for byte in [
        state.a,
        state.x,
        state.y,
        state.s,
        state.pc as u8,
        (state.pc >> 8) as u8,
        state.flag_n as u8,
        state.flag_v as u8,
        state.flag_b as u8,
        state.flag_d as u8,
        state.flag_i as u8,
        state.flag_z as u8,
        state.flag_c as u8,
        state.halted as u8,
    ] {
        feed(byte);
    }
    for &byte in &state.memory {
        feed(byte);
    }
    hash
}

#[test]
fn all_256_encodings_match_the_python_oracle() {
    assert_eq!(PYTHON_ORACLE.iter().filter(|&&hash| hash != 0).count(), 151);
    for opcode in 0u8..=u8::MAX {
        let mut sim = initialized();
        sim.load_program(&[opcode, 0x20, 0x20]).unwrap();
        let before = sim.snapshot();
        let expected_hash = PYTHON_ORACLE[opcode as usize];
        if expected_hash == 0 {
            assert_eq!(
                sim.step(),
                Err(Mos6502Error::UnknownOpcode { address: 0, opcode })
            );
            assert_eq!(sim.snapshot(), before, "opcode {opcode:#04X}");
            continue;
        }
        let trace = sim
            .step()
            .unwrap_or_else(|error| panic!("opcode {opcode:#04X}: {error}"));
        assert_eq!(trace.address, 0);
        assert_eq!(
            trace.raw.len(),
            opcodes::lookup(opcode).unwrap().1.instruction_len()
        );
        assert_eq!(trace.state_before, before);
        assert_eq!(trace.state_after, sim.snapshot());
        assert_eq!(
            state_hash(&trace.state_after),
            expected_hash,
            "opcode {opcode:#04X}"
        );
    }
}

#[test]
fn typed_failures_and_transactional_runs_preserve_state() {
    let mut sim = initialized();
    sim.set_input_port(4, 0xA5).unwrap();
    let before = sim.snapshot();
    assert_eq!(
        sim.load_program(&vec![0; 65_537]),
        Err(Mos6502Error::ProgramTooLarge {
            length: 65_537,
            capacity: 65_536,
        })
    );
    assert_eq!(sim.snapshot(), before);
    assert_eq!(
        sim.run(&[0xA9, 0x42, 0x02]),
        Err(Mos6502Error::UnknownOpcode {
            address: 2,
            opcode: 0x02,
        })
    );
    assert_eq!(sim.snapshot(), before);

    sim.load_program(&[0x00]).unwrap();
    sim.step().unwrap();
    let halted = sim.snapshot();
    assert_eq!(sim.step(), Err(Mos6502Error::Halted));
    assert_eq!(sim.snapshot(), halted);
    assert_eq!(
        sim.set_input_port(240, 0),
        Err(Mos6502Error::InvalidPort { port: 240 })
    );
    assert_eq!(
        sim.get_output_port(255),
        Err(Mos6502Error::InvalidPort { port: 255 })
    );
}

#[test]
fn wraparound_fetch_and_memory_mapped_io_are_observable() {
    let mut wrapping = Mos6502Simulator::new(1);
    wrapping.load_program_at(&[0xA9, 0x42], 0xFFFF).unwrap();
    let trace = wrapping.step().unwrap();
    assert_eq!(trace.raw, [0xA9, 0x42]);
    assert_eq!(trace.state_after.a, 0x42);
    assert_eq!(trace.state_after.pc, 1);

    let mut io = Mos6502Simulator::new(65_536);
    io.set_input_port(5, 0xCD).unwrap();
    let result = io.run(&[0xAD, 0x05, 0xFF, 0x8D, 0x0A, 0xFF, 0x00]).unwrap();
    assert_eq!(result.final_state.a, 0xCD);
    assert_eq!(io.get_output_port(10), Ok(0xCD));
    assert_eq!(result.final_state.memory[0xFF0A], 0);
}
