//! The header owns direction; title, accessibility and configuration text do not.
//! Whole IR expectations also prevent a direction repair from widening readers.
use diagram_ir::{
    TimelineConfig, TimelineDiagram, TimelineDirection, TimelinePeriod, TimelineSection,
};
use mermaid_parser::parse_timeline;

fn config(padding: f64, left_margin: f64) -> TimelineConfig {
    TimelineConfig {
        padding,
        left_margin,
        use_max_width: false,
        disable_multicolor: false,
        section_colors: [
            "#dbeafe", "#dcfce7", "#fef3c7", "#fae8ff", "#ffe4e6", "#cffafe",
        ]
        .map(str::to_string)
        .to_vec(),
        section_label_colors: vec!["#0f172a".into(); 6],
        section_inverse_colors: [
            "#2563eb", "#16a34a", "#d97706", "#a21caf", "#e11d48", "#0891b2",
        ]
        .map(str::to_string)
        .to_vec(),
    }
}

fn complete_headers(headers: &[(&str, TimelineDirection)]) {
    for &(header, direction) in headers {
        for context in 0..4 {
            let prefix = match context {
                2 => "%%{init: {\"timeline\":{\"padding\":72,\"leftMargin\":80}}}%%\n",
                3 => "---\nconfig:\n  timeline:\n    padding: 72\n    leftMargin: 80\n---\n",
                _ => "",
            };
            let description = if context == 1 {
                "timeline TD\n%%{init: {\"timeline\":{\"padding\":999}}}%%"
            } else {
                "timeline TD remains description"
            };
            let accessibility = if context == 1 {
                format!("accDescr {{\n{description}\n}}\n")
            } else {
                format!("accDescr: {description}\n")
            };
            let source = format!("{prefix}{header}\ntitle timeline TD\naccTitle: timeline LR\n{accessibility}section timeline TD\nP : E : still one event\n");
            let expected = (
                Some("timeline TD".into()),
                TimelineDiagram {
                    accessibility_title: Some("timeline LR".into()),
                    // Shared preprocessing blanks directive-like spans before
                    // accessibility tokenization; keep the authored source above.
                    accessibility_description: Some(if context == 1 {
                        "timeline TD".into()
                    } else {
                        description.into()
                    }),
                    config: if context >= 2 {
                        config(72.0, 80.0)
                    } else {
                        config(50.0, 50.0)
                    },
                    direction,
                    sections: vec![TimelineSection {
                        label: Some("timeline TD".into()),
                        periods: vec![TimelinePeriod {
                            label: "P".into(),
                            events: vec!["E : still one event".into()],
                        }],
                    }],
                },
            );
            assert_eq!(
                parse_timeline(&source).unwrap(),
                expected,
                "whole IR {header}:{context}"
            );
            println!("timeline-direction-source COMPLETE {header}:{context}");
        }
        let source = format!("{header}\n");
        assert_eq!(
            parse_timeline(&source).unwrap(),
            (
                None,
                TimelineDiagram {
                    accessibility_title: None,
                    accessibility_description: None,
                    config: config(50.0, 50.0),
                    direction,
                    sections: Vec::new(),
                }
            ),
            "whole empty IR {header}"
        );
        println!("timeline-direction-empty COMPLETE {header}");
    }
}

#[test]
fn bare_headers_have_complete_left_right_ir() {
    complete_headers(&[
        ("timeline", TimelineDirection::LeftRight),
        ("TiMeLiNe", TimelineDirection::LeftRight),
    ]);
}

#[test]
fn explicit_headers_keep_complete_ir_and_owned_content() {
    complete_headers(&[
        ("timeline TD", TimelineDirection::TopDown),
        ("TiMeLiNe\ttD", TimelineDirection::TopDown),
        ("timeline LR", TimelineDirection::LeftRight),
        ("TiMeLiNe\tlR", TimelineDirection::LeftRight),
    ]);
}

