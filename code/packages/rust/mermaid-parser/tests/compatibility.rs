use std::collections::BTreeSet;

use mermaid_parser::{
    detect_mermaid_type, parse_any_mermaid, parse_architecture, parse_block, parse_gantt, parse_gitgraph, parse_journey, parse_pie,
    parse_kanban, parse_mindmap, parse_packet, parse_quadrant_chart, parse_requirement_diagram, parse_sankey,
    parse_cynefin, parse_event_modeling, parse_info, parse_ishikawa, parse_radar, parse_railroad, parse_sequence_diagram, parse_swimlane, parse_timeline, parse_treeview, parse_treemap, parse_venn, parse_wardley, parse_xychart, parse_zenuml,
    MERMAID_COMPATIBILITY_BASELINE,
};
use serde_json::Value;

const COMPATIBILITY_MANIFEST: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../grammars/mermaid/compatibility.json"
));
const SWIMLANE_CORPUS: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../grammars/mermaid/swimlane-11.16.1-corpus.json"
));
const SWIMLANE_VISUAL_CORPUS: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../grammars/mermaid/swimlane-11.16.1-visual-corpus.json"
));
const QUADRANT_CORPUS: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../grammars/mermaid/quadrant-11.16.1-corpus.json"
));
const JOURNEY_CORPUS: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../grammars/mermaid/journey-11.16.1-corpus.json"
));
const TIMELINE_CORPUS: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../grammars/mermaid/timeline-11.16.1-corpus.json"
));
const MINDMAP_CORPUS: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../grammars/mermaid/mindmap-11.16.1-corpus.json"
));
const BLOCK_CORPUS: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../grammars/mermaid/block-11.16.1-corpus.json"
));
const PACKET_CORPUS: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../grammars/mermaid/packet-11.16.1-corpus.json"
));
const KANBAN_CORPUS: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../grammars/mermaid/kanban-11.16.1-corpus.json"
));
const ARCHITECTURE_CORPUS: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../grammars/mermaid/architecture-11.16.1-corpus.json"
));
const RADAR_CORPUS: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../grammars/mermaid/radar-11.16.1-corpus.json"
));
const EVENTMODELING_CORPUS: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../grammars/mermaid/eventmodeling-11.16.1-corpus.json"
));
const EVENTMODELING_VISUAL_CORPUS: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../grammars/mermaid/eventmodeling-11.16.1-visual-corpus.json"
));
const TREEMAP_CORPUS: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../grammars/mermaid/treemap-11.16.1-corpus.json"
));
const VENN_CORPUS: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../grammars/mermaid/venn-11.16.1-corpus.json"));
const ISHIKAWA_CORPUS: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../grammars/mermaid/ishikawa-11.16.1-corpus.json"));
const WARDLEY_CORPUS: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../grammars/mermaid/wardley-11.16.1-corpus.json"));
const CYNEFIN_CORPUS: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../grammars/mermaid/cynefin-11.16.1-corpus.json"));
const CYNEFIN_VISUAL_CORPUS: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../grammars/mermaid/cynefin-11.16.1-visual-corpus.json"));
const TREEVIEW_CORPUS: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../grammars/mermaid/treeview-11.16.1-corpus.json"));
const TREEVIEW_VISUAL_CORPUS: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../grammars/mermaid/treeview-11.16.1-visual-corpus.json"));
const RAILROAD_CORPUS: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../grammars/mermaid/railroad-11.16.1-corpus.json"));
const RAILROAD_EBNF_CORPUS: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../grammars/mermaid/railroad-ebnf-11.16.1-corpus.json"));
const RAILROAD_ABNF_CORPUS: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../grammars/mermaid/railroad-abnf-11.16.1-corpus.json"));
const RAILROAD_PEG_CORPUS: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../grammars/mermaid/railroad-peg-11.16.1-corpus.json"));
const INFO_CORPUS: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../grammars/mermaid/info-11.16.1-corpus.json"));
const ZENUML_CORPUS: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../grammars/mermaid/zenuml-11.16.1-corpus.json"));

#[test]
fn pinned_zenuml_subset_corpus_lowers_to_sequence_ir() {
    let corpus: Value = serde_json::from_str(ZENUML_CORPUS).expect("zenuml corpus must be JSON");
    assert_eq!(corpus["upstream_version"].as_str(), Some("11.16.1"));
    for fixture in corpus["fixtures"].as_array().expect("fixture array") {
        let result = parse_zenuml(fixture["source"].as_str().expect("fixture source"));
        assert_eq!(result.is_ok(), fixture["compatible"].as_bool().unwrap(), "fixture {:?}", fixture["name"]);
    }
}

#[test]
fn info_full_status_is_backed_by_the_complete_pinned_corpus() {
    let corpus: Value = serde_json::from_str(INFO_CORPUS).expect("info corpus must be JSON");
    assert_eq!(corpus["upstream_version"].as_str(), Some("11.16.1"));
    for fixture in corpus["valid"].as_array().expect("valid fixture array") {
        let diagram = parse_info(fixture["source"].as_str().expect("fixture source"))
            .unwrap_or_else(|error| panic!("valid info fixture {:?} failed: {error}", fixture["name"]));
        assert_eq!(diagram.version, "11.16.1");
    }
    for fixture in corpus["invalid"].as_array().expect("invalid fixture array") {
        assert!(parse_info(fixture["source"].as_str().expect("fixture source")).is_err(),
            "invalid info fixture {:?} should fail", fixture["name"]);
    }
}

#[test]
fn pinned_railroad_subset_corpus_parses_to_recursive_ir() {
    let corpus: Value = serde_json::from_str(RAILROAD_CORPUS).expect("railroad corpus must be JSON");
    assert_eq!(corpus["upstream_version"].as_str(), Some("11.16.1"));
    for fixture in corpus["fixtures"].as_array().expect("fixture array") {
        let name = fixture["name"].as_str().expect("fixture name");
        let diagram = parse_railroad(fixture["source"].as_str().expect("fixture source"))
            .unwrap_or_else(|error| panic!("railroad fixture {name} failed: {error}"));
        assert!(!diagram.rules.is_empty());
    }
}

#[test]
fn pinned_railroad_ebnf_corpus_parses_to_recursive_ir() {
    let corpus: Value = serde_json::from_str(RAILROAD_EBNF_CORPUS).expect("railroad EBNF corpus must be JSON");
    for fixture in corpus["fixtures"].as_array().expect("fixtures must be an array") {
        let name = fixture["name"].as_str().expect("fixture name");
        let diagram = parse_railroad(fixture["source"].as_str().expect("fixture source"))
            .unwrap_or_else(|error| panic!("railroad EBNF fixture {name} failed: {error}"));
        assert!(!diagram.rules.is_empty());
        if name == "metadata-and-comments" {
            assert_eq!(diagram.title.as_deref(), Some("Expression grammar"));
            assert_eq!(diagram.accessibility_description.as_deref(), Some("EBNF grammar"));
        }
    }
}

#[test]
fn pinned_railroad_abnf_corpus_parses_to_recursive_ir() {
    let corpus: Value = serde_json::from_str(RAILROAD_ABNF_CORPUS).expect("railroad ABNF corpus must be JSON");
    for fixture in corpus["fixtures"].as_array().expect("fixtures must be an array") {
        let name = fixture["name"].as_str().expect("fixture name");
        let diagram = parse_railroad(fixture["source"].as_str().expect("fixture source"))
            .unwrap_or_else(|error| panic!("railroad ABNF fixture {name} failed: {error}"));
        assert!(!diagram.rules.is_empty());
    }
}

#[test]
fn pinned_railroad_peg_corpus_parses_to_recursive_ir() {
    let corpus: Value = serde_json::from_str(RAILROAD_PEG_CORPUS).expect("railroad PEG corpus must be JSON");
    for fixture in corpus["fixtures"].as_array().expect("fixtures must be an array") {
        let name = fixture["name"].as_str().expect("fixture name");
        let diagram = parse_railroad(fixture["source"].as_str().expect("fixture source"))
            .unwrap_or_else(|error| panic!("railroad PEG fixture {name} failed: {error}"));
        assert!(!diagram.rules.is_empty());
        if name == "metadata-and-comments" {
            assert_eq!(diagram.title.as_deref(), Some("Path grammar"));
            assert_eq!(diagram.accessibility_description.as_deref(), Some("PEG grammar"));
        }
    }
}

