use m68k_simulator::{M68kSimulator, M68kState};
use motorola68020_simulator::{
    Mc68020Error, Mc68020Simulator, Mc68020State, INITIAL_SP, MEMORY_SIZE,
};

const ORIGIN: u32 = 0x1000;

fn move_l(src_mode: u16, src_reg: u16, dst_mode: u16, dst_reg: u16) -> [u8; 2] {
    (0x2000 | dst_reg << 9 | dst_mode << 6 | src_mode << 3 | src_reg).to_be_bytes()
}

fn configured(program: &[u8]) -> Mc68020Simulator {
    let mut cpu = Mc68020Simulator::new();
    cpu.load(program, ORIGIN).unwrap();
    cpu
}

fn set_state(cpu: &mut Mc68020Simulator, edit: impl FnOnce(&mut Mc68020State)) {
    let mut state = cpu.state();
    edit(&mut state);
    cpu.restore(&state).unwrap();
}

#[test]
fn lifecycle_state_banks_transport_and_helpers_are_checked() {
    let mut cpu = Mc68020Simulator::new();
    let original = cpu.state();
    assert_eq!(cpu.load(&[], ORIGIN), Err(Mc68020Error::EmptyProgram));
    assert_eq!(cpu.state(), original);
    assert_eq!(
        cpu.load(&[0; 2], 1),
        Err(Mc68020Error::MisalignedProgram { origin: 1 })
    );
    assert!(matches!(
        cpu.load(&[0; 4], MEMORY_SIZE as u32 - 2),
        Err(Mc68020Error::ProgramOutOfRange { .. })
    ));
    cpu.load(&[0x4e, 0x4f], ORIGIN).unwrap();
    assert_eq!(cpu.state().installed_origin, ORIGIN);
    assert_eq!(cpu.state().installed_bytes, 2);
    assert_eq!(cpu.address_register(7).unwrap(), INITIAL_SP);
    assert!(matches!(
        cpu.data_register(8),
        Err(Mc68020Error::InvalidRegister { .. })
    ));
    assert!(matches!(
        cpu.set_address_register(9, 0),
        Err(Mc68020Error::InvalidRegister { .. })
    ));
    cpu.write_byte(3, 0xaa).unwrap();
    assert_eq!(cpu.read_byte(3).unwrap(), 0xaa);
    cpu.write_word(3, 0x1234).unwrap();
    assert_eq!(cpu.read_word(3).unwrap(), 0x1234);
    cpu.write_long(5, 0x89ab_cdef).unwrap();
    assert_eq!(cpu.read_long(5).unwrap(), 0x89ab_cdef);
    let before = cpu.state();
    assert!(matches!(
        cpu.write_long(MEMORY_SIZE as u32 - 3, 0xfeed_face),
        Err(Mc68020Error::StorageOutOfRange { .. })
    ));
    assert_eq!(cpu.state(), before);
    assert!(matches!(
        cpu.read_byte(MEMORY_SIZE as u32),
        Err(Mc68020Error::StorageOutOfRange { .. })
    ));

    let mut bad = cpu.state();
    bad.memory.pop();
    assert!(matches!(
        cpu.restore(&bad),
        Err(Mc68020Error::InvalidState(_))
    ));
    let mut bad = cpu.state();
    bad.a[7] ^= 4;
    assert!(matches!(
        cpu.restore(&bad),
        Err(Mc68020Error::InvalidState(_))
    ));

    set_state(&mut cpu, |state| {
        state.sr = 0x2000;
        state.a[7] = 0x2000;
        state.isp = 0x2000;
    });
    cpu.set_address_register(7, 0x2222).unwrap();
    assert_eq!(cpu.state().isp, 0x2222);
    set_state(&mut cpu, |state| {
        state.sr = 0x3000;
        state.a[7] = 0x3000;
        state.msp = 0x3000;
    });
    cpu.set_address_register(7, 0x3333).unwrap();
    assert_eq!(cpu.state().msp, 0x3333);
}

