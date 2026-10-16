use diagram_ir::*;
use diagram_layout_geometric::{layout_wardley,try_layout_wardley};
fn config()->WardleyConfig {WardleyConfig {width:900.0,height:600.0,padding:48.0,node_radius:6.0,node_label_offset:8.0,axis_font_size:12.0,label_font_size:10.0,show_grid:false,use_max_width:true,theme:WardleyTheme {annotation_stroke:"#000000".into(),annotation_text_color:"#222222".into(),annotation_fill:"#ffffff".into(),background_color:"#ffffff".into(),axis_color:"#000000".into(),axis_text_color:"#222222".into(),grid_color:"rgba(100, 100, 100, 0.2)".into(),component_fill:"#ffffff".into(),component_stroke:"#000000".into(),component_label_color:"#222222".into(),link_stroke:"#000000".into(),evolution_stroke:"#dc3545".into()}}}
fn diagram()->WardleyDiagram {WardleyDiagram{accessibility_title:None,accessibility_description:None,title:None,config:config(),stages:vec![],stage_boundaries:vec![],nodes:vec![],links:vec![],evolves:vec![],notes:vec![],annotations_box:None,annotations:vec![],forces:vec![],pipelines:vec![]}}

fn invalid(case:usize)->WardleyDiagram {let mut d=diagram();d.stages=vec!["A".into(),"B".into()];d.stage_boundaries=match case {
 0=>vec![0.5],1=>vec![0.2,0.5,1.0],2=>vec![f64::NAN,1.0],3=>vec![0.2,f64::INFINITY],4=>vec![f64::NEG_INFINITY,1.0],5=>vec![-0.1,1.0],6=>vec![0.2,1.1],7=>vec![0.8,0.2],8=>{d.stages.clear();vec![0.5]},_=>unreachable!()};d}
#[test]
fn unsupported_typed_stage_vectors_and_compatibility_refuse_before_mapping() {
 for case in 0..9 {let d=invalid(case);assert!(try_layout_wardley(&d).is_err(),"case{case}");assert!(std::panic::catch_unwind(||layout_wardley(&d)).is_err(),"compat{case}");eprintln!("WARDLEY-STAGE-LEGACY-REFUSAL {case}");}
}
#[test]
fn admitted_uniform_zero_equal_partial_final_and_small_stage_vectors_have_whole_layout() {
 for (names,ends) in [(vec![],vec![]),(vec!["A"],vec![]),(vec!["A"],vec![0.0]),(vec!["A"],vec![0.7]),(vec!["A","B"],vec![]),(vec!["A","B"],vec![0.0,1.0]),(vec!["A","B"],vec![0.3,0.3]),(vec!["A","B","C"],vec![0.0,0.5,0.8])] {
 let mut d=diagram();d.stages=names.iter().map(|v|v.to_string()).collect();d.stage_boundaries=ends.clone();
 let want=LayoutedWardleyDiagram{width:900.0,height:600.0,title:None,config:config(),stages:names.iter().map(|v|v.to_string()).collect(),stage_boundaries:ends,nodes:vec![],links:vec![],evolves:vec![],notes:vec![],annotations_box:None,annotations:vec![],forces:vec![],pipelines:vec![]};
 assert_eq!(try_layout_wardley(&d),Ok(want.clone()));assert_eq!(layout_wardley(&d),want);eprintln!("WARDLEY-STAGE-WHOLE-LAYOUT {}",names.len());
 }
}
