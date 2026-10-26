use cortex_m33_simulator::{
    CortexM33Error, CortexM33Simulator, SauRegion, SecurityAttribute, SecurityState,
    MAX_RETAINED_STEPS,
};
use cortex_m3_simulator::encoding::{bl, ldrex, movw, mrs, tbb};
use cortex_m3_simulator::CortexM3Error;

fn bytes(halfwords: &[u16]) -> Vec<u8> {
    halfwords
        .iter()
        .flat_map(|halfword| halfword.to_le_bytes())
        .collect()
}

fn region(base: u32, limit: u32, nsc: bool) -> SauRegion {
    SauRegion {
        base,
        limit,
        enabled: true,
        non_secure_callable: nsc,
    }
}

#[test]
fn reset_and_sau_attribution_cover_security_configurations() {
    let mut secure = CortexM33Simulator::new(true);
    assert_eq!(secure.security_state(), SecurityState::Secure);
    assert_eq!(secure.attribute(0x200), SecurityAttribute::Secure);

    secure.set_sau_control(true, true).unwrap();
    assert_eq!(secure.attribute(0x200), SecurityAttribute::NonSecure);
    secure
        .configure_sau_region(0, region(0x100, 0x11f, true))
        .unwrap();
    assert_eq!(
        secure.attribute(0x100),
        SecurityAttribute::NonSecureCallable
    );
    secure
        .configure_sau_region(1, region(0x100, 0x13f, false))
        .unwrap();
    assert_eq!(secure.attribute(0x100), SecurityAttribute::Secure);
    assert_eq!(secure.attribute(0x120), SecurityAttribute::NonSecure);

    let plain = CortexM33Simulator::new(false);
    assert_eq!(plain.security_state(), SecurityState::NonSecure);
    assert_eq!(plain.attribute(0), SecurityAttribute::NonSecure);
}

#[test]
fn sau_configuration_rejects_privilege_and_shape_errors_atomically() {
    let mut cpu = CortexM33Simulator::new(true);
    let before = cpu.get_state().unwrap();
    assert!(matches!(
        cpu.configure_sau_region(8, region(0, 0x1f, false)),
        Err(CortexM33Error::InvalidSauRegion { .. })
    ));
    assert!(matches!(
        cpu.configure_sau_region(0, region(1, 0x1f, false)),
        Err(CortexM33Error::InvalidSauRegion { .. })
    ));
    assert!(matches!(
        cpu.configure_sau_region(0, region(0, 0x20, false)),
        Err(CortexM33Error::InvalidSauRegion { .. })
    ));
    assert!(matches!(
        cpu.configure_sau_region(0, region(0x40, 0x1f, false)),
        Err(CortexM33Error::InvalidSauRegion { .. })
    ));
    assert_eq!(cpu.get_state().unwrap(), before);

    let mut unprivileged = cpu.get_state().unwrap();
    unprivileged.core.control = 1;
    unprivileged.secure_bank.control = 1;
    cpu.restore(unprivileged).unwrap();
    assert_eq!(
        cpu.set_sau_control(true, false),
        Err(CortexM33Error::PrivilegeViolation)
    );

    let mut plain = CortexM33Simulator::new(false);
    assert_eq!(
        plain.set_sau_control(true, true),
        Err(CortexM33Error::SecurityExtensionUnavailable)
    );
}

#[test]
fn security_transitions_swap_banked_control_and_require_attributed_targets() {
    let mut cpu = CortexM33Simulator::new(true);
    cpu.set_sau_control(true, false).unwrap();
    cpu.configure_sau_region(0, region(0x8000, 0x8fff, false))
        .unwrap();
    cpu.configure_sau_region(1, region(0x1000, 0x101f, true))
        .unwrap();
    cpu.write_register(13, 0x7000).unwrap();
    cpu.set_stack_limits(0x6000, 0x5000).unwrap();

    assert!(matches!(
        cpu.branch_non_secure(0x1000, false),
        Err(CortexM33Error::InvalidSecurityTransition { .. })
    ));
    cpu.branch_non_secure(0x8000, true).unwrap();
    assert_eq!(cpu.security_state(), SecurityState::NonSecure);
    assert_eq!(cpu.read_register(13).unwrap(), 0x1_0000);
    cpu.write_register(13, 0x8800).unwrap();
    cpu.set_stack_limits(0x8000, 0x8100).unwrap();
    assert!(matches!(
        cpu.branch_non_secure(0x8020, false),
        Err(CortexM33Error::InvalidSecurityTransition { .. })
    ));

    cpu.enter_secure_gateway(0x1000).unwrap();
    assert_eq!(cpu.security_state(), SecurityState::Secure);
    assert_eq!(cpu.read_register(13).unwrap(), 0x7000);
    assert_eq!(cpu.get_state().unwrap().core.lr & 1, 0);
}

