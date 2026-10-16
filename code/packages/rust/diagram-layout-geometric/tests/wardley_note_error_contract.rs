use diagram_ir::{WardleyConfig,WardleyDiagram,WardleyNote,WardleyTheme};
use diagram_layout_geometric::{try_layout_wardley,WardleyLayoutError};
fn theme()->WardleyTheme {WardleyTheme {background_color:"#ffffff".into(),axis_color:"#000000".into(),axis_text_color:"#222222".into(),grid_color:"rgba(100, 100, 100, 0.2)".into(),component_fill:"#ffffff".into(),component_stroke:"#000000".into(),component_label_color:"#222222".into(),link_stroke:"#000000".into(),evolution_stroke:"#dc3545".into()}}
fn config()->WardleyConfig {WardleyConfig {width:900.0,height:600.0,padding:48.0,node_radius:6.0,node_label_offset:8.0,axis_font_size:12.0,label_font_size:10.0,show_grid:false,use_max_width:true,theme:theme()}}
fn diagram()->WardleyDiagram {WardleyDiagram{title:None,config:config(),stages:vec![],nodes:vec![],links:vec![],evolves:vec![],notes:vec![],annotations_box:None,annotations:vec![],forces:vec![]}}

#[test]
fn note_coordinate_errors_are_exact_and_keep_scalar_priority() {
 for populated in [false,true] {for axis in 0..2 {for value in [-0.1,1.1,f64::NAN,f64::INFINITY,f64::NEG_INFINITY,f64::MAX] {
  let mut d=diagram();if populated {d.nodes.push(diagram_ir::WardleyNode{id:"a".into(),label:"A".into(),visibility:0.5,evolution:0.5,anchor:false});}
  d.notes.push(WardleyNote{text:"N".into(),visibility:if axis==0 {value}else{0.5},evolution:if axis==1 {value}else{0.5}});
  assert_eq!(try_layout_wardley(&d),Err(WardleyLayoutError::NoteCoordinates));
  eprintln!("WARDLEY-NOTE-NAMED {populated}/{axis}/{value}");
 }}}
 let mut d=diagram();d.notes.push(WardleyNote{text:"N".into(),visibility:f64::NAN,evolution:0.5});
 d.config.width=0.0;assert_eq!(try_layout_wardley(&d),Err(WardleyLayoutError::Width));
 d.config.width=900.0;d.nodes.push(diagram_ir::WardleyNode{id:"a".into(),label:"A".into(),visibility:f64::NAN,evolution:0.5,anchor:false});
 assert_eq!(try_layout_wardley(&d),Err(WardleyLayoutError::NodeCoordinates));
 d.nodes[0].visibility=0.5;d.evolves.push(diagram_ir::WardleyEvolution{component:"A".into(),target:f64::NAN});
 assert_eq!(try_layout_wardley(&d),Err(WardleyLayoutError::EvolutionTarget));
 d.evolves.clear();d.links.push(diagram_ir::WardleyLink{source:"missing".into(),target:"A".into()});
 assert_eq!(try_layout_wardley(&d),Err(WardleyLayoutError::NoteCoordinates));
 eprintln!("WARDLEY-NOTE-PRIORITY 4");
}
