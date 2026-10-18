//! Original pre-reconciliation corpora and complete original consumer assertions.
//!
//! The live main corpora remain independent and unchanged. This target retains
//! complete owned JSON and all nine original consumer bodies, including original
//! semantic/refusal assertions. Any inherited domain extension must be migrated
//! explicitly after an actual failure; no actual-output normalization or
//! presence-only replacement is permitted.
use mermaid_parser::{parse_architecture, parse_ishikawa, parse_kanban,
    parse_mindmap, parse_timeline, parse_treemap, parse_venn, parse_wardley};
use serde_json::Value;

const ARCHITECTURE_CORPUS: &str = include_str!("current-main-legacy-corpora/architecture.json");
const ISHIKAWA_CORPUS: &str = include_str!("current-main-legacy-corpora/ishikawa.json");
const KANBAN_CORPUS: &str = include_str!("current-main-legacy-corpora/kanban.json");
const MINDMAP_CORPUS: &str = include_str!("current-main-legacy-corpora/mindmap.json");
const TIMELINE_CORPUS: &str = include_str!("current-main-legacy-corpora/timeline.json");
const TREEMAP_CORPUS: &str = include_str!("current-main-legacy-corpora/treemap.json");
const VENN_CORPUS: &str = include_str!("current-main-legacy-corpora/venn.json");
const WARDLEY_CORPUS: &str = include_str!("current-main-legacy-corpora/wardley.json");

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
            assert_eq!(diagram.config.font_size, 22.0);
            assert_eq!(diagram.config.font_family, "Georgia");
            assert_eq!(diagram.config.line_color, "#123456");
            assert_eq!(diagram.config.fill_color, "#abcdef");
            assert_eq!(diagram.config.text_color, "#654321");
        }
        if name == "deep-long-labels" {
            assert_eq!(diagram.causes.iter().map(|cause| cause.depth).max(), Some(3));
            assert!(diagram.causes.iter().any(|cause| cause.label == "Legal approval queues delay publication"));
        }
        if name == "hand-drawn-config" {
            assert_eq!(diagram.config.look, diagram_ir::IshikawaLook::HandDrawn);
            assert_eq!(diagram.config.hand_drawn_seed, 17);
        }
    }
}

#[test]
    fn kanban_unquotes_delimited_labels() {
        let board = parse_kanban(
            "kanban\n  todo[\"Todo queue\"]\n    card[\"Quoted card\"]",
        )
        .unwrap();
        assert_eq!(board.columns[0].id, "todo");
        assert_eq!(board.columns[0].label.text, "Todo queue");
        assert_eq!(board.columns[0].cards[0].label.text, "Quoted card");
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
            assert_eq!(diagram.annotations_box, Some((0.10, 0.90)));
            assert_eq!(diagram.annotations.len(), 1);
            assert_eq!(diagram.annotations[0].coordinates, [(0.60, 0.65), (0.50, 0.50)]);
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
        if name == "annotation-theme" {
            assert_eq!((&diagram.config.theme.annotation_stroke, &diagram.config.theme.annotation_text_color,
                &diagram.config.theme.annotation_fill), (&"#7c3aed".to_string(), &"#f97316".to_string(),
                &"#fef3c7".to_string()));
        }
        if name == "stage-boundaries" {
            assert_eq!(diagram.stages, ["Genesis / Concept", "Custom / Emerging", "Product / Converging", "Commodity / Accepted"]);
            assert_eq!(diagram.stage_boundaries, [0.2, 0.4, 0.75, 1.0]);
        }
        if name == "label-offsets" {
            assert_eq!((diagram.nodes[0].label_offset_x, diagram.nodes[0].label_offset_y), (Some(-50.0), Some(10.0)));
        }
        if name == "decorators" {
            assert_eq!(diagram.nodes.iter().map(|node| node.source_strategy).collect::<Vec<_>>(), [
                Some(diagram_ir::WardleySourceStrategy::Build), Some(diagram_ir::WardleySourceStrategy::Buy),
                Some(diagram_ir::WardleySourceStrategy::Outsource), Some(diagram_ir::WardleySourceStrategy::Market)]);
            assert!(diagram.nodes[3].inertia);
        }
        if name == "flow-links" {
            assert_eq!(diagram.links.len(), 6);
            assert_eq!(diagram.links[0].label.as_deref(), Some("request"));
            assert!(diagram.links[1].dashed);
            assert_eq!(diagram.links[2].flow, Some(diagram_ir::WardleyLinkFlow::Forward));
            assert_eq!(diagram.links[3].flow, Some(diagram_ir::WardleyLinkFlow::Backward));
            assert_eq!(diagram.links[4].flow, Some(diagram_ir::WardleyLinkFlow::Bidirectional));
            assert_eq!(diagram.links[4].label.as_deref(), Some("sync"));
            assert_eq!(diagram.links[5].flow, Some(diagram_ir::WardleyLinkFlow::Forward));
        }
        if name == "accessibility" {
            assert_eq!(diagram.accessibility_title.as_deref(), Some("Accessible strategy map"));
            assert_eq!(diagram.accessibility_description.as_deref(),
                Some("Components and dependencies\nacross the value chain"));
        }
        if name == "forces" {
            assert_eq!(diagram.forces.len(), 2);
            assert_eq!(diagram.forces[0].label, "AI Adoption");
            assert_eq!(diagram.forces[0].kind, diagram_ir::WardleyForceKind::Accelerator);
            assert_eq!((diagram.forces[1].visibility, diagram.forces[1].evolution), (0.15, 0.75));
            assert_eq!(diagram.forces[1].kind, diagram_ir::WardleyForceKind::Deaccelerator);
        }
        if name == "pipeline" {
            assert_eq!(diagram.pipelines.len(), 1);
            assert_eq!(diagram.pipelines[0].component_ids.len(), 4);
            assert_eq!(diagram.nodes.len(), 5);
            assert!(diagram.nodes[0].pipeline_parent);
            assert_eq!((diagram.nodes[1].label.as_str(), diagram.nodes[1].visibility, diagram.nodes[1].evolution),
                ("File System", 0.40, 0.25));
            assert_eq!((diagram.nodes[1].label_offset_x, diagram.nodes[1].label_offset_y), (Some(-12.0), Some(8.0)));
        }
    }
}
