//! Integration tests for ConfigurableBRAM.

use block_ram::bram::ConfigurableBRAM;

#[test]
fn test_bram_dimensions() {
    let bram = ConfigurableBRAM::new(1024, 8);
    assert_eq!(bram.total_bits(), 1024);
    assert_eq!(bram.width(), 8);
    assert_eq!(bram.depth(), 128); // 1024 / 8
}

#[test]
fn test_bram_reconfigure_changes_dimensions() {
    let mut bram = ConfigurableBRAM::new(1024, 8);

    bram.reconfigure(16);
    assert_eq!(bram.width(), 16);
    assert_eq!(bram.depth(), 64); // 1024 / 16

    bram.reconfigure(4);
    assert_eq!(bram.width(), 4);
    assert_eq!(bram.depth(), 256); // 1024 / 4

    bram.reconfigure(1);
    assert_eq!(bram.width(), 1);
    assert_eq!(bram.depth(), 1024);
}

#[test]
fn test_bram_reconfigure_clears_data() {
    let mut bram = ConfigurableBRAM::new(64, 4);
    // depth = 16

    // Write data
    bram.tick_a(0, 0, &[1, 0, 1, 0], 1);
    bram.tick_a(1, 0, &[1, 0, 1, 0], 1);

    // Reconfigure
    bram.reconfigure(4);

    // Read — should be all zeros (data cleared)
    bram.tick_a(0, 0, &[0; 4], 0);
    let out = bram.tick_a(1, 0, &[0; 4], 0);
    assert_eq!(out, vec![0, 0, 0, 0]);
}

#[test]
fn test_bram_port_a_write_read() {
    let mut bram = ConfigurableBRAM::new(64, 4);
    // depth = 16

    // Write via port A
    bram.tick_a(0, 5, &[1, 1, 0, 1], 1);
    bram.tick_a(1, 5, &[1, 1, 0, 1], 1);

    // Read via port A
    bram.tick_a(0, 5, &[0; 4], 0);
    let out = bram.tick_a(1, 5, &[0; 4], 0);
    assert_eq!(out, vec![1, 1, 0, 1]);
}

#[test]
fn test_bram_port_b_write_read() {
    let mut bram = ConfigurableBRAM::new(64, 4);

    // Write via port B
    bram.tick_b(0, 3, &[0, 1, 1, 0], 1);
    bram.tick_b(1, 3, &[0, 1, 1, 0], 1);

    // Read via port B
    bram.tick_b(0, 3, &[0; 4], 0);
    let out = bram.tick_b(1, 3, &[0; 4], 0);
    assert_eq!(out, vec![0, 1, 1, 0]);
}

#[test]
fn test_bram_total_bits_preserved() {
    let bram = ConfigurableBRAM::new(18432, 8);
    assert_eq!(bram.total_bits(), 18432);
    assert_eq!(bram.depth(), 2304);
}

#[test]
#[should_panic(expected = "does not evenly divide")]
fn test_bram_reconfigure_invalid_width() {
    let mut bram = ConfigurableBRAM::new(1024, 8);
    bram.reconfigure(3); // 1024 % 3 != 0
}

#[test]
#[should_panic(expected = "total_bits must be >= 1")]
fn test_bram_zero_total_bits() {
    ConfigurableBRAM::new(0, 1);
}

#[test]
#[should_panic(expected = "width must be >= 1")]
fn test_bram_zero_width() {
    ConfigurableBRAM::new(1024, 0);
}
