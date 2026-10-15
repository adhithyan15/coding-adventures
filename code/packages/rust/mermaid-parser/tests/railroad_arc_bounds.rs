//! The inherited numeric parser already admits the new field through its cap.
use mermaid_parser::parse_railroad;
use diagram_ir::RailroadConfig;
const BODIES:[&str;4]=["railroad-beta\nr = terminal(\"A\");","railroad-ebnf-beta\nr = 'A';",
    "railroad-abnf-beta\nr = \"A\";","railroad-peg-beta\nr <- \"A\";"];
fn source(body:&str,value:&str,yaml:bool)->String {
    if yaml {format!("---\nconfig:\n  railroad:\n    arcRadius: {value}\n---\n{body}")}
    else {format!("%%{{init: {{\"railroad\": {{\"arcRadius\": {value}}}}}}}%%\n{body}")}
}
#[test]
fn railroad_arc_parser_retains_complete_defaults_and_every_valid_boundary() {
    let mut cells=0;
    for body in BODIES {
        assert_eq!(parse_railroad(body).unwrap().config,RailroadConfig::default());
        for yaml in [false,true] { for radius in [0.0,1e-300,0.5,4.0,10.0,24.0,65536.0] {
            let parsed=parse_railroad(&source(body,&radius.to_string(),yaml)).unwrap();
            let expected=RailroadConfig {arc_radius:radius,..Default::default()};
            assert_eq!(parsed.config,expected);cells+=1;
        }}
    }
    assert_eq!(cells,56);
}
#[test]
fn railroad_arc_parser_has_exact_errors_for_all_source_forms() {
    let mut cells=0;
    for body in BODIES {for yaml in [false,true] {for value in ["NaN","inf","-inf","-1","65537","1e20","1e308"] {
        let result=parse_railroad(&source(body,value,yaml));
        assert!(matches!(result,Err(e) if e.message=="Mermaid Railroad arcRadius must be finite and in 0..=65536"&&e.line==1&&e.col==1));
        cells+=1;
    }}}
    assert_eq!(cells,56);
}
