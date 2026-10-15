use diagram_ir::{IshikawaCause,IshikawaConfig,IshikawaDiagram};

// Each setting case retains the whole causal IR, including identity and order.
// Expected defaults are literal so a production-default change cannot rescue a
// failed source-origin or malformed-present-value control.
fn defaults() -> IshikawaConfig { IshikawaConfig {diagram_padding:20.0,use_max_width:false,
    font_family:"sans-serif".into(),line_color:"#333333".into(),fill_color:"#ffffff".into(),text_color:"#333333".into()} }
fn custom() -> IshikawaConfig { IshikawaConfig {diagram_padding:80.0,use_max_width:true,
    font_family:"Georgia".into(),line_color:"#123456".into(),fill_color:"#abcdef".into(),text_color:"#654321".into()} }
fn complete(prefix:&str,config:IshikawaConfig) {
    let source=[prefix,"ishikawa-beta\nE\n  A\n"].concat();
    assert_eq!(mermaid_parser::parse_ishikawa(&source).unwrap(),IshikawaDiagram {effect:"E".into(),
        causes:vec![IshikawaCause {id:"cause-1".into(),label:"A".into(),parent_id:None,depth:1}],config},"source={source}");
}
fn init(config:&str,theme:&str)->String { ["%%{init: {\"ishikawa\":",config,",\"themeVariables\":",theme,"}}%%\n"].concat() }
const YAML:&str="---\nconfig:\n  ishikawa:\n    diagramPadding: 80\n    useMaxWidth: true\nthemeVariables:\n  fontFamily: Georgia\n  lineColor: '#123456'\n  mainBkg: '#abcdef'\n  textColor: '#654321'\n---\n";

#[test]
fn ishikawa_settings_keep_direct_semantic_origins_and_complete_ir() {
    complete("",defaults());complete(YAML,custom());
    complete(&init(r#"{"diagramPadding":80,"useMaxWidth":true}"#,r##"{"fontFamily":"Georgia","lineColor":"#123456","mainBkg":"#abcdef","textColor":"#654321"}"##),custom());
    for prefix in [
        "%%{init: {\"nested\":{\"ishikawa\":{\"diagramPadding\":80,\"useMaxWidth\":true},\"themeVariables\":{\"fontFamily\":\"Georgia\",\"lineColor\":\"red\"}}}}%%\n",
        "%%{init: {\"ishikawa\":{\"nested\":{\"diagramPadding\":80,\"useMaxWidth\":true}},\"themeVariables\":{\"nested\":{\"fontFamily\":\"Georgia\",\"lineColor\":\"red\",\"mainBkg\":\"blue\",\"textColor\":\"green\"}}}}%%\n",
        "---\nnotes: |\n  %%{init: {\"ishikawa\":{\"diagramPadding\":80,\"useMaxWidth\":true},\"themeVariables\":{\"fontFamily\":\"Georgia\",\"lineColor\":\"red\"}}}%%\n---\n",
        "---\nunrelated:\n  config:\n    ishikawa:\n      diagramPadding: 80\n      useMaxWidth: true\n  themeVariables:\n    fontFamily: Georgia\n    lineColor: red\n---\n",
        "---\nconfig:\n  ishikawa:\n    nested:\n      diagramPadding: 80\n      useMaxWidth: true\nthemeVariables:\n  nested:\n    fontFamily: Georgia\n    lineColor: red\n    mainBkg: blue\n    textColor: green\n---\n",
        "---\nconfig:\n  ishikawa:\n    -\n      diagramPadding: 80\n      useMaxWidth: true\nthemeVariables:\n  -\n    fontFamily: Georgia\n    lineColor: red\n---\n",
        "---\nconfig:\n  ishikawa: |\n    diagramPadding: 80\n    useMaxWidth: true\nthemeVariables: |\n  fontFamily: Georgia\n  lineColor: red\n---\n",
    ] {complete(prefix,defaults());}
    complete(&init(r#"{"nested":{"diagramPadding":1,"useMaxWidth":false},"diagramPadding":80,"useMaxWidth":true}"#,
        r##"{"nested":{"fontFamily":"Wrong","lineColor":"red","mainBkg":"blue","textColor":"green"},"fontFamily":"Georgia","lineColor":"#123456","mainBkg":"#abcdef","textColor":"#654321"}"##),custom());
}

#[test]
fn ishikawa_theme_consumes_active_quotes_and_complete_values() {
    for (value,expected) in [(r#""O'Reilly""#,"O'Reilly"),(r#""A\"B""#,"A\"B"),
        (r#""'Georgia'""#,"'Georgia'"),(r#""Georgia'url(evil)""#,"sans-serif"),
        (r#""Georgia"junk"#,"sans-serif"),(r#""Georgia\q""#,"sans-serif")] {
        let theme=["{\"fontFamily\":",value,"}"].concat();let mut expected_config=defaults();expected_config.font_family=expected.into();
        complete(&init("{}",&theme),expected_config);
    }
    for key in ["lineColor","mainBkg","textColor"] {for value in [r#""red'url(evil)""#,r#""red'evil""#,r#""red"junk"#,r#""red\q""#] {
        complete(&init("{}",&["{\"",key,"\":",value,"}"].concat()),defaults());
    }}
    for (key,expected) in [("lineColor","line"),("mainBkg","fill"),("textColor","text")] {
        let mut config=defaults();match expected {"line"=>config.line_color="#123456".into(),"fill"=>config.fill_color="#123456".into(),_=>config.text_color="#123456".into()}
        complete(&init("{}",&["{\"",key,"\":\"#123456\"}"].concat()),config);
    }
    let mut font=defaults();font.font_family="O'Reilly".into();complete("---\nthemeVariables:\n  fontFamily: \"O'Reilly\"\n---\n",font);
}

#[test]
fn ishikawa_numeric_boolean_admission_keeps_exact_complete_ir() {
    for value in ["-1","NaN","inf","-inf","1e999","true","{}","[]","12x",r#""80"junk"#] {
        complete(&init(&["{\"diagramPadding\":",value,"}"].concat(),"{}"),defaults());
        complete(&format!("---\nconfig:\n  ishikawa:\n    diagramPadding: {value}\n---\n"),defaults());
    }
    for (value,expected) in [("0",0.0),("80",80.0),("1e300",1e300),(r#""80""#,80.0)] {
        let mut config=defaults();config.diagram_padding=expected;complete(&init(&["{\"diagramPadding\":",value,"}"].concat(),"{}"),config);
    }
    for (value,expected) in [("true",true),("false",false),(r#""TRUE""#,true),("1",false),("truthy",false),("null",false),(r#""true"junk"#,false)] {
        let mut config=defaults();config.use_max_width=expected;complete(&init(&["{\"useMaxWidth\":",value,"}"].concat(),"{}"),config);
    }
}

#[test]
fn ishikawa_empty_init_objects_replace_corresponding_frontmatter_wholesale() {
    complete(&[YAML,&init("{}","{}")].concat(),defaults());
    let mut config=custom();config.font_family="sans-serif".into();config.line_color="#333333".into();config.fill_color="#ffffff".into();config.text_color="#333333".into();
    complete(&[YAML,"%%{init: {\"themeVariables\":{}}}%%\n"].concat(),config);
    let mut theme=custom();theme.diagram_padding=20.0;theme.use_max_width=false;
    complete(&[YAML,"%%{init: {\"ishikawa\":{}}}%%\n"].concat(),theme);
}
