//! The fixture's functions against plain arithmetic, so a wrong fixture cannot hide behind
//! the generated harness that is derived from the same reading of "insert".

use vos_probe_ready::{insert10, insert8, pick8};

#[test]
fn insert8_sets_exactly_the_named_bit() {
    for mask in 0..=255u8 {
        for t in 0..=255u8 {
            let expect = if t < 8 { mask | (1 << t) } else { mask };
            assert_eq!(insert8(mask, t), expect, "mask {mask} t {t}");
        }
    }
}

#[test]
fn insert10_never_sets_a_bit_above_nine() {
    for mask in 0..1024u16 {
        for t in 0..=255u8 {
            let r = insert10(mask, t);
            assert_eq!(r >> 10, 0, "mask {mask} t {t}");
            let expect = if t < 10 { mask | (1 << t) } else { mask };
            assert_eq!(r, expect);
        }
    }
}

#[test]
fn pick8_is_the_highest_set_bit_or_255() {
    assert_eq!(pick8(0), 255);
    for mask in 1..=255u8 {
        let highest = (0..8).rev().find(|b| mask >> b & 1 == 1).unwrap();
        assert_eq!(pick8(mask), highest);
    }
}