#[test]
fn pinned_treeview_subset_corpus_parses_to_tree_ir() {
    let corpus: Value = serde_json::from_str(TREEVIEW_CORPUS).expect("treeview corpus must be JSON");
    assert_eq!(corpus["upstream"].as_str(), Some("mermaid@11.16.1"));
    assert_eq!(corpus["level"].as_str(), Some("full"));
    for fixture in corpus["fixtures"].as_array().expect("fixture array") {
        let name = fixture["name"].as_str().expect("fixture name");
        let diagram = parse_treeview(fixture["source"].as_str().expect("fixture source"))
            .unwrap_or_else(|error| panic!("treeview fixture {name} failed: {error}"));
        assert_eq!(diagram.nodes[0].label, "/");
        assert!(diagram.nodes[0].is_implicit_root);
        assert_eq!(diagram.nodes[0].depth, 0);
        if name == "header-only" {
            assert_eq!(diagram.nodes.len(), 1);
        } else if name == "empty-metadata" {
            assert_eq!(diagram.title.as_deref(), Some(""));
            assert_eq!(diagram.accessibility_title.as_deref(), Some(""));
            assert_eq!(diagram.accessibility_description.as_deref(), Some(""));
            assert_eq!(diagram.nodes.len(), 1);
        } else if name == "multiline-accessibility-description" {
            assert_eq!(diagram.accessibility_description.as_deref(),
                Some("Files grouped by\ntheir directory hierarchy"));
            assert_eq!(diagram.nodes.len(), 2);
        } else if name == "layout-and-icon-config" {
            assert_eq!((diagram.config.row_indent, diagram.config.padding_x, diagram.config.padding_y),
                (18.0, 9.0, 7.0));
            assert_eq!(diagram.config.line_thickness, 3.0);
            assert!(!diagram.config.use_max_width);
            assert!(!diagram.config.show_icons);
            assert_eq!(diagram.nodes[2].icon.as_deref(), Some("mermaid-treeview:folder"));
        } else if name == "front-matter-config" {
            assert_eq!((diagram.config.row_indent, diagram.config.padding_x), (14.0, 8.0));
            assert!(!diagram.config.show_icons);
        } else if name == "icon-resolution-config" {
            assert_eq!(diagram.config.default_icon_pack, "devicon");
            assert_eq!(diagram.nodes.iter().skip(1).map(|node| node.icon.as_deref()).collect::<Vec<_>>(),
                [Some("logos:markdown"), Some("none"), Some("devicon:rust"),
                    Some("logos:typescript"), Some("mermaid-treeview:file"), Some("devicon:custom")]);
        } else if name == "icon-detection-precedence" {
            assert_eq!(diagram.nodes.iter().skip(1).map(|node| node.icon.as_deref()).collect::<Vec<_>>(),
                [Some("logos:special"), Some("devicon:typescript"), Some("mermaid-treeview:file"),
                    Some("mermaid-treeview:folder"), Some("none"), Some("devicon:typescript")]);
        } else if name == "built-in-and-unprefixed-icons" {
            assert_eq!(diagram.nodes.iter().skip(1).map(|node| node.icon.as_deref()).collect::<Vec<_>>(),
                [Some("mermaid-treeview:folder"), Some("mermaid-treeview:file"),
                    Some("mermaid-treeview:custom")]);
        } else if name == "parser-terminal-semantics" {
            assert_eq!(diagram.nodes.iter().skip(1).map(|node| node.label.as_str()).collect::<Vec<_>>(),
                ["", "my folder", ".gitignore", "docker-compose.yml", "My Documents", "index.js"]);
            assert!(matches!(diagram.nodes[2].kind, diagram_ir::TreeViewNodeKind::Directory));
            assert_eq!(diagram.nodes[5].class_selector.as_deref(), Some("my-class"));
            assert_eq!(diagram.nodes[6].description, None);
        } else if name == "icon-suppression-and-annotation-order" {
            assert!(diagram.nodes.iter().skip(1).all(|node| node.icon.as_deref() == Some("none")));
            assert_eq!(diagram.nodes[1].class_selector.as_deref(), Some("highlight"));
            assert_eq!(diagram.nodes[1].description.as_deref(), Some("entry point"));
        } else if name == "unicode-and-emoji-labels" {
            assert_eq!(diagram.nodes[1].label, "But  _  _ton💓.tsx");
            assert_eq!(diagram.nodes[2].label, "🚀 rocket-app");
            assert_eq!(diagram.nodes[3].parent_id.as_deref(), Some("treeview-2"));
        } else if name == "quoted-complex-hierarchy" {
            assert_eq!(diagram.nodes.len(), 9);
            assert_eq!(diagram.nodes[8].depth, 4);
        } else if name == "multiple-root-hierarchy" {
            assert_eq!(diagram.nodes.iter().filter(|node| node.parent_id.as_deref() == Some("treeview-root")).count(), 3);
        } else if matches!(name, "rooted-box-drawing-hierarchy" | "compact-box-drawing-segments") {
            assert_eq!(diagram.nodes.iter().map(|node| node.depth).collect::<Vec<_>>(), [0, 1, 2, 3, 2]);
            assert_eq!(diagram.nodes.iter().map(|node| node.parent_id.as_deref()).collect::<Vec<_>>(),
                [None, Some("treeview-root"), Some("treeview-1"), Some("treeview-2"), Some("treeview-1")]);
            if name == "compact-box-drawing-segments" {
                assert_eq!(diagram.nodes[2].class_selector.as_deref(), Some("highlight"));
                assert_eq!(diagram.nodes[3].description.as_deref(), Some("crate root"));
            }
        } else if name == "front-matter-theme-variables" {
            assert_eq!(diagram.config.theme.label_font_size, 20.0);
            assert_eq!(diagram.config.theme.label_color, "#112233");
            assert_eq!(diagram.config.theme.line_color, "#234567");
            assert_eq!(diagram.config.theme.icon_color, "#345678");
            assert_eq!(diagram.config.theme.description_color, "#456789");
            assert_eq!(diagram.config.theme.highlight_background, "rgba(10, 20, 30, 0.25)");
            assert_eq!(diagram.config.theme.highlight_stroke, "#56789a");
        } else if name == "directive-theme-variables" {
            assert_eq!(diagram.config.theme.label_font_size, 18.0);
            assert_eq!(diagram.config.theme.label_color, "#abcdef");
            assert_eq!(diagram.config.theme.line_color, "#123456");
        } else if name == "quoted-labels-and-annotations" {
            assert_eq!(diagram.title.as_deref(), Some("Application Files"));
            assert_eq!(diagram.nodes[1].label, "my project");
        } else if name == "aligned-description-rows" {
            assert_eq!(diagram.nodes[2].description.as_deref(), Some("short label"));
            assert_eq!(diagram.nodes[3].description.as_deref(), Some("long label"));
        } else {
            assert!(!diagram.nodes.is_empty());
        }
    }
    for fixture in corpus["invalid"].as_array().expect("invalid fixture array") {
        let name = fixture["name"].as_str().expect("invalid fixture name");
        assert!(parse_treeview(fixture["source"].as_str().expect("invalid fixture source")).is_err(),
            "invalid upstream fixture {name} unexpectedly parsed");
    }
}

#[test]
fn treeview_full_status_is_backed_by_pinned_syntax_and_visual_corpora() {
    let manifest: Value = serde_json::from_str(COMPATIBILITY_MANIFEST).expect("compatibility manifest must be JSON");
    let family = manifest["families"].as_array().expect("families array").iter()
        .find(|family| family["id"] == "treeview").expect("treeview family");
    assert_eq!(family["status"].as_str(), Some("full"));

    let syntax: Value = serde_json::from_str(TREEVIEW_CORPUS).expect("treeview corpus must be JSON");
    let visual: Value = serde_json::from_str(TREEVIEW_VISUAL_CORPUS).expect("treeview visual corpus must be JSON");
    assert_eq!(syntax["upstream_commit"], visual["upstream_commit"]);
    let syntax_names = syntax["fixtures"].as_array().expect("fixture array").iter()
        .map(|fixture| fixture["name"].as_str().expect("fixture name")).collect::<BTreeSet<_>>();
    let visual_names = visual["fixtures"].as_array().expect("visual fixture array").iter()
        .map(|name| name.as_str().expect("visual fixture name")).collect::<BTreeSet<_>>();
    assert!(!syntax["invalid"].as_array().expect("invalid fixture array").is_empty());
    assert!(visual_names.is_subset(&syntax_names));
    assert_eq!(visual_names.len(), visual["fixtures"].as_array().expect("visual fixture array").len());
    assert!(BTreeSet::from([
        "parser-terminal-semantics", "icon-suppression-and-annotation-order", "unicode-and-emoji-labels",
        "quoted-complex-hierarchy", "multiple-root-hierarchy", "icon-detection-precedence",
        "rooted-box-drawing-hierarchy", "front-matter-theme-variables", "intrinsic-description-width",
    ]).is_subset(&syntax_names));
}

#[test]
fn treeview_box_drawing_rejects_mixed_indentation_with_original_line_numbers() {
    let mixed = parse_treeview("treeView-beta\n├── src/\n    index.ts\n└── README.md")
        .expect_err("box-drawing mode must reject indentation-only children");
    assert_eq!(mixed.line, 3);
    assert!(mixed.message.contains("unexpected indentation"));

    let empty = parse_treeview("treeView-beta\nroot/\n├── src/\n│   └── ")
        .expect_err("empty box-drawing nodes must fail");
    assert_eq!(empty.line, 4);
    assert!(empty.message.contains("empty TreeView box-drawing node"));

    let remapped = parse_treeview("treeView-beta\n├── src/\n│\n└── \"unterminated")
        .expect_err("semantic errors must retain their original source line");
    assert_eq!(remapped.line, 4);
    assert!(remapped.message.contains("unterminated TreeView quoted label"));
}

#[test]
fn pinned_cynefin_corpus_matches_upstream_acceptance() {
    let corpus: Value = serde_json::from_str(CYNEFIN_CORPUS).expect("cynefin corpus must be JSON");
    assert_eq!(corpus["upstream"].as_str(), Some("mermaid@11.16.1"));
    assert_eq!(corpus["level"].as_str(), Some("full"));
    assert_eq!(corpus["upstream_commit"].as_str(), Some("7ecca0cd7f1658ef74f4e7e91f925724ef403bbf"));
    for fixture in corpus["valid"].as_array().expect("valid fixture array") {
        let name = fixture["id"].as_str().expect("fixture id");
        let diagram = parse_cynefin(fixture["source"].as_str().expect("fixture source"))
            .unwrap_or_else(|error| panic!("cynefin fixture {name} failed: {error}"));
        if name == "accessibility" {
            assert_eq!(diagram.accessibility_title.as_deref(), Some("Incident response framework"));
            assert_eq!(diagram.accessibility_description.as_deref(), Some("Practices organized by\nuncertainty and causality"));
        }
        if name == "canvas-config" {
            assert_eq!((diagram.config.width, diagram.config.height, diagram.config.padding), (640.0, 420.0, 24.0));
            assert!(!diagram.config.show_domain_descriptions);
            assert_eq!((diagram.config.boundary_amplitude, diagram.config.seed), (12.0, 17));
        }
        if name == "theme-config" {
            let style = &diagram.config.style;
            assert_eq!((style.domain_font_size, style.item_font_size), (18.0, 13.0));
            assert_eq!((&style.boundary_color, style.boundary_width, &style.cliff_color, style.cliff_width),
                (&"#112233".to_string(), 3.0, &"#441111".to_string(), 5.0));
            assert_eq!((&style.arrow_color, style.arrow_width, &style.confusion_bg, &style.label_color),
                (&"#224466".to_string(), 4.0, &"#eee1f1".to_string(), &"#101010".to_string()));
        }
        if name == "front-matter-config-and-theme" {
            assert_eq!((diagram.config.width, diagram.config.padding), (680.0, 28.0));
            assert_eq!(diagram.config.style.boundary_color, "#334455");
            assert_eq!(diagram.config.style.complex_bg, "#ddeedd");
        }
        if name == "quoted-strings" {
            assert_eq!(diagram.domains[0].items,
                ["Single quoted", "Escaped \"quote\" and \\ slash", "Line\nfeed"]);
            assert_eq!(diagram.transitions[0].label.as_deref(), Some("Shift's label"));
        }
        if name == "metadata-variants" {
            assert_eq!(diagram.title.as_deref(), Some(""));
            assert_eq!(diagram.accessibility_title.as_deref(), Some("Spaced accessibility title"));
            assert_eq!(diagram.accessibility_description.as_deref(), Some("First line\nSecond line"));
        }
        if name == "inline-accessibility-description" {
            assert_eq!(diagram.accessibility_description.as_deref(), Some("Inline description"));
        }
        if name == "duplicate-domain-last-wins" {
            assert_eq!(diagram.domains[0].items, ["Replacement item"]);
        }
    }
    for fixture in corpus["invalid"].as_array().expect("invalid fixture array") {
        let name = fixture["id"].as_str().expect("invalid fixture id");
        assert!(parse_cynefin(fixture["source"].as_str().expect("invalid fixture source")).is_err(),
            "invalid upstream fixture {name} unexpectedly parsed");
    }
    assert_eq!(parse_cynefin("cynefin-beta\nclear").expect("default seed source").config.seed, 145_697_634);
    assert_eq!(parse_cynefin("%%{init: {\"cynefin\": {\"seed\": 0}}}%%\ncynefin-beta\nclear")
        .expect("zero seed source").config.seed, 715_869_649);
}

