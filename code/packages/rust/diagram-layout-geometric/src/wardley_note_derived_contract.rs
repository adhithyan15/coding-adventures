use super::*;
#[test]
fn wardley_derived_note_points_have_exact_finite_gate() {
 for axis in 0..2 {for value in [f64::NAN,f64::INFINITY,f64::NEG_INFINITY] {
  let mut layout=LayoutedWardleyDiagram {width:900.0,height:600.0,title:None,config:diagram_ir::WardleyConfig {width:900.0,height:600.0,padding:48.0,node_radius:6.0,node_label_offset:8.0,axis_font_size:12.0,label_font_size:10.0,show_grid:false,use_max_width:true,theme:diagram_ir::WardleyTheme {background_color:"#ffffff".into(),axis_color:"#000000".into(),axis_text_color:"#222222".into(),grid_color:"rgba(100, 100, 100, 0.2)".into(),component_fill:"#ffffff".into(),component_stroke:"#000000".into(),component_label_color:"#222222".into(),link_stroke:"#000000".into(),evolution_stroke:"#dc3545".into()}},stages:vec![],nodes:vec![],links:vec![],evolves:vec![],notes:vec![]};
  layout.notes.push(diagram_ir::LayoutedWardleyNote{text:"N".into(),position:Point{x:if axis==0 {value}else{0.5},y:if axis==1 {value}else{0.5}}});
  assert_eq!(validate_wardley_geometry(&layout),Err(WardleyLayoutError::DerivedGeometry));
  eprintln!("WARDLEY-NOTE-DERIVED {axis}/{value}");
 }}
}
