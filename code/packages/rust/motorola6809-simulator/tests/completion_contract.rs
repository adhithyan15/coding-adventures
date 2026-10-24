use motorola6809_simulator::{
    indexed_postbyte_is_valid, opcode_is_documented, Motorola6809Error, Motorola6809Simulator,
    CC_C, CC_E, CC_F, CC_H, CC_I, CC_N, CC_V, CC_Z, MAX_RETAINED_STEPS, MEMORY_SIZE,
};

fn image(parts: &[&[u8]]) -> Vec<u8> {
    parts.iter().flat_map(|part| part.iter().copied()).collect()
}

#[test]
fn opcode_pages_and_indexed_postbytes_have_exact_classification() {
    assert_eq!(
        (0_u8..=u8::MAX)
            .filter(|opcode| opcode_is_documented(0, *opcode))
            .count(),
        223
    );
    assert_eq!(
        (0_u8..=u8::MAX)
            .filter(|opcode| opcode_is_documented(0x10, *opcode))
            .count(),
        38
    );
    assert_eq!(
        (0_u8..=u8::MAX)
            .filter(|opcode| opcode_is_documented(0x11, *opcode))
            .count(),
        9
    );
    assert_eq!(
        (0_u8..=u8::MAX)
            .filter(|postbyte| indexed_postbyte_is_valid(*postbyte))
            .count(),
        217
    );
}

#[test]
fn every_documented_encoding_executes_from_a_nontrivial_state() {
    for page in [0_u8, 0x10, 0x11] {
        for opcode in 0_u8..=u8::MAX {
            if !opcode_is_documented(page, opcode) || (page == 0 && matches!(opcode, 0x10 | 0x11)) {
                continue;
            }
            let mut program = if page == 0 {
                vec![opcode, 0, 0, 0, 0, 0, 0x13]
            } else {
                vec![page, opcode, 0, 0, 0, 0, 0, 0x13]
            };
            let mut cpu = Motorola6809Simulator::new();
            cpu.load(&program, 0).unwrap();
            cpu.write_a(0x12);
            cpu.write_b(0x34);
            cpu.write_x(0x2000);
            cpu.write_y(0x2100);
            cpu.write_u(0x3000);
            cpu.write_s(0x4000);
            cpu.write_memory(0xfff2, &[0, 0]).unwrap();
            cpu.write_memory(0xfff4, &[0, 0]).unwrap();
            cpu.write_memory(0xfffa, &[0, 0]).unwrap();
            cpu.step().unwrap_or_else(|error| {
                panic!("page {page:#04x} opcode {opcode:#04x} failed: {error}")
            });
            program.clear();
        }
    }
}

#[test]
fn load_restore_registers_and_memory_are_complete_and_checked() {
    let mut cpu = Motorola6809Simulator::new();
    cpu.load(&[0x12, 0x13], 0x1000).unwrap();
    cpu.write_a(0x12);
    cpu.write_b(0x34);
    cpu.write_x(0x4567);
    cpu.write_y(0x89ab);
    cpu.write_u(0xcdef);
    cpu.write_s(0x0200);
    cpu.write_dp(0x40);
    cpu.write_cc(CC_E | CC_F | CC_H | CC_I | CC_N | CC_Z | CC_V | CC_C);
    cpu.write_memory(0x4000, &[1, 2, 3]).unwrap();
    let state = cpu.get_state();
    assert_eq!(state.d(), 0x1234);
    assert_eq!(state.memory.len(), MEMORY_SIZE);
    assert_eq!((state.loaded_origin, state.loaded_len), (0x1000, 2));
    assert_eq!(cpu.read_memory(0x4000, 3).unwrap(), vec![1, 2, 3]);

    cpu.reset();
    cpu.restore(state.clone()).unwrap();
    assert_eq!(cpu.get_state(), state);
    let before = cpu.get_state();
    assert!(cpu.write_memory(0xffff, &[1, 2]).is_err());
    assert_eq!(cpu.get_state(), before);
}

