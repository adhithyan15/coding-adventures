use coding_adventures_html_parser::{
    parse_browser_document, BrowserAnchor, BrowserComponentHydrationTarget, BrowserDataAttribute,
    BrowserDocument, BrowserDocumentMetadata, BrowserEmbeddedContext, BrowserForm,
    BrowserFormControl, BrowserFormFieldset, BrowserFormLabel, BrowserFormOutput,
    BrowserFormSubmitter, BrowserHeading, BrowserHttpEquivHint, BrowserImage, BrowserImageSource,
    BrowserInteractiveElement, BrowserLink, BrowserMedia, BrowserMeta, BrowserMetadataDirective,
    BrowserRefresh, BrowserResource, BrowserResourceHint, BrowserScript, BrowserSelectOption,
    BrowserStructuredItem, BrowserStructuredProperty, BrowserStylesheet, BrowserTable,
    BrowserTemplate, BrowserThemeColor,
};
use serde::Deserialize;

const BROWSER_READINESS_FIXTURE: &str = include_str!("fixtures/html-browser-readiness.json");

#[derive(Debug, Deserialize)]
struct BrowserReadinessSuite {
    format: String,
    suite: String,
    cases: Vec<BrowserReadinessCase>,
}

#[derive(Debug, Deserialize)]
struct BrowserReadinessCase {
    id: String,
    input: String,
    expected: ExpectedBrowserDocument,
}

#[derive(Debug, Deserialize)]
struct ExpectedBrowserDocument {
    title: Option<String>,
    base_href: Option<String>,
    base_target: Option<String>,
    #[serde(default)]
    metadata: ExpectedDocumentMetadata,
    #[serde(default)]
    document_lang: Option<String>,
    #[serde(default)]
    document_dir: Option<String>,
    #[serde(default)]
    body_id: Option<String>,
    #[serde(default)]
    body_classes: Vec<String>,
    #[serde(default)]
    body_lang: Option<String>,
    #[serde(default)]
    body_dir: Option<String>,
    #[serde(default)]
    document_event_handlers: Vec<String>,
    #[serde(default)]
    body_event_handlers: Vec<String>,
    body_text: String,
    metas: Vec<ExpectedMeta>,
    resources: Vec<ExpectedResource>,
    #[serde(default)]
    scripts: Vec<ExpectedScript>,
    #[serde(default)]
    stylesheets: Vec<ExpectedStylesheet>,
    anchors: Vec<ExpectedAnchor>,
    headings: Vec<ExpectedHeading>,
    links: Vec<ExpectedLink>,
    images: Vec<ExpectedImage>,
    #[serde(default)]
    media: Vec<ExpectedMedia>,
    #[serde(default)]
    embedded_contexts: Vec<ExpectedEmbeddedContext>,
    #[serde(default)]
    interactive_elements: Vec<ExpectedInteractiveElement>,
    #[serde(default)]
    component_hydration_targets: Vec<ExpectedComponentHydrationTarget>,
    #[serde(default)]
    structured_items: Vec<ExpectedStructuredItem>,
    #[serde(default)]
    templates: Vec<ExpectedTemplate>,
    forms: Vec<ExpectedForm>,
    tables: Vec<ExpectedTable>,
}

