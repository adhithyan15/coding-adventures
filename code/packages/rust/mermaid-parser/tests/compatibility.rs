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
        if name == "basic" {
            assert_eq!(diagram.notes.len(), 1);
            assert_eq!(diagram.notes[0].text, "Standardising power accelerates evolution");
            assert_eq!((diagram.notes[0].visibility, diagram.notes[0].evolution), (0.30, 0.49));
        }
        if name == "renderer-config" {
            assert_eq!((diagram.config.width, diagram.config.height, diagram.config.padding), (760.0, 480.0, 36.0));
            assert_eq!((diagram.config.node_radius, diagram.config.node_label_offset), (9.0, 12.0));
            assert_eq!((diagram.config.axis_font_size, diagram.config.label_font_size), (15.0, 13.0));
            assert!(diagram.config.show_grid); assert!(!diagram.config.use_max_width);
            assert_eq!((&diagram.config.theme.background_color, &diagram.config.theme.axis_color,
                &diagram.config.theme.component_fill, &diagram.config.theme.link_stroke),
                (&"#f8fafc".to_string(), &"#123456".to_string(), &"#abcdef".to_string(), &"#6789ab".to_string()));
        }
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
            assert_eq!(diagram.config.diagram_padding, 64.0);
            assert!(diagram.config.use_max_width);
            assert_eq!(diagram.config.font_family, "Georgia");
            assert_eq!(diagram.config.line_color, "#123456");
            assert_eq!(diagram.config.fill_color, "#abcdef");
            assert_eq!(diagram.config.text_color, "#654321");
        }
        if name == "deep-long-labels" {
            assert_eq!(diagram.causes.iter().map(|cause| cause.depth).max(), Some(3));
            assert!(diagram.causes.iter().any(|cause| cause.label == "Legal approval queues delay publication"));
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
    let configured = parse_venn("---\nthemeVariables:\n  fontFamily: Georgia\n  vennTitleTextColor: '#123456'\n  vennSetTextColor: '#654321'\nconfig:\n  venn:\n    width: 640\n    height: 360\n    padding: 24\n    useMaxWidth: false\n    useDebugLayout: true\n---\nvenn-beta\nset A").unwrap();
    assert_eq!((configured.config.width, configured.config.height, configured.config.padding), (640.0, 360.0, 24.0));
    assert!(!configured.config.use_max_width);
    assert!(configured.config.use_debug_layout);
    assert_eq!((&configured.config.font_family, &configured.config.title_text_color, &configured.config.set_text_color),
        (&"Georgia".to_string(), &"#123456".to_string(), &"#654321".to_string()));
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
        "---\nconfig:\n  ishikawa:\n    diagramPadding: 52\n    useMaxWidth: true\nthemeVariables:\n  fontFamily: Georgia\n  lineColor: '#123456'\n  mainBkg: '#abcdef'\n  textColor: '#654321'\n---\nishikawa\nFailure\n  People",
    )
    .unwrap();
    assert_eq!(configured.config.diagram_padding, 52.0);
    assert!(configured.config.use_max_width);
    assert_eq!((&configured.config.font_family, &configured.config.line_color, &configured.config.fill_color,
        &configured.config.text_color), (&"Georgia".to_string(), &"#123456".to_string(),
        &"#abcdef".to_string(), &"#654321".to_string()));
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
    assert!(parse_wardley("wardley-beta\ncomponent API [0.6, 0.5]\nnote unquoted [0.4, 0.5]").is_err());
    let sized = parse_wardley("%%{init: {\"wardley-beta\": {\"width\": 700, \"height\": 420}}}%%\nwardley-beta\nsize [640, 360]\ncomponent API [0.6, 0.5]").unwrap();
    assert_eq!((sized.config.width, sized.config.height), (640.0, 360.0));
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
fn swimlane_loose_nodes_lower_into_the_default_lane() {
    let diagram = parse_swimlane(
        "swimlane-beta LR\nsubgraph OwnedLane\n  owned[Owned node]\nend\nloose[Loose node] --> owned",
    ).expect("loose nodes should parse into a default lane");

    assert_eq!(diagram.lanes.len(), 2);
    assert_eq!(diagram.lanes[1].id, "__swimlane_default__");
    assert_eq!(diagram.lanes[1].label, "");
    assert_eq!(diagram.lanes[1].node_ids, ["loose"]);
    assert_eq!(diagram.nodes.iter().find(|node| node.id == "loose")
        .and_then(|node| node.lane_id.as_deref()), Some("__swimlane_default__"));
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
fn swimlane_cloud_and_bang_attributes_reach_semantic_ir() {
    let diagram = parse_swimlane(
        "swimlane-beta LR\nsubgraph Runtime\n  hosted@{ shape: cloud, label: \"Hosted service\" }\n  alert@{ label: \"Failure alert\", shape: bang }\nend\nhosted --> alert",
    ).expect("expanded Swimlane shapes should parse");

    assert_eq!(diagram.nodes[0].label, "Hosted service");
    assert_eq!(diagram.nodes[0].shape, diagram_ir::DiagramShape::Cloud);
    assert_eq!(diagram.nodes[1].label, "Failure alert");
    assert_eq!(diagram.nodes[1].shape, diagram_ir::DiagramShape::Bang);
    assert!(parse_swimlane("swimlane-beta\nsubgraph A\n  one@{ shape: browser }\nend").is_err());
}

#[test]
fn swimlane_classic_shape_attributes_and_aliases_reach_semantic_ir() {
    use diagram_ir::DiagramShape;

    let diagram = parse_swimlane(
        "swimlane-beta LR\nsubgraph Workflow\n  process@{ shape: rect }\n  event@{ shape: rounded }\n  choice@{ shape: question }\n  input@{ shape: in-out }\n  manual@{ shape: trap-t }\n  database@{ shape: db }\n  subprocess@{ shape: subproc }\n  stop@{ shape: double-circle }\nend\nprocess --> event --> choice --> input --> manual --> database --> subprocess --> stop",
    ).expect("classic Swimlane shape attributes should parse");

    let shapes = diagram.nodes.iter().map(|node| node.shape.clone()).collect::<Vec<_>>();
    assert_eq!(shapes, [
        DiagramShape::Rect, DiagramShape::RoundedRect, DiagramShape::Diamond,
        DiagramShape::ParallelogramRight, DiagramShape::InvertedTrapezoid,
        DiagramShape::Cylinder, DiagramShape::Subroutine, DiagramShape::DoubleCircle,
    ]);
}

#[test]
fn swimlane_hourglass_and_collate_attributes_reach_semantic_ir() {
    let diagram = parse_swimlane(
        "swimlane-beta LR\nsubgraph Merge\n  collect@{ shape: collate, label: \"Collect\" }\n  combine@{ shape: hourglass, label: \"Combine\" }\nend\ncollect --> combine",
    ).expect("Swimlane hourglass aliases should parse");

    assert_eq!(diagram.nodes[0].shape, diagram_ir::DiagramShape::Hourglass);
    assert_eq!(diagram.nodes[1].shape, diagram_ir::DiagramShape::Hourglass);
}

#[test]
fn swimlane_triangle_and_extract_attributes_reach_semantic_ir() {
    let diagram = parse_swimlane(
        "swimlane-beta LR\nsubgraph Extract\n  first@{ shape: tri, label: \"First\" }\n  second@{ shape: extract, label: \"Second\" }\n  third@{ shape: triangle, label: \"Third\" }\nend\nfirst --> second --> third",
    ).expect("Swimlane triangle aliases should parse");

    assert!(diagram.nodes.iter().all(|node| node.shape == diagram_ir::DiagramShape::Triangle));
}

#[test]
fn swimlane_flipped_triangle_and_manual_file_attributes_reach_semantic_ir() {
    let diagram = parse_swimlane(
        "swimlane-beta LR\nsubgraph Manual\n  first@{ shape: flip-tri, label: \"First\" }\n  second@{ shape: manual-file, label: \"Second\" }\nend\nfirst --> second",
    ).expect("Swimlane flipped-triangle aliases should parse");

    assert!(diagram.nodes.iter().all(|node| node.shape == diagram_ir::DiagramShape::InvertedTriangle));
}

#[test]
fn swimlane_notched_rectangle_and_card_attributes_reach_semantic_ir() {
    let diagram = parse_swimlane(
        "swimlane-beta LR\nsubgraph Cards\n  first@{ shape: notch-rect, label: \"First\" }\n  second@{ shape: card, label: \"Second\" }\n  third@{ shape: notched-rectangle, label: \"Third\" }\nend\nfirst --> second --> third",
    ).expect("Swimlane notched-rectangle aliases should parse");

    assert!(diagram.nodes.iter().all(|node| node.shape == diagram_ir::DiagramShape::NotchedRect));
}

#[test]
fn swimlane_lined_process_attributes_reach_semantic_ir() {
    let diagram = parse_swimlane(
        "swimlane-beta LR\nsubgraph Process\n  first@{ shape: lin-rect, label: \"First\" }\n  second@{ shape: lined-process, label: \"Second\" }\n  third@{ shape: shaded-process, label: \"Third\" }\nend\nfirst --> second --> third",
    ).expect("Swimlane lined-process aliases should parse");

    assert!(diagram.nodes.iter().all(|node| node.shape == diagram_ir::DiagramShape::LinedRect));
}

#[test]
fn swimlane_text_block_attribute_reaches_semantic_ir() {
    let diagram = parse_swimlane(
        "swimlane-beta LR\nsubgraph Notes\n  explanation@{ shape: text, label: \"Context only\" }\nend",
    ).expect("Swimlane text block should parse");

    assert_eq!(diagram.nodes[0].shape, diagram_ir::DiagramShape::TextBlock);
    assert_eq!(diagram.nodes[0].label, "Context only");
}

#[test]
fn swimlane_internal_label_rect_reaches_label_only_semantic_ir() {
    let diagram = parse_swimlane(
        "swimlane-beta LR\nsubgraph Labels\n  caption@{ shape: labelRect, label: \"Context only\" }\nend",
    ).expect("Mermaid 11.16.1 labelRect should parse");

    assert_eq!(diagram.nodes[0].shape, diagram_ir::DiagramShape::TextBlock);
    assert_eq!(diagram.nodes[0].label, "Context only");
}

#[test]
fn swimlane_internal_anchor_reaches_semantic_ir() {
    let diagram = parse_swimlane(
        "swimlane-beta LR\nsubgraph Routing\n  pin@{ shape: anchor }\nend",
    ).expect("Mermaid 11.16.1 anchor should parse");

    assert_eq!(diagram.nodes[0].shape, diagram_ir::DiagramShape::Anchor);
}

#[test]
fn swimlane_internal_rect_with_title_reaches_semantic_ir() {
    let diagram = parse_swimlane(
        "swimlane-beta LR\nsubgraph State\n  item@{ shape: rectWithTitle, label: \"Details\" }\nend",
    ).expect("Mermaid 11.16.1 rectWithTitle should parse");

    assert_eq!(diagram.nodes[0].shape, diagram_ir::DiagramShape::TitledRect);
    assert_eq!(diagram.nodes[0].label, "Details");
}

#[test]
fn swimlane_internal_container_shapes_reach_shared_semantic_ir() {
    use diagram_ir::DiagramShape;

    let diagram = parse_swimlane(
        "swimlane-beta LR\nsubgraph Internal\n  card@{ shape: kanbanItem, label: \"Card\" }\n  topic@{ shape: mindmapCircle, label: \"Topic\" }\n  branch@{ shape: defaultMindmapNode, label: \"Branch\" }\nend\ncard --> topic --> branch",
    ).expect("Mermaid 11.16.1 internal container shapes should parse");

    assert_eq!(diagram.nodes.iter().map(|node| &node.shape).collect::<Vec<_>>(), vec![
        &DiagramShape::RoundedRect,
        &DiagramShape::Ellipse,
        &DiagramShape::RoundedRect,
    ]);
}

#[test]
fn swimlane_internal_compartment_shapes_reach_titled_semantic_ir() {
    let diagram = parse_swimlane(
        "swimlane-beta LR\nsubgraph Internal\n  classifier@{ shape: classBox, label: \"Class\" }\n  entity@{ shape: erBox, label: \"Entity\" }\n  requirement@{ shape: requirementBox, label: \"Requirement\" }\nend\nclassifier --> entity --> requirement",
    ).expect("Mermaid 11.16.1 internal compartment shapes should parse");

    assert!(diagram.nodes.iter().all(|node|
        node.shape == diagram_ir::DiagramShape::TitledRect));
}

#[test]
fn swimlane_start_and_stop_control_shapes_reach_semantic_ir() {
    use diagram_ir::DiagramShape;

    let diagram = parse_swimlane(
        "swimlane-beta LR\nsubgraph Control\n  first@{ shape: sm-circ }\n  second@{ shape: start }\n  third@{ shape: small-circle }\n  fourth@{ shape: fr-circ }\n  fifth@{ shape: stop }\n  sixth@{ shape: framed-circle }\nend\nfirst --> fourth",
    ).expect("Swimlane start and stop control shapes should parse");

    let shapes = diagram.nodes.iter().map(|node| node.shape.clone()).collect::<Vec<_>>();
    assert_eq!(shapes, [
        DiagramShape::SmallCircle, DiagramShape::SmallCircle, DiagramShape::SmallCircle,
        DiagramShape::FramedCircle, DiagramShape::FramedCircle, DiagramShape::FramedCircle,
    ]);
}

#[test]
fn swimlane_fork_and_join_attributes_reach_semantic_ir() {
    let diagram = parse_swimlane(
        "swimlane-beta LR\nsubgraph Control\n  first@{ shape: fork }\n  second@{ shape: join }\nend\nfirst --> second",
    ).expect("Swimlane fork and join controls should parse");

    assert!(diagram.nodes.iter().all(|node| node.shape == diagram_ir::DiagramShape::ForkJoin));
}

#[test]
fn swimlane_lightning_bolt_attributes_reach_semantic_ir() {
    let diagram = parse_swimlane(
        "swimlane-beta LR\nsubgraph Communication\n  first@{ shape: bolt }\n  second@{ shape: com-link }\n  third@{ shape: lightning-bolt }\nend\nfirst --> second --> third",
    ).expect("Swimlane lightning-bolt aliases should parse");

    assert!(diagram.nodes.iter().all(|node| node.shape == diagram_ir::DiagramShape::LightningBolt));
}

#[test]
fn swimlane_comment_brace_attributes_reach_semantic_ir() {
    use diagram_ir::DiagramShape;

    let diagram = parse_swimlane(
        "swimlane-beta LR\nsubgraph Comments\n  first@{ shape: brace, label: \"Left\" }\n  second@{ shape: comment, label: \"Comment\" }\n  third@{ shape: brace-l, label: \"Alias\" }\n  fourth@{ shape: brace-r, label: \"Right\" }\n  fifth@{ shape: braces, label: \"Both\" }\nend\nfirst --> second --> third --> fourth --> fifth",
    ).expect("Swimlane comment-brace aliases should parse");

    let shapes = diagram.nodes.iter().map(|node| node.shape.clone()).collect::<Vec<_>>();
    assert_eq!(shapes, [
        DiagramShape::CurlyBraceLeft, DiagramShape::CurlyBraceLeft,
        DiagramShape::CurlyBraceLeft, DiagramShape::CurlyBraceRight,
        DiagramShape::CurlyBraces,
    ]);
}

#[test]
fn swimlane_document_attributes_reach_semantic_ir() {
    let diagram = parse_swimlane(
        "swimlane-beta LR\nsubgraph Records\n  first@{ shape: doc, label: \"Draft\" }\n  second@{ shape: document, label: \"Final\" }\nend\nfirst --> second",
    ).expect("Swimlane document aliases should parse");

    assert!(diagram.nodes.iter().all(|node| node.shape == diagram_ir::DiagramShape::Document));
    assert_eq!(diagram.nodes[0].label, "Draft");
    assert_eq!(diagram.nodes[1].label, "Final");
}

#[test]
fn swimlane_delay_attributes_reach_semantic_ir() {
    let diagram = parse_swimlane(
        "swimlane-beta LR\nsubgraph Waiting\n  first@{ shape: delay, label: \"Pause\" }\n  second@{ shape: half-rounded-rectangle, label: \"Resume later\" }\nend\nfirst --> second",
    ).expect("Swimlane delay aliases should parse");

    assert!(diagram.nodes.iter().all(|node| node.shape == diagram_ir::DiagramShape::Delay));
}

#[test]
fn swimlane_horizontal_cylinder_attributes_reach_semantic_ir() {
    let diagram = parse_swimlane(
        "swimlane-beta LR\nsubgraph Storage\n  first@{ shape: h-cyl, label: \"Primary\" }\n  second@{ shape: das, label: \"Replica\" }\n  third@{ shape: horizontal-cylinder, label: \"Archive\" }\nend\nfirst --> second --> third",
    ).expect("Swimlane direct-access-storage aliases should parse");

    assert!(diagram.nodes.iter().all(|node| node.shape == diagram_ir::DiagramShape::HorizontalCylinder));
}

#[test]
fn swimlane_lined_cylinder_attributes_reach_semantic_ir() {
    let diagram = parse_swimlane(
        "swimlane-beta LR\nsubgraph Disks\n  first@{ shape: lin-cyl, label: \"Primary\" }\n  second@{ shape: disk, label: \"Replica\" }\n  third@{ shape: lined-cylinder, label: \"Archive\" }\nend\nfirst --> second --> third",
    ).expect("Swimlane disk-storage aliases should parse");

    assert!(diagram.nodes.iter().all(|node| node.shape == diagram_ir::DiagramShape::LinedCylinder));
}

#[test]
fn swimlane_display_attributes_reach_semantic_ir() {
    let diagram = parse_swimlane(
        "swimlane-beta LR\nsubgraph Screens\n  first@{ shape: curv-trap, label: \"Primary\" }\n  second@{ shape: curved-trapezoid, label: \"Secondary\" }\n  third@{ shape: display, label: \"Preview\" }\nend\nfirst --> second --> third",
    ).expect("Swimlane display aliases should parse");

    assert!(diagram.nodes.iter().all(|node| node.shape == diagram_ir::DiagramShape::Display));
}

#[test]
fn swimlane_divided_process_attributes_reach_semantic_ir() {
    let diagram = parse_swimlane(
        "swimlane-beta LR\nsubgraph Processing\n  first@{ shape: div-rect }\n  second@{ shape: div-proc }\n  third@{ shape: divided-rectangle }\n  fourth@{ shape: divided-process }\nend\nfirst --> second --> third --> fourth",
    ).expect("Swimlane divided-process aliases should parse");

    assert!(diagram.nodes.iter().all(|node| node.shape == diagram_ir::DiagramShape::DividedRect));
}

#[test]
fn swimlane_internal_storage_attributes_reach_semantic_ir() {
    let diagram = parse_swimlane(
        "swimlane-beta LR\nsubgraph Storage\n  first@{ shape: win-pane }\n  second@{ shape: internal-storage }\n  third@{ shape: window-pane }\nend\nfirst --> second --> third",
    ).expect("Swimlane internal-storage aliases should parse");

    assert!(diagram.nodes.iter().all(|node| node.shape == diagram_ir::DiagramShape::WindowPane));
}

#[test]
fn swimlane_junction_attributes_reach_semantic_ir() {
    let diagram = parse_swimlane(
        "swimlane-beta LR\nsubgraph Junctions\n  first@{ shape: f-circ }\n  second@{ shape: junction }\n  third@{ shape: filled-circle }\nend\nfirst --> second --> third",
    ).expect("Swimlane junction aliases should parse");

    assert!(diagram.nodes.iter().all(|node| node.shape == diagram_ir::DiagramShape::FilledCircle));
}

#[test]
fn swimlane_loop_limit_attributes_reach_semantic_ir() {
    let diagram = parse_swimlane(
        "swimlane-beta LR\nsubgraph Loops\n  first@{ shape: notch-pent, label: \"First\" }\n  second@{ shape: loop-limit, label: \"Second\" }\n  third@{ shape: notched-pentagon, label: \"Third\" }\nend\nfirst --> second --> third",
    ).expect("Swimlane loop-limit aliases should parse");

    assert!(diagram.nodes.iter().all(|node| node.shape == diagram_ir::DiagramShape::LoopLimit));
}

#[test]
fn swimlane_manual_input_attributes_reach_semantic_ir() {
    let diagram = parse_swimlane(
        "swimlane-beta LR\nsubgraph Input\n  first@{ shape: sl-rect, label: \"First\" }\n  second@{ shape: manual-input, label: \"Second\" }\n  third@{ shape: sloped-rectangle, label: \"Third\" }\nend\nfirst --> second --> third",
    ).expect("Swimlane manual-input aliases should parse");

    assert!(diagram.nodes.iter().all(|node| node.shape == diagram_ir::DiagramShape::ManualInput));
}

#[test]
fn swimlane_multi_document_attributes_reach_semantic_ir() {
    let diagram = parse_swimlane(
        "swimlane-beta LR\nsubgraph Records\n  first@{ shape: docs, label: \"First\" }\n  second@{ shape: documents, label: \"Second\" }\n  third@{ shape: st-doc, label: \"Third\" }\n  fourth@{ shape: stacked-document, label: \"Fourth\" }\nend\nfirst --> second --> third --> fourth",
    ).expect("Swimlane multi-document aliases should parse");

    assert!(diagram.nodes.iter().all(|node| node.shape == diagram_ir::DiagramShape::MultiDocument));
}

#[test]
fn swimlane_multi_process_attributes_reach_semantic_ir() {
    let diagram = parse_swimlane(
        "swimlane-beta LR\nsubgraph Processing\n  first@{ shape: st-rect, label: \"First\" }\n  second@{ shape: procs, label: \"Second\" }\n  third@{ shape: processes, label: \"Third\" }\n  fourth@{ shape: stacked-rectangle, label: \"Fourth\" }\nend\nfirst --> second --> third --> fourth",
    ).expect("Swimlane multi-process aliases should parse");

    assert!(diagram.nodes.iter().all(|node| node.shape == diagram_ir::DiagramShape::MultiProcess));
}

#[test]
fn swimlane_stored_data_attributes_reach_semantic_ir() {
    let diagram = parse_swimlane(
        "swimlane-beta LR\nsubgraph Storage\n  first@{ shape: bow-rect, label: \"First\" }\n  second@{ shape: stored-data, label: \"Second\" }\n  third@{ shape: bow-tie-rectangle, label: \"Third\" }\nend\nfirst --> second --> third",
    ).expect("Swimlane stored-data aliases should parse");

    assert!(diagram.nodes.iter().all(|node| node.shape == diagram_ir::DiagramShape::StoredData));
}

#[test]
fn swimlane_summary_attributes_reach_semantic_ir() {
    let diagram = parse_swimlane(
        "swimlane-beta LR\nsubgraph Summary\n  first@{ shape: cross-circ }\n  second@{ shape: summary }\n  third@{ shape: crossed-circle }\nend\nfirst --> second --> third",
    ).expect("Swimlane summary aliases should parse");

    assert!(diagram.nodes.iter().all(|node| node.shape == diagram_ir::DiagramShape::Summary));
}

#[test]
fn swimlane_tagged_document_attributes_reach_semantic_ir() {
    let diagram = parse_swimlane(
        "swimlane-beta LR\nsubgraph Records\n  first@{ shape: tag-doc, label: \"First\" }\n  second@{ shape: tagged-document, label: \"Second\" }\nend\nfirst --> second",
    ).expect("Swimlane tagged-document aliases should parse");

    assert!(diagram.nodes.iter().all(|node| node.shape == diagram_ir::DiagramShape::TaggedDocument));
}

#[test]
fn swimlane_tagged_process_attributes_reach_semantic_ir() {
    let diagram = parse_swimlane(
        "swimlane-beta LR\nsubgraph Processing\n  first@{ shape: tag-rect, label: \"First\" }\n  second@{ shape: tagged-rectangle, label: \"Second\" }\n  third@{ shape: tag-proc, label: \"Third\" }\n  fourth@{ shape: tagged-process, label: \"Fourth\" }\nend\nfirst --> second --> third --> fourth",
    ).expect("Swimlane tagged-process aliases should parse");

    assert!(diagram.nodes.iter().all(|node| node.shape == diagram_ir::DiagramShape::TaggedProcess));
}

#[test]
fn swimlane_paper_tape_attributes_reach_semantic_ir() {
    let diagram = parse_swimlane(
        "swimlane-beta LR\nsubgraph Output\n  first@{ shape: flag, label: \"First\" }\n  second@{ shape: paper-tape, label: \"Second\" }\nend\nfirst --> second",
    ).expect("Swimlane paper-tape aliases should parse");

    assert!(diagram.nodes.iter().all(|node| node.shape == diagram_ir::DiagramShape::PaperTape));
}

#[test]
fn swimlane_lined_document_attributes_reach_semantic_ir() {
    let diagram = parse_swimlane(
        "swimlane-beta LR\nsubgraph Records\n  first@{ shape: lin-doc, label: \"First\" }\n  second@{ shape: lined-document, label: \"Second\" }\nend\nfirst --> second",
    ).expect("Swimlane lined-document aliases should parse");

    assert!(diagram.nodes.iter().all(|node| node.shape == diagram_ir::DiagramShape::LinedDocument));
}

#[test]
fn swimlane_data_store_attributes_reach_semantic_ir() {
    let diagram = parse_swimlane(
        "swimlane-beta LR\nsubgraph Storage\n  first@{ shape: datastore, label: \"First\" }\n  second@{ shape: data-store, label: \"Second\" }\nend\nfirst --> second",
    ).expect("Swimlane data-store aliases should parse");

    assert!(diagram.nodes.iter().all(|node| node.shape == diagram_ir::DiagramShape::DataStore));
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
fn swimlane_theme_variables_reach_semantic_ir() {
    let diagram = parse_swimlane(
        "%%{init: {\"themeVariables\": {\"mainBkg\": \"#ffe1ef\", \"nodeBorder\": \"#225577\", \"lineColor\": \"#118844\"}}}%%\nswimlane-beta LR\nsubgraph ThemeLane\n  themed[Themed node]\n  next[Next node]\nend\nthemed --> next",
    ).expect("Swimlane theme variables should parse");

    assert_eq!(diagram.theme.main_background, "#ffe1ef");
    assert_eq!(diagram.theme.node_border, "#225577");
    assert_eq!(diagram.theme.line_color, "#118844");

    let front_matter = parse_swimlane(
        "---\nthemeVariables:\n  mainBkg: '#fef3c7'\n  nodeBorder: '#b45309'\n  lineColor: '#166534'\n---\nswimlane-beta\nsubgraph A\n  one[One]\nend",
    ).expect("front-matter Swimlane theme variables should parse");
    assert_eq!(front_matter.theme.main_background, "#fef3c7");
}

#[test]
fn swimlane_theme_typography_reaches_semantic_ir() {
    let diagram = parse_swimlane(
        "%%{init: {\"themeVariables\": {\"primaryTextColor\": \"#7c2d12\", \"fontFamily\": \"Helvetica Neue\", \"fontSize\": \"19px\"}}}%%\nswimlane-beta LR\nsubgraph ThemeLane\n  themed[Themed typography]\n  next[Next node]\nend\nthemed --> next",
    ).expect("Swimlane theme typography should parse");

    assert_eq!(diagram.theme.primary_text_color, "#7c2d12");
    assert_eq!(diagram.theme.font_family.as_deref(), Some("Helvetica Neue"));
    assert_eq!(diagram.theme.font_size, Some(19.0));
}

#[test]
fn swimlane_edge_label_theme_reaches_semantic_ir() {
    let diagram = parse_swimlane(
        "%%{init: {\"themeVariables\": {\"textColor\": \"#172554\", \"edgeLabelBackground\": \"#dbeafe\"}}}%%\nswimlane-beta LR\nsubgraph ThemeLane\n  request[Request]\n  done[Done]\nend\nrequest -->|themed handoff| done",
    ).expect("Swimlane edge label theme should parse");

    assert_eq!(diagram.theme.text_color, "#172554");
    assert_eq!(diagram.theme.edge_label_background.as_deref(), Some("#dbeafe"));
    assert_eq!(diagram.edges[0].label.as_deref(), Some("themed handoff"));
}

#[test]
fn swimlane_cluster_theme_reaches_semantic_ir() {
    let diagram = parse_swimlane(
        "%%{init: {\"themeVariables\": {\"clusterBkg\": \"#fef3c7\", \"clusterBorder\": \"#b45309\", \"titleColor\": \"#7c2d12\"}}}%%\nswimlane-beta LR\ntitle Themed workflow\nsubgraph Review[Review lane]\n  subgraph Approval[Approval phase]\n    request[Request]\n    done[Done]\n  end\nend\nrequest --> done",
    ).expect("Swimlane cluster theme should parse");

    assert_eq!(diagram.theme.cluster_background.as_deref(), Some("#fef3c7"));
    assert_eq!(diagram.theme.cluster_border.as_deref(), Some("#b45309"));
    assert_eq!(diagram.theme.title_color.as_deref(), Some("#7c2d12"));
    assert_eq!(diagram.groups.len(), 1);
}

#[test]
fn swimlane_node_and_edge_theme_reaches_semantic_ir() {
    let diagram = parse_swimlane(
        "%%{init: {\"themeVariables\": {\"nodeTextColor\": \"#14532d\", \"arrowheadColor\": \"#dc2626\", \"strokeWidth\": \"4px\"}}}%%\nswimlane-beta LR\nsubgraph ThemeLane\n  request[Request]\n  done[Done]\nend\nrequest --> done",
    ).expect("Swimlane node and edge theme should parse");

    assert_eq!(diagram.theme.node_text_color.as_deref(), Some("#14532d"));
    assert_eq!(diagram.theme.arrowhead_color.as_deref(), Some("#dc2626"));
    assert_eq!(diagram.theme.stroke_width, Some(4.0));
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
fn swimlane_node_typography_lowers_to_semantic_ir() {
    let diagram = parse_swimlane(
        "swimlane-beta LR\nsubgraph Review\n  emphasized[Typographic approval step]\n  direct[Typographic delivery step]\nend\nclassDef emphasis font-size:22px,font-weight:bold,font-style:italic,font-family:\"Helvetica Neue\",color:#7c2d12\nclass emphasized emphasis\nstyle direct font-size:18px,font-weight:500,font-style:normal,font-family:Helvetica,color:#14532d\nemphasized --> direct",
    ).expect("Swimlane node typography should parse");

    let emphasized = &diagram.nodes[0].style;
    assert_eq!(emphasized.font_size, Some(22.0));
    assert_eq!(emphasized.font_weight, Some(700));
    assert_eq!(emphasized.font_italic, Some(true));
    assert_eq!(emphasized.font_family.as_deref(), Some("Helvetica Neue"));
    let direct = &diagram.nodes[1].style;
    assert_eq!(direct.font_size, Some(18.0));
    assert_eq!(direct.font_weight, Some(500));
    assert_eq!(direct.font_italic, Some(false));
    assert_eq!(direct.font_family.as_deref(), Some("Helvetica"));
    assert!(parse_swimlane(
        "swimlane-beta\nsubgraph A\n  one[One]\nend\nstyle one font-weight:heavy"
    ).is_err());
}

#[test]
fn swimlane_node_corner_radii_lower_to_semantic_ir() {
    let diagram = parse_swimlane(
        "swimlane-beta LR\nsubgraph Review\n  rounded[Class rounded]\n  compact[Direct compact]\nend\nclassDef soft fill:#fef3c7,stroke:#b45309,rx:16px,ry:16px\nclass rounded soft\nstyle compact fill:#dcfce7,stroke:#166534,rx:4px,ry:4px\nrounded --> compact",
    ).expect("Swimlane node corner radii should parse");

    assert_eq!(diagram.nodes[0].style.corner_radius, Some(16.0));
    assert_eq!(diagram.nodes[1].style.corner_radius, Some(4.0));
    assert!(parse_swimlane(
        "swimlane-beta\nsubgraph A\n  one[One]\nend\nstyle one rx:4px,ry:8px"
    ).is_err());
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
fn swimlane_class_statements_accept_terminal_semicolons() {
    let diagram = parse_swimlane(
        "swimlane-beta LR\nsubgraph Intake\n  request[Request]\nend\nsubgraph Support\n  done[Done]\nend\nclass request,done accent;\nclassDef accent fill:#dbeafe,stroke:#1d4ed8,color:#172554,stroke-width:3px;\nrequest --> done",
    ).expect("semicolon-terminated Swimlane classes should parse");

    assert_eq!(diagram.nodes[0].classes, ["accent"]);
    assert_eq!(diagram.nodes[1].classes, ["accent"]);
    assert_eq!(diagram.nodes[0].style.fill.as_deref(), Some("#dbeafe"));
    assert_eq!(diagram.nodes[1].style.stroke.as_deref(), Some("#1d4ed8"));
    assert_eq!(diagram.nodes[1].style.stroke_width, Some(3.0));
}

#[test]
fn swimlane_node_class_decorators_reach_semantic_ir() {
    let diagram = parse_swimlane(
        "swimlane-beta LR\nsubgraph Intake\n  request[Request]:::accent\nend\nsubgraph Support\n  done[Done]\nend\nrequest --> done:::complete\nclassDef accent fill:#dbeafe,stroke:#1d4ed8\nclassDef complete fill:#dcfce7,stroke:#166534",
    ).expect("Swimlane class decorators should parse");

    assert_eq!(diagram.nodes[0].classes, ["accent"]);
    assert_eq!(diagram.nodes[0].style.stroke.as_deref(), Some("#1d4ed8"));
    assert_eq!(diagram.nodes[1].classes, ["complete"]);
    assert_eq!(diagram.nodes[1].style.fill.as_deref(), Some("#dcfce7"));
}

#[test]
fn swimlane_click_links_reach_semantic_ir() {
    let diagram = parse_swimlane(
        "swimlane-beta LR\nsubgraph Intake\n  request[Request]\nend\nsubgraph Support\n  done[Done]\nend\nclick request href \"https://example.com/request?a=1&amp;b=2\" \"Open \\\"request\\\"\" _blank\nrequest --> done",
    ).expect("Swimlane click links should parse");

    assert_eq!(diagram.links.len(), 1);
    assert_eq!(diagram.links[0].node_id, "request");
    assert_eq!(diagram.links[0].url, "https://example.com/request?a=1&b=2");
    assert_eq!(diagram.links[0].tooltip.as_deref(), Some("Open \"request\""));
    assert_eq!(diagram.links[0].target.as_deref(), Some("_blank"));
    assert!(parse_swimlane("swimlane-beta\nsubgraph A\n  one[One]\nend\nclick missing \"https://example.com\"").is_err());
}

#[test]
fn swimlane_callback_actions_reach_semantic_ir() {
    let diagram = parse_swimlane(
        "swimlane-beta LR\nsubgraph Intake\n  request[Request]\nend\nsubgraph Support\n  done[Done]\nend\nclick request call openRequest(request, priority) \"Open request\"\nrequest --> done",
    ).expect("Swimlane callback actions should parse");

    assert_eq!(diagram.callbacks.len(), 1);
    assert_eq!(diagram.callbacks[0].node_id, "request");
    assert_eq!(diagram.callbacks[0].name, "openRequest");
    assert_eq!(diagram.callbacks[0].arguments.as_deref(), Some("request, priority"));
    assert_eq!(diagram.callbacks[0].tooltip.as_deref(), Some("Open request"));
}

#[test]
fn swimlane_legacy_callback_aliases_reach_semantic_ir() {
    let diagram = parse_swimlane(
        "swimlane-beta LR\nsubgraph Intake\n  request[Request]\nend\nsubgraph Support\n  done[Done]\nend\nclick request openRequest \"Open request\"\nrequest --> done",
    ).expect("legacy Swimlane callback aliases should parse");

    assert_eq!(diagram.callbacks.len(), 1);
    assert_eq!(diagram.callbacks[0].node_id, "request");
    assert_eq!(diagram.callbacks[0].name, "openRequest");
    assert_eq!(diagram.callbacks[0].arguments, None);
    assert_eq!(diagram.callbacks[0].tooltip.as_deref(), Some("Open request"));
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
fn swimlane_invisible_links_lower_to_semantic_ir() {
    let diagram = parse_swimlane(
        "swimlane-beta LR\nsubgraph Review\n  draft[Draft]\n  published[Published]\nend\ndraft ~~~ published",
    ).expect("Flowchart invisible links should parse");

    assert_eq!(diagram.edges.len(), 1);
    assert_eq!(diagram.edges[0].kind, diagram_ir::SwimlaneEdgeKind::Invisible);
    assert_eq!(diagram.edges[0].start_marker, diagram_ir::EdgeMarker::None);
    assert_eq!(diagram.edges[0].end_marker, diagram_ir::EdgeMarker::None);
}

#[test]
fn swimlane_open_styled_links_lower_to_semantic_ir() {
    use diagram_ir::{EdgeMarker, SwimlaneEdgeKind};

    let diagram = parse_swimlane(
        "swimlane-beta LR\nsubgraph Review\n  draft[Draft]\n  review[Review]\n  published[Published]\nend\ndraft -.- review === published",
    ).expect("open dotted and thick Flowchart links should parse");

    assert_eq!(diagram.edges.len(), 2);
    assert_eq!((diagram.edges[0].kind, diagram.edges[0].end_marker),
        (SwimlaneEdgeKind::Dotted, EdgeMarker::None));
    assert_eq!((diagram.edges[1].kind, diagram.edges[1].end_marker),
        (SwimlaneEdgeKind::Thick, EdgeMarker::None));
}

#[test]
fn swimlane_styled_endpoint_markers_lower_to_semantic_ir() {
    use diagram_ir::{EdgeMarker, SwimlaneEdgeKind};

    let diagram = parse_swimlane(
        "swimlane-beta LR\nsubgraph Review\n  a[A]\n  b[B]\n  c[C]\n  d[D]\n  e[E]\n  f[F]\n  g[G]\n  h[H]\n  i[I]\nend\na o-.-o b x-.-x c o-.-x d x-.-o e o-.-> f x-.-> g -.-o h -.-x i\na o==o b x==x c o==x d x==o e o==> f x==> g ==o h ==x i",
    ).expect("dotted and thick endpoint markers should parse");

    let expected_markers = [
        (EdgeMarker::Circle, EdgeMarker::Circle),
        (EdgeMarker::Cross, EdgeMarker::Cross),
        (EdgeMarker::Circle, EdgeMarker::Cross),
        (EdgeMarker::Cross, EdgeMarker::Circle),
        (EdgeMarker::Circle, EdgeMarker::Point),
        (EdgeMarker::Cross, EdgeMarker::Point),
        (EdgeMarker::None, EdgeMarker::Circle),
        (EdgeMarker::None, EdgeMarker::Cross),
    ];
    assert_eq!(diagram.edges.len(), 16);
    for (edge, markers) in diagram.edges[..8].iter().zip(expected_markers) {
        assert_eq!(edge.kind, SwimlaneEdgeKind::Dotted);
        assert_eq!((edge.start_marker, edge.end_marker), markers);
    }
    for (edge, markers) in diagram.edges[8..].iter().zip(expected_markers) {
        assert_eq!(edge.kind, SwimlaneEdgeKind::Thick);
        assert_eq!((edge.start_marker, edge.end_marker), markers);
    }
}

#[test]
fn swimlane_minimum_link_lengths_lower_to_semantic_ir() {
    let diagram = parse_swimlane(
        "swimlane-beta LR\nsubgraph Review\n  a[A]\n  b[B]\n  c[C]\n  d[D]\n  e[E]\n  f[F]\n  g[G]\nend\na ---> b ---- c ===> d ==== e -..-> f -..- g\na ----> b ----- c ====> d ===== e -...-> f -...- g",
    ).expect("minimum-length Flowchart links should parse");

    assert_eq!(diagram.edges.len(), 12);
    assert!(diagram.edges[..6].iter().all(|edge| edge.min_length == 2));
    assert!(diagram.edges[6..].iter().all(|edge| edge.min_length == 3));
    assert_eq!(diagram.edges[0].kind, diagram_ir::SwimlaneEdgeKind::Directed);
    assert_eq!(diagram.edges[2].kind, diagram_ir::SwimlaneEdgeKind::Thick);
    assert_eq!(diagram.edges[4].kind, diagram_ir::SwimlaneEdgeKind::Dotted);
}

#[test]
fn swimlane_styled_inline_labels_lower_to_semantic_ir() {
    use diagram_ir::SwimlaneEdgeKind;

    let diagram = parse_swimlane(
        "swimlane-beta LR\nsubgraph Review\n  a[A]\n  b[B]\n  c[C]\n  d[D]\nend\na -- normal ---> b == thick ===> c -. dotted ...-> d",
    ).expect("styled Flowchart inline labels should parse");

    assert_eq!(diagram.edges.iter().map(|edge| edge.label.as_deref()).collect::<Vec<_>>(),
        vec![Some("normal"), Some("thick"), Some("dotted")]);
    assert_eq!(diagram.edges.iter().map(|edge| edge.kind).collect::<Vec<_>>(),
        vec![SwimlaneEdgeKind::Directed, SwimlaneEdgeKind::Thick, SwimlaneEdgeKind::Dotted]);
    assert_eq!(diagram.edges.iter().map(|edge| edge.min_length).collect::<Vec<_>>(), vec![2, 2, 3]);
}

#[test]
fn swimlane_labeled_bidirectional_links_lower_to_semantic_ir() {
    use diagram_ir::{EdgeMarker, SwimlaneEdgeKind};

    let diagram = parse_swimlane(
        "swimlane-beta LR\nsubgraph Review\n  a[A]\n  b[B]\n  c[C]\n  d[D]\nend\na<-- normal -->b<== thick ==>c<-. dotted .->d",
    ).expect("labeled bidirectional Flowchart links should parse");

    assert_eq!(diagram.edges.iter().map(|edge| edge.label.as_deref()).collect::<Vec<_>>(),
        vec![Some("normal"), Some("thick"), Some("dotted")]);
    assert_eq!(diagram.edges.iter().map(|edge| edge.kind).collect::<Vec<_>>(),
        vec![SwimlaneEdgeKind::Directed, SwimlaneEdgeKind::Thick, SwimlaneEdgeKind::Dotted]);
    assert!(diagram.edges.iter().all(|edge|
        (edge.start_marker, edge.end_marker) == (EdgeMarker::Point, EdgeMarker::Point)));
}

#[test]
fn swimlane_labeled_circle_and_cross_links_lower_to_semantic_ir() {
    use diagram_ir::{EdgeMarker, SwimlaneEdgeKind};

    let diagram = parse_swimlane(
        "swimlane-beta LR\nsubgraph Review\n  a[A]\n  b[B]\n  c[C]\n  d[D]\n  e[E]\n  f[F]\n  g[G]\nend\na o-- solid circle --o b\nb x-- solid cross --x c\nc o== thick circle ==o d\nd x== thick cross ==x e\ne o-. dotted circle .-o f\nf x-. dotted cross .-x g",
    ).expect("labeled circle and cross Flowchart links should parse");

    assert_eq!(diagram.edges.iter().map(|edge| edge.kind).collect::<Vec<_>>(), vec![
        SwimlaneEdgeKind::Directed, SwimlaneEdgeKind::Directed,
        SwimlaneEdgeKind::Thick, SwimlaneEdgeKind::Thick,
        SwimlaneEdgeKind::Dotted, SwimlaneEdgeKind::Dotted,
    ]);
    for (index, edge) in diagram.edges.iter().enumerate() {
        let marker = if index % 2 == 0 { EdgeMarker::Circle } else { EdgeMarker::Cross };
        assert_eq!((edge.start_marker, edge.end_marker), (marker, marker));
        assert!(edge.label.is_some());
    }
}

#[test]
fn swimlane_labeled_mixed_endpoint_markers_lower_to_semantic_ir() {
    use diagram_ir::EdgeMarker;

    let diagram = parse_swimlane(
        "swimlane-beta LR\nsubgraph Review\n  a[A]\n  b[B]\n  c[C]\n  d[D]\n  e[E]\n  f[F]\n  g[G]\nend\na o-- circle to cross --x b\nb x== cross to circle ==o c\nc <-. point to cross .-x d\nd o-. circle to point .-> e\ne -- none to circle --o f\nf <== point to cross ==x g",
    ).expect("labeled mixed endpoint markers should parse");

    let markers = diagram.edges.iter().map(|edge| (edge.start_marker, edge.end_marker))
        .collect::<Vec<_>>();
    assert_eq!(markers, vec![
        (EdgeMarker::Circle, EdgeMarker::Cross),
        (EdgeMarker::Cross, EdgeMarker::Circle),
        (EdgeMarker::Point, EdgeMarker::Cross),
        (EdgeMarker::Circle, EdgeMarker::Point),
        (EdgeMarker::None, EdgeMarker::Circle),
        (EdgeMarker::Point, EdgeMarker::Cross),
    ]);
    assert!(diagram.edges.iter().all(|edge| edge.label.is_some()));
}

#[test]
fn swimlane_labeled_open_links_lower_to_semantic_ir() {
    use diagram_ir::{EdgeMarker, SwimlaneEdgeKind};

    let diagram = parse_swimlane(
        "swimlane-beta LR\nsubgraph Review\n  a[A]\n  b[B]\n  c[C]\n  d[D]\n  e[E]\n  f[F]\n  g[G]\nend\na -- solid open --- b\nb o-- circle open --- c\nc x== cross open === d\nd <== point open === e\ne -. dotted open .- f\nf o-. dotted circle open .- g",
    ).expect("labeled links without end markers should parse");

    assert_eq!(diagram.edges.iter().map(|edge| edge.kind).collect::<Vec<_>>(), vec![
        SwimlaneEdgeKind::Directed, SwimlaneEdgeKind::Directed,
        SwimlaneEdgeKind::Thick, SwimlaneEdgeKind::Thick,
        SwimlaneEdgeKind::Dotted, SwimlaneEdgeKind::Dotted,
    ]);
    assert_eq!(diagram.edges.iter().map(|edge| edge.start_marker).collect::<Vec<_>>(),
        vec![EdgeMarker::None, EdgeMarker::Circle, EdgeMarker::Cross,
            EdgeMarker::Point, EdgeMarker::None, EdgeMarker::Circle]);
    assert!(diagram.edges.iter().all(|edge| edge.end_marker == EdgeMarker::None));
}

#[test]
fn swimlane_edge_ids_lower_to_semantic_ir() {
    let diagram = parse_swimlane(
        "swimlane-beta LR\nsubgraph Review\n  request[Request]\n  review[Review]\n  done[Done]\nend\nrequest submit@--> review release@==> done",
    ).expect("Flowchart edge identifiers should parse");

    assert_eq!(diagram.edges.iter().map(|edge| edge.id.as_deref()).collect::<Vec<_>>(),
        vec![Some("submit"), Some("release")]);
    assert_eq!(diagram.edges[0].from, "request");
    assert_eq!(diagram.edges[1].to, "done");
}

#[test]
fn swimlane_edge_classes_and_styles_lower_to_semantic_ir() {
    let diagram = parse_swimlane(
        "swimlane-beta LR\nsubgraph Review\n  request[Request]\n  review[Review]\n  done[Done]\nend\nrequest submit@--> review release@==> done\nclassDef urgent stroke:#dc2626,stroke-width:4px,stroke-dasharray:8 3,color:#991b1b\nclass submit urgent\nstyle release stroke:#059669,stroke-width:5px,stroke-dasharray:2 2,color:#065f46",
    ).expect("Flowchart edge classes and direct styles should parse");

    assert_eq!(diagram.edges[0].classes, ["urgent"]);
    assert_eq!(diagram.edges[0].style.stroke.as_deref(), Some("#dc2626"));
    assert_eq!(diagram.edges[0].style.stroke_width, Some(4.0));
    assert_eq!(diagram.edges[0].style.stroke_dash.as_deref(), Some([8.0, 3.0].as_slice()));
    assert_eq!(diagram.edges[0].style.text_color.as_deref(), Some("#991b1b"));
    assert_eq!(diagram.edges[1].style.stroke.as_deref(), Some("#059669"));
    assert_eq!(diagram.edges[1].style.stroke_width, Some(5.0));
    assert_eq!(diagram.edges[1].style.stroke_dash.as_deref(), Some([2.0, 2.0].as_slice()));
    assert_eq!(diagram.edges[1].style.text_color.as_deref(), Some("#065f46"));
}

#[test]
fn swimlane_edge_label_typography_lowers_to_semantic_ir() {
    let diagram = parse_swimlane(
        "swimlane-beta LR\nsubgraph Review\n  request[Request]\n  review[Review]\n  done[Done]\nend\nrequest submit@-- submit --> review release@-- release --> done\nclassDef urgent font-size:22px,font-weight:bold,font-style:italic,font-family:\"Helvetica Neue\",color:#7c2d12\nclass submit urgent\nstyle release font-size:18px,font-weight:500,font-style:normal,font-family:Helvetica,color:#14532d",
    ).expect("Flowchart edge label typography should parse");

    let submit = &diagram.edges[0].style;
    assert_eq!(submit.font_size, Some(22.0));
    assert_eq!(submit.font_weight, Some(700));
    assert_eq!(submit.font_italic, Some(true));
    assert_eq!(submit.font_family.as_deref(), Some("Helvetica Neue"));
    let release = &diagram.edges[1].style;
    assert_eq!(release.font_size, Some(18.0));
    assert_eq!(release.font_weight, Some(500));
    assert_eq!(release.font_italic, Some(false));
    assert_eq!(release.font_family.as_deref(), Some("Helvetica"));
}

#[test]
fn swimlane_positional_link_styles_lower_to_semantic_ir() {
    let diagram = parse_swimlane(
        "swimlane-beta LR\nsubgraph Review\n  a[A]\n  b[B]\n  c[C]\n  d[D]\nend\na --> b\nb --> c\nc --> d\nlinkStyle default stroke:#64748b,stroke-width:2px\nlinkStyle 0, 2 stroke:#dc2626,stroke-width:5px,stroke-dasharray:7 3,color:#991b1b;",
    ).expect("positional link styles should parse");

    assert_eq!(diagram.edges[0].style.stroke.as_deref(), Some("#dc2626"));
    assert_eq!(diagram.edges[1].style.stroke.as_deref(), Some("#64748b"));
    assert_eq!(diagram.edges[2].style.stroke.as_deref(), Some("#dc2626"));
    assert_eq!(diagram.edges[0].style.stroke_width, Some(5.0));
    assert_eq!(diagram.edges[1].style.stroke_width, Some(2.0));
    assert_eq!(diagram.edges[2].style.stroke_dash.as_deref(), Some([7.0, 3.0].as_slice()));
    assert_eq!(diagram.edges[2].style.text_color.as_deref(), Some("#991b1b"));
}

#[test]
fn swimlane_positional_link_curves_lower_to_semantic_ir() {
    use diagram_ir::SwimlaneEdgeCurve;

    let diagram = parse_swimlane(
        "swimlane-beta LR\nsubgraph Intake\n  first[First]\n  second[Second]\nend\nsubgraph Review\n  third[Third]\n  fourth[Fourth]\nend\nfirst --> fourth\nsecond --> third\nlinkStyle default interpolate basis\nlinkStyle 1 interpolate stepBefore;",
    ).expect("positional link curves should parse");

    assert_eq!(diagram.edges[0].curve, Some(SwimlaneEdgeCurve::Basis));
    assert_eq!(diagram.edges[1].curve, Some(SwimlaneEdgeCurve::StepBefore));
    assert!(parse_swimlane(
        "swimlane-beta\nsubgraph A\n  one[One]\n  two[Two]\nend\none --> two\nlinkStyle 0 interpolate imaginary"
    ).is_err());
}

#[test]
fn swimlane_edge_animation_config_lowers_to_semantic_ir() {
    use diagram_ir::SwimlaneEdgeAnimation;

    let diagram = parse_swimlane(
        "swimlane-beta LR\nsubgraph Review\n  request[Request]\n  review[Review]\n  done[Done]\nend\nrequest submit@--> review release@==> done\nsubmit@{ animate: true, animation: fast }\nrelease@{ animation: slow }",
    ).expect("Flowchart edge animation configuration should parse");

    assert!(diagram.edges[0].animate);
    assert_eq!(diagram.edges[0].animation, Some(SwimlaneEdgeAnimation::Fast));
    assert!(diagram.edges[1].animate);
    assert_eq!(diagram.edges[1].animation, Some(SwimlaneEdgeAnimation::Slow));
    assert!(parse_swimlane(
        "swimlane-beta\nsubgraph A\n  one[One]\n  two[Two]\nend\none link@--> two\nlink@{ animation: instant }"
    ).is_err());
}

#[test]
fn swimlane_edge_curve_config_lowers_to_semantic_ir() {
    use diagram_ir::SwimlaneEdgeCurve;

    let diagram = parse_swimlane(
        "swimlane-beta LR\nsubgraph Intake\n  request[Request]\nend\nsubgraph Review\n  review[Review]\n  done[Done]\nend\nrequest curved@--> done\nreview linear@--> done\ncurved@{ curve: basis }\nlinear@{ curve: linear }",
    ).expect("Flowchart edge curve configuration should parse");

    assert_eq!(diagram.edges[0].curve, Some(SwimlaneEdgeCurve::Basis));
    assert_eq!(diagram.edges[1].curve, Some(SwimlaneEdgeCurve::Linear));
    assert!(parse_swimlane(
        "swimlane-beta\nsubgraph A\n  one[One]\n  two[Two]\nend\none link@--> two\nlink@{ curve: imaginary }"
    ).is_err());
}

#[test]
fn swimlane_step_curve_config_lowers_to_semantic_ir() {
    use diagram_ir::SwimlaneEdgeCurve;

    let diagram = parse_swimlane(
        "swimlane-beta LR\nsubgraph Intake\n  first[First]\n  second[Second]\n  third[Third]\nend\nsubgraph Review\n  fourth[Fourth]\n  fifth[Fifth]\n  sixth[Sixth]\nend\nfirst stepped@--> sixth\nsecond before@--> fifth\nthird after@--> fourth\nstepped@{ curve: step }\nbefore@{ curve: stepBefore }\nafter@{ curve: stepAfter }",
    ).expect("Flowchart step curve configuration should parse");

    assert_eq!(diagram.edges.iter().map(|edge| edge.curve).collect::<Vec<_>>(), vec![
        Some(SwimlaneEdgeCurve::Step),
        Some(SwimlaneEdgeCurve::StepBefore),
        Some(SwimlaneEdgeCurve::StepAfter),
    ]);
}

#[test]
fn swimlane_bump_curve_config_lowers_to_semantic_ir() {
    use diagram_ir::SwimlaneEdgeCurve;

    let diagram = parse_swimlane(
        "swimlane-beta LR\nsubgraph Intake\n  first[First]\n  second[Second]\nend\nsubgraph Review\n  third[Third]\n  fourth[Fourth]\nend\nfirst horizontal@--> fourth\nsecond vertical@--> third\nhorizontal@{ curve: bumpX }\nvertical@{ curve: bumpY }",
    ).expect("Flowchart bump curve configuration should parse");

    assert_eq!(diagram.edges.iter().map(|edge| edge.curve).collect::<Vec<_>>(), vec![
        Some(SwimlaneEdgeCurve::BumpX), Some(SwimlaneEdgeCurve::BumpY),
    ]);
}

#[test]
fn swimlane_monotone_curve_config_lowers_to_semantic_ir() {
    use diagram_ir::SwimlaneEdgeCurve;

    let diagram = parse_swimlane(
        "swimlane-beta LR\nsubgraph Intake\n  first[First]\n  second[Second]\nend\nsubgraph Review\n  third[Third]\n  fourth[Fourth]\nend\nfirst horizontal@--> fourth\nsecond vertical@--> third\nhorizontal@{ curve: monotoneX }\nvertical@{ curve: monotoneY }",
    ).expect("Flowchart monotone curve configuration should parse");

    assert_eq!(diagram.edges.iter().map(|edge| edge.curve).collect::<Vec<_>>(), vec![
        Some(SwimlaneEdgeCurve::MonotoneX), Some(SwimlaneEdgeCurve::MonotoneY),
    ]);
}

#[test]
fn swimlane_spline_curve_config_lowers_to_semantic_ir() {
    use diagram_ir::SwimlaneEdgeCurve;

    let diagram = parse_swimlane(
        "swimlane-beta LR\nsubgraph Intake\n  first[First]\n  second[Second]\n  third[Third]\nend\nsubgraph Review\n  fourth[Fourth]\n  fifth[Fifth]\n  sixth[Sixth]\nend\nfirst cardinal@--> sixth\nsecond catmull@--> fifth\nthird natural@--> fourth\ncardinal@{ curve: cardinal }\ncatmull@{ curve: catmullRom }\nnatural@{ curve: natural }",
    ).expect("Flowchart spline curve configuration should parse");

    assert_eq!(diagram.edges.iter().map(|edge| edge.curve).collect::<Vec<_>>(), vec![
        Some(SwimlaneEdgeCurve::Cardinal),
        Some(SwimlaneEdgeCurve::CatmullRom),
        Some(SwimlaneEdgeCurve::Natural),
    ]);
}

#[test]
fn swimlane_rounded_curve_config_lowers_to_semantic_ir() {
    use diagram_ir::SwimlaneEdgeCurve;

    let diagram = parse_swimlane(
        "swimlane-beta LR\nsubgraph Intake\n  request[Request]\n  queued[Queued]\nend\nsubgraph Review\n  review[Review]\n  done[Done]\nend\nrequest rounded@--> done\nrounded@{ curve: rounded }",
    ).expect("Flowchart rounded curve configuration should parse");

    assert_eq!(diagram.edges[0].curve, Some(SwimlaneEdgeCurve::Rounded));
}

#[test]
fn swimlane_line_hops_config_lowers_to_semantic_ir() {
    use diagram_ir::SwimlaneLineHops;

    let source = "swimlane-beta LR\nsubgraph Intake\n  first[First]\n  second[Second]\nend\nsubgraph Review\n  third[Third]\n  fourth[Fourth]\nend\nfirst --> fourth\nsecond --> third";
    let arc = parse_swimlane(&format!("---\nconfig:\n  swimlane:\n    lineHops: arc\n---\n{source}"))
        .expect("arc line hops should parse");
    let gap = parse_swimlane(&format!("---\nconfig:\n  swimlane:\n    lineHops: gap\n---\n{source}"))
        .expect("gap line hops should parse");
    let disabled = parse_swimlane(&format!("---\nconfig:\n  swimlane:\n    lineHops: false\n---\n{source}"))
        .expect("disabled line hops should parse");

    assert_eq!(arc.line_hops, Some(SwimlaneLineHops::Arc));
    assert_eq!(gap.line_hops, Some(SwimlaneLineHops::Gap));
    assert_eq!(disabled.line_hops, None);
}

#[test]
fn swimlane_automatic_lane_ordering_config_lowers_to_semantic_ir() {
    let source = "swimlane-beta LR\nsubgraph A\n  a[A]\nend\nsubgraph B\n  b[B]\nend\na --> b";
    let enabled = parse_swimlane(&format!("---\nconfig:\n  swimlane:\n    automaticLaneOrdering: true\n---\n{source}"))
        .expect("automatic lane ordering should parse");
    let disabled = parse_swimlane(&format!("---\nconfig:\n  swimlane:\n    automaticLaneOrdering: false\n---\n{source}"))
        .expect("disabled automatic lane ordering should parse");

    assert!(enabled.automatic_lane_ordering);
    assert!(!disabled.automatic_lane_ordering);
}

#[test]
fn swimlane_ignore_cross_lane_edges_config_lowers_to_semantic_ir() {
    let source = "swimlane-beta LR\nsubgraph A\n  a[A]\nend\nsubgraph B\n  b[B]\nend\na --> b";
    let defaulted = parse_swimlane(source).expect("default cross-lane behavior should parse");
    let enabled = parse_swimlane(&format!("---\nconfig:\n  swimlane:\n    ignoreCrossLaneEdges: true\n---\n{source}"))
        .expect("ignored cross-lane edges should parse");
    let disabled = parse_swimlane(&format!("---\nconfig:\n  swimlane:\n    ignoreCrossLaneEdges: false\n---\n{source}"))
        .expect("cross-lane rank constraints should parse");

    assert!(defaulted.ignore_cross_lane_edges);
    assert!(enabled.ignore_cross_lane_edges);
    assert!(!disabled.ignore_cross_lane_edges);
}

#[test]
fn swimlane_optimize_ranks_by_crossings_config_lowers_to_semantic_ir() {
    let source = "swimlane-beta LR\nsubgraph A\n  a[A]\nend\nsubgraph B\n  b[B]\nend\na --> b";
    let defaulted = parse_swimlane(source).expect("default rank optimization should parse");
    let enabled = parse_swimlane(&format!("---\nconfig:\n  swimlane:\n    optimizeRanksByCrossings: true\n---\n{source}"))
        .expect("rank crossing optimization should parse");
    let disabled = parse_swimlane(&format!("---\nconfig:\n  swimlane:\n    optimizeRanksByCrossings: false\n---\n{source}"))
        .expect("disabled rank crossing optimization should parse");

    assert!(defaulted.optimize_ranks_by_crossings);
    assert!(enabled.optimize_ranks_by_crossings);
    assert!(!disabled.optimize_ranks_by_crossings);
}

#[test]
fn swimlane_flowchart_spacing_config_lowers_to_semantic_ir() {
    let source = "swimlane-beta LR\nsubgraph A\n  first[First]\n  second[Second]\nend\nfirst --> second";
    let front_matter = parse_swimlane(&format!(
        "---\nconfig:\n  flowchart:\n    nodeSpacing: 96\n    rankSpacing: 28\n    diagramPadding: 32\n---\n{source}"
    )).expect("Flowchart spacing front matter should parse for Swimlane");
    let directive = parse_swimlane(&format!(
        "%%{{init: {{\"flowchart\": {{\"nodeSpacing\": 72, \"rankSpacing\": 24, \"diagramPadding\": 18}}}}}}%%\n{source}"
    )).expect("Flowchart spacing init directive should parse for Swimlane");

    assert_eq!(front_matter.flowchart_config.node_spacing, Some(96.0));
    assert_eq!(front_matter.flowchart_config.rank_spacing, Some(28.0));
    assert_eq!(front_matter.flowchart_config.diagram_padding, Some(32.0));
    assert_eq!(directive.flowchart_config.node_spacing, Some(72.0));
    assert_eq!(directive.flowchart_config.rank_spacing, Some(24.0));
    assert_eq!(directive.flowchart_config.diagram_padding, Some(18.0));
    assert!(parse_swimlane(&format!(
        "---\nconfig:\n  flowchart:\n    rankSpacing: -1\n---\n{source}"
    )).is_err(), "negative Flowchart spacing must be rejected");
}

#[test]
fn swimlane_flowchart_wrapping_width_lowers_to_semantic_ir() {
    let source = "swimlane-beta LR\nsubgraph A\n  first[Review a substantially longer customer request label]\nend";
    let front_matter = parse_swimlane(&format!(
        "---\nconfig:\n  flowchart:\n    wrappingWidth: 180\n---\n{source}"
    )).expect("Flowchart wrapping width front matter should parse for Swimlane");
    let directive = parse_swimlane(&format!(
        "%%{{init: {{\"flowchart\": {{\"wrappingWidth\": 220}}}}}}%%\n{source}"
    )).expect("Flowchart wrapping width init directive should parse for Swimlane");

    assert_eq!(front_matter.flowchart_config.wrapping_width, Some(180.0));
    assert_eq!(directive.flowchart_config.wrapping_width, Some(220.0));
    assert!(parse_swimlane(&format!(
        "---\nconfig:\n  flowchart:\n    wrappingWidth: -1\n---\n{source}"
    )).is_err(), "negative Flowchart wrapping width must be rejected");
}

#[test]
fn swimlane_flowchart_node_padding_lowers_to_semantic_ir() {
    let source = "swimlane-beta LR\nsubgraph A\n  first[Review]\nend";
    let front_matter = parse_swimlane(&format!(
        "---\nconfig:\n  flowchart:\n    padding: 24\n---\n{source}"
    )).expect("Flowchart node padding front matter should parse for Swimlane");
    let directive = parse_swimlane(&format!(
        "%%{{init: {{\"flowchart\": {{\"padding\": 20}}}}}}%%\n{source}"
    )).expect("Flowchart node padding init directive should parse for Swimlane");

    assert_eq!(front_matter.flowchart_config.padding, Some(24.0));
    assert_eq!(directive.flowchart_config.padding, Some(20.0));
    assert!(parse_swimlane(&format!(
        "---\nconfig:\n  flowchart:\n    padding: -1\n---\n{source}"
    )).is_err(), "negative Flowchart node padding must be rejected");
}

#[test]
fn swimlane_flowchart_title_margin_lowers_to_semantic_ir() {
    let source = "swimlane-beta LR\ntitle Approval flow\nsubgraph A\n  first[Review]\nend";
    let front_matter = parse_swimlane(&format!(
        "---\nconfig:\n  flowchart:\n    titleTopMargin: 26\n---\n{source}"
    )).expect("Flowchart title margin front matter should parse for Swimlane");
    let directive = parse_swimlane(&format!(
        "%%{{init: {{\"flowchart\": {{\"titleTopMargin\": 18}}}}}}%%\n{source}"
    )).expect("Flowchart title margin init directive should parse for Swimlane");

    assert_eq!(front_matter.flowchart_config.title_top_margin, Some(26.0));
    assert_eq!(directive.flowchart_config.title_top_margin, Some(18.0));
    assert!(parse_swimlane(&format!(
        "---\nconfig:\n  flowchart:\n    titleTopMargin: -1\n---\n{source}"
    )).is_err(), "negative Flowchart title margin must be rejected");
}

#[test]
fn swimlane_flowchart_subgraph_title_margins_lower_to_semantic_ir() {
    let source = "swimlane-beta LR\nsubgraph Review\n  first[Review]\nend";
    let front_matter = parse_swimlane(&format!(
        "---\nconfig:\n  flowchart:\n    subGraphTitleMargin:\n      top: 12\n      bottom: 18\n---\n{source}"
    )).expect("Flowchart subgraph title margins should parse from Swimlane front matter");
    let directive = parse_swimlane(&format!(
        "%%{{init: {{\"flowchart\": {{\"subGraphTitleMargin\": {{\"top\": 8, \"bottom\": 14}}}}}}}}%%\n{source}"
    )).expect("Flowchart subgraph title margins should parse from a Swimlane init directive");

    assert_eq!(front_matter.flowchart_config.subgraph_title_margin_top, Some(12.0));
    assert_eq!(front_matter.flowchart_config.subgraph_title_margin_bottom, Some(18.0));
    assert_eq!(directive.flowchart_config.subgraph_title_margin_top, Some(8.0));
    assert_eq!(directive.flowchart_config.subgraph_title_margin_bottom, Some(14.0));
    assert!(parse_swimlane(&format!(
        "---\nconfig:\n  flowchart:\n    subGraphTitleMargin:\n      top: -1\n---\n{source}"
    )).is_err(), "negative Flowchart subgraph title margins must be rejected");
}

#[test]
fn swimlane_flowchart_curve_config_lowers_to_semantic_ir() {
    use diagram_ir::SwimlaneEdgeCurve;

    let source = "swimlane-beta LR\nsubgraph A\n  first[First]\n  second[Second]\n  third[Third]\nend\nfirst --> second\nsecond override@--> third\noverride@{ curve: linear }";
    let front_matter = parse_swimlane(&format!(
        "---\nconfig:\n  flowchart:\n    curve: stepAfter\n---\n{source}"
    )).expect("Flowchart curve front matter should parse for Swimlane");
    let directive = parse_swimlane(&format!(
        "%%{{init: {{\"flowchart\": {{\"curve\": \"basis\"}}}}}}%%\n{source}"
    )).expect("Flowchart curve init directive should parse for Swimlane");

    assert_eq!(front_matter.flowchart_config.curve, Some(SwimlaneEdgeCurve::StepAfter));
    assert_eq!(directive.flowchart_config.curve, Some(SwimlaneEdgeCurve::Basis));
    assert_eq!(front_matter.edges[0].curve, None,
        "the parser must keep the global curve separate from per-edge overrides");
    assert_eq!(front_matter.edges[1].curve, Some(SwimlaneEdgeCurve::Linear));
    assert!(parse_swimlane(&format!(
        "---\nconfig:\n  flowchart:\n    curve: unsupported\n---\n{source}"
    )).is_err(), "unsupported Flowchart curves must be rejected");
}

#[test]
fn swimlane_nested_subgraphs_reach_semantic_ir() {
    let diagram = parse_swimlane("swimlane-beta LR\nsubgraph Engineering\n  subgraph Platform[Platform]\n    plan[Plan]\n    subgraph Runtime[Runtime]\n      build[Build]\n    end\n  end\n  deploy[Deploy]\nend\nsubgraph Operations\n  observe[Observe]\nend\nplan --> build --> deploy --> observe")
        .expect("nested Swimlane subgraphs should parse");

    assert_eq!(diagram.lanes.iter().map(|lane| lane.id.as_str()).collect::<Vec<_>>(),
        vec!["Engineering", "Operations"]);
    assert_eq!(diagram.groups.len(), 2);
    assert_eq!(diagram.groups[0].lane_id, "Engineering");
    assert_eq!(diagram.groups[0].parent_id, None);
    assert_eq!(diagram.groups[0].node_ids, vec!["plan"]);
    assert_eq!(diagram.groups[1].parent_id.as_deref(), Some("Platform"));
    assert_eq!(diagram.groups[1].node_ids, vec!["build"]);
}

#[test]
fn swimlane_subgraph_styles_reach_semantic_ir() {
    let diagram = parse_swimlane(
        "swimlane-beta LR\nsubgraph Review[Review lane]\n  subgraph Approval[Approval phase]\n    request[Request]\n    approve[Approve]\n  end\nend\nclassDef phase fill:#fef3c7,stroke:#b45309,stroke-width:3px,stroke-dasharray:5 3,color:#7c2d12,font-size:17px,font-weight:bold,rx:12px,ry:12px\nclass Approval phase\nstyle Review fill:#ecfdf5,stroke:#047857,stroke-width:2px,color:#064e3b,font-size:20px,rx:14px,ry:14px\nrequest --> approve",
    ).expect("Swimlane subgraph styles should parse");

    assert_eq!(diagram.lanes[0].style.fill.as_deref(), Some("#ecfdf5"));
    assert_eq!(diagram.lanes[0].style.corner_radius, Some(14.0));
    assert_eq!(diagram.groups[0].classes, ["phase"]);
    assert_eq!(diagram.groups[0].style.stroke_width, Some(3.0));
    assert_eq!(diagram.groups[0].style.font_weight, Some(700));
}

#[test]
fn swimlane_subgraph_class_decorators_reach_semantic_ir() {
    let diagram = parse_swimlane(
        "swimlane-beta LR\nsubgraph Review[Review lane]:::laneStyle\n  subgraph Approval[Approval phase]:::phaseStyle\n    request[Request]\n    approve[Approve]\n  end\nend\nclassDef laneStyle fill:#eff6ff,stroke:#1d4ed8,stroke-width:2px,color:#172554,rx:14px,ry:14px\nclassDef phaseStyle fill:#fef3c7,stroke:#b45309,stroke-width:3px,stroke-dasharray:5 3,color:#7c2d12,font-size:17px,font-weight:bold,rx:12px,ry:12px\nrequest --> approve",
    ).expect("Swimlane subgraph class decorators should parse");

    assert_eq!(diagram.lanes[0].classes, ["laneStyle"]);
    assert_eq!(diagram.lanes[0].style.stroke.as_deref(), Some("#1d4ed8"));
    assert_eq!(diagram.groups[0].classes, ["phaseStyle"]);
    assert_eq!(diagram.groups[0].style.corner_radius, Some(12.0));
    assert!(parse_swimlane(
        "swimlane-beta\nsubgraph Review:::missing\n  request[Request]\nend"
    ).is_err());
}

#[test]
fn swimlane_state_choice_and_note_attributes_reach_semantic_ir() {
    use diagram_ir::DiagramShape;
    let diagram = parse_swimlane("swimlane-beta LR\nsubgraph Workflow\n  pending@{ shape: state, label: \"Pending\" }\n  branch@{ shape: choice, label: \"Choose\" }\n  memo@{ shape: note, label: \"Decision note\" }\nend\npending --> branch --> memo")
        .expect("state, choice, and note shapes should parse");

    assert_eq!(diagram.nodes[0].shape, DiagramShape::RoundedRect);
    assert_eq!(diagram.nodes[1].shape, DiagramShape::Diamond);
    assert_eq!(diagram.nodes[2].shape, DiagramShape::Note);
}

#[test]
fn swimlane_legacy_internal_shape_aliases_reach_semantic_ir() {
    use diagram_ir::DiagramShape;
    let diagram = parse_swimlane("swimlane-beta LR\nsubgraph Legacy\n  process@{ shape: squareRect, label: \"Process\" }\n  event@{ shape: roundedRect, label: \"Event\" }\n  start@{ shape: stateStart }\n  stop@{ shape: stateEnd }\n  branch@{ shape: forkJoin }\n  manual@{ shape: inv_trapezoid, label: \"Manual\" }\n  final@{ shape: doublecircle, label: \"Final\" }\n  odd@{ shape: rect_left_inv_arrow, label: \"Odd\" }\nend\nstart --> process --> event --> branch --> manual --> final --> stop\nbranch --> odd --> final")
        .expect("legacy internal ShapeIDs accepted by Mermaid 11.16.1 should parse");

    assert_eq!(diagram.nodes.iter().map(|node| &node.shape).collect::<Vec<_>>(), vec![
        &DiagramShape::Rect,
        &DiagramShape::RoundedRect,
        &DiagramShape::SmallCircle,
        &DiagramShape::FramedCircle,
        &DiagramShape::ForkJoin,
        &DiagramShape::InvertedTrapezoid,
        &DiagramShape::DoubleCircle,
        &DiagramShape::Asymmetric,
    ]);
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
fn railroad_preserves_layout_spacing_config_for_every_notation() {
    let constructor = parse_railroad(
        "---\nconfig:\n  railroad:\n    compactMode: true\n    padding: 24\n    verticalSeparation: 30\n    horizontalSeparation: 36\n    arcRadius: 18\n---\nrailroad-beta\nvalue = terminal(\"x\");",
    ).expect("Railroad front matter config should parse");
    assert!(constructor.config.compact_mode);
    assert_eq!(constructor.config.padding, 24.0);
    assert_eq!(constructor.config.vertical_separation, 30.0);
    assert_eq!(constructor.config.horizontal_separation, 36.0);
    assert_eq!(constructor.config.arc_radius, 18.0);

    let ebnf = parse_railroad(
        "%%{init: {\"railroad\": {\"padding\": 18, \"verticalSeparation\": 20, \"horizontalSeparation\": 22}}}%%\nrailroad-ebnf-beta\nvalue = 'x';",
    ).expect("Railroad init config should parse");
    assert_eq!(ebnf.config.padding, 18.0);
    assert_eq!(ebnf.config.vertical_separation, 20.0);
    assert_eq!(ebnf.config.horizontal_separation, 22.0);
    assert!(parse_railroad(
        "---\nconfig:\n  railroad:\n    horizontalSeparation: -1\n---\nrailroad-beta\nvalue = terminal(\"x\");",
    ).is_err());
    assert!(parse_railroad(
        "---\nconfig:\n  railroad:\n    compactMode: sometimes\n---\nrailroad-beta\nvalue = terminal(\"x\");",
    ).is_err());
}

#[test]
fn railroad_preserves_backend_neutral_style_config() {
    let diagram = parse_railroad(
        "---\nconfig:\n  railroad:\n    fontSize: 18\n    fontFamily: Courier New\n    terminalFill: '#dcfce7'\n    terminalStroke: '#166534'\n    terminalTextColor: '#14532d'\n    nonTerminalFill: '#dbeafe'\n    nonTerminalStroke: '#1d4ed8'\n    nonTerminalTextColor: '#172554'\n    lineColor: '#dc2626'\n    strokeWidth: 3\n    markerFill: '#7c2d12'\n    specialFill: '#fef3c7'\n    specialStroke: '#b45309'\n    ruleNameColor: '#581c87'\n    showMarkers: false\n    markerRadius: 8\n---\nrailroad-beta\nvalue = sequence(terminal(\"x\"), nonterminal(\"item\"), special(\"end\"));",
    ).expect("Railroad style config should parse");
    assert_eq!(diagram.config.font_size, 18.0);
    assert_eq!(diagram.config.font_family, "Courier New");
    assert_eq!(diagram.config.terminal_fill, "#dcfce7");
    assert_eq!(diagram.config.non_terminal_stroke, "#1d4ed8");
    assert_eq!(diagram.config.line_color, "#dc2626");
    assert_eq!(diagram.config.stroke_width, 3.0);
    assert_eq!(diagram.config.marker_fill, "#7c2d12");
    assert_eq!(diagram.config.special_stroke, "#b45309");
    assert_eq!(diagram.config.rule_name_color, "#581c87");
    assert!(!diagram.config.show_markers);
    assert_eq!(diagram.config.marker_radius, 8.0);
    assert!(parse_railroad(
        "%%{init: {\"railroad\": {\"showMarkers\": \"sometimes\"}}}%%\nrailroad-beta\nvalue = terminal(\"x\");",
    ).is_err());
}

#[test]
fn railroad_resolves_theme_variables_before_explicit_style_overrides() {
    let diagram = parse_railroad(
        "---\nthemeVariables:\n  fontFamily: Georgia\n  fontSize: 17px\n  secondBkg: '#fef3c7'\n  secondaryBorderColor: '#b45309'\n  secondaryTextColor: '#78350f'\n  mainBkg: '#dbeafe'\n  primaryBorderColor: '#1d4ed8'\n  primaryTextColor: '#172554'\n  lineColor: '#15803d'\n  tertiaryColor: '#fce7f3'\n  tertiaryBorderColor: '#be185d'\n  titleColor: '#581c87'\nconfig:\n  railroad:\n    terminalFill: '#dcfce7'\n---\nrailroad-beta\nvalue = sequence(terminal(\"x\"), nonterminal(\"item\"), special(\"end\"));",
    ).expect("Railroad theme variables should resolve into semantic config");
    assert_eq!(diagram.config.font_family, "Georgia");
    assert_eq!(diagram.config.font_size, 17.0);
    assert_eq!(diagram.config.terminal_fill, "#dcfce7");
    assert_eq!(diagram.config.terminal_stroke, "#b45309");
    assert_eq!(diagram.config.terminal_text_color, "#78350f");
    assert_eq!(diagram.config.non_terminal_fill, "#dbeafe");
    assert_eq!(diagram.config.non_terminal_stroke, "#1d4ed8");
    assert_eq!(diagram.config.non_terminal_text_color, "#172554");
    assert_eq!(diagram.config.line_color, "#15803d");
    assert_eq!(diagram.config.marker_fill, "#15803d");
    assert_eq!(diagram.config.special_fill, "#fce7f3");
    assert_eq!(diagram.config.special_stroke, "#be185d");
    assert_eq!(diagram.config.rule_name_color, "#581c87");

    let init = parse_railroad(
        "%%{init: {\"themeVariables\": {\"secondaryColor\": \"#aabbcc\", \"lineColor\": \"#112233\"}, \"railroad\": {\"lineColor\": \"#334455\"}}}%%\nrailroad-beta\nvalue = terminal(\"x\");",
    ).expect("Railroad init theme variables should parse");
    assert_eq!(init.config.terminal_fill, "#aabbcc");
    assert_eq!(init.config.marker_fill, "#112233");
    assert_eq!(init.config.line_color, "#334455");
}

#[test]
fn railroad_sanitizes_portable_colors_and_font_families() {
    let diagram = parse_railroad(
        "---\nthemeVariables:\n  fontFamily: 'Georgia; color: red'\n  secondBkg: '#aabbcc'\n  lineColor: '#12zzzz'\nconfig:\n  railroad:\n    terminalFill: 'url(javascript:alert(1))'\n    terminalTextColor: 'hsl(120 50% 50%)'\n---\nrailroad-beta\nvalue = terminal(\"x\");",
    ).expect("Railroad invalid styles should fall back safely");
    assert_eq!(diagram.config.font_family, "monospace");
    assert_eq!(diagram.config.terminal_fill, "#aabbcc");
    assert_eq!(diagram.config.terminal_text_color, "hsl(120 50% 50%)");
    assert_eq!(diagram.config.line_color, "#000000");
    assert_eq!(diagram.config.marker_fill, "#000000");
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
