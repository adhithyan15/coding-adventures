//! Numeric corner goldens and absolute authored routes, separate from the
//! production corner walker. These bounded controls do not claim visual parity.
use super::*;
use PathCommand::{CubicTo as C, LineTo as L, MoveTo as M};

fn commands_close(actual: &[PathCommand], expected: &[PathCommand]) {
    assert_eq!(actual.len(), expected.len(), "{actual:?}");
    for (actual, expected) in actual.iter().zip(expected) {
        let (a, b) = match (actual, expected) {
            (M { x, y }, M { x: ex, y: ey }) | (L { x, y }, L { x: ex, y: ey }) =>
                (vec![*x, *y], vec![*ex, *ey]),
            (C { cx1, cy1, cx2, cy2, x, y }, C { cx1: a, cy1: b, cx2: c, cy2: d, x: e, y: f }) =>
                (vec![*cx1, *cy1, *cx2, *cy2, *x, *y], vec![*a, *b, *c, *d, *e, *f]),
            _ => panic!("command variant mismatch: {actual:?} != {expected:?}"),
        };
        for (a, b) in a.into_iter().zip(b) {
            assert!(a.is_finite() && (a-b).abs() <= 1e-12, "{actual:?} != {expected:?}");
        }
    }
}

#[test]
fn railroad_circular_eight_orientations_keep_numeric_controls_clamps_and_radial_error() {
    let mut cells = 0;
    // Orthogonal reflections/rotations of one independently tabulated corner
    // cover every ordered pair of perpendicular cardinal directions.
    for (swap, sx, sy) in [
        (false,1.0,1.0),(false,1.0,-1.0),(false,-1.0,1.0),(false,-1.0,-1.0),
        (true,1.0,1.0),(true,1.0,-1.0),(true,-1.0,1.0),(true,-1.0,-1.0),
    ] {
        let xy = |x: f64, y: f64| if swap {(sx*y,sy*x)} else {(sx*x,sy*y)};
        let point = |x, y| {let (x,y)=xy(x,y);Point{x,y}};
        let points = [point(-10.0,0.0),point(0.0,0.0),point(0.0,20.0)];
        // (requested radius, actual cut, independently rounded tangent inset).
        for (radius, cut, inset) in [(0.25,0.25,0.111_928_812_542_301_6),
            (4.0,4.0,1.790_861_000_676_825_7),(100.0,5.0,2.238_576_250_846_032)] {
            let (mx,my)=xy(-10.0,0.0);let (lx,ly)=xy(-cut,0.0);
            let (a,b)=xy(-inset,0.0);let (c,d)=xy(0.0,inset);
            let (ex,ey)=xy(0.0,cut);let (fx,fy)=xy(0.0,20.0);
            let actual=rounded_railroad_path(&points,radius);
            commands_close(&actual,&[M{x:mx,y:my},L{x:lx,y:ly},
                C{cx1:a,cy1:b,cx2:c,cy2:d,x:ex,y:ey},L{x:fx,y:fy}]);
            let C{cx1,cy1,cx2,cy2,x,y}=actual[2] else {panic!("missing numeric cubic")};
            let (center_x,center_y)=xy(-cut,cut);
            // Evaluate the emitted polynomial, rather than merely checking its
            // command type. A quadratic corner misses this radial tolerance.
            for step in 0..=64 {
                let t=step as f64/64.0;let u=1.0-t;
                let px=u*u*u*lx+3.0*u*u*t*cx1+3.0*u*t*t*cx2+t*t*t*x;
                let py=u*u*u*ly+3.0*u*u*t*cy1+3.0*u*t*t*cy2+t*t*t*y;
                let radial=(px-center_x).hypot(py-center_y);
                assert!((radial-cut).abs() <= cut*0.0003+1e-14);
            }
            cells+=1;
        }
    }
    assert_eq!(cells,24);
}

