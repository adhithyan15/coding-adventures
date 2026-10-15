//! Portable style scalar admission, independent of Apple-only raster tests.
use mermaid_parser::parse_railroad;
use diagram_ir::RailroadConfig;

const BODIES:[&str;4]=["railroad-beta\nr = terminal(\"A\");","railroad-ebnf-beta\nr = 'A';",
    "railroad-abnf-beta\nr = \"A\";","railroad-peg-beta\nr <- \"A\";"];
fn source(body:&str,field:&str,value:&str,yaml:bool)->String {
    if yaml {format!("---\nconfig:\n  railroad:\n    {field}: {value}\n---\n{body}")}
    else {format!("%%{{init: {{\"railroad\": {{\"{field}\": {value}}}}}}}%%\n{body}")}
}
#[test]
fn railroad_style_refuses_exact_numeric_domains_in_every_source_form() {
    let mut failures=Vec::new();let mut cells=0;
    for body in BODIES {for yaml in [false,true] {for field in ["fontSize","strokeWidth","markerRadius"] {
        for value in ["NaN","inf","-inf","-1","65537","1e20","1e308"] {
            let result=parse_railroad(&source(body,field,value,yaml));
            let expected=format!("Mermaid Railroad {field} must be finite and in 0..=65536");
            if !matches!(&result,Err(e) if e.message==expected&&e.line==1&&e.col==1) {failures.push(format!("{field}={value},yaml={yaml}: {result:?}"));}
            cells+=1;
        }
    }
        let result=parse_railroad(&source(body,"fontSize","0",yaml));
        assert!(matches!(result,Err(e) if e.message=="Mermaid Railroad fontSize must be a positive number"&&e.line==1&&e.col==1));
        cells+=1;
    }}
    assert_eq!(cells,176);assert!(failures.is_empty(),"{} exact style refusals failed: {failures:?}",failures.len());
}
#[test]
fn railroad_style_retains_complete_config_at_accepted_numeric_boundaries() {
    let mut cells=0;
    for body in BODIES {for yaml in [false,true] {for (field,index) in [("fontSize",0),("strokeWidth",1),("markerRadius",2)] {
        for value in [0.5,14.0,18.25,64.0,65536.0] {
            let parsed=parse_railroad(&source(body,field,&value.to_string(),yaml)).unwrap();
            let mut expected=RailroadConfig::default();
            match index {0=>expected.font_size=value,1=>expected.stroke_width=value,_=>expected.marker_radius=value}
            assert_eq!(parsed.config,expected);cells+=1;
        }
    }}}
    assert_eq!(cells,120);
    for field in ["strokeWidth","markerRadius"] {
        assert!(parse_railroad(&source(BODIES[0],field,"0",false)).is_ok());
    }
}
