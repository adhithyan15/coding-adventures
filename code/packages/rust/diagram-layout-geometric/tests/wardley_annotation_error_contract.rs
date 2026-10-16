use diagram_ir::*;
use diagram_layout_geometric::{try_layout_wardley,WardleyLayoutError};
fn config()->WardleyConfig {WardleyConfig {width:900.0,height:600.0,padding:48.0,node_radius:6.0,node_label_offset:8.0,axis_font_size:12.0,label_font_size:10.0,show_grid:false,use_max_width:true,theme:WardleyTheme {annotation_stroke:"#000000".into(),annotation_text_color:"#222222".into(),annotation_fill:"#ffffff".into(),background_color:"#ffffff".into(),axis_color:"#000000".into(),axis_text_color:"#222222".into(),grid_color:"rgba(100, 100, 100, 0.2)".into(),component_fill:"#ffffff".into(),component_stroke:"#000000".into(),component_label_color:"#222222".into(),link_stroke:"#000000".into(),evolution_stroke:"#dc3545".into()}}}
fn diagram()->WardleyDiagram {WardleyDiagram{accessibility_title:None,accessibility_description:None,title:None,config:config(),stages:vec![],nodes:vec![],links:vec![],evolves:vec![],notes:vec![],annotations_box:None,annotations:vec![],forces:vec![],pipelines:vec![],stage_boundaries:vec![]}}
fn invalidate(d:&mut WardleyDiagram,box_field:bool,axis:usize,value:f64,populated:bool) {
 if populated {d.nodes.push(WardleyNode{label_offset_x:None,label_offset_y:None,source_strategy:None,inertia:false,id:"a".into(),label:"A".into(),visibility:0.5,evolution:0.5,anchor:false,pipeline_parent:false});}
 let point=(if axis==0 {value}else{0.5},if axis==1 {value}else{0.5});
 if box_field {d.annotations_box=Some(point);}else{d.annotations.push(WardleyAnnotation{number:3,text:"N".into(),coordinates:vec![point]});}
}

#[test]
fn every_annotation_axis_has_an_exact_named_refusal() {
 for box_field in [true,false] {for populated in [false,true] {for axis in 0..2 {for value in [-0.1,1.1,f64::NAN,f64::INFINITY,f64::NEG_INFINITY,f64::MAX] {
 let mut d=diagram();invalidate(&mut d,box_field,axis,value,populated);
 let want=if box_field {WardleyLayoutError::AnnotationBoxCoordinates}else{WardleyLayoutError::AnnotationCoordinates};
 assert_eq!(try_layout_wardley(&d),Err(want),"named {box_field}/{populated}/{axis}/{value}");
 eprintln!("WARDLEY-ANNOTATION-NAMED {box_field}/{populated}/{axis}/{value}");
 }}}}
}
#[test]
fn annotations_have_declared_input_and_reference_error_priority() {
 for (case,want) in [(0,WardleyLayoutError::Width),(1,WardleyLayoutError::EvolutionTarget),(2,WardleyLayoutError::NoteCoordinates),(3,WardleyLayoutError::AnnotationBoxCoordinates),(4,WardleyLayoutError::AnnotationCoordinates),(5,WardleyLayoutError::UnknownLinkEndpoint)] {
 let mut d=diagram();d.annotations_box=Some((f64::NAN,0.5));d.annotations.push(WardleyAnnotation{number:1,text:"A".into(),coordinates:vec![(0.5,f64::NAN)]});d.links.push(WardleyLink{dashed:false,label:None,flow:None,source:"missing".into(),target:"missing".into()});
 if case==0 {d.config.width=f64::NAN;}
 if case==1 {d.evolves.push(WardleyEvolution{component:"missing".into(),target:f64::NAN});}
 if case==2 {d.notes.push(WardleyNote{text:"N".into(),visibility:f64::NAN,evolution:0.5});}
 if case>=4 {d.annotations_box=None;}
 if case>=5 {d.annotations.clear();}
 assert_eq!(try_layout_wardley(&d),Err(want),"priority {case}");eprintln!("WARDLEY-ANNOTATION-PRIORITY {case}");
 }
}