#[test]
fn railroad_circular_public_short_paths_and_collinear_reversals_keep_their_commands() {
    use diagram_ir::{LayoutedRailroadDiagram,LayoutedRailroadPath,LayoutedRailroadRule,RailroadConfig};
    for (points,commands) in [
        (vec![],vec![]),
        (vec![Point{x:3.0,y:4.0}],vec![M{x:3.0,y:4.0}]),
        (vec![Point{x:3.0,y:4.0},Point{x:8.0,y:4.0}],vec![M{x:3.0,y:4.0},L{x:8.0,y:4.0}]),
        (vec![Point{x:0.0,y:0.0},Point{x:5.0,y:0.0},Point{x:10.0,y:0.0}],vec![M{x:0.0,y:0.0},L{x:5.0,y:0.0},L{x:10.0,y:0.0}]),
        (vec![Point{x:0.0,y:0.0},Point{x:5.0,y:0.0},Point{x:0.0,y:0.0}],vec![M{x:0.0,y:0.0},L{x:5.0,y:0.0},L{x:0.0,y:0.0}]),
    ] {
        let diagram=LayoutedRailroadDiagram{width:100.0,height:50.0,title:None,accessibility_title:None,accessibility_description:None,
            config:RailroadConfig{show_markers:false,..Default::default()},rules:vec![LayoutedRailroadRule{name:"r".into(),y:0.0,height:40.0,start_marker:Point{x:70.0,y:20.0},end_marker:Point{x:90.0,y:20.0},elements:vec![],paths:vec![LayoutedRailroadPath{points,loopback:false}]}]};
        let scene=diagram_to_paint_railroad(&diagram,&make_opts(&FakeShaper,&FakeMetrics,&FakeResolver));
        let paths:Vec<_>=scene.instructions.iter().filter_map(|i|if let PaintInstruction::Path(p)=i{Some(p)}else{None}).collect();
        assert_eq!(paths.len(),1);commands_close(&paths[0].commands,&commands);
    }
}

// These goldens use one authored coordinate frame: font20, padding0, radius4,
// vertical gap2. Decimal tangent offsets are independent numeric fixtures.
fn bypass_golden() -> Vec<PathCommand> { vec![
    M{x:130.0,y:35.5},L{x:132.0,y:35.5},
    C{cx1:133.104_569_499_661_6,cy1:35.5,cx2:134.0,cy2:34.604_569_499_661_6,x:134.0,y:33.5},
    L{x:134.0,y:20.0},
    C{cx1:134.0,cy1:17.790_861_000_676_824,cx2:135.790_861_000_676_8,cy2:16.0,x:138.0,y:16.0},
    L{x:201.0,y:16.0},
    C{cx1:203.209_138_999_323_2,cy1:16.0,cx2:205.0,cy2:17.790_861_000_676_824,x:205.0,y:20.0},
    L{x:205.0,y:33.5},
    C{cx1:205.0,cy1:34.604_569_499_661_6,cx2:205.895_430_500_338_4,cy2:35.5,x:207.0,y:35.5},
    L{x:209.0,y:35.5},
] }
fn loop_golden() -> Vec<PathCommand> { vec![
    M{x:201.0,y:35.5},L{x:203.0,y:35.5},
    C{cx1:204.104_569_499_661_6,cy1:35.5,cx2:205.0,cy2:36.395_430_500_338_4,x:205.0,y:37.5},
    L{x:205.0,y:51.0},
    C{cx1:205.0,cy1:53.209_138_999_323_17,cx2:203.209_138_999_323_2,cy2:55.0,x:201.0,y:55.0},
    L{x:138.0,y:55.0},
    C{cx1:135.790_861_000_676_8,cy1:55.0,cx2:134.0,cy2:53.209_138_999_323_17,x:134.0,y:51.0},
    L{x:134.0,y:37.5},
    C{cx1:134.0,cy1:36.395_430_500_338_4,cx2:134.895_430_500_338_4,cy2:35.5,x:136.0,y:35.5},
    L{x:138.0,y:35.5},
] }