#[test]
fn exact_fetch_halt_and_transactional_run_contract() {
    let mut cpu = configured(&[0x4e, 0x4f]);
    let trace = cpu.step().unwrap();
    assert_eq!(trace.raw, [0x4e, 0x4f]);
    assert_eq!(trace.width, 2);
    assert_eq!(trace.mnemonic, "TRAP #15");
    assert!(cpu.state().halted);
    let halted = cpu.state();
    assert_eq!(cpu.step(), Err(Mc68020Error::Halted));
    assert_eq!(cpu.state(), halted);

    let mut cpu = configured(&[0x70]);
    let before = cpu.state();
    assert!(matches!(
        cpu.step(),
        Err(Mc68020Error::TruncatedInstruction { .. })
    ));
    assert_eq!(cpu.state(), before);
    let mut cpu = configured(&[0x20, 0x3c]);
    assert!(matches!(
        cpu.step(),
        Err(Mc68020Error::TruncatedInstruction {
            needed: 4,
            available: 2,
            ..
        })
    ));
    let mut cpu = configured(&[0x70, 0x01]);
    set_state(&mut cpu, |state| state.pc = ORIGIN + 2);
    assert!(matches!(
        cpu.step(),
        Err(Mc68020Error::FetchOutsideProgram { .. })
    ));
    let mut cpu = configured(&[0x70, 0x01]);
    set_state(&mut cpu, |state| state.pc = ORIGIN + 1);
    assert!(matches!(
        cpu.step(),
        Err(Mc68020Error::MisalignedFetch { .. })
    ));

    let mut cpu = Mc68020Simulator::new();
    let entry = cpu.state();
    assert_eq!(
        cpu.execute(&[0x70, 0x01, 0xff, 0xff], ORIGIN, 3),
        Err(Mc68020Error::UnknownOpcode { opcode: 0xffff })
    );
    assert_eq!(cpu.state(), entry);
    assert_eq!(cpu.run(0), Err(Mc68020Error::MaxStepsZero));
    assert_eq!(
        cpu.execute(&[0x4e, 0x71], ORIGIN, 1),
        Err(Mc68020Error::MaxStepsExceeded { max_steps: 1 })
    );
    assert_eq!(cpu.state(), entry);
}

#[test]
fn inherited_register_subset_is_in_complete_state_lockstep() {
    let program = [
        0x4e, 0x71, // NOP
        0x70, 0x80, // MOVEQ #-128,D0
        0x48, 0x40, // SWAP D0
        0x48, 0x80, // EXT.W D0
        0x48, 0xc0, // EXT.L D0
        0x52, 0x40, // ADDQ.W #1,D0
        0x53, 0x80, // SUBQ.L #1,D0
    ];
    let mut cpu = configured(&program);
    let mut inherited = M68kSimulator::architectural();
    let mut memory = vec![0; MEMORY_SIZE];
    memory[ORIGIN as usize..ORIGIN as usize + program.len()].copy_from_slice(&program);
    inherited
        .restore(&M68kState {
            d: [0; 8],
            a: [0, 0, 0, 0, 0, 0, 0, INITIAL_SP],
            pc: ORIGIN,
            sr: 0,
            halted: false,
            memory,
        })
        .unwrap();
    for _ in 0..7 {
        let ours = cpu.step().unwrap();
        let theirs = inherited.step_checked().unwrap();
        let state = cpu.state();
        let reference = inherited.get_state();
        assert_eq!(state.d, reference.d, "{}", ours.mnemonic);
        assert_eq!(state.a, reference.a, "{}", ours.mnemonic);
        assert_eq!(state.pc, reference.pc, "{}", ours.mnemonic);
        assert_eq!(
            state.sr, reference.sr,
            "{} / {}",
            ours.mnemonic, theirs.mnemonic
        );
        assert_eq!(state.memory, reference.memory);
    }
}

#[test]
fn move_long_register_immediate_flags_and_movea() {
    let mut program = Vec::new();
    program.extend(move_l(7, 4, 0, 0));
    program.extend(0x8000_0000u32.to_be_bytes());
    program.extend(move_l(0, 0, 1, 1));
    program.extend(move_l(7, 4, 0, 2));
    program.extend(0u32.to_be_bytes());
    program.extend(move_l(1, 3, 0, 4));
    let mut cpu = configured(&program);
    set_state(&mut cpu, |state| state.sr = 0x11);
    let trace = cpu.step().unwrap();
    assert_eq!(trace.mnemonic, "MOVE.L");
    assert_eq!(cpu.state().d[0], 0x8000_0000);
    assert!(cpu.state().x());
    assert!(cpu.state().n());
    assert!(!cpu.state().z());
    assert!(!cpu.state().v());
    assert!(!cpu.state().c());
    let sr = cpu.state().sr;
    assert_eq!(cpu.step().unwrap().mnemonic, "MOVEA.L");
    assert_eq!(cpu.state().a[1], 0x8000_0000);
    assert_eq!(cpu.state().sr, sr);
    cpu.step().unwrap();
    assert_eq!(cpu.state().d[2], 0);
    assert!(cpu.state().z());
    set_state(&mut cpu, |state| state.a[3] = 0x1234_5678);
    cpu.step().unwrap();
    assert_eq!(cpu.state().d[4], 0x1234_5678);
}

