//! Caller-owned style values must be refused before clamp or paint arithmetic.
use super::*;
use diagram_ir::{RailroadConfig,RailroadDiagram,RailroadExpression as E,RailroadRule};
fn input(config:RailroadConfig,empty:bool,title:bool)->RailroadDiagram {
    RailroadDiagram{title:title.then(||"Grammar".into()),accessibility_title:None,accessibility_description:None,config,
        rules:if empty{vec![]}else{vec![RailroadRule{name:"r".into(),definition:E::Terminal("A".into())}]}}
}
#[test]
fn railroad_style_checked_refusals_precede_clamp_and_never_panic() {
    let mut failures=Vec::new();let mut cells=0;
    for (field,index) in [("fontSize",0),("strokeWidth",1),("markerRadius",2)] {
        let mut values=vec![f64::NAN,f64::INFINITY,f64::NEG_INFINITY,-1.0,65537.0,1e20,1e308];
        if index==0 {values.push(0.0);}
        for value in values {for empty in [false,true] {for title in [false,true] {
            let mut config=RailroadConfig::default();match index {0=>config.font_size=value,1=>config.stroke_width=value,_=>config.marker_radius=value}
            let diagram=input(config,empty,title);let result=std::panic::catch_unwind(||try_layout_railroad(&diagram));
            let expected=if index==0 {"Railroad fontSize must be finite and in (0, 65536]".to_string()}
                else {format!("Railroad {field} must be finite and in 0..=65536")};
            if !matches!(&result,Ok(Err(e)) if e.to_string()==expected) {failures.push(format!("{field}={value},empty={empty},title={title}: {result:?}"));}
            cells+=1;
        }}}
    }
    assert_eq!(cells,88);assert!(failures.is_empty(),"{} typed style refusals failed: {failures:?}",failures.len());
}
#[test]
fn railroad_style_preserves_all_valid_scalars_and_font_measured_boxes() {
    let mut cells=0;
    for font_size in [0.5,14.0,18.25,64.0,65536.0] {for stroke_width in [0.0,0.5,2.0,65536.0] {for marker_radius in [0.0,0.5,5.0,65536.0] {
        let config=RailroadConfig{font_size,stroke_width,marker_radius,padding:0.0,..Default::default()};
        let diagram=input(config.clone(),false,false);let layout=try_layout_railroad(&diagram).unwrap();
        assert_eq!(layout.config,config);let e=&layout.rules[0].elements[0];
        assert_eq!((e.width,e.height),(font_size*3.15,font_size*1.15));
        assert!([layout.width,layout.height,e.x,e.y,e.width,e.height].iter().all(|v|v.is_finite()&&*v>=0.0));cells+=1;
    }}}
    assert_eq!(cells,80);
}

#[test]
fn railroad_explicit_marker_points_cross_the_complete_derived_geometry_gate() {
    let layout=try_layout_railroad(&input(RailroadConfig::default(),false,false)).unwrap();
    assert!(railroad_geometry_is_valid(&layout));
    let mut cells=0;
    for marker in [0,1] {for axis in [0,1] {for value in [f64::NAN,f64::INFINITY,f64::NEG_INFINITY] {
        let mut forged=layout.clone();
        let point=if marker==0 {&mut forged.rules[0].start_marker}else{&mut forged.rules[0].end_marker};
        if axis==0 {point.x=value;}else{point.y=value;}
        assert!(!railroad_geometry_is_valid(&forged),"marker={marker},axis={axis},value={value}");
        cells+=1;
    }}}
    assert_eq!(cells,12);
    // A negative finite marker coordinate is separate visual-clearance debt;
    // the gate promises finite points, not arbitrary marker enclosure.
    let mut finite=layout.clone();finite.rules[0].start_marker.x= -1.0;
    assert!(railroad_geometry_is_valid(&finite));
}
