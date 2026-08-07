use coding_adventures_html_parser::{
    parse_browser_render_tree, parse_browser_render_tree_with_document_url, BrowserRenderNode,
    BrowserRenderTree,
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
    attributionsrc: Vec<String>,
    #[serde(default)]
    resolved_attributionsrc: Vec<String>,
    #[serde(default)]
    hreflang: Option<String>,
    src: Option<String>,
    #[serde(default)]
    resolved_src: Option<String>,
    alt: Option<String>,
    #[serde(default)]
    resource_kind: Option<String>,
    #[serde(default)]
    slot: Option<String>,
    #[serde(default)]
    slot_name: Option<String>,
    #[serde(default)]
    custom_element: bool,
    #[serde(default)]
    custom_element_name: Option<String>,
    #[serde(default)]
    custom_element_is: Option<String>,
    #[serde(default)]
    canvas_fallback_text: Option<String>,
    #[serde(default)]
    width: Option<String>,
    #[serde(default)]
    height: Option<String>,
    #[serde(default)]
    type_hint: Option<String>,
    #[serde(default)]
    image_map_name: Option<String>,
    #[serde(default)]
    image_map_shape: Option<String>,
    #[serde(default)]
    image_map_coords: Option<String>,
    #[serde(default)]
    srcset: Option<String>,
    #[serde(default)]
    resolved_srcset: Option<String>,
    #[serde(default)]
    sizes: Option<String>,
    #[serde(default)]
    track_kind: Option<String>,
    #[serde(default)]
    srclang: Option<String>,
    #[serde(default)]
    track_label: Option<String>,
    #[serde(default)]
    default_track: bool,
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
    accessible_description: Option<String>,
    #[serde(default)]
    aria_label: Option<String>,
    #[serde(default)]
    aria_labelledby: Vec<String>,
    #[serde(default)]
    aria_describedby: Vec<String>,
    #[serde(default)]
    aria_controls: Vec<String>,
    #[serde(default)]
    aria_owns: Vec<String>,
    #[serde(default)]
    aria_activedescendant: Option<String>,
    #[serde(default)]
    aria_current: Option<String>,
    #[serde(default)]
    aria_expanded: Option<String>,
    #[serde(default)]
    aria_haspopup: Option<String>,
    #[serde(default)]
    aria_modal: Option<String>,
    #[serde(default)]
    aria_pressed: Option<String>,
    #[serde(default)]
    aria_selected: Option<String>,
    #[serde(default)]
    aria_invalid: Option<String>,
    #[serde(default)]
    aria_live: Option<String>,
    #[serde(default)]
    aria_busy: Option<String>,
    #[serde(default)]
    aria_disabled: Option<String>,
    #[serde(default)]
    aria_required: Option<String>,
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
    event_handlers: Vec<String>,
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
    capture: Option<String>,
    #[serde(default)]
    inputmode: Option<String>,
    #[serde(default)]
    pattern: Option<String>,
    #[serde(default)]
    min: Option<String>,
    #[serde(default)]
    max: Option<String>,
    #[serde(default)]
    low: Option<String>,
    #[serde(default)]
    high: Option<String>,
    #[serde(default)]
    optimum: Option<String>,
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
    description_list_kind: Option<String>,
    #[serde(default)]
    term_kind: Option<String>,
    #[serde(default)]
    quote_cite: Option<String>,
    #[serde(default)]
    resolved_quote_cite: Option<String>,
    #[serde(default)]
    data_value: Option<String>,
    #[serde(default)]
    datetime: Option<String>,
    #[serde(default)]
    edit_cite: Option<String>,
    #[serde(default)]
    resolved_edit_cite: Option<String>,
    #[serde(default)]
    edit_datetime: Option<String>,
    #[serde(default)]
    item_scope: bool,
    #[serde(default)]
    item_type: Vec<String>,
    #[serde(default)]
    item_id: Option<String>,
    #[serde(default)]
    resolved_item_id: Option<String>,
    #[serde(default)]
    item_ref: Vec<String>,
    #[serde(default)]
    itemprop: Vec<String>,
    #[serde(default)]
    item_value: Option<String>,
    #[serde(default)]
    item_value_url: Option<String>,
    #[serde(default)]
    resolved_item_value_url: Option<String>,
    #[serde(default)]
    ruby_kind: Option<String>,
    #[serde(default)]
    bidi_kind: Option<String>,
    #[serde(default)]
    break_kind: Option<String>,
    #[serde(default)]
    grouping_kind: Option<String>,
    #[serde(default)]
    disclosure_kind: Option<String>,
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

#[test]
fn navigation_document_url_resolves_relative_links_and_images() {
    let tree = parse_browser_render_tree_with_document_url(
        "<body><p><a href=next.html>Next</a><img src=../images/logo.gif alt=Logo>",
        "http://example.test/docs/current.html",
    )
    .expect("navigation HTML should parse");

    let link = find_render_node(&tree.children, "link").expect("link should be projected");
    assert_eq!(
        link.resolved_href.as_deref(),
        Some("http://example.test/docs/next.html")
    );

    let image = find_render_node(&tree.children, "image").expect("image should be projected");
    assert_eq!(
        image.resolved_src.as_deref(),
        Some("http://example.test/images/logo.gif")
    );
}

#[test]
fn relative_authored_base_resolves_against_navigation_document_url() {
    let tree = parse_browser_render_tree_with_document_url(
        "<head><base href=../assets/ ></head><body><a href=guide.html>Guide</a>",
        "http://example.test/docs/current.html",
    )
    .expect("navigation HTML should parse");

    let link = find_render_node(&tree.children, "link").expect("link should be projected");
    assert_eq!(
        link.resolved_href.as_deref(),
        Some("http://example.test/assets/guide.html")
    );
}

fn find_render_node<'a>(
    nodes: &'a [BrowserRenderNode],
    role: &str,
) -> Option<&'a BrowserRenderNode> {
    nodes.iter().find_map(|node| {
        if node.role == role {
            Some(node)
        } else {
            find_render_node(&node.children, role)
        }
    })
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
            attributionsrc: self.attributionsrc,
            resolved_attributionsrc: self.resolved_attributionsrc,
            hreflang: self.hreflang,
            src: self.src,
            resolved_src: self.resolved_src,
            alt: self.alt,
            resource_kind: self.resource_kind,
            slot: self.slot,
            slot_name: self.slot_name,
            custom_element: self.custom_element,
            custom_element_name: self.custom_element_name,
            custom_element_is: self.custom_element_is,
            canvas_fallback_text: self.canvas_fallback_text,
            width: self.width,
            height: self.height,
            type_hint: self.type_hint,
            image_map_name: self.image_map_name,
            image_map_shape: self.image_map_shape,
            image_map_coords: self.image_map_coords,
            srcset: self.srcset,
            resolved_srcset: self.resolved_srcset,
            sizes: self.sizes,
            track_kind: self.track_kind,
            srclang: self.srclang,
            track_label: self.track_label,
            default_track: self.default_track,
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
            accessible_description: self.accessible_description,
            aria_label: self.aria_label,
            aria_labelledby: self.aria_labelledby,
            aria_describedby: self.aria_describedby,
            aria_controls: self.aria_controls,
            aria_owns: self.aria_owns,
            aria_activedescendant: self.aria_activedescendant,
            aria_current: self.aria_current,
            aria_expanded: self.aria_expanded,
            aria_haspopup: self.aria_haspopup,
            aria_modal: self.aria_modal,
            aria_pressed: self.aria_pressed,
            aria_selected: self.aria_selected,
            aria_invalid: self.aria_invalid,
            aria_live: self.aria_live,
            aria_busy: self.aria_busy,
            aria_disabled: self.aria_disabled,
            aria_required: self.aria_required,
            aria_hidden: self.aria_hidden,
            hidden: self.hidden,
            inert: self.inert,
            open: self.open,
            tabindex: self.tabindex,
            accesskey: self.accesskey,
            event_handlers: self.event_handlers,
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
            capture: self.capture,
            inputmode: self.inputmode,
            pattern: self.pattern,
            min: self.min,
            max: self.max,
            low: self.low,
            high: self.high,
            optimum: self.optimum,
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
            description_list_kind: self.description_list_kind,
            term_kind: self.term_kind,
            quote_cite: self.quote_cite,
            resolved_quote_cite: self.resolved_quote_cite,
            data_value: self.data_value,
            datetime: self.datetime,
            edit_cite: self.edit_cite,
            resolved_edit_cite: self.resolved_edit_cite,
            edit_datetime: self.edit_datetime,
            item_scope: self.item_scope,
            item_type: self.item_type,
            item_id: self.item_id,
            resolved_item_id: self.resolved_item_id,
            item_ref: self.item_ref,
            itemprop: self.itemprop,
            item_value: self.item_value,
            item_value_url: self.item_value_url,
            resolved_item_value_url: self.resolved_item_value_url,
            ruby_kind: self.ruby_kind,
            bidi_kind: self.bidi_kind,
            break_kind: self.break_kind,
            grouping_kind: self.grouping_kind,
            disclosure_kind: self.disclosure_kind,
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
