use hardware_vm::HardwareVm;

// ---------------------------------------------------------------------------
// Helpers: build HIR JSON for common test circuits
// ---------------------------------------------------------------------------

fn adder_hir_json() -> &'static str {
    r#"{
      "format": "HIR", "version": "0.1.0", "top": "adder",
      "modules": {
        "adder": {
          "name": "adder",
          "ports": [
            {"name": "a",   "type": {"kind": "vec", "width": 4}, "direction": "in"},
            {"name": "b",   "type": {"kind": "vec", "width": 4}, "direction": "in"},
            {"name": "sum", "type": {"kind": "vec", "width": 5}, "direction": "out"}
          ],
          "cont_assigns": [{
            "target": {"kind": "port_ref", "name": "sum"},
            "rhs":    {"kind": "binary", "op": "+",
                       "lhs": {"kind": "port_ref", "name": "a"},
                       "rhs": {"kind": "port_ref", "name": "b"}}
          }]
        }
      }
    }"#
}

fn and_hir_json() -> &'static str {
    r#"{
      "format": "HIR", "version": "0.1.0", "top": "andgate",
      "modules": {
        "andgate": {
          "name": "andgate",
          "ports": [
            {"name": "a", "type": {"kind": "bit"}, "direction": "in"},
            {"name": "b", "type": {"kind": "bit"}, "direction": "in"},
            {"name": "y", "type": {"kind": "bit"}, "direction": "out"}
          ],
          "cont_assigns": [{
            "target": {"kind": "port_ref", "name": "y"},
            "rhs":    {"kind": "binary", "op": "AND",
                       "lhs": {"kind": "port_ref", "name": "a"},
                       "rhs": {"kind": "port_ref", "name": "b"}}
          }]
        }
      }
    }"#
}

// ---------------------------------------------------------------------------
// Basic tests
// ---------------------------------------------------------------------------

#[test]
fn test_adder_zero_plus_zero() {
    let hir = hdl_ir::Hir::from_json(adder_hir_json()).unwrap();
    let vm = HardwareVm::new(hir).unwrap();
    assert_eq!(vm.read("sum"), 0);
}

#[test]
fn test_adder_three_plus_five() {
    let hir = hdl_ir::Hir::from_json(adder_hir_json()).unwrap();
    let mut vm = HardwareVm::new(hir).unwrap();
    vm.set_input("a", 3).unwrap();
    vm.set_input("b", 5).unwrap();
    assert_eq!(vm.read("sum"), 8);
}

#[test]
fn test_adder_reactive_update() {
    let hir = hdl_ir::Hir::from_json(adder_hir_json()).unwrap();
    let mut vm = HardwareVm::new(hir).unwrap();
    vm.set_input("a", 1).unwrap();
    vm.set_input("b", 1).unwrap();
    assert_eq!(vm.read("sum"), 2);
    // Change a; sum should update immediately.
    vm.set_input("a", 7).unwrap();
    assert_eq!(vm.read("sum"), 8);
}

#[test]
fn test_and_gate_truth_table() {
    let hir = hdl_ir::Hir::from_json(and_hir_json()).unwrap();
    let mut vm = HardwareVm::new(hir).unwrap();
    for (a, b, expected) in [(0,0,0),(0,1,0),(1,0,0),(1,1,1)] {
        vm.set_input("a", a).unwrap();
        vm.set_input("b", b).unwrap();
        assert_eq!(vm.read("y"), expected, "AND({a},{b})");
    }
}

#[test]
fn test_set_output_fails() {
    let hir = hdl_ir::Hir::from_json(adder_hir_json()).unwrap();
    let mut vm = HardwareVm::new(hir).unwrap();
    let result = vm.set_input("sum", 0);
    assert!(result.is_err());
}

#[test]
fn test_set_unknown_signal_fails() {
    let hir = hdl_ir::Hir::from_json(adder_hir_json()).unwrap();
    let mut vm = HardwareVm::new(hir).unwrap();
    let result = vm.set_input("nonexistent", 0);
    assert!(result.is_err());
}

