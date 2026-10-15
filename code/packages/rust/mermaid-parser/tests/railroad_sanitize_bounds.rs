//! Whole configuration lineage under the incoming lexical string filters.
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
const THEME:&[(&str,&str)]=&[("fontFamily","Courier New"),("fontSize","17px"),("secondBkg","#010203"),("secondaryBorderColor","#040506"),("secondaryTextColor","#070809"),("mainBkg","#101112"),("primaryBorderColor","#131415"),("primaryTextColor","#161718"),("lineColor","#192021"),("tertiaryColor","#222324"),("tertiaryBorderColor","#252627"),("titleColor","#282930")];
fn expected()->RailroadConfig { RailroadConfig{font_family:"Courier New".into(),font_size:17.0,terminal_fill:"#010203".into(),terminal_stroke:"#040506".into(),terminal_text_color:"#070809".into(),non_terminal_fill:"#101112".into(),non_terminal_stroke:"#131415".into(),non_terminal_text_color:"#161718".into(),line_color:"#192021".into(),marker_fill:"#192021".into(),special_fill:"#222324".into(),special_stroke:"#252627".into(),rule_name_color:"#282930".into(),..Default::default()} }

const FIELDS:[&str;11]=["terminalFill","terminalStroke","terminalTextColor","nonTerminalFill","nonTerminalStroke","nonTerminalTextColor","lineColor","markerFill","specialFill","specialStroke","ruleNameColor"];
fn set_color(config:&mut RailroadConfig,index:usize,value:&str) {
    let slot=match index {0=>&mut config.terminal_fill,1=>&mut config.terminal_stroke,2=>&mut config.terminal_text_color,
        3=>&mut config.non_terminal_fill,4=>&mut config.non_terminal_stroke,5=>&mut config.non_terminal_text_color,
        6=>&mut config.line_color,7=>&mut config.marker_fill,8=>&mut config.special_fill,9=>&mut config.special_stroke,_=>&mut config.rule_name_color};
    *slot=value.to_string();
}
#[test]
fn railroad_sanitize_color_domains_preserve_complete_theme_and_explicit_lineage() {
    // These are the producer's lexical allowlist, not a full CSS grammar.
    let valid=["#abc","#abcd","#A1B2C3","#A1B2C3D4"," Red "," RGB(1, 2, 3) ","rgba(1,2,3,.5)","hsl(120 50% 50%)","hsla(120,50%,50%,.5)","hwb(0 0% 0%)","lab(50% 1 2)","lch(50% 2 3)","oklab(.5 .1 .2)","oklch(.5 .1 2)","rgb( )","rgb(--//)"];
    let invalid=["url(x)","#12zzzz","#12345","rgb()","rgb(1;2;3)","var(--x)","red;display:none","<red>","rgb(1e3,2,3)"];
    let mut accepted=0;let mut fallback=0;
    for body in BODIES {for form in 0..3 {for (index,field) in FIELDS.iter().enumerate() {
        for value in valid {
            let diagram=parse_railroad(&source(body,THEME,&[(field,value)],form)).unwrap();let mut want=expected();set_color(&mut want,index,value.trim());
            assert_eq!(diagram.config,want,"accepted field={field},value={value},form={form}");accepted+=1;
        }
        for value in invalid {
            let diagram=parse_railroad(&source(body,THEME,&[(field,value)],form)).unwrap();
            assert_eq!(diagram.config,expected(),"fallback field={field},value={value},form={form}");fallback+=1;
        }
    }}}
    assert_eq!((accepted,fallback),(2112,1188));
}
#[test]
fn railroad_sanitize_font_family_preserves_whole_config_and_theme_fallback() {
    let mut cells=0;
    for body in BODIES {for form in 0..3 {
        for value in ["Courier New","Georgia, serif","Noto-Sans_123","A.B"," monospace "] {
            let diagram=parse_railroad(&source(body,THEME,&[("fontFamily",value)],form)).unwrap();let mut want=expected();want.font_family=value.trim().into();assert_eq!(diagram.config,want);cells+=1;
        }
        for value in ["Georgia; color:red","../bad/font","Font@Name","Font_Name!","url(x)"] {
            assert_eq!(parse_railroad(&source(body,THEME,&[("fontFamily",value)],form)).unwrap().config,expected());cells+=1;
        }
    }}
    assert_eq!(cells,120);
}
#[test]
fn railroad_sanitize_invalid_primary_theme_uses_field_default_before_secondary() {
    let mut theme=THEME.iter().map(|(name,_)|(*name,if *name=="fontSize"{"17px"}else{"invalid;value"})).collect::<Vec<_>>();
    theme.push(("secondaryColor","#abcdef"));theme.push(("background","#abcdef"));theme.push(("textColor","#abcdef"));
    let mut cells=0;
    for body in BODIES {for form in 0..3 {
        let diagram=parse_railroad(&source(body,&theme,&[],form)).unwrap();
        assert_eq!(diagram.config,RailroadConfig{font_size:17.0,..Default::default()});cells+=1;
    }}
    assert_eq!(cells,12);
}
