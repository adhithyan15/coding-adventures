use microwatt_powerisa_simulator::encoding::{
    assemble, attn, b_form, d_form, i_form, x_form, xfx_form, xl_form, xo_form,
};
use microwatt_powerisa_simulator::{MicrowattPowerIsaSimulator, Xer};

fn run(raw: u32, configure: impl FnOnce(&mut microwatt_powerisa_simulator::State)) -> String {
    let mut cpu = MicrowattPowerIsaSimulator::new();
    cpu.load_program(&assemble(&[raw, attn()]), 0x100).unwrap();
    let mut state = cpu.get_state();
    state.gpr[3] = 0x1234_5678_9abc_def0;
    state.gpr[4] = 3;
    state.gpr[6] = 0x200;
    state.lr = 0x104;
    state.ctr = 0x104;
    state.cr = 0xa5a5_5a5a;
    state.memory[0x200..0x208].copy_from_slice(&0x8877_6655_4433_2211_u64.to_le_bytes());
    configure(&mut state);
    cpu.restore(&state).unwrap();
    cpu.step().unwrap().mnemonic
}

fn md(rs: u32, ra: u32, sh: u32, mb: u32, xo: u32, rc: bool) -> u32 {
    (30 << 26)
        | (rs << 21)
        | (ra << 16)
        | ((sh & 31) << 11)
        | ((mb & 31) << 6)
        | (mb & 32)
        | ((xo & 7) << 2)
        | ((sh & 32) >> 4)
        | u32::from(rc)
}

fn mds(rs: u32, ra: u32, rb: u32, mb: u32, xo: u32, rc: bool) -> u32 {
    (30 << 26)
        | (rs << 21)
        | (ra << 16)
        | (rb << 11)
        | ((mb & 63) << 6)
        | ((xo & 15) << 1)
        | u32::from(rc)
}

#[test]
fn immediate_rotate_and_doubleword_rotate_families_execute() {
    let rows = [
        (d_form(7, 5, 3, -7), "mulli"),
        (d_form(8, 5, 3, -7), "subfic"),
        (d_form(10, 0, 3, 9), "cmplwi"),
        (d_form(11, 0, 3, -9), "cmpwi"),
        (d_form(12, 5, 3, -7), "addic"),
        (d_form(13, 5, 3, -7), "addic."),
        (d_form(14, 5, 3, -7), "addi"),
        (d_form(15, 5, 3, -7), "addis"),
        (d_form(24, 3, 5, 0x1234), "ori"),
        (d_form(25, 3, 5, 0x1234), "oris"),
        (d_form(26, 3, 5, 0x1234), "xori"),
        (d_form(27, 3, 5, 0x1234), "xoris"),
        (d_form(28, 3, 5, 0x1234), "andi."),
        (d_form(29, 3, 5, 0x1234), "andis."),
        (
            (20 << 26) | (3 << 21) | (5 << 16) | (7 << 11) | (4 << 6) | (20 << 1),
            "rlwimi",
        ),
        (
            (21 << 26) | (3 << 21) | (5 << 16) | (7 << 11) | (4 << 6) | (20 << 1),
            "rlwinm",
        ),
        (
            (23 << 26) | (3 << 21) | (5 << 16) | (4 << 11) | (4 << 6) | (20 << 1),
            "rlwnm",
        ),
        (md(3, 5, 7, 9, 0, false), "rldicl"),
        (md(3, 5, 7, 9, 1, false), "rldicr"),
        (md(3, 5, 7, 9, 2, false), "rldic"),
        (md(3, 5, 7, 9, 3, false), "rldimi"),
        (mds(3, 5, 4, 9, 8, false), "rldcl"),
        (mds(3, 5, 4, 9, 9, false), "rldcr"),
    ];
    for (raw, expected) in rows {
        assert_eq!(run(raw, |_| {}), expected, "raw={raw:#010x}");
    }
}

#[test]
fn arithmetic_logical_count_and_shift_families_execute() {
    let arithmetic = [
        (8, "subfc"),
        (10, "addc"),
        (40, "subf"),
        (104, "neg"),
        (136, "subfe"),
        (138, "adde"),
        (200, "subfze"),
        (202, "addze"),
        (232, "subfme"),
        (233, "mulld"),
        (234, "addme"),
        (235, "mullw"),
        (266, "add"),
        (457, "divdu"),
        (459, "divwu"),
        (489, "divd"),
        (491, "divw"),
    ];
    for (xo, expected) in arithmetic {
        assert_eq!(
            run(xo_form(5, 3, 4, true, xo, true), |_| {}),
            format!("{expected}o.")
        );
    }

    let x_rows = [
        (9, "mulhdu"),
        (11, "mulhwu"),
        (73, "mulhd"),
        (75, "mulhw"),
        (24, "slw"),
        (27, "sld"),
        (28, "and"),
        (60, "andc"),
        (124, "nor"),
        (284, "eqv"),
        (316, "xor"),
        (412, "orc"),
        (444, "or"),
        (476, "nand"),
        (536, "srw"),
        (539, "srd"),
        (792, "sraw"),
        (794, "srad"),
        (26, "cntlzw"),
        (58, "cntlzd"),
        (538, "cnttzw"),
        (570, "cnttzd"),
        (922, "extsh"),
        (954, "extsb"),
        (986, "extsw"),
    ];
    for (xo, expected) in x_rows {
        assert_eq!(
            run(x_form(31, 3, 5, 4, xo, true), |_| {}),
            format!("{expected}.")
        );
    }
    for (xo, expected) in [
        (122, "popcntb"),
        (154, "prtyw"),
        (186, "prtyd"),
        (378, "popcntw"),
        (506, "popcntd"),
    ] {
        assert_eq!(run(x_form(31, 3, 5, 4, xo, false), |_| {}), expected);
    }
    assert_eq!(run(x_form(31, 3, 5, 7, 824, true), |_| {}), "srawi.");
    assert_eq!(run(x_form(31, 3, 5, 7, 826, true), |_| {}), "sradi.");
    assert_eq!(run(x_form(31, 3, 5, 7, 827, true), |_| {}), "sradi.");
}

