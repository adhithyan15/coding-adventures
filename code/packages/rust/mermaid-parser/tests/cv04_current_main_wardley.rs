//! Incoming accessibility/link flows must coexist with delivered active quotes.
//! Use the public parser and shared old fields so the parent baseline compiles.
use mermaid_parser::parse_wardley;

#[test]
fn current_main_accessibility_and_flow_keep_pipeline_quote_owned_delimiters() {
    let diagram = parse_wardley(
        "wardley-beta\naccTitle: Map\naccDescr: Visible map\ncomponent P [0.5,0.1]\npipeline P {\ncomponent \"α [quoted] \\\"C\\\"\" [0.25]\n}\ncomponent B [0.5,0.75]\nP +> B\nannotation 3,[0.25,0.5] 'A%%[]'\n",
    ).unwrap();
    assert_eq!(diagram.nodes.len(), 3);
    assert_eq!(diagram.nodes[1].label, "α [quoted] \"C\"");
    assert_eq!(diagram.pipelines.len(), 1);
    assert_eq!(
        diagram.pipelines[0].component_ids,
        vec![diagram.nodes[1].id.clone()]
    );
    assert_eq!(diagram.links.len(), 1);
    assert_eq!(diagram.links[0].source, "P");
    assert_eq!(diagram.links[0].target, "B");
    assert_eq!(diagram.annotations[0].text, "A%%[]");
    assert_eq!(diagram.annotations[0].coordinates, vec![(0.25, 0.5)]);
    println!("CV04_MAIN_WARDLEY_MIXED accepted");
}

#[test]
fn current_main_accessibility_does_not_hide_malformed_quoted_pipeline_suffix() {
    let error = parse_wardley("wardley-beta\naccTitle: Map\ncomponent P [0.5,0.1]\npipeline P {\ncomponent 'C' junk [0.25]\n}\n").unwrap_err();
    assert!(format!("{error:?}").contains("suffix"), "{error:?}");
    println!("CV04_MAIN_WARDLEY_SUFFIX refused");
}
