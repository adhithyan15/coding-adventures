//! Integration tests for fpga-bitstream.

use fpga_bitstream::{
    emit_bitstream, write_bin,
    ClbConfig, FpgaConfig, Ice40Part, PART_SPECS,
};
use fpga_bitstream::bitstream::cmd;

// ---------------------------------------------------------------------------
// PART_SPECS
// ---------------------------------------------------------------------------

#[test]
fn test_part_specs_cover_all_variants() {
    let parts = [Ice40Part::Hx1k, Ice40Part::Hx8k, Ice40Part::Up5k, Ice40Part::Lp1k];
    for part in parts {
        assert!(
            PART_SPECS.iter().any(|(p, ..)| *p == part),
            "{part:?} missing from PART_SPECS"
        );
    }
}

// ---------------------------------------------------------------------------
// emit_bitstream — empty config
// ---------------------------------------------------------------------------

#[test]
fn test_empty_config_emits_minimal_stream() {
    let config = FpgaConfig::new(Ice40Part::Hx1k);
    let (data, report) = emit_bitstream(&config);
    assert_eq!(report.bytes_written, data.len());
    assert_eq!(report.clb_count, 0);
    // Preamble
    assert_eq!(data[0], 0xFF);
    assert_eq!(data[1], 0x00);
    // End marker
    assert_eq!(data[data.len()-2], 0xFF);
    assert_eq!(data[data.len()-1], 0xFF);
}

#[test]
fn test_clb_count_in_report() {
    let mut config = FpgaConfig::new(Ice40Part::Hx1k);
    config.clbs.insert((0, 0), ClbConfig::default());
    config.clbs.insert((0, 1), ClbConfig::default());
    let (_, report) = emit_bitstream(&config);
    assert_eq!(report.clb_count, 2);
}

#[test]
fn test_part_in_report() {
    let config = FpgaConfig::new(Ice40Part::Up5k);
    let (_, report) = emit_bitstream(&config);
    assert_eq!(report.part, Ice40Part::Up5k);
}

// ---------------------------------------------------------------------------
// ClbConfig defaults
// ---------------------------------------------------------------------------

#[test]
fn test_clb_config_default_truth_table_size() {
    let clb = ClbConfig::default();
    assert_eq!(clb.lut_a_truth_table.len(), 16);
    assert_eq!(clb.lut_b_truth_table.len(), 16);
    assert!(clb.lut_a_truth_table.iter().all(|&b| b == 0));
    assert!(clb.lut_b_truth_table.iter().all(|&b| b == 0));
}

// ---------------------------------------------------------------------------
// Bitstream grows with more CLBs
// ---------------------------------------------------------------------------

#[test]
fn test_more_clbs_means_larger_bitstream() {
    let small = FpgaConfig::new(Ice40Part::Hx1k);
    let mut big = FpgaConfig::new(Ice40Part::Hx1k);
    for i in 0..10u32 {
        big.clbs.insert((0, i), ClbConfig::default());
    }
    let (_, small_r) = emit_bitstream(&small);
    let (_, big_r)   = emit_bitstream(&big);
    assert!(big_r.bytes_written > small_r.bytes_written);
}

// ---------------------------------------------------------------------------
// write_bin
// ---------------------------------------------------------------------------

#[test]
fn test_write_bin_creates_file() {
    let dir = std::env::temp_dir().join("fpga_bitstream_test");
    std::fs::create_dir_all(&dir).ok();
    let path = dir.join("out.bin");

    let mut config = FpgaConfig::new(Ice40Part::Hx1k);
    config.clbs.insert((2, 3), ClbConfig::default());

    let report = write_bin(&path, &config).unwrap();
    assert!(path.exists());
    let data = std::fs::read(&path).unwrap();
    assert_eq!(data.len(), report.bytes_written);
    // Preamble
    assert_eq!(data[0], 0xFF);
    assert_eq!(data[1], 0x00);
    // End marker
    assert_eq!(*data.last().unwrap(), 0xFF);
}

// ---------------------------------------------------------------------------
// cmd helper
// ---------------------------------------------------------------------------

#[test]
fn test_cmd_returns_correct_format() {
    let result = cmd(0x05, &[0x12, 0x34]);
    // total_len = 2 + 2 = 4; command = 0x05; payload = 0x12 0x34
    assert_eq!(result, vec![0x04u8, 0x05, 0x12, 0x34]);
}

#[test]
#[should_panic(expected = "payload too long")]
fn test_cmd_payload_too_long_panics() {
    let long = vec![0u8; 254];
    cmd(0x05, &long);
}

#[test]
fn test_cmd_empty_payload() {
    let result = cmd(0x07, &[]);
    // total_len = 0 + 2 = 2; command = 0x07; no payload
    assert_eq!(result, vec![0x02u8, 0x07]);
}

// ---------------------------------------------------------------------------
// Part dimensions sanity
// ---------------------------------------------------------------------------

#[test]
fn test_part_specs_non_zero() {
    for (_, rows, cols, cram_bits) in PART_SPECS {
        assert!(*rows > 0);
        assert!(*cols > 0);
        assert!(*cram_bits > 0);
    }
}

// ---------------------------------------------------------------------------
// 4-bit adder smoke test
// ---------------------------------------------------------------------------

#[test]
fn test_4bit_adder_smoke() {
    let dir = std::env::temp_dir().join("fpga_bitstream_adder_test");
    std::fs::create_dir_all(&dir).ok();
    let path = dir.join("adder4.bin");

    let mut config = FpgaConfig::new(Ice40Part::Hx1k);
    for i in 0..20u32 {
        let row = i / 8;
        let col = i % 8;
        config.clbs.insert((row, col), ClbConfig {
            lut_a_truth_table: vec![0, 1, 1, 0, 0, 1, 1, 0, 0, 1, 1, 0, 0, 1, 1, 0],
            ..Default::default()
        });
    }

    let report = write_bin(&path, &config).unwrap();
    assert_eq!(report.clb_count, 20);
    assert!(report.bytes_written > 100);

    let data = std::fs::read(&path).unwrap();
    assert_eq!(data[0], 0xFF);
    assert_eq!(data[1], 0x00);
    assert_eq!(data[data.len()-2], 0xFF);
    assert_eq!(data[data.len()-1], 0xFF);
}