#[test]
fn move_long_compatible_memory_source_modes() {
    // (A0), (A0)+, -(A0), d16(A0), d8(A0,D1.L*2)
    let encodings = [
        (move_l(2, 0, 0, 0).to_vec(), 0x2000, 0x2000),
        (move_l(3, 0, 0, 0).to_vec(), 0x2000, 0x2000),
        (move_l(4, 0, 0, 0).to_vec(), 0x2004, 0x2000),
        (
            {
                let mut p = move_l(5, 0, 0, 0).to_vec();
                p.extend(4u16.to_be_bytes());
                p
            },
            0x1ffc,
            0x2000,
        ),
        (
            {
                let mut p = move_l(6, 0, 0, 0).to_vec();
                p.extend(0x1a04u16.to_be_bytes());
                p
            },
            0x1ff8,
            0x2000,
        ),
    ];
    for (program, a0, address) in encodings {
        let mut cpu = configured(&program);
        cpu.write_long(address, 0x1122_3344).unwrap();
        set_state(&mut cpu, |state| {
            state.a[0] = a0;
            state.d[1] = 2;
        });
        cpu.step().unwrap();
        assert_eq!(cpu.state().d[0], 0x1122_3344);
    }
    // Absolute word, absolute long, PC displacement, immediate.
    let sources = [
        {
            let mut p = move_l(7, 0, 0, 0).to_vec();
            p.extend(0x2000u16.to_be_bytes());
            p
        },
        {
            let mut p = move_l(7, 1, 0, 0).to_vec();
            p.extend(0x0000_2000u32.to_be_bytes());
            p
        },
        {
            let mut p = move_l(7, 2, 0, 0).to_vec();
            p.extend(0x0ffeu16.to_be_bytes());
            p
        },
    ];
    for program in sources {
        let mut cpu = configured(&program);
        cpu.write_long(0x2000, 0x5566_7788).unwrap();
        cpu.step().unwrap();
        assert_eq!(cpu.state().d[0], 0x5566_7788);
    }
}

#[test]
fn brief_index_scales_and_pc_base_are_exact() {
    for (scale_bits, scale) in [(0, 1), (1, 2), (2, 4), (3, 8)] {
        let mut program = move_l(6, 0, 0, 0).to_vec();
        let extension = 0x1800 | (scale_bits << 9) | 0x04; // D1.L*scale + 4
        program.extend((extension as u16).to_be_bytes());
        let mut cpu = configured(&program);
        let address = 0x2000 + 2 * scale + 4;
        cpu.write_long(address, scale).unwrap();
        set_state(&mut cpu, |state| {
            state.a[0] = 0x2000;
            state.d[1] = 2;
        });
        cpu.step().unwrap();
        assert_eq!(cpu.state().d[0], scale);
    }
    let mut program = move_l(7, 3, 0, 0).to_vec();
    program.extend(0x1804u16.to_be_bytes()); // PC(extension address)+D1.L+4
    let mut cpu = configured(&program);
    cpu.write_long(ORIGIN + 2 + 2 + 4, 0xaabb_ccdd).unwrap();
    set_state(&mut cpu, |state| state.d[1] = 2);
    cpu.step().unwrap();
    assert_eq!(cpu.state().d[0], 0xaabb_ccdd);
}

