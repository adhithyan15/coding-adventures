//! Theme defaults must cross the same selected numeric boundary as explicit style.
use diagram_ir::RailroadConfig;
use mermaid_parser::parse_railroad;
const BODIES:[&str;4]=["railroad-beta\nr = terminal(\"A\");","railroad-ebnf-beta\nr = 'A';",
    "railroad-abnf-beta\nr = \"A\";","railroad-peg-beta\nr <- \"A\";"];
fn source(body:&str,theme:&[(&str,&str)],explicit:&[(&str,&str)],form:usize)->String {
    let json=|fields:&[(&str,&str)]|fields.iter().map(|(k,v)|format!("\"{k}\":\"{v}\"")).collect::<Vec<_>>().join(",");
    let yaml=|fields:&[(&str,&str)],indent:&str|fields.iter().map(|(k,v)|format!("{indent}{k}: '{v}'\n")).collect::<String>();
    match form {
        0=>format!("%%{{init: {{\"themeVariables\": {{{}}}, \"railroad\": {{{}}}}}}}%%\n{body}",json(theme),json(explicit)),
        1=>format!("---\nthemeVariables:\n{}config:\n  railroad:\n{}---\n{body}",yaml(theme,"  "),yaml(explicit,"    ")),
        _=>format!("---\nconfig:\n  themeVariables:\n{}  railroad:\n{}---\n{body}",yaml(theme,"    "),yaml(explicit,"    ")),
    }
}
const BOUND_ERROR:&str="Mermaid Railroad fontSize must be finite and in 0..=65536";
#[test]
fn railroad_theme_selected_font_refuses_exact_excessive_finite_values() {
    let mut failures=Vec::new();let mut cells=0;
    for body in BODIES {for form in 0..3 {for font in ["65537","65537px","1e20","1e20px","1e308","1e308px"] {
        let result=parse_railroad(&source(body,&[("fontSize",font)],&[],form));
        if !matches!(&result,Err(e) if e.message==BOUND_ERROR&&e.line==1&&e.col==1) {failures.push(format!("form={form},font={font},body={body}: {result:?}"));}
        cells+=1;
    }}}
    assert_eq!(cells,72);assert!(failures.is_empty(),"{} selected theme cap refusals failed: {failures:?}",failures.len());
}
#[test]
fn railroad_theme_valid_fonts_and_invalid_theme_fallback_keep_whole_config() {
    let mut valid=0;let mut fallback=0;
    for body in BODIES {for form in 0..3 {
        for size in [0.5,14.0,17.25,64.0,65536.0] {for suffix in ["","px"] {
            let font=format!("{size}{suffix}");let diagram=parse_railroad(&source(body,&[("fontSize",&font)],&[],form)).unwrap();
            assert_eq!(diagram.config,RailroadConfig{font_size:size,..Default::default()});valid+=1;
        }}
        for font in ["garbage","0","0px","-1","-1px","NaN","inf","-inf"] {
            let diagram=parse_railroad(&source(body,&[("fontSize",font)],&[],form)).unwrap();
            assert_eq!(diagram.config,RailroadConfig::default());fallback+=1;
        }
    }}
    assert_eq!((valid,fallback),(120,96));
}
#[test]
fn railroad_theme_font_precedence_validates_only_the_selected_explicit_value() {
    let mut cells=0;
    for body in BODIES {for form in 0..3 {for theme in ["65537px","1e308px"] {
        for explicit in ["17","65537","NaN","0"] {
            let result=parse_railroad(&source(body,&[("fontSize",theme)],&[("fontSize",explicit)],form));
            if explicit=="17" {assert_eq!(result.unwrap().config,RailroadConfig{font_size:17.0,..Default::default()});}
            else {let error=result.unwrap_err();assert_eq!((error.message.as_str(),error.line,error.col),(if explicit=="0"{"Mermaid Railroad fontSize must be a positive number"}else{BOUND_ERROR},1,1));}
            cells+=1;
        }
    }}}
    assert_eq!(cells,96);
}
#[test]
fn railroad_theme_primary_secondary_and_explicit_styles_keep_complete_field_lineage() {
    let primary=[("fontFamily","Courier New"),("fontSize","17px"),("secondBkg","#010203"),("secondaryBorderColor","#040506"),("secondaryTextColor","#070809"),("mainBkg","#101112"),("primaryBorderColor","#131415"),("primaryTextColor","#161718"),("lineColor","#192021"),("tertiaryColor","#222324"),("tertiaryBorderColor","#252627"),("titleColor","#282930")];
    let secondary=[("fontFamily","Courier New"),("fontSize","17px"),("secondaryColor","#aabbcc"),("lineColor","#112233"),("textColor","#445566"),("background","#778899")];
    let explicit=[("fontFamily","Override"),("fontSize","19"),("terminalFill","#a10001"),("terminalStroke","#a10002"),("terminalTextColor","#a10003"),("nonTerminalFill","#a10004"),("nonTerminalStroke","#a10005"),("nonTerminalTextColor","#a10006"),("lineColor","#a10007"),("markerFill","#a10008"),("specialFill","#a10009"),("specialStroke","#a1000a"),("ruleNameColor","#a1000b")];
    let mut cells=0;
    for body in BODIES {for form in 0..3 {
        let p=RailroadConfig{font_family:"Courier New".into(),font_size:17.0,terminal_fill:"#010203".into(),terminal_stroke:"#040506".into(),terminal_text_color:"#070809".into(),non_terminal_fill:"#101112".into(),non_terminal_stroke:"#131415".into(),non_terminal_text_color:"#161718".into(),line_color:"#192021".into(),marker_fill:"#192021".into(),special_fill:"#222324".into(),special_stroke:"#252627".into(),rule_name_color:"#282930".into(),..Default::default()};
        assert_eq!(parse_railroad(&source(body,&primary,&[],form)).unwrap().config,p);cells+=1;
        let s=RailroadConfig{font_family:"Courier New".into(),font_size:17.0,terminal_fill:"#aabbcc".into(),terminal_stroke:"#112233".into(),terminal_text_color:"#445566".into(),non_terminal_fill:"#778899".into(),non_terminal_stroke:"#112233".into(),non_terminal_text_color:"#445566".into(),line_color:"#112233".into(),marker_fill:"#112233".into(),special_fill:"#aabbcc".into(),rule_name_color:"#445566".into(),..Default::default()};
        assert_eq!(parse_railroad(&source(body,&secondary,&[],form)).unwrap().config,s);cells+=1;
        let mut line_override=s;line_override.line_color="#334455".into();
        assert_eq!(parse_railroad(&source(body,&secondary,&[("lineColor","#334455")],form)).unwrap().config,line_override);cells+=1;
        let e=RailroadConfig{font_family:"Override".into(),font_size:19.0,terminal_fill:"#a10001".into(),terminal_stroke:"#a10002".into(),terminal_text_color:"#a10003".into(),non_terminal_fill:"#a10004".into(),non_terminal_stroke:"#a10005".into(),non_terminal_text_color:"#a10006".into(),line_color:"#a10007".into(),marker_fill:"#a10008".into(),special_fill:"#a10009".into(),special_stroke:"#a1000a".into(),rule_name_color:"#a1000b".into(),..Default::default()};
        assert_eq!(parse_railroad(&source(body,&primary,&explicit,form)).unwrap().config,e);cells+=1;
    }}
    assert_eq!(cells,48);
}