#[derive(Debug, Default, Deserialize)]
struct ExpectedDocumentMetadata {
    #[serde(default)]
    charset: Option<String>,
    #[serde(default)]
    viewport: Option<String>,
    #[serde(default)]
    viewport_directives: Vec<ExpectedMetadataDirective>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    application_name: Option<String>,
    #[serde(default)]
    referrer_policy: Option<String>,
    #[serde(default)]
    robots: Option<String>,
    #[serde(default)]
    robots_directives: Vec<String>,
    #[serde(default)]
    color_scheme: Option<String>,
    #[serde(default)]
    http_equiv_hints: Vec<ExpectedHttpEquivHint>,
    #[serde(default)]
    resource_hints: Vec<ExpectedResourceHint>,
    #[serde(default)]
    theme_colors: Vec<ExpectedThemeColor>,
    #[serde(default)]
    refresh: Option<ExpectedRefresh>,
    #[serde(default)]
    canonical_url: Option<String>,
    #[serde(default)]
    resolved_canonical_url: Option<String>,
    #[serde(default)]
    manifest_url: Option<String>,
    #[serde(default)]
    resolved_manifest_url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ExpectedMetadataDirective {
    name: String,
    #[serde(default)]
    value: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ExpectedHttpEquivHint {
    name: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct ExpectedResourceHint {
    kind: String,
    url: String,
    #[serde(default)]
    resolved_url: Option<String>,
    #[serde(default)]
    rel: Option<String>,
    #[serde(default)]
    rel_tokens: Vec<String>,
    #[serde(default)]
    as_hint: Option<String>,
    #[serde(default)]
    type_hint: Option<String>,
    #[serde(default)]
    media: Option<String>,
    #[serde(default)]
    integrity: Option<String>,
    #[serde(default)]
    crossorigin: Option<String>,
    #[serde(default)]
    nonce: Option<String>,
    #[serde(default)]
    referrerpolicy: Option<String>,
    #[serde(default)]
    fetchpriority: Option<String>,
    #[serde(default)]
    blocking: Option<String>,
    #[serde(default)]
    blocking_tokens: Vec<String>,
    #[serde(default)]
    imagesrcset: Option<String>,
    #[serde(default)]
    resolved_imagesrcset: Option<String>,
    #[serde(default)]
    imagesizes: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ExpectedThemeColor {
    color: String,
    #[serde(default)]
    media: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ExpectedRefresh {
    #[serde(default)]
    delay: Option<String>,
    #[serde(default)]
    url: Option<String>,
    #[serde(default)]
    resolved_url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ExpectedMeta {
    name: Option<String>,
    http_equiv: Option<String>,
    property: Option<String>,
    charset: Option<String>,
    content: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ExpectedStructuredItem {
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    item_type: Vec<String>,
    #[serde(default)]
    item_id: Option<String>,
    #[serde(default)]
    resolved_item_id: Option<String>,
    #[serde(default)]
    item_ref: Vec<String>,
    #[serde(default)]
    properties: Vec<ExpectedStructuredProperty>,
}

#[derive(Debug, Deserialize)]
struct ExpectedStructuredProperty {
    name: String,
    #[serde(default)]
    value: Option<String>,
    #[serde(default)]
    value_url: Option<String>,
    #[serde(default)]
    resolved_value_url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ExpectedTemplate {
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    shadowrootmode: Option<String>,
    #[serde(default)]
    shadowrootdelegatesfocus: bool,
    #[serde(default)]
    shadowrootclonable: bool,
    #[serde(default)]
    shadowrootserializable: bool,
    content_text: String,
}

#[derive(Debug, Deserialize)]
struct ExpectedComponentHydrationTarget {
    element: String,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    classes: Vec<String>,
    #[serde(default)]
    custom_element: bool,
    #[serde(default)]
    custom_element_name: Option<String>,
    #[serde(default)]
    custom_element_is: Option<String>,
    #[serde(default)]
    slot: Option<String>,
    #[serde(default)]
    slot_name: Option<String>,
    #[serde(default)]
    part: Vec<String>,
    #[serde(default)]
    exportparts: Option<String>,
    #[serde(default)]
    data_attributes: Vec<ExpectedDataAttribute>,
    #[serde(default)]
    canvas_fallback_text: Option<String>,
    #[serde(default)]
    text: String,
}

#[derive(Debug, Deserialize)]
struct ExpectedDataAttribute {
    name: String,
    #[serde(default)]
    value: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ExpectedResource {
    kind: String,
    url: String,
    resolved_url: Option<String>,
    rel: Option<String>,
    #[serde(default)]
    rel_tokens: Vec<String>,
    #[serde(default)]
    as_hint: Option<String>,
    type_hint: Option<String>,
    media: Option<String>,
    title: Option<String>,
    #[serde(default)]
    sizes: Option<String>,
    #[serde(default)]
    hreflang: Option<String>,
    #[serde(default)]
    color: Option<String>,
    #[serde(default)]
    width: Option<String>,
    #[serde(default)]
    height: Option<String>,
    #[serde(default)]
    integrity: Option<String>,
    #[serde(default)]
    crossorigin: Option<String>,
    #[serde(default)]
    nonce: Option<String>,
    #[serde(default)]
    referrerpolicy: Option<String>,
    #[serde(default)]
    fetchpriority: Option<String>,
    #[serde(default)]
    csp: Option<String>,
    #[serde(default)]
    blocking: Option<String>,
    #[serde(default)]
    blocking_tokens: Vec<String>,
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
    srcdoc: Option<String>,
    #[serde(default)]
    credentialless: bool,
    #[serde(default)]
    imagesrcset: Option<String>,
    #[serde(default)]
    resolved_imagesrcset: Option<String>,
    #[serde(default)]
    imagesizes: Option<String>,
    #[serde(default)]
    track_kind: Option<String>,
    #[serde(default)]
    srclang: Option<String>,
    #[serde(default)]
    track_label: Option<String>,
    #[serde(default)]
    default_track: bool,
    async_script: bool,
    defer_script: bool,
}

#[derive(Debug, Deserialize)]
struct ExpectedScript {
    script_kind: String,
    src: Option<String>,
    #[serde(default)]
    resolved_src: Option<String>,
    type_hint: Option<String>,
    #[serde(default)]
    async_script: bool,
    #[serde(default)]
    defer_script: bool,
    #[serde(default)]
    nomodule: bool,
    #[serde(default)]
    integrity: Option<String>,
    #[serde(default)]
    crossorigin: Option<String>,
    #[serde(default)]
    nonce: Option<String>,
    #[serde(default)]
    referrerpolicy: Option<String>,
    #[serde(default)]
    fetchpriority: Option<String>,
    #[serde(default)]
    blocking: Option<String>,
    #[serde(default)]
    blocking_tokens: Vec<String>,
    #[serde(default)]
    text: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ExpectedStylesheet {
    source: String,
    #[serde(default)]
    href: Option<String>,
    #[serde(default)]
    resolved_href: Option<String>,
    #[serde(default)]
    rel: Option<String>,
    #[serde(default)]
    rel_tokens: Vec<String>,
    #[serde(default)]
    type_hint: Option<String>,
    #[serde(default)]
    media: Option<String>,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    disabled: bool,
    #[serde(default)]
    alternate: bool,
    #[serde(default)]
    integrity: Option<String>,
    #[serde(default)]
    crossorigin: Option<String>,
    #[serde(default)]
    nonce: Option<String>,
    #[serde(default)]
    referrerpolicy: Option<String>,
    #[serde(default)]
    fetchpriority: Option<String>,
    #[serde(default)]
    blocking: Option<String>,
    #[serde(default)]
    blocking_tokens: Vec<String>,
    #[serde(default)]
    text: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ExpectedAnchor {
    id: Option<String>,
    name: Option<String>,
    text: String,
}

#[derive(Debug, Deserialize)]
struct ExpectedHeading {
    level: u8,
    text: String,
}

#[derive(Debug, Deserialize)]
struct ExpectedLink {
    #[serde(default = "default_browser_link_element")]
    element: String,
    #[serde(default)]
    id: Option<String>,
    href: Option<String>,
    resolved_href: Option<String>,
    name: Option<String>,
    target: Option<String>,
    #[serde(default)]
    effective_target: Option<String>,
    rel: Option<String>,
    #[serde(default)]
    rel_tokens: Vec<String>,
    #[serde(default)]
    rel_external: bool,
    #[serde(default)]
    rel_nofollow: bool,
    #[serde(default)]
    rel_noopener: bool,
    #[serde(default)]
    rel_noreferrer: bool,
    title: Option<String>,
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
    #[serde(default)]
    type_hint: Option<String>,
    #[serde(default)]
    referrerpolicy: Option<String>,
    text: String,
}

#[derive(Debug, Deserialize)]
struct ExpectedImage {
    src: Option<String>,
    resolved_src: Option<String>,
    alt: Option<String>,
    width: Option<String>,
    height: Option<String>,
    #[serde(default)]
    srcset: Option<String>,
    #[serde(default)]
    resolved_srcset: Option<String>,
    #[serde(default)]
    sizes: Option<String>,
    #[serde(default)]
    loading: Option<String>,
    #[serde(default)]
    decoding: Option<String>,
    #[serde(default)]
    fetchpriority: Option<String>,
    #[serde(default)]
    crossorigin: Option<String>,
    #[serde(default)]
    referrerpolicy: Option<String>,
    #[serde(default)]
    usemap: Option<String>,
    #[serde(default)]
    ismap: bool,
    #[serde(default)]
    sources: Vec<ExpectedImageSource>,
}

#[derive(Debug, Deserialize)]
struct ExpectedImageSource {
    #[serde(default)]
    srcset: Option<String>,
    #[serde(default)]
    resolved_srcset: Option<String>,
    #[serde(default)]
    sizes: Option<String>,
    #[serde(default)]
    media: Option<String>,
    #[serde(default)]
    type_hint: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ExpectedMedia {
    kind: String,
    src: Option<String>,
    #[serde(default)]
    resolved_src: Option<String>,
    #[serde(default)]
    poster: Option<String>,
    #[serde(default)]
    resolved_poster: Option<String>,
    #[serde(default)]
    width: Option<String>,
    #[serde(default)]
    height: Option<String>,
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
    preload: Option<String>,
    #[serde(default)]
    crossorigin: Option<String>,
    #[serde(default)]
    controlslist: Option<String>,
    #[serde(default)]
    controlslist_tokens: Vec<String>,
    #[serde(default)]
    disableremoteplayback: bool,
    #[serde(default)]
    disablepictureinpicture: bool,
}

#[derive(Debug, Deserialize)]
struct ExpectedEmbeddedContext {
    element: String,
    #[serde(default)]
    url: Option<String>,
    #[serde(default)]
    resolved_url: Option<String>,
    #[serde(default)]
    browsing_context_name: Option<String>,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    type_hint: Option<String>,
    #[serde(default)]
    width: Option<String>,
    #[serde(default)]
    height: Option<String>,
    #[serde(default)]
    loading: Option<String>,
    #[serde(default)]
    fetchpriority: Option<String>,
    #[serde(default)]
    csp: Option<String>,
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
    #[serde(default)]
    fallback_text: String,
}

#[derive(Debug, Deserialize)]
struct ExpectedInteractiveElement {
    element: String,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    role: Option<String>,
    #[serde(default)]
    authored_role: Option<String>,
    #[serde(default)]
    text: String,
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
    disabled: bool,
}

#[derive(Debug, Deserialize)]
struct ExpectedForm {
    #[serde(default)]
    id: Option<String>,
    action: Option<String>,
    resolved_action: Option<String>,
    #[serde(default)]
    name: Option<String>,
    method: String,
    enctype: Option<String>,
    target: Option<String>,
    #[serde(default)]
    effective_target: Option<String>,
    #[serde(default)]
    accept_charset: Option<String>,
    #[serde(default)]
    accept_charset_tokens: Vec<String>,
    #[serde(default)]
    autocomplete: Option<String>,
    #[serde(default)]
    autocomplete_tokens: Vec<String>,
    #[serde(default)]
    rel: Option<String>,
    #[serde(default)]
    rel_tokens: Vec<String>,
    #[serde(default)]
    rel_external: bool,
    #[serde(default)]
    rel_nofollow: bool,
    #[serde(default)]
    rel_noopener: bool,
    #[serde(default)]
    rel_noreferrer: bool,
    #[serde(default)]
    novalidate: bool,
    #[serde(default)]
    fieldsets: Vec<ExpectedFormFieldset>,
    #[serde(default)]
    labels: Vec<ExpectedFormLabel>,
    #[serde(default)]
    outputs: Vec<ExpectedFormOutput>,
    controls: Vec<ExpectedFormControl>,
    #[serde(default)]
    submitters: Vec<ExpectedFormSubmitter>,
}

#[derive(Debug, Deserialize)]
struct ExpectedFormFieldset {
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    form_owner: Option<String>,
    #[serde(default)]
    legend: Option<String>,
    #[serde(default)]
    disabled: bool,
    #[serde(default)]
    control_ids: Vec<String>,
    #[serde(default)]
    control_names: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ExpectedFormLabel {
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    for_control: Option<String>,
    text: String,
    #[serde(default)]
    control_id: Option<String>,
    #[serde(default)]
    control_name: Option<String>,
    #[serde(default)]
    control_type: Option<String>,
    association: String,
}

#[derive(Debug, Deserialize)]
struct ExpectedFormOutput {
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    form_owner: Option<String>,
    #[serde(default)]
    for_tokens: Vec<String>,
    #[serde(default)]
    value: Option<String>,
    text: String,
}

#[derive(Debug, Deserialize)]
struct ExpectedFormControl {
    #[serde(default)]
    id: Option<String>,
    control_type: String,
    name: Option<String>,
    #[serde(default)]
    form_owner: Option<String>,
    #[serde(default)]
    labels: Vec<String>,
    #[serde(default)]
    accessible_name: Option<String>,
    #[serde(default)]
    accessible_description: Option<String>,
    #[serde(default)]
    placeholder: Option<String>,
    #[serde(default)]
    autocomplete: Option<String>,
    #[serde(default)]
    autocomplete_tokens: Vec<String>,
    #[serde(default)]
    autocapitalize: Option<String>,
    #[serde(default)]
    enterkeyhint: Option<String>,
    #[serde(default)]
    dirname: Option<String>,
    #[serde(default)]
    spellcheck: Option<String>,
    #[serde(default)]
    autocorrect: Option<String>,
    #[serde(default)]
    accept: Option<String>,
    #[serde(default)]
    accept_tokens: Vec<String>,
    #[serde(default)]
    capture: Option<String>,
    #[serde(default)]
    src: Option<String>,
    #[serde(default)]
    resolved_src: Option<String>,
    #[serde(default)]
    alt: Option<String>,
    #[serde(default)]
    width: Option<String>,
    #[serde(default)]
    height: Option<String>,
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
    rows: Option<String>,
    #[serde(default)]
    cols: Option<String>,
    #[serde(default)]
    wrap: Option<String>,
    #[serde(default)]
    list: Option<String>,
    #[serde(default)]
    datalist_options: Vec<String>,
    #[serde(default)]
    output_for: Vec<String>,
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
    value: Option<String>,
    #[serde(default)]
    successful: bool,
    #[serde(default)]
    submission_values: Vec<String>,
    #[serde(default)]
    autofocus: bool,
    disabled: bool,
    #[serde(default)]
    required: bool,
    #[serde(default)]
    readonly: bool,
    #[serde(default)]
    will_validate: bool,
    #[serde(default)]
    validation_attributes: Vec<String>,
    #[serde(default)]
    validation_barred_reason: Option<String>,
    checked: bool,
    #[serde(default)]
    multiple: bool,
    #[serde(default)]
    selected_options: Vec<String>,
    #[serde(default)]
    option_items: Vec<ExpectedSelectOption>,
    text: String,
    options: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ExpectedSelectOption {
    value: String,
    #[serde(default)]
    label: Option<String>,
    text: String,
    #[serde(default)]
    selected: bool,
    #[serde(default)]
    disabled: bool,
    #[serde(default)]
    group_label: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ExpectedFormSubmitter {
    #[serde(default)]
    id: Option<String>,
    control_type: String,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    accessible_name: Option<String>,
    #[serde(default)]
    action: Option<String>,
    #[serde(default)]
    resolved_action: Option<String>,
    method: String,
    #[serde(default)]
    enctype: Option<String>,
    #[serde(default)]
    target: Option<String>,
    #[serde(default)]
    effective_target: Option<String>,
    #[serde(default)]
    novalidate: bool,
    #[serde(default)]
    value: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ExpectedTable {
    caption: Option<String>,
    row_count: usize,
    #[serde(default)]
    column_count: usize,
    #[serde(default)]
    column_hint_count: usize,
    cell_count: usize,
    header_cell_count: usize,
}

#[test]
fn browser_readiness_cases_extract_browser_document_facts() {
    let suite: BrowserReadinessSuite = serde_json::from_str(BROWSER_READINESS_FIXTURE)
        .expect("browser readiness fixture should parse");

    assert_eq!(suite.format, "venture-html-browser-readiness/v1");
    assert_eq!(suite.suite, "browser-readiness");
    assert!(!suite.cases.is_empty(), "fixture should contain cases");

    for case in suite.cases {
        let actual = parse_browser_document(&case.input)
            .unwrap_or_else(|error| panic!("{} should parse: {error}", case.id));

        assert_eq!(
            actual,
            case.expected.into_browser_document(),
            "{} extracted browser facts should match",
            case.id
        );
    }
}

#[test]
fn browser_script_style_security_metadata_tracks_nonces_and_fetch_policy() {
    let suite: BrowserReadinessSuite = serde_json::from_str(BROWSER_READINESS_FIXTURE)
        .expect("browser readiness fixture should parse");
    let case = suite
        .cases
        .into_iter()
        .find(|case| case.id == "script-style-loading-page")
        .expect("script/style loading fixture case should exist");

    let actual = parse_browser_document(&case.input)
        .expect("script/style loading fixture should parse into browser document facts");
    let expected = case.expected.into_browser_document();

    assert_eq!(
        actual.metadata.resource_hints, expected.metadata.resource_hints,
        "resource hints should preserve CSP nonces and fetch policy metadata",
    );
    assert_eq!(
        actual.resources, expected.resources,
        "resources should preserve script and stylesheet security metadata",
    );
    assert_eq!(
        actual.scripts, expected.scripts,
        "scripts should preserve CSP nonces and loading policy metadata",
    );
    assert_eq!(
        actual.stylesheets, expected.stylesheets,
        "stylesheets should preserve CSP nonces and loading policy metadata",
    );
}

#[test]
fn browser_resource_priority_metadata_tracks_rel_and_blocking_tokens() {
    let suite: BrowserReadinessSuite = serde_json::from_str(BROWSER_READINESS_FIXTURE)
        .expect("browser readiness fixture should parse");
    let case = suite
        .cases
        .into_iter()
        .find(|case| case.id == "script-style-loading-page")
        .expect("script style loading fixture case should exist");

    let actual = parse_browser_document(&case.input)
        .expect("script style loading fixture should parse into browser document facts");
    let expected = case.expected.into_browser_document();

    assert_eq!(
        actual.metadata.resource_hints, expected.metadata.resource_hints,
        "resource hints should preserve rel and blocking token metadata",
    );
    assert_eq!(
        actual.resources, expected.resources,
        "resources should preserve rel and blocking token metadata",
    );
    assert_eq!(
        actual.scripts, expected.scripts,
        "scripts should preserve blocking token metadata",
    );
    assert_eq!(
        actual.stylesheets, expected.stylesheets,
        "stylesheets should preserve rel and blocking token metadata",
    );
}

#[test]
fn browser_link_descriptor_metadata_tracks_icon_and_alternate_fields() {
    let suite: BrowserReadinessSuite = serde_json::from_str(BROWSER_READINESS_FIXTURE)
        .expect("browser readiness fixture should parse");
    let case = suite
        .cases
        .into_iter()
        .find(|case| case.id == "link-resource-metadata-page")
        .expect("link resource fixture case should exist");

    let actual = parse_browser_document(&case.input)
        .expect("link resource fixture should parse into browser document facts");
    let expected = case.expected.into_browser_document();

    assert_eq!(
        actual.resources, expected.resources,
        "link resources should preserve icon sizes, mask colors, and alternate language descriptors",
    );
}

#[test]
fn browser_form_validation_metadata_tracks_constraint_candidates() {
    let suite: BrowserReadinessSuite = serde_json::from_str(BROWSER_READINESS_FIXTURE)
        .expect("browser readiness fixture should parse");
    let case = suite
        .cases
        .into_iter()
        .find(|case| case.id == "form-accessibility-document-page")
        .expect("form accessibility fixture case should exist");

    let actual = parse_browser_document(&case.input)
        .expect("form accessibility fixture should parse into browser document facts");

    assert_eq!(
        actual.forms,
        case.expected.into_browser_document().forms,
        "form controls should preserve validation candidate metadata",
    );
}

#[test]
fn browser_form_validation_descriptor_metadata_tracks_constraint_attributes_and_barred_reasons() {
    let suite: BrowserReadinessSuite = serde_json::from_str(BROWSER_READINESS_FIXTURE)
        .expect("browser readiness fixture should parse");
    let case = suite
        .cases
        .into_iter()
        .find(|case| case.id == "form-accessibility-document-page")
        .expect("form accessibility fixture case should exist");

    let actual = parse_browser_document(&case.input)
        .expect("form accessibility fixture should parse into browser document facts");
    let expected = case.expected.into_browser_document();

    assert_eq!(
        actual.forms, expected.forms,
        "form controls should preserve validation attributes and barred validation reasons",
    );
}

#[test]
fn browser_form_fieldset_metadata_disables_descendant_controls() {
    let suite: BrowserReadinessSuite = serde_json::from_str(BROWSER_READINESS_FIXTURE)
        .expect("browser readiness fixture should parse");
    let case = suite
        .cases
        .into_iter()
        .find(|case| case.id == "form-accessibility-document-page")
        .expect("form accessibility fixture case should exist");

    let actual = parse_browser_document(&case.input)
        .expect("form accessibility fixture should parse into browser document facts");
    let expected = case.expected.into_browser_document();

    assert_eq!(
        actual.forms, expected.forms,
        "form controls should reflect disabled fieldset ancestry",
    );
}

#[test]
fn browser_form_fieldset_group_metadata_tracks_legends_disabled_state_and_controls() {
    let suite: BrowserReadinessSuite = serde_json::from_str(BROWSER_READINESS_FIXTURE)
        .expect("browser readiness fixture should parse");
    let case = suite
        .cases
        .into_iter()
        .find(|case| case.id == "form-accessibility-document-page")
        .expect("form accessibility fixture case should exist");

    let actual = parse_browser_document(&case.input)
        .expect("form accessibility fixture should parse into browser document facts");
    let expected = case.expected.into_browser_document();

    assert_eq!(
        actual.forms, expected.forms,
        "form summaries should preserve fieldset legends, disabled state, and grouped controls",
    );
}

#[test]
fn browser_form_successful_control_metadata_tracks_submission_values() {
    let suite: BrowserReadinessSuite = serde_json::from_str(BROWSER_READINESS_FIXTURE)
        .expect("browser readiness fixture should parse");
    let case = suite
        .cases
        .into_iter()
        .find(|case| case.id == "catalog-form-table-page")
        .expect("catalog form fixture case should exist");

    let actual = parse_browser_document(&case.input)
        .expect("catalog form fixture should parse into browser document facts");
    let expected = case.expected.into_browser_document();

    assert_eq!(
        actual.forms, expected.forms,
        "form controls should preserve successful-control and submission value metadata",
    );
}

#[test]
fn browser_form_descriptor_metadata_tracks_accept_and_autocomplete_tokens() {
    let suite: BrowserReadinessSuite = serde_json::from_str(BROWSER_READINESS_FIXTURE)
        .expect("browser readiness fixture should parse");
    let case = suite
        .cases
        .into_iter()
        .find(|case| case.id == "form-accessibility-document-page")
        .expect("form accessibility fixture case should exist");

    let actual = parse_browser_document(&case.input)
        .expect("form accessibility fixture should parse into browser document facts");
    let expected = case.expected.into_browser_document();

    assert_eq!(
        actual.forms, expected.forms,
        "form metadata should preserve tokenized accept-charset, autocomplete, and file accept descriptors",
    );
}

#[test]
fn browser_text_control_descriptor_metadata_tracks_editing_and_layout_hints() {
    let suite: BrowserReadinessSuite = serde_json::from_str(BROWSER_READINESS_FIXTURE)
        .expect("browser readiness fixture should parse");
    let case = suite
        .cases
        .into_iter()
        .find(|case| case.id == "form-accessibility-document-page")
        .expect("form accessibility fixture case should exist");

    let actual = parse_browser_document(&case.input)
        .expect("form accessibility fixture should parse into browser document facts");
    let expected = case.expected.into_browser_document();

    assert_eq!(
        actual.forms, expected.forms,
        "text controls should preserve spellcheck, autocorrect, rows, cols, and wrapping hints",
    );
}

#[test]
fn browser_select_option_descriptor_metadata_tracks_values_labels_groups_and_state() {
    let suite: BrowserReadinessSuite = serde_json::from_str(BROWSER_READINESS_FIXTURE)
        .expect("browser readiness fixture should parse");
    let case = suite
        .cases
        .into_iter()
        .find(|case| case.id == "form-accessibility-document-page")
        .expect("form accessibility fixture case should exist");

    let actual = parse_browser_document(&case.input)
        .expect("form accessibility fixture should parse into browser document facts");
    let expected = case.expected.into_browser_document();

    assert_eq!(
        actual.forms, expected.forms,
        "select controls should preserve option values, labels, optgroup labels, selected state, and disabled state",
    );
}

#[test]
fn browser_embedded_context_metadata_tracks_frame_object_embed_and_srcdoc() {
    let suite: BrowserReadinessSuite = serde_json::from_str(BROWSER_READINESS_FIXTURE)
        .expect("browser readiness fixture should parse");
    let case = suite
        .cases
        .into_iter()
        .find(|case| case.id == "embedded-resource-page")
        .expect("embedded resource fixture case should exist");

    let actual = parse_browser_document(&case.input)
        .expect("embedded resource fixture should parse into browser document facts");

    assert_eq!(
        actual.embedded_contexts,
        case.expected.into_browser_document().embedded_contexts,
        "embedded contexts should preserve frame, object, embed, and srcdoc-only iframe metadata",
    );
}

#[test]
fn browser_embedded_policy_metadata_tracks_fetchpriority_and_csp() {
    let suite: BrowserReadinessSuite = serde_json::from_str(BROWSER_READINESS_FIXTURE)
        .expect("browser readiness fixture should parse");
    let case = suite
        .cases
        .into_iter()
        .find(|case| case.id == "embedded-resource-page")
        .expect("embedded resource fixture case should exist");

    let actual = parse_browser_document(&case.input)
        .expect("embedded resource fixture should parse into browser document facts");
    let expected = case.expected.into_browser_document();

    assert_eq!(
        actual.resources, expected.resources,
        "frame resources should preserve fetch priority and CSP policy metadata",
    );
    assert_eq!(
        actual.embedded_contexts, expected.embedded_contexts,
        "embedded contexts should preserve fetch priority and CSP policy metadata",
    );
}

#[test]
fn browser_media_policy_metadata_tracks_controls_and_remote_playback_hints() {
    let suite: BrowserReadinessSuite = serde_json::from_str(BROWSER_READINESS_FIXTURE)
        .expect("browser readiness fixture should parse");
    let case = suite
        .cases
        .into_iter()
        .find(|case| case.id == "media-playback-poster-page")
        .expect("media playback fixture case should exist");

    let actual = parse_browser_document(&case.input)
        .expect("media playback fixture should parse into browser document facts");
    let expected = case.expected.into_browser_document();

    assert_eq!(
        actual.resources, expected.resources,
        "media resources should preserve cross-origin fetch metadata",
    );
    assert_eq!(
        actual.media, expected.media,
        "media summaries should preserve controlslist and remote playback policy metadata",
    );
}

#[test]
fn browser_interactive_element_metadata_tracks_focus_editing_and_commands() {
    let suite: BrowserReadinessSuite = serde_json::from_str(BROWSER_READINESS_FIXTURE)
        .expect("browser readiness fixture should parse");
    let case = suite
        .cases
        .into_iter()
        .find(|case| case.id == "interactive-element-state-page")
        .expect("interactive element fixture case should exist");

    let actual = parse_browser_document(&case.input)
        .expect("interactive element fixture should parse into browser document facts");

    assert_eq!(
        actual.interactive_elements,
        case.expected.into_browser_document().interactive_elements,
        "interactive elements should preserve focus, editing, popover, command, hidden, and event metadata",
    );
}

#[test]
fn browser_accessible_description_metadata_tracks_describedby_text() {
    let suite: BrowserReadinessSuite = serde_json::from_str(BROWSER_READINESS_FIXTURE)
        .expect("browser readiness fixture should parse");

    let mut cases = suite.cases.into_iter();
    let form = cases
        .find(|case| case.id == "form-accessibility-document-page")
        .expect("form describedby fixture case should exist");
    let interactive = cases
        .find(|case| case.id == "interactive-element-state-page")
        .expect("interactive describedby fixture case should exist");

    let actual_interactive = parse_browser_document(&interactive.input)
        .expect("interactive describedby fixture should parse into browser document facts");
    assert_eq!(
        actual_interactive.interactive_elements,
        interactive
            .expected
            .into_browser_document()
            .interactive_elements,
        "interactive summaries should resolve aria-describedby text into accessible descriptions",
    );

    let actual_form = parse_browser_document(&form.input)
        .expect("form describedby fixture should parse into browser document facts");
    assert_eq!(
        actual_form.forms,
        form.expected.into_browser_document().forms,
        "form controls should resolve aria-describedby text into accessible descriptions",
    );
}

#[test]
fn browser_aria_state_relation_metadata_tracks_composite_and_live_states() {
    let suite: BrowserReadinessSuite = serde_json::from_str(BROWSER_READINESS_FIXTURE)
        .expect("browser readiness fixture should parse");
    let case = suite
        .cases
        .into_iter()
        .find(|case| case.id == "interactive-element-state-page")
        .expect("interactive ARIA state relation fixture case should exist");

    let actual = parse_browser_document(&case.input)
        .expect("interactive ARIA state relation fixture should parse into browser document facts");

    assert_eq!(
        actual.interactive_elements,
        case.expected.into_browser_document().interactive_elements,
        "interactive summaries should preserve ARIA relationship, popup, modal, validation, disabled, required, and live-region states",
    );
}

#[test]
fn browser_navigation_attribution_metadata_tracks_links_areas_and_form_targets() {
    let suite: BrowserReadinessSuite = serde_json::from_str(BROWSER_READINESS_FIXTURE)
        .expect("browser readiness fixture should parse");

    let legacy = suite
        .cases
        .into_iter()
        .find(|case| case.id == "legacy-directory-page")
        .expect("legacy directory fixture case should exist");
    let legacy_actual = parse_browser_document(&legacy.input)
        .expect("legacy directory fixture should parse into browser document facts");
    assert_eq!(
        legacy_actual.links,
        legacy.expected.into_browser_document().links,
        "link metadata should preserve attribution source registration URLs",
    );

    let suite: BrowserReadinessSuite = serde_json::from_str(BROWSER_READINESS_FIXTURE)
        .expect("browser readiness fixture should parse");
    let forms = suite
        .cases
        .into_iter()
        .find(|case| case.id == "form-accessibility-document-page")
        .expect("form accessibility fixture case should exist");
    let forms_actual = parse_browser_document(&forms.input)
        .expect("form accessibility fixture should parse into browser document facts");
    assert_eq!(
        forms_actual.forms,
        forms.expected.into_browser_document().forms,
        "form metadata should preserve rel tokens and base-target effective submitter targets",
    );
}

#[test]
fn browser_component_hydration_metadata_tracks_custom_elements_slots_parts_and_data() {
    let suite: BrowserReadinessSuite = serde_json::from_str(BROWSER_READINESS_FIXTURE)
        .expect("browser readiness fixture should parse");
    let case = suite
        .cases
        .into_iter()
        .find(|case| case.id == "component-template-page")
        .expect("component template fixture case should exist");

    let actual = parse_browser_document(&case.input)
        .expect("component template fixture should parse into browser document facts");

    assert_eq!(
        actual.component_hydration_targets,
        case.expected
            .into_browser_document()
            .component_hydration_targets,
        "component hydration metadata should preserve custom elements, slots, parts, data attributes, and canvas fallback text",
    );
}

impl ExpectedBrowserDocument {
    fn into_browser_document(self) -> BrowserDocument {
        BrowserDocument {
            title: self.title,
            base_href: self.base_href,
            base_target: self.base_target,
            metadata: self.metadata.into_browser_document_metadata(),
            document_lang: self.document_lang,
            document_dir: self.document_dir,
            body_id: self.body_id,
            body_classes: self.body_classes,
            body_lang: self.body_lang,
            body_dir: self.body_dir,
            document_event_handlers: self.document_event_handlers,
            body_event_handlers: self.body_event_handlers,
            body_text: self.body_text,
            metas: self
                .metas
                .into_iter()
                .map(ExpectedMeta::into_browser_meta)
                .collect(),
            resources: self
                .resources
                .into_iter()
                .map(ExpectedResource::into_browser_resource)
                .collect(),
            scripts: self
                .scripts
                .into_iter()
                .map(ExpectedScript::into_browser_script)
                .collect(),
            stylesheets: self
                .stylesheets
                .into_iter()
                .map(ExpectedStylesheet::into_browser_stylesheet)
                .collect(),
            anchors: self
                .anchors
                .into_iter()
                .map(ExpectedAnchor::into_browser_anchor)
                .collect(),
            headings: self
                .headings
                .into_iter()
                .map(ExpectedHeading::into_browser_heading)
                .collect(),
            links: self
                .links
                .into_iter()
                .map(ExpectedLink::into_browser_link)
                .collect(),
            images: self
                .images
                .into_iter()
                .map(ExpectedImage::into_browser_image)
                .collect(),
            media: self
                .media
                .into_iter()
                .map(ExpectedMedia::into_browser_media)
                .collect(),
            embedded_contexts: self
                .embedded_contexts
                .into_iter()
                .map(ExpectedEmbeddedContext::into_browser_embedded_context)
                .collect(),
            interactive_elements: self
                .interactive_elements
                .into_iter()
                .map(ExpectedInteractiveElement::into_browser_interactive_element)
                .collect(),
            component_hydration_targets: self
                .component_hydration_targets
                .into_iter()
                .map(ExpectedComponentHydrationTarget::into_browser_component_hydration_target)
                .collect(),
            structured_items: self
                .structured_items
                .into_iter()
                .map(ExpectedStructuredItem::into_browser_structured_item)
                .collect(),
            templates: self
                .templates
                .into_iter()
                .map(ExpectedTemplate::into_browser_template)
                .collect(),
            forms: self
                .forms
                .into_iter()
                .map(ExpectedForm::into_browser_form)
                .collect(),
            tables: self
                .tables
                .into_iter()
                .map(ExpectedTable::into_browser_table)
                .collect(),
        }
    }
}

impl ExpectedDocumentMetadata {
    fn into_browser_document_metadata(self) -> BrowserDocumentMetadata {
        BrowserDocumentMetadata {
            charset: self.charset,
            viewport: self.viewport,
            viewport_directives: self
                .viewport_directives
                .into_iter()
                .map(ExpectedMetadataDirective::into_browser_metadata_directive)
                .collect(),
            description: self.description,
            application_name: self.application_name,
            referrer_policy: self.referrer_policy,
            robots: self.robots,
            robots_directives: self.robots_directives,
            color_scheme: self.color_scheme,
            http_equiv_hints: self
                .http_equiv_hints
                .into_iter()
                .map(ExpectedHttpEquivHint::into_browser_http_equiv_hint)
                .collect(),
            resource_hints: self
                .resource_hints
                .into_iter()
                .map(ExpectedResourceHint::into_browser_resource_hint)
                .collect(),
            theme_colors: self
                .theme_colors
                .into_iter()
                .map(ExpectedThemeColor::into_browser_theme_color)
                .collect(),
            refresh: self.refresh.map(ExpectedRefresh::into_browser_refresh),
            canonical_url: self.canonical_url,
            resolved_canonical_url: self.resolved_canonical_url,
            manifest_url: self.manifest_url,
            resolved_manifest_url: self.resolved_manifest_url,
        }
    }
}

impl ExpectedMetadataDirective {
    fn into_browser_metadata_directive(self) -> BrowserMetadataDirective {
        BrowserMetadataDirective {
            name: self.name,
            value: self.value,
        }
    }
}

impl ExpectedHttpEquivHint {
    fn into_browser_http_equiv_hint(self) -> BrowserHttpEquivHint {
        BrowserHttpEquivHint {
            name: self.name,
            content: self.content,
        }
    }
}

impl ExpectedResourceHint {
    fn into_browser_resource_hint(self) -> BrowserResourceHint {
        let rel_tokens = expected_tokens_from_raw(self.rel_tokens, self.rel.as_deref());
        let blocking_tokens =
            expected_tokens_from_raw(self.blocking_tokens, self.blocking.as_deref());
        BrowserResourceHint {
            kind: self.kind,
            url: self.url,
            resolved_url: self.resolved_url,
            rel: self.rel,
            rel_tokens,
            as_hint: self.as_hint,
            type_hint: self.type_hint,
            media: self.media,
            integrity: self.integrity,
            crossorigin: self.crossorigin,
            nonce: self.nonce,
            referrerpolicy: self.referrerpolicy,
            fetchpriority: self.fetchpriority,
            blocking: self.blocking,
            blocking_tokens,
            imagesrcset: self.imagesrcset,
            resolved_imagesrcset: self.resolved_imagesrcset,
            imagesizes: self.imagesizes,
        }
    }
}

impl ExpectedThemeColor {
    fn into_browser_theme_color(self) -> BrowserThemeColor {
        BrowserThemeColor {
            color: self.color,
            media: self.media,
        }
    }
}

impl ExpectedRefresh {
    fn into_browser_refresh(self) -> BrowserRefresh {
        BrowserRefresh {
            delay: self.delay,
            url: self.url,
            resolved_url: self.resolved_url,
        }
    }
}

impl ExpectedMeta {
    fn into_browser_meta(self) -> BrowserMeta {
        BrowserMeta {
            name: self.name,
            http_equiv: self.http_equiv,
            property: self.property,
            charset: self.charset,
            content: self.content,
        }
    }
}

impl ExpectedStructuredItem {
    fn into_browser_structured_item(self) -> BrowserStructuredItem {
        BrowserStructuredItem {
            id: self.id,
            item_type: self.item_type,
            item_id: self.item_id,
            resolved_item_id: self.resolved_item_id,
            item_ref: self.item_ref,
            properties: self
                .properties
                .into_iter()
                .map(ExpectedStructuredProperty::into_browser_structured_property)
                .collect(),
        }
    }
}

impl ExpectedStructuredProperty {
    fn into_browser_structured_property(self) -> BrowserStructuredProperty {
        BrowserStructuredProperty {
            name: self.name,
            value: self.value,
            value_url: self.value_url,
            resolved_value_url: self.resolved_value_url,
        }
    }
}

impl ExpectedTemplate {
    fn into_browser_template(self) -> BrowserTemplate {
        BrowserTemplate {
            id: self.id,
            shadowrootmode: self.shadowrootmode,
            shadowrootdelegatesfocus: self.shadowrootdelegatesfocus,
            shadowrootclonable: self.shadowrootclonable,
            shadowrootserializable: self.shadowrootserializable,
            content_text: self.content_text,
        }
    }
}

impl ExpectedComponentHydrationTarget {
    fn into_browser_component_hydration_target(self) -> BrowserComponentHydrationTarget {
        BrowserComponentHydrationTarget {
            element: self.element,
            id: self.id,
            classes: self.classes,
            custom_element: self.custom_element,
            custom_element_name: self.custom_element_name,
            custom_element_is: self.custom_element_is,
            slot: self.slot,
            slot_name: self.slot_name,
            part: self.part,
            exportparts: self.exportparts,
            data_attributes: self
                .data_attributes
                .into_iter()
                .map(ExpectedDataAttribute::into_browser_data_attribute)
                .collect(),
            canvas_fallback_text: self.canvas_fallback_text,
            text: self.text,
        }
    }
}

impl ExpectedDataAttribute {
    fn into_browser_data_attribute(self) -> BrowserDataAttribute {
        BrowserDataAttribute {
            name: self.name,
            value: self.value,
        }
    }
}

impl ExpectedResource {
    fn into_browser_resource(self) -> BrowserResource {
        let rel_tokens = expected_tokens_from_raw(self.rel_tokens, self.rel.as_deref());
        let blocking_tokens =
            expected_tokens_from_raw(self.blocking_tokens, self.blocking.as_deref());
        BrowserResource {
            kind: self.kind,
            url: self.url,
            resolved_url: self.resolved_url,
            rel: self.rel,
            rel_tokens,
            as_hint: self.as_hint,
            type_hint: self.type_hint,
            media: self.media,
            title: self.title,
            sizes: self.sizes,
            hreflang: self.hreflang,
            color: self.color,
            width: self.width,
            height: self.height,
            integrity: self.integrity,
            crossorigin: self.crossorigin,
            nonce: self.nonce,
            referrerpolicy: self.referrerpolicy,
            fetchpriority: self.fetchpriority,
            csp: self.csp,
            blocking: self.blocking,
            blocking_tokens,
            browsing_context_name: self.browsing_context_name,
            loading: self.loading,
            sandbox: self.sandbox,
            allow: self.allow,
            allowfullscreen: self.allowfullscreen,
            srcdoc: self.srcdoc,
            credentialless: self.credentialless,
            imagesrcset: self.imagesrcset,
            resolved_imagesrcset: self.resolved_imagesrcset,
            imagesizes: self.imagesizes,
            track_kind: self.track_kind,
            srclang: self.srclang,
            track_label: self.track_label,
            default_track: self.default_track,
            async_script: self.async_script,
            defer_script: self.defer_script,
        }
    }
}

impl ExpectedScript {
    fn into_browser_script(self) -> BrowserScript {
        let blocking_tokens =
            expected_tokens_from_raw(self.blocking_tokens, self.blocking.as_deref());
        BrowserScript {
            script_kind: self.script_kind,
            src: self.src,
            resolved_src: self.resolved_src,
            type_hint: self.type_hint,
            async_script: self.async_script,
            defer_script: self.defer_script,
            nomodule: self.nomodule,
            integrity: self.integrity,
            crossorigin: self.crossorigin,
            nonce: self.nonce,
            referrerpolicy: self.referrerpolicy,
            fetchpriority: self.fetchpriority,
            blocking: self.blocking,
            blocking_tokens,
            text: self.text,
        }
    }
}

impl ExpectedStylesheet {
    fn into_browser_stylesheet(self) -> BrowserStylesheet {
        let rel_tokens = expected_tokens_from_raw(self.rel_tokens, self.rel.as_deref());
        let blocking_tokens =
            expected_tokens_from_raw(self.blocking_tokens, self.blocking.as_deref());
        BrowserStylesheet {
            source: self.source,
            href: self.href,
            resolved_href: self.resolved_href,
            rel: self.rel,
            rel_tokens,
            type_hint: self.type_hint,
            media: self.media,
            title: self.title,
            disabled: self.disabled,
            alternate: self.alternate,
            integrity: self.integrity,
            crossorigin: self.crossorigin,
            nonce: self.nonce,
            referrerpolicy: self.referrerpolicy,
            fetchpriority: self.fetchpriority,
            blocking: self.blocking,
            blocking_tokens,
            text: self.text,
        }
    }
}

fn expected_tokens_from_raw(tokens: Vec<String>, raw: Option<&str>) -> Vec<String> {
    if !tokens.is_empty() {
        return tokens;
    }
    raw.map(split_html_test_tokens).unwrap_or_default()
}

fn split_html_test_tokens(raw: &str) -> Vec<String> {
    raw.split_ascii_whitespace()
        .filter(|token| !token.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

impl ExpectedAnchor {
    fn into_browser_anchor(self) -> BrowserAnchor {
        BrowserAnchor {
            id: self.id,
            name: self.name,
            text: self.text,
        }
    }
}

impl ExpectedHeading {
    fn into_browser_heading(self) -> BrowserHeading {
        BrowserHeading {
            level: self.level,
            text: self.text,
        }
    }
}

impl ExpectedLink {
    fn into_browser_link(self) -> BrowserLink {
        BrowserLink {
            element: self.element,
            id: self.id,
            href: self.href,
            resolved_href: self.resolved_href,
            name: self.name,
            target: self.target,
            effective_target: self.effective_target,
            rel: self.rel,
            rel_tokens: self.rel_tokens,
            rel_external: self.rel_external,
            rel_nofollow: self.rel_nofollow,
            rel_noopener: self.rel_noopener,
            rel_noreferrer: self.rel_noreferrer,
            title: self.title,
            download: self.download,
            ping: self.ping,
            resolved_ping: self.resolved_ping,
            attributionsrc: self.attributionsrc,
            resolved_attributionsrc: self.resolved_attributionsrc,
            hreflang: self.hreflang,
            type_hint: self.type_hint,
            referrerpolicy: self.referrerpolicy,
            text: self.text,
        }
    }
}

fn default_browser_link_element() -> String {
    "a".to_string()
}

impl ExpectedImage {
    fn into_browser_image(self) -> BrowserImage {
        BrowserImage {
            src: self.src,
            resolved_src: self.resolved_src,
            alt: self.alt,
            width: self.width,
            height: self.height,
            srcset: self.srcset,
            resolved_srcset: self.resolved_srcset,
            sizes: self.sizes,
            loading: self.loading,
            decoding: self.decoding,
            fetchpriority: self.fetchpriority,
            crossorigin: self.crossorigin,
            referrerpolicy: self.referrerpolicy,
            usemap: self.usemap,
            ismap: self.ismap,
            sources: self
                .sources
                .into_iter()
                .map(ExpectedImageSource::into_browser_image_source)
                .collect(),
        }
    }
}

impl ExpectedImageSource {
    fn into_browser_image_source(self) -> BrowserImageSource {
        BrowserImageSource {
            srcset: self.srcset,
            resolved_srcset: self.resolved_srcset,
            sizes: self.sizes,
            media: self.media,
            type_hint: self.type_hint,
        }
    }
}

impl ExpectedMedia {
    fn into_browser_media(self) -> BrowserMedia {
        BrowserMedia {
            kind: self.kind,
            src: self.src,
            resolved_src: self.resolved_src,
            poster: self.poster,
            resolved_poster: self.resolved_poster,
            width: self.width,
            height: self.height,
            controls: self.controls,
            autoplay: self.autoplay,
            loop_media: self.loop_media,
            muted: self.muted,
            playsinline: self.playsinline,
            preload: self.preload,
            crossorigin: self.crossorigin,
            controlslist: self.controlslist,
            controlslist_tokens: self.controlslist_tokens,
            disableremoteplayback: self.disableremoteplayback,
            disablepictureinpicture: self.disablepictureinpicture,
        }
    }
}

impl ExpectedEmbeddedContext {
    fn into_browser_embedded_context(self) -> BrowserEmbeddedContext {
        BrowserEmbeddedContext {
            element: self.element,
            url: self.url,
            resolved_url: self.resolved_url,
            browsing_context_name: self.browsing_context_name,
            title: self.title,
            type_hint: self.type_hint,
            width: self.width,
            height: self.height,
            loading: self.loading,
            fetchpriority: self.fetchpriority,
            csp: self.csp,
            sandbox: self.sandbox,
            allow: self.allow,
            allowfullscreen: self.allowfullscreen,
            referrerpolicy: self.referrerpolicy,
            srcdoc: self.srcdoc,
            credentialless: self.credentialless,
            fallback_text: self.fallback_text,
        }
    }
}

impl ExpectedInteractiveElement {
    fn into_browser_interactive_element(self) -> BrowserInteractiveElement {
        BrowserInteractiveElement {
            element: self.element,
            id: self.id,
            role: self.role,
            authored_role: self.authored_role,
            text: self.text,
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
            disabled: self.disabled,
        }
    }
}

impl ExpectedForm {
    fn into_browser_form(self) -> BrowserForm {
        BrowserForm {
            id: self.id,
            action: self.action,
            resolved_action: self.resolved_action,
            name: self.name,
            method: self.method,
            enctype: self.enctype,
            target: self.target,
            effective_target: self.effective_target,
            accept_charset: self.accept_charset,
            accept_charset_tokens: self.accept_charset_tokens,
            autocomplete: self.autocomplete,
            autocomplete_tokens: self.autocomplete_tokens,
            rel: self.rel,
            rel_tokens: self.rel_tokens,
            rel_external: self.rel_external,
            rel_nofollow: self.rel_nofollow,
            rel_noopener: self.rel_noopener,
            rel_noreferrer: self.rel_noreferrer,
            novalidate: self.novalidate,
            fieldsets: self
                .fieldsets
                .into_iter()
                .map(ExpectedFormFieldset::into_browser_form_fieldset)
                .collect(),
            labels: self
                .labels
                .into_iter()
                .map(ExpectedFormLabel::into_browser_form_label)
                .collect(),
            outputs: self
                .outputs
                .into_iter()
                .map(ExpectedFormOutput::into_browser_form_output)
                .collect(),
            controls: self
                .controls
                .into_iter()
                .map(ExpectedFormControl::into_browser_form_control)
                .collect(),
            submitters: self
                .submitters
                .into_iter()
                .map(ExpectedFormSubmitter::into_browser_form_submitter)
                .collect(),
        }
    }
}

impl ExpectedFormFieldset {
    fn into_browser_form_fieldset(self) -> BrowserFormFieldset {
        BrowserFormFieldset {
            id: self.id,
            form_owner: self.form_owner,
            legend: self.legend,
            disabled: self.disabled,
            control_ids: self.control_ids,
            control_names: self.control_names,
        }
    }
}

impl ExpectedFormLabel {
    fn into_browser_form_label(self) -> BrowserFormLabel {
        BrowserFormLabel {
            id: self.id,
            for_control: self.for_control,
            text: self.text,
            control_id: self.control_id,
            control_name: self.control_name,
            control_type: self.control_type,
            association: self.association,
        }
    }
}

impl ExpectedFormOutput {
    fn into_browser_form_output(self) -> BrowserFormOutput {
        BrowserFormOutput {
            id: self.id,
            name: self.name,
            form_owner: self.form_owner,
            for_tokens: self.for_tokens,
            value: self.value,
            text: self.text,
        }
    }
}

impl ExpectedFormControl {
    fn into_browser_form_control(self) -> BrowserFormControl {
        BrowserFormControl {
            id: self.id,
            control_type: self.control_type,
            name: self.name,
            form_owner: self.form_owner,
            labels: self.labels,
            accessible_name: self.accessible_name,
            accessible_description: self.accessible_description,
            placeholder: self.placeholder,
            autocomplete: self.autocomplete,
            autocomplete_tokens: self.autocomplete_tokens,
            autocapitalize: self.autocapitalize,
            enterkeyhint: self.enterkeyhint,
            dirname: self.dirname,
            spellcheck: self.spellcheck,
            autocorrect: self.autocorrect,
            accept: self.accept,
            accept_tokens: self.accept_tokens,
            capture: self.capture,
            src: self.src,
            resolved_src: self.resolved_src,
            alt: self.alt,
            width: self.width,
            height: self.height,
            inputmode: self.inputmode,
            pattern: self.pattern,
            min: self.min,
            max: self.max,
            step: self.step,
            minlength: self.minlength,
            maxlength: self.maxlength,
            size: self.size,
            rows: self.rows,
            cols: self.cols,
            wrap: self.wrap,
            list: self.list,
            datalist_options: self.datalist_options,
            output_for: self.output_for,
            form_action: self.form_action,
            resolved_form_action: self.resolved_form_action,
            form_enctype: self.form_enctype,
            form_method: self.form_method,
            form_target: self.form_target,
            form_novalidate: self.form_novalidate,
            value: self.value,
            successful: self.successful,
            submission_values: self.submission_values,
            autofocus: self.autofocus,
            disabled: self.disabled,
            required: self.required,
            readonly: self.readonly,
            will_validate: self.will_validate,
            validation_attributes: self.validation_attributes,
            validation_barred_reason: self.validation_barred_reason,
            checked: self.checked,
            multiple: self.multiple,
            selected_options: self.selected_options,
            option_items: self
                .option_items
                .into_iter()
                .map(ExpectedSelectOption::into_browser_select_option)
                .collect(),
            text: self.text,
            options: self.options,
        }
    }
}

impl ExpectedSelectOption {
    fn into_browser_select_option(self) -> BrowserSelectOption {
        BrowserSelectOption {
            value: self.value,
            label: self.label,
            text: self.text,
            selected: self.selected,
            disabled: self.disabled,
            group_label: self.group_label,
        }
    }
}

impl ExpectedFormSubmitter {
    fn into_browser_form_submitter(self) -> BrowserFormSubmitter {
        BrowserFormSubmitter {
            id: self.id,
            control_type: self.control_type,
            name: self.name,
            accessible_name: self.accessible_name,
            action: self.action,
            resolved_action: self.resolved_action,
            method: self.method,
            enctype: self.enctype,
            target: self.target,
            effective_target: self.effective_target,
            novalidate: self.novalidate,
            value: self.value,
        }
    }
}

impl ExpectedTable {
    fn into_browser_table(self) -> BrowserTable {
        BrowserTable {
            caption: self.caption,
            row_count: self.row_count,
            column_count: self.column_count,
            column_hint_count: self.column_hint_count,
            cell_count: self.cell_count,
            header_cell_count: self.header_cell_count,
        }
    }
}
