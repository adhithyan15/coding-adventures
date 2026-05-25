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
    #[serde(default)]
    authored_role: Option<String>,
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
    #[serde(default)]
    target: Option<String>,
    #[serde(default)]
    rel: Option<String>,
    #[serde(default)]
    rel_tokens: Vec<String>,
    #[serde(default)]
    download: Option<String>,
    #[serde(default)]
    ping: Vec<String>,
    #[serde(default)]
    resolved_ping: Vec<String>,
    #[serde(default)]
    hreflang: Option<String>,
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
    #[serde(default)]
    browsing_context_name: Option<String>,
    #[serde(default)]
    loading: Option<String>,
    #[serde(default)]
    sandbox: Vec<String>,
    #[serde(default)]
    allow: Option<String>,
    #[serde(default)]
    allowfullscreen: bool,
    #[serde(default)]
    referrerpolicy: Option<String>,
    #[serde(default)]
    srcdoc: Option<String>,
    #[serde(default)]
    credentialless: bool,
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
    aria_label: Option<String>,
    #[serde(default)]
    aria_labelledby: Vec<String>,
    #[serde(default)]
    aria_describedby: Vec<String>,
    #[serde(default)]
    aria_controls: Vec<String>,
    #[serde(default)]
    aria_current: Option<String>,
    #[serde(default)]
    aria_expanded: Option<String>,
    #[serde(default)]
    aria_pressed: Option<String>,
    #[serde(default)]
    aria_selected: Option<String>,
    #[serde(default)]
    aria_hidden: bool,
    #[serde(default)]
    hidden: bool,
    #[serde(default)]
    inert: bool,
    #[serde(default)]
    open: bool,
    #[serde(default)]
    tabindex: Option<String>,
    #[serde(default)]
    accesskey: Vec<String>,
    #[serde(default)]
    focusable: Option<bool>,
    #[serde(default)]
    contenteditable: Option<String>,
    #[serde(default)]
    editing_mode: Option<String>,
    #[serde(default)]
    draggable: Option<String>,
    #[serde(default)]
    draggable_state: Option<String>,
    #[serde(default)]
    spellcheck: Option<String>,
    #[serde(default)]
    translate: Option<String>,
    #[serde(default)]
    popover: Option<String>,
    #[serde(default)]
    popover_target: Option<String>,
    #[serde(default)]
    popover_target_action: Option<String>,
    #[serde(default)]
    command: Option<String>,
    #[serde(default)]
    command_for: Option<String>,
    #[serde(default)]
    placeholder: Option<String>,
    #[serde(default)]
    autocomplete: Option<String>,
    #[serde(default)]
    autocapitalize: Option<String>,
    #[serde(default)]
    enterkeyhint: Option<String>,
    #[serde(default)]
    dirname: Option<String>,
    #[serde(default)]
    accept: Option<String>,
    #[serde(default)]
    inputmode: Option<String>,
    #[serde(default)]
    pattern: Option<String>,
    #[serde(default)]
    min: Option<String>,
    #[serde(default)]
    max: Option<String>,
    #[serde(default)]
    step: Option<String>,
    #[serde(default)]
    minlength: Option<String>,
    #[serde(default)]
    maxlength: Option<String>,
    #[serde(default)]
    size: Option<String>,
    #[serde(default)]
    list: Option<String>,
    #[serde(default)]
    form_action: Option<String>,
    #[serde(default)]
    resolved_form_action: Option<String>,
    #[serde(default)]
    form_enctype: Option<String>,
    #[serde(default)]
    form_method: Option<String>,
    #[serde(default)]
    form_target: Option<String>,
    #[serde(default)]
    form_novalidate: bool,
    #[serde(default)]
    value: Option<String>,
    #[serde(default)]
    autofocus: bool,
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
            authored_role: self.authored_role,
            name: self.name,
            id: self.id,
            classes: self.classes,
            title: self.title,
            lang: self.lang,
            dir: self.dir,
            text: self.text,
            href: self.href,
            resolved_href: self.resolved_href,
            target: self.target,
            rel: self.rel,
            rel_tokens: self.rel_tokens,
            download: self.download,
            ping: self.ping,
            resolved_ping: self.resolved_ping,
            hreflang: self.hreflang,
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
            browsing_context_name: self.browsing_context_name,
            loading: self.loading,
            sandbox: self.sandbox,
            allow: self.allow,
            allowfullscreen: self.allowfullscreen,
            referrerpolicy: self.referrerpolicy,
            srcdoc: self.srcdoc,
            credentialless: self.credentialless,
            control_type: self.control_type,
            form_owner: self.form_owner,
            label_for: self.label_for,
            labels: self.labels,
            accessible_name: self.accessible_name,
            aria_label: self.aria_label,
            aria_labelledby: self.aria_labelledby,
            aria_describedby: self.aria_describedby,
            aria_controls: self.aria_controls,
            aria_current: self.aria_current,
            aria_expanded: self.aria_expanded,
            aria_pressed: self.aria_pressed,
            aria_selected: self.aria_selected,
            aria_hidden: self.aria_hidden,
            hidden: self.hidden,
            inert: self.inert,
            open: self.open,
            tabindex: self.tabindex,
            accesskey: self.accesskey,
            focusable: self.focusable,
            contenteditable: self.contenteditable,
            editing_mode: self.editing_mode,
            draggable: self.draggable,
            draggable_state: self.draggable_state,
            spellcheck: self.spellcheck,
            translate: self.translate,
            popover: self.popover,
            popover_target: self.popover_target,
            popover_target_action: self.popover_target_action,
            command: self.command,
            command_for: self.command_for,
            placeholder: self.placeholder,
            autocomplete: self.autocomplete,
            autocapitalize: self.autocapitalize,
            enterkeyhint: self.enterkeyhint,
            dirname: self.dirname,
            accept: self.accept,
            inputmode: self.inputmode,
            pattern: self.pattern,
            min: self.min,
            max: self.max,
            step: self.step,
            minlength: self.minlength,
            maxlength: self.maxlength,
            size: self.size,
            list: self.list,
            form_action: self.form_action,
            resolved_form_action: self.resolved_form_action,
            form_enctype: self.form_enctype,
            form_method: self.form_method,
            form_target: self.form_target,
            form_novalidate: self.form_novalidate,
            value: self.value,
            autofocus: self.autofocus,
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
                .map(ExpectedContentNode::into_browser_content_node)
                .collect(),
        }
    }
}
