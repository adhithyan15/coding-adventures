//! Exercise the private postcondition with independently forged resolved values.
use super::*;
fn empty()->LayoutedWardleyDiagram { LayoutedWardleyDiagram {accessibility_title:None,accessibility_description:None,width:900.0,height:600.0,
title:None,config:diagram_ir::WardleyConfig{width:900.0,height:600.0,padding:48.0,node_radius:6.0,
node_label_offset:8.0,axis_font_size:12.0,label_font_size:10.0,show_grid:false,use_max_width:true,theme:diagram_ir::WardleyTheme {annotation_stroke:"#000000".into(),annotation_text_color:"#222222".into(),annotation_fill:"#ffffff".into(),background_color:"#ffffff".into(),axis_color:"#000000".into(),axis_text_color:"#222222".into(),grid_color:"rgba(100, 100, 100, 0.2)".into(),component_fill:"#ffffff".into(),component_stroke:"#000000".into(),component_label_color:"#222222".into(),link_stroke:"#000000".into(),evolution_stroke:"#dc3545".into()}},
stages:vec![],nodes:vec![],links:vec![],evolves:vec![],notes:vec![],annotations_box:None,annotations:vec![],forces:vec![],pipelines:vec![],stage_boundaries:vec![]} }
#[test]
fn wardley_postcondition_rejects_each_nonfinite_geometry_or_style_route() {
    for which in 0..8 {
        let mut d=empty();
        match which {
            0=>d.width=f64::NAN,
            1=>d.height=f64::INFINITY,
            2=>d.nodes.push(LayoutedWardleyNode{label_offset_x:None,label_offset_y:None,source_strategy:None,inertia:false,id:"a".into(),label:"A".into(),position:Point{x:f64::NAN,y:0.0},anchor:true,pipeline_parent:false}),
            3=>d.links.push(LayoutedWardleyLink{from:Point{x:0.0,y:0.0},to:Point{x:f64::INFINITY,y:0.0},dashed:false,label:None,label_position:Point{x:0.0,y:0.0},label_angle:0.0,flow:None}),
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

#[test]
fn wardley_link_label_postcondition_owns_rotation_translation_and_text_box() {
    let mut supported=empty();
    supported.links.push(LayoutedWardleyLink{from:Point{x:100.0,y:100.0},to:Point{x:200.0,y:200.0},dashed:false,
        label:Some("demand".into()),label_position:Point{x:150.0,y:150.0},label_angle:45.0,flow:None});
    assert_eq!(validate_wardley_geometry(&supported),Ok(()));
    for route in 0..5 {
        let mut forged=supported.clone();
        match route {
            0=>forged.links[0].label_angle=f64::NAN,
            1=>forged.links[0].label_position.y=f64::INFINITY,
            2=>{forged.links[0].label_position.x=f64::MAX;forged.links[0].label_position.y=0.0;forged.links[0].label_angle=180.0;},
            3=>{forged.links[0].from.x=f64::MAX;forged.links[0].to.x= -f64::MAX;},
            _=>forged.config.label_font_size=1e308,
        }
        assert_eq!(validate_wardley_geometry(&forged),Err(WardleyLayoutError::DerivedGeometry),"rotation/text route={route}");
    }
}


#[test]
fn node_decorator_postcondition_checks_only_enabled_paint_parameters() {
    let mut plain=empty();
    plain.config.node_radius=1.2e308;
    plain.nodes.push(LayoutedWardleyNode{label_offset_x:None,label_offset_y:None,source_strategy:None,inertia:false,
        id:"a".into(),label:"A".into(),position:Point{x:0.0,y:0.0},anchor:false,pipeline_parent:false});
    assert_eq!(validate_wardley_geometry(&plain),Ok(()),"ordinary finite radius has no decorator envelope");
    for route in 0..3 {
        let mut decorated=plain.clone();
        match route {
            0=>decorated.nodes[0].source_strategy=Some(diagram_ir::WardleySourceStrategy::Buy),
            1=>decorated.nodes[0].inertia=true,
            _=>decorated.nodes[0].pipeline_parent=true,
        }
        assert_eq!(validate_wardley_geometry(&decorated),Err(WardleyLayoutError::DerivedGeometry),"extreme route={route}");
    }
    for strategy in [diagram_ir::WardleySourceStrategy::Buy,diagram_ir::WardleySourceStrategy::Market] {
        let mut decorated=plain.clone();decorated.config.node_radius=6.0;
        decorated.nodes[0].source_strategy=Some(strategy);decorated.nodes[0].inertia=true;
        assert_eq!(validate_wardley_geometry(&decorated),Ok(()));
        decorated.config.node_radius=1e307;decorated.nodes[0].position.y= -f64::MAX;
        assert_eq!(validate_wardley_geometry(&decorated),Err(WardleyLayoutError::DerivedGeometry),"inertia lower endpoint");
    }
    let mut market=plain;market.config.node_radius=1e307;
    market.nodes[0].position.y= -f64::MAX;market.nodes[0].source_strategy=Some(diagram_ir::WardleySourceStrategy::Market);
    assert_eq!(validate_wardley_geometry(&market),Err(WardleyLayoutError::DerivedGeometry),"Market top dot");
}
