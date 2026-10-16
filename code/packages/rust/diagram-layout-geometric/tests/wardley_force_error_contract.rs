use diagram_ir::*;
use diagram_layout_geometric::{try_layout_wardley,WardleyLayoutError};
fn config()->WardleyConfig {WardleyConfig {width:900.0,height:600.0,padding:48.0,node_radius:6.0,node_label_offset:8.0,axis_font_size:12.0,label_font_size:10.0,show_grid:false,use_max_width:true,theme:WardleyTheme {background_color:"#ffffff".into(),axis_color:"#000000".into(),axis_text_color:"#222222".into(),grid_color:"rgba(100, 100, 100, 0.2)".into(),component_fill:"#ffffff".into(),component_stroke:"#000000".into(),component_label_color:"#222222".into(),link_stroke:"#000000".into(),evolution_stroke:"#dc3545".into()}}}
fn diagram()->WardleyDiagram {WardleyDiagram{title:None,config:config(),stages:vec![],nodes:vec![],links:vec![],evolves:vec![],notes:vec![],annotations_box:None,annotations:vec![],forces:vec![]}}
fn invalidate(d:&mut WardleyDiagram,kind:WardleyForceKind,axis:usize,value:f64,populated:bool) {
 if populated {d.nodes.push(WardleyNode{id:"a".into(),label:"A".into(),visibility:0.5,evolution:0.5,anchor:false});d.forces.push(WardleyForce{label:"Good".into(),visibility:0.0,evolution:1.0,kind:kind.clone()});}
 d.forces.push(WardleyForce{label:"Unknown".into(),visibility:if axis==0 {value}else{0.5},evolution:if axis==1 {value}else{0.5},kind});
}
#[test]
fn every_force_axis_has_exact_named_refusal() {
 for kind in [WardleyForceKind::Accelerator,WardleyForceKind::Deaccelerator] {for populated in [false,true] {for axis in 0..2 {for value in [-0.1,1.1,f64::NAN,f64::INFINITY,f64::NEG_INFINITY,f64::MAX] {
 let mut d=diagram();invalidate(&mut d,kind.clone(),axis,value,populated);assert_eq!(try_layout_wardley(&d),Err(WardleyLayoutError::ForceCoordinates));eprintln!("WARDLEY-FORCE-NAMED {kind:?}/{populated}/{axis}/{value}");
 }}}}
}
#[test]
fn force_error_priority_preserves_all_old_gates_before_references() {
 for (case,want) in [(0,WardleyLayoutError::Width),(1,WardleyLayoutError::NodeCoordinates),(2,WardleyLayoutError::EvolutionTarget),(3,WardleyLayoutError::NoteCoordinates),(4,WardleyLayoutError::AnnotationBoxCoordinates),(5,WardleyLayoutError::AnnotationCoordinates),(6,WardleyLayoutError::ForceCoordinates),(7,WardleyLayoutError::UnknownLinkEndpoint),(8,WardleyLayoutError::UnknownEvolutionComponent)] {
 let mut d=diagram();invalidate(&mut d,WardleyForceKind::Accelerator,0,f64::NAN,false);
 d.links.push(WardleyLink{source:"missing".into(),target:"missing".into()});d.evolves.push(WardleyEvolution{component:"missing".into(),target:0.5});
 match case {0=>d.config.width=f64::NAN,1=>d.nodes.push(WardleyNode{id:"a".into(),label:"A".into(),visibility:f64::NAN,evolution:0.5,anchor:false}),2=>d.evolves[0].target=f64::NAN,3=>d.notes.push(WardleyNote{text:"N".into(),visibility:f64::NAN,evolution:0.5}),4=>d.annotations_box=Some((f64::NAN,0.5)),5=>d.annotations.push(WardleyAnnotation{number:1,text:"A".into(),coordinates:vec![(0.5,f64::NAN)]}),7=>d.forces.clear(),8=>{d.forces.clear();d.links.clear();},_=>{}}
 assert_eq!(try_layout_wardley(&d),Err(want),"priority {case}");eprintln!("WARDLEY-FORCE-PRIORITY {case}");
 }
}
