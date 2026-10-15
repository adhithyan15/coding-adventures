//! Compact mode is typed configuration and metadata, with independent marker state.
use diagram_ir::RailroadConfig;
use mermaid_parser::parse_railroad;
const BODIES:[&str;4]=["railroad-beta\nr = terminal(\"A\");","railroad-ebnf-beta\nr = 'A';",
    "railroad-abnf-beta\nr = \"A\";","railroad-peg-beta\nr <- \"A\";"];
fn source(body:&str,fields:&[(&str,&str)],form:usize)->String {
    let json=fields.iter().map(|(k,v)|format!("\"{k}\":\"{v}\"")).collect::<Vec<_>>().join(",");
    let yaml=|indent:&str|fields.iter().map(|(k,v)|format!("{indent}{k}: '{v}'\n")).collect::<String>();
    match form {
        0=>format!("%%{{init: {{\"railroad\": {{{json}}}}}}}%%\n{body}"),
        1=>format!("---\nconfig:\n  railroad:\n{}---\n{body}",yaml("    ")),
        _=>format!("---\nconfig:\n  themeVariables:\n    fontFamily: monospace\n  railroad:\n{}---\n{body}",yaml("    ")),
    }
}
#[test]
fn railroad_compact_absent_empty_casefolded_and_bare_booleans_keep_whole_config() {
    let mut cells=0;
    for body in BODIES {for form in 0..3 {
        for (mode,compact) in [(None,false),(Some(""),false),(Some("true"),true),(Some("TRUE"),true),(Some("false"),false),(Some("False"),false)] {
            let markers=if compact{"false"}else{"true"};
            let mut fields=vec![("padding","0.5"),("fontSize","17"),("arcRadius","12.5"),("lineColor","#010203"),("showMarkers",markers)];
            if let Some(value)=mode {fields.push(("compactMode",value));}
            let parsed=parse_railroad(&source(body,&fields,form)).unwrap();
            assert_eq!(parsed.config,RailroadConfig{compact_mode:compact,padding:0.5,font_size:17.0,arc_radius:12.5,line_color:"#010203".into(),show_markers:!compact,..Default::default()});cells+=1;
        }
        for (value,compact) in [("true",true),("false",false)] {
            let raw=source(body,&[("compactMode",value)],form)
                .replace(&format!("\"compactMode\":\"{value}\""),&format!("\"compactMode\":{value}"))
                .replace(&format!("compactMode: '{value}'"),&format!("compactMode: {value}"));
            assert_eq!(parse_railroad(&raw).unwrap().config,RailroadConfig{compact_mode:compact,..Default::default()});cells+=1;
        }
    }}
    assert_eq!(cells,96);
}
#[test]
fn railroad_compact_malformed_present_values_have_exact_typed_errors() {
    let mut cells=0;
    for body in BODIES {for form in 0..3 {for value in ["sometimes","1","0","null","NaN","yes","truefalse"] {
        let error=parse_railroad(&source(body,&[("compactMode",value)],form)).unwrap_err();
        assert_eq!((error.message.as_str(),error.line,error.col),("Mermaid Railroad compactMode must be true or false",1,1));cells+=1;
    }}}
    assert_eq!(cells,84);
}
#[test]
fn railroad_compact_directive_selects_the_whole_object_before_front_matter() {
    let mut cells=0;
    for body in BODIES {for (fields,compact) in [(vec![("compactMode","true"),("showMarkers","false")],true),(vec![("compactMode","false"),("showMarkers","false")],false),(vec![("showMarkers","false")],false)] {
        let directive=source(body,&fields,0);
        let input=format!("---\nconfig:\n  railroad:\n    compactMode: true\n    padding: 99\n    showMarkers: true\n---\n{directive}");
        assert_eq!(parse_railroad(&input).unwrap().config,RailroadConfig{compact_mode:compact,show_markers:false,..Default::default()});cells+=1;
    }}
    assert_eq!(cells,12);
}
