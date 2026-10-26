use microwatt_powerisa_simulator::{classify, encoding, Classification};

fn is_supported(class: Classification) -> bool {
    matches!(class, Classification::Supported(_))
}

#[test]
fn all_primary_selectors_have_the_reviewed_boundary() {
    let supported = [
        7_u32, 8, 10, 11, 12, 13, 14, 15, 16, 18, 20, 21, 23, 24, 25, 26, 27, 28, 29, 32, 33, 34,
        35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 58, 62,
    ];
    let unsupported = [
        2_u32, 3, 4, 17, 46, 47, 48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 59, 60, 61, 63,
    ];
    for primary in 0..64 {
        if matches!(primary, 19 | 30 | 31) {
            continue;
        }
        let raw = if matches!(primary, 33 | 35 | 37 | 39 | 41 | 43 | 45) {
            (primary << 26) | (2 << 21) | (1 << 16)
        } else {
            primary << 26
        };
        let actual = classify(raw);
        if supported.contains(&primary) {
            assert!(is_supported(actual), "primary {primary}: {actual:?}");
        } else if unsupported.contains(&primary) {
            assert!(
                matches!(actual, Classification::Unsupported(_)),
                "primary {primary}: {actual:?}"
            );
        } else {
            assert_eq!(actual, Classification::Illegal, "primary {primary}");
        }
    }
}

#[test]
fn all_primary_19_extended_selectors_are_partitioned() {
    const SUPPORTED: &[u32] = &[0, 16, 33, 129, 150, 193, 225, 257, 289, 417, 449, 528];
    const UNSUPPORTED: &[u32] = &[18, 50, 274, 306, 370, 402, 566];
    for xo in 0..1024 {
        let mut raw = (19 << 26) | (xo << 1);
        if matches!(xo, 16 | 528) {
            raw |= 20 << 21;
        }
        let actual = classify(raw);
        if SUPPORTED.contains(&xo) {
            assert!(is_supported(actual), "XO19 {xo}: {actual:?}");
        } else if UNSUPPORTED.contains(&xo) {
            assert!(
                matches!(actual, Classification::Unsupported(_)),
                "XO19 {xo}: {actual:?}"
            );
        } else {
            assert_eq!(actual, Classification::Illegal, "XO19 {xo}");
        }
    }
}

#[test]
fn all_primary_31_extended_selectors_are_partitioned() {
    const ARITHMETIC_XO9: &[u32] = &[
        8, 10, 40, 104, 136, 138, 200, 202, 232, 233, 234, 235, 266, 457, 459, 489, 491,
    ];
    const SUPPORTED_XO10: &[u32] = &[
        0, 9, 11, 19, 21, 23, 24, 26, 27, 28, 32, 53, 55, 58, 60, 73, 75, 87, 119, 122, 124, 144,
        149, 151, 154, 181, 183, 186, 215, 247, 279, 284, 311, 316, 339, 341, 343, 373, 375, 378,
        407, 412, 439, 444, 467, 476, 506, 532, 534, 536, 538, 539, 570, 598, 660, 662, 790, 792,
        794, 824, 826, 827, 854, 918, 922, 954, 986,
    ];
    const UNSUPPORTED_XO10: &[u32] = &[
        4, 20, 54, 83, 84, 86, 146, 150, 214, 306, 334, 371, 498, 566, 982, 1014,
    ];
    for xo in 0..1024 {
        let raw = if matches!(xo, 53 | 55 | 119 | 181 | 183 | 247 | 311 | 373 | 375 | 439) {
            (31 << 26) | (2 << 21) | (1 << 16) | (xo << 1)
        } else {
            (31 << 26) | (xo << 1)
        };
        let actual = classify(raw);
        let normalized_xo9 = xo & 0x1ff;
        let arithmetic = ARITHMETIC_XO9.contains(&normalized_xo9);
        let high_multiply = [9, 11, 73, 75].contains(&normalized_xo9);
        if arithmetic || high_multiply || SUPPORTED_XO10.contains(&xo) {
            assert!(is_supported(actual), "XO31 {xo}: {actual:?}");
        } else if UNSUPPORTED_XO10.contains(&xo) {
            assert!(
                matches!(actual, Classification::Unsupported(_)),
                "XO31 {xo}: {actual:?}"
            );
        } else {
            assert_eq!(actual, Classification::Illegal, "XO31 {xo}");
        }
    }
}

