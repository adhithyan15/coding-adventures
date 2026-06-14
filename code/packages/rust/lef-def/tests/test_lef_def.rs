use lef_def::{
    write_cells_lef_str, write_def_str, write_tech_lef_str,
    CellLef, Component, Def, DefPin, Direction, LayerDef, Net,
    PinDef, PinPort, Rect, Row, Segment, SiteDef, TechLef, Use,
};

// ---------------------------------------------------------------------------
// Model tests
// ---------------------------------------------------------------------------

#[test]
fn test_rect_area() {
    let r = Rect::new(0.0, 0.0, 4.0, 2.72);
    assert!((r.area() - 10.88).abs() < 1e-6);
}

#[test]
fn test_def_defaults() {
    let d = Def::new("adder4");
    assert_eq!(d.version, "5.8");
    assert_eq!(d.units_microns, 1000);
}

#[test]
fn test_component_new() {
    let c = Component::new("xor_0", "sky130_fd_sc_hd__xor2_1");
    assert!(!c.placed);
    assert_eq!(c.orientation, "N");
}

// ---------------------------------------------------------------------------
// DEF writer
// ---------------------------------------------------------------------------

fn simple_def() -> Def {
    let mut d = Def::new("adder4");
    d.die_area = Some(Rect::new(0.0, 0.0, 40.0, 30.0));
    d.rows.push(Row {
        name: "row_0".into(),
        site: "unithd".into(),
        origin_x: 10.0,
        origin_y: 10.0,
        orientation: "N".into(),
        num_x: 80,
        num_y: 1,
        step_x: 0.46,
        step_y: 0.0,
    });
    d.components.push(Component {
        name: "xor_0".into(),
        cell_type: "sky130_fd_sc_hd__xor2_1".into(),
        placed: true,
        location_x: Some(10.0),
        location_y: Some(10.0),
        orientation: "N".into(),
    });
    d.pins.push(DefPin {
        name: "a".into(),
        net: "a".into(),
        direction: Direction::Input,
        use_: Use::Signal,
        layer: Some("met2".into()),
        rect: Some(Rect::new(-0.5, 5.0, 0.0, 5.2)),
    });
    d.nets.push(Net {
        name: "n0".into(),
        connections: vec![("xor_0".into(), "A".into()), ("and_0".into(), "A".into())],
        routed_segments: vec![Segment {
            layer: "met1".into(),
            points: vec![(10.0, 10.0), (12.0, 10.0)],
        }],
    });
    d
}

#[test]
fn test_def_contains_design_name() {
    let s = write_def_str(&simple_def());
    assert!(s.contains("DESIGN adder4"), "DEF:\n{s}");
}

#[test]
fn test_def_contains_diearea() {
    let s = write_def_str(&simple_def());
    assert!(s.contains("DIEAREA"), "DEF:\n{s}");
}

#[test]
fn test_def_contains_row() {
    let s = write_def_str(&simple_def());
    assert!(s.contains("ROW row_0 unithd"), "DEF:\n{s}");
}

#[test]
fn test_def_placed_component() {
    let s = write_def_str(&simple_def());
    assert!(s.contains("PLACED"), "DEF:\n{s}");
    assert!(s.contains("xor_0"), "DEF:\n{s}");
}

#[test]
fn test_def_pins_section() {
    let s = write_def_str(&simple_def());
    assert!(s.contains("PINS 1"), "DEF:\n{s}");
    assert!(s.contains("DIRECTION INPUT"), "DEF:\n{s}");
}

#[test]
fn test_def_nets_section() {
    let s = write_def_str(&simple_def());
    assert!(s.contains("NETS 1"), "DEF:\n{s}");
    assert!(s.contains("ROUTED met1"), "DEF:\n{s}");
}

#[test]
fn test_def_ends_with_end_design() {
    let s = write_def_str(&simple_def());
    assert!(s.trim_end().ends_with("END DESIGN"), "DEF:\n{s}");
}

// ---------------------------------------------------------------------------
// LEF writer
// ---------------------------------------------------------------------------

#[test]
fn test_tech_lef_has_version() {
    let tech = TechLef::new();
    let s = write_tech_lef_str(&tech);
    assert!(s.contains("VERSION 5.8"), "LEF:\n{s}");
}

#[test]
fn test_tech_lef_layer_section() {
    let mut tech = TechLef::new();
    tech.layers.push(LayerDef {
        name: "met1".into(),
        r#type: "ROUTING".into(),
        direction: Some("HORIZONTAL".into()),
        pitch: 0.34,
        width: 0.14,
        spacing: 0.14,
    });
    let s = write_tech_lef_str(&tech);
    assert!(s.contains("LAYER met1"), "LEF:\n{s}");
    assert!(s.contains("DIRECTION HORIZONTAL"), "LEF:\n{s}");
}

#[test]
fn test_cells_lef_macro_block() {
    let mut cell = CellLef::new("sky130_fd_sc_hd__inv_1");
    cell.width = 1.38;
    cell.height = 2.72;
    cell.site = "unithd".into();
    cell.pins.push(PinDef {
        name: "A".into(),
        direction: Direction::Input,
        use_: Use::Signal,
        ports: vec![PinPort {
            layer: "li1".into(),
            rect: Rect::new(0.0, 0.0, 0.14, 0.28),
        }],
    });
    let s = write_cells_lef_str(&[cell]);
    assert!(s.contains("MACRO sky130_fd_sc_hd__inv_1"), "LEF:\n{s}");
    assert!(s.contains("PIN A"), "LEF:\n{s}");
    assert!(s.contains("DIRECTION INPUT"), "LEF:\n{s}");
    assert!(s.contains("SIZE 1.38 BY 2.72"), "LEF:\n{s}");
}

#[test]
fn test_site_def_in_tech_lef() {
    let mut tech = TechLef::new();
    tech.sites.push(SiteDef {
        name: "unithd".into(),
        class: "CORE".into(),
        width: 0.46,
        height: 2.72,
    });
    let s = write_tech_lef_str(&tech);
    assert!(s.contains("SITE unithd"), "LEF:\n{s}");
    assert!(s.contains("SIZE 0.46 BY 2.72"), "LEF:\n{s}");
}
