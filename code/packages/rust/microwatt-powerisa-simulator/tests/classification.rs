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
        407, 412, 413, 439, 444, 445, 467, 476, 506, 532, 534, 536, 538, 539, 570, 598, 660, 662,
        790, 792, 794, 824, 854, 918, 922, 954, 986,
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
        let arithmetic = ARITHMETIC_XO9.contains(&(xo & 0x1ff));
        if arithmetic || SUPPORTED_XO10.contains(&xo) {
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
fn sync_levels_and_compare_reserved_bit_are_exact() {
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
    assert_eq!(classify((10 << 26) | (1 << 22)), Classification::Illegal);
    assert_eq!(classify((11 << 26) | (1 << 22)), Classification::Illegal);
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