#[test]
fn test_subscribe_receives_events() {
    use std::sync::{Arc, Mutex};
    let hir = hdl_ir::Hir::from_json(adder_hir_json()).unwrap();
    let mut vm = HardwareVm::new(hir).unwrap();

    let events: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let events_clone = events.clone();
    vm.subscribe(move |ev| {
        events_clone.lock().unwrap().push(ev.signal.clone());
    });

    vm.set_input("a", 3).unwrap();
    vm.set_input("b", 5).unwrap();

    let captured = events.lock().unwrap();
    // a, b, and sum should each have triggered events.
    assert!(captured.contains(&"a".to_string()));
    assert!(captured.contains(&"b".to_string()));
    assert!(captured.contains(&"sum".to_string()));
}

#[test]
fn test_force_and_release() {
    let hir = hdl_ir::Hir::from_json(adder_hir_json()).unwrap();
    let mut vm = HardwareVm::new(hir).unwrap();
    vm.set_input("a", 3).unwrap();
    vm.set_input("b", 5).unwrap();
    assert_eq!(vm.read("sum"), 8);

    vm.force("sum", 99);
    assert_eq!(vm.read("sum"), 99);

    vm.release("sum");
    // After release, sum should revert to a+b = 8.
    assert_eq!(vm.read("sum"), 8);
}

#[test]
fn test_stats_event_count() {
    let hir = hdl_ir::Hir::from_json(adder_hir_json()).unwrap();
    let mut vm = HardwareVm::new(hir).unwrap();
    vm.set_input("a", 1).unwrap();
    let stats = vm.stats();
    assert!(stats.event_count > 0);
    assert!(stats.cont_assign_runs > 0);
}

#[test]
fn test_no_update_on_same_value() {
    use std::sync::{Arc, Mutex};
    let hir = hdl_ir::Hir::from_json(adder_hir_json()).unwrap();
    let mut vm = HardwareVm::new(hir).unwrap();

    let count = Arc::new(Mutex::new(0u32));
    let count_c = count.clone();
    vm.subscribe(move |_| { *count_c.lock().unwrap() += 1; });

    vm.set_input("a", 0).unwrap(); // same value (0) — no event
    let c0 = *count.lock().unwrap();
    vm.set_input("a", 7).unwrap(); // different — events fire
    let c1 = *count.lock().unwrap();
    assert!(c1 > c0);
}

#[test]
fn test_ternary_expr() {
    let hir_json = r#"{
      "format": "HIR", "version": "0.1.0", "top": "mux",
      "modules": {
        "mux": {
          "name": "mux",
          "ports": [
            {"name": "sel", "type": {"kind": "bit"},            "direction": "in"},
            {"name": "a",   "type": {"kind": "vec", "width": 4}, "direction": "in"},
            {"name": "b",   "type": {"kind": "vec", "width": 4}, "direction": "in"},
            {"name": "y",   "type": {"kind": "vec", "width": 4}, "direction": "out"}
          ],
          "cont_assigns": [{
            "target": {"kind": "port_ref", "name": "y"},
            "rhs": {"kind": "ternary",
                    "cond":      {"kind": "port_ref", "name": "sel"},
                    "then_expr": {"kind": "port_ref", "name": "a"},
                    "else_expr": {"kind": "port_ref", "name": "b"}}
          }]
        }
      }
    }"#;
    let hir = hdl_ir::Hir::from_json(hir_json).unwrap();
    let mut vm = HardwareVm::new(hir).unwrap();
    vm.set_input("a", 3).unwrap();
    vm.set_input("b", 7).unwrap();
    vm.set_input("sel", 0).unwrap();
    assert_eq!(vm.read("y"), 7); // sel=0 → b
    vm.set_input("sel", 1).unwrap();
    assert_eq!(vm.read("y"), 3); // sel=1 → a
}