#[test]
fn arithmetic_logic_and_direct_page_paths_follow_the_manual() {
    let program = image(&[
        &[0x86, 0x7f],       // LDA #$7f
        &[0xc6, 0x01],       // LDB #1
        &[0xcb, 0x01],       // ADDB #1
        &[0xc3, 0x00, 0x10], // ADDD #$0010
        &[0x97, 0x20],       // STA <$20
        &[0xd7, 0x21],       // STB <$21
        &[0x3d],             // MUL
        &[0x13],             // SYNC
    ]);
    let mut cpu = Motorola6809Simulator::new();
    cpu.load(&program, 0).unwrap();
    cpu.write_dp(0x40);
    cpu.run(32).unwrap();
    let state = cpu.get_state();
    assert!(state.halted);
    assert_eq!(cpu.read_memory(0x4020, 2).unwrap(), vec![0x7f, 0x12]);
    assert_ne!(state.cc & CC_C, 0);
}

#[test]
fn every_documented_indexed_form_resolves_or_fails_only_for_missing_bytes() {
    for postbyte in 0_u8..=u8::MAX {
        let mut program = vec![0xa6, postbyte, 0, 0, 0x13]; // LDA indexed, operands, SYNC
        let mut cpu = Motorola6809Simulator::new();
        cpu.load(&program, 0).unwrap();
        cpu.write_x(0x0100);
        cpu.write_y(0x0200);
        cpu.write_u(0x0300);
        cpu.write_s(0x0400);
        cpu.write_memory(0x0100, &[0x5a]).unwrap();
        let result = cpu.step();
        if indexed_postbyte_is_valid(postbyte) {
            result.unwrap_or_else(|error| panic!("postbyte {postbyte:#04x}: {error}"));
        } else {
            assert!(matches!(
                result,
                Err(Motorola6809Error::InvalidIndexedPostbyte { postbyte: value, .. })
                    if value == postbyte
            ));
        }
        program.clear();
    }
}

#[test]
fn indexed_auto_update_indirection_and_pc_relative_forms_are_exact() {
    let mut cpu = Motorola6809Simulator::new();
    cpu.load(
        &image(&[
            &[0xa6, 0x80],       // LDA ,X+
            &[0xe6, 0x91],       // LDB [,X++]
            &[0xae, 0x8c, 0x01], // LDX 1,PCR
            &[0x13],
            &[0x12, 0x34],
        ]),
        0,
    )
    .unwrap();
    cpu.write_x(0x2000);
    cpu.write_memory(0x2000, &[0xaa, 0x30, 0x00]).unwrap();
    cpu.write_memory(0x3000, &[0xbb]).unwrap();
    cpu.step().unwrap();
    assert_eq!((cpu.read_a(), cpu.read_x()), (0xaa, 0x2001));
    cpu.step().unwrap();
    assert_eq!((cpu.read_b(), cpu.read_x()), (0xbb, 0x2003));
    cpu.step().unwrap();
    assert_eq!(cpu.read_x(), 0x1234);
}

#[test]
fn branches_calls_transfers_and_both_stack_banks_round_trip() {
    let mut cpu = Motorola6809Simulator::new();
    cpu.load(
        &image(&[
            &[0x86, 0x12], // LDA #$12
            &[0xc6, 0x34], // LDB #$34
            &[0x1f, 0x01], // TFR D,X
            &[0x34, 0x17], // PSHS X,B,A,CC
            &[0x4f, 0x5f], // CLRA; CLRB
            &[0x35, 0x17], // PULS X,B,A,CC
            &[0x8d, 0x01], // BSR subroutine
            &[0x13],       // SYNC
            &[0x4c, 0x39], // INCA; RTS
        ]),
        0,
    )
    .unwrap();
    cpu.write_s(0x3000);
    cpu.run(32).unwrap();
    let state = cpu.get_state();
    assert_eq!((state.a, state.b, state.x), (0x13, 0x34, 0x1234));
    assert_eq!(state.s, 0x3000);
}

#[test]
fn short_and_long_branch_predicates_cover_taken_and_not_taken() {
    for code in 0_u8..16 {
        let opcode = 0x20 | code;
        for cc in [0, CC_C, CC_Z, CC_V, CC_N, CC_N | CC_V, CC_Z | CC_C] {
            let mut short = Motorola6809Simulator::new();
            short.load(&[opcode, 2, 0x12, 0x13, 0x13], 0).unwrap();
            short.write_cc(cc);
            short.step().unwrap();

            if code != 0 {
                let mut long = Motorola6809Simulator::new();
                long.load(&[0x10, opcode, 0, 2, 0x12, 0x13, 0x13], 0)
                    .unwrap();
                long.write_cc(cc);
                long.step().unwrap();
            }
        }
    }
}