#[test]
fn cynefin_full_status_is_backed_by_pinned_syntax_and_visual_corpora() {
    let manifest: Value =
        serde_json::from_str(COMPATIBILITY_MANIFEST).expect("compatibility manifest must be JSON");
    let family = manifest["families"]
        .as_array()
        .expect("families array")
        .iter()
        .find(|family| family["id"] == "cynefin")
        .expect("cynefin family");
    assert_eq!(family["status"].as_str(), Some("full"));

    let syntax: Value = serde_json::from_str(CYNEFIN_CORPUS).expect("cynefin corpus must be JSON");
    let visual: Value =
        serde_json::from_str(CYNEFIN_VISUAL_CORPUS).expect("cynefin visual corpus must be JSON");
    assert_eq!(syntax["upstream_commit"], visual["upstream_commit"]);
    let valid_ids = syntax["valid"]
        .as_array()
        .expect("valid fixture array")
        .iter()
        .map(|fixture| fixture["id"].as_str().expect("fixture id"))
        .collect::<BTreeSet<_>>();
    let visual_ids = visual["fixtures"]
        .as_array()
        .expect("visual fixture array")
        .iter()
        .map(|id| id.as_str().expect("visual fixture id"))
        .collect::<BTreeSet<_>>();
    assert!(!syntax["invalid"].as_array().expect("invalid fixture array").is_empty());
    assert!(!visual_ids.is_empty());
    assert!(visual_ids.is_subset(&valid_ids));
    assert_eq!(visual_ids.len(), visual["fixtures"].as_array().expect("visual fixture array").len());
}

#[test]
fn pinned_wardley_subset_corpus_parses_to_strategic_map_ir() {
    let corpus: Value = serde_json::from_str(WARDLEY_CORPUS).expect("wardley corpus must be JSON");
    assert_eq!(corpus["upstream"].as_str(), Some("mermaid@11.16.1"));
    for fixture in corpus["fixtures"].as_array().expect("fixture array") {
        let name = fixture["name"].as_str().expect("fixture name");
        let diagram = parse_wardley(fixture["source"].as_str().expect("fixture source"))
            .unwrap_or_else(|error| panic!("wardley fixture {name} failed: {error}"));
        assert!(!diagram.nodes.is_empty());
    }
}

#[test]
fn pinned_ishikawa_subset_corpus_parses_to_causal_tree_ir() {
    let corpus: Value = serde_json::from_str(ISHIKAWA_CORPUS).expect("ishikawa corpus must be JSON");
    assert_eq!(corpus["upstream"].as_str(), Some("mermaid@11.16.1"));
    for fixture in corpus["fixtures"].as_array().expect("fixture array") {
        let name = fixture["name"].as_str().expect("fixture name");
        let diagram = parse_ishikawa(fixture["source"].as_str().expect("fixture source"))
            .unwrap_or_else(|error| panic!("ishikawa fixture {name} failed: {error}"));
        assert!(!diagram.effect.is_empty());
        assert!(!diagram.causes.is_empty());
        if name == "diagram-padding-config" {
            assert_eq!(diagram.diagram_padding, 64.0);
        }
    }
}

#[test]
fn pinned_venn_subset_corpus_parses_to_set_ir() {
    let corpus: Value = serde_json::from_str(VENN_CORPUS).expect("venn corpus must be JSON");
    assert_eq!(corpus["upstream"].as_str(), Some("mermaid@11.16.1"));
    for fixture in corpus["fixtures"].as_array().expect("fixture array") {
        let name = fixture["name"].as_str().expect("fixture name");
        let diagram = parse_venn(fixture["source"].as_str().expect("fixture source"))
            .unwrap_or_else(|error| panic!("venn fixture {name} failed: {error}"));
        assert!(diagram.regions.iter().any(|region| region.sets.len() == 1));
    }
}

#[test]
fn pinned_treemap_subset_corpus_parses_to_hierarchy_ir() {
    let corpus: Value = serde_json::from_str(TREEMAP_CORPUS).expect("treemap corpus must be JSON");
    assert_eq!(corpus["upstream"].as_str(), Some("mermaid@11.16.1"));
    for fixture in corpus["fixtures"].as_array().expect("fixture array") {
        let name = fixture["name"].as_str().expect("fixture name");
        let source = fixture["source"].as_str().expect("fixture source");
        let diagram = parse_treemap(source)
            .unwrap_or_else(|error| panic!("treemap fixture {name} failed: {error}"));
        if name == "empty" {
            assert!(diagram.nodes.is_empty());
        } else {
            assert!(!diagram.nodes.is_empty());
        }
    }
    for fixture in corpus["invalid"].as_array().expect("invalid fixture array") {
        let name = fixture["name"].as_str().expect("fixture name");
        let source = fixture["source"].as_str().expect("fixture source");
        let error = match parse_treemap(source) {
            Ok(_) => panic!("invalid treemap fixture {name} parsed"),
            Err(error) => error,
        };
        assert!(error.message.contains("Multiple root nodes"));
    }
}

#[test]
fn pinned_event_modeling_corpus_matches_upstream_acceptance() {
    let corpus: Value = serde_json::from_str(EVENTMODELING_CORPUS)
        .expect("event modeling corpus must be JSON");
    assert_eq!(corpus["upstream"].as_str(), Some("mermaid@11.16.1"));
    assert_eq!(corpus["level"].as_str(), Some("full"));
    for fixture in corpus["valid"].as_array().expect("valid fixture array") {
        let id = fixture["id"].as_str().expect("fixture id");
        let source = fixture["source"].as_str().expect("fixture source");
        let diagram = parse_event_modeling(source)
            .unwrap_or_else(|error| panic!("event modeling fixture {id} failed: {error}"));
        if id == "inline-data" {
            assert_eq!(diagram.frames[1].data.as_deref(), Some("description: string"));
            assert_eq!(diagram.frames[2].data_type.as_deref(), Some("json"));
            assert_eq!(diagram.frames[2].data.as_deref(), Some("\"description\": \"book\""));
        } else if id == "data-blocks" {
            assert_eq!(diagram.data_blocks.len(), 2);
            assert_eq!(diagram.frames[1].data_reference.as_deref(), Some("AddItemData"));
            assert_eq!(diagram.frames[1].data_type.as_deref(), Some("json"));
            assert!(diagram.frames[1].data.as_deref().is_some_and(|data| data.contains("quantity")));
            assert_eq!(diagram.frames[2].data_reference.as_deref(), Some("ItemAddedData"));
        } else if id == "quoted-inline-data" {
            assert_eq!(diagram.frames[0].data.as_deref(), Some("shopping cart"));
            assert_eq!(diagram.frames[1].data_type.as_deref(), Some("json"));
            assert_eq!(diagram.frames[1].data.as_deref(), Some("{ \"quantity\": 2 }"));
            assert_eq!(diagram.frames[2].data.as_deref(), Some("accepted"));
        } else if id == "frame-notes" {
            assert_eq!(diagram.notes.len(), 2);
            assert_eq!(diagram.notes[0].source_frame, "01");
            assert_eq!(diagram.notes[0].data_type.as_deref(), Some("md"));
            assert!(diagram.notes[0].data.contains("Shows pending items"));
        } else if id == "given-when-then" {
            assert_eq!(diagram.gwt.len(), 2);
            assert_eq!(diagram.gwt[0].source_frame, "02");
            assert_eq!(diagram.gwt[0].given.len(), 2);
            assert_eq!(diagram.gwt[0].when.len(), 2);
            assert_eq!(diagram.gwt[0].then[0].entity_id, "ItemAdded");
            assert_eq!(diagram.gwt[1].source_frame, "03");
            assert!(diagram.gwt[1].when.is_empty());
        } else if id == "standalone-entities" {
            assert_eq!(diagram.entities.len(), 3);
            assert_eq!(diagram.entities[0].id, "Sales.CartUI");
            assert_eq!(diagram.entities[0].namespace.as_deref(), Some("Sales"));
            assert_eq!(diagram.entities[1].id, "AddItem");
            assert_eq!(diagram.entities[1].namespace, None);
        } else if id == "init-layout-config" {
            assert_eq!(diagram.config.padding, 18.0);
            assert_eq!(diagram.config.row_height, 40.0);
            assert!(!diagram.config.use_max_width);
        } else if id == "front-matter-layout-config" {
            assert_eq!(diagram.config.padding, 22.0);
            assert_eq!(diagram.config.row_height, 36.0);
            assert!(!diagram.config.use_max_width);
        } else if id == "init-theme-colors" {
            assert_eq!(diagram.config.styles.ui_fill, "#102030");
            assert_eq!(diagram.config.styles.command_stroke, "#405060");
            assert_eq!(diagram.config.styles.event_fill, "#506070");
        } else if id == "front-matter-theme-colors" {
            assert_eq!(diagram.config.styles.processor_fill, "#112233");
            assert_eq!(diagram.config.styles.processor_stroke, "#223344");
            assert_eq!(diagram.config.styles.read_model_fill, "#334455");
            assert_eq!(diagram.config.styles.read_model_stroke, "#445566");
        } else if id == "header-only" {
            assert!(diagram.frames.is_empty());
        } else if id == "upstream-complex-model" {
            assert_eq!(diagram.frames.len(), 6);
            assert_eq!(diagram.data_blocks.len(), 3);
            assert_eq!(diagram.notes.len(), 2);
            assert_eq!(diagram.gwt.len(), 2);
        } else if id == "separated-block-opening-and-data-types" {
            assert_eq!(diagram.data_blocks[0].data_type.as_deref(), Some("html"));
            assert_eq!(diagram.notes[0].data_type.as_deref(), Some("json"));
        }
    }
    for fixture in corpus["invalid"].as_array().expect("invalid fixture array") {
        let id = fixture["id"].as_str().expect("fixture id");
        let source = fixture["source"].as_str().expect("fixture source");
        assert!(parse_event_modeling(source).is_err(), "invalid fixture {id} parsed");
    }
    let error = parse_event_modeling(
        "eventmodeling\nrf 01 evt Changed\nrf 02 cmd Update\nrf 03 pcr Projector ->> 01 ->> 02",
    )
    .expect_err("processor must reject every non-read-model source");
    assert_eq!(error.message.lines().count(), 2);
    assert!(error.message.contains("not from 'evt'"));
    assert!(error.message.contains("not from 'cmd'"));
}

#[test]
fn event_modeling_full_status_is_backed_by_pinned_syntax_and_visual_corpora() {
    let manifest: Value =
        serde_json::from_str(COMPATIBILITY_MANIFEST).expect("compatibility manifest must be JSON");
    let family = manifest["families"]
        .as_array()
        .expect("families array")
        .iter()
        .find(|family| family["id"] == "eventmodeling")
        .expect("event modeling family");
    assert_eq!(family["status"].as_str(), Some("full"));

    let syntax: Value =
        serde_json::from_str(EVENTMODELING_CORPUS).expect("event modeling corpus must be JSON");
    let visual: Value = serde_json::from_str(EVENTMODELING_VISUAL_CORPUS)
        .expect("event modeling visual corpus must be JSON");
    assert_eq!(syntax["upstream_commit"], visual["upstream_commit"]);
    let valid_ids = syntax["valid"]
        .as_array()
        .expect("valid fixture array")
        .iter()
        .map(|fixture| fixture["id"].as_str().expect("fixture id"))
        .collect::<BTreeSet<_>>();
    let visual_ids = visual["fixtures"]
        .as_array()
        .expect("visual fixture array")
        .iter()
        .map(|id| id.as_str().expect("visual fixture id"))
        .collect::<BTreeSet<_>>();
    assert!(!syntax["invalid"].as_array().expect("invalid fixture array").is_empty());
    assert!(!visual_ids.is_empty());
    assert!(visual_ids.is_subset(&valid_ids));
}