#[test]
fn inherited_integer_execution_matches_cortex_m3_complete_core_state() {
    let program = bytes(&[0x2001, 0x3002, 0xbe00]); // movs r0,#1; adds r0,#2; halt
    let mut m3 = cortex_m3_simulator::CortexM3Simulator::new();
    let mut m33 = CortexM33Simulator::new(false);
    m3.load(&program, 0).unwrap();
    m33.load(&program, 0).unwrap();

    for expected_mnemonic in ["movs", "adds", "bkpt 0"] {
        let expected = m3.step().unwrap();
        let actual = m33.step().unwrap();
        assert_eq!(actual.mnemonic, expected_mnemonic);
        assert_eq!(actual.raw, expected.raw);
        assert_eq!(actual.width, expected.width);
        assert_eq!(actual.state_after.core, expected.state_after);
        assert_eq!(actual.security_before, SecurityState::NonSecure);
    }
}

#[test]
fn every_inherited_decode_family_has_an_exact_differential_witness() {
    struct Case {
        name: &'static str,
        words: Vec<u16>,
        registers: &'static [(u8, u32)],
    }

    let branch_link = bl(0);
    let read_special = mrs(0, 0);
    let wide_data = movw(0, 0x1234);
    let exclusive = ldrex(1, 0, 0);
    let table = tbb(0, 1);
    let cases = [
        Case {
            name: "shift/add/subtract",
            words: vec![0x0000],
            registers: &[],
        },
        Case {
            name: "immediate",
            words: vec![0x2001],
            registers: &[],
        },
        Case {
            name: "data processing",
            words: vec![0x4000],
            registers: &[],
        },
        Case {
            name: "special data",
            words: vec![0x4600],
            registers: &[],
        },
        Case {
            name: "literal load",
            words: vec![0x4800],
            registers: &[],
        },
        Case {
            name: "register memory",
            words: vec![0x5008],
            registers: &[(0, 0x400), (1, 0)],
        },
        Case {
            name: "immediate memory",
            words: vec![0x6008],
            registers: &[(1, 0x400)],
        },
        Case {
            name: "stack memory",
            words: vec![0x9000],
            registers: &[(13, 0x800)],
        },
        Case {
            name: "address generation",
            words: vec![0xa000],
            registers: &[],
        },
        Case {
            name: "miscellaneous",
            words: vec![0xbf00],
            registers: &[],
        },
        Case {
            name: "multiple memory",
            words: vec![0xc002],
            registers: &[(0, 0x400)],
        },
        Case {
            name: "conditional branch",
            words: vec![0xd100],
            registers: &[],
        },
        Case {
            name: "unconditional branch",
            words: vec![0xe000],
            registers: &[],
        },
        Case {
            name: "branch with link",
            words: branch_link.to_vec(),
            registers: &[],
        },
        Case {
            name: "barrier",
            words: vec![0xf3bf, 0x8f5f],
            registers: &[],
        },
        Case {
            name: "special register",
            words: read_special.to_vec(),
            registers: &[],
        },
        Case {
            name: "Thumb-2 data",
            words: wide_data.to_vec(),
            registers: &[],
        },
        Case {
            name: "Thumb-2 memory",
            words: vec![0xf8c0, 0x1000],
            registers: &[(0, 0x400)],
        },
        Case {
            name: "exclusive",
            words: exclusive.to_vec(),
            registers: &[(0, 0x400)],
        },
        Case {
            name: "table branch",
            words: table.to_vec(),
            registers: &[(0, 0x400), (1, 0)],
        },
    ];

    for case in cases {
        let program = bytes(&case.words);
        let mut m3 = cortex_m3_simulator::CortexM3Simulator::new();
        let mut m33 = CortexM33Simulator::new(false);
        m3.load(&program, 0x100).unwrap();
        m33.load(&program, 0x100).unwrap();
        for &(register, value) in case.registers {
            m3.write_register(register, value).unwrap();
            m33.write_register(register, value).unwrap();
        }

        let expected = m3.step().unwrap();
        let actual = m33.step().unwrap();
        assert_eq!(actual.raw, expected.raw, "{} raw", case.name);
        assert_eq!(actual.width, expected.width, "{} width", case.name);
        assert_eq!(actual.mnemonic, expected.mnemonic, "{} mnemonic", case.name);
        assert_eq!(
            actual.state_after.core, expected.state_after,
            "{} state",
            case.name
        );
    }
}

