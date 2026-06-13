use asic_routing::{pin_at, route, PinAccess, RouteOptions};
use lef_def::{Def, Rect};

fn placed_def() -> Def {
    let mut d = Def::new("adder4");
    d.die_area = Some(Rect::new(0.0, 0.0, 20.0, 20.0));
    d
}

fn two_pin_net() -> (String, Vec<PinAccess>) {
    (
        "n0".into(),
        vec![
            PinAccess { cell_instance: "xor_0".into(), pin_name: "A".into(), x: 1, y: 2 },
            PinAccess { cell_instance: "and_0".into(), pin_name: "A".into(), x: 5, y: 2 },
        ],
    )
}

#[test]
fn test_route_two_pin_net() {
    let def = placed_def();
    let nets = vec![two_pin_net()];
    let (routed, report) = route(&def, &nets, None).unwrap();
    assert_eq!(report.nets_routed, 1);
    assert_eq!(report.nets_failed, 0);
    assert!(!routed.nets[0].routed_segments.is_empty());
}

#[test]
fn test_segment_on_correct_layer() {
    let def = placed_def();
    let nets = vec![two_pin_net()];
    let (routed, _) = route(&def, &nets, None).unwrap();
    let seg = &routed.nets[0].routed_segments[0];
    assert_eq!(seg.layer, "met1");
}

#[test]
fn test_segment_connects_endpoints() {
    let def = placed_def();
    let nets = vec![two_pin_net()];
    let (routed, _) = route(&def, &nets, None).unwrap();
    let seg = &routed.nets[0].routed_segments[0];
    // Segment should start near grid (1, 2) × pitch and end near (5, 2) × pitch.
    let pitch = 0.34;
    let first = seg.points[0];
    let last = seg.points[seg.points.len() - 1];
    assert!((first.0 - 1.0 * pitch).abs() < 1e-6);
    assert!((last.0 - 5.0 * pitch).abs() < 1e-6);
}

#[test]
fn test_total_wire_length_positive() {
    let def = placed_def();
    let nets = vec![two_pin_net()];
    let (_, report) = route(&def, &nets, None).unwrap();
    assert!(report.total_wire_length > 0.0);
}

#[test]
fn test_no_die_area_error() {
    let mut def = Def::new("t");
    def.die_area = None;
    let result = route(&def, &[], None);
    assert!(result.is_err());
}

#[test]
fn test_single_pin_net_skipped() {
    let def = placed_def();
    let nets = vec![(
        "vdd".into(),
        vec![PinAccess { cell_instance: "cell_0".into(), pin_name: "VDD".into(), x: 0, y: 0 }],
    )];
    let (routed, report) = route(&def, &nets, None).unwrap();
    assert_eq!(report.nets_routed, 0);
    assert!(routed.nets[0].routed_segments.is_empty());
}

#[test]
fn test_three_pin_net() {
    let def = placed_def();
    let nets = vec![(
        "n1".into(),
        vec![
            PinAccess { cell_instance: "a".into(), pin_name: "Y".into(), x: 1, y: 1 },
            PinAccess { cell_instance: "b".into(), pin_name: "A".into(), x: 5, y: 1 },
            PinAccess { cell_instance: "c".into(), pin_name: "A".into(), x: 9, y: 1 },
        ],
    )];
    let (routed, report) = route(&def, &nets, None).unwrap();
    assert_eq!(report.nets_routed, 1);
    assert_eq!(routed.nets[0].routed_segments.len(), 2);
}

#[test]
fn test_pin_at_helper() {
    let p = pin_at("cell", "A", 0.68, 0.34, 0.34);
    assert_eq!(p.x, 2);
    assert_eq!(p.y, 1);
}

#[test]
fn test_custom_layer() {
    let def = placed_def();
    let opts = RouteOptions { pitch: 0.34, layer: "met2".into(), max_iters_per_net: 100_000 };
    let nets = vec![two_pin_net()];
    let (routed, _) = route(&def, &nets, Some(opts)).unwrap();
    assert_eq!(routed.nets[0].routed_segments[0].layer, "met2");
}
