//! Integration tests for SinglePortRAM and DualPortRAM.

use block_ram::ram::{DualPortRAM, ReadMode, SinglePortRAM};

// ===========================================================================
// Helper — perform a full write cycle (low + high)
// ===========================================================================

fn sp_write(ram: &mut SinglePortRAM, addr: usize, data: &[u8]) {
    ram.tick(0, addr, data, 1);
    ram.tick(1, addr, data, 1);
}

fn sp_read(ram: &mut SinglePortRAM, addr: usize) -> Vec<u8> {
    let zeros = vec![0u8; ram.width()];
    ram.tick(0, addr, &zeros, 0);
    ram.tick(1, addr, &zeros, 0)
}

// ===========================================================================
// SinglePortRAM tests
// ===========================================================================

#[test]
fn test_sp_write_and_read() {
    let mut ram = SinglePortRAM::new(16, 8, ReadMode::ReadFirst);
    sp_write(&mut ram, 0, &[1, 0, 1, 0, 1, 0, 1, 0]);
    let out = sp_read(&mut ram, 0);
    assert_eq!(out, vec![1, 0, 1, 0, 1, 0, 1, 0]);
}

#[test]
fn test_sp_unwritten_address_reads_zero() {
    let mut ram = SinglePortRAM::new(16, 4, ReadMode::ReadFirst);
    let out = sp_read(&mut ram, 5);
    assert_eq!(out, vec![0, 0, 0, 0]);
}

#[test]
fn test_sp_multiple_addresses() {
    let mut ram = SinglePortRAM::new(16, 4, ReadMode::ReadFirst);
    sp_write(&mut ram, 0, &[1, 0, 0, 0]);
    sp_write(&mut ram, 1, &[0, 1, 0, 0]);
    sp_write(&mut ram, 15, &[1, 1, 1, 1]);

    assert_eq!(sp_read(&mut ram, 0), vec![1, 0, 0, 0]);
    assert_eq!(sp_read(&mut ram, 1), vec![0, 1, 0, 0]);
    assert_eq!(sp_read(&mut ram, 15), vec![1, 1, 1, 1]);
}

#[test]
fn test_sp_read_first_returns_old_value() {
    let mut ram = SinglePortRAM::new(4, 4, ReadMode::ReadFirst);
    sp_write(&mut ram, 0, &[1, 0, 1, 0]);

    // Overwrite — ReadFirst should return old value
    ram.tick(0, 0, &[0, 1, 0, 1], 1);
    let out = ram.tick(1, 0, &[0, 1, 0, 1], 1);
    assert_eq!(out, vec![1, 0, 1, 0]);
}

#[test]
fn test_sp_write_first_returns_new_value() {
    let mut ram = SinglePortRAM::new(4, 4, ReadMode::WriteFirst);
    sp_write(&mut ram, 0, &[1, 0, 1, 0]);

    // Overwrite — WriteFirst should return new value
    ram.tick(0, 0, &[0, 1, 0, 1], 1);
    let out = ram.tick(1, 0, &[0, 1, 0, 1], 1);
    assert_eq!(out, vec![0, 1, 0, 1]);
}

#[test]
fn test_sp_no_change_retains_previous() {
    let mut ram = SinglePortRAM::new(4, 4, ReadMode::NoChange);

    // Read from addr 0 first to set last_read
    let out = sp_read(&mut ram, 0);
    assert_eq!(out, vec![0, 0, 0, 0]);

    // Write — NoChange should return previous read
    ram.tick(0, 0, &[1, 1, 1, 1], 1);
    let out = ram.tick(1, 0, &[1, 1, 1, 1], 1);
    assert_eq!(out, vec![0, 0, 0, 0]); // unchanged

    // Verify data was actually written
    let out = sp_read(&mut ram, 0);
    assert_eq!(out, vec![1, 1, 1, 1]);
}

