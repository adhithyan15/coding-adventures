mod rtl_support;

use coding_adventures_sha256::sha256_hex;
use std::collections::HashSet;

#[test]
fn manifest_and_corpus_are_exact_and_complete() {
    let vectors = include_bytes!("rtl/vectors.tsv");
    let harness = include_bytes!("rtl/microwatt_oracle_tb.vhdl");
    let generator = include_bytes!("rtl/generate.py");
    let manifest = include_str!("rtl/manifest.txt");
    let rows = rtl_support::rows();
    assert_eq!(rows.len(), 164);
    assert!(manifest.contains("microwatt_commit=f341bed306a9a7281bf65200df8cc1f0a002617d"));
    assert!(manifest.contains("ghdl_version=GHDL 6.0.0 (6.0.0.r0.ge589c698c) [Dunoon edition]"));
    assert!(manifest.contains("row_count=164"));
    assert!(manifest.contains(&format!("vectors_sha256={}", sha256_hex(vectors))));
    assert!(manifest.contains(&format!("harness_sha256={}", sha256_hex(harness))));
    assert!(manifest.contains(&format!("generator_sha256={}", sha256_hex(generator))));

    let names = rows.iter().map(|row| row.name).collect::<HashSet<_>>();
    for required in [
        "mulli",
        "b_relative",
        "bcctrl",
        "rldimi",
        "lwzu",
        "stdbrx",
        "addo_ov32_so0",
        "addo_ov32_so1",
        "mullwo_no_overflow",
        "mullwo_positive_overflow",
        "mullwo_negative_overflow",
        "mulhwu_shape",
        "divw_shape",
        "sraw_sign_extend_32",
        "prtyw_byte_lsb",
        "prtyd_byte_lsb",
        "sync",
        "lwsync",
        "attn",
    ] {
        assert!(names.contains(required), "missing {required}");
    }
    assert!(rows
        .iter()
        .all(|row| row.cia_before == rtl_support::TARGET_CIA));
    assert!(rows.iter().all(|row| row.memory_sha256.len() == 64));
}

#[test]
fn source_locked_rows_independently_assert_the_pinned_rtl_choices() {
    let rows = rtl_support::rows();
    let row = |name| rows.iter().find(|row| row.name == name).unwrap();

    for name in ["addo_ov32_so0", "addo_ov32_so1"] {
        let item = row(name);
        assert_eq!(item.after_xer & (1 << 30), 0, "OV must stay clear");
        assert_ne!(item.after_xer & (1 << 19), 0, "OV32 must set");
        assert_eq!(item.after_xer & (1 << 31), item.before_xer & (1 << 31));
    }
    for name in ["mullwo_positive_overflow", "mullwo_negative_overflow"] {
        let item = row(name);
        assert_ne!(item.after_xer & (1 << 30), 0);
        assert_ne!(item.after_xer & (1 << 19), 0);
        assert_ne!(item.after_xer & (1 << 31), 0);
    }
    let no_overflow = row("mullwo_no_overflow");
    assert_eq!(
        no_overflow.after_xer & ((1 << 31) | (1 << 30) | (1 << 19)),
        0
    );
    let high = row("mulhwu_shape").after_gprs[5];
    assert_eq!(high >> 32, high & 0xffff_ffff);
    assert_eq!(row("divw_shape").after_gprs[5] >> 32, 0);
    assert_eq!(row("sraw_sign_extend_32").after_gprs[5], u64::MAX);
    assert_eq!(row("prtyw_byte_lsb").after_gprs[5], 0x0000_0001_0000_0000);
    assert_eq!(row("prtyd_byte_lsb").after_gprs[5], 0);
}
