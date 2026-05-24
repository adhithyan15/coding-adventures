use coding_adventures_html_parser::{
    parse_browser_content_tree, BrowserContentNode, BrowserContentTree,
};
use serde::Deserialize;

const BROWSER_CONTENT_TREE_FIXTURE: &str = include_str!("fixtures/html-browser-content-tree.json");

#[derive(Debug, Deserialize)]
struct BrowserContentTreeSuite {
    format: String,
    suite: String,
    cases: Vec<BrowserContentTreeCase>,
}

#[derive(Debug, Deserialize)]
struct BrowserContentTreeCase {
    id: String,
    input: String,
    expected: ExpectedContentTree,
}

#[derive(Debug, Deserialize)]
struct ExpectedContentTree {
    children: Vec<ExpectedContentNode>,
}

#[derive(Debug, Deserialize)]
struct ExpectedContentNode {
    role: String,
    name: Option<String>,
    text: Option<String>,
    href: Option<String>,
    src: Option<String>,
    alt: Option<String>,
    control_type: Option<String>,
    #[serde(default)]
    value: Option<String>,
    #[serde(default)]
    disabled: bool,
    #[serde(default)]
    checked: bool,
    #[serde(default)]
    selected: bool,
    #[serde(default)]
    options: Vec<String>,
    children: Vec<ExpectedContentNode>,
}

#[test]
fn browser_content_tree_cases_extract_renderable_body_structure() {
    let suite: BrowserContentTreeSuite = serde_json::from_str(BROWSER_CONTENT_TREE_FIXTURE)
        .expect("browser content tree fixture should parse");

    assert_eq!(suite.format, "venture-html-browser-content-tree/v1");
    assert_eq!(suite.suite, "browser-content-tree");
    assert!(!suite.cases.is_empty(), "fixture should contain cases");

    for case in suite.cases {
        let actual = parse_browser_content_tree(&case.input)
            .unwrap_or_else(|error| panic!("{} should parse: {error}", case.id));

        assert_eq!(
            actual,
            case.expected.into_browser_content_tree(),
            "{} extracted browser content tree should match",
            case.id
        );
    }
}

impl ExpectedContentTree {
    fn into_browser_content_tree(self) -> BrowserContentTree {
        BrowserContentTree {
            children: self
                .children
                .into_iter()
                .map(ExpectedContentNode::into_browser_content_node)
                .collect(),
        }
    }
}

impl ExpectedContentNode {
    fn into_browser_content_node(self) -> BrowserContentNode {
        BrowserContentNode {
            role: self.role,
            name: self.name,
            text: self.text,
            href: self.href,
            src: self.src,
            alt: self.alt,
            control_type: self.control_type,
            value: self.value,
            disabled: self.disabled,
            checked: self.checked,
            selected: self.selected,
            options: self.options,
            children: self
                .children
                .into_iter()
                .map(ExpectedContentNode::into_browser_content_node)
                .collect(),
        }
    }
}