#[test]
fn full_extension_suppression_displacements_and_indirection() {
    // Base active, D1.L index, word BD, no indirection.
    let mut program = move_l(6, 0, 0, 0).to_vec();
    program.extend(0x1920u16.to_be_bytes());
    program.extend(6u16.to_be_bytes());
    let mut cpu = configured(&program);
    cpu.write_long(0x2010, 0x1111_2222).unwrap();
    set_state(&mut cpu, |state| {
        state.a[0] = 0x2000;
        state.d[1] = 10;
    });
    cpu.step().unwrap();
    assert_eq!(cpu.state().d[0], 0x1111_2222);

    // Base and index suppressed, long BD supplies the active term.
    let mut program = move_l(6, 0, 0, 0).to_vec();
    program.extend(0x19f0u16.to_be_bytes());
    program.extend(0x0000_3000u32.to_be_bytes());
    let mut cpu = configured(&program);
    cpu.write_long(0x3000, 0x3333_4444).unwrap();
    cpu.step().unwrap();
    assert_eq!(cpu.state().d[0], 0x3333_4444);

    // Preindexed: ([A0 + word BD + D1.L] + long OD).
    let mut program = move_l(6, 0, 0, 0).to_vec();
    program.extend(0x1923u16.to_be_bytes());
    program.extend(4u16.to_be_bytes());
    program.extend(8u32.to_be_bytes());
    let mut cpu = configured(&program);
    cpu.write_long(0x200a, 0x3000).unwrap();
    cpu.write_long(0x3008, 0x5555_6666).unwrap();
    set_state(&mut cpu, |state| {
        state.a[0] = 0x2000;
        state.d[1] = 6;
    });
    cpu.step().unwrap();
    assert_eq!(cpu.state().d[0], 0x5555_6666);

    // Postindexed: ([A0 + long BD] + D1.L + word OD).
    let mut program = move_l(6, 0, 0, 0).to_vec();
    program.extend(0x1936u16.to_be_bytes());
    program.extend(4u32.to_be_bytes());
    program.extend(6u16.to_be_bytes());
    let mut cpu = configured(&program);
    cpu.write_long(0x2004, 0x3000).unwrap();
    cpu.write_long(0x300a, 0x7777_8888).unwrap();
    set_state(&mut cpu, |state| {
        state.a[0] = 0x2000;
        state.d[1] = 4;
    });
    cpu.step().unwrap();
    assert_eq!(cpu.state().d[0], 0x7777_8888);

    // Null BD and null OD, with index suppressed: ([A0]).
    let mut program = move_l(6, 0, 0, 0).to_vec();
    program.extend(0x19_51u16.to_be_bytes());
    let mut cpu = configured(&program);
    cpu.write_long(0x2000, 0x3000).unwrap();
    cpu.write_long(0x3000, 0x9999_aaaa).unwrap();
    set_state(&mut cpu, |state| state.a[0] = 0x2000);
    cpu.step().unwrap();
    assert_eq!(cpu.state().d[0], 0x9999_aaaa);

    // PC full format uses the extension-word address as its unsuppressed base.
    let mut program = move_l(7, 3, 0, 0).to_vec();
    program.extend(0x19_60u16.to_be_bytes()); // index suppressed, word BD
    program.extend(0x0ffeu16.to_be_bytes());
    let mut cpu = configured(&program);
    cpu.write_long(0x2000, 0xbbbb_cccc).unwrap();
    cpu.step().unwrap();
    assert_eq!(cpu.state().d[0], 0xbbbb_cccc);

    // Address arithmetic wraps at 32 bits before the bounded transfer check.
    let mut program = move_l(5, 0, 0, 0).to_vec();
    program.extend(0x0020u16.to_be_bytes());
    let mut cpu = configured(&program);
    cpu.write_long(0x10, 0xdddd_eeee).unwrap();
    set_state(&mut cpu, |state| state.a[0] = 0xffff_fff0);
    cpu.step().unwrap();
    assert_eq!(cpu.state().d[0], 0xdddd_eeee);
}

#[test]
fn invalid_full_extensions_and_candidate_updates_roll_back() {
    let invalid = [0x19, 0xc0]; // full, base/index suppressed, null BD, no indirect
    let mut program = move_l(6, 0, 0, 0).to_vec();
    program.extend(invalid);
    let mut cpu = configured(&program);
    let before = cpu.state();
    assert!(matches!(
        cpu.step(),
        Err(Mc68020Error::InvalidEffectiveAddress { .. })
    ));
    assert_eq!(cpu.state(), before);

    let mut program = move_l(3, 0, 3, 1).to_vec(); // (A0)+ -> (A1)+, range failure
    program.extend([0x4e, 0x4f]);
    let mut cpu = configured(&program);
    cpu.write_long(0x2000, 42).unwrap();
    set_state(&mut cpu, |state| {
        state.a[0] = 0x2000;
        state.a[1] = MEMORY_SIZE as u32 - 3;
    });
    let before = cpu.state();
    assert!(matches!(
        cpu.step(),
        Err(Mc68020Error::StorageOutOfRange { .. })
    ));
    assert_eq!(cpu.state(), before);
}