#[test]
fn software_interrupts_and_rti_restore_full_frames() {
    for (instruction, vector) in [
        (&[0x3f][..], 0xfffa),
        (&[0x10, 0x3f], 0xfff4),
        (&[0x11, 0x3f], 0xfff2),
    ] {
        let mut program = instruction.to_vec();
        program.extend_from_slice(&[0x13, 0x3b]);
        let handler = (instruction.len() + 1) as u16;
        let mut cpu = Motorola6809Simulator::new();
        cpu.load(&program, 0).unwrap();
        cpu.write_s(0x4000);
        cpu.write_a(0x12);
        cpu.write_b(0x34);
        cpu.write_x(0x5678);
        cpu.write_y(0x9abc);
        cpu.write_u(0xdef0);
        cpu.write_memory(vector, &handler.to_be_bytes()).unwrap();
        cpu.run(8).unwrap();
        let state = cpu.get_state();
        assert_eq!(
            (state.a, state.b, state.x, state.y, state.u),
            (0x12, 0x34, 0x5678, 0x9abc, 0xdef0)
        );
    }
}

#[test]
fn traces_faults_and_bounded_execution_are_atomic() {
    let mut cpu = Motorola6809Simulator::new();
    cpu.load(&[0x86, 0x2a, 0x13], 0x100).unwrap();
    let trace = cpu.step().unwrap();
    assert_eq!((trace.pc_before, trace.pc_after), (0x100, 0x102));
    assert_eq!(trace.raw, vec![0x86, 0x2a]);
    assert_eq!(trace.mnemonic, "LDA");
    assert_eq!(trace.state_after.a, 42);

    let mut unknown = Motorola6809Simulator::new();
    unknown.load(&[0x01], 0).unwrap();
    let before = unknown.get_state();
    assert!(matches!(
        unknown.step(),
        Err(Motorola6809Error::UnknownOpcode { .. })
    ));
    assert_eq!(unknown.get_state(), before);

    let mut bounded = Motorola6809Simulator::new();
    bounded.load(&[0x12, 0x13], 0).unwrap();
    let before = bounded.get_state();
    assert_eq!(
        bounded.run(MAX_RETAINED_STEPS + 1),
        Err(Motorola6809Error::RetentionLimitExceeded {
            requested: MAX_RETAINED_STEPS + 1,
            limit: MAX_RETAINED_STEPS,
        })
    );
    assert_eq!(bounded.get_state(), before);
}

#[test]
fn typed_decode_lifecycle_faults_are_atomic_and_have_stable_text() {
    let cases: &[&[u8]] = &[
        &[0x10],       // truncated prefix
        &[0x10, 0x12], // unused page-2 opcode
        &[0xa6, 0x87], // reserved indexed postbyte
        &[0x1f, 0x06], // invalid transfer register
        &[0x86],       // truncated immediate
    ];
    for bytes in cases {
        let mut cpu = Motorola6809Simulator::new();
        cpu.load(bytes, 0).unwrap();
        let before = cpu.get_state();
        let error = cpu.step().expect_err("malformed instruction must fail");
        assert!(!error.to_string().is_empty());
        assert_eq!(cpu.get_state(), before);
    }

    let mut cpu = Motorola6809Simulator::new();
    assert_eq!(cpu.load(&[], 0), Err(Motorola6809Error::EmptyProgram));
    assert!(matches!(
        cpu.load(&[0; 2], 0xffff),
        Err(Motorola6809Error::ProgramOutOfRange { .. })
    ));

    cpu.load(&[0x12], 0).unwrap();
    let mut invalid = cpu.get_state();
    invalid.memory.pop();
    assert!(matches!(
        cpu.restore(invalid),
        Err(Motorola6809Error::InvalidState { .. })
    ));

    let before = cpu.get_state();
    assert!(matches!(
        cpu.execute(&[0x12], 0, 1),
        Err(Motorola6809Error::StepLimitExceeded { limit: 1 })
    ));
    assert_eq!(cpu.get_state(), before);
}

#[test]
fn arbitrary_byte_streams_do_not_panic_or_partially_mutate() {
    let mut seed = 0x6809_u64;
    for len in 1..=32 {
        let mut bytes = vec![0; len];
        for byte in &mut bytes {
            seed = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
            *byte = (seed >> 32) as u8;
        }
        let mut cpu = Motorola6809Simulator::new();
        cpu.load(&bytes, 0).unwrap();
        let before = cpu.get_state();
        if cpu.step().is_err() {
            assert_eq!(cpu.get_state(), before);
        }
    }
}