// Exact old bare-source bytes, retained separately from the original fixtures.
// Their former oracles checked acceptance/configuration; this adds full LR IR.
#[test]
fn retained_bare_sources_have_complete_left_right_ir() {
    // Original code/grammars/mermaid/compatibility.json: $.families[13].smoke_source
    let source = "timeline\n2026 : Started";
    let cfg = config(50.0, 50.0);
    assert_eq!(
        parse_timeline(source).unwrap(),
        (
            None,
            TimelineDiagram {
                accessibility_title: None,
                accessibility_description: None,
                config: cfg,
                direction: TimelineDirection::LeftRight,
                sections: vec![TimelineSection {
                    label: None,
                    periods: vec![TimelinePeriod {
                        label: "2026".into(),
                        events: vec!["Started".into()]
                    }]
                }],
            }
        ),
        "complete retained source 0"
    );
    println!("timeline-direction-retained COMPLETE 0");
    // Original code/grammars/mermaid/timeline-11.16.1-corpus.json: $.valid[0].source
    let source = "timeline\n  title Product history\n  section Foundation\n  2024 : Prototype\n       : First customer\n  section Growth\n  2025 : General availability";
    let cfg = config(50.0, 50.0);
    assert_eq!(
        parse_timeline(source).unwrap(),
        (
            Some("Product history".into()),
            TimelineDiagram {
                accessibility_title: None,
                accessibility_description: None,
                config: cfg,
                direction: TimelineDirection::LeftRight,
                sections: vec![
                    TimelineSection {
                        label: Some("Foundation".into()),
                        periods: vec![TimelinePeriod {
                            label: "2024".into(),
                            events: vec!["Prototype".into(), "First customer".into()]
                        }]
                    },
                    TimelineSection {
                        label: Some("Growth".into()),
                        periods: vec![TimelinePeriod {
                            label: "2025".into(),
                            events: vec!["General availability".into()]
                        }]
                    },
                ],
            }
        ),
        "complete retained source 1"
    );
    println!("timeline-direction-retained COMPLETE 1");
    // Original code/packages/rust/mermaid-parser/src/lib.rs: 16314
    let source = "%%{init: {\"timeline\": {\"padding\": 72, \"leftMargin\": 80, \"useMaxWidth\": true, \"disableMulticolor\": true}, \"themeVariables\": {\"cScale0\": \"#112233\", \"cScaleLabel0\": \"#fefefe\", \"cScaleInv0\": \"#445566\"}}}%%\ntimeline\n2025 : Public beta\n2026 : General availability\n";
    let mut cfg = config(72.0, 80.0);
    cfg.use_max_width = true;
    cfg.disable_multicolor = true;
    cfg.section_colors[0] = "#112233".into();
    cfg.section_label_colors[0] = "#fefefe".into();
    cfg.section_inverse_colors[0] = "#445566".into();
    assert_eq!(
        parse_timeline(source).unwrap(),
        (
            None,
            TimelineDiagram {
                accessibility_title: None,
                accessibility_description: None,
                config: cfg,
                direction: TimelineDirection::LeftRight,
                sections: vec![TimelineSection {
                    label: None,
                    periods: vec![
                        TimelinePeriod {
                            label: "2025".into(),
                            events: vec!["Public beta".into()]
                        },
                        TimelinePeriod {
                            label: "2026".into(),
                            events: vec!["General availability".into()]
                        }
                    ]
                }],
            }
        ),
        "complete retained source 2"
    );
    println!("timeline-direction-retained COMPLETE 2");
    // Original code/packages/rust/mermaid-parser/src/lib.rs: 16327
    let source = "---\nconfig:\n  timeline:\n    padding: 24\n    leftMargin: 36\n    useMaxWidth: true\n    disableMulticolor: true\n---\ntimeline\n2025\n";
    let mut cfg = config(24.0, 36.0);
    cfg.use_max_width = true;
    cfg.disable_multicolor = true;
    assert_eq!(
        parse_timeline(source).unwrap(),
        (
            None,
            TimelineDiagram {
                accessibility_title: None,
                accessibility_description: None,
                config: cfg,
                direction: TimelineDirection::LeftRight,
                sections: vec![TimelineSection {
                    label: None,
                    periods: vec![TimelinePeriod {
                        label: "2025".into(),
                        events: Vec::new()
                    }]
                }],
            }
        ),
        "complete retained source 3"
    );
    println!("timeline-direction-retained COMPLETE 3");
    // Original code/packages/rust/mermaid-parser/src/lib.rs: 16339
    let source = "%%{init: {\"timeline\": {\"padding\": 72, \"useMaxWidth\": true, \"disableMulticolor\": true}, \"themeVariables\": {\"cScale0\": \"#112233\", \"cScaleLabel0\": \"#fefefe\", \"cScaleInv0\": \"#445566\"}}}%%\ntimeline\n2025 : Public beta\n2026 : General availability\n";
    let mut cfg = config(72.0, 50.0);
    cfg.use_max_width = true;
    cfg.disable_multicolor = true;
    cfg.section_colors[0] = "#112233".into();
    cfg.section_label_colors[0] = "#fefefe".into();
    cfg.section_inverse_colors[0] = "#445566".into();
    assert_eq!(
        parse_timeline(source).unwrap(),
        (
            None,
            TimelineDiagram {
                accessibility_title: None,
                accessibility_description: None,
                config: cfg,
                direction: TimelineDirection::LeftRight,
                sections: vec![TimelineSection {
                    label: None,
                    periods: vec![
                        TimelinePeriod {
                            label: "2025".into(),
                            events: vec!["Public beta".into()]
                        },
                        TimelinePeriod {
                            label: "2026".into(),
                            events: vec!["General availability".into()]
                        }
                    ]
                }],
            }
        ),
        "complete retained source 4"
    );
    println!("timeline-direction-retained COMPLETE 4");
    // Original code/packages/rust/mermaid-parser/src/lib.rs: 16351
    let source = "---\nconfig:\n  timeline:\n    padding: 24\n    useMaxWidth: true\n    disableMulticolor: true\n---\ntimeline\n2025\n";
    let mut cfg = config(24.0, 50.0);
    cfg.use_max_width = true;
    cfg.disable_multicolor = true;
    assert_eq!(
        parse_timeline(source).unwrap(),
        (
            None,
            TimelineDiagram {
                accessibility_title: None,
                accessibility_description: None,
                config: cfg,
                direction: TimelineDirection::LeftRight,
                sections: vec![TimelineSection {
                    label: None,
                    periods: vec![TimelinePeriod {
                        label: "2025".into(),
                        events: Vec::new()
                    }]
                }],
            }
        ),
        "complete retained source 5"
    );
    println!("timeline-direction-retained COMPLETE 5");
    // Original code/packages/rust/mermaid-parser/src/lib.rs: 16362
    let source = "%%{init: {\"timeline\": {\"padding\": -1}}}%%\ntimeline\n2025\n";
    let cfg = config(50.0, 50.0);
    assert_eq!(
        parse_timeline(source).unwrap(),
        (
            None,
            TimelineDiagram {
                accessibility_title: None,
                accessibility_description: None,
                config: cfg,
                direction: TimelineDirection::LeftRight,
                sections: vec![TimelineSection {
                    label: None,
                    periods: vec![TimelinePeriod {
                        label: "2025".into(),
                        events: Vec::new()
                    }]
                }],
            }
        ),
        "complete retained source 6"
    );
    println!("timeline-direction-retained COMPLETE 6");
}