#[test]
fn unaligned_data_sources_destinations_and_indirect_pointers_are_architectural() {
    let mut cpu = configured(&move_l(2, 0, 0, 0)); // MOVE.L (A0),D0
    cpu.write_long(0x2001, 0x1122_3344).unwrap();
    set_state(&mut cpu, |state| state.a[0] = 0x2001);
    cpu.step().unwrap();
    assert_eq!(cpu.state().d[0], 0x1122_3344);

    let mut cpu = configured(&move_l(0, 0, 2, 1)); // MOVE.L D0,(A1)
    set_state(&mut cpu, |state| {
        state.d[0] = 0x5566_7788;
        state.a[1] = 0x2101;
    });
    cpu.step().unwrap();
    assert_eq!(cpu.read_long(0x2101).unwrap(), 0x5566_7788);

    // Full preindexed ([A0 + D1.L]) reads its pointer at odd 0x2201, then
    // reads the final longword at odd 0x3001.
    let mut program = move_l(6, 0, 0, 0).to_vec();
    program.extend(0x1911u16.to_be_bytes());
    let mut cpu = configured(&program);
    cpu.write_long(0x2201, 0x3001).unwrap();
    cpu.write_long(0x3001, 0x99aa_bbcc).unwrap();
    set_state(&mut cpu, |state| {
        state.a[0] = 0x2200;
        state.d[1] = 1;
    });
    cpu.step().unwrap();
    assert_eq!(cpu.state().d[0], 0x99aa_bbcc);
}

#[test]
fn full_extension_index_suppression_rejects_every_reserved_iis_and_keeps_neighbors() {
    // Motorola reserves IS=1 with I/IS=100-111. Rejection must happen from
    // the extension word itself, before a missing BD/OD could be fetched.
    for iis in 4u16..=7 {
        let mut program = move_l(6, 0, 0, 0).to_vec();
        program.extend((0x1950 | iis).to_be_bytes());
        let mut cpu = configured(&program);
        let before = cpu.state();
        assert!(matches!(
            cpu.step(),
            Err(Mc68020Error::InvalidEffectiveAddress { .. })
        ));
        assert_eq!(cpu.state(), before, "reserved I/IS {iis:03b}");
    }

    // IS=1 remains legal for direct and preindexed I/IS=000-011.
    for (iis, outer) in [
        (0u16, Vec::new()),
        (1, Vec::new()),
        (2, 4u16.to_be_bytes().to_vec()),
        (3, 4u32.to_be_bytes().to_vec()),
    ] {
        let mut program = move_l(6, 0, 0, 0).to_vec();
        program.extend((0x1950 | iis).to_be_bytes());
        program.extend(outer);
        let mut cpu = configured(&program);
        set_state(&mut cpu, |state| state.a[0] = 0x2000);
        if iis == 0 {
            cpu.write_long(0x2000, 0x1111_0000 | u32::from(iis))
                .unwrap();
        } else {
            cpu.write_long(0x2000, 0x3000).unwrap();
            cpu.write_long(
                0x3000 + if iis >= 2 { 4 } else { 0 },
                0x1111_0000 | u32::from(iis),
            )
            .unwrap();
        }
        cpu.step().unwrap();
        assert_eq!(cpu.state().d[0], 0x1111_0000 | u32::from(iis));
    }

    // With IS=0, postindexed I/IS=101-111 are the legal neighbors.
    for (iis, outer) in [
        (5u16, Vec::new()),
        (6, 4u16.to_be_bytes().to_vec()),
        (7, 4u32.to_be_bytes().to_vec()),
    ] {
        let mut program = move_l(6, 0, 0, 0).to_vec();
        program.extend((0x1910 | iis).to_be_bytes());
        program.extend(outer);
        let mut cpu = configured(&program);
        set_state(&mut cpu, |state| {
            state.a[0] = 0x2000;
            state.d[1] = 8;
        });
        cpu.write_long(0x2000, 0x3000).unwrap();
        let address = 0x3008 + if iis >= 6 { 4 } else { 0 };
        cpu.write_long(address, 0x2222_0000 | u32::from(iis))
            .unwrap();
        cpu.step().unwrap();
        assert_eq!(cpu.state().d[0], 0x2222_0000 | u32::from(iis));
    }
}