#[test]
fn radar_full_status_is_backed_by_the_pinned_corpus() {
    let manifest: Value =
        serde_json::from_str(COMPATIBILITY_MANIFEST).expect("compatibility manifest must be JSON");
    let radar = manifest["families"]
        .as_array()
        .expect("families array")
        .iter()
        .find(|family| family["id"] == "radar")
        .expect("radar family");
    assert_eq!(radar["status"].as_str(), Some("full"));

    let corpus: Value = serde_json::from_str(RADAR_CORPUS).expect("radar corpus must be JSON");
    assert_eq!(corpus["upstream"].as_str(), Some("mermaid@11.16.1"));
    for fixture in corpus["fixtures"].as_array().expect("fixture array") {
        let id = fixture["id"].as_str().expect("fixture id");
        let source = fixture["source"].as_str().expect("fixture source");
        let chart = parse_radar(source)
            .unwrap_or_else(|error| panic!("radar fixture {id} failed: {error}"));
        assert!(!chart.series.is_empty());
        if id == "angular-label-anchors" {
            assert_eq!(
                chart.x_axis.as_ref().map(|axis| axis.categories.len()),
                Some(8)
            );
            assert_eq!(chart.series[0].data.len(), 8);
        } else if id == "core-options" {
            assert!(!chart.radar_config.show_legend);
            assert_eq!(chart.radar_config.ticks, 4.0);
            assert_eq!(
                chart.y_axis.as_ref().map(|axis| (axis.min, axis.max)),
                Some((10.0, 90.0))
            );
        } else if id == "curve-list-and-multiline" {
            assert_eq!(chart.series.len(), 2);
            assert_eq!(chart.series[1].label.as_deref(), Some("Second"));
            assert_eq!(chart.series[1].data[2].value, 1.0);
        } else if id == "init-layout-config" {
            assert_eq!(
                (chart.radar_config.width, chart.radar_config.height),
                (Some(520.0), Some(480.0))
            );
            assert_eq!(
                (
                    chart.radar_config.margin_left,
                    chart.radar_config.margin_right
                ),
                (Some(25.0), Some(35.0))
            );
            assert_eq!(chart.radar_config.axis_label_factor, Some(1.1));
            assert_eq!(chart.radar_config.curve_tension, Some(0.25));
        } else if id == "front-matter-layout-config" {
            assert_eq!(chart.radar_config.width, Some(440.0));
            assert_eq!(chart.radar_config.axis_scale_factor, Some(0.8));
        } else if id == "init-theme-style" {
            assert_eq!(chart.radar_config.axis_color.as_deref(), Some("#203040"));
            assert_eq!(chart.radar_config.axis_stroke_width, Some(3.0));
            assert_eq!(chart.radar_config.curve_opacity, Some(0.4));
            assert_eq!(chart.radar_config.graticule_stroke_width, Some(2.0));
            assert_eq!(chart.radar_config.series_colors, vec!["#102030"]);
        } else if id == "front-matter-theme-style" {
            assert_eq!(chart.radar_config.axis_color.as_deref(), Some("#506070"));
            assert_eq!(chart.radar_config.curve_opacity, Some(0.35));
            assert_eq!(chart.radar_config.series_colors, vec!["#405060"]);
        } else if id == "sparse-theme-palette" {
            assert_eq!(chart.radar_config.series_colors, vec!["", "#708090"]);
        }
    }
}

#[test]
fn pinned_architecture_subset_corpus_parses_to_structural_ir() {
    let corpus: Value = serde_json::from_str(ARCHITECTURE_CORPUS)
        .expect("architecture corpus must be JSON");
    assert_eq!(corpus["upstream"].as_str(), Some("mermaid@11.16.1"));
    for fixture in corpus["fixtures"].as_array().expect("fixture array") {
        let id = fixture["id"].as_str().expect("fixture id");
        let source = fixture["source"].as_str().expect("fixture source");
        let diagram = parse_architecture(source)
            .unwrap_or_else(|error| panic!("architecture fixture {id} failed: {error}"));
        assert!(!diagram.nodes.is_empty());
        if id == "edge-elasticity-config" {
            assert_eq!(
                diagram.architecture_config.as_ref().unwrap().edge_elasticity,
                0.8
            );
        }
        if id == "seeded-random-layout-config" {
            let config = diagram.architecture_config.as_ref().unwrap();
            assert!(config.randomize);
            assert_eq!(config.seed, 17);
        }
    }
}

#[test]
fn pinned_kanban_subset_corpus_parses_to_board_ir() {
    let corpus: Value = serde_json::from_str(KANBAN_CORPUS).expect("kanban corpus must be JSON");
    assert_eq!(corpus["upstream"].as_str(), Some("mermaid@11.16.1"));
    for fixture in corpus["fixtures"].as_array().expect("fixture array") {
        let id = fixture["id"].as_str().expect("fixture id");
        let source = fixture["source"].as_str().expect("fixture source");
        let board = parse_kanban(source)
            .unwrap_or_else(|error| panic!("kanban fixture {id} failed: {error}"));
        assert!(!board.columns.is_empty());
        if id == "inline-card-metadata" {
            let card = &board.columns[0].cards[0];
            assert_eq!(card.ticket.as_deref(), Some("MC-42"));
            assert_eq!(card.assigned.as_deref(), Some("Ada"));
            assert_eq!(card.priority.as_deref(), Some("high"));
        }
        if id == "multiline-card-metadata" {
            assert_eq!(board.columns[0].cards[0].label.text, "Grammar parser");
            assert_eq!(board.columns[0].cards[0].icon.as_deref(), Some("heart"));
        }
        if id == "card-icon-decoration" {
            assert_eq!(board.columns[0].cards[0].icon.as_deref(), Some("heart"));
        }
        if id == "class-decorations" {
            assert_eq!(board.columns[0].classes, ["backlog"]);
            assert_eq!(board.columns[0].cards[0].classes, ["urgent", "blocked"]);
        }
        if id == "ticket-base-url" {
            assert_eq!(
                board.ticket_base_url.as_deref(),
                Some("https://tracker.example/issues/#TICKET#")
            );
        }
        if id == "layout-config" {
            assert_eq!(board.config.section_width, 300.0);
            assert_eq!(board.config.padding, 32.0);
        }
        if id == "section-ticket-metadata" {
            assert_eq!(board.columns[0].ticket.as_deref(), Some("KB-7"));
        }
        if id == "shape-delimited-labels" {
            assert_eq!(board.columns[0].label.text, "Todo");
            assert_eq!(board.columns[0].cards[0].label.text, "Rounded card");
            assert_eq!(board.columns[0].cards[1].label.text, "Hex card");
            assert_eq!(board.columns[1].cards[0].label.text, "Circle card");
        }
        if id == "cloud-and-bang-labels" {
            assert_eq!(board.columns[0].label.text, "Cloud section");
            assert_eq!(board.columns[0].cards[0].label.text, "Bang card");
            assert_eq!(board.columns[0].cards[1].label.text, "Cloud card");
            assert_eq!(board.columns[0].cards[2].label.text, "Burst card");
        }
        if id == "quoted-labels" {
            assert_eq!(board.columns[0].label.text, "Todo queue");
            assert_eq!(board.columns[0].cards[0].label.text, "Quoted card");
        }
        if id == "markdown-labels" {
            let column = &board.columns[0];
            let card = &column.cards[0];
            assert_eq!(column.label.text, "Todo queue");
            assert_eq!(column.label.markdown.as_deref(), Some("**Todo** queue"));
            assert!(column.label.spans[0].bold);
            assert_eq!(card.label.text, "Quoted card");
            assert!(card.label.spans.iter().any(|span| span.italic));
        }
        if id == "inline-markdown-labels" {
            assert_eq!(board.columns[0].label.text, "Todo queue");
            assert!(board.columns[0].label.spans[0].bold);
            assert_eq!(board.columns[0].cards[0].label.text, "Quoted card");
            assert!(board.columns[0].cards[0].label.spans.iter().any(|span| span.italic));
        }
        if id == "multiline-labels" {
            assert_eq!(board.columns[0].label.text, "Todo\nqueue");
            assert_eq!(board.columns[0].cards[0].label.text, "Line 1\nLine 2\nLine 3");
        }
        if id == "priority-markers" {
            assert_eq!(board.columns[0].cards[0].priority.as_deref(), Some("Very High"));
            assert_eq!(board.columns[0].cards[1].priority.as_deref(), Some("Very Low"));
        }
    }
}

#[test]
fn packet_full_status_is_backed_by_the_pinned_corpus() {
    let manifest: Value =
        serde_json::from_str(COMPATIBILITY_MANIFEST).expect("compatibility manifest must be JSON");
    let packet = manifest["families"]
        .as_array()
        .expect("families array")
        .iter()
        .find(|family| family["id"] == "packet")
        .expect("packet family");
    assert_eq!(packet["status"].as_str(), Some("full"));

    let corpus: Value = serde_json::from_str(PACKET_CORPUS).expect("packet corpus must be JSON");
    assert_eq!(corpus["upstream"].as_str(), Some("mermaid@11.16.1"));
    assert_eq!(corpus["level"].as_str(), Some("full"));
    for fixture in corpus["valid"].as_array().expect("valid fixture array") {
        let id = fixture["id"].as_str().expect("fixture id");
        let source = fixture["source"].as_str().expect("fixture source");
        parse_packet(source).unwrap_or_else(|error| panic!("packet fixture {id} failed: {error}"));
    }
    for fixture in corpus["invalid"].as_array().expect("invalid fixture array") {
        let id = fixture["id"].as_str().expect("fixture id");
        let source = fixture["source"].as_str().expect("fixture source");
        assert!(parse_packet(source).is_err(), "invalid packet fixture {id} parsed");
    }
}

#[test]
fn block_full_status_is_backed_by_the_complete_pinned_corpus() {
    let manifest: Value =
        serde_json::from_str(COMPATIBILITY_MANIFEST).expect("compatibility manifest must be JSON");
    let block = manifest["families"]
        .as_array()
        .expect("families array")
        .iter()
        .find(|family| family["id"] == "block")
        .expect("block family");
    assert_eq!(block["status"].as_str(), Some("full"));

    let corpus: Value = serde_json::from_str(BLOCK_CORPUS).expect("block corpus must be JSON");
    assert_eq!(corpus["upstream"].as_str(), Some("mermaid@11.16.1"));
    assert_eq!(
        corpus["upstream_commit"].as_str(),
        Some("7ecca0cd7f1658ef74f4e7e91f925724ef403bbf")
    );
    assert_eq!(corpus["level"].as_str(), Some("full"));
    for fixture in corpus["fixtures"].as_array().expect("fixture array") {
        let id = fixture["id"].as_str().expect("fixture id");
        let source = fixture["source"].as_str().expect("fixture source");
        let diagram = parse_block(source)
            .unwrap_or_else(|error| panic!("block fixture {id} failed: {error}"));
        assert!(diagram.cells.iter().any(|cell| cell.visible));
    }
}