#[test]
fn sync_levels_and_compare_slash_fields_are_exact() {
    assert_eq!(
        classify(encoding::sync(0)),
        Classification::Supported("sync")
    );
    assert_eq!(
        classify(encoding::sync(1)),
        Classification::Supported("lwsync")
    );
    assert_eq!(
        classify(encoding::sync(2)),
        Classification::Unsupported("ptesync")
    );
    assert_eq!(classify(encoding::sync(3)), Classification::Illegal);
    assert_eq!(
        classify((10 << 26) | (1 << 22)),
        Classification::Supported("cmpli")
    );
    assert_eq!(
        classify((11 << 26) | (1 << 22)),
        Classification::Supported("cmpi")
    );
}

#[test]
fn every_reviewed_slash_field_bit_is_ignored() {
    fn supported_with_each_bit(base: u32, bits: &[u32], name: &'static str) {
        assert_eq!(classify(base), Classification::Supported(name));
        for &bit in bits {
            assert_eq!(
                classify(base | (1 << bit)),
                Classification::Supported(name),
                "{name}: slash bit {bit}"
            );
        }
    }

    for op in [10, 11] {
        supported_with_each_bit(op << 26, &[22], if op == 10 { "cmpli" } else { "cmpi" });
    }
    supported_with_each_bit(19 << 26, &[0, 11, 12, 13, 14, 15, 16, 17, 21, 22], "mcrf");
    supported_with_each_bit((19 << 26) | (20 << 21) | (16 << 1), &[13, 14, 15], "bclr");
    supported_with_each_bit((19 << 26) | (20 << 21) | (528 << 1), &[13, 14, 15], "bcctr");
    for (xo, name) in [
        (33, "crnor"),
        (129, "crandc"),
        (193, "crxor"),
        (225, "crnand"),
        (257, "crand"),
        (289, "creqv"),
        (417, "crorc"),
        (449, "cror"),
    ] {
        supported_with_each_bit((19 << 26) | (xo << 1), &[0], name);
    }
    supported_with_each_bit(
        (19 << 26) | (150 << 1),
        &[
            0, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25,
        ],
        "isync",
    );
    for (xo, name) in [(0, "cmp"), (32, "cmpl")] {
        supported_with_each_bit((31 << 26) | (xo << 1), &[0, 22], name);
    }
    supported_with_each_bit(
        (31 << 26) | (19 << 1),
        &[0, 11, 12, 13, 14, 15, 16, 17, 18, 19],
        "mfcr",
    );
    supported_with_each_bit((31 << 26) | (144 << 1), &[0, 11], "mtcrf");
    for (xo, name) in [
        (122, "popcntb"),
        (154, "prtyw"),
        (186, "prtyd"),
        (378, "popcntw"),
        (506, "popcntd"),
    ] {
        supported_with_each_bit((31 << 26) | (xo << 1), &[0, 11, 12, 13, 14, 15], name);
    }
    for (xo, name) in [(9, "mulhdu"), (11, "mulhwu"), (73, "mulhd"), (75, "mulhw")] {
        supported_with_each_bit((31 << 26) | (xo << 1), &[10], name);
    }
    supported_with_each_bit(
        (31 << 26) | (598 << 1),
        &[0, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 23, 24, 25],
        "sync",
    );
    supported_with_each_bit(
        (31 << 26) | (854 << 1),
        &[
            0, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25,
        ],
        "eieio",
    );
}

#[test]
fn mfcr_and_mtcrf_l_boundaries_are_typed() {
    let mfcr = (31 << 26) | (19 << 1);
    let mtcrf = (31 << 26) | (144 << 1);
    for fxm in 0..=0xff {
        assert_eq!(
            classify(mfcr | (fxm << 12)),
            Classification::Supported("mfcr")
        );
        assert_eq!(
            classify(mtcrf | (fxm << 12)),
            Classification::Supported("mtcrf")
        );
        assert_eq!(
            classify(mfcr | (1 << 20) | (fxm << 12)),
            Classification::Unsupported("mfocrf")
        );
        assert_eq!(
            classify(mtcrf | (1 << 20) | (fxm << 12)),
            Classification::Unsupported("mtocrf")
        );
    }
}

#[test]
fn all_branch_condition_fields_remain_classification_invariant() {
    for bo in 0..32 {
        for bi in 0..32 {
            for absolute in [false, true] {
                for link in [false, true] {
                    assert!(is_supported(classify(encoding::b_form(
                        bo, bi, 4, absolute, link
                    ))));
                }
            }
        }
    }
    for bo in 0..32 {
        for bi in 0..32 {
            for bh in 0..4 {
                let class = classify(encoding::xl_form(bo, bi, bh, 528, false));
                assert_eq!(is_supported(class), bo & 4 != 0, "bo={bo} bi={bi} bh={bh}");
            }
        }
    }
}