#[test]
fn test_sp_no_operation_on_falling_edge() {
    let mut ram = SinglePortRAM::new(4, 4, ReadMode::ReadFirst);
    sp_write(&mut ram, 0, &[1, 0, 1, 0]);

    // Falling edge (1->0) should not trigger an operation
    let out1 = ram.tick(1, 0, &[0; 4], 0);
    let out2 = ram.tick(0, 0, &[0; 4], 0); // falling edge
    // Both should return the same cached value
    assert_eq!(out1, out2);
}

#[test]
fn test_sp_dump() {
    let mut ram = SinglePortRAM::new(4, 2, ReadMode::ReadFirst);
    sp_write(&mut ram, 0, &[1, 0]);
    sp_write(&mut ram, 2, &[0, 1]);

    let dump = ram.dump();
    assert_eq!(dump.len(), 4);
    assert_eq!(dump[0], vec![1, 0]);
    assert_eq!(dump[1], vec![0, 0]);
    assert_eq!(dump[2], vec![0, 1]);
    assert_eq!(dump[3], vec![0, 0]);
}

// ===========================================================================
// DualPortRAM tests
// ===========================================================================

#[test]
fn test_dp_write_a_read_b() {
    let mut ram = DualPortRAM::new(4, 4, ReadMode::ReadFirst, ReadMode::ReadFirst);

    // Write via port A to address 0
    ram.tick(0, 0, &[1, 1, 0, 0], 1, 1, &[0; 4], 0).unwrap();
    ram.tick(1, 0, &[1, 1, 0, 0], 1, 1, &[0; 4], 0).unwrap();

    // Read via port B from address 0
    ram.tick(0, 0, &[0; 4], 0, 0, &[0; 4], 0).unwrap();
    let (_, out_b) = ram.tick(1, 0, &[0; 4], 0, 0, &[0; 4], 0).unwrap();
    assert_eq!(out_b, vec![1, 1, 0, 0]);
}

#[test]
fn test_dp_simultaneous_read_different_addresses() {
    let mut ram = DualPortRAM::new(4, 4, ReadMode::ReadFirst, ReadMode::ReadFirst);

    // Write to addr 0 and 1
    ram.tick(0, 0, &[1, 0, 0, 0], 1, 1, &[0, 1, 0, 0], 1).unwrap();
    ram.tick(1, 0, &[1, 0, 0, 0], 1, 1, &[0, 1, 0, 0], 1).unwrap();

    // Read from both addresses simultaneously
    ram.tick(0, 0, &[0; 4], 0, 1, &[0; 4], 0).unwrap();
    let (out_a, out_b) = ram.tick(1, 0, &[0; 4], 0, 1, &[0; 4], 0).unwrap();
    assert_eq!(out_a, vec![1, 0, 0, 0]);
    assert_eq!(out_b, vec![0, 1, 0, 0]);
}

#[test]
fn test_dp_write_collision_returns_error() {
    let mut ram = DualPortRAM::new(4, 4, ReadMode::ReadFirst, ReadMode::ReadFirst);

    // Both ports write to address 0
    ram.tick(0, 0, &[1; 4], 1, 0, &[0; 4], 1).unwrap();
    let result = ram.tick(1, 0, &[1; 4], 1, 0, &[0; 4], 1);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.address, 0);
    assert_eq!(
        err.to_string(),
        "Write collision: both ports writing to address 0"
    );
}

#[test]
fn test_dp_write_different_addresses_no_collision() {
    let mut ram = DualPortRAM::new(4, 4, ReadMode::ReadFirst, ReadMode::ReadFirst);

    // Both ports write to different addresses — should succeed
    ram.tick(0, 0, &[1; 4], 1, 1, &[0, 1, 0, 1], 1).unwrap();
    let result = ram.tick(1, 0, &[1; 4], 1, 1, &[0, 1, 0, 1], 1);
    assert!(result.is_ok());
}

#[test]
fn test_dp_properties() {
    let ram = DualPortRAM::new(256, 8, ReadMode::ReadFirst, ReadMode::WriteFirst);
    assert_eq!(ram.depth(), 256);
    assert_eq!(ram.width(), 8);
}