#[test]
fn pinned_mindmap_subset_corpus_parses_to_tree_edges() {
    let corpus: Value = serde_json::from_str(MINDMAP_CORPUS).expect("mindmap corpus must be JSON");
    assert_eq!(corpus["upstream"].as_str(), Some("mermaid@11.16.1"));
    for fixture in corpus["fixtures"].as_array().expect("fixture array") {
        let id = fixture["id"].as_str().expect("fixture id");
        let source = fixture["source"].as_str().expect("fixture source");
        let diagram = parse_mindmap(source)
            .unwrap_or_else(|error| panic!("mindmap fixture {id} failed: {error}"));
        assert_eq!(diagram.edges.len() + 1, diagram.nodes.len());
    }
}
const REQUIREMENT_CORPUS: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../grammars/mermaid/requirement-11.16.1-corpus.json"
));
const GITGRAPH_CORPUS: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../grammars/mermaid/gitgraph-11.16.1-corpus.json"
));
const PIE_CORPUS: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../grammars/mermaid/pie-11.16.1-corpus.json"
));
const SANKEY_CORPUS: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../grammars/mermaid/sankey-11.16.1-corpus.json"
));
const XYCHART_CORPUS: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../grammars/mermaid/xychart-11.16.1-corpus.json"
));
const GANTT_CORPUS: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../grammars/mermaid/gantt-11.16.1-corpus.json"
));
const GANTT_VISUAL_CORPUS: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../grammars/mermaid/gantt-11.16.1-visual-corpus.json"
));
const SEQUENCE_CORPUS: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../grammars/mermaid/sequence-11.16.1-corpus.json"
));
const SEQUENCE_VISUAL_CORPUS: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../grammars/mermaid/sequence-11.16.1-visual-corpus.json"
));

#[test]
fn pinned_timeline_core_corpus_matches_upstream_acceptance() {
    let corpus: Value = serde_json::from_str(TIMELINE_CORPUS).expect("timeline corpus must be JSON");
    assert_eq!(corpus["upstream_commit"].as_str(),
        Some("7ecca0cd7f1658ef74f4e7e91f925724ef403bbf"));
    for fixture in corpus["valid"].as_array().expect("valid corpus array") {
        let id = fixture["id"].as_str().expect("fixture id");
        let source = fixture["source"].as_str().expect("fixture source");
        parse_timeline(source)
            .unwrap_or_else(|error| panic!("valid upstream fixture {id} failed: {error}"));
    }
    for fixture in corpus["invalid"].as_array().expect("invalid corpus array") {
        let id = fixture["id"].as_str().expect("fixture id");
        let source = fixture["source"].as_str().expect("fixture source");
        assert!(parse_timeline(source).is_err(),
            "invalid upstream fixture {id} unexpectedly parsed");
    }
}

#[test]
fn pinned_sequence_corpus_matches_upstream_acceptance() {
    let corpus: Value = serde_json::from_str(SEQUENCE_CORPUS).expect("sequence corpus must be JSON");
    assert_eq!(
        corpus["upstream_commit"].as_str(),
        Some("7ecca0cd7f1658ef74f4e7e91f925724ef403bbf")
    );
    for fixture in corpus["valid"].as_array().expect("valid corpus array") {
        let id = fixture["id"].as_str().expect("fixture id");
        let source = fixture["source"].as_str().expect("fixture source");
        parse_sequence_diagram(source)
            .unwrap_or_else(|error| panic!("valid upstream fixture {id} failed: {error}"));
    }
    for fixture in corpus["invalid"].as_array().expect("invalid corpus array") {
        let id = fixture["id"].as_str().expect("fixture id");
        let source = fixture["source"].as_str().expect("fixture source");
        assert!(
            parse_sequence_diagram(source).is_err(),
            "invalid upstream fixture {id} unexpectedly parsed"
        );
    }
}

#[test]
fn pinned_sequence_visual_corpus_references_valid_syntax_fixtures() {
    let syntax: Value = serde_json::from_str(SEQUENCE_CORPUS).expect("sequence corpus must be JSON");
    let visual: Value =
        serde_json::from_str(SEQUENCE_VISUAL_CORPUS).expect("sequence visual corpus must be JSON");
    assert_eq!(visual["upstream_commit"], syntax["upstream_commit"]);

    let valid = syntax["valid"]
        .as_array()
        .expect("valid corpus array")
        .iter()
        .map(|fixture| fixture["id"].as_str().expect("fixture id"))
        .collect::<BTreeSet<_>>();
    let fixtures = visual["fixtures"]
        .as_array()
        .expect("visual fixture array")
        .iter()
        .map(|id| id.as_str().expect("visual fixture id"))
        .collect::<BTreeSet<_>>();
    assert!(fixtures.is_subset(&valid));
    assert_eq!(
        fixtures.len(),
        visual["fixtures"].as_array().expect("visual fixture array").len(),
        "visual fixture ids must be unique"
    );
}

#[test]
fn pinned_gantt_supported_corpus_matches_upstream_acceptance() {
    let corpus: Value = serde_json::from_str(GANTT_CORPUS).expect("gantt corpus must be JSON");
    assert_eq!(corpus["upstream_commit"].as_str(),
        Some("7ecca0cd7f1658ef74f4e7e91f925724ef403bbf"));
    for fixture in corpus["valid"].as_array().expect("valid corpus array") {
        let id = fixture["id"].as_str().expect("fixture id");
        let source = fixture["source"].as_str().expect("fixture source");
        parse_gantt(source).unwrap_or_else(|error| panic!("valid upstream fixture {id} failed: {error}"));
    }
    for fixture in corpus["invalid"].as_array().expect("invalid corpus array") {
        let id = fixture["id"].as_str().expect("fixture id");
        let source = fixture["source"].as_str().expect("fixture source");
        assert!(parse_gantt(source).is_err(), "invalid upstream fixture {id} unexpectedly parsed");
    }
}

#[test]
fn pinned_gantt_visual_corpus_covers_every_valid_fixture() {
    let syntax: Value = serde_json::from_str(GANTT_CORPUS).expect("Gantt corpus must be JSON");
    let visual: Value =
        serde_json::from_str(GANTT_VISUAL_CORPUS).expect("Gantt visual corpus must be JSON");
    assert_eq!(visual["upstream_commit"], syntax["upstream_commit"]);

    let valid = syntax["valid"].as_array().expect("valid corpus array");
    let expected = valid
        .iter()
        .map(|fixture| fixture["id"].as_str().expect("fixture id"))
        .collect::<BTreeSet<_>>();
    let actual = visual["fixtures"]
        .as_array()
        .expect("visual fixture array")
        .iter()
        .map(|id| id.as_str().expect("visual fixture id"))
        .collect::<BTreeSet<_>>();
    assert_eq!(actual, expected);
    assert_eq!(actual.len(), valid.len(), "visual fixture ids must be unique");
}

#[test]
fn gantt_full_status_is_backed_by_pinned_syntax_and_visual_corpora() {
    let manifest: Value =
        serde_json::from_str(COMPATIBILITY_MANIFEST).expect("compatibility manifest must be JSON");
    let gantt = manifest["families"]
        .as_array()
        .expect("families array")
        .iter()
        .find(|family| family["id"] == "gantt")
        .expect("gantt family");
    assert_eq!(gantt["status"].as_str(), Some("full"));

    let syntax: Value = serde_json::from_str(GANTT_CORPUS).expect("Gantt corpus must be JSON");
    let visual: Value =
        serde_json::from_str(GANTT_VISUAL_CORPUS).expect("Gantt visual corpus must be JSON");
    assert_eq!(syntax["upstream_commit"], visual["upstream_commit"]);
    assert!(!syntax["valid"].as_array().expect("valid corpus").is_empty());
    assert!(!syntax["invalid"].as_array().expect("invalid corpus").is_empty());
}

#[test]
fn xychart_full_status_is_backed_by_the_pinned_corpus() {
    let manifest: Value =
        serde_json::from_str(COMPATIBILITY_MANIFEST).expect("compatibility manifest must be JSON");
    let xychart = manifest["families"]
        .as_array()
        .expect("families array")
        .iter()
        .find(|family| family["id"] == "xychart")
        .expect("xychart family");
    assert_eq!(xychart["status"].as_str(), Some("full"));

    let corpus: Value = serde_json::from_str(XYCHART_CORPUS).expect("xychart corpus must be JSON");
    assert!(!corpus["valid"].as_array().expect("valid corpus").is_empty());
    assert!(!corpus["invalid"]
        .as_array()
        .expect("invalid corpus")
        .is_empty());
}

#[test]
fn pinned_xychart_corpus_matches_upstream_acceptance() {
    let corpus: Value = serde_json::from_str(XYCHART_CORPUS).expect("xychart corpus must be JSON");
    assert_eq!(
        corpus["upstream_commit"].as_str(),
        Some("7ecca0cd7f1658ef74f4e7e91f925724ef403bbf")
    );
    for fixture in corpus["valid"].as_array().expect("valid corpus array") {
        let id = fixture["id"].as_str().expect("fixture id");
        let source = fixture["source"].as_str().expect("fixture source");
        parse_xychart(source)
            .unwrap_or_else(|error| panic!("valid upstream fixture {id} failed: {error}"));
    }
    for fixture in corpus["invalid"].as_array().expect("invalid corpus array") {
        let id = fixture["id"].as_str().expect("fixture id");
        let source = fixture["source"].as_str().expect("fixture source");
        assert!(
            parse_xychart(source).is_err(),
            "invalid upstream fixture {id} unexpectedly parsed"
        );
    }
}

#[test]
fn sankey_full_status_is_backed_by_the_pinned_corpus() {
    let manifest: Value =
        serde_json::from_str(COMPATIBILITY_MANIFEST).expect("compatibility manifest must be JSON");
    let sankey = manifest["families"]
        .as_array()
        .expect("families array")
        .iter()
        .find(|family| family["id"] == "sankey")
        .expect("sankey family");
    assert_eq!(sankey["status"].as_str(), Some("full"));

    let corpus: Value = serde_json::from_str(SANKEY_CORPUS).expect("sankey corpus must be JSON");
    assert!(!corpus["valid"].as_array().expect("valid corpus").is_empty());
    assert!(!corpus["invalid"]
        .as_array()
        .expect("invalid corpus")
        .is_empty());
}

