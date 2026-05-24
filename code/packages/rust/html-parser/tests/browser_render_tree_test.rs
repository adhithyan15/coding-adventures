use coding_adventures_html_parser::{
    parse_browser_render_tree, BrowserRenderNode, BrowserRenderTree,
};
use serde::Deserialize;

const BROWSER_RENDER_TREE_FIXTURE: &str = include_str!("fixtures/html-browser-render-tree.json");

#[derive(Debug, Deserialize)]
struct BrowserRenderTreeSuite {
    format: String,
    suite: String,
    cases: Vec<BrowserRenderTreeCase>,
}

#[derive(Debug, Deserialize)]
struct BrowserRenderTreeCase {
    id: String,
    input: String,
    expected: ExpectedRenderTree,
}

#[derive(Debug, Deserialize)]
struct ExpectedRenderTree {
    children: Vec<ExpectedRenderNode>,
}

#[derive(Debug, Deserialize)]
struct ExpectedRenderNode {
    display: String,
    role: String,
    name: Option<String>,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    classes: Vec<String>,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    lang: Option<String>,
    #[serde(default)]
    dir: Option<String>,
    text: Option<String>,
    href: Option<String>,
    #[serde(default)]
    resolved_href: Option<String>,
    src: Option<String>,
    #[serde(default)]
    resolved_src: Option<String>,
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
    children: Vec<ExpectedRenderNode>,
}

#[test]
fn browser_render_tree_cases_extract_default_display_structure() {
    let suite: BrowserRenderTreeSuite = serde_json::from_str(BROWSER_RENDER_TREE_FIXTURE)
        .expect("browser render tree fixture should parse");

    assert_eq!(suite.format, "venture-html-browser-render-tree/v1");
    assert_eq!(suite.suite, "browser-render-tree");
    assert!(!suite.cases.is_empty(), "fixture should contain cases");

    for case in suite.cases {
        let actual = parse_browser_render_tree(&case.input)
            .unwrap_or_else(|error| panic!("{} should parse: {error}", case.id));

        assert_eq!(
            actual,
            case.expected.into_browser_render_tree(),
            "{} extracted browser render tree should match",
            case.id
        );
    }
}

impl ExpectedRenderTree {
    fn into_browser_render_tree(self) -> BrowserRenderTree {
        BrowserRenderTree {
            children: self
                .children
                .into_iter()
                .map(ExpectedRenderNode::into_browser_render_node)
                .collect(),
        }
    }
}

impl ExpectedRenderNode {
    fn into_browser_render_node(self) -> BrowserRenderNode {
        BrowserRenderNode {
            display: self.display,
            role: self.role,
            name: self.name,
            id: self.id,
            classes: self.classes,
            title: self.title,
            lang: self.lang,
            dir: self.dir,
            text: self.text,
            href: self.href,
            resolved_href: self.resolved_href,
            src: self.src,
            resolved_src: self.resolved_src,
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
                .map(ExpectedRenderNode::into_browser_render_node)
                .collect(),
        }
    }
}
