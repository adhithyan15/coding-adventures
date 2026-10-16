//! Literal whole force IR and physical-source errors, committed before force import/repair.
use diagram_ir::{WardleyConfig,WardleyDiagram,WardleyForce,WardleyForceKind,WardleyTheme};
use mermaid_parser::parse_wardley;
fn expected()->WardleyDiagram {WardleyDiagram {title:None,config:WardleyConfig {
 width:900.0,height:600.0,padding:48.0,node_radius:6.0,node_label_offset:8.0,axis_font_size:12.0,label_font_size:10.0,show_grid:false,use_max_width:true,
 theme:WardleyTheme {background_color:"#ffffff".into(),axis_color:"#000000".into(),axis_text_color:"#222222".into(),grid_color:"rgba(100, 100, 100, 0.2)".into(),component_fill:"#ffffff".into(),component_stroke:"#000000".into(),component_label_color:"#222222".into(),link_stroke:"#000000".into(),evolution_stroke:"#dc3545".into()}},
 stages:vec!["Genesis".into(),"Custom Built".into(),"Product".into(),"Commodity".into()],nodes:vec![],links:vec![],evolves:vec![],notes:vec![],annotations_box:None,annotations:vec![],forces:vec![]}}
#[test]
fn force_labels_have_complete_literal_ir_for_both_directions() {
 for (keyword,kind) in [("accelerator",WardleyForceKind::Accelerator),("deaccelerator",WardleyForceKind::Deaccelerator)] {
 {let mut want=expected();want.forces=vec![WardleyForce{label:"A".into(),visibility:0.25,evolution:0.75,kind:kind.clone()}];
 let source=format!("wardley-beta\n{keyword} {}\n","A [0.25,0.75]");assert_eq!(parse_wardley(&source).unwrap_or_else(|e|panic!("{source}: {e:?}")),want);eprintln!("WARDLEY-FORCE-IR {keyword}/bare");}
 {let mut want=expected();want.forces=vec![WardleyForce{label:"Alpha Beta".into(),visibility:0.25,evolution:0.75,kind:kind.clone()}];
 let source=format!("wardley-beta\n{keyword} {}\n","  Alpha Beta  [0.25,0.75]");assert_eq!(parse_wardley(&source).unwrap_or_else(|e|panic!("{source}: {e:?}")),want);eprintln!("WARDLEY-FORCE-IR {keyword}/bare-many");}
 {let mut want=expected();want.forces=vec![WardleyForce{label:"A".into(),visibility:0.25,evolution:0.75,kind:kind.clone()}];
 let source=format!("wardley-beta\n{keyword} {}\n","\"A\" [0.25,0.75]");assert_eq!(parse_wardley(&source).unwrap_or_else(|e|panic!("{source}: {e:?}")),want);eprintln!("WARDLEY-FORCE-IR {keyword}/double");}
 {let mut want=expected();want.forces=vec![WardleyForce{label:"A".into(),visibility:0.25,evolution:0.75,kind:kind.clone()}];
 let source=format!("wardley-beta\n{keyword} {}\n","'A' [0.25,0.75]");assert_eq!(parse_wardley(&source).unwrap_or_else(|e|panic!("{source}: {e:?}")),want);eprintln!("WARDLEY-FORCE-IR {keyword}/single");}
 {let mut want=expected();want.forces=vec![WardleyForce{label:"  A  ".into(),visibility:0.25,evolution:0.75,kind:kind.clone()}];
 let source=format!("wardley-beta\n{keyword} {}\n","\"  A  \" [0.25,0.75]");assert_eq!(parse_wardley(&source).unwrap_or_else(|e|panic!("{source}: {e:?}")),want);eprintln!("WARDLEY-FORCE-IR {keyword}/spaces");}
 {let mut want=expected();want.forces=vec![WardleyForce{label:" ".into(),visibility:0.25,evolution:0.75,kind:kind.clone()}];
 let source=format!("wardley-beta\n{keyword} {}\n","' ' [0.25,0.75]");assert_eq!(parse_wardley(&source).unwrap_or_else(|e|panic!("{source}: {e:?}")),want);eprintln!("WARDLEY-FORCE-IR {keyword}/one-space");}
 {let mut want=expected();want.forces=vec![WardleyForce{label:"A'B".into(),visibility:0.25,evolution:0.75,kind:kind.clone()}];
 let source=format!("wardley-beta\n{keyword} {}\n","\"A'B\" [0.25,0.75]");assert_eq!(parse_wardley(&source).unwrap_or_else(|e|panic!("{source}: {e:?}")),want);eprintln!("WARDLEY-FORCE-IR {keyword}/inactive-single");}
 {let mut want=expected();want.forces=vec![WardleyForce{label:"A\"B".into(),visibility:0.25,evolution:0.75,kind:kind.clone()}];
 let source=format!("wardley-beta\n{keyword} {}\n","'A\"B' [0.25,0.75]");assert_eq!(parse_wardley(&source).unwrap_or_else(|e|panic!("{source}: {e:?}")),want);eprintln!("WARDLEY-FORCE-IR {keyword}/inactive-double");}
 {let mut want=expected();want.forces=vec![WardleyForce{label:"A\"B".into(),visibility:0.25,evolution:0.75,kind:kind.clone()}];
 let source=format!("wardley-beta\n{keyword} {}\n","\"A\\\"B\" [0.25,0.75]");assert_eq!(parse_wardley(&source).unwrap_or_else(|e|panic!("{source}: {e:?}")),want);eprintln!("WARDLEY-FORCE-IR {keyword}/escaped-double");}
 {let mut want=expected();want.forces=vec![WardleyForce{label:"A'B".into(),visibility:0.25,evolution:0.75,kind:kind.clone()}];
 let source=format!("wardley-beta\n{keyword} {}\n","'A\\'B' [0.25,0.75]");assert_eq!(parse_wardley(&source).unwrap_or_else(|e|panic!("{source}: {e:?}")),want);eprintln!("WARDLEY-FORCE-IR {keyword}/escaped-single");}
 {let mut want=expected();want.forces=vec![WardleyForce{label:"A\\B".into(),visibility:0.25,evolution:0.75,kind:kind.clone()}];
 let source=format!("wardley-beta\n{keyword} {}\n","\"A\\\\B\" [0.25,0.75]");assert_eq!(parse_wardley(&source).unwrap_or_else(|e|panic!("{source}: {e:?}")),want);eprintln!("WARDLEY-FORCE-IR {keyword}/escaped-slash");}
 {let mut want=expected();want.forces=vec![WardleyForce{label:"α निर्णय".into(),visibility:0.25,evolution:0.75,kind:kind.clone()}];
 let source=format!("wardley-beta\n{keyword} {}\n","\"α निर्णय\" [0.25,0.75]");assert_eq!(parse_wardley(&source).unwrap_or_else(|e|panic!("{source}: {e:?}")),want);eprintln!("WARDLEY-FORCE-IR {keyword}/unicode");}
 {let mut want=expected();want.forces=vec![WardleyForce{label:"[A], ] B".into(),visibility:0.25,evolution:0.75,kind:kind.clone()}];
 let source=format!("wardley-beta\n{keyword} {}\n","\"[A], ] B\" [0.25,0.75]");assert_eq!(parse_wardley(&source).unwrap_or_else(|e|panic!("{source}: {e:?}")),want);eprintln!("WARDLEY-FORCE-IR {keyword}/brackets");}
 {let mut want=expected();want.forces=vec![WardleyForce{label:"A%%B".into(),visibility:0.25,evolution:0.75,kind:kind.clone()}];
 let source=format!("wardley-beta\n{keyword} {}\n","\"A%%B\" [0.25,0.75]");assert_eq!(parse_wardley(&source).unwrap_or_else(|e|panic!("{source}: {e:?}")),want);eprintln!("WARDLEY-FORCE-IR {keyword}/percent");}
 {let mut want=expected();want.forces=vec![WardleyForce{label:"A%%B".into(),visibility:0.25,evolution:0.75,kind:kind.clone()}];
 let source=format!("wardley-beta\n{keyword} {}\n","'A%%B' [0.25,0.75]");assert_eq!(parse_wardley(&source).unwrap_or_else(|e|panic!("{source}: {e:?}")),want);eprintln!("WARDLEY-FORCE-IR {keyword}/single-percent");}
 {let mut want=expected();want.forces=vec![WardleyForce{label:"A".into(),visibility:0.25,evolution:0.75,kind:kind.clone()}];
 let source=format!("wardley-beta\n{keyword} {}\n","A [0.25,0.75] %% \"junk");assert_eq!(parse_wardley(&source).unwrap_or_else(|e|panic!("{source}: {e:?}")),want);eprintln!("WARDLEY-FORCE-IR {keyword}/outside-comment");}
 {let mut want=expected();want.forces=vec![WardleyForce{label:"A%%B".into(),visibility:0.25,evolution:0.75,kind:kind.clone()}];
 let source=format!("wardley-beta\n{keyword} {}\n","\"A%%B\" [0.25,0.75] %% \"junk");assert_eq!(parse_wardley(&source).unwrap_or_else(|e|panic!("{source}: {e:?}")),want);eprintln!("WARDLEY-FORCE-IR {keyword}/inside-and-outside");}
 {let mut want=expected();want.forces=vec![WardleyForce{label:"A".into(),visibility:0.25,evolution:0.75,kind:kind.clone()}];
 let source=format!("wardley-beta\n{keyword} {}\n","A [ 0.25 , 0.75 ]");assert_eq!(parse_wardley(&source).unwrap_or_else(|e|panic!("{source}: {e:?}")),want);eprintln!("WARDLEY-FORCE-IR {keyword}/coordinate-space");}
 }}