#[test]
fn pinned_sankey_corpus_matches_upstream_acceptance() {
    let corpus: Value = serde_json::from_str(SANKEY_CORPUS).expect("sankey corpus must be JSON");
    assert_eq!(
        corpus["upstream_commit"].as_str(),
        Some("7ecca0cd7f1658ef74f4e7e91f925724ef403bbf")
    );
    for fixture in corpus["valid"].as_array().expect("valid corpus array") {
        let id = fixture["id"].as_str().expect("fixture id");
        let source = fixture["source"].as_str().expect("fixture source");
        parse_sankey(source)
            .unwrap_or_else(|error| panic!("valid upstream fixture {id} failed: {error}"));
    }
    for fixture in corpus["invalid"].as_array().expect("invalid corpus array") {
        let id = fixture["id"].as_str().expect("fixture id");
        let source = fixture["source"].as_str().expect("fixture source");
        assert!(
            parse_sankey(source).is_err(),
            "invalid upstream fixture {id} unexpectedly parsed"
        );
    }
}

#[test]
fn pie_full_status_is_backed_by_the_pinned_corpus() {
    let manifest: Value =
        serde_json::from_str(COMPATIBILITY_MANIFEST).expect("compatibility manifest must be JSON");
    let pie = manifest["families"]
        .as_array()
        .expect("families array")
        .iter()
        .find(|family| family["id"] == "pie")
        .expect("pie family");
    assert_eq!(pie["status"].as_str(), Some("full"));

    let corpus: Value = serde_json::from_str(PIE_CORPUS).expect("pie corpus must be JSON");
    assert!(!corpus["valid"].as_array().expect("valid corpus").is_empty());
    assert!(!corpus["invalid"]
        .as_array()
        .expect("invalid corpus")
        .is_empty());
}

#[test]
fn pinned_pie_corpus_matches_upstream_acceptance() {
    let corpus: Value = serde_json::from_str(PIE_CORPUS).expect("pie corpus must be JSON");
    assert_eq!(
        corpus["upstream_commit"].as_str(),
        Some("7ecca0cd7f1658ef74f4e7e91f925724ef403bbf")
    );
    for fixture in corpus["valid"].as_array().expect("valid corpus array") {
        let id = fixture["id"].as_str().expect("fixture id");
        let source = fixture["source"].as_str().expect("fixture source");
        parse_pie(source)
            .unwrap_or_else(|error| panic!("valid upstream fixture {id} failed: {error}"));
    }
    for fixture in corpus["invalid"].as_array().expect("invalid corpus array") {
        let id = fixture["id"].as_str().expect("fixture id");
        let source = fixture["source"].as_str().expect("fixture source");
        assert!(
            parse_pie(source).is_err(),
            "invalid upstream fixture {id} unexpectedly parsed"
        );
    }
}

#[test]
fn gitgraph_full_status_is_backed_by_the_pinned_corpus() {
    let manifest: Value =
        serde_json::from_str(COMPATIBILITY_MANIFEST).expect("compatibility manifest must be JSON");
    let gitgraph = manifest["families"]
        .as_array()
        .expect("families array")
        .iter()
        .find(|family| family["id"] == "gitgraph")
        .expect("gitgraph family");
    assert_eq!(gitgraph["status"].as_str(), Some("full"));

    let corpus: Value =
        serde_json::from_str(GITGRAPH_CORPUS).expect("gitgraph corpus must be JSON");
    assert!(!corpus["valid"].as_array().expect("valid corpus").is_empty());
    assert!(!corpus["invalid"]
        .as_array()
        .expect("invalid corpus")
        .is_empty());
}

#[test]
fn pinned_gitgraph_corpus_matches_upstream_acceptance() {
    let corpus: Value =
        serde_json::from_str(GITGRAPH_CORPUS).expect("gitgraph corpus must be JSON");
    assert_eq!(
        corpus["upstream_commit"].as_str(),
        Some("7ecca0cd7f1658ef74f4e7e91f925724ef403bbf")
    );
    for fixture in corpus["valid"].as_array().expect("valid corpus array") {
        let id = fixture["id"].as_str().expect("fixture id");
        let source = fixture["source"].as_str().expect("fixture source");
        parse_gitgraph(source)
            .unwrap_or_else(|error| panic!("valid upstream fixture {id} failed: {error}"));
    }
    for fixture in corpus["invalid"].as_array().expect("invalid corpus array") {
        let id = fixture["id"].as_str().expect("fixture id");
        let source = fixture["source"].as_str().expect("fixture source");
        assert!(
            parse_gitgraph(source).is_err(),
            "invalid upstream fixture {id} unexpectedly parsed"
        );
    }
}

#[test]
fn requirement_full_status_is_backed_by_the_pinned_corpus() {
    let manifest: Value =
        serde_json::from_str(COMPATIBILITY_MANIFEST).expect("compatibility manifest must be JSON");
    let requirement = manifest["families"]
        .as_array()
        .expect("families array")
        .iter()
        .find(|family| family["id"] == "requirement")
        .expect("requirement family");
    assert_eq!(requirement["status"].as_str(), Some("full"));

    let corpus: Value =
        serde_json::from_str(REQUIREMENT_CORPUS).expect("requirement corpus must be JSON");
    assert!(!corpus["valid"].as_array().expect("valid corpus").is_empty());
    assert!(!corpus["invalid"].as_array().expect("invalid corpus").is_empty());
}

#[test]
fn pinned_requirement_corpus_matches_upstream_acceptance() {
    let corpus: Value =
        serde_json::from_str(REQUIREMENT_CORPUS).expect("requirement corpus must be JSON");
    assert_eq!(
        corpus["upstream_commit"].as_str(),
        Some("7ecca0cd7f1658ef74f4e7e91f925724ef403bbf")
    );
    for fixture in corpus["valid"].as_array().expect("valid corpus array") {
        let id = fixture["id"].as_str().expect("fixture id");
        let source = fixture["source"].as_str().expect("fixture source");
        parse_requirement_diagram(source)
            .unwrap_or_else(|error| panic!("valid upstream fixture {id} failed: {error}"));
    }
    for fixture in corpus["invalid"].as_array().expect("invalid corpus array") {
        let id = fixture["id"].as_str().expect("fixture id");
        let source = fixture["source"].as_str().expect("fixture source");
        assert!(
            parse_requirement_diagram(source).is_err(),
            "invalid upstream fixture {id} unexpectedly parsed"
        );
    }
}

#[test]
fn journey_full_status_is_backed_by_the_pinned_corpus() {
    let manifest: Value =
        serde_json::from_str(COMPATIBILITY_MANIFEST).expect("compatibility manifest must be JSON");
    let journey = manifest["families"]
        .as_array()
        .expect("families array")
        .iter()
        .find(|family| family["id"] == "journey")
        .expect("journey family");
    assert_eq!(journey["status"].as_str(), Some("full"));

    let corpus: Value = serde_json::from_str(JOURNEY_CORPUS).expect("journey corpus must be JSON");
    assert!(!corpus["valid"].as_array().expect("valid corpus").is_empty());
    assert!(!corpus["invalid"].as_array().expect("invalid corpus").is_empty());
}

#[test]
fn pinned_journey_corpus_matches_upstream_acceptance() {
    let corpus: Value = serde_json::from_str(JOURNEY_CORPUS).expect("journey corpus must be JSON");
    assert_eq!(
        corpus["upstream_commit"].as_str(),
        Some("7ecca0cd7f1658ef74f4e7e91f925724ef403bbf")
    );
    for fixture in corpus["valid"].as_array().expect("valid corpus array") {
        let id = fixture["id"].as_str().expect("fixture id");
        let source = fixture["source"].as_str().expect("fixture source");
        parse_journey(source)
            .unwrap_or_else(|error| panic!("valid upstream fixture {id} failed: {error}"));
    }
    for fixture in corpus["invalid"].as_array().expect("invalid corpus array") {
        let id = fixture["id"].as_str().expect("fixture id");
        let source = fixture["source"].as_str().expect("fixture source");
        assert!(
            parse_journey(source).is_err(),
            "invalid upstream fixture {id} unexpectedly parsed"
        );
    }
}

#[test]
fn pinned_quadrant_corpus_matches_upstream_acceptance() {
    let corpus: Value =
        serde_json::from_str(QUADRANT_CORPUS).expect("quadrant corpus must be JSON");
    assert_eq!(
        corpus["upstream_commit"].as_str(),
        Some("7ecca0cd7f1658ef74f4e7e91f925724ef403bbf")
    );
    for fixture in corpus["valid"].as_array().expect("valid corpus array") {
        let id = fixture["id"].as_str().expect("fixture id");
        let source = fixture["source"].as_str().expect("fixture source");
        parse_quadrant_chart(source)
            .unwrap_or_else(|error| panic!("valid upstream fixture {id} failed: {error}"));
    }
    for fixture in corpus["invalid"].as_array().expect("invalid corpus array") {
        let id = fixture["id"].as_str().expect("fixture id");
        let source = fixture["source"].as_str().expect("fixture source");
        assert!(
            parse_quadrant_chart(source).is_err(),
            "invalid upstream fixture {id} unexpectedly parsed"
        );
    }
}

#[test]
fn compatibility_manifest_matches_detector_and_dispatch() {
    let manifest: Value =
        serde_json::from_str(COMPATIBILITY_MANIFEST).expect("compatibility manifest must be JSON");
    assert_eq!(
        manifest["upstream"]["version"].as_str(),
        Some(MERMAID_COMPATIBILITY_BASELINE)
    );

    let families = manifest["families"]
        .as_array()
        .expect("families must be an array");
    assert!(
        families.len() >= 32,
        "the Mermaid 11.16.1 baseline should enumerate every documented family"
    );

    for family in families {
        let id = family["id"].as_str().expect("family id must be a string");
        let smoke_source = family["smoke_source"]
            .as_str()
            .expect("family smoke_source must be a string");
        let detected = detect_mermaid_type(smoke_source)
            .unwrap_or_else(|error| panic!("failed to detect {id}: {error}"));
        assert_eq!(detected.canonical_id(), id);

        if matches!(family["status"].as_str(), Some("partial" | "full")) {
            parse_any_mermaid(smoke_source)
                .unwrap_or_else(|error| panic!("native family {id} must parse: {error}"));
            assert!(detected.has_native_pipeline());
        }
    }
}

#[test]
fn detection_skips_front_matter_directives_and_comments() {
    let source = r#"---
title: Example
---
%%{init: {"theme": "neutral"}}%%
%% comment
sequenceDiagram
Alice->>Bob: Hello
"#;

    let detected = detect_mermaid_type(source).expect("sequence diagram should be detected");
    assert_eq!(detected.canonical_id(), "sequence");
}

#[test]
fn radar_dispatches_to_chart_ir() {
    let diagram = parse_any_mermaid("radar-beta\naxis A, B, C\ncurve x{1, 2, 3}")
        .expect("radar subset should parse");
    match diagram {
        mermaid_parser::MermaidDiagram::Chart(chart) => assert_eq!(chart.series[0].data.len(), 3),
        _ => panic!("radar should lower to chart IR"),
    }
}

