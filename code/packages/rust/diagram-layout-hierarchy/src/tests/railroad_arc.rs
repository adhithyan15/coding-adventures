//! Radius admission and explicit midpoint ports, not just enclosing boxes.
use super::*;
use diagram_ir::{RailroadConfig, RailroadDiagram, RailroadExpression as E, RailroadRule};

fn leaf() -> E { E::Terminal("A".into()) }
fn repeat(min: usize) -> E { E::Repetition { element: Box::new(leaf()), min, max: None } }
fn diagram(config: RailroadConfig, definition: Option<E>, title: bool) -> RailroadDiagram {
    RailroadDiagram { title: title.then(|| "Grammar".into()), accessibility_title: None,
        accessibility_description: None, config,
        rules: definition.map(|definition| vec![RailroadRule { name: "r".into(), definition }]).unwrap_or_default() }
}
fn points(values: &[(f64, f64)]) -> Vec<Point> {
    values.iter().map(|&(x,y)| Point { x,y }).collect()
}
fn same(a: &Point, b: &Point) {
    assert!((a.x-b.x).abs() <= 1e-8 && (a.y-b.y).abs() <= 1e-8, "port {a:?} != {b:?}");
}

#[test]
fn railroad_arc_typed_refusals_precede_empty_and_geometry_handling() {
    let mut failures = Vec::new(); let mut cells = 0;
    for radius in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, -1.0, 65537.0, 1e20, 1e308] {
        for definition in [None, Some(leaf()), Some(E::Optional(Box::new(leaf()))), Some(repeat(1))] {
            for title in [false, true] {
                let input = diagram(RailroadConfig { arc_radius: radius, ..Default::default() }, definition.clone(), title);
                let result = std::panic::catch_unwind(|| try_layout_railroad(&input));
                if !matches!(&result, Ok(Err(error)) if matches!(error, RailroadLayoutError::UnsupportedStyle { field: "arcRadius" })
                    && error.to_string() == "Railroad arcRadius must be finite and in 0..=65536") {
                    failures.push(format!("radius={radius},title={title}: {result:?}"));
                }
                cells += 1;
            }
        }
    }
    assert_eq!(cells, 56);
    assert!(failures.is_empty(), "{} precise arc refusals failed: {failures:?}", failures.len());
}

#[test]
fn railroad_arc_valid_scalars_retain_config_and_finite_enclosed_geometry() {
    let mut cells = 0;
    for radius in [0.0, 1e-300, 0.5, 4.0, 10.0, 24.0, 65536.0] {
        for expression in [leaf(), E::Optional(Box::new(leaf())), repeat(0), repeat(1), E::Choice(vec![leaf(), leaf()])] {
            let config = RailroadConfig { arc_radius: radius, ..Default::default() };
            let input = diagram(config.clone(), Some(expression), false);
            let layout = try_layout_railroad(&input).unwrap();
            assert_eq!(layout.config, config);
            assert!([layout.width,layout.height].iter().all(|v| v.is_finite() && *v >= 0.0));
            for rule in &layout.rules {
                for p in rule.paths.iter().flat_map(|p| &p.points) {
                    assert!(p.x.is_finite() && p.y.is_finite() && p.x >= 0.0 && p.x <= layout.width
                        && p.y >= rule.y && p.y <= rule.y+rule.height, "{p:?} in {layout:?}");
                }
            }
            cells += 1;
        }
    }
    assert_eq!(cells, 35);
}

#[test]
fn railroad_arc_wrapper_boxes_and_every_ordered_port_have_independent_goldens() {
    // font20/padding1 gives a65x25 leaf. Radius5 reserves10 on each side:
    // an85x45 wrapper, child at110,210, main lane222.5, bypass200, loop245.
    let config = RailroadConfig { font_size:20.0, padding:1.0, arc_radius:5.0, ..Default::default() };
    for (kind,expression) in [(0,E::Optional(Box::new(leaf()))),(1,repeat(1)),(2,repeat(0))] {
        let mut elements=Vec::new(); let mut paths=Vec::new();
        let size=layout_railroad_expression(&expression,100.0,200.0,&config,&mut elements,&mut paths);
        assert_eq!(size,(85.0,45.0)); assert_eq!(measure_railroad_expression(&expression,&config),size);
        assert_eq!(elements.len(),1);
        assert_eq!((elements[0].x,elements[0].y,elements[0].width,elements[0].height),(110.0,210.0,65.0,25.0));
        let entry=points(&[(100.0,222.5),(110.0,222.5)]);
        let exit=points(&[(175.0,222.5),(185.0,222.5)]);
        let bypass=points(&[(100.0,222.5),(105.0,222.5),(105.0,200.0),(180.0,200.0),(180.0,222.5),(185.0,222.5)]);
        let loopback=points(&[(175.0,222.5),(180.0,222.5),(180.0,245.0),(105.0,245.0),(105.0,222.5),(110.0,222.5)]);
        let expected=if kind==0 {vec![(entry,false),(exit,false),(bypass,false)]}
            else if kind==1 {vec![(loopback,true),(entry,false),(exit,false)]}
            else {vec![(loopback,true),(entry,false),(exit,false),(bypass,false)]};
        assert_eq!(paths.iter().map(|p|(p.points.clone(),p.loopback)).collect::<Vec<_>>(),expected);
    }
}

#[test]
fn railroad_arc_nested_sequence_choice_and_root_connectors_meet_wrapper_ports() {
    for radius in [0.5,4.0,10.0,24.0] {
        let config=RailroadConfig { arc_radius:radius, ..Default::default() };
        // Check root ingress/egress against the wrapper's own main ports.
        for expression in [E::Optional(Box::new(leaf())),repeat(0),repeat(1)] {
            let layout=try_layout_railroad(&diagram(config.clone(),Some(expression.clone()),false)).unwrap();
            let paths=&layout.rules[0].paths; let last=paths.len()-2;
            let (entry,exit)=if matches!(expression,E::Optional(_)){(0,1)}else{(1,2)};
            same(paths[last].points.last().unwrap(),&paths[entry].points[0]);
            same(&paths[last+1].points[0],paths[exit].points.last().unwrap());
        }
        let expression=E::Sequence(vec![leaf(),E::Optional(Box::new(repeat(1))),leaf()]);
        let mut elements=Vec::new();let mut paths=Vec::new();
        layout_railroad_expression(&expression,100.0,200.0,&config,&mut elements,&mut paths);
        assert_eq!(paths.len(),8);
        same(paths[6].points.last().unwrap(),&paths[3].points[0]);
        same(&paths[7].points[0],paths[4].points.last().unwrap());
        same(paths[3].points.last().unwrap(),&paths[1].points[0]);
        same(&paths[4].points[0],paths[2].points.last().unwrap());
        let expression=E::Choice(vec![E::Optional(Box::new(leaf())),leaf()]);
        let mut elements=Vec::new();let mut paths=Vec::new();
        layout_railroad_expression(&expression,100.0,200.0,&config,&mut elements,&mut paths);
        assert_eq!(paths.len(),7);
        same(paths[3].points.last().unwrap(),&paths[0].points[0]);
        same(&paths[4].points[0],paths[1].points.last().unwrap());
    }
}
