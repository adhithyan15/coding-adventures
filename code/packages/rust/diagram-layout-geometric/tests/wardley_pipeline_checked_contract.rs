use diagram_ir::*;
use diagram_layout_geometric::{layout_wardley,try_layout_wardley};
fn config()->WardleyConfig {WardleyConfig {width:900.0,height:600.0,padding:48.0,node_radius:6.0,node_label_offset:8.0,axis_font_size:12.0,label_font_size:10.0,show_grid:false,use_max_width:true,theme:WardleyTheme {background_color:"#ffffff".into(),axis_color:"#000000".into(),axis_text_color:"#222222".into(),grid_color:"rgba(100, 100, 100, 0.2)".into(),component_fill:"#ffffff".into(),component_stroke:"#000000".into(),component_label_color:"#222222".into(),link_stroke:"#000000".into(),evolution_stroke:"#dc3545".into()}}}
fn diagram()->WardleyDiagram {WardleyDiagram{title:None,config:config(),stages:vec![],nodes:vec![],links:vec![],evolves:vec![],notes:vec![],annotations_box:None,annotations:vec![],forces:vec![],pipelines:vec![],stage_boundaries:vec![]}}
fn n(id:&str,v:f64,e:f64,parent:bool)->WardleyNode {WardleyNode{id:id.into(),label:id.into(),visibility:v,evolution:e,anchor:false,pipeline_parent:parent}}
fn pipe(parent:&str,ids:&[&str])->WardleyPipeline {WardleyPipeline{parent_id:parent.into(),component_ids:ids.iter().map(|s|s.to_string()).collect()}}
fn seeded()->WardleyDiagram {let mut d=diagram();d.nodes=vec![n("P",0.5,0.9,true),n("C",0.5,0.25,false),n("Q",0.0,0.8,true),n("D",0.0,0.75,false)];d.pipelines=vec![pipe("P",&["C"]),pipe("Q",&["D"])];d}
fn invalid(case:usize)->WardleyDiagram {let mut d=seeded();match case {
 0=>d.pipelines[0].parent_id="missing".into(),1=>d.pipelines[0].component_ids=vec!["missing".into()],2=>d.nodes[0].pipeline_parent=false,3=>{d.pipelines.clear();},4=>d.nodes[1].pipeline_parent=true,5=>d.nodes.push(d.nodes[1].clone()),
 6=>d.pipelines[0].component_ids.clear(),7=>d.pipelines.push(pipe("P",&["C"])),8=>d.pipelines[0].component_ids=vec!["P".into()],9=>d.pipelines[0].component_ids=vec!["Q".into()],10=>d.pipelines[1].component_ids=vec!["C".into()],11=>d.pipelines[1].component_ids.clear(),_=>unreachable!()};d}
#[test]
fn invalid_pipeline_graphs_refuse_before_legacy_mapping() {
 for case in 0..12 {let d=invalid(case);assert!(try_layout_wardley(&d).is_err(),"case{case}");assert!(std::panic::catch_unwind(||layout_wardley(&d)).is_err());eprintln!("WARDLEY-PIPELINE-LEGACY {case}");}
}
fn p(x:f64,y:f64)->Point {Point{x,y}}
fn ln(id:&str,x:f64,y:f64,parent:bool)->LayoutedWardleyNode {LayoutedWardleyNode{id:id.into(),label:id.into(),position:p(x,y),anchor:false,pipeline_parent:parent}}
fn lp(x:f64,y:f64,w:f64,positions:&[(f64,f64)])->LayoutedWardleyPipeline {LayoutedWardleyPipeline{x,y,width:w,height:24.0,component_positions:positions.iter().map(|(x,y)|p(*x,*y)).collect()}}
#[test]
fn flat_pipeline_sorted_occurrences_and_final_parent_references_have_whole_layout() {
 for case in 0..5 {
 let mut d=diagram();d.nodes=vec![n("P",0.5,0.9,true),n("C",0.5,0.25,false)];d.pipelines=vec![pipe("P",&["C"])];
 let (nodes,pipelines)=match case {
 0=>(vec![ln("P",249.0,286.4,true),ln("C",249.0,300.0,false)],vec![lp(234.0,288.0,30.0,&[(249.0,300.0)])]),
 1=>{d.nodes[1].evolution=0.75;d.nodes.push(n("D",0.5,0.25,false));d.pipelines[0].component_ids=vec!["C".into(),"D".into()];(vec![ln("P",450.0,286.4,true),ln("C",651.0,300.0,false),ln("D",249.0,300.0,false)],vec![lp(234.0,288.0,432.0,&[(249.0,300.0),(651.0,300.0)])])},
 2=>{d.nodes[1].visibility=0.75;d.nodes.push(n("D",0.25,0.25,false));d.pipelines[0].component_ids=vec!["D".into(),"C".into(),"D".into()];(vec![ln("P",249.0,412.4,true),ln("C",249.0,174.0,false),ln("D",249.0,426.0,false)],vec![lp(234.0,414.0,30.0,&[(249.0,426.0),(249.0,174.0),(249.0,426.0)])])},
 3=>{d.pipelines[0].component_ids.push("C".into());(vec![ln("P",249.0,286.4,true),ln("C",249.0,300.0,false)],vec![lp(234.0,288.0,30.0,&[(249.0,300.0),(249.0,300.0)])])},
 4=>{d=seeded();(vec![ln("P",249.0,286.4,true),ln("C",249.0,300.0,false),ln("Q",651.0,538.4,true),ln("D",651.0,552.0,false)],vec![lp(234.0,288.0,30.0,&[(249.0,300.0)]),lp(636.0,540.0,30.0,&[(651.0,552.0)])])},_=>unreachable!()};
 let parent=nodes[0].position.clone();d.links=vec![WardleyLink{source:"P".into(),target:"C".into()}];d.evolves=vec![WardleyEvolution{component:"P".into(),target:1.0}];
 let want=LayoutedWardleyDiagram{width:900.0,height:600.0,title:None,config:config(),stages:vec![],links:vec![LayoutedWardleyLink{from:parent.clone(),to:nodes[1].position.clone()}],evolves:vec![LayoutedWardleyEvolution{from:parent,to:p(852.0,300.0)}],nodes,notes:vec![],annotations_box:None,annotations:vec![],forces:vec![],pipelines,stage_boundaries:vec![]};
 assert_eq!(try_layout_wardley(&d),Ok(want.clone()),"case{case}");assert_eq!(layout_wardley(&d),want);eprintln!("WARDLEY-PIPELINE-WHOLE-LAYOUT {case}");
 }
}