#[test]
fn event_modeling_dispatches_to_semantic_ir() {
    let diagram = parse_any_mermaid(
        "eventmodeling\ntf 01 ui CartUI\ntf 02 cmd AddItem ->> 01\n",
    )
    .expect("event modeling subset should parse");
    match diagram {
        mermaid_parser::MermaidDiagram::EventModel(diagram) => {
            assert_eq!(diagram.frames.len(), 2);
            assert_eq!(diagram.frames[1].source_frames, ["01"]);
        }
        _ => panic!("event modeling should lower to dedicated semantic IR"),
    }
}

#[test]
fn treemap_dispatches_to_hierarchy_ir() {
    let diagram = parse_any_mermaid("treemap-beta\n\"Root\"\n  \"Child\": 10")
        .expect("treemap subset should parse");
    match diagram {
        mermaid_parser::MermaidDiagram::Treemap(diagram) => {
            assert_eq!(diagram.nodes.len(), 2);
            assert_eq!(diagram.nodes[1].parent_id.as_deref(), Some("treemap-1"));
        }
        _ => panic!("treemap should lower to dedicated hierarchy IR"),
    }
}

#[test]
fn venn_dispatches_to_dedicated_set_ir_and_validates_unions() {
    let diagram = parse_any_mermaid("venn-beta\nset A[\"Alpha\"]:20\nset B\nunion A,B[\"AB\"]:3").expect("venn subset should parse");
    match diagram {
        mermaid_parser::MermaidDiagram::Venn(diagram) => {
            assert_eq!(diagram.regions.len(), 3);
            assert_eq!(diagram.regions[2].sets, ["A", "B"]);
        }
        _ => panic!("venn should lower to dedicated set IR"),
    }
    assert!(parse_venn("venn-beta\nset A\nunion A,Missing").is_err());
    let styled = parse_venn("venn-beta\nset A\nstyle A fill:rgba(255, 0, 128, 0.5), stroke-width:3px").unwrap();
    assert_eq!(styled.regions[0].style.fill.as_deref(), Some("rgba(255, 0, 128, 0.5)"));
    assert_eq!(styled.regions[0].style.stroke_width, Some(3.0));
}

#[test]
fn ishikawa_dispatches_to_dedicated_causal_tree_ir() {
    let diagram = parse_any_mermaid("ishikawa\nFailure\n  People\n    Training\n  Process")
        .expect("ishikawa subset should parse");
    match diagram {
        mermaid_parser::MermaidDiagram::Ishikawa(diagram) => {
            assert_eq!(diagram.effect, "Failure");
            assert_eq!(diagram.causes.len(), 3);
            assert_eq!(diagram.causes[1].parent_id.as_deref(), Some("cause-1"));
            assert_eq!(diagram.causes[1].depth, 2);
        }
        _ => panic!("ishikawa should lower to dedicated causal tree IR"),
    }
    let indented_effect = parse_ishikawa("ishikawa-beta\n    Failure\nPeople\n  Training").unwrap();
    assert_eq!(indented_effect.causes[0].parent_id, None);
    assert_eq!(indented_effect.causes[1].parent_id.as_deref(), Some("cause-1"));

    let configured = parse_ishikawa(
        "---\nconfig:\n  ishikawa:\n    diagramPadding: 52\n---\nishikawa\nFailure\n  People",
    )
    .unwrap();
    assert_eq!(configured.diagram_padding, 52.0);
}

#[test]
fn wardley_dispatches_to_dedicated_strategic_map_ir() {
    let diagram = parse_any_mermaid("wardley-beta\nanchor User [0.9, 0.8]\ncomponent API [0.6, 0.5]\nUser -> API\nevolve API 0.75")
        .expect("wardley subset should parse");
    match diagram {
        mermaid_parser::MermaidDiagram::Wardley(diagram) => {
            assert_eq!(diagram.nodes.len(), 2); assert!(diagram.nodes[0].anchor);
            assert_eq!(diagram.links[0].source, "User"); assert_eq!(diagram.evolves[0].target, 0.75);
        }
        _ => panic!("wardley should lower to dedicated strategic map IR"),
    }
    assert!(parse_wardley("wardley-beta\ncomponent API [0.6, 0.5]\nAPI -> Missing").is_err());
}

#[test]
fn cynefin_dispatches_to_dedicated_domain_map_ir() {
    let diagram = parse_any_mermaid("cynefin-beta:\ncomplex\n  \"Probe\"\nclear\n  \"Known fix\"\ncomplex --> clear : \"Codified\"\nclear --> clear")
        .expect("cynefin subset should parse");
    match diagram {
        mermaid_parser::MermaidDiagram::Cynefin(diagram) => {
            assert_eq!(diagram.domains[0].items, ["Probe"]); assert_eq!(diagram.transitions.len(), 1);
            assert_eq!(diagram.transitions[0].label.as_deref(), Some("Codified"));
        }
        _ => panic!("cynefin should lower to dedicated domain-map IR"),
    }
    assert!(parse_cynefin("cynefin-beta\n\"orphan item\"").is_err());
}

#[test]
fn treeview_dispatches_to_dedicated_tree_ir() {
    let diagram = parse_any_mermaid("treeView-beta\nproject/ :::highlight\n  App.tsx icon(logos:react) ## main component")
        .expect("treeview subset should parse");
    match diagram {
        mermaid_parser::MermaidDiagram::TreeView(diagram) => {
            assert_eq!(diagram.nodes.len(), 3);
            assert_eq!(diagram.nodes[0].label, "/");
            assert_eq!(diagram.nodes[1].parent_id.as_deref(), Some("treeview-root"));
            assert_eq!(diagram.nodes[2].parent_id.as_deref(), Some("treeview-1"));
            assert_eq!(diagram.nodes[2].icon.as_deref(), Some("logos:react"));
            assert_eq!(diagram.nodes[2].description.as_deref(), Some("main component"));
        }
        _ => panic!("treeview should lower to dedicated tree IR"),
    }
    assert!(parse_treeview("treeView-beta\n\"unterminated").is_err());
}

#[test]
fn swimlane_dispatches_to_dedicated_ownership_ir() {
    let source = "swimlane-beta LR\ntitle Support flow\naccTitle: Support handoff\nsubgraph customer [Customer]\n  request([Request])\n  receive((Update))\nend\nsubgraph support [Support]\n  triage{Known?}\n  answer[Answer]\nend\nrequest --> triage\ntriage -->|Yes| answer ==> receive";
    let diagram = parse_any_mermaid(source).expect("swimlane subset should parse");
    match diagram {
        mermaid_parser::MermaidDiagram::Swimlane(diagram) => {
            assert_eq!(diagram.lanes.len(), 2); assert_eq!(diagram.lanes[0].node_ids, ["request", "receive"]);
            assert_eq!(diagram.nodes.len(), 4); assert_eq!(diagram.edges.len(), 3);
            assert_eq!(diagram.edges[1].label.as_deref(), Some("Yes"));
            assert_eq!(diagram.edges[2].kind, diagram_ir::SwimlaneEdgeKind::Thick);
        }
        _ => panic!("swimlane should lower to dedicated ownership IR"),
    }
    assert!(parse_swimlane("swimlane-beta\nsubgraph A\n  one[One]").is_err());
}

#[test]
fn pinned_swimlane_subset_corpus_parses_to_ownership_ir() {
    let corpus: Value = serde_json::from_str(SWIMLANE_CORPUS).expect("swimlane corpus must be JSON");
    for fixture in corpus["fixtures"].as_array().expect("fixtures must be an array") {
        let source = fixture["source"].as_str().expect("fixture source must be a string");
        assert!(parse_swimlane(source).is_ok(), "fixture {:?} should parse", fixture["name"]);
    }
}

#[test]
fn pinned_swimlane_visual_corpus_references_syntax_fixtures() {
    let syntax: Value = serde_json::from_str(SWIMLANE_CORPUS).expect("swimlane corpus must be JSON");
    let visual: Value = serde_json::from_str(SWIMLANE_VISUAL_CORPUS).expect("swimlane visual corpus must be JSON");
    assert_eq!(syntax["upstream_commit"], visual["upstream_commit"]);
    let syntax_names = syntax["fixtures"].as_array().expect("fixtures must be an array").iter()
        .map(|fixture| fixture["name"].as_str().expect("fixture name")).collect::<BTreeSet<_>>();
    let visual_names = visual["fixtures"].as_array().expect("visual fixtures must be an array").iter()
        .map(|fixture| fixture.as_str().expect("visual fixture name")).collect::<BTreeSet<_>>();
    assert_eq!(visual_names.len(), visual["fixtures"].as_array().expect("visual fixtures").len());
    assert!(visual_names.is_subset(&syntax_names));
}

#[test]
fn swimlane_parallel_endpoints_lower_to_individual_handoffs() {
    let diagram = parse_swimlane(
        "swimlane-beta LR\nsubgraph Review\n  decide{Approved?}\n  revise[Revise]\nend\nsubgraph Delivery\n  ship[Ship]\n  notify[Notify]\nend\ndecide --> ship & notify\nship & notify --> revise",
    ).expect("parallel Swimlane endpoints should parse");

    assert_eq!(diagram.edges.len(), 4);
    assert_eq!(diagram.edges.iter().map(|edge| (edge.from.as_str(), edge.to.as_str())).collect::<Vec<_>>(), [
        ("decide", "ship"), ("decide", "notify"), ("ship", "revise"), ("notify", "revise"),
    ]);
    assert_eq!(diagram.lanes[1].node_ids, ["ship", "notify"]);
}

#[test]
fn swimlane_flowchart_style_edge_labels_lower_to_semantic_edges() {
    let diagram = parse_swimlane(
        "swimlane-beta LR\nsubgraph Intake\n  Start\n  Process1\nend\nStart --Yes --> Process1 --> Start",
    ).expect("Flowchart-style Swimlane edge labels should parse");

    assert_eq!(diagram.edges.len(), 2);
    assert_eq!(diagram.edges[0].label.as_deref(), Some("Yes"));
    assert_eq!(diagram.edges[1].label, None);
}

#[test]
fn swimlane_multiline_edges_continue_from_the_previous_endpoint() {
    let diagram = parse_swimlane(
        "swimlane-beta TD\nsubgraph Intake\n  Start\nend\nsubgraph Delivery\n  Validate\n  Finish\nend\nStart --> Validate\n  --> Finish",
    ).expect("multiline Swimlane edges should parse");

    assert_eq!(diagram.edges.len(), 2);
    assert_eq!((diagram.edges[1].from.as_str(), diagram.edges[1].to.as_str()), ("Validate", "Finish"));
    assert!(parse_swimlane("swimlane-beta\n--> Missing\nsubgraph A\n  Start\nend").is_err());
}

#[test]
fn swimlane_storage_and_subprocess_shapes_reach_semantic_ir() {
    let diagram = parse_swimlane(
        "swimlane-beta LR\nsubgraph Data\n  source[(Database)]\n  process[[Transform]]\nend\nsource --> process",
    ).expect("storage and subprocess nodes should parse");

    assert_eq!(diagram.nodes[0].shape, diagram_ir::DiagramShape::Cylinder);
    assert_eq!(diagram.nodes[1].shape, diagram_ir::DiagramShape::Subroutine);
}

