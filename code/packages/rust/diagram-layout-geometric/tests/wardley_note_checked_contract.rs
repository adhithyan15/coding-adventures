use diagram_ir::{WardleyConfig,WardleyDiagram,WardleyNote,WardleyTheme,LayoutedWardleyDiagram,LayoutedWardleyNote,Point};
use diagram_layout_geometric::{layout_wardley,try_layout_wardley};
fn theme()->WardleyTheme {WardleyTheme {background_color:"#ffffff".into(),axis_color:"#000000".into(),axis_text_color:"#222222".into(),grid_color:"rgba(100, 100, 100, 0.2)".into(),component_fill:"#ffffff".into(),component_stroke:"#000000".into(),component_label_color:"#222222".into(),link_stroke:"#000000".into(),evolution_stroke:"#dc3545".into()}}
fn config()->WardleyConfig {WardleyConfig {width:900.0,height:600.0,padding:48.0,node_radius:6.0,node_label_offset:8.0,axis_font_size:12.0,label_font_size:10.0,show_grid:false,use_max_width:true,theme:theme()}}
fn diagram()->WardleyDiagram {WardleyDiagram{title:None,config:config(),stages:vec![],nodes:vec![],links:vec![],evolves:vec![],notes:vec![],annotations_box:None,annotations:vec![],forces:vec![],pipelines:vec![]}}

#[test]
fn valid_note_edges_have_literal_complete_layout_and_order() {
 let mut d=diagram();d.notes=vec![WardleyNote{text:"L".into(),visibility:0.0,evolution:0.0},WardleyNote{text:"R".into(),visibility:1.0,evolution:1.0},WardleyNote{text:"M".into(),visibility:0.5,evolution:0.5}];
 let expected=LayoutedWardleyDiagram {width:900.0,height:600.0,title:None,config:config(),stages:vec![],nodes:vec![],links:vec![],evolves:vec![],notes:vec![
 LayoutedWardleyNote{text:"L".into(),position:Point{x:48.0,y:552.0}},
 LayoutedWardleyNote{text:"R".into(),position:Point{x:852.0,y:48.0}},
 LayoutedWardleyNote{text:"M".into(),position:Point{x:450.0,y:300.0}}],annotations_box:None,annotations:vec![],forces:vec![],pipelines:vec![]};
 assert_eq!(try_layout_wardley(&d),Ok(expected.clone()));assert_eq!(layout_wardley(&d),expected);
 eprintln!("WARDLEY-NOTE-LAYOUT 3literalpoints/wholelayout");
}
#[test]
fn invalid_note_axes_are_refused_before_mapping_empty_or_populated() {
 for populated in [false,true] {for axis in 0..2 {for value in [-0.1,1.1,f64::NAN,f64::INFINITY,f64::NEG_INFINITY,f64::MAX] {
  let mut d=diagram();
  if populated {d.nodes.push(diagram_ir::WardleyNode{id:"a".into(),label:"A".into(),visibility:0.5,evolution:0.5,anchor:false,pipeline_parent:false});}
  d.notes.push(WardleyNote{text:"N".into(),visibility:if axis==0 {value}else{0.5},evolution:if axis==1 {value}else{0.5}});
  assert!(try_layout_wardley(&d).is_err(),"checked note {populated}/{axis}/{value}");
  assert!(std::panic::catch_unwind(||layout_wardley(&d)).is_err(),"compat note {populated}/{axis}/{value}");
  eprintln!("WARDLEY-NOTE-REFUSAL {populated}/{axis}/{value}");
 }}}
}
