//! Venn theme selection shares the same source boundary as canvas selection.
use diagram_ir::VennConfig;
use mermaid_parser::parse_venn;

fn baseline() -> VennConfig {
    VennConfig { width:800.0,height:450.0,padding:8.0,use_max_width:true,use_debug_layout:false,
        font_family:"sans-serif".into(),title_text_color:"#0f172a".into(),set_text_color:"#0f172a".into(),theme_colors:["#60a5fa","#f59e0b","#34d399","#f472b6","#a78bfa","#22d3ee","#fb7185","#a3e635"].into_iter().map(str::to_string).collect() }
}
fn config(prefix: &str) -> VennConfig {
    parse_venn(&format!("{prefix}\nvenn-beta\nset A")).unwrap().config
}
fn theme(family: &str,title: &str,set: &str) -> VennConfig {
    VennConfig {font_family:family.into(),title_text_color:title.into(),set_text_color:set.into(),..baseline()}
}

#[test]
fn venn_theme_defaults_and_complete_generic_specific_precedence() {
    assert_eq!(config(""),baseline());
    for prefix in [
        "---\nthemeVariables:\n  fontFamily: Georgia\n  titleColor: '#112233'\n  textColor: '#445566'\n---",
        "%%{init: {'themeVariables':{'fontFamily':'Georgia','titleColor':'#112233','textColor':'#445566'}}}%%",
        r##"%%{init: {"themeVariables":{"fontFamily":"Georgia","titleColor":"#112233","textColor":"#445566"}}}%%"##,
        "%%{init: {'themeVariables':{'fontFamily':'Georgia','titleColor':'red','textColor':'blue','vennTitleTextColor':'#112233','vennSetTextColor':'#445566'}}}%%",
    ] {assert_eq!(config(prefix),theme("Georgia","#112233","#445566"),"{prefix}");}
    assert_eq!(config("%%{init: {'themeVariables':{'vennTitleTextColor':'bad;css','titleColor':'red','vennSetTextColor':'bad;css','textColor':'blue','fontFamily':'bad;font'}}}%%"),baseline(),"present invalid specific values suppress generic fallback");
}

#[test]
fn venn_theme_directive_replaces_whole_front_matter_object() {
    let front="---\nthemeVariables:\n  fontFamily: Georgia\n  titleColor: '#112233'\n  textColor: '#445566'\nconfig:\n  venn:\n    width: 640\n---";
    assert_eq!(config(&format!("{front}\n%%{{init: {{'themeVariables':{{'fontFamily':'Courier New'}}}}}}%%")),
        VennConfig {width:640.0,..theme("Courier New","#0f172a","#0f172a")});
    assert_eq!(config(&format!("{front}\n%%{{init: {{'themeVariables':{{}}}}}}%%")),VennConfig {width:640.0,..baseline()});
    assert_eq!(config(&format!("{front}\n%%{{init: {{'themeVariables':{{'fontFamily':'bad;font'}}}}}}%%")),VennConfig {width:640.0,..baseline()});
}

#[test]
fn venn_theme_requires_real_post_header_closed_init() {
    for marker in ["|",">","|-"] {
        let front=format!("---\nnote: {marker}\n  %%{{init: {{'themeVariables':{{'fontFamily':'Georgia'}}}}}}%%\n---");
        assert_eq!(config(&front),baseline(),"{front}");
        assert_eq!(config(&format!("{front}\n%%{{init: {{'themeVariables':{{'fontFamily':'Courier New'}}}}}}%%")),theme("Courier New","#0f172a","#0f172a"));
    }
    for prefix in ["%%{other: {'themeVariables':{'fontFamily':'Georgia'}}}%%",
        "%%{init: {'themeVariables':{'fontFamily':'Georgia'}}%%",
        "%%{init: {'themeVariables':{'fontFamily':'Georgia'}} trailing}%%",
        r##"%%{init: {"note":"themeVariables","other":{"fontFamily":"Georgia"}}}%%"##,
        r##"%%{init: {"note":"\"themeVariables\":{\"fontFamily\":\"Georgia\"}"}}%%"##,
    ] {assert_eq!(config(prefix),baseline(),"{prefix}");}
    let error=parse_venn("venn-beta\nset \"themeVariables\"\nstyle \"themeVariables\" fill:{\"fontFamily\":\"Georgia\"}").unwrap_err();
    assert_eq!((error.message.as_str(),error.line,error.col),("invalid Venn fill color",3,1));
    let d=parse_venn("venn-beta\nset \"themeVariables\"[{\"fontFamily\":\"Georgia\"}]\nstyle \"themeVariables\" fill:red").unwrap();
    assert_eq!(d.config,baseline());
    assert_eq!(d.regions[0].label.as_deref(),Some("{\"fontFamily\":\"Georgia\"}"));
    assert_eq!(d.regions[0].style.fill.as_deref(),Some("red"));
}

