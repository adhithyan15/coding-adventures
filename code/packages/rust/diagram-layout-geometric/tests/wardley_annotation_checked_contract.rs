use diagram_ir::*;
use diagram_layout_geometric::{layout_wardley,try_layout_wardley};
fn config()->WardleyConfig {WardleyConfig {width:900.0,height:600.0,padding:48.0,node_radius:6.0,node_label_offset:8.0,axis_font_size:12.0,label_font_size:10.0,show_grid:false,use_max_width:true,theme:WardleyTheme {annotation_stroke:"#000000".into(),annotation_text_color:"#222222".into(),annotation_fill:"#ffffff".into(),background_color:"#ffffff".into(),axis_color:"#000000".into(),axis_text_color:"#222222".into(),grid_color:"rgba(100, 100, 100, 0.2)".into(),component_fill:"#ffffff".into(),component_stroke:"#000000".into(),component_label_color:"#222222".into(),link_stroke:"#000000".into(),evolution_stroke:"#dc3545".into()}}}
fn diagram()->WardleyDiagram {WardleyDiagram{accessibility_title:None,accessibility_description:None,title:None,config:config(),stages:vec![],nodes:vec![],links:vec![],evolves:vec![],notes:vec![],annotations_box:None,annotations:vec![],forces:vec![],pipelines:vec![],stage_boundaries:vec![]}}
fn invalidate(d:&mut WardleyDiagram,box_field:bool,axis:usize,value:f64,populated:bool) {
 if populated {d.nodes.push(WardleyNode{label_offset_x:None,label_offset_y:None,source_strategy:None,inertia:false,id:"a".into(),label:"A".into(),visibility:0.5,evolution:0.5,anchor:false,pipeline_parent:false});}
 let point=(if axis==0 {value}else{0.5},if axis==1 {value}else{0.5});
 if box_field {d.annotations_box=Some(point);}else{d.annotations.push(WardleyAnnotation{number:3,text:"N".into(),coordinates:vec![point]});}
}

#[test]
fn valid_annotation_edges_keep_literal_complete_layout_and_group_order() {
 let mut d=diagram();d.annotations_box=Some((0.25,0.75));d.annotations=vec![
 WardleyAnnotation{number:3,text:"Three".into(),coordinates:vec![(0.0,0.0),(0.5,0.5),(1.0,1.0)]},
 WardleyAnnotation{number:1,text:"".into(),coordinates:vec![(0.75,0.25)]}];
 let expected=LayoutedWardleyDiagram{width:900.0,height:600.0,title:None,config:config(),stages:vec![],nodes:vec![],links:vec![],evolves:vec![],notes:vec![],annotations_box:Some(Point{x:651.0,y:426.0}),annotations:vec![
 LayoutedWardleyAnnotation{number:3,text:"Three".into(),positions:vec![Point{x:48.0,y:552.0},Point{x:450.0,y:300.0},Point{x:852.0,y:48.0}]},
 LayoutedWardleyAnnotation{number:1,text:"".into(),positions:vec![Point{x:249.0,y:174.0}]}],forces:vec![],pipelines:vec![],stage_boundaries:vec![]};
 assert_eq!(try_layout_wardley(&d),Ok(expected.clone()));assert_eq!(layout_wardley(&d),expected);
 eprintln!("WARDLEY-ANNOTATION-LAYOUT wholelayout/box+fourmarkers/orderedgroups");
}
#[test]
fn invalid_annotation_axes_refuse_before_mapping_empty_or_populated() {
 for box_field in [true,false] {for populated in [false,true] {for axis in 0..2 {for value in [-0.1,1.1,f64::NAN,f64::INFINITY,f64::NEG_INFINITY,f64::MAX] {
 let mut d=diagram();invalidate(&mut d,box_field,axis,value,populated);
 assert!(try_layout_wardley(&d).is_err(),"legacy {box_field}/{populated}/{axis}/{value}");
 assert!(std::panic::catch_unwind(||layout_wardley(&d)).is_err(),"compat {box_field}/{populated}/{axis}/{value}");
 eprintln!("WARDLEY-ANNOTATION-REFUSAL {box_field}/{populated}/{axis}/{value}");
 }}}}
}