#[test]
fn force_separators_edges_and_duplicates_preserve_order_and_independent_names() {
 {let mut want=expected();want.forces=vec![WardleyForce{label:"Missing".into(),visibility:0.0,evolution:0.0,kind:WardleyForceKind::Accelerator},WardleyForce{label:"Missing".into(),visibility:0.0,evolution:0.0,kind:WardleyForceKind::Deaccelerator},WardleyForce{label:"Missing".into(),visibility:0.0,evolution:0.0,kind:WardleyForceKind::Accelerator}];assert_eq!(parse_wardley("wardley-beta\naccelerator Missing [0,0]\ndeaccelerator Missing [0,0]\naccelerator Missing [0,0]\n").unwrap(),want);eprintln!("WARDLEY-FORCE-ORDER ' '/0/0");}
 {let mut want=expected();want.forces=vec![WardleyForce{label:"Missing".into(),visibility:0.0,evolution:1.0,kind:WardleyForceKind::Accelerator},WardleyForce{label:"Missing".into(),visibility:0.0,evolution:1.0,kind:WardleyForceKind::Deaccelerator},WardleyForce{label:"Missing".into(),visibility:0.0,evolution:1.0,kind:WardleyForceKind::Accelerator}];assert_eq!(parse_wardley("wardley-beta\naccelerator Missing [0,1]\ndeaccelerator Missing [0,1]\naccelerator Missing [0,1]\n").unwrap(),want);eprintln!("WARDLEY-FORCE-ORDER ' '/0/1");}
 {let mut want=expected();want.forces=vec![WardleyForce{label:"Missing".into(),visibility:1.0,evolution:0.0,kind:WardleyForceKind::Accelerator},WardleyForce{label:"Missing".into(),visibility:1.0,evolution:0.0,kind:WardleyForceKind::Deaccelerator},WardleyForce{label:"Missing".into(),visibility:1.0,evolution:0.0,kind:WardleyForceKind::Accelerator}];assert_eq!(parse_wardley("wardley-beta\naccelerator Missing [1,0]\ndeaccelerator Missing [1,0]\naccelerator Missing [1,0]\n").unwrap(),want);eprintln!("WARDLEY-FORCE-ORDER ' '/1/0");}
 {let mut want=expected();want.forces=vec![WardleyForce{label:"Missing".into(),visibility:1.0,evolution:1.0,kind:WardleyForceKind::Accelerator},WardleyForce{label:"Missing".into(),visibility:1.0,evolution:1.0,kind:WardleyForceKind::Deaccelerator},WardleyForce{label:"Missing".into(),visibility:1.0,evolution:1.0,kind:WardleyForceKind::Accelerator}];assert_eq!(parse_wardley("wardley-beta\naccelerator Missing [1,1]\ndeaccelerator Missing [1,1]\naccelerator Missing [1,1]\n").unwrap(),want);eprintln!("WARDLEY-FORCE-ORDER ' '/1/1");}
 {let mut want=expected();want.forces=vec![WardleyForce{label:"Missing".into(),visibility:0.0,evolution:0.0,kind:WardleyForceKind::Accelerator},WardleyForce{label:"Missing".into(),visibility:0.0,evolution:0.0,kind:WardleyForceKind::Deaccelerator},WardleyForce{label:"Missing".into(),visibility:0.0,evolution:0.0,kind:WardleyForceKind::Accelerator}];assert_eq!(parse_wardley("wardley-beta\naccelerator\tMissing [0,0]\ndeaccelerator\tMissing [0,0]\naccelerator\tMissing [0,0]\n").unwrap(),want);eprintln!("WARDLEY-FORCE-ORDER '\t'/0/0");}
 {let mut want=expected();want.forces=vec![WardleyForce{label:"Missing".into(),visibility:0.0,evolution:1.0,kind:WardleyForceKind::Accelerator},WardleyForce{label:"Missing".into(),visibility:0.0,evolution:1.0,kind:WardleyForceKind::Deaccelerator},WardleyForce{label:"Missing".into(),visibility:0.0,evolution:1.0,kind:WardleyForceKind::Accelerator}];assert_eq!(parse_wardley("wardley-beta\naccelerator\tMissing [0,1]\ndeaccelerator\tMissing [0,1]\naccelerator\tMissing [0,1]\n").unwrap(),want);eprintln!("WARDLEY-FORCE-ORDER '\t'/0/1");}
 {let mut want=expected();want.forces=vec![WardleyForce{label:"Missing".into(),visibility:1.0,evolution:0.0,kind:WardleyForceKind::Accelerator},WardleyForce{label:"Missing".into(),visibility:1.0,evolution:0.0,kind:WardleyForceKind::Deaccelerator},WardleyForce{label:"Missing".into(),visibility:1.0,evolution:0.0,kind:WardleyForceKind::Accelerator}];assert_eq!(parse_wardley("wardley-beta\naccelerator\tMissing [1,0]\ndeaccelerator\tMissing [1,0]\naccelerator\tMissing [1,0]\n").unwrap(),want);eprintln!("WARDLEY-FORCE-ORDER '\t'/1/0");}
 {let mut want=expected();want.forces=vec![WardleyForce{label:"Missing".into(),visibility:1.0,evolution:1.0,kind:WardleyForceKind::Accelerator},WardleyForce{label:"Missing".into(),visibility:1.0,evolution:1.0,kind:WardleyForceKind::Deaccelerator},WardleyForce{label:"Missing".into(),visibility:1.0,evolution:1.0,kind:WardleyForceKind::Accelerator}];assert_eq!(parse_wardley("wardley-beta\naccelerator\tMissing [1,1]\ndeaccelerator\tMissing [1,1]\naccelerator\tMissing [1,1]\n").unwrap(),want);eprintln!("WARDLEY-FORCE-ORDER '\t'/1/1");}
 {let mut want=expected();want.forces=vec![WardleyForce{label:"Missing".into(),visibility:0.0,evolution:0.0,kind:WardleyForceKind::Accelerator},WardleyForce{label:"Missing".into(),visibility:0.0,evolution:0.0,kind:WardleyForceKind::Deaccelerator},WardleyForce{label:"Missing".into(),visibility:0.0,evolution:0.0,kind:WardleyForceKind::Accelerator}];assert_eq!(parse_wardley("wardley-beta\naccelerator \t  \tMissing [0,0]\ndeaccelerator \t  \tMissing [0,0]\naccelerator \t  \tMissing [0,0]\n").unwrap(),want);eprintln!("WARDLEY-FORCE-ORDER ' \t  \t'/0/0");}
 {let mut want=expected();want.forces=vec![WardleyForce{label:"Missing".into(),visibility:0.0,evolution:1.0,kind:WardleyForceKind::Accelerator},WardleyForce{label:"Missing".into(),visibility:0.0,evolution:1.0,kind:WardleyForceKind::Deaccelerator},WardleyForce{label:"Missing".into(),visibility:0.0,evolution:1.0,kind:WardleyForceKind::Accelerator}];assert_eq!(parse_wardley("wardley-beta\naccelerator \t  \tMissing [0,1]\ndeaccelerator \t  \tMissing [0,1]\naccelerator \t  \tMissing [0,1]\n").unwrap(),want);eprintln!("WARDLEY-FORCE-ORDER ' \t  \t'/0/1");}
 {let mut want=expected();want.forces=vec![WardleyForce{label:"Missing".into(),visibility:1.0,evolution:0.0,kind:WardleyForceKind::Accelerator},WardleyForce{label:"Missing".into(),visibility:1.0,evolution:0.0,kind:WardleyForceKind::Deaccelerator},WardleyForce{label:"Missing".into(),visibility:1.0,evolution:0.0,kind:WardleyForceKind::Accelerator}];assert_eq!(parse_wardley("wardley-beta\naccelerator \t  \tMissing [1,0]\ndeaccelerator \t  \tMissing [1,0]\naccelerator \t  \tMissing [1,0]\n").unwrap(),want);eprintln!("WARDLEY-FORCE-ORDER ' \t  \t'/1/0");}
 {let mut want=expected();want.forces=vec![WardleyForce{label:"Missing".into(),visibility:1.0,evolution:1.0,kind:WardleyForceKind::Accelerator},WardleyForce{label:"Missing".into(),visibility:1.0,evolution:1.0,kind:WardleyForceKind::Deaccelerator},WardleyForce{label:"Missing".into(),visibility:1.0,evolution:1.0,kind:WardleyForceKind::Accelerator}];assert_eq!(parse_wardley("wardley-beta\naccelerator \t  \tMissing [1,1]\ndeaccelerator \t  \tMissing [1,1]\naccelerator \t  \tMissing [1,1]\n").unwrap(),want);eprintln!("WARDLEY-FORCE-ORDER ' \t  \t'/1/1");}
}
#[test]
fn force_source_errors_bind_whole_message_and_physical_line() {
 for keyword in ["accelerator","deaccelerator"] {for (tail,want) in [
 ("[0.5,0.5]","Wardley force label must be non-empty"),
 ("\"\" [0.5,0.5]","Wardley force label must be non-empty"),
 ("'' [0.5,0.5]","Wardley force label must be non-empty"),
 ("A","Wardley force requires coordinates"),
 ("A %% [0.5,0.5]","Wardley force requires coordinates"),
 ("\"A\"","Wardley force requires coordinates"),
 ("\"A [0.5,0.5]","Wardley force label must be a complete quoted string"),
 ("'A [0.5,0.5]","Wardley force label must be a complete quoted string"),
 ("\"A\\nB\" [0.5,0.5]","Wardley force label must be a complete quoted string"),
 ("'A\\\"B' [0.5,0.5]","Wardley force label must be a complete quoted string"),
 ("\"A\\","Wardley force label must be a complete quoted string"),
 ("\"A\" junk [0.5,0.5]","invalid Wardley force label suffix"),
 ("\"A\"junk [0.5,0.5]","invalid Wardley force label suffix"),
 ("A [0.5,0.5]junk","invalid Wardley force coordinates suffix"),
 ("A [0.5,0.5] [0.2,0.2]","invalid Wardley force coordinates suffix"),
 ("A [0.5,0.5","invalid Wardley coordinates"),
 ("A [0.5]","coordinates require two values"),
 ("A [bad,0.5]","invalid coordinate"),
 ("A [0.5,bad]","invalid coordinate"),
 ("A [0.5,0.5,0.5]","invalid coordinate"),
 ("A [-0.1,0.5]","Wardley force coordinates must be between 0 and 1"),
 ("A [0.5,-0.1]","Wardley force coordinates must be between 0 and 1"),
 ("A [1.1,0.5]","Wardley force coordinates must be between 0 and 1"),
 ("A [0.5,1.1]","Wardley force coordinates must be between 0 and 1"),
 ("A [NaN,0.5]","Wardley force coordinates must be between 0 and 1"),
 ("A [0.5,NaN]","Wardley force coordinates must be between 0 and 1"),
 ("A [inf,0.5]","Wardley force coordinates must be between 0 and 1"),
 ("A [0.5,inf]","Wardley force coordinates must be between 0 and 1"),
 ("A [-inf,0.5]","Wardley force coordinates must be between 0 and 1"),
 ("A [0.5,-inf]","Wardley force coordinates must be between 0 and 1"),
 ("A [1.7976931348623157e308,0.5]","Wardley force coordinates must be between 0 and 1"),
 ("A [0.5,1.7976931348623157e308]","Wardley force coordinates must be between 0 and 1"),
] {let source=format!("wardley-beta\n\n{keyword} {tail}\n");let e=parse_wardley(&source).unwrap_err();assert_eq!((e.message.as_str(),e.line,e.col),(want,3,1),"{source}");eprintln!("WARDLEY-FORCE-SOURCE-ERROR {keyword}/{tail}");}}
}
