use super::*;
fn empty()->LayoutedWardleyDiagram {
 let layout=LayoutedWardleyDiagram {width:900.0,height:600.0,title:None,config:diagram_ir::WardleyConfig{width:900.0,height:600.0,padding:48.0,node_radius:6.0,node_label_offset:8.0,axis_font_size:12.0,label_font_size:10.0,show_grid:false,use_max_width:true,theme:diagram_ir::WardleyTheme{background_color:"#ffffff".into(),axis_color:"#000000".into(),axis_text_color:"#222222".into(),grid_color:"rgba(100, 100, 100, 0.2)".into(),component_fill:"#ffffff".into(),component_stroke:"#000000".into(),component_label_color:"#222222".into(),link_stroke:"#000000".into(),evolution_stroke:"#dc3545".into()}},stages:vec![],nodes:vec![],links:vec![],evolves:vec![],notes:vec![],annotations_box:None,annotations:vec![],forces:vec![],pipelines:vec![]};
 layout
}
#[test]
fn derived_pipeline_rectangles_and_occurrences_have_exact_finite_gate() {
 for field in 0..6 {for value in [f64::NAN,f64::INFINITY,f64::NEG_INFINITY] {
 let mut l=empty();let mut pipeline=diagram_ir::LayoutedWardleyPipeline{x:-15.0,y:-12.0,width:30.0,height:24.0,component_positions:vec![Point{x:0.0,y:0.0}]};
 match field {0=>pipeline.x=value,1=>pipeline.y=value,2=>pipeline.width=value,3=>pipeline.height=value,4=>pipeline.component_positions[0].x=value,5=>pipeline.component_positions[0].y=value,_=>unreachable!()};l.pipelines.push(pipeline);
 assert_eq!(validate_wardley_geometry(&l),Err(WardleyLayoutError::DerivedGeometry));eprintln!("WARDLEY-PIPELINE-DERIVED {field}/{value}");
 }}
 for field in [0,1] {let mut l=empty();l.pipelines.push(diagram_ir::LayoutedWardleyPipeline{x:-15.0,y:-12.0,width:if field==0 {-0.1}else{30.0},height:if field==1 {-0.1}else{24.0},component_positions:vec![]});assert_eq!(validate_wardley_geometry(&l),Err(WardleyLayoutError::DerivedGeometry));eprintln!("WARDLEY-PIPELINE-DERIVED negative{field}");}
 let mut l=empty();l.pipelines.push(diagram_ir::LayoutedWardleyPipeline{x:-15.0,y:-12.0,width:0.0,height:0.0,component_positions:vec![Point{x:0.0,y:0.0}]});assert_eq!(validate_wardley_geometry(&l),Ok(()));eprintln!("WARDLEY-PIPELINE-DERIVED negativeposition/zerodimensions/accepted");
}
#[test]
fn derived_parent_square_checks_its_new_scaled_radius_without_global_containment_claim() {
 let mut l=empty();l.config.node_radius=1.2e308;l.nodes.push(diagram_ir::LayoutedWardleyNode{id:"P".into(),label:"P".into(),position:Point{x:0.0,y:0.0},anchor:false,pipeline_parent:false});assert_eq!(validate_wardley_geometry(&l),Ok(()));l.nodes[0].pipeline_parent=true;assert_eq!(validate_wardley_geometry(&l),Err(WardleyLayoutError::DerivedGeometry));eprintln!("WARDLEY-PIPELINE-DERIVED parent-square-only");
}
