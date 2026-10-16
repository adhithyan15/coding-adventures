//! Exercise the private postcondition with independently forged resolved values.
use super::*;
fn empty()->LayoutedWardleyDiagram { LayoutedWardleyDiagram {width:900.0,height:600.0,
title:None,config:diagram_ir::WardleyConfig{width:900.0,height:600.0,padding:48.0,node_radius:6.0,
node_label_offset:8.0,axis_font_size:12.0,label_font_size:10.0,show_grid:false,use_max_width:true,theme:diagram_ir::WardleyTheme {background_color:"#ffffff".into(),axis_color:"#000000".into(),axis_text_color:"#222222".into(),grid_color:"rgba(100, 100, 100, 0.2)".into(),component_fill:"#ffffff".into(),component_stroke:"#000000".into(),component_label_color:"#222222".into(),link_stroke:"#000000".into(),evolution_stroke:"#dc3545".into()}},
stages:vec![],nodes:vec![],links:vec![],evolves:vec![],notes:vec![],annotations_box:None,annotations:vec![]} }
#[test]
fn wardley_postcondition_rejects_each_nonfinite_geometry_or_style_route() {
    for which in 0..8 {
        let mut d=empty();
        match which {
            0=>d.width=f64::NAN,
            1=>d.height=f64::INFINITY,
            2=>d.nodes.push(LayoutedWardleyNode{id:"a".into(),label:"A".into(),position:Point{x:f64::NAN,y:0.0},anchor:true}),
            3=>d.links.push(LayoutedWardleyLink{from:Point{x:0.0,y:0.0},to:Point{x:f64::INFINITY,y:0.0}}),
            4=>d.evolves.push(LayoutedWardleyEvolution{from:Point{x:0.0,y:f64::NAN},to:Point{x:0.0,y:0.0}}),
            5=>d.config.node_radius=1.5e308,
            6=>d.config.axis_font_size=1.75e308,
            _=>d.config.label_font_size= -1.0,
        }
        assert_eq!(validate_wardley_geometry(&d),Err(WardleyLayoutError::DerivedGeometry),"route{which}");
        eprintln!("WARDLEY-DERIVED route{which}");
    }
    assert_eq!(validate_wardley_geometry(&empty()),Ok(()));
}