#[test]
fn self_modifying_move_trace_keeps_the_bytes_fetched_before_the_store() {
    let mut program = move_l(0, 0, 7, 1).to_vec(); // MOVE.L D0,(abs.l)
    program.extend(ORIGIN.to_be_bytes());
    let original = program.clone();
    let mut cpu = configured(&program);
    set_state(&mut cpu, |state| state.d[0] = 0x4e71_4e71);

    let trace = cpu.step().unwrap();

    assert_eq!(trace.raw, original);
    assert_eq!(trace.width, 6);
    assert_eq!(trace.mnemonic, "MOVE.L");
    assert_eq!(cpu.read_long(ORIGIN).unwrap(), 0x4e71_4e71);
    assert_eq!(
        trace.state_after.memory[ORIGIN as usize..ORIGIN as usize + 4],
        [0x4e, 0x71, 0x4e, 0x71]
    );
}

#[test]
fn full_extension_destination_writes_through_resolved_pointer() {
    let mut program = move_l(0, 0, 6, 1).to_vec();
    program.extend(0x1925u16.to_be_bytes()); // ([word BD,A1],D1.L,null OD)
    program.extend(4u16.to_be_bytes());
    let mut cpu = configured(&program);
    cpu.write_long(0x2004, 0x3000).unwrap();
    set_state(&mut cpu, |state| {
        state.d[0] = 0xfeed_face;
        state.d[1] = 8;
        state.a[1] = 0x2000;
    });
    cpu.step().unwrap();
    assert_eq!(cpu.read_long(0x3008).unwrap(), 0xfeed_face);
}

#[test]
fn move_destinations_update_once_and_preserve_alias_order() {
    let destinations = [
        (2, 0, Vec::new(), 0x2000),
        (3, 0, Vec::new(), 0x2000),
        (4, 0, Vec::new(), 0x1ffc),
        (5, 0, 4u16.to_be_bytes().to_vec(), 0x2004),
        (6, 0, 0x2804u16.to_be_bytes().to_vec(), 0x2006),
        (7, 0, 0x2000u16.to_be_bytes().to_vec(), 0x2000),
        (7, 1, 0x0000_2000u32.to_be_bytes().to_vec(), 0x2000),
    ];
    for (mode, register, extension, address) in destinations {
        let mut program = move_l(0, 1, mode, register).to_vec();
        program.extend(extension);
        let mut cpu = configured(&program);
        set_state(&mut cpu, |state| {
            state.d[1] = 0xdead_beef;
            state.d[2] = 2;
            state.a[0] = 0x2000;
        });
        cpu.step().unwrap();
        assert_eq!(cpu.read_long(address).unwrap(), 0xdead_beef);
    }
    let mut cpu = configured(&move_l(3, 0, 3, 0)); // (A0)+ -> (A0)+
    cpu.write_long(0x2000, 0xcafe_babe).unwrap();
    set_state(&mut cpu, |state| state.a[0] = 0x2000);
    cpu.step().unwrap();
    assert_eq!(cpu.read_long(0x2004).unwrap(), 0xcafe_babe);
    assert_eq!(cpu.state().a[0], 0x2008);
}

#[test]
fn byte_word_long_branches_conditions_and_bsr_stack() {
    for (program, target) in [
        (vec![0x60, 0x02, 0x4e, 0x71], ORIGIN + 4),
        (vec![0x60, 0x00, 0x00, 0x04, 0x4e, 0x71], ORIGIN + 6),
        (
            vec![0x60, 0xff, 0x00, 0x00, 0x00, 0x06, 0x4e, 0x71],
            ORIGIN + 8,
        ),
    ] {
        let mut cpu = configured(&program);
        cpu.step().unwrap();
        assert_eq!(cpu.state().pc, target);
    }
    let mut cpu = configured(&[0x61, 0xff, 0, 0, 0, 6, 0x4e, 0x71]);
    cpu.step().unwrap();
    assert_eq!(cpu.state().pc, ORIGIN + 8);
    assert_eq!(cpu.state().a[7], INITIAL_SP - 4);
    assert_eq!(cpu.read_long(INITIAL_SP - 4).unwrap(), ORIGIN + 6);

    let mut cpu = configured(&[0x60, 0xfe]); // branch back to the opcode itself
    cpu.step().unwrap();
    assert_eq!(cpu.state().pc, ORIGIN);

    // BEQ false consumes the word extension; true branches from opcode+2.
    let mut cpu = configured(&[0x67, 0x00, 0x00, 0x06, 0x4e, 0x71, 0x4e, 0x71]);
    cpu.step().unwrap();
    assert_eq!(cpu.state().pc, ORIGIN + 4);
    let mut cpu = configured(&[0x67, 0x00, 0x00, 0x06, 0x4e, 0x71, 0x4e, 0x71]);
    set_state(&mut cpu, |state| state.sr |= 4);
    cpu.step().unwrap();
    assert_eq!(cpu.state().pc, ORIGIN + 8);
}

