//! Independent, bounded Venn admission and complete geometry controls.
use super::*;
use diagram_ir::{VennConfig, VennRegion, VennStyle, VennText, MAX_VENN_CANVAS};

fn example() -> VennDiagram {
    VennDiagram { config: VennConfig { width: 400.0, height: 300.0, padding: 25.0,
        use_max_width: false, use_debug_layout: true, ..Default::default() }, title: None,
        regions: vec![
            VennRegion { sets: vec!["A".into()], size: 1.0, label: Some("Alpha".into()), style: VennStyle::default() },
            VennRegion { sets: vec!["B".into()], size: 1.0, label: None, style: VennStyle::default() },
            VennRegion { sets: vec!["A".into(), "B".into()], size: 0.5, label: Some("AB".into()), style: VennStyle::default() }],
        texts: vec![VennText { sets: vec!["A".into(), "B".into()], id: "note".into(), label: Some("Both".into()), style: VennStyle::default() }] }
}

#[test]
fn venn_checked_scalars_precede_empty_and_titled_arithmetic() {
    for title in [None, Some("Title".to_string())] {
        for empty in [false, true] {
            for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, -1.0, 65_537.0, 1e308] {
                for field in [VennLayoutError::Width, VennLayoutError::Height, VennLayoutError::Padding] {
                    let mut d = example(); d.title = title.clone();
                    if empty { d.regions.clear(); d.texts.clear(); }
                    match field { VennLayoutError::Width => d.config.width = value,
                        VennLayoutError::Height => d.config.height = value,
                        VennLayoutError::Padding => d.config.padding = value, _ => unreachable!() }
                    assert_eq!(try_layout_venn(&d), Err(field), "{field:?} {value} empty={empty}");
                }
            }
            for field in [VennLayoutError::Width, VennLayoutError::Height] {
                let mut d = VennDiagram { title: title.clone(), ..Default::default() };
                if field == VennLayoutError::Width { d.config.width = 0.0; } else { d.config.height = 0.0; }
                assert_eq!(try_layout_venn(&d), Err(field));
            }
        }
    }
}

#[test]
fn venn_derived_region_coordinates_are_checked() {
    for size in [-1.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let mut d = example(); d.regions[0].size = size;
        assert_eq!(try_layout_venn(&d), Err(VennLayoutError::DerivedGeometry), "size={size}");
    }
}

#[test]
fn venn_huge_finite_union_witness_is_refused_before_centroid_sum() {
    let mut d = example(); d.config.width = 1e308; d.config.height = 1e308; d.config.padding = 0.0;
    d.regions = ["A", "B", "C", "D"].into_iter().map(|id| VennRegion {
        sets: vec![id.into()], size: 1.0, label: None, style: Default::default() }).collect();
    d.regions.push(VennRegion { sets: vec!["A".into(), "B".into(), "C".into(), "D".into()],
        size: 1.0, label: Some("Overflow".into()), style: Default::default() });
    assert_eq!(try_layout_venn(&d), Err(VennLayoutError::Width));
}

#[test]
#[should_panic(expected = "unsupported Venn geometry")]
fn venn_compatibility_wrapper_rejects_unsupported_typed_ir() {
    let mut d = example(); d.config.height = f64::NAN; layout_venn(&d);
}

#[test]
fn venn_complete_two_set_geometry_and_boolean_transport_are_exact() {
    // Available side250, orbit40, radius62.5. For two sets the centers are
    // (240,150)/(160,150); outward label offset36.25 and union center(200,150).
    for max_width in [false, true] { for debug in [false, true] {
        let mut d = example(); d.config.use_max_width = max_width; d.config.use_debug_layout = debug;
        let expected = LayoutedVennDiagram { width: 400.0, height: 300.0, title: None, config: d.config.clone(),
            circles: vec![
                LayoutedVennCircle { id: "A".into(), label: "Alpha".into(), cx: 240.0, cy: 150.0, radius: 62.5, style: Default::default() },
                LayoutedVennCircle { id: "B".into(), label: "B".into(), cx: 160.0, cy: 150.0, radius: 62.5, style: Default::default() }],
            labels: vec![
                LayoutedVennLabel {font_size:12.0, text: "Alpha".into(), x: 276.25, y: 150.0, style: Default::default() },
                LayoutedVennLabel {font_size:12.0, text: "B".into(), x: 123.75, y: 150.0, style: Default::default() },
                LayoutedVennLabel {font_size:12.0, text: "AB".into(), x: 200.0, y: 150.0, style: Default::default() },
                LayoutedVennLabel {font_size:10.0, text: "Both".into(), x: 200.0, y: 170.0, style: Default::default() }], debug_circles: if debug {vec![LayoutedVennDebugCircle {cx:200.0,cy:150.0,radius:22.5}]} else {vec![]},
            debug_cells: if debug {vec![LayoutedVennDebugCell {x:178.625,y:141.75,width:42.75,height:42.75}]} else {vec![]} };
        assert_eq!(try_layout_venn(&d), Ok(expected.clone()));
        assert_eq!(layout_venn(&d), expected);
    }}
}

#[test]
fn venn_admitted_padding_is_retained_while_placement_uses_canvas_quarter() {
    let mut d = example(); d.config.padding = MAX_VENN_CANVAS;
    let g = try_layout_venn(&d).unwrap();
    assert_eq!(g.config, d.config);
    assert_eq!((g.circles[0].cx, g.circles[1].cx, g.circles[0].radius), (224.0, 176.0, 37.5));
    assert_eq!(g.labels.iter().map(|l| (l.x,l.y)).collect::<Vec<_>>(),
        vec![(245.75,150.0),(154.25,150.0),(200.0,150.0),(200.0,170.0)]);
    for value in [f64::MIN_POSITIVE, 1.0, MAX_VENN_CANVAS] {
        for title in [None, Some("Small".into())] {
            d.config.width=value; d.config.height=value; d.config.padding=0.0; d.title=title;
            assert!(try_layout_venn(&d).is_ok());
        }
    }
}
