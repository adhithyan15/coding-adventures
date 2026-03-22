//! Integration tests for SRAM cells and arrays.

use block_ram::sram::{SRAMArray, SRAMCell};

// ===========================================================================
// SRAMCell tests
// ===========================================================================

#[test]
fn test_cell_default_value_is_zero() {
    let cell = SRAMCell::new();
    assert_eq!(cell.value(), 0);
}

#[test]
fn test_cell_write_one_read_one() {
    let mut cell = SRAMCell::new();
    cell.write(1, 1);
    assert_eq!(cell.read(1), Some(1));
}

#[test]
fn test_cell_write_zero_over_one() {
    let mut cell = SRAMCell::new();
    cell.write(1, 1);
    cell.write(1, 0);
    assert_eq!(cell.value(), 0);
}

#[test]
fn test_cell_not_selected_read_returns_none() {
    let mut cell = SRAMCell::new();
    cell.write(1, 1);
    assert_eq!(cell.read(0), None);
}

#[test]
fn test_cell_not_selected_write_no_effect() {
    let mut cell = SRAMCell::new();
    cell.write(1, 1);
    cell.write(0, 0); // word_line=0, no effect
    assert_eq!(cell.value(), 1);
}

#[test]
fn test_cell_clone_independence() {
    let mut cell = SRAMCell::new();
    cell.write(1, 1);
    let clone = cell.clone();
    cell.write(1, 0);
    assert_eq!(clone.value(), 1); // clone unaffected
    assert_eq!(cell.value(), 0);
}

// ===========================================================================
// SRAMArray tests
// ===========================================================================

#[test]
fn test_array_initial_all_zeros() {
    let arr = SRAMArray::new(4, 4);
    for row in 0..4 {
        assert_eq!(arr.read(row), vec![0, 0, 0, 0]);
    }
}

#[test]
fn test_array_write_and_read_row() {
    let mut arr = SRAMArray::new(4, 8);
    arr.write(0, &[1, 0, 1, 0, 0, 1, 0, 1]);
    assert_eq!(arr.read(0), vec![1, 0, 1, 0, 0, 1, 0, 1]);
    assert_eq!(arr.read(1), vec![0, 0, 0, 0, 0, 0, 0, 0]);
}

#[test]
fn test_array_write_multiple_rows() {
    let mut arr = SRAMArray::new(4, 4);
    arr.write(0, &[1, 1, 0, 0]);
    arr.write(1, &[0, 0, 1, 1]);
    arr.write(3, &[1, 0, 1, 0]);
    assert_eq!(arr.read(0), vec![1, 1, 0, 0]);
    assert_eq!(arr.read(1), vec![0, 0, 1, 1]);
    assert_eq!(arr.read(2), vec![0, 0, 0, 0]);
    assert_eq!(arr.read(3), vec![1, 0, 1, 0]);
}

#[test]
fn test_array_overwrite_row() {
    let mut arr = SRAMArray::new(2, 4);
    arr.write(0, &[1, 1, 1, 1]);
    arr.write(0, &[0, 0, 0, 0]);
    assert_eq!(arr.read(0), vec![0, 0, 0, 0]);
}

#[test]
fn test_array_shape() {
    let arr = SRAMArray::new(8, 16);
    assert_eq!(arr.shape(), (8, 16));
}

#[test]
#[should_panic(expected = "rows must be >= 1")]
fn test_array_zero_rows_panics() {
    SRAMArray::new(0, 4);
}

#[test]
#[should_panic(expected = "cols must be >= 1")]
fn test_array_zero_cols_panics() {
    SRAMArray::new(4, 0);
}

#[test]
#[should_panic(expected = "out of range")]
fn test_array_read_out_of_range() {
    let arr = SRAMArray::new(4, 4);
    arr.read(4);
}

#[test]
#[should_panic(expected = "does not match cols")]
fn test_array_write_wrong_length() {
    let mut arr = SRAMArray::new(4, 4);
    arr.write(0, &[1, 0, 1]); // too short
}
