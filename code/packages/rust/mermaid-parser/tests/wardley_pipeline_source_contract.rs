//! Complete pipeline IR and physical-source errors before import/reader repair.
use diagram_ir::{WardleyConfig,WardleyDiagram,WardleyNode,WardleyPipeline,WardleyTheme};
use mermaid_parser::parse_wardley;
fn expected()->WardleyDiagram {WardleyDiagram {accessibility_title:None,accessibility_description:None,title:None,config:WardleyConfig {
 width:900.0,height:600.0,padding:48.0,node_radius:6.0,node_label_offset:8.0,axis_font_size:12.0,label_font_size:10.0,show_grid:false,use_max_width:true,
 theme:WardleyTheme {annotation_stroke:"#000000".into(),annotation_text_color:"#222222".into(),annotation_fill:"#ffffff".into(),background_color:"#ffffff".into(),axis_color:"#000000".into(),axis_text_color:"#222222".into(),grid_color:"rgba(100, 100, 100, 0.2)".into(),component_fill:"#ffffff".into(),component_stroke:"#000000".into(),component_label_color:"#222222".into(),link_stroke:"#000000".into(),evolution_stroke:"#dc3545".into()}},
 stages:vec!["Genesis".into(),"Custom Built".into(),"Product".into(),"Commodity".into()],nodes:vec![],links:vec![],evolves:vec![],notes:vec![],annotations_box:None,annotations:vec![],forces:vec![],pipelines:vec![],stage_boundaries:vec![]}}