#[test]
fn nonsecure_fetch_and_data_access_require_nonsecure_attribution() {
    let program = bytes(&[0x6008, 0xbe00]); // str r0,[r1,#0]; halt
    let mut cpu = CortexM33Simulator::new(true);
    cpu.load(&program, 0).unwrap();
    cpu.set_sau_control(true, false).unwrap();
    cpu.configure_sau_region(0, region(0, 0x1f, false)).unwrap();
    cpu.write_register(1, 0x100).unwrap();
    cpu.branch_non_secure(0, false).unwrap();
    let before = cpu.get_state().unwrap();
    assert!(matches!(
        cpu.step(),
        Err(CortexM33Error::SecurityAccessViolation {
            address: 0x100,
            width: 4,
            ..
        })
    ));
    assert_eq!(cpu.get_state().unwrap(), before);

    let mut state = cpu.get_state().unwrap();
    state.security_state = SecurityState::Secure;
    state.core.msp = state.secure_bank.msp;
    state.core.psp = state.secure_bank.psp;
    state.core.primask = state.secure_bank.primask;
    state.core.basepri = state.secure_bank.basepri;
    state.core.faultmask = state.secure_bank.faultmask;
    state.core.control = state.secure_bank.control;
    cpu.restore(state).unwrap();
    cpu.configure_sau_region(1, region(0x100, 0x11f, false))
        .unwrap();
    cpu.branch_non_secure(0, false).unwrap();
    cpu.step().unwrap();
    assert_eq!(cpu.read_memory(0x100, 4).unwrap(), 0);

    let mut fetch_cpu = CortexM33Simulator::new(true);
    fetch_cpu.load(&[0; 0x42], 0).unwrap();
    fetch_cpu.set_sau_control(true, false).unwrap();
    fetch_cpu
        .configure_sau_region(0, region(0, 0x1f, false))
        .unwrap();
    fetch_cpu.branch_non_secure(0, false).unwrap();
    fetch_cpu.write_register(15, 0x40).unwrap();
    let fetch_before = fetch_cpu.get_state().unwrap();
    assert!(matches!(
        fetch_cpu.step(),
        Err(CortexM33Error::SecurityAccessViolation {
            address: 0x40,
            width: 2,
            ..
        })
    ));
    assert_eq!(fetch_cpu.get_state().unwrap(), fetch_before);
}

#[test]
fn direct_nonsecure_access_rejects_secure_and_nsc_memory() {
    let mut cpu = CortexM33Simulator::new(true);
    cpu.set_sau_control(true, false).unwrap();
    cpu.configure_sau_region(0, region(0x8000, 0x8fff, false))
        .unwrap();
    cpu.configure_sau_region(1, region(0x1000, 0x101f, true))
        .unwrap();
    cpu.branch_non_secure(0x8000, false).unwrap();
    assert!(matches!(
        cpu.read_memory(0, 1),
        Err(CortexM33Error::SecurityAccessViolation { .. })
    ));
    assert!(matches!(
        cpu.write_memory(0x1000, 1, 1),
        Err(CortexM33Error::SecurityAccessViolation { .. })
    ));
    cpu.write_memory(0x8000, 4, 0x4433_2211).unwrap();
    assert_eq!(cpu.read_memory(0x8000, 4).unwrap(), 0x4433_2211);
    assert!(matches!(
        cpu.read_memory(0xffff, 4),
        Err(CortexM33Error::Core(CortexM3Error::DataOutOfRange { .. }))
    ));
}

