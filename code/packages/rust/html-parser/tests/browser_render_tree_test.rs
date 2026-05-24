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
    #[serde(default)]
    resource_kind: Option<String>,
    #[serde(default)]
    width: Option<String>,
    #[serde(default)]
    height: Option<String>,
    #[serde(default)]
    type_hint: Option<String>,
    #[serde(default)]
    media: Option<String>,
    #[serde(default)]
    poster: Option<String>,
    #[serde(default)]
    resolved_poster: Option<String>,
    #[serde(default)]
    preload: Option<String>,
    #[serde(default)]
    controls: bool,
    #[serde(default)]
    autoplay: bool,
    #[serde(default)]
    loop_media: bool,
    #[serde(default)]
    muted: bool,
    #[serde(default)]
    playsinline: bool,
    control_type: Option<String>,
    #[serde(default)]
    form_owner: Option<String>,
    #[serde(default)]
    label_for: Option<String>,
    #[serde(default)]
    labels: Vec<String>,
    #[serde(default)]
    accessible_name: Option<String>,
    #[serde(default)]
    placeholder: Option<String>,
    #[serde(default)]
    autocomplete: Option<String>,
    #[serde(default)]
    value: Option<String>,
    #[serde(default)]
    disabled: bool,
    #[serde(default)]
    required: bool,
    #[serde(default)]
    readonly: bool,
    #[serde(default)]
    checked: bool,
    #[serde(default)]
    selected: bool,
    #[serde(default)]
    multiple: bool,
    #[serde(default)]
    options: Vec<String>,
    #[serde(default)]
    table_section_kind: Option<String>,
    #[serde(default)]
    colspan: Option<String>,
    #[serde(default)]
    rowspan: Option<String>,
    #[serde(default)]
    span: Option<String>,
    #[serde(default)]
    scope: Option<String>,
    #[serde(default)]
    headers: Vec<String>,
    #[serde(default)]
    abbr: Option<String>,
    #[serde(default)]
    text_flow: Option<String>,
    #[serde(default)]
    list_kind: Option<String>,
    #[serde(default)]
    list_start: Option<String>,
    #[serde(default)]
    list_marker_type: Option<String>,
    #[serde(default)]
    list_reversed: bool,
    #[serde(default)]
    list_item_value: Option<String>,
    #[serde(default)]
    quote_cite: Option<String>,
    #[serde(default)]
    resolved_quote_cite: Option<String>,
    #[serde(default)]
    break_kind: Option<String>,
    #[serde(default)]
    heading_level: Option<u8>,
    #[serde(default)]
    section_kind: Option<String>,
    #[serde(default)]
    landmark_kind: Option<String>,
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
            resource_kind: self.resource_kind,
            width: self.width,
            height: self.height,
            type_hint: self.type_hint,
            media: self.media,
            poster: self.poster,
            resolved_poster: self.resolved_poster,
            preload: self.preload,
            controls: self.controls,
            autoplay: self.autoplay,
            loop_media: self.loop_media,
            muted: self.muted,
            playsinline: self.playsinline,
            control_type: self.control_type,
            form_owner: self.form_owner,
            label_for: self.label_for,
            labels: self.labels,
            accessible_name: self.accessible_name,
            placeholder: self.placeholder,
            autocomplete: self.autocomplete,
            value: self.value,
            disabled: self.disabled,
            required: self.required,
            readonly: self.readonly,
            checked: self.checked,
            selected: self.selected,
            multiple: self.multiple,
            options: self.options,
            table_section_kind: self.table_section_kind,
            colspan: self.colspan,
            rowspan: self.rowspan,
            span: self.span,
            scope: self.scope,
            headers: self.headers,
            abbr: self.abbr,
            text_flow: self.text_flow,
            list_kind: self.list_kind,
            list_start: self.list_start,
            list_marker_type: self.list_marker_type,
            list_reversed: self.list_reversed,
            list_item_value: self.list_item_value,
            quote_cite: self.quote_cite,
            resolved_quote_cite: self.resolved_quote_cite,
            break_kind: self.break_kind,
            heading_level: self.heading_level,
            section_kind: self.section_kind,
            landmark_kind: self.landmark_kind,
            children: self
                .children
                .into_iter()
                .map(ExpectedRenderNode::into_browser_render_node)
                .collect(),
        }
    }
}
