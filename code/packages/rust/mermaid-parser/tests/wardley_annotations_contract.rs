//! Complete annotation text/group IR and exact source errors, before import/repair.
use diagram_ir::{WardleyAnnotation,WardleyConfig,WardleyDiagram,WardleyNote,WardleyTheme};
use mermaid_parser::parse_wardley;
fn expected()->WardleyDiagram {WardleyDiagram {title:None,config:WardleyConfig {
 width:900.0,height:600.0,padding:48.0,node_radius:6.0,node_label_offset:8.0,axis_font_size:12.0,label_font_size:10.0,show_grid:false,use_max_width:true,
 theme:WardleyTheme {background_color:"#ffffff".into(),axis_color:"#000000".into(),axis_text_color:"#222222".into(),grid_color:"rgba(100, 100, 100, 0.2)".into(),component_fill:"#ffffff".into(),component_stroke:"#000000".into(),component_label_color:"#222222".into(),link_stroke:"#000000".into(),evolution_stroke:"#dc3545".into()}},
 stages:vec!["Genesis".into(),"Custom Built".into(),"Product".into(),"Commodity".into()],nodes:vec![],links:vec![],evolves:vec![],notes:vec![],annotations_box:None,annotations:vec![],forces:vec![]}}
#[test]
fn annotation_text_and_group_order_have_complete_literal_ir() {
 {let mut want=expected();want.annotations_box=None;want.annotations=vec![WardleyAnnotation{number:7,text:"Decision".into(),coordinates:vec![(0.25,0.75)]}];want.notes=vec![];
 assert_eq!(parse_wardley("wardley-beta\nannotation 7,[0.25,0.75] \"Decision\"\n").unwrap_or_else(|e|panic!("double: {e:?}")),want,"double");eprintln!("WARDLEY-ANNOTATION-IR double");}
 {let mut want=expected();want.annotations_box=None;want.annotations=vec![WardleyAnnotation{number:7,text:"Decision".into(),coordinates:vec![(0.25,0.75)]}];want.notes=vec![];
 assert_eq!(parse_wardley("wardley-beta\nannotation 7,[0.25,0.75] 'Decision'\n").unwrap_or_else(|e|panic!("single: {e:?}")),want,"single");eprintln!("WARDLEY-ANNOTATION-IR single");}
 {let mut want=expected();want.annotations_box=None;want.annotations=vec![WardleyAnnotation{number:7,text:"".into(),coordinates:vec![(0.25,0.75)]}];want.notes=vec![];
 assert_eq!(parse_wardley("wardley-beta\nannotation 7,[0.25,0.75] \"\"\n").unwrap_or_else(|e|panic!("empty-double: {e:?}")),want,"empty-double");eprintln!("WARDLEY-ANNOTATION-IR empty-double");}
 {let mut want=expected();want.annotations_box=None;want.annotations=vec![WardleyAnnotation{number:7,text:"".into(),coordinates:vec![(0.25,0.75)]}];want.notes=vec![];
 assert_eq!(parse_wardley("wardley-beta\nannotation 7,[0.25,0.75] ''\n").unwrap_or_else(|e|panic!("empty-single: {e:?}")),want,"empty-single");eprintln!("WARDLEY-ANNOTATION-IR empty-single");}
 {let mut want=expected();want.annotations_box=None;want.annotations=vec![WardleyAnnotation{number:7,text:"  inside  ".into(),coordinates:vec![(0.25,0.75)]}];want.notes=vec![];
 assert_eq!(parse_wardley("wardley-beta\nannotation 7,[0.25,0.75] \"  inside  \"\n").unwrap_or_else(|e|panic!("spaces: {e:?}")),want,"spaces");eprintln!("WARDLEY-ANNOTATION-IR spaces");}
 {let mut want=expected();want.annotations_box=None;want.annotations=vec![WardleyAnnotation{number:7,text:"A'B".into(),coordinates:vec![(0.25,0.75)]}];want.notes=vec![];
 assert_eq!(parse_wardley("wardley-beta\nannotation 7,[0.25,0.75] \"A'B\"\n").unwrap_or_else(|e|panic!("inactive-single: {e:?}")),want,"inactive-single");eprintln!("WARDLEY-ANNOTATION-IR inactive-single");}
 {let mut want=expected();want.annotations_box=None;want.annotations=vec![WardleyAnnotation{number:7,text:"A\"B".into(),coordinates:vec![(0.25,0.75)]}];want.notes=vec![];
 assert_eq!(parse_wardley("wardley-beta\nannotation 7,[0.25,0.75] 'A\"B'\n").unwrap_or_else(|e|panic!("inactive-double: {e:?}")),want,"inactive-double");eprintln!("WARDLEY-ANNOTATION-IR inactive-double");}
 {let mut want=expected();want.annotations_box=None;want.annotations=vec![WardleyAnnotation{number:7,text:"A\"B".into(),coordinates:vec![(0.25,0.75)]}];want.notes=vec![];
 assert_eq!(parse_wardley("wardley-beta\nannotation 7,[0.25,0.75] \"A\\\"B\"\n").unwrap_or_else(|e|panic!("escaped-double: {e:?}")),want,"escaped-double");eprintln!("WARDLEY-ANNOTATION-IR escaped-double");}
 {let mut want=expected();want.annotations_box=None;want.annotations=vec![WardleyAnnotation{number:7,text:"A'B".into(),coordinates:vec![(0.25,0.75)]}];want.notes=vec![];
 assert_eq!(parse_wardley("wardley-beta\nannotation 7,[0.25,0.75] 'A\\'B'\n").unwrap_or_else(|e|panic!("escaped-single: {e:?}")),want,"escaped-single");eprintln!("WARDLEY-ANNOTATION-IR escaped-single");}
 {let mut want=expected();want.annotations_box=None;want.annotations=vec![WardleyAnnotation{number:7,text:"A\\B".into(),coordinates:vec![(0.25,0.75)]}];want.notes=vec![];
 assert_eq!(parse_wardley("wardley-beta\nannotation 7,[0.25,0.75] \"A\\\\B\"\n").unwrap_or_else(|e|panic!("backslash: {e:?}")),want,"backslash");eprintln!("WARDLEY-ANNOTATION-IR backslash");}
 {let mut want=expected();want.annotations_box=None;want.annotations=vec![WardleyAnnotation{number:7,text:"α निर्णय".into(),coordinates:vec![(0.25,0.75)]}];want.notes=vec![];
 assert_eq!(parse_wardley("wardley-beta\nannotation 7,[0.25,0.75] \"α निर्णय\"\n").unwrap_or_else(|e|panic!("unicode: {e:?}")),want,"unicode");eprintln!("WARDLEY-ANNOTATION-IR unicode");}
 {let mut want=expected();want.annotations_box=None;want.annotations=vec![WardleyAnnotation{number:7,text:"[A], ] B".into(),coordinates:vec![(0.25,0.75)]}];want.notes=vec![];
 assert_eq!(parse_wardley("wardley-beta\nannotation 7,[0.25,0.75] \"[A], ] B\"\n").unwrap_or_else(|e|panic!("brackets: {e:?}")),want,"brackets");eprintln!("WARDLEY-ANNOTATION-IR brackets");}
 {let mut want=expected();want.annotations_box=None;want.annotations=vec![WardleyAnnotation{number:7,text:"50%% complete".into(),coordinates:vec![(0.25,0.75)]}];want.notes=vec![];
 assert_eq!(parse_wardley("wardley-beta\nannotation 7,[0.25,0.75] \"50%% complete\"\n").unwrap_or_else(|e|panic!("percent-inside: {e:?}")),want,"percent-inside");eprintln!("WARDLEY-ANNOTATION-IR percent-inside");}
 {let mut want=expected();want.annotations_box=None;want.annotations=vec![WardleyAnnotation{number:7,text:"A".into(),coordinates:vec![(0.25,0.75)]}];want.notes=vec![];
 assert_eq!(parse_wardley("wardley-beta\nannotation 7,[0.25,0.75] \"A\" %% outside \"junk\n").unwrap_or_else(|e|panic!("comment-outside: {e:?}")),want,"comment-outside");eprintln!("WARDLEY-ANNOTATION-IR comment-outside");}
 {let mut want=expected();want.annotations_box=None;want.annotations=vec![WardleyAnnotation{number:7,text:"A%%B".into(),coordinates:vec![(0.25,0.75)]}];want.notes=vec![];
 assert_eq!(parse_wardley("wardley-beta\nannotation 7,[0.25,0.75] 'A%%B' %% outside\n").unwrap_or_else(|e|panic!("single-percent: {e:?}")),want,"single-percent");eprintln!("WARDLEY-ANNOTATION-IR single-percent");}
 {let mut want=expected();want.annotations_box=None;want.annotations=vec![WardleyAnnotation{number:3,text:"Now".into(),coordinates:vec![(0.0,0.0),(0.5,0.5),(1.0,1.0)]}];want.notes=vec![];
 assert_eq!(parse_wardley("wardley-beta\nannotation 3,[0.0,0.0] ''\nannotation 3,[0.5,0.5] \"Now\"\nannotation 3,[1.0,1.0] \"Ignored\"\n").unwrap_or_else(|e|panic!("empty-first-single-updates: {e:?}")),want,"empty-first-single-updates");eprintln!("WARDLEY-ANNOTATION-IR empty-first-single-updates");}
 {let mut want=expected();want.annotations_box=None;want.annotations=vec![WardleyAnnotation{number:1,text:" ".into(),coordinates:vec![(0.0,1.0),(1.0,0.0)]}];want.notes=vec![];
 assert_eq!(parse_wardley("wardley-beta\nannotation 1,[0.0,1.0] \" \"\nannotation 1,[1.0,0.0] \"Ignored\"\n").unwrap_or_else(|e|panic!("space-is-nonempty: {e:?}")),want,"space-is-nonempty");eprintln!("WARDLEY-ANNOTATION-IR space-is-nonempty");}
 {let mut want=expected();want.annotations_box=Some((0.25,0.75));want.annotations=vec![WardleyAnnotation{number:3,text:"Three".into(),coordinates:vec![(0.0,0.0),(0.5,0.5)]},WardleyAnnotation{number:1,text:"One".into(),coordinates:vec![(0.25,0.75),(0.75,0.25)]},WardleyAnnotation{number:2,text:"Two".into(),coordinates:vec![(1.0,1.0)]}];want.notes=vec![];
 assert_eq!(parse_wardley("wardley-beta\nannotations [0.25,0.75]\nannotation 3,[0.0,0.0] \"\"\nannotation 1,[0.25,0.75] \"One\"\nannotation 3,[0.5,0.5] \"Three\"\nannotation 2,[1.0,1.0] \"Two\"\nannotation 1,[0.75,0.25] \"Ignored\"\n").unwrap_or_else(|e|panic!("stable-interleaved-groups: {e:?}")),want,"stable-interleaved-groups");eprintln!("WARDLEY-ANNOTATION-IR stable-interleaved-groups");}
 {let mut want=expected();want.annotations_box=Some((0.75,0.25));want.annotations=vec![];want.notes=vec![];
 assert_eq!(parse_wardley("wardley-beta\nannotations [0.0,1.0]\nannotations [0.75,0.25] %% outside\n").unwrap_or_else(|e|panic!("box-only-last-wins: {e:?}")),want,"box-only-last-wins");eprintln!("WARDLEY-ANNOTATION-IR box-only-last-wins");}
 {let mut want=expected();want.annotations_box=None;want.annotations=vec![WardleyAnnotation{number:0,text:"A%%[]".into(),coordinates:vec![(0.25,0.75)]}];want.notes=vec![WardleyNote{text:"N%%[]".into(),visibility:0.5,evolution:0.5}];
 assert_eq!(parse_wardley("wardley-beta\nnote 'N%%[]' [0.5,0.5] %% outside\nannotation 0,[0.25,0.75] 'A%%[]' %% outside\n").unwrap_or_else(|e|panic!("note-reader-retained: {e:?}")),want,"note-reader-retained");eprintln!("WARDLEY-ANNOTATION-IR note-reader-retained");}
}
#[test]
fn annotation_errors_bind_complete_message_and_physical_line() {
 for (statement,want) in [
  ("annotation bad","invalid Wardley annotation"),
  ("annotation x,[0.5,0.5] \"A\"","invalid Wardley annotation number"),
  ("annotation -1,[0.5,0.5] \"A\"","invalid Wardley annotation number"),
  ("annotation 1,[0.5,0.5 \"A\"","Wardley annotation requires coordinates"),
  ("annotation 1,[0.5] \"A\"","coordinates require two values"),
  ("annotation 1,[oops,0.5] \"A\"","invalid coordinate"),
  ("annotation 1,[0.5,0.5]","Wardley annotation text must be quoted"),
  ("annotation 1,[0.5,0.5] A","Wardley annotation text must be quoted"),
  ("annotation 1,[0.5,0.5] \"A","Wardley annotation text must be a complete quoted string"),
  ("annotation 1,[0.5,0.5] 'A","Wardley annotation text must be a complete quoted string"),
  ("annotation 1,[0.5,0.5] \"A\\nB\"","Wardley annotation text must be a complete quoted string"),
  ("annotation 1,[0.5,0.5] \"A\"junk\"","invalid Wardley annotation text suffix"),
  ("annotation 1,[0.5,0.5] 'A' junk","invalid Wardley annotation text suffix"),
  ("annotation 1,[0.5,0.5] \"A\" [0.5,0.5]","invalid Wardley annotation text suffix"),
  ("annotation 1,[-0.1,0.5] \"A\"","Wardley annotation coordinates must be between 0 and 1"),
  ("annotations [-0.1,0.5]","Wardley annotations box coordinates must be between 0 and 1"),
  ("annotation 1,[0.5,-0.1] \"A\"","Wardley annotation coordinates must be between 0 and 1"),
  ("annotations [0.5,-0.1]","Wardley annotations box coordinates must be between 0 and 1"),
  ("annotation 1,[1.1,0.5] \"A\"","Wardley annotation coordinates must be between 0 and 1"),
  ("annotations [1.1,0.5]","Wardley annotations box coordinates must be between 0 and 1"),
  ("annotation 1,[0.5,1.1] \"A\"","Wardley annotation coordinates must be between 0 and 1"),
  ("annotations [0.5,1.1]","Wardley annotations box coordinates must be between 0 and 1"),
  ("annotation 1,[NaN,0.5] \"A\"","Wardley annotation coordinates must be between 0 and 1"),
  ("annotations [NaN,0.5]","Wardley annotations box coordinates must be between 0 and 1"),
  ("annotation 1,[0.5,NaN] \"A\"","Wardley annotation coordinates must be between 0 and 1"),
  ("annotations [0.5,NaN]","Wardley annotations box coordinates must be between 0 and 1"),
  ("annotation 1,[inf,0.5] \"A\"","Wardley annotation coordinates must be between 0 and 1"),
  ("annotations [inf,0.5]","Wardley annotations box coordinates must be between 0 and 1"),
  ("annotation 1,[0.5,inf] \"A\"","Wardley annotation coordinates must be between 0 and 1"),
  ("annotations [0.5,inf]","Wardley annotations box coordinates must be between 0 and 1"),
  ("annotation 1,[-inf,0.5] \"A\"","Wardley annotation coordinates must be between 0 and 1"),
  ("annotations [-inf,0.5]","Wardley annotations box coordinates must be between 0 and 1"),
  ("annotation 1,[0.5,-inf] \"A\"","Wardley annotation coordinates must be between 0 and 1"),
  ("annotations [0.5,-inf]","Wardley annotations box coordinates must be between 0 and 1"),
 ] {let source=format!("wardley-beta\n\n{statement}\n");let e=parse_wardley(&source).unwrap_err();
 assert_eq!((e.message.as_str(),e.line,e.col),(want,3,1),"{statement}");eprintln!("WARDLEY-ANNOTATION-SOURCE-ERROR {statement}");}
}
