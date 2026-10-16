//! Source controls for every newly discovered incoming-field safety boundary.
use mermaid_parser::{parse_ishikawa, parse_wardley};

#[test]
fn signed_label_offsets_are_bounded_without_losing_supported_endpoints() {
    for offset in ["NaN", "inf", "-inf", "65537", "-65537", "1e308", "0.5"] {
        for axis in [0, 1] {
            let pair = if axis == 0 { format!("{offset},0") } else { format!("0,{offset}") };
            let source = format!("wardley-beta\ncomponent A [0.5,0.5] label [{pair}]\n");
            assert!(parse_wardley(&source).is_err(), "{source}");
        }
    }
    for offset in [-65536, -1, 0, 1, 65536] {
        let diagram = parse_wardley(&format!("wardley-beta\ncomponent A [0.5,0.5] label [{offset},{offset}]\n")).unwrap();
        assert_eq!((diagram.nodes[0].label_offset_x, diagram.nodes[0].label_offset_y), (Some(f64::from(offset)), Some(f64::from(offset))));
    }
}

#[test]
fn link_endpoints_decode_the_same_active_quotes_as_components() {
    for (authored, decoded) in [
        ("'A'", "A"),
        (r#""α \" -> ; +> still label""#, "α \" -> ; +> still label"),
        (r"'α \' -> ; +> still label'", "α ' -> ; +> still label"),
        (r#""α \\ literal""#, r"α \ literal"),
    ] {
        for operator in ["->", "-.->", "+>", "+<>"] {
            let diagram=parse_wardley(&format!("wardley-beta\ncomponent {authored} [0.5,0.2]\ncomponent B [0.5,0.8]\n{authored} {operator} B\nB {operator} {authored}\n")).unwrap();
            assert_eq!(diagram.nodes[0].label, decoded);
            assert_eq!((diagram.links[0].source.as_str(), diagram.links[0].target.as_str()),(decoded,"B"));
            assert_eq!((diagram.links[1].source.as_str(), diagram.links[1].target.as_str()),("B",decoded));
        }
    }
}

#[test]
fn percent_pairs_belong_to_quoted_nodes_endpoints_and_flow_labels() {
    let diagram=parse_wardley("wardley-beta\ncomponent 'A%%B' [0.5,0.2]\ncomponent C [0.5,0.8]\n'A%%B' +'demand %% signal'> C %% outside\n").unwrap();
    assert_eq!(diagram.nodes[0].label,"A%%B");
    assert_eq!((diagram.links[0].source.as_str(),diagram.links[0].target.as_str(),diagram.links[0].label.as_deref()),("A%%B","C",Some("demand %% signal")));
}

#[test]
fn no_space_link_operators_preserve_quoted_target_comments() {
    for quote in ['\'', '"'] {
        for operator in ["->","-->","-.->","+>","+<","+<>"] {
            let source=format!("wardley-beta\ncomponent A [0.5,0.2]\ncomponent {quote}B%%C{quote} [0.5,0.8]\nA{operator}{quote}B%%C{quote} %% outside\n");
            let diagram=parse_wardley(&source).unwrap_or_else(|e|panic!("{source}: {e:?}"));
            assert_eq!(diagram.nodes[1].label,"B%%C");
            assert_eq!((diagram.links[0].source.as_str(),diagram.links[0].target.as_str()),("A","B%%C"));
            assert_eq!(diagram.links[0].dashed,operator=="-.->");
        }
    }
    // An interior apostrophe in a bare word does not capture the outside comment.
    let diagram=parse_wardley("wardley-beta\ncomponent O'Reilly [0.5,0.2] %% outside\npipeline 'O\\'Reilly' {\ncomponent C [0.25]\n}\n").unwrap();
    assert_eq!(diagram.nodes[0].label,"O'Reilly");
    assert_eq!(diagram.pipelines[0].component_ids,[diagram.nodes[1].id.clone()]);
}

#[test]
fn legacy_bare_brackets_before_label_word_are_not_coordinate_suffixes() {
    for label in ["A[X] label","A[0,0] label"] {
        let diagram=parse_wardley(&format!("wardley-beta\ncomponent {label} [0.5,0.2]\npipeline '{label}' {{\ncomponent C [0.25]\n}}\n")).unwrap();
        assert_eq!(diagram.nodes[0].label,label);
        assert_eq!((diagram.nodes[0].visibility,diagram.nodes[0].evolution),(0.5,0.2));
        assert!(diagram.nodes[0].pipeline_parent);
        assert_eq!(diagram.pipelines[0].parent_id,diagram.nodes[0].id);
        assert_eq!(diagram.pipelines[0].component_ids,[diagram.nodes[1].id.clone()]);
    }
}

#[test]
fn distinct_init_directives_preserve_per_key_selection() {
    let diagram=parse_ishikawa("%%{init: {\"fontSize\":22}}%%\n%%{init: {\"themeVariables\":{\"fontFamily\":\"Georgia\"}}}%%\nishikawa-beta\nE\n  A\n").unwrap();
    assert_eq!(diagram.config.font_size,22.0);
    assert_eq!(diagram.config.font_family,"Georgia");
}

#[test]
fn multiline_internal_references_cannot_capture_authored_description_text() {
    for literal in ["__WARDLEY_MULTILINE_0", "__WARDLEY_MULTILINE_1"] {
        let diagram=parse_wardley(&format!("wardley-beta\naccDescr {{\n first\n}}\naccDescr: {literal}\n")).unwrap();
        assert_eq!(diagram.accessibility_description.as_deref(),Some(literal));
    }
    let error=parse_wardley("wardley-beta\naccDescr {\n first\n second\n}\nα\n").unwrap_err();
    assert_eq!((error.line,error.col,error.message.as_str()),(6,1,"unsupported Wardley statement: α"));
}

#[test]
fn incoming_ishikawa_font_size_has_a_finite_source_domain() {
    for (value, expected) in [("1",1.0),("22",22.0),("65536",65536.0),("65537",16.0),("1e308",16.0),("0.0001",16.0),("0",16.0),("-1",16.0)] {
        let source=format!("%%{{init: {{\"fontSize\":{value}}}}}%%\nishikawa-beta\nE\n  A\n");
        assert_eq!(parse_ishikawa(&source).unwrap().config.font_size,expected,"{source}");
    }
}