#[test]
fn railroad_circular_authored_choice_keeps_both_branch_directions_and_absolute_controls() {
    let left=vec![M{x:130.0,y:40.0},L{x:132.0,y:40.0},
        C{cx1:133.104_569_499_661_6,cy1:40.0,cx2:134.0,cy2:39.104_569_499_661_6,x:134.0,y:38.0},
        L{x:134.0,y:29.5},
        C{cx1:134.0,cy1:28.395_430_500_338_4,cx2:134.895_430_500_338_4,cy2:27.5,x:136.0,y:27.5},
        L{x:138.0,y:27.5}];
    let right=vec![M{x:201.0,y:27.5},L{x:203.0,y:27.5},
        C{cx1:204.104_569_499_661_6,cy1:27.5,cx2:205.0,cy2:28.395_430_500_338_4,x:205.0,y:29.5},
        L{x:205.0,y:38.0},
        C{cx1:205.0,cy1:39.104_569_499_661_6,cx2:205.895_430_500_338_4,cy2:40.0,x:207.0,y:40.0},
        L{x:209.0,y:40.0}];
    let reflect=|commands:&[PathCommand]|commands.iter().map(|c|match *c {
        M{x,y}=>M{x,y:80.0-y},L{x,y}=>L{x,y:80.0-y},
        C{cx1,cy1,cx2,cy2,x,y}=>C{cx1,cy1:80.0-cy1,cx2,cy2:80.0-cy2,x,y:80.0-y},
        _=>panic!("unexpected golden command"),
    }).collect::<Vec<_>>();
    for compact in [false,true] {
        let input=mermaid_parser::parse_railroad(&format!("---\nconfig:\n  railroad:\n    compactMode: {compact}\n    padding: 0\n    fontSize: 20\n    verticalSeparation: 2\n    arcRadius: 4\n---\nrailroad-beta\nr = choice(terminal(\"A\"), terminal(\"B\"));")).unwrap();
        let layout=diagram_layout_hierarchy::try_layout_railroad(&input).unwrap();
        let original=layout.clone();let rule=&layout.rules[0];
        assert_eq!(rule.elements.iter().map(|e|(e.x,e.y,e.width,e.height)).collect::<Vec<_>>(),
            vec![(84.6,16.0,63.0,23.0),(84.6,41.0,63.0,23.0)]);
        let scene=diagram_to_paint_railroad(&layout,&make_opts(&FakeShaper,&FakeMetrics,&FakeResolver));
        assert_eq!(layout,original);
        let paths:Vec<_>=scene.instructions.iter().filter_map(|i|if let PaintInstruction::Path(p)=i{Some(p)}else{None}).collect();
        let expected=vec![left.clone(),right.clone(),reflect(&left),reflect(&right),
            vec![M{x:115.0,y:40.0},L{x:130.0,y:40.0}],vec![M{x:209.0,y:40.0},L{x:214.0,y:40.0}]];
        assert_eq!(paths.len(),expected.len());
        for (path,golden) in paths.into_iter().zip(expected) {
            commands_close(&path.commands,&translated_name_origin(&golden));assert_eq!(path.stroke_dash,None);
            assert_eq!((path.stroke.as_deref(),path.stroke_width),(Some("#000000"),Some(2.0)));
        }
        assert_eq!(scene.metadata,Some(HashMap::from([
            ("railroad.compactMode".into(),compact.to_string()),("railroad.padding".into(),"0".into()),
            ("railroad.verticalSeparation".into(),"2".into()),("railroad.horizontalSeparation".into(),"10".into()),
            ("railroad.arcRadius".into(),"4".into()),("railroad.fontSize".into(),"20".into()),
            ("railroad.fontFamily".into(),"monospace".into()),("railroad.lineColor".into(),"#000000".into()),
            ("railroad.strokeWidth".into(),"2".into()),("railroad.markerFill".into(),"#000000".into()),
            ("railroad.showMarkers".into(),"true".into()),("railroad.markerRadius".into(),"5".into()),
        ])));
    }
}