fn node(id:&str,label:&str,v:f64,e:f64,parent:bool)->WardleyNode {WardleyNode{label_offset_x:None,label_offset_y:None,source_strategy:None,inertia:false,id:id.into(),label:label.into(),visibility:v,evolution:e,anchor:false,pipeline_parent:parent}}
fn one(label:&str)->WardleyDiagram {let mut d=expected();d.nodes=vec![node("wardley-1","P",0.5,0.1,true),node("wardley-2",label,0.5,0.25,false)];d.pipelines=vec![WardleyPipeline{parent_id:"wardley-1".into(),component_ids:vec!["wardley-2".into()]}];d}
#[test]
fn pipeline_labels_bind_complete_ir_without_losing_quote_owned_delimiters() {
assert_eq!(parse_wardley("wardley-beta\ncomponent P [0.5,0.1]\npipeline P {\ncomponent C [0.25] %% outside \"junk\n} %% done\n").unwrap_or_else(|e|panic!("bare {e:?}")),one("C"));eprintln!("WARDLEY-PIPELINE-IR bare/space");
assert_eq!(parse_wardley("wardley-beta\ncomponent P [0.5,0.1]\npipeline\tP {\ncomponent\tC [0.25] %% outside \"junk\n} %% done\n").unwrap_or_else(|e|panic!("bare {e:?}")),one("C"));eprintln!("WARDLEY-PIPELINE-IR bare/tab");
assert_eq!(parse_wardley("wardley-beta\ncomponent P [0.5,0.1]\npipeline P {\ncomponent Alpha Beta [0.25] %% outside \"junk\n} %% done\n").unwrap_or_else(|e|panic!("multi {e:?}")),one("Alpha Beta"));eprintln!("WARDLEY-PIPELINE-IR multi/space");
assert_eq!(parse_wardley("wardley-beta\ncomponent P [0.5,0.1]\npipeline\tP {\ncomponent\tAlpha Beta [0.25] %% outside \"junk\n} %% done\n").unwrap_or_else(|e|panic!("multi {e:?}")),one("Alpha Beta"));eprintln!("WARDLEY-PIPELINE-IR multi/tab");
assert_eq!(parse_wardley("wardley-beta\ncomponent P [0.5,0.1]\npipeline P {\ncomponent \"C\" [0.25] %% outside \"junk\n} %% done\n").unwrap_or_else(|e|panic!("double {e:?}")),one("C"));eprintln!("WARDLEY-PIPELINE-IR double/space");
assert_eq!(parse_wardley("wardley-beta\ncomponent P [0.5,0.1]\npipeline\tP {\ncomponent\t\"C\" [0.25] %% outside \"junk\n} %% done\n").unwrap_or_else(|e|panic!("double {e:?}")),one("C"));eprintln!("WARDLEY-PIPELINE-IR double/tab");
assert_eq!(parse_wardley("wardley-beta\ncomponent P [0.5,0.1]\npipeline P {\ncomponent 'C' [0.25] %% outside \"junk\n} %% done\n").unwrap_or_else(|e|panic!("single {e:?}")),one("C"));eprintln!("WARDLEY-PIPELINE-IR single/space");
assert_eq!(parse_wardley("wardley-beta\ncomponent P [0.5,0.1]\npipeline\tP {\ncomponent\t'C' [0.25] %% outside \"junk\n} %% done\n").unwrap_or_else(|e|panic!("single {e:?}")),one("C"));eprintln!("WARDLEY-PIPELINE-IR single/tab");
assert_eq!(parse_wardley("wardley-beta\ncomponent P [0.5,0.1]\npipeline P {\ncomponent \"  C  \" [0.25] %% outside \"junk\n} %% done\n").unwrap_or_else(|e|panic!("spaces {e:?}")),one("  C  "));eprintln!("WARDLEY-PIPELINE-IR spaces/space");
assert_eq!(parse_wardley("wardley-beta\ncomponent P [0.5,0.1]\npipeline\tP {\ncomponent\t\"  C  \" [0.25] %% outside \"junk\n} %% done\n").unwrap_or_else(|e|panic!("spaces {e:?}")),one("  C  "));eprintln!("WARDLEY-PIPELINE-IR spaces/tab");
assert_eq!(parse_wardley("wardley-beta\ncomponent P [0.5,0.1]\npipeline P {\ncomponent ' ' [0.25] %% outside \"junk\n} %% done\n").unwrap_or_else(|e|panic!("one-space {e:?}")),one(" "));eprintln!("WARDLEY-PIPELINE-IR one-space/space");
assert_eq!(parse_wardley("wardley-beta\ncomponent P [0.5,0.1]\npipeline\tP {\ncomponent\t' ' [0.25] %% outside \"junk\n} %% done\n").unwrap_or_else(|e|panic!("one-space {e:?}")),one(" "));eprintln!("WARDLEY-PIPELINE-IR one-space/tab");
assert_eq!(parse_wardley("wardley-beta\ncomponent P [0.5,0.1]\npipeline P {\ncomponent \"α निर्णय\" [0.25] %% outside \"junk\n} %% done\n").unwrap_or_else(|e|panic!("unicode {e:?}")),one("α निर्णय"));eprintln!("WARDLEY-PIPELINE-IR unicode/space");
assert_eq!(parse_wardley("wardley-beta\ncomponent P [0.5,0.1]\npipeline\tP {\ncomponent\t\"α निर्णय\" [0.25] %% outside \"junk\n} %% done\n").unwrap_or_else(|e|panic!("unicode {e:?}")),one("α निर्णय"));eprintln!("WARDLEY-PIPELINE-IR unicode/tab");
assert_eq!(parse_wardley("wardley-beta\ncomponent P [0.5,0.1]\npipeline P {\ncomponent \"A%%B\" [0.25] %% outside \"junk\n} %% done\n").unwrap_or_else(|e|panic!("percent {e:?}")),one("A%%B"));eprintln!("WARDLEY-PIPELINE-IR percent/space");
assert_eq!(parse_wardley("wardley-beta\ncomponent P [0.5,0.1]\npipeline\tP {\ncomponent\t\"A%%B\" [0.25] %% outside \"junk\n} %% done\n").unwrap_or_else(|e|panic!("percent {e:?}")),one("A%%B"));eprintln!("WARDLEY-PIPELINE-IR percent/tab");
assert_eq!(parse_wardley("wardley-beta\ncomponent P [0.5,0.1]\npipeline P {\ncomponent 'A%%B' [0.25] %% outside \"junk\n} %% done\n").unwrap_or_else(|e|panic!("single-percent {e:?}")),one("A%%B"));eprintln!("WARDLEY-PIPELINE-IR single-percent/space");
assert_eq!(parse_wardley("wardley-beta\ncomponent P [0.5,0.1]\npipeline\tP {\ncomponent\t'A%%B' [0.25] %% outside \"junk\n} %% done\n").unwrap_or_else(|e|panic!("single-percent {e:?}")),one("A%%B"));eprintln!("WARDLEY-PIPELINE-IR single-percent/tab");
assert_eq!(parse_wardley("wardley-beta\ncomponent P [0.5,0.1]\npipeline P {\ncomponent \"A[ B ]\" [0.25] %% outside \"junk\n} %% done\n").unwrap_or_else(|e|panic!("brackets {e:?}")),one("A[ B ]"));eprintln!("WARDLEY-PIPELINE-IR brackets/space");
assert_eq!(parse_wardley("wardley-beta\ncomponent P [0.5,0.1]\npipeline\tP {\ncomponent\t\"A[ B ]\" [0.25] %% outside \"junk\n} %% done\n").unwrap_or_else(|e|panic!("brackets {e:?}")),one("A[ B ]"));eprintln!("WARDLEY-PIPELINE-IR brackets/tab");
assert_eq!(parse_wardley("wardley-beta\ncomponent P [0.5,0.1]\npipeline P {\ncomponent \"A{ B }\" [0.25] %% outside \"junk\n} %% done\n").unwrap_or_else(|e|panic!("braces {e:?}")),one("A{ B }"));eprintln!("WARDLEY-PIPELINE-IR braces/space");
assert_eq!(parse_wardley("wardley-beta\ncomponent P [0.5,0.1]\npipeline\tP {\ncomponent\t\"A{ B }\" [0.25] %% outside \"junk\n} %% done\n").unwrap_or_else(|e|panic!("braces {e:?}")),one("A{ B }"));eprintln!("WARDLEY-PIPELINE-IR braces/tab");
assert_eq!(parse_wardley("wardley-beta\ncomponent P [0.5,0.1]\npipeline P {\ncomponent \"A'B\" [0.25] %% outside \"junk\n} %% done\n").unwrap_or_else(|e|panic!("inactive-single {e:?}")),one("A'B"));eprintln!("WARDLEY-PIPELINE-IR inactive-single/space");
assert_eq!(parse_wardley("wardley-beta\ncomponent P [0.5,0.1]\npipeline\tP {\ncomponent\t\"A'B\" [0.25] %% outside \"junk\n} %% done\n").unwrap_or_else(|e|panic!("inactive-single {e:?}")),one("A'B"));eprintln!("WARDLEY-PIPELINE-IR inactive-single/tab");
assert_eq!(parse_wardley("wardley-beta\ncomponent P [0.5,0.1]\npipeline P {\ncomponent 'A\"B' [0.25] %% outside \"junk\n} %% done\n").unwrap_or_else(|e|panic!("inactive-double {e:?}")),one("A\"B"));eprintln!("WARDLEY-PIPELINE-IR inactive-double/space");
assert_eq!(parse_wardley("wardley-beta\ncomponent P [0.5,0.1]\npipeline\tP {\ncomponent\t'A\"B' [0.25] %% outside \"junk\n} %% done\n").unwrap_or_else(|e|panic!("inactive-double {e:?}")),one("A\"B"));eprintln!("WARDLEY-PIPELINE-IR inactive-double/tab");
assert_eq!(parse_wardley("wardley-beta\ncomponent P [0.5,0.1]\npipeline P {\ncomponent \"A\\\"B\" [0.25] %% outside \"junk\n} %% done\n").unwrap_or_else(|e|panic!("escaped-double {e:?}")),one("A\"B"));eprintln!("WARDLEY-PIPELINE-IR escaped-double/space");
assert_eq!(parse_wardley("wardley-beta\ncomponent P [0.5,0.1]\npipeline\tP {\ncomponent\t\"A\\\"B\" [0.25] %% outside \"junk\n} %% done\n").unwrap_or_else(|e|panic!("escaped-double {e:?}")),one("A\"B"));eprintln!("WARDLEY-PIPELINE-IR escaped-double/tab");
assert_eq!(parse_wardley("wardley-beta\ncomponent P [0.5,0.1]\npipeline P {\ncomponent 'A\\'B' [0.25] %% outside \"junk\n} %% done\n").unwrap_or_else(|e|panic!("escaped-single {e:?}")),one("A'B"));eprintln!("WARDLEY-PIPELINE-IR escaped-single/space");
assert_eq!(parse_wardley("wardley-beta\ncomponent P [0.5,0.1]\npipeline\tP {\ncomponent\t'A\\'B' [0.25] %% outside \"junk\n} %% done\n").unwrap_or_else(|e|panic!("escaped-single {e:?}")),one("A'B"));eprintln!("WARDLEY-PIPELINE-IR escaped-single/tab");
assert_eq!(parse_wardley("wardley-beta\ncomponent P [0.5,0.1]\npipeline P {\ncomponent \"A\\\\B\" [0.25] %% outside \"junk\n} %% done\n").unwrap_or_else(|e|panic!("slash {e:?}")),one("A\\B"));eprintln!("WARDLEY-PIPELINE-IR slash/space");
assert_eq!(parse_wardley("wardley-beta\ncomponent P [0.5,0.1]\npipeline\tP {\ncomponent\t\"A\\\\B\" [0.25] %% outside \"junk\n} %% done\n").unwrap_or_else(|e|panic!("slash {e:?}")),one("A\\B"));eprintln!("WARDLEY-PIPELINE-IR slash/tab");
}
#[test]
fn pipeline_header_names_and_ascii_separators_keep_exact_first_authored_parent() {
 {let mut want=one("C");want.nodes[0].label="P".into();assert_eq!(parse_wardley("wardley-beta\ncomponent P [0.5,0.1]\npipeline P { %% outside\ncomponent C [0.25]\n}\n").unwrap(),want);eprintln!("WARDLEY-PIPELINE-HEADER bare/' '");}
 {let mut want=one("C");want.nodes[0].label="P".into();assert_eq!(parse_wardley("wardley-beta\ncomponent P [0.5,0.1]\npipeline\tP { %% outside\ncomponent C [0.25]\n}\n").unwrap(),want);eprintln!("WARDLEY-PIPELINE-HEADER bare/'\t'");}
 {let mut want=one("C");want.nodes[0].label="P".into();assert_eq!(parse_wardley("wardley-beta\ncomponent P [0.5,0.1]\npipeline \t  P { %% outside\ncomponent C [0.25]\n}\n").unwrap(),want);eprintln!("WARDLEY-PIPELINE-HEADER bare/' \t  '");}
 {let mut want=one("C");want.nodes[0].label="P".into();assert_eq!(parse_wardley("wardley-beta\ncomponent P [0.5,0.1]\npipeline \"P\" { %% outside\ncomponent C [0.25]\n}\n").unwrap(),want);eprintln!("WARDLEY-PIPELINE-HEADER double/' '");}
 {let mut want=one("C");want.nodes[0].label="P".into();assert_eq!(parse_wardley("wardley-beta\ncomponent P [0.5,0.1]\npipeline\t\"P\" { %% outside\ncomponent C [0.25]\n}\n").unwrap(),want);eprintln!("WARDLEY-PIPELINE-HEADER double/'\t'");}
 {let mut want=one("C");want.nodes[0].label="P".into();assert_eq!(parse_wardley("wardley-beta\ncomponent P [0.5,0.1]\npipeline \t  \"P\" { %% outside\ncomponent C [0.25]\n}\n").unwrap(),want);eprintln!("WARDLEY-PIPELINE-HEADER double/' \t  '");}
 {let mut want=one("C");want.nodes[0].label="P".into();assert_eq!(parse_wardley("wardley-beta\ncomponent P [0.5,0.1]\npipeline 'P' { %% outside\ncomponent C [0.25]\n}\n").unwrap(),want);eprintln!("WARDLEY-PIPELINE-HEADER single/' '");}
 {let mut want=one("C");want.nodes[0].label="P".into();assert_eq!(parse_wardley("wardley-beta\ncomponent P [0.5,0.1]\npipeline\t'P' { %% outside\ncomponent C [0.25]\n}\n").unwrap(),want);eprintln!("WARDLEY-PIPELINE-HEADER single/'\t'");}
 {let mut want=one("C");want.nodes[0].label="P".into();assert_eq!(parse_wardley("wardley-beta\ncomponent P [0.5,0.1]\npipeline \t  'P' { %% outside\ncomponent C [0.25]\n}\n").unwrap(),want);eprintln!("WARDLEY-PIPELINE-HEADER single/' \t  '");}
 {let mut want=one("C");want.nodes[0].label="A{B}".into();assert_eq!(parse_wardley("wardley-beta\ncomponent A{B} [0.5,0.1]\npipeline \"A{B}\" { %% outside\ncomponent C [0.25]\n}\n").unwrap(),want);eprintln!("WARDLEY-PIPELINE-HEADER brace/' '");}
 {let mut want=one("C");want.nodes[0].label="A{B}".into();assert_eq!(parse_wardley("wardley-beta\ncomponent A{B} [0.5,0.1]\npipeline\t\"A{B}\" { %% outside\ncomponent C [0.25]\n}\n").unwrap(),want);eprintln!("WARDLEY-PIPELINE-HEADER brace/'\t'");}
 {let mut want=one("C");want.nodes[0].label="A{B}".into();assert_eq!(parse_wardley("wardley-beta\ncomponent A{B} [0.5,0.1]\npipeline \t  \"A{B}\" { %% outside\ncomponent C [0.25]\n}\n").unwrap(),want);eprintln!("WARDLEY-PIPELINE-HEADER brace/' \t  '");}
 {let mut want=one("C");want.nodes[0].label="A[B]".into();assert_eq!(parse_wardley("wardley-beta\ncomponent A[B] [0.5,0.1]\npipeline \"A[B]\" { %% outside\ncomponent C [0.25]\n}\n").unwrap(),want);eprintln!("WARDLEY-PIPELINE-HEADER bracket/' '");}
 {let mut want=one("C");want.nodes[0].label="A[B]".into();assert_eq!(parse_wardley("wardley-beta\ncomponent A[B] [0.5,0.1]\npipeline\t\"A[B]\" { %% outside\ncomponent C [0.25]\n}\n").unwrap(),want);eprintln!("WARDLEY-PIPELINE-HEADER bracket/'\t'");}
 {let mut want=one("C");want.nodes[0].label="A[B]".into();assert_eq!(parse_wardley("wardley-beta\ncomponent A[B] [0.5,0.1]\npipeline \t  \"A[B]\" { %% outside\ncomponent C [0.25]\n}\n").unwrap(),want);eprintln!("WARDLEY-PIPELINE-HEADER bracket/' \t  '");}
 {let mut want=one("C");want.nodes[0].label="A\"B".into();assert_eq!(parse_wardley("wardley-beta\ncomponent A\"B [0.5,0.1]\npipeline \"A\\\"B\" { %% outside\ncomponent C [0.25]\n}\n").unwrap(),want);eprintln!("WARDLEY-PIPELINE-HEADER escaped-double/' '");}
 {let mut want=one("C");want.nodes[0].label="A\"B".into();assert_eq!(parse_wardley("wardley-beta\ncomponent A\"B [0.5,0.1]\npipeline\t\"A\\\"B\" { %% outside\ncomponent C [0.25]\n}\n").unwrap(),want);eprintln!("WARDLEY-PIPELINE-HEADER escaped-double/'\t'");}
 {let mut want=one("C");want.nodes[0].label="A\"B".into();assert_eq!(parse_wardley("wardley-beta\ncomponent A\"B [0.5,0.1]\npipeline \t  \"A\\\"B\" { %% outside\ncomponent C [0.25]\n}\n").unwrap(),want);eprintln!("WARDLEY-PIPELINE-HEADER escaped-double/' \t  '");}
 {let mut want=one("C");want.nodes[0].label="A'B".into();assert_eq!(parse_wardley("wardley-beta\ncomponent A'B [0.5,0.1]\npipeline 'A\\'B' { %% outside\ncomponent C [0.25]\n}\n").unwrap(),want);eprintln!("WARDLEY-PIPELINE-HEADER escaped-single/' '");}
 {let mut want=one("C");want.nodes[0].label="A'B".into();assert_eq!(parse_wardley("wardley-beta\ncomponent A'B [0.5,0.1]\npipeline\t'A\\'B' { %% outside\ncomponent C [0.25]\n}\n").unwrap(),want);eprintln!("WARDLEY-PIPELINE-HEADER escaped-single/'\t'");}
 {let mut want=one("C");want.nodes[0].label="A'B".into();assert_eq!(parse_wardley("wardley-beta\ncomponent A'B [0.5,0.1]\npipeline \t  'A\\'B' { %% outside\ncomponent C [0.25]\n}\n").unwrap(),want);eprintln!("WARDLEY-PIPELINE-HEADER escaped-single/' \t  '");}
 {let mut want=one("C");want.nodes[0].label="α निर्णय".into();assert_eq!(parse_wardley("wardley-beta\ncomponent α निर्णय [0.5,0.1]\npipeline \"α निर्णय\" { %% outside\ncomponent C [0.25]\n}\n").unwrap(),want);eprintln!("WARDLEY-PIPELINE-HEADER unicode/' '");}
 {let mut want=one("C");want.nodes[0].label="α निर्णय".into();assert_eq!(parse_wardley("wardley-beta\ncomponent α निर्णय [0.5,0.1]\npipeline\t\"α निर्णय\" { %% outside\ncomponent C [0.25]\n}\n").unwrap(),want);eprintln!("WARDLEY-PIPELINE-HEADER unicode/'\t'");}
 {let mut want=one("C");want.nodes[0].label="α निर्णय".into();assert_eq!(parse_wardley("wardley-beta\ncomponent α निर्णय [0.5,0.1]\npipeline \t  \"α निर्णय\" { %% outside\ncomponent C [0.25]\n}\n").unwrap(),want);eprintln!("WARDLEY-PIPELINE-HEADER unicode/' \t  '");}
}
#[test]
fn first_label_duplicates_and_flat_block_order_have_complete_ir() {
 let source="wardley-beta
component P [0.8,0.1]
component P [0.2,0.9]
pipeline P {
component C [1]
component C [0]
}
component Q [0.25,0.5]
pipeline Q {
component D [0.5]
}
";
 let mut want=expected();want.nodes=vec![node("wardley-1","P",0.8,0.1,true),node("wardley-2","P",0.2,0.9,false),node("wardley-3","C",0.8,1.0,false),node("wardley-4","C",0.8,0.0,false),node("wardley-5","Q",0.25,0.5,true),node("wardley-6","D",0.25,0.5,false)];want.pipelines=vec![WardleyPipeline{parent_id:"wardley-1".into(),component_ids:vec!["wardley-3".into(),"wardley-4".into()]},WardleyPipeline{parent_id:"wardley-5".into(),component_ids:vec!["wardley-6".into()]}];
 assert_eq!(parse_wardley(source).unwrap(),want);eprintln!("WARDLEY-PIPELINE-FIRST-LABEL wholeIR");
}
#[test]
fn pipeline_closed_source_errors_have_exact_messages_and_physical_locations() {
 for (body,message,line) in [
 ("pipeline {\ncomponent C [0.5]\n}\n","Wardley pipeline parent label must be non-empty",4),
 ("pipeline \"\" {\ncomponent C [0.5]\n}\n","Wardley pipeline parent label must be non-empty",4),
 ("pipeline '' {\ncomponent C [0.5]\n}\n","Wardley pipeline parent label must be non-empty",4),
 ("pipeline \"P {\ncomponent C [0.5]\n}\n","Wardley pipeline parent label must be a complete quoted string",4),
 ("pipeline 'P {\ncomponent C [0.5]\n}\n","Wardley pipeline parent label must be a complete quoted string",4),
 ("pipeline \"P\\n\" {\ncomponent C [0.5]\n}\n","Wardley pipeline parent label must be a complete quoted string",4),
 ("pipeline \"P\" junk {\ncomponent C [0.5]\n}\n","invalid Wardley pipeline parent suffix",4),
 ("pipeline \"P\"junk {\ncomponent C [0.5]\n}\n","invalid Wardley pipeline parent suffix",4),
 ("pipeline P\ncomponent C [0.5]\n}\n","Wardley pipeline requires an opening brace",4),
 ("pipeline \"P\"\ncomponent C [0.5]\n}\n","Wardley pipeline requires an opening brace",4),
 ("pipeline P { junk\ncomponent C [0.5]\n}\n","invalid Wardley pipeline header suffix",4),
 ("pipeline Missing {\ncomponent C [0.5]\n}\n","Wardley pipeline Missing must reference an existing component",4),
 ("pipeline \"A%%B\" {\ncomponent C [0.5]\n}\n","Wardley pipeline A%%B must reference an existing component",4),
 ("pipeline P {\ncomponent [0.5]\n}\n","Wardley pipeline component label must be non-empty",5),
 ("pipeline P {\ncomponent \"\" [0.5]\n}\n","Wardley pipeline component label must be non-empty",5),
 ("pipeline P {\ncomponent '' [0.5]\n}\n","Wardley pipeline component label must be non-empty",5),
 ("pipeline P {\ncomponent \"C [0.5]\n}\n","Wardley pipeline component label must be a complete quoted string",5),
 ("pipeline P {\ncomponent 'C [0.5]\n}\n","Wardley pipeline component label must be a complete quoted string",5),
 ("pipeline P {\ncomponent \"C\\n\" [0.5]\n}\n","Wardley pipeline component label must be a complete quoted string",5),
 ("pipeline P {\ncomponent \"C\" junk [0.5]\n}\n","invalid Wardley pipeline component suffix",5),
 ("pipeline P {\ncomponent C\n}\n","Wardley pipeline component requires evolution",5),
 ("pipeline P {\ncomponent C [0.5\n}\n","invalid Wardley pipeline evolution",5),
 ("pipeline P {\ncomponent C [0.5,0.5]\n}\n","invalid Wardley pipeline evolution",5),
 ("pipeline P {\ncomponent C [bad]\n}\n","invalid Wardley pipeline evolution",5),
 ("pipeline P {\ncomponent C [0.5]junk\n}\n","invalid Wardley pipeline evolution suffix",5),
 ("pipeline P {\ncomponent C [0.5] [0.2]\n}\n","invalid Wardley pipeline evolution suffix",5),
 ("pipeline P {\ncomponent C [-0.1]\n}\n","Wardley pipeline evolution must be between 0 and 1",5),
 ("pipeline P {\ncomponent C [1.1]\n}\n","Wardley pipeline evolution must be between 0 and 1",5),
 ("pipeline P {\ncomponent C [NaN]\n}\n","Wardley pipeline evolution must be between 0 and 1",5),
 ("pipeline P {\ncomponent C [inf]\n}\n","Wardley pipeline evolution must be between 0 and 1",5),
 ("pipeline P {\ncomponent C [-inf]\n}\n","Wardley pipeline evolution must be between 0 and 1",5),
 ("pipeline P {\ncomponent C [1.7976931348623157e308]\n}\n","Wardley pipeline evolution must be between 0 and 1",5),
 ("pipeline P {\n}\n","Wardley pipeline requires at least one component",5),
 ("pipeline P {\ncomponent C [0.5]\n","unterminated Wardley pipeline",5),
 ("}\n","unexpected Wardley pipeline terminator",4),
 ("pipeline P {\ncomponent C [0.5]\n} junk\n","invalid Wardley pipeline terminator",6),
 ("pipeline P {\nnote \"N\" [0.5,0.5]\n}\n","Wardley pipeline bodies only accept components",5),
 ("pipeline P {\nannotation 1,[0.5,0.5] \"A\"\n}\n","Wardley pipeline bodies only accept components",5),
 ("pipeline P {\naccelerator A [0.5,0.5]\n}\n","Wardley pipeline bodies only accept components",5),
 ("pipeline P {\ndeaccelerator D [0.5,0.5]\n}\n","Wardley pipeline bodies only accept components",5),
 ("pipeline P {\nanchor A [0.5,0.5]\n}\n","Wardley pipeline bodies only accept components",5),
 ("pipeline P {\npipeline P {\ncomponent C [0.5]\n}\n}\n","nested Wardley pipelines are unsupported",5),
 ("pipeline P {\ncomponent C [0.5]\n}\npipeline P {\ncomponent D [0.5]\n}\n","repeated Wardley pipeline parent",7),
 ("pipeline P {\ncomponent C [0.5]\n}\npipeline C {\ncomponent D [0.5]\n}\n","nested Wardley pipeline parents are unsupported",7),
 ] {let source=format!("wardley-beta
component P [0.5,0.1]

{body}");let e=parse_wardley(&source).unwrap_err();assert_eq!((e.message.as_str(),e.line,e.col),(message,line,1),"{source}");eprintln!("WARDLEY-PIPELINE-SOURCE-ERROR line={line} {message}");}
}
