//! Source configuration admission is independent of the Venn semantic body.
use diagram_ir::VennConfig;
use mermaid_parser::parse_venn;

fn config(prefix: &str) -> VennConfig {
    parse_venn(&format!("{prefix}\nvenn-beta\nset A")).unwrap().config
}

#[test]
fn venn_source_defaults_and_exact_whole_object_precedence() {
    assert_eq!(config(""), VennConfig::default());
    let front="---\nconfig:\n  venn:\n    width: 640\n    height: 360\n    padding: 24\n    useMaxWidth: false\n    useDebugLayout: true\n---";
    assert_eq!(config(front), VennConfig { width:640.0,height:360.0,padding:24.0,use_max_width:false,use_debug_layout:true });
    for object in [r#"{"venn":{"width":720,"height":420,"padding":30,"useMaxWidth":false,"useDebugLayout":true}}"#,
        "{'venn':{'width':720,'height':420,'padding':30,'useMaxWidth':false,'useDebugLayout':true}}"] {
        assert_eq!(config(&format!("%%{{init: {object}}}%%")), VennConfig {
            width:720.0,height:420.0,padding:30.0,use_max_width:false,use_debug_layout:true });
    }
    // A selected object replaces the entire front-matter object; absent or
    // invalid fields default instead of inheriting the front matter's values.
    assert_eq!(config(&format!("{front}\n%%{{init: {{'venn':{{'width':900,'height':0,'useMaxWidth':'invalid'}}}}}}%%")),
        VennConfig {width:900.0,..Default::default()});
    assert_eq!(config("%%{init: {'venn':{'width':640}}}%%\n%%{init: {'venn':{'width':720}}}%%").width,640.0);
}

#[test]
fn venn_source_numeric_admission_defaults_each_invalid_field() {
    for bad in ["NaN","inf","-inf","-1","65537","1e308"] {
        for field in ["width","height","padding"] {
            let prefix=format!("%%{{init: {{'venn':{{'{field}':{bad}}}}}}}%%");
            assert_eq!(config(&prefix), VennConfig::default(), "{field}={bad}");
        }
    }
    assert_eq!(config("%%{init: {'venn':{'width':0,'height':0,'padding':0}}}%%"),
        VennConfig {padding:0.0,..Default::default()});
    assert_eq!(config("%%{init: {'venn':{'width':65536,'height':65536,'padding':65536}}}%%"),
        VennConfig {width:65536.0,height:65536.0,padding:65536.0,..Default::default()});
}

#[test]
fn venn_configuration_requires_a_real_init_object_key() {
    for prefix in [
        r#"%%{init: {"theme":"venn","other":{"width":640}}}%%"#,
        r#"%%{init: {"note":"\"venn\":{\"width\":640}"}}%%"#,
        "%%{other: {'venn':{'width':640}}}%%",
        "%%{init: {'venn':{'width':640}}}",
        "%% init: {'venn':{'width':640}}",
    ] { assert_eq!(config(prefix),VennConfig::default(),"{prefix}"); }
    let d=parse_venn("venn-beta\nset \"venn\"\nstyle \"venn\" fill:{\"width\":640}").unwrap();
    assert_eq!(d.config,VennConfig::default());
    assert_eq!(d.regions[0].style.fill.as_deref(),Some("{\"width\":640}"));
    assert_eq!(config(r#"%%{ init : {"note":"venn","other":{},"venn":{"width":640}} }%%"#).width,640.0);
}

#[test]
fn venn_huge_source_union_uses_default_scalars() {
    let d=parse_venn("%%{init: {'venn':{'width':1e308,'height':1e308,'padding':0}}}%%\nvenn-beta\nset A\nset B\nset C\nset D\nunion A,B,C,D[\"Four\"]:1").unwrap();
    assert_eq!(d.config,VennConfig {padding:0.0,..Default::default()});
    assert_eq!(d.regions.last().unwrap().sets,vec!["A","B","C","D"]);
}

#[test]
fn venn_nested_keys_and_quoted_field_names_are_decoys() {
    for prefix in [
        "%%{init: {'other':{'venn':{'width':640}}}}%%",
        "%%{init: {'other':[{'venn':{'width':640}}]}}%%",
        "%%{init: {'venn':{'other':{'width':640}}}}%%",
        "%%{init: {'venn':{'other':[{'width':640}]}}}%%",
        r#"%%{init: {"venn":{"note":"width","other":640}}}%%"#,
    ] { assert_eq!(config(prefix),VennConfig::default(),"{prefix}"); }
    for prefix in [
        "%%{init: {'other':{'venn':{'width':640}},'venn':{'width':720}}}%%",
        "%%{init: {'venn':{'other':{'width':640},'width':720}}}%%",
        "%%{init: {'venn':{'other':[{'width':640}],'width':720}}}%%",
    ] { assert_eq!(config(prefix),VennConfig {width:720.0,..Default::default()},"{prefix}"); }
    for prefix in [
        "%%{init: {'venn':{'width':640}}%%", // missing the init object's close
        "%%{init: {'venn':{'width':640}} trailing}%%",
    ] { assert_eq!(config(prefix),VennConfig::default(),"{prefix}"); }
}

#[test]
fn venn_front_matter_preserves_direct_parent_indentation() {
    for nested in ["    other:\n      width: 640", "    other:\n      - width: 640",
        "    other:\n      - nested:\n          width: 640"] {
        let front=format!("---\nconfig:\n  venn:\n{nested}\n    height: 360\n---");
        assert_eq!(config(&front),VennConfig {height:360.0,..Default::default()},"{front}");
        let front=format!("---\nconfig:\n  venn:\n{nested}\n    width: 720\n    height: 360\n---");
        assert_eq!(config(&front),VennConfig {width:720.0,height:360.0,..Default::default()},"{front}");
    }
    for front in ["---\nconfig:\n  other:\n    venn:\n      width: 640\n---",
        "---\nother:\n  config:\n    venn:\n      width: 640\n---"] {
        assert_eq!(config(front),VennConfig::default(),"{front}");
    }
}

#[test]
fn venn_front_matter_target_sections_must_be_mappings() {
    for marker in ["|", ">", "|-", ">+", "false", "'text'"] {
        for front in [format!("---\nconfig:\n  venn: {marker}\n    width: 640\n---"),
            format!("---\nconfig: {marker}\n  venn:\n    width: 640\n---")] {
            assert_eq!(config(&front),VennConfig::default(),"{front}");
        }
    }
}