#[test]
fn railroad_circular_authored_bypass_and_loopback_keep_absolute_geometry_and_compact_state() {
    let mut cells=0;
    for compact in [false,true] {for (expression,loopback,bypass) in [
        ("optional(terminal(\"A\"))",false,true),
        ("oneOrMore(terminal(\"A\"))",true,false),
        ("zeroOrMore(terminal(\"A\"))",true,true),
    ] {
        let source=format!("---\nconfig:\n  railroad:\n    compactMode: {compact}\n    padding: 0\n    fontSize: 20\n    verticalSeparation: 2\n    arcRadius: 4\n---\nrailroad-beta\nr = {expression};");
        let input=mermaid_parser::parse_railroad(&source).unwrap();
        let layout=diagram_layout_hierarchy::try_layout_railroad(&input).unwrap();let original=layout.clone();
        let scene=diagram_to_paint_railroad(&layout,&make_opts(&FakeShaper,&FakeMetrics,&FakeResolver));
        assert_eq!(layout,original);assert_eq!(layout.rules[0].elements.len(),1);
        let e=&layout.rules[0].elements[0];assert_eq!((e.x,e.y,e.width,e.height),(84.6,24.0,63.0,23.0));
        let paths:Vec<_>=scene.instructions.iter().filter_map(|i|if let PaintInstruction::Path(p)=i{Some(p)}else{None}).collect();
        let mut expected=Vec::new();
        if loopback {expected.push((loop_golden(),Some(vec![4.0,3.0])));}
        expected.push((vec![M{x:130.0,y:35.5},L{x:138.0,y:35.5}],None));
        expected.push((vec![M{x:201.0,y:35.5},L{x:209.0,y:35.5}],None));
        if bypass {expected.push((bypass_golden(),None));}
        expected.push((vec![M{x:115.0,y:35.5},L{x:130.0,y:35.5}],None));
        expected.push((vec![M{x:209.0,y:35.5},L{x:214.0,y:35.5}],None));
        assert_eq!(paths.len(),expected.len());
        for (actual,(commands,dash)) in paths.iter().zip(expected) {
            commands_close(&actual.commands,&translated_name_origin(&commands));assert_eq!(actual.stroke_dash,dash);
            assert_eq!((actual.stroke.as_deref(),actual.stroke_width),(Some("#000000"),Some(2.0)));
        }
        assert_eq!(scene.metadata,Some(HashMap::from([
            ("railroad.compactMode".into(),compact.to_string()),("railroad.padding".into(),"0".into()),
            ("railroad.verticalSeparation".into(),"2".into()),("railroad.horizontalSeparation".into(),"10".into()),
            ("railroad.arcRadius".into(),"4".into()),("railroad.fontSize".into(),"20".into()),
            ("railroad.fontFamily".into(),"monospace".into()),("railroad.lineColor".into(),"#000000".into()),
            ("railroad.strokeWidth".into(),"2".into()),("railroad.markerFill".into(),"#000000".into()),
            ("railroad.showMarkers".into(),"true".into()),("railroad.markerRadius".into(),"5".into()),
        ])));cells+=1;
    }}
    assert_eq!(cells,6);
}

// Authored font20/name "r =" reserves36.6+20 and expression ingress20:
// new origin76.6, old130. Root marker ports above are separately tabulated.
fn translated_name_origin(commands:&[PathCommand])->Vec<PathCommand> {
    commands.iter().map(|c|match *c {
        M{x,y}=>M{x:x-53.4,y}, L{x,y}=>L{x:x-53.4,y},
        C{cx1,cy1,cx2,cy2,x,y}=>C{cx1:cx1-53.4,cy1,cx2:cx2-53.4,cy2,x:x-53.4,y},
        _=>panic!("unexpected authored command"),
    }).collect()
}
