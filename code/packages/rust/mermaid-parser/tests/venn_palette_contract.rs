//! Each slot is selected at the reviewed Venn source boundary, not by a whole-source search.
use diagram_ir::VennConfig;
use mermaid_parser::parse_venn;

fn defaults() -> Vec<String> {
    ["#60a5fa", "#f59e0b", "#34d399", "#f472b6", "#a78bfa", "#22d3ee", "#fb7185", "#a3e635"]
        .into_iter().map(str::to_string).collect()
}
fn config(prefix: &str) -> VennConfig {
    parse_venn(&format!("{prefix}\nvenn-beta\nset A")).unwrap().config
}
fn expected(colors: Vec<String>) -> VennConfig {
    VennConfig { width:800.0,height:450.0,padding:8.0,use_max_width:true,use_debug_layout:false,
        font_family:"sans-serif".into(),title_text_color:"#0f172a".into(),set_text_color:"#0f172a".into(),theme_colors:colors }
}

#[test]
fn venn_palette_complete_defaults_all_slots_and_per_slot_fallback() {
    assert_eq!(config(""),expected(defaults()));
    let custom:Vec<_>=["#010101","#020202","#030303","#040404","#050505","#060606","#070707","#080808"]
        .into_iter().map(str::to_string).collect();
    for quote in ['\'', '"'] {
        let fields=custom.iter().enumerate().map(|(i,v)|format!("{quote}venn{}{quote}:{quote}{v}{quote}",i+1)).collect::<Vec<_>>().join(",");
        assert_eq!(config(&format!("%%{{init: {{{quote}themeVariables{quote}:{{{fields}}}}}}}%%")),expected(custom.clone()));
    }
    let fields=custom.iter().enumerate().map(|(i,v)|format!("  venn{}: '{v}'",i+1)).collect::<Vec<_>>().join("\n");
    assert_eq!(config(&format!("---\nthemeVariables:\n{fields}\n---")),expected(custom));
    for slot in 0..8 {
        for invalid in [false,true] {
            let mut colors=defaults(); if !invalid {colors[slot]="#112233".into();}
            let value=if invalid {"url(evil)"} else {"#112233"};
            assert_eq!(config(&format!("%%{{init: {{'themeVariables':{{'venn{}':'{value}','venn9':'red'}}}}}}%%",slot+1)),expected(colors));
        }
    }
}

#[test]
fn venn_palette_each_slot_excludes_header_nested_sequence_and_quote_decoys() {
    for slot in 1..=8 {
        let key=format!("venn{slot}");
        for prefix in [
            format!("---\nnote: |\n  %%{{init: {{'themeVariables':{{'{key}':'red'}}}}}}%%\n---"),
            format!("%%{{init: {{'other':{{'themeVariables':{{'{key}':'red'}}}}}}}}%%"),
            format!("%%{{init: {{'themeVariables':{{'other':{{'{key}':'red'}}}}}}}}%%"),
            format!("%%{{init: {{'themeVariables':{{'other':[{{'{key}':'red'}}]}}}}}}%%"),
            format!("---\nthemeVariables:\n  other:\n    {key}: red\n---"),
            format!("---\nthemeVariables:\n  -\n    {key}: red\n---"),
            format!("---\nother:\n  themeVariables:\n    {key}: red\n---"),
            format!("%%{{init: {{\"note\":\"\\\"themeVariables\\\":{{\\\"{key}\\\":\\\"red\\\"}}\"}}}}%%"),
        ] {assert_eq!(config(&prefix),expected(defaults()),"{prefix}");}
        for marker in ["|",">","false","'text'"] {
            let prefix=format!("---\nthemeVariables: {marker}\n  {key}: red\n---");
            assert_eq!(config(&prefix),expected(defaults()),"{prefix}");
        }
        let mut colors=defaults();colors[slot-1]="#112233".into();
        for members in [format!("'other':{{'{key}':'red'}},'{key}':'#112233'"),format!("'{key}':'#112233','other':{{'{key}':'red'}}")] {
            assert_eq!(config(&format!("%%{{init: {{'themeVariables':{{{members}}}}}}}%%")),expected(colors.clone()));
        }
    }
}

#[test]
fn venn_palette_present_invalid_or_empty_directive_never_inherits_front_matter() {
    for slot in 1..=8 {
        let front=format!("---\nthemeVariables:\n  venn{slot}: red\n---");
        for fields in [String::new(),format!("'venn{slot}':'url(evil)'"),format!("'venn{slot}':'#112233'junk"),format!("'venn{slot}':'bad\\qColor'"),"'venn9':'red'".into()] {
            assert_eq!(config(&format!("{front}\n%%{{init: {{'themeVariables':{{{fields}}}}}}}%%")),expected(defaults()));
        }
    }
}
