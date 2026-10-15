//! Full independent path-command goldens for rounding, clamping and degeneracy.
use super::*;
use diagram_ir::{RailroadConfig,LayoutedRailroadDiagram,LayoutedRailroadRule,LayoutedRailroadPath};
#[test]
fn railroad_arc_emits_exact_portable_commands_styles_order_and_metadata() {
    use PathCommand::{MoveTo as M,LineTo as L,QuadTo as Q};
    let normal=vec![Point{x:0.0,y:0.0},Point{x:20.0,y:0.0},Point{x:20.0,y:20.0},Point{x:40.0,y:20.0}];
    let duplicate=vec![Point{x:0.0,y:0.0},Point{x:0.0,y:0.0},Point{x:1e-300,y:0.0}];
    let cases=vec![
        (0.0,normal.clone(),vec![M{x:0.0,y:0.0},L{x:20.0,y:0.0},L{x:20.0,y:20.0},L{x:40.0,y:20.0}]),
        (5.0,normal.clone(),vec![M{x:0.0,y:0.0},L{x:15.0,y:0.0},Q{cx:20.0,cy:0.0,x:20.0,y:5.0},L{x:20.0,y:15.0},Q{cx:20.0,cy:20.0,x:25.0,y:20.0},L{x:40.0,y:20.0}]),
        (100.0,normal,vec![M{x:0.0,y:0.0},L{x:10.0,y:0.0},Q{cx:20.0,cy:0.0,x:20.0,y:10.0},L{x:20.0,y:10.0},Q{cx:20.0,cy:20.0,x:30.0,y:20.0},L{x:40.0,y:20.0}]),
        (0.0,duplicate.clone(),vec![M{x:0.0,y:0.0},L{x:0.0,y:0.0},L{x:1e-300,y:0.0}]),
        (5.0,duplicate,vec![M{x:0.0,y:0.0},L{x:1e-300,y:0.0}]),
    ];
    let mut cells=0;
    for (radius,points,commands) in cases {for loopback in [false,true] {
        let config=RailroadConfig {arc_radius:radius,line_color:"#123456".into(),stroke_width:3.0,show_markers:false,..Default::default()};
        let layout=LayoutedRailroadDiagram {width:100.0,height:60.0,title:None,accessibility_title:None,accessibility_description:None,
            config,rules:vec![LayoutedRailroadRule{name:"r".into(),y:0.0,height:60.0,elements:vec![],
                paths:vec![LayoutedRailroadPath{points:points.clone(),loopback}]}]};
        let original=layout.clone();let options=make_opts(&FakeShaper,&FakeMetrics,&FakeResolver);
        let scene=diagram_to_paint_railroad(&layout,&options);assert_eq!(layout,original);
        let kinds:Vec<_>=scene.instructions.iter().map(|i|match i{PaintInstruction::Path(_)=>"path",PaintInstruction::GlyphRun(_)=>"glyph",_=>"other"}).collect();
        assert_eq!(kinds,vec!["path","glyph"]);
        let expected=PaintPath {base:PaintBase::default(),commands:commands.clone(),fill:Some("none".into()),fill_rule:None,
            stroke:Some("#123456".into()),stroke_width:Some(3.0),stroke_cap:Some(StrokeCap::Round),stroke_join:Some(StrokeJoin::Round),
            stroke_dash:loopback.then(||vec![4.0,3.0]),stroke_dash_offset:None};
        assert_eq!(scene.instructions[0],PaintInstruction::Path(expected));
        assert_eq!(scene.metadata,Some(HashMap::from([
            ("railroad.compactMode".into(),"false".into()),
            ("railroad.padding".into(),"10".into()),("railroad.verticalSeparation".into(),"8".into()),
            ("railroad.horizontalSeparation".into(),"10".into()),("railroad.arcRadius".into(),radius.to_string()),
            ("railroad.fontSize".into(),"14".into()),("railroad.fontFamily".into(),"monospace".into()),
            ("railroad.lineColor".into(),"#123456".into()),("railroad.strokeWidth".into(),"3".into()),
            ("railroad.markerFill".into(),"#000000".into()),("railroad.showMarkers".into(),"false".into()),
            ("railroad.markerRadius".into(),"5".into()),
        ])));cells+=1;
    }}
    assert_eq!(cells,10);
}
