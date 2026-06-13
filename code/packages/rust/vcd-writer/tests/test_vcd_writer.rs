use std::collections::HashMap;
use vcd_writer::{VcdWriter, attach, SignalEvent};

// ---------------------------------------------------------------------------
// Header generation
// ---------------------------------------------------------------------------

#[test]
fn test_header_contains_timescale() {
    let mut vcd = VcdWriter::new("1ns");
    vcd.end_definitions();
    let text = vcd.finish();
    assert!(text.contains("$timescale 1ns $end"), "expected timescale in header");
}

#[test]
fn test_header_contains_enddefinitions() {
    let mut vcd = VcdWriter::new("1ps");
    vcd.end_definitions();
    let text = vcd.finish();
    assert!(text.contains("$enddefinitions $end"));
}

#[test]
fn test_scope_appears_in_header() {
    let mut vcd = VcdWriter::new("1ps");
    vcd.open_scope("top");
    vcd.close_scope();
    vcd.end_definitions();
    let text = vcd.finish();
    assert!(text.contains("$scope module top $end"));
    assert!(text.contains("$upscope $end"));
}

#[test]
fn test_declare_var_appears_in_header() {
    let mut vcd = VcdWriter::new("1ps");
    vcd.open_scope("dut");
    let id = vcd.declare("clk", 1, "wire");
    vcd.close_scope();
    vcd.end_definitions();
    let text = vcd.finish();
    assert!(text.contains(&format!("$var wire 1 {id} clk $end")));
}

#[test]
fn test_declare_wide_var_includes_bit_range() {
    let mut vcd = VcdWriter::new("1ps");
    let id = vcd.declare("data", 8, "wire");
    vcd.end_definitions();
    let text = vcd.finish();
    assert!(text.contains(&format!("$var wire 8 {id} data [7:0] $end")));
}

// ---------------------------------------------------------------------------
// Time and value changes
// ---------------------------------------------------------------------------

#[test]
fn test_time_stamp_written() {
    let mut vcd = VcdWriter::new("1ps");
    let id = vcd.declare("clk", 1, "wire");
    vcd.end_definitions();
    vcd.time(0);
    vcd.value_change(&id, 0);
    vcd.time(5);
    vcd.value_change(&id, 1);
    let text = vcd.finish();
    assert!(text.contains("#0"));
    assert!(text.contains("#5"));
}

#[test]
fn test_scalar_value_change_format() {
    let mut vcd = VcdWriter::new("1ps");
    let id = vcd.declare("clk", 1, "wire");
    vcd.end_definitions();
    vcd.time(0);
    vcd.value_change(&id, 1);
    let text = vcd.finish();
    // 1-bit signal: "<value><id>\n"
    assert!(text.contains(&format!("1{id}")));
}

#[test]
fn test_vector_value_change_format() {
    let mut vcd = VcdWriter::new("1ps");
    let id = vcd.declare("bus", 4, "wire");
    vcd.end_definitions();
    vcd.time(0);
    vcd.value_change(&id, 0b1010);
    let text = vcd.finish();
    assert!(text.contains(&format!("b1010 {id}")));
}

#[test]
fn test_no_duplicate_emission_on_same_value() {
    let mut vcd = VcdWriter::new("1ps");
    let id = vcd.declare("sig", 1, "wire");
    vcd.end_definitions();
    vcd.time(0);
    vcd.value_change(&id, 0);
    vcd.time(5);
    vcd.value_change(&id, 0); // same — should not emit
    vcd.time(10);
    vcd.value_change(&id, 1); // changed — should emit
    let text = vcd.finish();
    // Only one #5 is written (the time stamp may appear even if no values change,
    // but more importantly, the value should only appear twice: once at t=0 (0), once at t=10 (1).
    let count_1 = text.matches(&format!("1{id}")).count();
    assert_eq!(count_1, 1, "value 1 should appear exactly once");
}

#[test]
fn test_value_change_at_convenience() {
    let mut vcd = VcdWriter::new("1ps");
    let id = vcd.declare("x", 4, "wire");
    vcd.end_definitions();
    vcd.value_change_at(100, &id, 0xf);
    let text = vcd.finish();
    assert!(text.contains("#100"));
    assert!(text.contains(&format!("b1111 {id}")));
}

#[test]
fn test_multiple_vars_unique_ids() {
    let mut vcd = VcdWriter::new("1ps");
    let id_a = vcd.declare("a", 1, "wire");
    let id_b = vcd.declare("b", 1, "wire");
    assert_ne!(id_a, id_b);
}

#[test]
fn test_text_before_finish() {
    let mut vcd = VcdWriter::new("1ps");
    vcd.end_definitions();
    assert!(vcd.text().contains("$enddefinitions $end"));
}

#[test]
fn test_dump_initial_block() {
    let mut vcd = VcdWriter::new("1ps");
    let id = vcd.declare("x", 1, "wire");
    vcd.end_definitions();
    let mut init = HashMap::new();
    init.insert(id.clone(), 0i64);
    vcd.dump_initial(&init);
    let text = vcd.finish();
    assert!(text.contains("$dumpvars"));
    assert!(text.contains("$end"));
}

// ---------------------------------------------------------------------------
// attach() helper
// ---------------------------------------------------------------------------

#[test]
fn test_attach_routes_known_signal() {
    let mut name_to_id = HashMap::new();
    name_to_id.insert("clk".to_string(), "!".to_string());
    let router = attach(name_to_id);
    let ev = SignalEvent { time: 10, signal: "clk".to_string(), new_value: 1 };
    let result = router(ev);
    assert_eq!(result, Some((10, "!".to_string(), 1)));
}

#[test]
fn test_attach_ignores_unknown_signal() {
    let router = attach(HashMap::new());
    let ev = SignalEvent { time: 5, signal: "ghost".to_string(), new_value: 1 };
    assert!(router(ev).is_none());
}

// ---------------------------------------------------------------------------
// Full round-trip
// ---------------------------------------------------------------------------

#[test]
fn test_full_vcd_output_structure() {
    let mut vcd = VcdWriter::new("1ps");
    vcd.open_scope("adder");
    let a_id = vcd.declare("a", 4, "wire");
    let b_id = vcd.declare("b", 4, "wire");
    let s_id = vcd.declare("sum", 5, "wire");
    vcd.close_scope();
    vcd.end_definitions();

    vcd.time(0);
    vcd.value_change(&a_id, 0);
    vcd.value_change(&b_id, 0);
    vcd.value_change(&s_id, 0);

    vcd.time(10);
    vcd.value_change(&a_id, 3);
    vcd.value_change(&b_id, 5);
    vcd.value_change(&s_id, 8);

    let text = vcd.finish();
    assert!(text.contains("$scope module adder $end"));
    assert!(text.contains("$upscope $end"));
    assert!(text.contains("#10"));
    assert!(text.contains("b11 ")); // 3 in binary
    assert!(text.contains("b101 ")); // 5 in binary
    assert!(text.contains("b1000 ")); // 8 in binary
}