#[test]
fn swimlane_hexagon_and_double_circle_shapes_reach_semantic_ir() {
    let diagram = parse_swimlane(
        "swimlane-beta LR\nsubgraph Review\n  decide{{Evaluate}}\n  finish(((Done)))\nend\ndecide --> finish",
    ).expect("hexagon and double-circle nodes should parse");

    assert_eq!(diagram.nodes[0].shape, diagram_ir::DiagramShape::Hexagon);
    assert_eq!(diagram.nodes[1].shape, diagram_ir::DiagramShape::DoubleCircle);
}

#[test]
fn swimlane_input_output_and_asymmetric_shapes_reach_semantic_ir() {
    let diagram = parse_swimlane(
        "swimlane-beta LR\nsubgraph Intake\n  input[/Payload/]\n  flag>Review]\nend\ninput --> flag",
    ).expect("input/output and asymmetric nodes should parse");

    assert_eq!(diagram.nodes[0].shape, diagram_ir::DiagramShape::ParallelogramRight);
    assert_eq!(diagram.nodes[1].shape, diagram_ir::DiagramShape::Asymmetric);
}

#[test]
fn swimlane_sloped_flowchart_shapes_reach_semantic_ir() {
    let diagram = parse_swimlane(
        r#"swimlane-beta LR
subgraph Shapes
  left[\Inbound\]
  wide[/Expand\]
  narrow[\Contract/]
end
left --> wide --> narrow"#,
    ).expect("sloped Flowchart nodes should parse");

    assert_eq!(diagram.nodes[0].shape, diagram_ir::DiagramShape::ParallelogramLeft);
    assert_eq!(diagram.nodes[1].shape, diagram_ir::DiagramShape::Trapezoid);
    assert_eq!(diagram.nodes[2].shape, diagram_ir::DiagramShape::InvertedTrapezoid);
}

#[test]
fn swimlane_quoted_node_labels_normalize_into_semantic_ir() {
    let diagram = parse_swimlane(
        r#"swimlane-beta LR
subgraph Intake
  request["Request \"priority\" service"]
  done(("Complete ✓"))
end
request --> done"#,
    ).expect("quoted Swimlane labels should parse");

    assert_eq!(diagram.nodes[0].label, "Request \"priority\" service");
    assert_eq!(diagram.nodes[1].label, "Complete ✓");
}

#[test]
fn swimlane_quoted_lane_labels_normalize_into_semantic_ir() {
    let diagram = parse_swimlane(
        r#"swimlane-beta LR
subgraph intake["Request \"priority\" team ✓"]
  request[Request]
end
subgraph "Completion \"crew\""
  done[Done]
end
request --> done"#,
    ).expect("quoted Swimlane lane labels should parse");

    assert_eq!(diagram.lanes[0].id, "intake");
    assert_eq!(diagram.lanes[0].label, "Request \"priority\" team ✓");
    assert_eq!(diagram.lanes[1].id, "swimlane-2");
    assert_eq!(diagram.lanes[1].label, "Completion \"crew\"");
}

#[test]
fn swimlane_quoted_edge_labels_normalize_into_semantic_ir() {
    let diagram = parse_swimlane(
        r#"swimlane-beta LR
subgraph Intake
  request[Request]
  done[Done]
end
request -->|"Priority \"A\""| done"#,
    ).expect("quoted Swimlane edge labels should parse");

    assert_eq!(diagram.edges[0].label.as_deref(), Some("Priority \"A\""));
}

#[test]
fn swimlane_label_entities_decode_into_semantic_ir() {
    let diagram = parse_swimlane(
        "swimlane-beta LR\nsubgraph intake[Request &amp; Review]\n  request[Input&nbsp;#1]\nend\nsubgraph done[Completion]\n  finish[Done]\nend\nrequest -->|Ready&#32;&check;| finish",
    ).expect("Swimlane label entities should parse");

    assert_eq!(diagram.lanes[0].label, "Request & Review");
    assert_eq!(diagram.nodes[0].label, "Input\u{a0}#1");
    assert_eq!(diagram.edges[0].label.as_deref(), Some("Ready ✓"));
}

#[test]
fn swimlane_metadata_entities_decode_into_semantic_ir() {
    let diagram = parse_swimlane(
        "swimlane-beta LR\ntitle Request &amp; Delivery\naccTitle: Accessible&#32;handoff\naccDescr: Intake &lt; Support\nsubgraph Intake\n  request[Request]\nend\nsubgraph Support\n  done[Done]\nend\nrequest --> done",
    ).expect("Swimlane metadata entities should parse");

    assert_eq!(diagram.title.as_deref(), Some("Request & Delivery"));
    assert_eq!(diagram.accessibility_title.as_deref(), Some("Accessible handoff"));
    assert_eq!(diagram.accessibility_description.as_deref(), Some("Intake < Support"));
}

#[test]
fn swimlane_inline_node_styles_lower_to_semantic_ir() {
    let diagram = parse_swimlane(
        "swimlane-beta LR\nsubgraph Intake\n  request[Request]\nend\nsubgraph Support\n  done[Done]\nend\nstyle request fill:#fef3c7,stroke:#b45309,color:#78350f,stroke-width:4px,stroke-dasharray:5 3\nrequest --> done",
    ).expect("Swimlane inline node styles should parse");

    let style = &diagram.nodes[0].style;
    assert_eq!(style.fill.as_deref(), Some("#fef3c7"));
    assert_eq!(style.stroke.as_deref(), Some("#b45309"));
    assert_eq!(style.text_color.as_deref(), Some("#78350f"));
    assert_eq!(style.stroke_width, Some(4.0));
    assert_eq!(style.stroke_dash.as_deref(), Some([5.0, 3.0].as_slice()));
    assert!(parse_swimlane("swimlane-beta\nsubgraph A\n  one[One]\nend\nstyle missing fill:red").is_err());
}

#[test]
fn swimlane_named_classes_resolve_into_semantic_styles() {
    let diagram = parse_swimlane(
        "swimlane-beta LR\nsubgraph Intake\n  request[Request]\nend\nsubgraph Support\n  done[Done]\nend\nclass request accent\nclassDef default fill:#f8fafc,stroke:#64748b\nclassDef accent fill:#dbeafe,stroke:#1d4ed8,color:#172554,stroke-width:3px\nstyle request fill:#dcfce7\nrequest --> done",
    ).expect("Swimlane named classes should parse");

    assert_eq!(diagram.nodes[0].style.fill.as_deref(), Some("#dcfce7"));
    assert_eq!(diagram.nodes[0].style.stroke.as_deref(), Some("#1d4ed8"));
    assert_eq!(diagram.nodes[0].style.stroke_width, Some(3.0));
    assert_eq!(diagram.nodes[1].style.fill.as_deref(), Some("#f8fafc"));
    assert!(parse_swimlane("swimlane-beta\nsubgraph A\n  one[One]\nend\nclass one missing").is_err());
}

#[test]
fn swimlane_flowchart_endpoint_markers_lower_to_semantic_ir() {
    use diagram_ir::EdgeMarker;

    let diagram = parse_swimlane(
        "swimlane-beta LR\nsubgraph Review\n  open[Open]\n  check[Check]\n  close[Close]\nend\nopen <--> check\ncheck o--x close\nclose x--> open\nopen --x close",
    ).expect("Flowchart endpoint markers should parse");

    assert_eq!((diagram.edges[0].start_marker, diagram.edges[0].end_marker), (EdgeMarker::Point, EdgeMarker::Point));
    assert_eq!((diagram.edges[1].start_marker, diagram.edges[1].end_marker), (EdgeMarker::Circle, EdgeMarker::Cross));
    assert_eq!((diagram.edges[2].start_marker, diagram.edges[2].end_marker), (EdgeMarker::Cross, EdgeMarker::Point));
    assert_eq!((diagram.edges[3].start_marker, diagram.edges[3].end_marker), (EdgeMarker::None, EdgeMarker::Cross));
}

#[test]
fn railroad_dispatches_all_notations_to_recursive_ir() {
    let source = "railroad-beta\ntitle Number\ndigit = choice(terminal(\"0\"), terminal(\"1\"));\nnumber = oneOrMore(nonterminal(\"digit\"));";
    let diagram = parse_any_mermaid(source).expect("railroad constructor notation should parse");
    match diagram {
        mermaid_parser::MermaidDiagram::Railroad(diagram) => {
            assert_eq!(diagram.rules.len(), 2);
            assert_eq!(diagram.rules[1].name, "number");
            assert!(matches!(diagram.rules[1].definition, diagram_ir::RailroadExpression::Repetition { min: 1, .. }));
        }
        _ => panic!("railroad should lower to dedicated recursive IR"),
    }
    let ebnf = parse_railroad("railroad-ebnf-beta\ndigit = '0' | '1';\nnumber = digit+;")
        .expect("railroad EBNF notation should parse");
    assert!(matches!(ebnf.rules[0].definition, diagram_ir::RailroadExpression::Choice(_)));
    assert!(matches!(ebnf.rules[1].definition, diagram_ir::RailroadExpression::Repetition { min: 1, .. }));
    let abnf = parse_railroad("railroad-abnf-beta\ndigits = 2*4DIGIT;")
        .expect("railroad ABNF notation should parse");
    assert!(matches!(
        abnf.rules[0].definition,
        diagram_ir::RailroadExpression::Repetition { min: 2, max: Some(4), .. }
    ));
    let peg = parse_railroad("railroad-peg-beta\nvalue <- !\"x\" item+;")
        .expect("railroad PEG notation should parse");
    let diagram_ir::RailroadExpression::Sequence(elements) = &peg.rules[0].definition else {
        panic!("PEG prefix and suffix should lower to a sequence");
    };
    assert!(matches!(&elements[0], diagram_ir::RailroadExpression::Special(label) if label == "!\"x\""));
    assert!(matches!(elements[1], diagram_ir::RailroadExpression::Repetition { min: 1, max: None, .. }));
    assert!(parse_railroad("railroad-beta\nvalue = optional(terminal(\"x\"), terminal(\"y\"));").is_err());
}

#[test]
fn info_dispatches_to_dedicated_version_ir() {
    match parse_any_mermaid("info showInfo").expect("info should parse") {
        mermaid_parser::MermaidDiagram::Info(diagram) => assert_eq!(diagram.version, "11.16.1"),
        _ => panic!("info should lower to dedicated version IR"),
    }
}

#[test]
fn zenuml_dispatches_to_shared_sequence_ir() {
    match parse_any_mermaid("zenuml\nA as Alice\nA->Bob: Hello").expect("zenuml should parse") {
        mermaid_parser::MermaidDiagram::Sequence(diagram) => {
            assert_eq!(diagram.participants[0].label.text, "Alice");
            assert_eq!(diagram.events.len(), 1);
        }
        _ => panic!("zenuml should lower to shared sequence IR"),
    }
}
