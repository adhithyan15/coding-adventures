use super::*;
#[test]
fn wardley_derived_annotation_points_have_exact_finite_gate() {
 for box_field in [true,false] {for populated in [false,true] {for axis in 0..2 {for value in [f64::NAN,f64::INFINITY,f64::NEG_INFINITY] {
 let mut layout=LayoutedWardleyDiagram {width:900.0,height:600.0,title:None,config:diagram_ir::WardleyConfig{width:900.0,height:600.0,padding:48.0,node_radius:6.0,node_label_offset:8.0,axis_font_size:12.0,label_font_size:10.0,show_grid:false,use_max_width:true,theme:diagram_ir::WardleyTheme{background_color:"#ffffff".into(),axis_color:"#000000".into(),axis_text_color:"#222222".into(),grid_color:"rgba(100, 100, 100, 0.2)".into(),component_fill:"#ffffff".into(),component_stroke:"#000000".into(),component_label_color:"#222222".into(),link_stroke:"#000000".into(),evolution_stroke:"#dc3545".into()}},stages:vec![],nodes:vec![],links:vec![],evolves:vec![],notes:vec![],annotations_box:None,annotations:vec![],forces:vec![]};
 if populated {layout.nodes.push(diagram_ir::LayoutedWardleyNode{id:"a".into(),label:"A".into(),position:Point{x:450.0,y:300.0},anchor:false});}
 let point=Point{x:if axis==0 {value}else{450.0},y:if axis==1 {value}else{300.0}};
 if box_field {layout.annotations_box=Some(point);}else{layout.annotations.push(diagram_ir::LayoutedWardleyAnnotation{number:1,text:"A".into(),positions:vec![point]});}
 assert_eq!(validate_wardley_geometry(&layout),Err(WardleyLayoutError::DerivedGeometry));
 eprintln!("WARDLEY-ANNOTATION-DERIVED {box_field}/{populated}/{axis}/{value}");
 }}}}
}