#[test]
fn condition_branch_and_special_register_families_execute() {
    let cr_rows = [
        (33, "crnor"),
        (129, "crandc"),
        (193, "crxor"),
        (225, "crnand"),
        (257, "crand"),
        (289, "creqv"),
        (417, "crorc"),
        (449, "cror"),
    ];
    for (xo, expected) in cr_rows {
        let raw = (19 << 26) | (1 << 21) | (2 << 16) | (3 << 11) | (xo << 1);
        assert_eq!(run(raw, |_| {}), expected);
    }
    assert_eq!(run((19 << 26) | (4 << 21) | (8 << 16), |_| {}), "mcrf");
    assert_eq!(run(xl_form(20, 0, 0, 16, true), |_| {}), "bclrl");
    assert_eq!(run(xl_form(20, 0, 0, 528, true), |_| {}), "bcctrl");
    assert_eq!(run((19 << 26) | (150 << 1), |_| {}), "isync");
    assert_eq!(run(i_form(4, false, true), |_| {}), "bl");
    assert_eq!(run(i_form(0x104, true, true), |_| {}), "bla");
    assert_eq!(run(b_form(20, 0, 4, false, true), |_| {}), "bcl");

    assert_eq!(run(x_form(31, 0, 3, 4, 0, false), |_| {}), "cmpw");
    assert_eq!(run(x_form(31, 0, 3, 4, 32, false), |_| {}), "cmplw");
    assert_eq!(run(x_form(31, 5, 0, 0, 19, false), |_| {}), "mfcr");
    let mtcrf = (31 << 26) | (3 << 21) | (0xff << 12) | (144 << 1);
    assert_eq!(run(mtcrf, |_| {}), "mtcrf");
    assert_eq!(run(xfx_form(5, 8, 339), |_| {}), "mfspr");
    assert_eq!(run(xfx_form(3, 8, 467), |_| {}), "mtspr");
    assert_eq!(
        run(xfx_form(5, 1, 339), |s| s.xer = Xer {
            so: true,
            ..Xer::default()
        }),
        "mfspr"
    );
    assert_eq!(run(xfx_form(3, 1, 467), |_| {}), "mtspr");
    assert_eq!(run(xfx_form(5, 9, 339), |_| {}), "mfspr");
    assert_eq!(run(xfx_form(3, 9, 467), |_| {}), "mtspr");
}

#[test]
fn every_displacement_and_indexed_memory_family_executes() {
    let displacement = [
        (32, "lwz"),
        (33, "lwzu"),
        (34, "lbz"),
        (35, "lbzu"),
        (36, "stw"),
        (37, "stwu"),
        (38, "stb"),
        (39, "stbu"),
        (40, "lhz"),
        (41, "lhzu"),
        (42, "lha"),
        (43, "lhau"),
        (44, "sth"),
        (45, "sthu"),
    ];
    for (op, expected) in displacement {
        assert_eq!(run(d_form(op, 3, 6, 0), |_| {}), expected);
    }
    for (raw, expected) in [
        ((58 << 26) | (3 << 21) | (6 << 16), "ld"),
        ((58 << 26) | (3 << 21) | (6 << 16) | 1, "ldu"),
        ((58 << 26) | (3 << 21) | (6 << 16) | 2, "lwa"),
        ((62 << 26) | (3 << 21) | (6 << 16), "std"),
        ((62 << 26) | (3 << 21) | (6 << 16) | 1, "stdu"),
    ] {
        assert_eq!(run(raw, |_| {}), expected);
    }

    let indexed = [
        (21, "ldx"),
        (53, "ldux"),
        (23, "lwzx"),
        (55, "lwzux"),
        (87, "lbzx"),
        (119, "lbzux"),
        (149, "stdx"),
        (181, "stdux"),
        (151, "stwx"),
        (183, "stwux"),
        (215, "stbx"),
        (247, "stbux"),
        (279, "lhzx"),
        (311, "lhzux"),
        (341, "lwax"),
        (373, "lwaux"),
        (343, "lhax"),
        (375, "lhaux"),
        (407, "sthx"),
        (439, "sthux"),
        (532, "ldbrx"),
        (534, "lwbrx"),
        (660, "stdbrx"),
        (662, "stwbrx"),
        (790, "lhbrx"),
        (918, "sthbrx"),
    ];
    for (xo, expected) in indexed {
        assert_eq!(
            run(x_form(31, 3, 6, 4, xo, false), |s| s.gpr[4] = 0),
            expected
        );
    }
}

#[test]
fn synchronization_noops_execute() {
    assert_eq!(run((31 << 26) | (598 << 1), |_| {}), "sync");
    assert_eq!(run((31 << 26) | (1 << 21) | (598 << 1), |_| {}), "lwsync");
    assert_eq!(run((31 << 26) | (854 << 1), |_| {}), "eieio");
}
