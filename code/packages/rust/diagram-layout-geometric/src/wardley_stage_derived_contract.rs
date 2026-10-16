use super::*;
fn empty()->LayoutedWardleyDiagram {
 LayoutedWardleyDiagram {width:900.0,height:600.0,title:None,config:diagram_ir::WardleyConfig{width:900.0,height:600.0,padding:48.0,node_radius:6.0,node_label_offset:8.0,axis_font_size:12.0,label_font_size:10.0,show_grid:false,use_max_width:true,theme:diagram_ir::WardleyTheme{background_color:"#ffffff".into(),axis_color:"#000000".into(),axis_text_color:"#222222".into(),grid_color:"rgba(100, 100, 100, 0.2)".into(),component_fill:"#ffffff".into(),component_stroke:"#000000".into(),component_label_color:"#222222".into(),link_stroke:"#000000".into(),evolution_stroke:"#dc3545".into()}},stages:vec![],stage_boundaries:vec![],nodes:vec![],links:vec![],evolves:vec![],notes:vec![],annotations_box:None,annotations:vec![],forces:vec![],pipelines:vec![]}
}

#[test]
fn forged_stage_intervals_fail_the_complete_derived_gate() {
 for ends in [vec![0.5],vec![0.1,0.5,1.0],vec![f64::NAN,1.0],vec![0.1,f64::INFINITY],vec![f64::NEG_INFINITY,1.0],vec![-0.1,1.0],vec![0.2,1.1],vec![0.8,0.2]] {
 let mut l=empty();l.stages=vec!["A".into(),"B".into()];l.stage_boundaries=ends;assert_eq!(validate_wardley_geometry(&l),Err(WardleyLayoutError::DerivedGeometry));eprintln!("WARDLEY-STAGE-DERIVED-REFUSAL {:?}",l.stage_boundaries);
 }
 let mut l=empty();l.stage_boundaries=vec![0.5];assert_eq!(validate_wardley_geometry(&l),Err(WardleyLayoutError::DerivedGeometry));eprintln!("WARDLEY-STAGE-DERIVED-REFUSAL empty-stages");
}
#[test]
fn derived_uniform_zero_equal_and_partial_final_bands_remain_supported() {
 for (names,ends) in [(vec![],vec![]),(vec!["A"],vec![]),(vec!["A"],vec![0.0]),(vec!["A","B"],vec![0.3,0.3]),(vec!["A","B"],vec![0.0,0.8])] {
 let mut l=empty();l.stages=names.into_iter().map(String::from).collect();l.stage_boundaries=ends;assert_eq!(validate_wardley_geometry(&l),Ok(()));eprintln!("WARDLEY-STAGE-DERIVED-ACCEPT {}",l.stages.len());
 }
}
