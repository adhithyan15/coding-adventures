//! Closed stage IR and literal physical errors declared before stage import.
use diagram_ir::{WardleyConfig,WardleyDiagram,WardleyTheme};
use mermaid_parser::parse_wardley;
fn expected()->WardleyDiagram {WardleyDiagram {title:None,config:WardleyConfig {
 width:900.0,height:600.0,padding:48.0,node_radius:6.0,node_label_offset:8.0,axis_font_size:12.0,label_font_size:10.0,show_grid:false,use_max_width:true,
 theme:WardleyTheme {background_color:"#ffffff".into(),axis_color:"#000000".into(),axis_text_color:"#222222".into(),grid_color:"rgba(100, 100, 100, 0.2)".into(),component_fill:"#ffffff".into(),component_stroke:"#000000".into(),component_label_color:"#222222".into(),link_stroke:"#000000".into(),evolution_stroke:"#dc3545".into()}},
 stage_boundaries:vec![],stages:vec!["Genesis".into(),"Custom Built".into(),"Product".into(),"Commodity".into()],nodes:vec![],links:vec![],evolves:vec![],notes:vec![],annotations_box:None,annotations:vec![],forces:vec![],pipelines:vec![]}}

#[test]
fn stage_labels_aliases_and_boundaries_preserve_complete_ir() {
 for separator in [" ","\t"," \t  "] {
  for (body,labels,boundaries) in [
   ("A -> B",vec!["A","B"],vec![]),
   ("Alpha Beta -> Gamma Delta",vec!["Alpha Beta","Gamma Delta"],vec![]),
   ("A@0 -> B@1",vec!["A","B"],vec![0.0,1.0]),
   ("A@0.2 -> B@0.7",vec!["A","B"],vec![0.2,0.7]),
   ("A@0.2 -> B@0.2 -> C@1",vec!["A","B","C"],vec![0.2,0.2,1.0]),
   ("A / Aux -> B / Second",vec!["A / Aux","B / Second"],vec![]),
   ("A@0.25 / Aux -> B@1 / Second",vec!["A / Aux","B / Second"],vec![0.25,1.0]),
   ("\"A@/->%%[{}]\"@0.25 / 'α %% / -> @' -> 'B'@1",vec!["A@/->%%[{}] / α %% / -> @","B"],vec![0.25,1.0]),
   ("\"A\\\"B\" -> 'C\\\'D'",vec!["A\"B","C'D"],vec![]),
   ("\"A\\\\B\" -> ' C '",vec!["A\\B"," C "],vec![]),
   ("'A\"B' / \" C' \" -> Z",vec!["A\"B /  C' ","Z"],vec![]),
   ("A@+0e0 -> B@1e0",vec!["A","B"],vec![0.0,1.0]),
  ] {
   let text=format!("wardley-beta\nevolution{separator}{body} %% outside unmatched ' \"\n");
   let mut want=expected();want.stages=labels.into_iter().map(String::from).collect();want.stage_boundaries=boundaries;
   assert_eq!(parse_wardley(&text).unwrap(),want,"{text}");eprintln!("WARDLEY-STAGE-SOURCE-IR separator={separator:?} {body}");
  }
 }
}
#[test]
fn later_stage_statements_replace_labels_and_boundaries_together() {
 for (body,labels,boundaries) in [
  ("evolution A@0.5 -> B@1\nevolution C -> D",vec!["C","D"],vec![]),
  ("evolution A -> B\nevolution C@0 -> D@0.8",vec!["C","D"],vec![0.0,0.8]),
 ] {let mut want=expected();want.stages=labels.into_iter().map(String::from).collect();want.stage_boundaries=boundaries;assert_eq!(parse_wardley(&format!("wardley-beta\n{body}\n")).unwrap(),want);eprintln!("WARDLEY-STAGE-REPLACE {body:?}");}
}
#[test]
fn stage_source_errors_bind_exact_physical_lines_and_messages() {
 for (body,message) in [
  ("A","evolution requires at least two stages"),
  ("-> B","Wardley stage label must be non-empty"),
  ("A ->","Wardley stage label must be non-empty"),
  ("'' -> B","Wardley stage label must be non-empty"),
  ("\"\"@0 -> B@1","Wardley stage label must be non-empty"),
  ("A / -> B","Wardley secondary stage label must be non-empty"),
  ("A / '' -> B","Wardley secondary stage label must be non-empty"),
  ("A / \"\" -> B","Wardley secondary stage label must be non-empty"),
  ("\"A -> B","Wardley stage label must be a complete quoted string"),
  ("'A -> B","Wardley stage label must be a complete quoted string"),
  ("\"A\\n\" -> B","Wardley stage label must be a complete quoted string"),
  ("A / \"B -> C","Wardley secondary stage label must be a complete quoted string"),
  ("A / 'B\\n' -> C","Wardley secondary stage label must be a complete quoted string"),
  ("\"A\"junk -> B","invalid Wardley stage label suffix"),
  ("\"A\" junk -> B","invalid Wardley stage label suffix"),
  ("A / \"B\" junk -> C","invalid Wardley secondary stage label suffix"),
  ("A / B / C -> D","invalid Wardley stage suffix"),
  ("A / B@0.5 -> C","invalid Wardley stage suffix"),
  ("A@ -> B@1","invalid Wardley stage boundary"),
  ("A@bad -> B@1","invalid Wardley stage boundary"),
  ("A@0.2 junk -> B@1","invalid Wardley stage boundary"),
  ("A@0.2@0.4 -> B@1","invalid Wardley stage boundary"),
  ("A@NaN -> B@1","Wardley stage boundaries must be between 0 and 1"),
  ("A@inf -> B@1","Wardley stage boundaries must be between 0 and 1"),
  ("A@-inf -> B@1","Wardley stage boundaries must be between 0 and 1"),
  ("A@-0.1 -> B@1","Wardley stage boundaries must be between 0 and 1"),
  ("A@0.1 -> B@1.1","Wardley stage boundaries must be between 0 and 1"),
  ("A@0.8 -> B@0.2","Wardley stage boundaries must be nondecreasing"),
  ("A@0.2 -> B -> C@1","Wardley stage boundaries must specify every stage"),
  ("A -> B@1","Wardley stage boundaries must specify every stage"),
  ("A@0 -> B","Wardley stage boundaries must specify every stage"),
 ] {let text=format!("wardley-beta\n\n%% kept physical line\nevolution {body}\n");let e=parse_wardley(&text).unwrap_err();assert_eq!((e.message.as_str(),e.line,e.col),(message,4,1),"{text}");eprintln!("WARDLEY-STAGE-SOURCE-ERROR {message}");}
 let e=parse_wardley("wardley-beta\ncomponent P [0.5,0.5]\npipeline P {\nevolution A -> B\n}\n").unwrap_err();assert_eq!((e.message.as_str(),e.line,e.col),("Wardley pipeline bodies only accept components",4,1));eprintln!("WARDLEY-STAGE-SOURCE-ERROR body-ownership");
}