#[test]
fn stack_limits_and_restore_fail_atomically() {
    let mut cpu = CortexM33Simulator::new(false);
    cpu.load(&bytes(&[0xb081, 0xbe00]), 0).unwrap(); // sub sp,#4
    cpu.write_register(13, 0x100).unwrap();
    cpu.set_stack_limits(0x100, 0).unwrap();
    let before = cpu.get_state().unwrap();
    assert_eq!(
        cpu.step(),
        Err(CortexM33Error::StackLimitViolation {
            stack_pointer: 0xfc,
            limit: 0x100,
        })
    );
    assert_eq!(cpu.get_state().unwrap(), before);

    assert_eq!(
        cpu.write_register(13, 0xf8),
        Err(CortexM33Error::StackLimitViolation {
            stack_pointer: 0xf8,
            limit: 0x100,
        })
    );
    assert_eq!(cpu.get_state().unwrap(), before);
    assert!(matches!(
        cpu.set_stack_limits(0x108, 0),
        Err(CortexM33Error::StackLimitViolation { .. })
    ));
    assert_eq!(cpu.get_state().unwrap(), before);

    let mut invalid = before.clone();
    invalid.non_secure_bank.msplim = 3;
    assert!(matches!(
        cpu.restore(invalid),
        Err(CortexM33Error::Core(CortexM3Error::InvalidState { .. }))
    ));
    assert_eq!(cpu.get_state().unwrap(), before);

    for mutate in [
        |state: &mut cortex_m33_simulator::CortexM33State| {
            state.secure_bank.msp = 0x100;
            state.secure_bank.msplim = 0x108;
        },
        |state: &mut cortex_m33_simulator::CortexM33State| {
            state.non_secure_bank.psp = 0x100;
            state.non_secure_bank.psplim = 0x108;
        },
    ] {
        let mut invalid = before.clone();
        mutate(&mut invalid);
        assert!(matches!(
            cpu.restore(invalid),
            Err(CortexM33Error::Core(CortexM3Error::InvalidState { .. }))
        ));
        assert_eq!(cpu.get_state().unwrap(), before);
    }
}

#[test]
fn run_execute_and_events_are_transactional_and_bounded() {
    let mut cpu = CortexM33Simulator::new(false);
    let program = bytes(&[0x2007, 0xbe00]);
    let result = cpu.execute(&program, 0, 2).unwrap();
    assert!(result.stopped);
    assert_eq!(result.steps, 2);
    assert_eq!(result.final_state, cpu.get_state().unwrap());

    let before = cpu.get_state().unwrap();
    assert_eq!(
        cpu.run(MAX_RETAINED_STEPS + 1),
        Err(CortexM33Error::RetentionLimitExceeded {
            requested: MAX_RETAINED_STEPS + 1,
            limit: MAX_RETAINED_STEPS,
        })
    );
    assert_eq!(cpu.get_state().unwrap(), before);

    let mut failing = CortexM33Simulator::new(false);
    failing.load(&bytes(&[0x2001, 0xde00]), 0).unwrap();
    let rollback = failing.get_state().unwrap();
    assert!(matches!(
        failing.run(2),
        Err(CortexM33Error::Core(
            CortexM3Error::UnknownInstruction16 { .. }
        ))
    ));
    assert_eq!(failing.get_state().unwrap(), rollback);

    let mut limited = CortexM33Simulator::new(false);
    limited.load(&bytes(&[0x2001, 0xe7fe]), 0).unwrap();
    let limited_before = limited.get_state().unwrap();
    assert_eq!(
        limited.run(1),
        Err(CortexM33Error::StepLimitExceeded { limit: 1 })
    );
    assert_eq!(limited.get_state().unwrap(), limited_before);

    let execute_before = limited.get_state().unwrap();
    assert!(matches!(
        limited.execute(&[], 0, 1),
        Err(CortexM33Error::Core(CortexM3Error::EmptyProgram))
    ));
    assert_eq!(limited.get_state().unwrap(), execute_before);

    failing.signal_event().unwrap();
    assert!(failing.get_state().unwrap().core.event_registered);
    failing.signal_interrupt().unwrap();
    assert!(!failing.get_state().unwrap().core.sleeping);
}

#[test]
fn extension_and_transition_errors_are_typed() {
    let mut plain = CortexM33Simulator::new(false);
    assert_eq!(
        plain.branch_non_secure(0, false),
        Err(CortexM33Error::SecurityExtensionUnavailable)
    );
    assert_eq!(
        plain.enter_secure_gateway(0),
        Err(CortexM33Error::SecurityExtensionUnavailable)
    );

    let mut secure = CortexM33Simulator::new(true);
    assert!(matches!(
        secure.enter_secure_gateway(0),
        Err(CortexM33Error::InvalidSecurityTransition { .. })
    ));
    assert!(CortexM33Error::PrivilegeViolation
        .to_string()
        .contains("privileged Secure"));
}