#[test]
fn venn_theme_direct_members_exclude_nested_objects_arrays_and_values() {
    for prefix in [
        "%%{init: {'other':{'themeVariables':{'fontFamily':'Georgia'}}}}%%",
        "%%{init: {'other':[{'themeVariables':{'fontFamily':'Georgia'}}]}}%%",
        "%%{init: {'themeVariables':{'other':{'fontFamily':'Georgia'}}}}%%",
        "%%{init: {'themeVariables':{'other':[{'fontFamily':'Georgia'}]}}}%%",
        r##"%%{init: {"themeVariables":{"note":"fontFamily","other":"Georgia"}}}%%"##,
    ] {assert_eq!(config(prefix),baseline(),"{prefix}");}
    for prefix in [
        "%%{init: {'other':{'themeVariables':{'fontFamily':'Georgia'}},'themeVariables':{'fontFamily':'Courier New'}}}%%",
        "%%{init: {'themeVariables':{'other':{'fontFamily':'Georgia'},'fontFamily':'Courier New'}}}%%",
        "%%{init: {'themeVariables':{'fontFamily':'Courier New','other':{'fontFamily':'Georgia'}}}}%%",
        "%%{init: {'themeVariables':{'other':[{'fontFamily':'Georgia'}],'fontFamily':'Courier New'}}}%%",
    ] {assert_eq!(config(prefix),theme("Courier New","#0f172a","#0f172a"),"{prefix}");}
}

#[test]
fn venn_theme_front_matter_requires_direct_root_mapping_and_fields() {
    for front in [
        "---\nother:\n  themeVariables:\n    fontFamily: Georgia\n---",
        "---\nconfig:\n  themeVariables:\n    fontFamily: Georgia\n---",
        "---\nthemeVariables:\n  other:\n    fontFamily: Georgia\n---",
        "---\nthemeVariables:\n  other:\n    - fontFamily: Georgia\n---",
        "---\nthemeVariables:\n  -\n    fontFamily: Georgia\n---",
        "---\nother:\n  -\n    themeVariables:\n      fontFamily: Georgia\n---",
    ] {assert_eq!(config(front),baseline(),"{front}");}
    for marker in ["|",">","|-",">+","false","'text'"] {
        let front=format!("---\nthemeVariables: {marker}\n  fontFamily: Georgia\n---");
        assert_eq!(config(&front),baseline(),"{front}");
    }
    let front="---\nthemeVariables:\n  other:\n    fontFamily: Georgia\n  fontFamily: Courier New\n  titleColor: '#112233'\n---";
    assert_eq!(config(front),theme("Courier New","#112233","#0f172a"));
}

#[test]
fn venn_theme_active_quote_and_escape_boundaries_preserve_authored_family() {
    for (prefix,family) in [
        (r##"%%{init: {'themeVariables':{'fontFamily':'Times "New" Roman'}}}%%"##,"Times \"New\" Roman"),
        (r##"%%{init: {"themeVariables":{"fontFamily":"Times 'New' Roman"}}}%%"##,"Times 'New' Roman"),
        (r##"%%{init: {"themeVariables":{"fontFamily":"Times \"New\" Roman"}}}%%"##,"Times \"New\" Roman"),
        (r##"%%{init: {'themeVariables':{'fontFamily':'O\'Brien'}}}%%"##,"O'Brien"),
        ("---\nthemeVariables:\n  fontFamily: '\"Times New Roman\", serif'\n---","\"Times New Roman\", serif"),
        ("---\nthemeVariables:\n  fontFamily: Times New Roman, serif\n---","Times New Roman, serif"),
    ] {assert_eq!(config(prefix),theme(family,"#0f172a","#0f172a"),"{prefix}");}
    for prefix in [
        r##"%%{init: {'themeVariables':{'fontFamily':'Times'junk}}}%%"##,
        r##"%%{init: {'themeVariables':{'fontFamily':'Bad\qFont'}}}%%"##,
        r##"%%{init: {'themeVariables':{'vennTitleTextColor':'#112233'junk,'titleColor':'red'}}}%%"##,
        r##"%%{init: {'themeVariables':{'vennTitleTextColor':'Bad\qColor','titleColor':'red'}}}%%"##,
    ] {assert_eq!(config(prefix),baseline(),"malformed PRESENT fields must default: {prefix}");}
}
