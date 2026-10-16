//! Complete pipeline IR and physical-source errors before import/reader repair.
use diagram_ir::{WardleyConfig,WardleyDiagram,WardleyNode,WardleyPipeline,WardleyTheme};
use super::{parse_wardley,pipeline_work_reset,pipeline_work_read};
fn expected()->WardleyDiagram {WardleyDiagram {title:None,config:WardleyConfig {
 width:900.0,height:600.0,padding:48.0,node_radius:6.0,node_label_offset:8.0,axis_font_size:12.0,label_font_size:10.0,show_grid:false,use_max_width:true,
 theme:WardleyTheme {background_color:"#ffffff".into(),axis_color:"#000000".into(),axis_text_color:"#222222".into(),grid_color:"rgba(100, 100, 100, 0.2)".into(),component_fill:"#ffffff".into(),component_stroke:"#000000".into(),component_label_color:"#222222".into(),link_stroke:"#000000".into(),evolution_stroke:"#dc3545".into()}},
 stages:vec!["Genesis".into(),"Custom Built".into(),"Product".into(),"Commodity".into()],nodes:vec![],links:vec![],evolves:vec![],notes:vec![],annotations_box:None,annotations:vec![],forces:vec![],pipelines:vec![],stage_boundaries:vec![]}}
fn node(id:&str,label:&str,v:f64,e:f64,parent:bool)->WardleyNode {WardleyNode{id:id.into(),label:label.into(),visibility:v,evolution:e,anchor:false,pipeline_parent:parent}}
#[test]
fn parent_last_children_use_one_visibility_read_and_bounded_actual_key_comparisons() {
 for count in [8usize,32,128] {
 let mut source=String::from("wardley-beta
");let mut want=expected();
 for i in 0..count-1 {source.push_str(&format!("component R{i} [0,0.5]
"));want.nodes.push(node(&format!("wardley-{}",i+1),&format!("R{i}"),0.0,0.5,false));}
 source.push_str("component P [0.75,0.25]
pipeline P {
");want.nodes.push(node(&format!("wardley-{count}"),"P",0.75,0.25,true));
 let mut ids=vec![];for i in 0..count {let e=if i%2==0 {0.0}else{1.0};source.push_str(&format!("component C{i} [{e}]
"));let id=format!("wardley-{}",count+i+1);ids.push(id.clone());want.nodes.push(node(&id,&format!("C{i}"),0.75,e,false));}source.push_str("}
");want.pipelines.push(WardleyPipeline{parent_id:format!("wardley-{count}"),component_ids:ids});
 pipeline_work_reset();assert_eq!(parse_wardley(&source).unwrap(),want);let (comparisons,visibility_visits)=pipeline_work_read();let bound=8*(2*count+1)*((2*count+1).ilog2() as usize+2);
 assert!(comparisons>0&&comparisons<=bound,"count{count}: comparisons{comparisons} bound{bound}");assert!(visibility_visits<=1,"count{count}: actual parent visibility visits{visibility_visits}");eprintln!("WARDLEY-PIPELINE-WORK parent-last count={count} comparisons={comparisons} visibility_visits={visibility_visits} bound={bound}");
 }
}
#[test]
fn interleaved_distinct_blocks_retain_complete_ir_and_bounded_actual_visits() {
 for count in [8usize,32,128] {
 let mut source=String::from("wardley-beta
");let mut want=expected();
 for i in 0..count {source.push_str(&format!("component R{i} [0,0.5]
"));want.nodes.push(node(&format!("wardley-{}",i+1),&format!("R{i}"),0.0,0.5,false));}
 for i in 0..count {source.push_str(&format!("component P{i} [0.75,0.25]
"));want.nodes.push(node(&format!("wardley-{}",count+i+1),&format!("P{i}"),0.75,0.25,true));}
 for i in 0..count {source.push_str(&format!("pipeline P{i} {{
component C{i} [0.5]
}}
"));let id=format!("wardley-{}",2*count+i+1);want.nodes.push(node(&id,&format!("C{i}"),0.75,0.5,false));want.pipelines.push(WardleyPipeline{parent_id:format!("wardley-{}",count+i+1),component_ids:vec![id]});}
 pipeline_work_reset();assert_eq!(parse_wardley(&source).unwrap(),want);let (comparisons,visibility_visits)=pipeline_work_read();let bound=8*(3*count+1)*((3*count+1).ilog2() as usize+2);
 assert!(comparisons>0&&comparisons<=bound);assert!(visibility_visits<=count,"count{count}: visibility visits{visibility_visits}");eprintln!("WARDLEY-PIPELINE-WORK interleaved count={count} comparisons={comparisons} visibility_visits={visibility_visits} bound={bound}");
 }
}