#[test]
fn rtd_link_long_and_extb_have_exact_stack_and_flags() {
    let mut cpu = configured(&[0x4e, 0x74, 0xff, 0xfc]);
    cpu.write_long(INITIAL_SP, 0x1234).unwrap();
    cpu.step().unwrap();
    assert_eq!(cpu.state().pc, 0x1234);
    assert_eq!(cpu.state().a[7], INITIAL_SP);

    let mut cpu = configured(&[0x48, 0x09, 0xff, 0xff, 0xff, 0xf0]);
    set_state(&mut cpu, |state| state.a[1] = 0xaaaa_bbbb);
    cpu.step().unwrap();
    assert_eq!(cpu.read_long(INITIAL_SP - 4).unwrap(), 0xaaaa_bbbb);
    assert_eq!(cpu.state().a[1], INITIAL_SP - 4);
    assert_eq!(cpu.state().a[7], INITIAL_SP - 20);

    // LINK.L A7 follows the manual's ordered micro-operations: decrement SP,
    // then save the now-current A7 value.
    let mut cpu = configured(&[0x48, 0x0f, 0xff, 0xff, 0xff, 0xf8]);
    cpu.step().unwrap();
    assert_eq!(cpu.read_long(INITIAL_SP - 4).unwrap(), INITIAL_SP - 4);
    assert_eq!(cpu.state().a[7], INITIAL_SP - 12);

    let mut cpu = configured(&[0x49, 0xc2]);
    set_state(&mut cpu, |state| {
        state.d[2] = 0x1234_5680;
        state.sr = 0x17;
    });
    cpu.step().unwrap();
    assert_eq!(cpu.state().d[2], 0xffff_ff80);
    assert!(cpu.state().x() && cpu.state().n());
    assert!(!cpu.state().z());
    assert!(!cpu.state().v());
    assert!(!cpu.state().c());
}

#[test]
fn trapcc_forms_consume_when_false_and_trap_atomically_when_true() {
    for (form, extension, width) in [
        (4u16, vec![], 2u32),
        (2, vec![0x12, 0x34], 4),
        (3, vec![0x12, 0x34, 0x56, 0x78], 6),
    ] {
        let opcode = 0x50f8 | (1 << 8) | form; // TRAPF
        let mut program = opcode.to_be_bytes().to_vec();
        program.extend(extension);
        let mut cpu = configured(&program);
        let trace = cpu.step().unwrap();
        assert_eq!(trace.width, width);
        assert_eq!(trace.mnemonic, "TRAPCC");
        assert_eq!(cpu.state().pc, ORIGIN + width);
    }
    let opcode: u16 = 0x50fc; // TRAPT
    let mut cpu = configured(&opcode.to_be_bytes());
    let before = cpu.state();
    assert_eq!(
        cpu.step(),
        Err(Mc68020Error::Trap {
            condition: 0,
            pc: ORIGIN
        })
    );
    assert_eq!(cpu.state(), before);
}

#[test]
fn privileged_stop_and_bad_destinations_fail_closed() {
    let mut cpu = configured(&[0x4e, 0x72, 0x27, 0x00]);
    let before = cpu.state();
    assert!(matches!(
        cpu.step(),
        Err(Mc68020Error::UnsupportedOpcode { .. })
    ));
    assert_eq!(cpu.state(), before);
    for opcode in [0x4e70u16, 0x4e73, 0x4e7a, 0x4e40, 0x007c] {
        let mut cpu = configured(&opcode.to_be_bytes());
        assert!(matches!(
            cpu.step(),
            Err(Mc68020Error::UnsupportedOpcode { .. })
        ));
    }
    let mut program = move_l(0, 0, 7, 2).to_vec();
    program.extend([0, 0]);
    let mut cpu = configured(&program);
    assert!(matches!(
        cpu.step(),
        Err(Mc68020Error::InvalidEffectiveAddress { .. })
    ));
}
