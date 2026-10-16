use diagram_ir::*;
use diagram_layout_geometric::{try_layout_wardley,WardleyLayoutError};
fn config()->WardleyConfig {WardleyConfig {width:900.0,height:600.0,padding:48.0,node_radius:6.0,node_label_offset:8.0,axis_font_size:12.0,label_font_size:10.0,show_grid:false,use_max_width:true,theme:WardleyTheme {background_color:"#ffffff".into(),axis_color:"#000000".into(),axis_text_color:"#222222".into(),grid_color:"rgba(100, 100, 100, 0.2)".into(),component_fill:"#ffffff".into(),component_stroke:"#000000".into(),component_label_color:"#222222".into(),link_stroke:"#000000".into(),evolution_stroke:"#dc3545".into()}}}
fn diagram()->WardleyDiagram {WardleyDiagram{title:None,config:config(),stages:vec![],nodes:vec![],links:vec![],evolves:vec![],notes:vec![],annotations_box:None,annotations:vec![],forces:vec![],pipelines:vec![],stage_boundaries:vec![]}}
fn n(id:&str,v:f64,e:f64,parent:bool)->WardleyNode {WardleyNode{id:id.into(),label:id.into(),visibility:v,evolution:e,anchor:false,pipeline_parent:parent}}
fn pipe(parent:&str,ids:&[&str])->WardleyPipeline {WardleyPipeline{parent_id:parent.into(),component_ids:ids.iter().map(|s|s.to_string()).collect()}}
fn seeded()->WardleyDiagram {let mut d=diagram();d.nodes=vec![n("P",0.5,0.9,true),n("C",0.5,0.25,false),n("Q",0.0,0.8,true),n("D",0.0,0.75,false)];d.pipelines=vec![pipe("P",&["C"]),pipe("Q",&["D"])];d}
fn invalid(case:usize)->WardleyDiagram {let mut d=seeded();match case {
 0=>d.pipelines[0].parent_id="missing".into(),1=>d.pipelines[0].component_ids=vec!["missing".into()],2=>d.nodes[0].pipeline_parent=false,3=>{d.pipelines.clear();},4=>d.nodes[1].pipeline_parent=true,5=>d.nodes.push(d.nodes[1].clone()),
 6=>d.pipelines[0].component_ids.clear(),7=>d.pipelines.push(pipe("P",&["C"])),8=>d.pipelines[0].component_ids=vec!["P".into()],9=>d.pipelines[0].component_ids=vec!["Q".into()],10=>d.pipelines[1].component_ids=vec!["C".into()],11=>d.pipelines[1].component_ids.clear(),_=>unreachable!()};d}
#[test]
fn pipeline_graph_errors_have_exact_declared_names() {
 for case in 0..12 {let want=if case<6 {WardleyLayoutError::PipelineReference}else{WardleyLayoutError::PipelineMembership};assert_eq!(try_layout_wardley(&invalid(case)),Err(want),"case{case}");eprintln!("WARDLEY-PIPELINE-NAMED {case}");}
}
#[test]
fn every_prior_error_keeps_priority_before_pipeline_reference_and_membership() {
 for (case,want) in [(0,WardleyLayoutError::Width),(1,WardleyLayoutError::NodeCoordinates),(2,WardleyLayoutError::EvolutionTarget),(3,WardleyLayoutError::NoteCoordinates),(4,WardleyLayoutError::AnnotationBoxCoordinates),(5,WardleyLayoutError::AnnotationCoordinates),(6,WardleyLayoutError::ForceCoordinates),(7,WardleyLayoutError::UnknownLinkEndpoint),(8,WardleyLayoutError::UnknownEvolutionComponent),(9,WardleyLayoutError::PipelineReference),(10,WardleyLayoutError::PipelineMembership)] {
 let mut d=invalid(0);match case {0=>d.config.width=f64::NAN,1=>d.nodes[0].visibility=f64::NAN,2=>d.evolves.push(WardleyEvolution{component:"missing".into(),target:f64::NAN}),3=>d.notes.push(WardleyNote{text:"N".into(),visibility:f64::NAN,evolution:0.5}),4=>d.annotations_box=Some((f64::NAN,0.5)),5=>d.annotations.push(WardleyAnnotation{number:1,text:"A".into(),coordinates:vec![(0.5,f64::NAN)]}),6=>d.forces.push(WardleyForce{label:"F".into(),visibility:f64::NAN,evolution:0.5,kind:WardleyForceKind::Accelerator}),7=>d.links.push(WardleyLink{source:"missing".into(),target:"missing".into()}),8=>d.evolves.push(WardleyEvolution{component:"missing".into(),target:0.5}),10=>d=invalid(6),_=>{}}
 assert_eq!(try_layout_wardley(&d),Err(want),"priority{case}");eprintln!("WARDLEY-PIPELINE-PRIORITY {case}");
 }
}
