use coding_adventures_html_parser::{
    parse_browser_document, BrowserActivationDescriptor, BrowserAnchor, BrowserAriaCollection,
    BrowserAriaCollectionItem, BrowserAriaLiveRegion, BrowserAriaRange,
    BrowserAriaRelationDescriptor, BrowserCommandElement, BrowserComponentHydrationTarget,
    BrowserDataAttribute, BrowserDataAttributeDescriptor, BrowserDatalistOption, BrowserDisclosure,
    BrowserDisclosureStateDescriptor, BrowserDocument, BrowserDocumentMetadata,
    BrowserDocumentPolicyDescriptor, BrowserEmbeddedContext, BrowserEmbeddedPolicyDescriptor,
    BrowserEventHandlerDescriptor, BrowserFetchPolicyDescriptor, BrowserFocusNavigationDescriptor,
    BrowserForm, BrowserFormButton, BrowserFormChoiceControl, BrowserFormControl,
    BrowserFormDatalist, BrowserFormFieldset, BrowserFormFileControl, BrowserFormHiddenControl,
    BrowserFormImageControl, BrowserFormLabel, BrowserFormMeasurement, BrowserFormObject,
    BrowserFormObjectParam, BrowserFormOutput, BrowserFormPolicyDescriptor,
    BrowserFormPolicySubmitterDescriptor, BrowserFormSelect, BrowserFormSubmitter,
    BrowserFormSuccessfulControl, BrowserFormTextEntry, BrowserFormValidationControl,
    BrowserGlobalStateDescriptor, BrowserHeading, BrowserHttpEquivHint, BrowserImage,
    BrowserImageCandidateDescriptor, BrowserImageMap, BrowserImageMapArea, BrowserImageSource,
    BrowserInteractiveElement, BrowserLink, BrowserLoadingHintDescriptor, BrowserMedia,
    BrowserMediaPlaybackDescriptor, BrowserMediaSource, BrowserMediaTrack, BrowserMeta,
    BrowserMetadataDirective, BrowserNavigationGroup, BrowserNavigationTargetDescriptor,
    BrowserPopover, BrowserPopoverInvoker, BrowserRefresh, BrowserResource,
    BrowserResourceEndpointDescriptor, BrowserResourceHint, BrowserScript,
    BrowserScriptExecutionDescriptor, BrowserSectionLandmark, BrowserSelectOption,
    BrowserStructuredItem, BrowserStructuredProperty, BrowserStylesheet,
    BrowserStylesheetPlanningDescriptor, BrowserTable, BrowserTableCell, BrowserTemplate,
    BrowserTextSemantic, BrowserThemeColor,
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
    #[serde(default)]
    event_handler_descriptors: Vec<ExpectedEventHandlerDescriptor>,
    body_text: String,
    metas: Vec<ExpectedMeta>,
    resources: Vec<ExpectedResource>,
    #[serde(default)]
    scripts: Vec<ExpectedScript>,
    #[serde(default)]
    script_execution_descriptors: Option<Vec<ExpectedScriptExecutionDescriptor>>,
    #[serde(default)]
    stylesheets: Vec<ExpectedStylesheet>,
    #[serde(default)]
    stylesheet_planning_descriptors: Option<Vec<ExpectedStylesheetPlanningDescriptor>>,
    #[serde(default)]
    document_policy_descriptors: Vec<ExpectedDocumentPolicyDescriptor>,
    #[serde(default)]
    loading_hint_descriptors: Vec<ExpectedLoadingHintDescriptor>,
    #[serde(default)]
    fetch_policy_descriptors: Vec<ExpectedFetchPolicyDescriptor>,
    #[serde(default)]
    resource_endpoint_descriptors: Option<Vec<ExpectedResourceEndpointDescriptor>>,
    #[serde(default)]
    form_policy_descriptors: Vec<ExpectedFormPolicyDescriptor>,
    anchors: Vec<ExpectedAnchor>,
    headings: Vec<ExpectedHeading>,
    #[serde(default)]
    text_semantics: Vec<ExpectedTextSemantic>,
    #[serde(default)]
    navigation_target_descriptors: Vec<ExpectedNavigationTargetDescriptor>,
    #[serde(default)]
    navigation_groups: Vec<ExpectedNavigationGroup>,
    #[serde(default)]
    section_landmarks: Vec<ExpectedSectionLandmark>,
    #[serde(default)]
    command_elements: Vec<ExpectedCommandElement>,
    #[serde(default)]
    activation_descriptors: Option<Vec<ExpectedActivationDescriptor>>,
    #[serde(default)]
    popovers: Vec<ExpectedPopover>,
    #[serde(default)]
    aria_collections: Vec<ExpectedAriaCollection>,
    #[serde(default)]
    aria_ranges: Vec<ExpectedAriaRange>,
    #[serde(default)]
    aria_live_regions: Vec<ExpectedAriaLiveRegion>,
    #[serde(default)]
    aria_relation_descriptors: Vec<ExpectedAriaRelationDescriptor>,
    links: Vec<ExpectedLink>,
    images: Vec<ExpectedImage>,
    #[serde(default)]
    image_candidate_descriptors: Option<Vec<ExpectedImageCandidateDescriptor>>,
    #[serde(default)]
    image_maps: Vec<ExpectedImageMap>,
    #[serde(default)]
    media: Vec<ExpectedMedia>,
    #[serde(default)]
    media_playback_descriptors: Option<Vec<ExpectedMediaPlaybackDescriptor>>,
    #[serde(default)]
    embedded_contexts: Vec<ExpectedEmbeddedContext>,
    #[serde(default)]
    embedded_policy_descriptors: Option<Vec<ExpectedEmbeddedPolicyDescriptor>>,
    #[serde(default)]
    interactive_elements: Vec<ExpectedInteractiveElement>,
    #[serde(default)]
    focus_navigation_descriptors: Option<Vec<ExpectedFocusNavigationDescriptor>>,
    #[serde(default)]
    disclosures: Vec<ExpectedDisclosure>,
    #[serde(default)]
    disclosure_state_descriptors: Option<Vec<ExpectedDisclosureStateDescriptor>>,
    #[serde(default)]
    component_hydration_targets: Vec<ExpectedComponentHydrationTarget>,
    #[serde(default)]
    data_attribute_descriptors: Vec<ExpectedDataAttributeDescriptor>,
    #[serde(default)]
    global_state_descriptors: Vec<ExpectedGlobalStateDescriptor>,
    #[serde(default)]
    structured_items: Vec<ExpectedStructuredItem>,
    #[serde(default)]
    templates: Vec<ExpectedTemplate>,
    forms: Vec<ExpectedForm>,
    tables: Vec<ExpectedTable>,
    #[serde(default)]
    table_cells: Vec<ExpectedTableCell>,
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
struct ExpectedDataAttributeDescriptor {
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
    data_attributes: Vec<ExpectedDataAttribute>,
    #[serde(default)]
    text: String,
}

#[derive(Debug, Deserialize)]
struct ExpectedGlobalStateDescriptor {
    element: String,
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
    #[serde(default)]
    hidden: bool,
    #[serde(default)]
    inert: bool,
    #[serde(default)]
    tabindex: Option<String>,
    #[serde(default)]
    accesskey: Vec<String>,
    #[serde(default)]
    autofocus: bool,
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
    text: String,
}

#[derive(Debug, Deserialize)]
struct ExpectedAriaRelationDescriptor {
    element: String,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    text: String,
    #[serde(default)]
    aria_details: Vec<String>,
    #[serde(default)]
    details_text: Vec<String>,
    #[serde(default)]
    aria_errormessage: Vec<String>,
    #[serde(default)]
    errormessage_text: Vec<String>,
    #[serde(default)]
    aria_flowto: Vec<String>,
    #[serde(default)]
    flowto_text: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ExpectedDocumentPolicyDescriptor {
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
    content_security_policy: Option<String>,
    #[serde(default)]
    permissions_policy: Option<String>,
    #[serde(default)]
    origin_trials: Vec<String>,
    #[serde(default)]
    accept_ch: Option<String>,
    #[serde(default)]
    accept_ch_tokens: Vec<String>,
    #[serde(default)]
    dns_prefetch_control: Option<String>,
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
struct ExpectedNavigationTargetDescriptor {
    element: String,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    href: Option<String>,
    #[serde(default)]
    resolved_href: Option<String>,
    #[serde(default)]
    text: String,
    #[serde(default)]
    target: Option<String>,
    #[serde(default)]
    effective_target: Option<String>,
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
    #[serde(default)]
    area_shape: Option<String>,
    #[serde(default)]
    area_coords: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ExpectedLoadingHintDescriptor {
    element: String,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    url: Option<String>,
    #[serde(default)]
    resolved_url: Option<String>,
    #[serde(default)]
    loading: Option<String>,
    #[serde(default)]
    decoding: Option<String>,
    #[serde(default)]
    fetchpriority: Option<String>,
    #[serde(default)]
    blocking: Option<String>,
    #[serde(default)]
    blocking_tokens: Vec<String>,
    #[serde(default)]
    preload: Option<String>,
    #[serde(default)]
    as_hint: Option<String>,
    #[serde(default)]
    media: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ExpectedFetchPolicyDescriptor {
    element: String,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    url: Option<String>,
    #[serde(default)]
    resolved_url: Option<String>,
    #[serde(default)]
    integrity: Option<String>,
    #[serde(default)]
    crossorigin: Option<String>,
    #[serde(default)]
    nonce: Option<String>,
    #[serde(default)]
    referrerpolicy: Option<String>,
    #[serde(default)]
    csp: Option<String>,
    #[serde(default)]
    sandbox: Vec<String>,
    #[serde(default)]
    allow: Option<String>,
    #[serde(default)]
    allowfullscreen: bool,
    #[serde(default)]
    credentialless: bool,
}

#[derive(Debug, Deserialize)]
struct ExpectedEventHandlerDescriptor {
    element: String,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    classes: Vec<String>,
    #[serde(default)]
    role: Option<String>,
    source: String,
    #[serde(default)]
    event_handlers: Vec<String>,
    #[serde(default)]
    handler_count: usize,
    #[serde(default)]
    activation_handlers: Vec<String>,
    #[serde(default)]
    keyboard_handlers: Vec<String>,
    #[serde(default)]
    pointer_handlers: Vec<String>,
    #[serde(default)]
    form_handlers: Vec<String>,
    #[serde(default)]
    media_handlers: Vec<String>,
    #[serde(default)]
    lifecycle_handlers: Vec<String>,
    #[serde(default)]
    error_handlers: Vec<String>,
    #[serde(default)]
    text: String,
}

#[derive(Debug, Deserialize)]
struct ExpectedFormPolicyDescriptor {
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    name: Option<String>,
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
    submitters: Vec<ExpectedFormPolicySubmitterDescriptor>,
}

#[derive(Debug, Deserialize)]
struct ExpectedFormPolicySubmitterDescriptor {
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
struct ExpectedResourceEndpointDescriptor {
    endpoint_kind: String,
    element: String,
    #[serde(default)]
    resource_kind: Option<String>,
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
    #[serde(default)]
    async_script: bool,
    #[serde(default)]
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
struct ExpectedScriptExecutionDescriptor {
    script_kind: String,
    #[serde(default)]
    execution_kind: String,
    #[serde(default)]
    src: Option<String>,
    #[serde(default)]
    resolved_src: Option<String>,
    #[serde(default)]
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
    blocking_token_count: usize,
    #[serde(default)]
    render_blocking: bool,
    #[serde(default)]
    has_text: bool,
    #[serde(default)]
    text_length: usize,
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
struct ExpectedStylesheetPlanningDescriptor {
    source: String,
    stylesheet_kind: String,
    #[serde(default)]
    href: Option<String>,
    #[serde(default)]
    resolved_href: Option<String>,
    #[serde(default)]
    rel: Option<String>,
    #[serde(default)]
    rel_tokens: Vec<String>,
    #[serde(default)]
    rel_token_count: usize,
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
    applies_by_default: bool,
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
    blocking_token_count: usize,
    #[serde(default)]
    render_blocking: bool,
    #[serde(default)]
    has_text: bool,
    #[serde(default)]
    text_length: usize,
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
struct ExpectedTextSemantic {
    element: String,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    title: Option<String>,
    role: String,
    text: String,
    #[serde(default)]
    lang: Option<String>,
    #[serde(default)]
    dir: Option<String>,
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
    ruby_kind: Option<String>,
    #[serde(default)]
    bidi_kind: Option<String>,
    #[serde(default)]
    phrase_kind: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ExpectedNavigationGroup {
    element: String,
    #[serde(default)]
    id: Option<String>,
    role: String,
    text: String,
    #[serde(default)]
    accessible_name: Option<String>,
    #[serde(default)]
    aria_label: Option<String>,
    #[serde(default)]
    aria_labelledby: Vec<String>,
    #[serde(default)]
    landmark_kind: Option<String>,
    #[serde(default)]
    list_kind: Option<String>,
    item_count: usize,
    #[serde(default)]
    list_start: Option<String>,
    #[serde(default)]
    list_marker_type: Option<String>,
    #[serde(default)]
    list_reversed: bool,
}

#[derive(Debug, Deserialize)]
struct ExpectedSectionLandmark {
    element: String,
    #[serde(default)]
    id: Option<String>,
    role: String,
    #[serde(default)]
    authored_role: Option<String>,
    text: String,
    #[serde(default)]
    accessible_name: Option<String>,
    #[serde(default)]
    aria_label: Option<String>,
    #[serde(default)]
    aria_labelledby: Vec<String>,
    #[serde(default)]
    section_kind: Option<String>,
    #[serde(default)]
    landmark_kind: Option<String>,
    #[serde(default)]
    heading_level: Option<u8>,
    #[serde(default)]
    heading_text: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ExpectedCommandElement {
    element: String,
    #[serde(default)]
    id: Option<String>,
    role: String,
    #[serde(default)]
    authored_role: Option<String>,
    command_kind: String,
    text: String,
    #[serde(default)]
    accessible_name: Option<String>,
    #[serde(default)]
    accessible_description: Option<String>,
    #[serde(default)]
    href: Option<String>,
    #[serde(default)]
    resolved_href: Option<String>,
    #[serde(default)]
    target: Option<String>,
    #[serde(default)]
    effective_target: Option<String>,
    #[serde(default)]
    control_type: Option<String>,
    #[serde(default)]
    form_owner: Option<String>,
    #[serde(default)]
    form_action: Option<String>,
    #[serde(default)]
    resolved_form_action: Option<String>,
    #[serde(default)]
    form_method: Option<String>,
    #[serde(default)]
    form_target: Option<String>,
    #[serde(default)]
    form_novalidate: bool,
    #[serde(default)]
    command: Option<String>,
    #[serde(default)]
    command_for: Option<String>,
    #[serde(default)]
    popover_target: Option<String>,
    #[serde(default)]
    popover_target_action: Option<String>,
    #[serde(default)]
    aria_controls: Vec<String>,
    #[serde(default)]
    aria_expanded: Option<String>,
    #[serde(default)]
    aria_haspopup: Option<String>,
    #[serde(default)]
    aria_pressed: Option<String>,
    #[serde(default)]
    aria_current: Option<String>,
    #[serde(default)]
    aria_disabled: Option<String>,
    #[serde(default)]
    tabindex: Option<String>,
    #[serde(default)]
    accesskey: Vec<String>,
    #[serde(default)]
    event_handlers: Vec<String>,
    #[serde(default)]
    focusable: bool,
    #[serde(default)]
    disabled: bool,
}

#[derive(Debug, Deserialize)]
struct ExpectedActivationDescriptor {
    element: String,
    #[serde(default)]
    id: Option<String>,
    role: String,
    #[serde(default)]
    authored_role: Option<String>,
    command_kind: String,
    activation_kind: String,
    #[serde(default)]
    target_id: Option<String>,
    target_kind: String,
    #[serde(default)]
    text: String,
    #[serde(default)]
    accessible_name: Option<String>,
    #[serde(default)]
    accessible_description: Option<String>,
    #[serde(default)]
    disabled: bool,
    #[serde(default)]
    focusable: bool,
    #[serde(default)]
    tabindex: Option<String>,
    #[serde(default)]
    accesskey: Vec<String>,
    #[serde(default)]
    event_handlers: Vec<String>,
    #[serde(default)]
    handler_count: usize,
    #[serde(default)]
    command: Option<String>,
    #[serde(default)]
    command_for: Option<String>,
    #[serde(default)]
    popover_target: Option<String>,
    #[serde(default)]
    popover_target_action: Option<String>,
    #[serde(default)]
    aria_controls: Vec<String>,
    #[serde(default)]
    aria_expanded: Option<String>,
    #[serde(default)]
    aria_haspopup: Option<String>,
    #[serde(default)]
    aria_pressed: Option<String>,
    #[serde(default)]
    aria_current: Option<String>,
    #[serde(default)]
    aria_disabled: Option<String>,
    #[serde(default)]
    control_type: Option<String>,
    #[serde(default)]
    href: Option<String>,
    #[serde(default)]
    resolved_href: Option<String>,
    #[serde(default)]
    effective_target: Option<String>,
    #[serde(default)]
    form_owner: Option<String>,
    #[serde(default)]
    form_action: Option<String>,
    #[serde(default)]
    resolved_form_action: Option<String>,
    #[serde(default)]
    form_method: Option<String>,
    #[serde(default)]
    form_target: Option<String>,
    #[serde(default)]
    form_novalidate: bool,
}

#[derive(Debug, Deserialize)]
struct ExpectedPopover {
    element: String,
    #[serde(default)]
    id: Option<String>,
    role: String,
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
    popover: String,
    #[serde(default)]
    invokers: Vec<ExpectedPopoverInvoker>,
}

#[derive(Debug, Deserialize)]
struct ExpectedPopoverInvoker {
    element: String,
    #[serde(default)]
    id: Option<String>,
    text: String,
    #[serde(default)]
    accessible_name: Option<String>,
    command_kind: String,
    #[serde(default)]
    command: Option<String>,
    #[serde(default)]
    command_for: Option<String>,
    #[serde(default)]
    popover_target: Option<String>,
    #[serde(default)]
    popover_target_action: Option<String>,
    #[serde(default)]
    aria_controls: Vec<String>,
    #[serde(default)]
    aria_expanded: Option<String>,
    #[serde(default)]
    focusable: bool,
}

#[derive(Debug, Deserialize)]
struct ExpectedAriaCollection {
    element: String,
    #[serde(default)]
    id: Option<String>,
    role: String,
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
    aria_orientation: Option<String>,
    #[serde(default)]
    aria_multiselectable: Option<String>,
    #[serde(default)]
    aria_activedescendant: Option<String>,
    #[serde(default)]
    aria_owns: Vec<String>,
    #[serde(default)]
    item_count: usize,
    #[serde(default)]
    selected_item_count: usize,
    #[serde(default)]
    checked_item_count: usize,
    #[serde(default)]
    current_item_count: usize,
    #[serde(default)]
    disabled_item_count: usize,
    #[serde(default)]
    items: Vec<ExpectedAriaCollectionItem>,
}

#[derive(Debug, Deserialize)]
struct ExpectedAriaCollectionItem {
    element: String,
    #[serde(default)]
    id: Option<String>,
    role: String,
    text: String,
    #[serde(default)]
    accessible_name: Option<String>,
    #[serde(default)]
    aria_selected: Option<String>,
    #[serde(default)]
    aria_checked: Option<String>,
    #[serde(default)]
    aria_current: Option<String>,
    #[serde(default)]
    aria_disabled: Option<String>,
    #[serde(default)]
    aria_expanded: Option<String>,
    #[serde(default)]
    aria_level: Option<String>,
    #[serde(default)]
    aria_posinset: Option<String>,
    #[serde(default)]
    aria_setsize: Option<String>,
    #[serde(default)]
    aria_rowindex: Option<String>,
    #[serde(default)]
    aria_colindex: Option<String>,
    #[serde(default)]
    aria_controls: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ExpectedAriaRange {
    element: String,
    #[serde(default)]
    id: Option<String>,
    role: String,
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
    aria_valuenow: Option<String>,
    #[serde(default)]
    aria_valuemin: Option<String>,
    #[serde(default)]
    aria_valuemax: Option<String>,
    #[serde(default)]
    aria_valuetext: Option<String>,
    #[serde(default)]
    aria_orientation: Option<String>,
    #[serde(default)]
    aria_disabled: Option<String>,
    #[serde(default)]
    aria_readonly: Option<String>,
    #[serde(default)]
    aria_required: Option<String>,
    #[serde(default)]
    tabindex: Option<String>,
    #[serde(default)]
    text_value: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ExpectedAriaLiveRegion {
    element: String,
    #[serde(default)]
    id: Option<String>,
    role: String,
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
    aria_live: Option<String>,
    #[serde(default)]
    aria_busy: Option<String>,
    #[serde(default)]
    aria_atomic: Option<String>,
    #[serde(default)]
    aria_relevant: Vec<String>,
    #[serde(default)]
    aria_hidden: bool,
    update_kind: String,
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
struct ExpectedImageCandidateDescriptor {
    #[serde(default)]
    src: Option<String>,
    #[serde(default)]
    resolved_src: Option<String>,
    #[serde(default)]
    srcset: Option<String>,
    #[serde(default)]
    resolved_srcset: Option<String>,
    #[serde(default)]
    sizes: Option<String>,
    #[serde(default)]
    alt: Option<String>,
    #[serde(default)]
    width: Option<String>,
    #[serde(default)]
    height: Option<String>,
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
    has_alt: bool,
    #[serde(default)]
    source_count: usize,
    #[serde(default)]
    source_srcset_count: usize,
    #[serde(default)]
    candidate_count: usize,
    #[serde(default)]
    source_type_hints: Vec<String>,
    #[serde(default)]
    source_media: Vec<String>,
    #[serde(default)]
    sources: Vec<ExpectedImageSource>,
}

#[derive(Debug, Deserialize)]
struct ExpectedImageMap {
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    areas: Vec<ExpectedImageMapArea>,
}

#[derive(Debug, Deserialize)]
struct ExpectedImageMapArea {
    #[serde(default)]
    id: Option<String>,
    #[serde(default = "default_image_map_area_shape")]
    shape: String,
    #[serde(default)]
    coords: Option<String>,
    #[serde(default)]
    href: Option<String>,
    #[serde(default)]
    resolved_href: Option<String>,
    #[serde(default)]
    alt: Option<String>,
    #[serde(default)]
    target: Option<String>,
    #[serde(default)]
    effective_target: Option<String>,
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
    ping: Vec<String>,
    #[serde(default)]
    resolved_ping: Vec<String>,
    #[serde(default)]
    attributionsrc: Vec<String>,
    #[serde(default)]
    resolved_attributionsrc: Vec<String>,
    #[serde(default)]
    download: Option<String>,
    #[serde(default)]
    hreflang: Option<String>,
    #[serde(default)]
    referrerpolicy: Option<String>,
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
    #[serde(default)]
    sources: Vec<ExpectedMediaSource>,
    #[serde(default)]
    tracks: Vec<ExpectedMediaTrack>,
}

#[derive(Debug, Deserialize)]
struct ExpectedMediaSource {
    #[serde(default)]
    src: Option<String>,
    #[serde(default)]
    resolved_src: Option<String>,
    #[serde(default)]
    type_hint: Option<String>,
    #[serde(default)]
    media: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ExpectedMediaTrack {
    #[serde(default = "default_track_kind")]
    kind: String,
    #[serde(default)]
    src: Option<String>,
    #[serde(default)]
    resolved_src: Option<String>,
    #[serde(default)]
    srclang: Option<String>,
    #[serde(default)]
    label: Option<String>,
    #[serde(default)]
    default_track: bool,
}

#[derive(Debug, Deserialize)]
struct ExpectedMediaPlaybackDescriptor {
    kind: String,
    #[serde(default)]
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
    #[serde(default)]
    source_count: usize,
    #[serde(default)]
    sources: Vec<ExpectedMediaSource>,
    #[serde(default)]
    track_count: usize,
    #[serde(default)]
    default_track_count: usize,
    #[serde(default)]
    tracks: Vec<ExpectedMediaTrack>,
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
struct ExpectedEmbeddedPolicyDescriptor {
    element: String,
    #[serde(default)]
    resource_kind: String,
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
    sandbox_token_count: usize,
    #[serde(default)]
    allow: Option<String>,
    #[serde(default)]
    allowfullscreen: bool,
    #[serde(default)]
    referrerpolicy: Option<String>,
    #[serde(default)]
    srcdoc: Option<String>,
    #[serde(default)]
    has_srcdoc: bool,
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
struct ExpectedFocusNavigationDescriptor {
    element: String,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    role: Option<String>,
    #[serde(default)]
    authored_role: Option<String>,
    focus_kind: String,
    #[serde(default)]
    focusable: bool,
    #[serde(default)]
    sequential_focus: bool,
    #[serde(default)]
    programmatic_focus: bool,
    #[serde(default)]
    focus_blocked: bool,
    #[serde(default)]
    focus_block_reasons: Vec<String>,
    #[serde(default)]
    tabindex: Option<String>,
    #[serde(default)]
    tabindex_order: Option<i32>,
    #[serde(default)]
    accesskey: Vec<String>,
    #[serde(default)]
    event_handlers: Vec<String>,
    #[serde(default)]
    contenteditable: Option<String>,
    #[serde(default)]
    editing_mode: Option<String>,
    #[serde(default)]
    command: Option<String>,
    #[serde(default)]
    command_for: Option<String>,
    #[serde(default)]
    popover_target: Option<String>,
    #[serde(default)]
    popover_target_action: Option<String>,
    #[serde(default)]
    aria_controls: Vec<String>,
    #[serde(default)]
    aria_activedescendant: Option<String>,
    #[serde(default)]
    aria_expanded: Option<String>,
    #[serde(default)]
    aria_haspopup: Option<String>,
    #[serde(default)]
    aria_disabled: Option<String>,
    #[serde(default)]
    disabled: bool,
    #[serde(default)]
    hidden: bool,
    #[serde(default)]
    inert: bool,
    #[serde(default)]
    aria_hidden: bool,
    #[serde(default)]
    text: String,
}

#[derive(Debug, Deserialize)]
struct ExpectedDisclosure {
    element: String,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    text: String,
    #[serde(default)]
    summary_text: Option<String>,
    #[serde(default)]
    open: bool,
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
    aria_modal: Option<String>,
    #[serde(default)]
    closedby: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ExpectedDisclosureStateDescriptor {
    element: String,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    name: Option<String>,
    disclosure_kind: String,
    #[serde(default)]
    open: bool,
    #[serde(default)]
    grouped: bool,
    #[serde(default)]
    group_name: Option<String>,
    #[serde(default)]
    has_summary: bool,
    #[serde(default)]
    summary_text: Option<String>,
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
    aria_modal: Option<String>,
    #[serde(default)]
    modal: bool,
    #[serde(default)]
    closedby: Option<String>,
    #[serde(default)]
    text: String,
    #[serde(default)]
    text_length: usize,
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
    datalists: Vec<ExpectedFormDatalist>,
    #[serde(default)]
    selects: Vec<ExpectedFormSelect>,
    #[serde(default)]
    outputs: Vec<ExpectedFormOutput>,
    #[serde(default)]
    measurements: Vec<ExpectedFormMeasurement>,
    #[serde(default)]
    object_controls: Vec<ExpectedFormObject>,
    #[serde(default)]
    successful_controls: Vec<ExpectedFormSuccessfulControl>,
    #[serde(default)]
    validation_controls: Vec<ExpectedFormValidationControl>,
    #[serde(default)]
    buttons: Vec<ExpectedFormButton>,
    #[serde(default)]
    text_entries: Vec<ExpectedFormTextEntry>,
    #[serde(default)]
    choice_controls: Vec<ExpectedFormChoiceControl>,
    #[serde(default)]
    file_controls: Vec<ExpectedFormFileControl>,
    #[serde(default)]
    hidden_controls: Vec<ExpectedFormHiddenControl>,
    #[serde(default)]
    image_controls: Vec<ExpectedFormImageControl>,
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
struct ExpectedFormDatalist {
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    control_ids: Vec<String>,
    #[serde(default)]
    control_names: Vec<String>,
    #[serde(default)]
    options: Vec<ExpectedDatalistOption>,
}

#[derive(Debug, Deserialize)]
struct ExpectedDatalistOption {
    value: String,
    #[serde(default)]
    label: Option<String>,
    text: String,
    #[serde(default)]
    disabled: bool,
}

#[derive(Debug, Deserialize)]
struct ExpectedFormSelect {
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    form_owner: Option<String>,
    #[serde(default)]
    labels: Vec<String>,
    #[serde(default)]
    accessible_name: Option<String>,
    #[serde(default)]
    disabled: bool,
    #[serde(default)]
    required: bool,
    #[serde(default)]
    multiple: bool,
    #[serde(default)]
    size: Option<String>,
    #[serde(default)]
    value: Option<String>,
    #[serde(default)]
    selected_options: Vec<String>,
    #[serde(default)]
    options: Vec<ExpectedSelectOption>,
    text: String,
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
    labels: Vec<String>,
    #[serde(default)]
    accessible_name: Option<String>,
    #[serde(default)]
    accessible_description: Option<String>,
    #[serde(default)]
    for_tokens: Vec<String>,
    #[serde(default)]
    for_control_ids: Vec<String>,
    #[serde(default)]
    for_control_names: Vec<String>,
    #[serde(default)]
    for_control_types: Vec<String>,
    #[serde(default)]
    value: Option<String>,
    #[serde(default)]
    disabled: bool,
    #[serde(default)]
    will_validate: bool,
    #[serde(default)]
    validation_barred_reason: Option<String>,
    text: String,
}

#[derive(Debug, Deserialize)]
struct ExpectedFormMeasurement {
    #[serde(default)]
    id: Option<String>,
    measurement_type: String,
    #[serde(default)]
    labels: Vec<String>,
    #[serde(default)]
    accessible_name: Option<String>,
    #[serde(default)]
    accessible_description: Option<String>,
    #[serde(default)]
    value: Option<String>,
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
    indeterminate: bool,
    text: String,
}

#[derive(Debug, Deserialize)]
struct ExpectedFormObject {
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    form_owner: Option<String>,
    #[serde(default)]
    accessible_name: Option<String>,
    #[serde(default)]
    accessible_description: Option<String>,
    #[serde(default)]
    data: Option<String>,
    #[serde(default)]
    resolved_data: Option<String>,
    #[serde(default)]
    type_hint: Option<String>,
    #[serde(default)]
    width: Option<String>,
    #[serde(default)]
    height: Option<String>,
    #[serde(default)]
    usemap: Option<String>,
    #[serde(default)]
    fallback_text: String,
    #[serde(default)]
    params: Vec<ExpectedFormObjectParam>,
}

#[derive(Debug, Deserialize)]
struct ExpectedFormObjectParam {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    value: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ExpectedFormSuccessfulControl {
    #[serde(default)]
    id: Option<String>,
    control_type: String,
    name: String,
    #[serde(default)]
    form_owner: Option<String>,
    #[serde(default)]
    submission_values: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ExpectedFormValidationControl {
    #[serde(default)]
    id: Option<String>,
    control_type: String,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    form_owner: Option<String>,
    #[serde(default)]
    will_validate: bool,
    #[serde(default)]
    required: bool,
    #[serde(default)]
    validation_attributes: Vec<String>,
    #[serde(default)]
    validation_barred_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ExpectedFormTextEntry {
    #[serde(default)]
    id: Option<String>,
    control_type: String,
    #[serde(default)]
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
    value: Option<String>,
    text: String,
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
}

#[derive(Debug, Deserialize)]
struct ExpectedFormChoiceControl {
    #[serde(default)]
    id: Option<String>,
    control_type: String,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    form_owner: Option<String>,
    #[serde(default)]
    labels: Vec<String>,
    #[serde(default)]
    accessible_name: Option<String>,
    #[serde(default)]
    value: Option<String>,
    checked: bool,
    disabled: bool,
    #[serde(default)]
    required: bool,
    #[serde(default)]
    group_required: bool,
    #[serde(default)]
    successful: bool,
    #[serde(default)]
    submission_values: Vec<String>,
    #[serde(default)]
    will_validate: bool,
    #[serde(default)]
    validation_attributes: Vec<String>,
    #[serde(default)]
    validation_barred_reason: Option<String>,
    #[serde(default)]
    group_name: Option<String>,
    #[serde(default)]
    group_checked_ids: Vec<String>,
    #[serde(default)]
    group_checked_values: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ExpectedFormFileControl {
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    form_owner: Option<String>,
    #[serde(default)]
    labels: Vec<String>,
    #[serde(default)]
    accessible_name: Option<String>,
    #[serde(default)]
    accept: Option<String>,
    #[serde(default)]
    accept_tokens: Vec<String>,
    #[serde(default)]
    capture: Option<String>,
    #[serde(default)]
    multiple: bool,
    disabled: bool,
    #[serde(default)]
    required: bool,
    #[serde(default)]
    successful: bool,
    #[serde(default)]
    submission_values: Vec<String>,
    #[serde(default)]
    will_validate: bool,
    #[serde(default)]
    validation_attributes: Vec<String>,
    #[serde(default)]
    validation_barred_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ExpectedFormHiddenControl {
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    form_owner: Option<String>,
    #[serde(default)]
    value: Option<String>,
    #[serde(default)]
    autocomplete: Option<String>,
    #[serde(default)]
    autocomplete_tokens: Vec<String>,
    disabled: bool,
    #[serde(default)]
    successful: bool,
    #[serde(default)]
    submission_values: Vec<String>,
    #[serde(default)]
    will_validate: bool,
    #[serde(default)]
    validation_barred_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ExpectedFormImageControl {
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    form_owner: Option<String>,
    #[serde(default)]
    labels: Vec<String>,
    #[serde(default)]
    accessible_name: Option<String>,
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
    disabled: bool,
    #[serde(default)]
    autofocus: bool,
    #[serde(default)]
    submitter: bool,
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
    #[serde(default)]
    coordinate_names: Vec<String>,
    #[serde(default)]
    will_validate: bool,
    #[serde(default)]
    validation_barred_reason: Option<String>,
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
struct ExpectedFormButton {
    #[serde(default)]
    id: Option<String>,
    control_type: String,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    form_owner: Option<String>,
    #[serde(default)]
    accessible_name: Option<String>,
    #[serde(default)]
    disabled: bool,
    #[serde(default)]
    autofocus: bool,
    #[serde(default)]
    submitter: bool,
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
    text: String,
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

#[derive(Debug, Deserialize)]
struct ExpectedTableCell {
    table_index: usize,
    #[serde(default)]
    table_id: Option<String>,
    #[serde(default)]
    table_caption: Option<String>,
    #[serde(default)]
    section_kind: Option<String>,
    row_index: usize,
    column_index: usize,
    element: String,
    #[serde(default)]
    id: Option<String>,
    text: String,
    #[serde(default)]
    accessible_name: Option<String>,
    #[serde(default)]
    header: bool,
    #[serde(default)]
    scope: Option<String>,
    #[serde(default)]
    headers: Vec<String>,
    #[serde(default)]
    abbr: Option<String>,
    #[serde(default)]
    rowspan: Option<String>,
    #[serde(default)]
    colspan: Option<String>,
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
fn browser_table_cell_descriptor_metadata_tracks_headers_spans_and_sections() {
    let suite: BrowserReadinessSuite = serde_json::from_str(BROWSER_READINESS_FIXTURE)
        .expect("browser readiness fixture should parse");
    let case = suite
        .cases
        .into_iter()
        .find(|case| case.id == "table-cell-descriptor-page")
        .expect("table cell descriptor fixture case should exist");

    let actual = parse_browser_document(&case.input)
        .expect("table cell descriptor fixture should parse into browser document facts");
    let expected = case.expected.into_browser_document();

    assert_eq!(
        actual.table_cells, expected.table_cells,
        "table cell descriptors should preserve table context, sections, spans, headers, and grid positions",
    );
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
fn browser_image_map_descriptor_metadata_tracks_area_navigation() {
    let suite: BrowserReadinessSuite = serde_json::from_str(BROWSER_READINESS_FIXTURE)
        .expect("browser readiness fixture should parse");
    let case = suite
        .cases
        .into_iter()
        .find(|case| case.id == "responsive-image-metadata-page")
        .expect("responsive image fixture case should exist");

    let actual = parse_browser_document(&case.input)
        .expect("responsive image fixture should parse into browser document facts");
    let expected = case.expected.into_browser_document();

    assert_eq!(
        actual.image_maps, expected.image_maps,
        "image maps should preserve area geometry, link policy, and resolved navigation metadata",
    );
}

#[test]
fn browser_text_semantic_descriptor_metadata_tracks_inline_annotations() {
    let suite: BrowserReadinessSuite = serde_json::from_str(BROWSER_READINESS_FIXTURE)
        .expect("browser readiness fixture should parse");
    let case = suite
        .cases
        .into_iter()
        .find(|case| case.id == "inline-semantic-metadata-page")
        .expect("inline semantic fixture case should exist");

    let actual = parse_browser_document(&case.input)
        .expect("inline semantic fixture should parse into browser document facts");
    let expected = case.expected.into_browser_document();

    assert_eq!(
        actual.text_semantics, expected.text_semantics,
        "text semantics should preserve machine-readable values, edits, quotes, phrase semantics, ruby annotations, and bidi metadata",
    );
}

#[test]
fn browser_navigation_group_descriptor_metadata_tracks_lists_and_landmarks() {
    let suite: BrowserReadinessSuite = serde_json::from_str(BROWSER_READINESS_FIXTURE)
        .expect("browser readiness fixture should parse");
    let case = suite
        .cases
        .into_iter()
        .find(|case| case.id == "navigation-menu-descriptor-page")
        .expect("navigation menu fixture case should exist");

    let actual = parse_browser_document(&case.input)
        .expect("navigation menu fixture should parse into browser document facts");
    let expected = case.expected.into_browser_document();

    assert_eq!(
        actual.navigation_groups, expected.navigation_groups,
        "navigation groups should preserve list/menu landmark names, item counts, and ordered list metadata",
    );
}

#[test]
fn browser_section_landmark_descriptor_metadata_tracks_outline_roles_and_names() {
    let suite: BrowserReadinessSuite = serde_json::from_str(BROWSER_READINESS_FIXTURE)
        .expect("browser readiness fixture should parse");
    let case = suite
        .cases
        .into_iter()
        .find(|case| case.id == "document-outline-landmark-page")
        .expect("document outline fixture case should exist");

    let actual = parse_browser_document(&case.input)
        .expect("document outline fixture should parse into browser document facts");
    let expected = case.expected.into_browser_document();

    assert_eq!(
        actual.section_landmarks, expected.section_landmarks,
        "section landmarks should preserve roles, accessible names, landmark kinds, and first headings",
    );
}

#[test]
fn browser_command_element_descriptor_metadata_tracks_activation_surfaces() {
    let suite: BrowserReadinessSuite = serde_json::from_str(BROWSER_READINESS_FIXTURE)
        .expect("browser readiness fixture should parse");
    let case = suite
        .cases
        .into_iter()
        .find(|case| case.id == "interactive-element-state-page")
        .expect("interactive element fixture case should exist");

    let actual = parse_browser_document(&case.input)
        .expect("interactive element fixture should parse into browser document facts");
    let expected = case.expected.into_browser_document();

    assert_eq!(
        actual.command_elements, expected.command_elements,
        "command descriptors should preserve routed, ARIA, popover, and disclosure activation metadata",
    );
}

#[test]
fn browser_activation_descriptors_track_command_routes_and_state() {
    let suite: BrowserReadinessSuite = serde_json::from_str(BROWSER_READINESS_FIXTURE)
        .expect("browser readiness fixture should parse");
    let case = suite
        .cases
        .into_iter()
        .find(|case| case.id == "interactive-element-state-page")
        .expect("interactive activation fixture case should exist");

    let actual = parse_browser_document(&case.input)
        .expect("interactive activation fixture should parse into browser document facts");
    let expected = case.expected.into_browser_document();

    assert_eq!(
        actual.activation_descriptors, expected.activation_descriptors,
        "activation descriptors should preserve command, popover, disclosure, ARIA, focus, and inline handler routing metadata",
    );
}

#[test]
fn browser_popover_descriptor_metadata_tracks_hosts_and_invokers() {
    let suite: BrowserReadinessSuite = serde_json::from_str(BROWSER_READINESS_FIXTURE)
        .expect("browser readiness fixture should parse");
    let case = suite
        .cases
        .into_iter()
        .find(|case| case.id == "popover-command-descriptor-page")
        .expect("popover command fixture case should exist");

    let actual = parse_browser_document(&case.input)
        .expect("popover command fixture should parse into browser document facts");
    let expected = case.expected.into_browser_document();

    assert_eq!(
        actual.popovers, expected.popovers,
        "popover descriptors should preserve host metadata and popovertarget/commandfor invoker relationships",
    );
}

#[test]
fn browser_aria_collection_descriptor_metadata_tracks_grouped_composites() {
    let suite: BrowserReadinessSuite = serde_json::from_str(BROWSER_READINESS_FIXTURE)
        .expect("browser readiness fixture should parse");
    let case = suite
        .cases
        .into_iter()
        .find(|case| case.id == "aria-collection-descriptor-page")
        .expect("ARIA collection fixture case should exist");

    let actual = parse_browser_document(&case.input)
        .expect("ARIA collection fixture should parse into browser document facts");
    let expected = case.expected.into_browser_document();

    assert_eq!(
        actual.aria_collections, expected.aria_collections,
        "ARIA collection descriptors should preserve composite roles, active descendants, item roles, and item states",
    );
}

#[test]
fn browser_aria_range_descriptor_metadata_tracks_value_widgets() {
    let suite: BrowserReadinessSuite = serde_json::from_str(BROWSER_READINESS_FIXTURE)
        .expect("browser readiness fixture should parse");
    let case = suite
        .cases
        .into_iter()
        .find(|case| case.id == "aria-range-descriptor-page")
        .expect("ARIA range fixture case should exist");

    let actual = parse_browser_document(&case.input)
        .expect("ARIA range fixture should parse into browser document facts");
    let expected = case.expected.into_browser_document();

    assert_eq!(
        actual.aria_ranges, expected.aria_ranges,
        "ARIA range descriptors should preserve value bounds, value text, orientation, and value widget states",
    );
}

#[test]
fn browser_aria_live_region_descriptor_metadata_tracks_update_semantics() {
    let suite: BrowserReadinessSuite = serde_json::from_str(BROWSER_READINESS_FIXTURE)
        .expect("browser readiness fixture should parse");
    let case = suite
        .cases
        .into_iter()
        .find(|case| case.id == "aria-live-region-descriptor-page")
        .expect("ARIA live-region fixture case should exist");

    let actual = parse_browser_document(&case.input)
        .expect("ARIA live-region fixture should parse into browser document facts");
    let expected = case.expected.into_browser_document();

    assert_eq!(
        actual.aria_live_regions, expected.aria_live_regions,
        "ARIA live-region descriptors should preserve live politeness, busy/atomic/relevant flags, and implicit update semantics",
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
fn browser_form_validation_control_descriptors_track_candidates_and_barred_controls() {
    let document = parse_browser_document(
        "<form id=signup>\
         <input id=email name=email type=email required minlength=3 maxlength=80>\
         <input id=age name=age type=number min=18 max=120 step=1>\
         <textarea id=bio name=bio readonly maxlength=200>About me</textarea>\
         <input id=token type=hidden name=token value=abc>\
         <fieldset disabled><legend>Legacy</legend><input id=legacy name=legacy required></fieldset>\
         <output id=preview name=preview for=\"email age\">Preview</output>\
         <input id=go type=image name=go src=go.png alt=Go>\
         </form>\
         <input id=external form=signup name=outside required>",
    )
    .expect("form validation descriptor fixture should parse");

    let form = document
        .forms
        .first()
        .expect("signup form should be summarized");
    let descriptors = &form.validation_controls;
    let ids: Vec<&str> = descriptors
        .iter()
        .filter_map(|control| control.id.as_deref())
        .collect();
    assert_eq!(
        ids,
        vec!["email", "age", "bio", "token", "legacy", "preview", "go", "external"]
    );

    assert!(descriptors[0].will_validate);
    assert!(descriptors[0].required);
    assert_eq!(
        descriptors[0].validation_attributes,
        vec!["required", "minlength", "maxlength"]
    );
    assert!(descriptors[1].will_validate);
    assert_eq!(
        descriptors[1].validation_attributes,
        vec!["min", "max", "step"]
    );
    assert!(!descriptors[2].will_validate);
    assert_eq!(
        descriptors[2].validation_barred_reason.as_deref(),
        Some("readonly")
    );
    assert_eq!(
        descriptors[3].validation_barred_reason.as_deref(),
        Some("input-type-hidden")
    );
    assert!(!descriptors[4].will_validate);
    assert!(descriptors[4].required);
    assert_eq!(
        descriptors[4].validation_barred_reason.as_deref(),
        Some("disabled")
    );
    assert_eq!(
        descriptors[5].validation_barred_reason.as_deref(),
        Some("output")
    );
    assert_eq!(
        descriptors[6].validation_barred_reason.as_deref(),
        Some("input-type-image")
    );
    assert_eq!(descriptors[7].form_owner.as_deref(), Some("signup"));
    assert!(descriptors[7].will_validate);
    assert_eq!(descriptors[7].validation_attributes, vec!["required"]);
}

#[test]
fn browser_form_output_descriptor_metadata_tracks_for_references_and_labels() {
    let document = parse_browser_document(
        "<form id=calc>\
         <label for=sum>Sum</label>\
         <input id=a name=a value=1>\
         <input id=b name=b value=2>\
         <p id=hint>Total of inputs</p>\
         <output id=sum name=sum for=\"a b missing\" aria-describedby=hint>3</output>\
         <fieldset disabled><legend>Preview</legend><output id=disabled name=preview for=a>Disabled output</output></fieldset>\
         </form>\
         <output id=external form=calc name=external for=sum aria-label=External>External</output>",
    )
    .expect("form output descriptor fixture should parse");

    let form = document
        .forms
        .first()
        .expect("calc form should be summarized");
    let ids: Vec<&str> = form
        .outputs
        .iter()
        .filter_map(|output| output.id.as_deref())
        .collect();
    assert_eq!(ids, vec!["sum", "disabled", "external"]);

    let sum = &form.outputs[0];
    assert_eq!(sum.name.as_deref(), Some("sum"));
    assert_eq!(sum.labels, vec!["Sum"]);
    assert_eq!(sum.accessible_name.as_deref(), Some("Sum"));
    assert_eq!(
        sum.accessible_description.as_deref(),
        Some("Total of inputs")
    );
    assert_eq!(sum.for_tokens, vec!["a", "b", "missing"]);
    assert_eq!(sum.for_control_ids, vec!["a", "b"]);
    assert_eq!(sum.for_control_names, vec!["a", "b"]);
    assert_eq!(sum.for_control_types, vec!["text", "text"]);
    assert_eq!(sum.value.as_deref(), Some("3"));
    assert_eq!(sum.text, "3");
    assert!(!sum.disabled);
    assert!(!sum.will_validate);
    assert_eq!(sum.validation_barred_reason.as_deref(), Some("output"));

    let disabled = &form.outputs[1];
    assert_eq!(disabled.name.as_deref(), Some("preview"));
    assert!(disabled.disabled);
    assert_eq!(
        disabled.validation_barred_reason.as_deref(),
        Some("disabled")
    );
    assert_eq!(disabled.for_control_ids, vec!["a"]);
    assert_eq!(disabled.for_control_names, vec!["a"]);

    let external = &form.outputs[2];
    assert_eq!(external.form_owner.as_deref(), Some("calc"));
    assert_eq!(external.accessible_name.as_deref(), Some("External"));
    assert_eq!(external.for_tokens, vec!["sum"]);
    assert_eq!(external.for_control_ids, vec!["sum"]);
    assert_eq!(external.for_control_names, vec!["sum"]);
    assert_eq!(external.for_control_types, vec!["output"]);
    assert_eq!(external.value.as_deref(), Some("External"));
}

#[test]
fn browser_form_object_descriptor_metadata_tracks_embedded_controls_and_params() {
    let document = parse_browser_document(
        "<base href=\"https://example.test/media/\">\
         <form id=player>\
         <p id=movie-help>Legacy plugin fallback</p>\
         <fieldset id=plugins><legend>Plugins</legend>\
         <object id=movie name=movie data=movie.swf type=\"application/x-shockwave-flash\" \
             width=400 height=300 usemap=#movie-map aria-label=\"Movie plugin\" aria-describedby=movie-help>\
             <param name=autoplay value=false><param name=quality value=high>Fallback player\
         </object>\
         </fieldset>\
         <input id=token name=token value=ok>\
         </form>\
         <object id=external name=external form=player data=/external.bin type=\"application/octet-stream\" aria-label=External>External fallback</object>\
         <object id=outside name=outside data=outside.bin>Outside fallback</object>",
    )
    .expect("form object descriptor fixture should parse");

    let form = document
        .forms
        .first()
        .expect("player form should be summarized");

    let object_ids: Vec<&str> = form
        .object_controls
        .iter()
        .filter_map(|object| object.id.as_deref())
        .collect();
    assert_eq!(object_ids, vec!["movie", "external"]);

    let movie = &form.object_controls[0];
    assert_eq!(movie.name.as_deref(), Some("movie"));
    assert_eq!(movie.accessible_name.as_deref(), Some("Movie plugin"));
    assert_eq!(
        movie.accessible_description.as_deref(),
        Some("Legacy plugin fallback")
    );
    assert_eq!(movie.data.as_deref(), Some("movie.swf"));
    assert_eq!(
        movie.resolved_data.as_deref(),
        Some("https://example.test/media/movie.swf")
    );
    assert_eq!(
        movie.type_hint.as_deref(),
        Some("application/x-shockwave-flash")
    );
    assert_eq!(movie.width.as_deref(), Some("400"));
    assert_eq!(movie.height.as_deref(), Some("300"));
    assert_eq!(movie.usemap.as_deref(), Some("#movie-map"));
    assert_eq!(movie.fallback_text, "Fallback player");
    assert_eq!(movie.params.len(), 2);
    assert_eq!(movie.params[0].name.as_deref(), Some("autoplay"));
    assert_eq!(movie.params[0].value.as_deref(), Some("false"));
    assert_eq!(movie.params[1].name.as_deref(), Some("quality"));
    assert_eq!(movie.params[1].value.as_deref(), Some("high"));

    let external = &form.object_controls[1];
    assert_eq!(external.form_owner.as_deref(), Some("player"));
    assert_eq!(external.name.as_deref(), Some("external"));
    assert_eq!(external.accessible_name.as_deref(), Some("External"));
    assert_eq!(
        external.resolved_data.as_deref(),
        Some("https://example.test/external.bin")
    );

    assert_eq!(form.fieldsets[0].control_ids, vec!["movie"]);
    assert_eq!(form.fieldsets[0].control_names, vec!["movie"]);
    assert_eq!(form.controls.len(), 1);
    assert_eq!(form.controls[0].id.as_deref(), Some("token"));
    assert_eq!(form.successful_controls.len(), 1);
    assert_eq!(form.successful_controls[0].name, "token");
    assert_eq!(form.validation_controls.len(), 1);
    assert_eq!(form.validation_controls[0].id.as_deref(), Some("token"));
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
fn browser_form_successful_control_descriptor_metadata_tracks_submission_entries() {
    let document = parse_browser_document(
        "<form id=checkout>\
         <input id=item name=item value=book>\
         <input id=gift type=checkbox name=gift value=yes checked>\
         <input id=skip type=checkbox name=skip value=yes>\
         <input id=disabled name=disabled value=no disabled>\
         <select id=shipping name=shipping><option value=ground>Ground<option value=air selected>Air</select>\
         <textarea id=note name=note>Leave at desk</textarea>\
         <input id=file type=file name=upload>\
         <button name=go>Submit</button></form>\
         <input id=outside form=checkout name=outside value=external>",
    )
    .expect("form successful-control descriptor fixture should parse");

    let form = document
        .forms
        .first()
        .expect("checkout form should be summarized");
    let names: Vec<&str> = form
        .successful_controls
        .iter()
        .map(|control| control.name.as_str())
        .collect();
    assert_eq!(
        names,
        vec!["item", "gift", "shipping", "note", "upload", "outside"]
    );
    assert_eq!(form.successful_controls[0].id.as_deref(), Some("item"));
    assert_eq!(form.successful_controls[0].control_type, "text");
    assert_eq!(form.successful_controls[0].submission_values, vec!["book"]);
    assert_eq!(form.successful_controls[1].control_type, "checkbox");
    assert_eq!(form.successful_controls[1].submission_values, vec!["yes"]);
    assert_eq!(form.successful_controls[2].control_type, "select");
    assert_eq!(form.successful_controls[2].submission_values, vec!["air"]);
    assert_eq!(form.successful_controls[3].control_type, "textarea");
    assert_eq!(
        form.successful_controls[3].submission_values,
        vec!["Leave at desk"]
    );
    assert_eq!(form.successful_controls[4].control_type, "file");
    assert!(form.successful_controls[4].submission_values.is_empty());
    assert_eq!(
        form.successful_controls[5].form_owner.as_deref(),
        Some("checkout")
    );
    assert_eq!(
        form.successful_controls[5].submission_values,
        vec!["external"]
    );
}

#[test]
fn browser_form_hidden_descriptor_metadata_tracks_hidden_entries_and_form_owners() {
    let document = parse_browser_document(
        "<form id=session>\
         <input id=csrf type=hidden name=csrf value=token autocomplete=\"section-auth one-time-code\">\
         <input id=charset type=hidden name=_charset_ value=utf-8>\
         <input id=disabled-token type=hidden name=disabled value=no disabled>\
         </form>\
         <input id=external type=hidden form=session name=outside value=external>",
    )
    .expect("form hidden descriptor fixture should parse");

    let form = document
        .forms
        .first()
        .expect("session form should be summarized");
    let ids: Vec<&str> = form
        .hidden_controls
        .iter()
        .filter_map(|hidden| hidden.id.as_deref())
        .collect();
    assert_eq!(ids, vec!["csrf", "charset", "disabled-token", "external"]);

    let csrf = &form.hidden_controls[0];
    assert_eq!(csrf.name.as_deref(), Some("csrf"));
    assert_eq!(csrf.value.as_deref(), Some("token"));
    assert_eq!(
        csrf.autocomplete.as_deref(),
        Some("section-auth one-time-code")
    );
    assert_eq!(
        csrf.autocomplete_tokens,
        vec!["section-auth", "one-time-code"]
    );
    assert!(csrf.successful);
    assert_eq!(csrf.submission_values, vec!["token"]);
    assert!(!csrf.will_validate);
    assert_eq!(
        csrf.validation_barred_reason.as_deref(),
        Some("input-type-hidden")
    );

    let charset = &form.hidden_controls[1];
    assert_eq!(charset.name.as_deref(), Some("_charset_"));
    assert_eq!(charset.value.as_deref(), Some("utf-8"));
    assert!(charset.successful);
    assert_eq!(charset.submission_values, vec!["utf-8"]);

    let disabled = &form.hidden_controls[2];
    assert_eq!(disabled.name.as_deref(), Some("disabled"));
    assert!(disabled.disabled);
    assert!(!disabled.successful);
    assert!(disabled.submission_values.is_empty());
    assert_eq!(
        disabled.validation_barred_reason.as_deref(),
        Some("disabled")
    );

    let external = &form.hidden_controls[3];
    assert_eq!(external.form_owner.as_deref(), Some("session"));
    assert_eq!(external.name.as_deref(), Some("outside"));
    assert_eq!(external.submission_values, vec!["external"]);
}

#[test]
fn browser_form_button_descriptor_metadata_tracks_submitters_and_button_controls() {
    let document = parse_browser_document(
        "<base href=\"https://example.test/forms/index.html\" target=_base>\
         <form id=actions action=submit method=post target=_form novalidate>\
         <button id=save name=save value=s type=submit formaction=save formenctype=text/plain \
             formmethod=get formtarget=_save formnovalidate autofocus>Save</button>\
         <button id=reset type=reset name=reset value=r>Reset</button>\
         <input id=plain type=button name=plain value=Plain>\
         <input id=image type=image name=img src=go.png alt=Image width=20 height=10>\
         <button id=disabled name=disabled disabled>Disabled</button>\
         </form>\
         <button id=external form=actions type=submit name=outside formtarget=_outside>Outside</button>",
    )
    .expect("form button descriptor fixture should parse");

    let form = document
        .forms
        .first()
        .expect("actions form should be summarized");
    let ids: Vec<&str> = form
        .buttons
        .iter()
        .filter_map(|button| button.id.as_deref())
        .collect();
    assert_eq!(
        ids,
        vec!["save", "reset", "plain", "image", "disabled", "external"]
    );

    let save = &form.buttons[0];
    assert!(save.submitter);
    assert!(save.autofocus);
    assert_eq!(save.accessible_name.as_deref(), Some("Save"));
    assert_eq!(save.action.as_deref(), Some("save"));
    assert_eq!(
        save.resolved_action.as_deref(),
        Some("https://example.test/forms/save")
    );
    assert_eq!(save.method, "get");
    assert_eq!(save.enctype.as_deref(), Some("text/plain"));
    assert_eq!(save.target.as_deref(), Some("_save"));
    assert_eq!(save.effective_target.as_deref(), Some("_save"));
    assert!(save.novalidate);
    assert_eq!(save.value.as_deref(), Some("s"));
    assert_eq!(save.text, "Save");

    let reset = &form.buttons[1];
    assert_eq!(reset.control_type, "reset");
    assert!(!reset.submitter);
    assert_eq!(reset.method, "post");
    assert_eq!(reset.text, "Reset");

    let plain = &form.buttons[2];
    assert_eq!(plain.control_type, "button");
    assert!(!plain.submitter);
    assert_eq!(plain.value.as_deref(), Some("Plain"));
    assert!(plain.text.is_empty());

    let image = &form.buttons[3];
    assert_eq!(image.control_type, "image");
    assert!(image.submitter);
    assert_eq!(image.accessible_name.as_deref(), Some("Image"));
    assert_eq!(
        image.resolved_action.as_deref(),
        Some("https://example.test/forms/submit")
    );
    assert_eq!(image.effective_target.as_deref(), Some("_form"));
    assert_eq!(image.src.as_deref(), Some("go.png"));
    assert_eq!(
        image.resolved_src.as_deref(),
        Some("https://example.test/forms/go.png")
    );
    assert_eq!(image.width.as_deref(), Some("20"));
    assert_eq!(image.height.as_deref(), Some("10"));

    let disabled = &form.buttons[4];
    assert!(disabled.disabled);
    assert!(!disabled.submitter);
    assert!(!disabled.novalidate);

    let external = &form.buttons[5];
    assert_eq!(external.form_owner.as_deref(), Some("actions"));
    assert!(external.submitter);
    assert_eq!(external.effective_target.as_deref(), Some("_outside"));
    assert_eq!(external.text, "Outside");
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
fn browser_form_text_entry_descriptor_metadata_tracks_textual_controls_and_editing_hints() {
    let document = parse_browser_document(
        "<form id=profile>\
         <label for=email>Email</label>\
         <input id=email type=email name=email placeholder=Email required \
             autocomplete=\"section-contact email\" inputmode=email minlength=3 maxlength=80 size=30>\
         <input id=search type=search name=q value=rust autocapitalize=words \
             enterkeyhint=search dirname=q.dir spellcheck=false autocorrect=off list=suggestions>\
         <datalist id=suggestions><option value=Rust><option value=HTML label=Markup></datalist>\
         <textarea id=bio name=bio rows=4 cols=40 wrap=hard readonly maxlength=200>About me</textarea>\
         <input id=age type=number name=age value=42 min=18 max=120 step=1>\
         <input id=check type=checkbox name=check checked>\
         </form>\
         <textarea id=external form=profile name=outside placeholder=Outside>External note</textarea>",
    )
    .expect("form text-entry descriptor fixture should parse");

    let form = document
        .forms
        .first()
        .expect("profile form should be summarized");
    let ids: Vec<&str> = form
        .text_entries
        .iter()
        .filter_map(|entry| entry.id.as_deref())
        .collect();
    assert_eq!(ids, vec!["email", "search", "bio", "age", "external"]);

    let email = &form.text_entries[0];
    assert_eq!(email.control_type, "email");
    assert_eq!(email.labels, vec!["Email"]);
    assert_eq!(email.accessible_name.as_deref(), Some("Email"));
    assert_eq!(email.placeholder.as_deref(), Some("Email"));
    assert_eq!(email.autocomplete_tokens, vec!["section-contact", "email"]);
    assert_eq!(email.inputmode.as_deref(), Some("email"));
    assert_eq!(email.minlength.as_deref(), Some("3"));
    assert_eq!(email.maxlength.as_deref(), Some("80"));
    assert_eq!(email.size.as_deref(), Some("30"));
    assert!(email.required);
    assert!(email.will_validate);
    assert_eq!(
        email.validation_attributes,
        vec!["required", "minlength", "maxlength"]
    );

    let search = &form.text_entries[1];
    assert_eq!(search.control_type, "search");
    assert_eq!(search.value.as_deref(), Some("rust"));
    assert_eq!(search.autocapitalize.as_deref(), Some("words"));
    assert_eq!(search.enterkeyhint.as_deref(), Some("search"));
    assert_eq!(search.dirname.as_deref(), Some("q.dir"));
    assert_eq!(search.spellcheck.as_deref(), Some("false"));
    assert_eq!(search.autocorrect.as_deref(), Some("off"));
    assert_eq!(search.list.as_deref(), Some("suggestions"));
    assert_eq!(search.datalist_options, vec!["Rust", "HTML"]);

    let bio = &form.text_entries[2];
    assert_eq!(bio.control_type, "textarea");
    assert_eq!(bio.text, "About me");
    assert_eq!(bio.rows.as_deref(), Some("4"));
    assert_eq!(bio.cols.as_deref(), Some("40"));
    assert_eq!(bio.wrap.as_deref(), Some("hard"));
    assert!(bio.readonly);
    assert!(!bio.will_validate);
    assert_eq!(bio.validation_barred_reason.as_deref(), Some("readonly"));

    let age = &form.text_entries[3];
    assert_eq!(age.control_type, "number");
    assert_eq!(age.min.as_deref(), Some("18"));
    assert_eq!(age.max.as_deref(), Some("120"));
    assert_eq!(age.step.as_deref(), Some("1"));

    let external = &form.text_entries[4];
    assert_eq!(external.form_owner.as_deref(), Some("profile"));
    assert_eq!(external.name.as_deref(), Some("outside"));
    assert_eq!(external.placeholder.as_deref(), Some("Outside"));
    assert_eq!(external.text, "External note");
}

#[test]
fn browser_form_choice_descriptor_metadata_tracks_checkbox_radio_state_and_groups() {
    let document = parse_browser_document(
        "<form id=prefs>\
         <label><input id=news type=checkbox name=news value=yes checked required>Newsletter</label>\
         <input id=updates type=checkbox name=updates>\
         <label for=plan-basic>Basic</label>\
         <input id=plan-basic type=radio name=plan value=basic checked required>\
         <input id=plan-pro type=radio name=plan value=pro>\
         <fieldset disabled><label><input id=legacy type=radio name=legacy value=old checked>Legacy</label></fieldset>\
         </form>\
         <input id=external type=radio form=prefs name=plan value=outside>",
    )
    .expect("form choice descriptor fixture should parse");

    let form = document
        .forms
        .first()
        .expect("prefs form should be summarized");
    let ids: Vec<&str> = form
        .choice_controls
        .iter()
        .filter_map(|choice| choice.id.as_deref())
        .collect();
    assert_eq!(
        ids,
        vec![
            "news",
            "updates",
            "plan-basic",
            "plan-pro",
            "legacy",
            "external"
        ]
    );

    let news = &form.choice_controls[0];
    assert_eq!(news.control_type, "checkbox");
    assert_eq!(news.labels, vec!["Newsletter"]);
    assert_eq!(news.accessible_name.as_deref(), Some("Newsletter"));
    assert_eq!(news.value.as_deref(), Some("yes"));
    assert!(news.checked);
    assert!(news.required);
    assert!(news.group_required);
    assert!(news.successful);
    assert_eq!(news.submission_values, vec!["yes"]);
    assert_eq!(news.group_checked_ids, vec!["news"]);
    assert_eq!(news.group_checked_values, vec!["yes"]);
    assert_eq!(news.validation_attributes, vec!["required"]);

    let updates = &form.choice_controls[1];
    assert_eq!(updates.control_type, "checkbox");
    assert_eq!(updates.value.as_deref(), Some("on"));
    assert!(!updates.checked);
    assert!(!updates.successful);
    assert!(updates.submission_values.is_empty());
    assert!(updates.group_checked_ids.is_empty());
    assert!(updates.group_checked_values.is_empty());

    let plan_basic = &form.choice_controls[2];
    assert_eq!(plan_basic.control_type, "radio");
    assert_eq!(plan_basic.labels, vec!["Basic"]);
    assert_eq!(plan_basic.group_name.as_deref(), Some("plan"));
    assert!(plan_basic.checked);
    assert!(plan_basic.required);
    assert!(plan_basic.group_required);
    assert_eq!(plan_basic.group_checked_ids, vec!["plan-basic"]);
    assert_eq!(plan_basic.group_checked_values, vec!["basic"]);

    let plan_pro = &form.choice_controls[3];
    assert_eq!(plan_pro.control_type, "radio");
    assert_eq!(plan_pro.group_name.as_deref(), Some("plan"));
    assert!(!plan_pro.required);
    assert!(plan_pro.group_required);
    assert_eq!(plan_pro.group_checked_ids, vec!["plan-basic"]);
    assert_eq!(plan_pro.group_checked_values, vec!["basic"]);

    let legacy = &form.choice_controls[4];
    assert_eq!(legacy.control_type, "radio");
    assert!(legacy.checked);
    assert!(legacy.disabled);
    assert!(!legacy.successful);
    assert_eq!(legacy.validation_barred_reason.as_deref(), Some("disabled"));
    assert_eq!(legacy.group_checked_values, vec!["old"]);

    let external = &form.choice_controls[5];
    assert_eq!(external.form_owner.as_deref(), Some("prefs"));
    assert_eq!(external.group_name.as_deref(), Some("plan"));
    assert_eq!(external.group_checked_ids, vec!["plan-basic"]);
    assert_eq!(external.group_checked_values, vec!["basic"]);
}

#[test]
fn browser_form_file_descriptor_metadata_tracks_upload_controls_and_hints() {
    let document = parse_browser_document(
        "<form id=uploads enctype=multipart/form-data>\
         <label for=avatar>Avatar</label>\
         <input id=avatar type=file name=avatar accept=\"image/png, image/jpeg, .webp\" capture=user multiple required>\
         <label>Attachment<input id=attachment type=file name=attachment disabled></label>\
         </form>\
         <input id=external type=file form=uploads name=outside accept=\"application/pdf\">",
    )
    .expect("form file descriptor fixture should parse");

    let form = document
        .forms
        .first()
        .expect("uploads form should be summarized");
    let ids: Vec<&str> = form
        .file_controls
        .iter()
        .filter_map(|file| file.id.as_deref())
        .collect();
    assert_eq!(ids, vec!["avatar", "attachment", "external"]);

    let avatar = &form.file_controls[0];
    assert_eq!(avatar.name.as_deref(), Some("avatar"));
    assert_eq!(avatar.labels, vec!["Avatar"]);
    assert_eq!(avatar.accessible_name.as_deref(), Some("Avatar"));
    assert_eq!(
        avatar.accept.as_deref(),
        Some("image/png, image/jpeg, .webp")
    );
    assert_eq!(
        avatar.accept_tokens,
        vec!["image/png", "image/jpeg", ".webp"]
    );
    assert_eq!(avatar.capture.as_deref(), Some("user"));
    assert!(avatar.multiple);
    assert!(avatar.required);
    assert!(avatar.successful);
    assert!(avatar.submission_values.is_empty());
    assert!(avatar.will_validate);
    assert_eq!(avatar.validation_attributes, vec!["required"]);

    let attachment = &form.file_controls[1];
    assert_eq!(attachment.labels, vec!["Attachment"]);
    assert!(attachment.disabled);
    assert!(!attachment.successful);
    assert!(!attachment.will_validate);
    assert_eq!(
        attachment.validation_barred_reason.as_deref(),
        Some("disabled")
    );

    let external = &form.file_controls[2];
    assert_eq!(external.form_owner.as_deref(), Some("uploads"));
    assert_eq!(external.accept.as_deref(), Some("application/pdf"));
    assert_eq!(external.accept_tokens, vec!["application/pdf"]);
    assert!(external.successful);
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
fn browser_form_select_descriptor_metadata_tracks_option_items_and_selection_state() {
    let document = parse_browser_document(
        "<form id=survey><label for=topic>Topic</label>\
         <select id=topic name=topic required multiple size=4>\
         <optgroup label=Primary><option value=rust selected>Rust<option value=html label=HTML>Markup</optgroup>\
         <optgroup label=Archived disabled><option value=mosaic>Mosaic</optgroup>\
         </select></form>\
         <select id=external form=survey name=external><option>One<option selected>Two</select>",
    )
    .expect("form select descriptor fixture should parse");

    let form = document
        .forms
        .first()
        .expect("survey form should be summarized");
    assert_eq!(form.selects.len(), 2);

    let topic = &form.selects[0];
    assert_eq!(topic.id.as_deref(), Some("topic"));
    assert_eq!(topic.name.as_deref(), Some("topic"));
    assert_eq!(topic.labels, vec!["Topic"]);
    assert_eq!(topic.accessible_name.as_deref(), Some("Topic"));
    assert!(topic.required);
    assert!(topic.multiple);
    assert_eq!(topic.size.as_deref(), Some("4"));
    assert_eq!(topic.value.as_deref(), Some("rust"));
    assert_eq!(topic.selected_options, vec!["rust"]);
    assert_eq!(topic.text, "Rust Markup Mosaic");
    assert_eq!(topic.options.len(), 3);
    assert_eq!(topic.options[0].value, "rust");
    assert_eq!(topic.options[0].group_label.as_deref(), Some("Primary"));
    assert!(topic.options[0].selected);
    assert_eq!(topic.options[1].label.as_deref(), Some("HTML"));
    assert_eq!(topic.options[1].text, "Markup");
    assert_eq!(topic.options[2].value, "mosaic");
    assert_eq!(topic.options[2].group_label.as_deref(), Some("Archived"));
    assert!(topic.options[2].disabled);

    let external = &form.selects[1];
    assert_eq!(external.id.as_deref(), Some("external"));
    assert_eq!(external.form_owner.as_deref(), Some("survey"));
    assert_eq!(external.selected_options, vec!["Two"]);
    assert_eq!(external.options[1].value, "Two");
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
fn browser_media_descriptor_metadata_tracks_sources_and_tracks() {
    let suite: BrowserReadinessSuite = serde_json::from_str(BROWSER_READINESS_FIXTURE)
        .expect("browser readiness fixture should parse");
    let case = suite
        .cases
        .into_iter()
        .find(|case| case.id == "media-source-track-page")
        .expect("media source and track fixture case should exist");

    let actual = parse_browser_document(&case.input)
        .expect("media source and track fixture should parse into browser document facts");
    let expected = case.expected.into_browser_document();

    assert_eq!(
        actual.resources, expected.resources,
        "media child resources should preserve source candidates and track metadata",
    );
    assert_eq!(
        actual.media, expected.media,
        "media summaries should preserve nested source and track descriptors",
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
fn browser_focus_navigation_descriptors_track_focus_order_and_blockers() {
    let suite: BrowserReadinessSuite = serde_json::from_str(BROWSER_READINESS_FIXTURE)
        .expect("browser readiness fixture should parse");
    let case = suite
        .cases
        .into_iter()
        .find(|case| case.id == "interactive-element-state-page")
        .expect("interactive focus-navigation fixture case should exist");

    let actual = parse_browser_document(&case.input)
        .expect("interactive focus-navigation fixture should parse into browser document facts");

    assert_eq!(
        actual.focus_navigation_descriptors,
        case.expected.into_browser_document().focus_navigation_descriptors,
        "focus-navigation descriptors should preserve focus order, programmatic focus, editing hosts, access keys, and blocked focus reasons",
    );
}

#[test]
fn browser_global_state_descriptor_metadata_tracks_non_form_global_states() {
    let suite: BrowserReadinessSuite = serde_json::from_str(BROWSER_READINESS_FIXTURE)
        .expect("browser readiness fixture should parse");
    let case = suite
        .cases
        .into_iter()
        .find(|case| case.id == "interactive-element-state-page")
        .expect("interactive global-state fixture case should exist");

    let actual = parse_browser_document(&case.input)
        .expect("interactive global-state fixture should parse into browser document facts");

    assert_eq!(
        actual.global_state_descriptors,
        case.expected.into_browser_document().global_state_descriptors,
        "global-state descriptors should preserve inert, hidden, focus, editing, drag, spellcheck, translate, and accesskey metadata outside form-specific summaries",
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
fn browser_disclosure_descriptor_metadata_tracks_details_and_dialogs() {
    let suite: BrowserReadinessSuite = serde_json::from_str(BROWSER_READINESS_FIXTURE)
        .expect("browser readiness fixture should parse");
    let case = suite
        .cases
        .into_iter()
        .find(|case| case.id == "interactive-element-state-page")
        .expect("interactive fixture case should exist");

    let actual = parse_browser_document(&case.input)
        .expect("interactive fixture should parse into browser document facts");

    assert_eq!(
        actual.disclosures,
        case.expected.into_browser_document().disclosures,
        "disclosure summaries should preserve details summary text and dialog naming metadata",
    );
}

#[test]
fn browser_disclosure_state_descriptor_metadata_tracks_details_and_dialogs() {
    let suite: BrowserReadinessSuite = serde_json::from_str(BROWSER_READINESS_FIXTURE)
        .expect("browser readiness fixture should parse");
    let case = suite
        .cases
        .into_iter()
        .find(|case| case.id == "interactive-element-state-page")
        .expect("interactive disclosure-state fixture case should exist");

    let actual = parse_browser_document(&case.input)
        .expect("interactive disclosure-state fixture should parse into browser document facts");

    assert_eq!(
        actual.disclosure_state_descriptors,
        case.expected.into_browser_document().disclosure_state_descriptors,
        "disclosure-state descriptors should preserve details/dialog open, summary, modal, naming, and grouping metadata",
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

#[test]
fn browser_data_attribute_descriptor_metadata_tracks_custom_and_standard_elements() {
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
        actual.data_attribute_descriptors,
        case.expected
            .into_browser_document()
            .data_attribute_descriptors,
        "data attribute descriptors should preserve custom element, slot, part, and visible-text context for all data-* carriers",
    );
}

#[test]
fn browser_event_handler_descriptors_track_document_and_element_wiring() {
    let suite: BrowserReadinessSuite = serde_json::from_str(BROWSER_READINESS_FIXTURE)
        .expect("browser readiness fixture should parse");
    let case = suite
        .cases
        .into_iter()
        .find(|case| case.id == "event-handler-page")
        .expect("event handler fixture case should exist");

    let actual = parse_browser_document(&case.input)
        .expect("event handler fixture should parse into browser document facts");

    assert_eq!(
        actual.event_handler_descriptors,
        case.expected
            .into_browser_document()
            .event_handler_descriptors,
        "event handler descriptors should preserve document/body handlers and categorize inline element handlers without evaluating script text",
    );
}

impl ExpectedBrowserDocument {
    fn into_browser_document(self) -> BrowserDocument {
        let metadata = self.metadata.into_browser_document_metadata();
        let resources: Vec<_> = self
            .resources
            .into_iter()
            .map(ExpectedResource::into_browser_resource)
            .collect();
        let scripts: Vec<_> = self
            .scripts
            .into_iter()
            .map(ExpectedScript::into_browser_script)
            .collect();
        let images: Vec<_> = self
            .images
            .into_iter()
            .map(ExpectedImage::into_browser_image)
            .collect();
        let stylesheets: Vec<_> = self
            .stylesheets
            .into_iter()
            .map(ExpectedStylesheet::into_browser_stylesheet)
            .collect();
        let media: Vec<_> = self
            .media
            .into_iter()
            .map(ExpectedMedia::into_browser_media)
            .collect();
        let embedded_contexts: Vec<_> = self
            .embedded_contexts
            .into_iter()
            .map(ExpectedEmbeddedContext::into_browser_embedded_context)
            .collect();
        let resource_endpoint_descriptors = self
            .resource_endpoint_descriptors
            .map(|descriptors| {
                descriptors
                    .into_iter()
                    .map(
                        ExpectedResourceEndpointDescriptor::into_browser_resource_endpoint_descriptor,
                    )
                    .collect()
            })
            .unwrap_or_else(|| expected_resource_endpoint_descriptors(&metadata, &resources));
        let media_playback_descriptors = self
            .media_playback_descriptors
            .map(|descriptors| {
                descriptors
                    .into_iter()
                    .map(ExpectedMediaPlaybackDescriptor::into_browser_media_playback_descriptor)
                    .collect()
            })
            .unwrap_or_else(|| expected_media_playback_descriptors(&media));
        let embedded_policy_descriptors = self
            .embedded_policy_descriptors
            .map(|descriptors| {
                descriptors
                    .into_iter()
                    .map(ExpectedEmbeddedPolicyDescriptor::into_browser_embedded_policy_descriptor)
                    .collect()
            })
            .unwrap_or_else(|| expected_embedded_policy_descriptors(&embedded_contexts));
        let script_execution_descriptors = self
            .script_execution_descriptors
            .map(|descriptors| {
                descriptors
                    .into_iter()
                    .map(
                        ExpectedScriptExecutionDescriptor::into_browser_script_execution_descriptor,
                    )
                    .collect()
            })
            .unwrap_or_else(|| expected_script_execution_descriptors(&scripts));
        let stylesheet_planning_descriptors = self
            .stylesheet_planning_descriptors
            .map(|descriptors| {
                descriptors
                    .into_iter()
                    .map(
                        ExpectedStylesheetPlanningDescriptor::into_browser_stylesheet_planning_descriptor,
                    )
                    .collect()
            })
            .unwrap_or_else(|| expected_stylesheet_planning_descriptors(&stylesheets));
        let image_candidate_descriptors = self
            .image_candidate_descriptors
            .map(|descriptors| {
                descriptors
                    .into_iter()
                    .map(ExpectedImageCandidateDescriptor::into_browser_image_candidate_descriptor)
                    .collect()
            })
            .unwrap_or_else(|| expected_image_candidate_descriptors(&images));
        let disclosures: Vec<_> = self
            .disclosures
            .into_iter()
            .map(ExpectedDisclosure::into_browser_disclosure)
            .collect();
        let command_elements: Vec<_> = self
            .command_elements
            .into_iter()
            .map(ExpectedCommandElement::into_browser_command_element)
            .collect();
        let popovers: Vec<_> = self
            .popovers
            .into_iter()
            .map(ExpectedPopover::into_browser_popover)
            .collect();
        let interactive_elements: Vec<_> = self
            .interactive_elements
            .into_iter()
            .map(ExpectedInteractiveElement::into_browser_interactive_element)
            .collect();
        let disclosure_state_descriptors = self
            .disclosure_state_descriptors
            .map(|descriptors| {
                descriptors
                    .into_iter()
                    .map(
                        ExpectedDisclosureStateDescriptor::into_browser_disclosure_state_descriptor,
                    )
                    .collect()
            })
            .unwrap_or_else(|| expected_disclosure_state_descriptors(&disclosures));
        let activation_descriptors = self
            .activation_descriptors
            .map(|descriptors| {
                descriptors
                    .into_iter()
                    .map(ExpectedActivationDescriptor::into_browser_activation_descriptor)
                    .collect()
            })
            .unwrap_or_else(|| {
                expected_activation_descriptors(&command_elements, &popovers, &disclosures)
            });
        let focus_navigation_descriptors = self
            .focus_navigation_descriptors
            .map(|descriptors| {
                descriptors
                    .into_iter()
                    .map(
                        ExpectedFocusNavigationDescriptor::into_browser_focus_navigation_descriptor,
                    )
                    .collect()
            })
            .unwrap_or_else(|| expected_focus_navigation_descriptors(&interactive_elements));

        BrowserDocument {
            title: self.title,
            base_href: self.base_href,
            base_target: self.base_target,
            metadata,
            document_lang: self.document_lang,
            document_dir: self.document_dir,
            body_id: self.body_id,
            body_classes: self.body_classes,
            body_lang: self.body_lang,
            body_dir: self.body_dir,
            document_event_handlers: self.document_event_handlers,
            body_event_handlers: self.body_event_handlers,
            event_handler_descriptors: self
                .event_handler_descriptors
                .into_iter()
                .map(ExpectedEventHandlerDescriptor::into_browser_event_handler_descriptor)
                .collect(),
            body_text: self.body_text,
            metas: self
                .metas
                .into_iter()
                .map(ExpectedMeta::into_browser_meta)
                .collect(),
            resources,
            scripts,
            script_execution_descriptors,
            stylesheets,
            stylesheet_planning_descriptors,
            document_policy_descriptors: self
                .document_policy_descriptors
                .into_iter()
                .map(ExpectedDocumentPolicyDescriptor::into_browser_document_policy_descriptor)
                .collect(),
            loading_hint_descriptors: self
                .loading_hint_descriptors
                .into_iter()
                .map(ExpectedLoadingHintDescriptor::into_browser_loading_hint_descriptor)
                .collect(),
            fetch_policy_descriptors: self
                .fetch_policy_descriptors
                .into_iter()
                .map(ExpectedFetchPolicyDescriptor::into_browser_fetch_policy_descriptor)
                .collect(),
            resource_endpoint_descriptors,
            form_policy_descriptors: self
                .form_policy_descriptors
                .into_iter()
                .map(ExpectedFormPolicyDescriptor::into_browser_form_policy_descriptor)
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
            text_semantics: self
                .text_semantics
                .into_iter()
                .map(ExpectedTextSemantic::into_browser_text_semantic)
                .collect(),
            navigation_target_descriptors: self
                .navigation_target_descriptors
                .into_iter()
                .map(ExpectedNavigationTargetDescriptor::into_browser_navigation_target_descriptor)
                .collect(),
            navigation_groups: self
                .navigation_groups
                .into_iter()
                .map(ExpectedNavigationGroup::into_browser_navigation_group)
                .collect(),
            section_landmarks: self
                .section_landmarks
                .into_iter()
                .map(ExpectedSectionLandmark::into_browser_section_landmark)
                .collect(),
            command_elements,
            activation_descriptors,
            popovers,
            aria_collections: self
                .aria_collections
                .into_iter()
                .map(ExpectedAriaCollection::into_browser_aria_collection)
                .collect(),
            aria_ranges: self
                .aria_ranges
                .into_iter()
                .map(ExpectedAriaRange::into_browser_aria_range)
                .collect(),
            aria_live_regions: self
                .aria_live_regions
                .into_iter()
                .map(ExpectedAriaLiveRegion::into_browser_aria_live_region)
                .collect(),
            aria_relation_descriptors: self
                .aria_relation_descriptors
                .into_iter()
                .map(ExpectedAriaRelationDescriptor::into_browser_aria_relation_descriptor)
                .collect(),
            links: self
                .links
                .into_iter()
                .map(ExpectedLink::into_browser_link)
                .collect(),
            images,
            image_candidate_descriptors,
            image_maps: self
                .image_maps
                .into_iter()
                .map(ExpectedImageMap::into_browser_image_map)
                .collect(),
            media,
            media_playback_descriptors,
            embedded_contexts,
            embedded_policy_descriptors,
            interactive_elements,
            focus_navigation_descriptors,
            disclosures,
            disclosure_state_descriptors,
            component_hydration_targets: self
                .component_hydration_targets
                .into_iter()
                .map(ExpectedComponentHydrationTarget::into_browser_component_hydration_target)
                .collect(),
            data_attribute_descriptors: self
                .data_attribute_descriptors
                .into_iter()
                .map(ExpectedDataAttributeDescriptor::into_browser_data_attribute_descriptor)
                .collect(),
            global_state_descriptors: self
                .global_state_descriptors
                .into_iter()
                .map(ExpectedGlobalStateDescriptor::into_browser_global_state_descriptor)
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
            table_cells: self
                .table_cells
                .into_iter()
                .map(ExpectedTableCell::into_browser_table_cell)
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

impl ExpectedDataAttributeDescriptor {
    fn into_browser_data_attribute_descriptor(self) -> BrowserDataAttributeDescriptor {
        BrowserDataAttributeDescriptor {
            element: self.element,
            id: self.id,
            classes: self.classes,
            custom_element: self.custom_element,
            custom_element_name: self.custom_element_name,
            custom_element_is: self.custom_element_is,
            slot: self.slot,
            slot_name: self.slot_name,
            part: self.part,
            data_attributes: self
                .data_attributes
                .into_iter()
                .map(ExpectedDataAttribute::into_browser_data_attribute)
                .collect(),
            text: self.text,
        }
    }
}

impl ExpectedGlobalStateDescriptor {
    fn into_browser_global_state_descriptor(self) -> BrowserGlobalStateDescriptor {
        BrowserGlobalStateDescriptor {
            element: self.element,
            id: self.id,
            classes: self.classes,
            title: self.title,
            lang: self.lang,
            dir: self.dir,
            hidden: self.hidden,
            inert: self.inert,
            tabindex: self.tabindex,
            accesskey: self.accesskey,
            autofocus: self.autofocus,
            contenteditable: self.contenteditable,
            editing_mode: self.editing_mode,
            draggable: self.draggable,
            draggable_state: self.draggable_state,
            spellcheck: self.spellcheck,
            translate: self.translate,
            text: self.text,
        }
    }
}

impl ExpectedEventHandlerDescriptor {
    fn into_browser_event_handler_descriptor(self) -> BrowserEventHandlerDescriptor {
        BrowserEventHandlerDescriptor {
            element: self.element,
            id: self.id,
            classes: self.classes,
            role: self.role,
            source: self.source,
            event_handlers: self.event_handlers,
            handler_count: self.handler_count,
            activation_handlers: self.activation_handlers,
            keyboard_handlers: self.keyboard_handlers,
            pointer_handlers: self.pointer_handlers,
            form_handlers: self.form_handlers,
            media_handlers: self.media_handlers,
            lifecycle_handlers: self.lifecycle_handlers,
            error_handlers: self.error_handlers,
            text: self.text,
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

fn expected_resource_endpoint_descriptors(
    metadata: &BrowserDocumentMetadata,
    resources: &[BrowserResource],
) -> Vec<BrowserResourceEndpointDescriptor> {
    let mut descriptors = Vec::new();
    if let Some(refresh) = &metadata.refresh {
        if let Some(url) = &refresh.url {
            descriptors.push(BrowserResourceEndpointDescriptor {
                endpoint_kind: "metadata-refresh".to_string(),
                element: "meta".to_string(),
                resource_kind: Some("refresh".to_string()),
                url: url.clone(),
                resolved_url: refresh.resolved_url.clone(),
                rel: None,
                rel_tokens: Vec::new(),
                as_hint: None,
                type_hint: None,
                media: None,
                title: None,
                sizes: None,
                hreflang: None,
                color: None,
                width: None,
                height: None,
                integrity: None,
                crossorigin: None,
                nonce: None,
                referrerpolicy: None,
                fetchpriority: None,
                csp: None,
                blocking: None,
                blocking_tokens: Vec::new(),
                browsing_context_name: None,
                loading: None,
                sandbox: Vec::new(),
                allow: None,
                allowfullscreen: false,
                srcdoc: None,
                credentialless: false,
                imagesrcset: None,
                resolved_imagesrcset: None,
                imagesizes: None,
                track_kind: None,
                srclang: None,
                track_label: None,
                default_track: false,
                async_script: false,
                defer_script: false,
            });
        }
    }
    descriptors.extend(resources.iter().map(expected_resource_endpoint_descriptor));
    descriptors
}

fn expected_resource_endpoint_descriptor(
    resource: &BrowserResource,
) -> BrowserResourceEndpointDescriptor {
    BrowserResourceEndpointDescriptor {
        endpoint_kind: "resource".to_string(),
        element: expected_resource_endpoint_element(&resource.kind).to_string(),
        resource_kind: Some(resource.kind.clone()),
        url: resource.url.clone(),
        resolved_url: resource.resolved_url.clone(),
        rel: resource.rel.clone(),
        rel_tokens: resource.rel_tokens.clone(),
        as_hint: resource.as_hint.clone(),
        type_hint: resource.type_hint.clone(),
        media: resource.media.clone(),
        title: resource.title.clone(),
        sizes: resource.sizes.clone(),
        hreflang: resource.hreflang.clone(),
        color: resource.color.clone(),
        width: resource.width.clone(),
        height: resource.height.clone(),
        integrity: resource.integrity.clone(),
        crossorigin: resource.crossorigin.clone(),
        nonce: resource.nonce.clone(),
        referrerpolicy: resource.referrerpolicy.clone(),
        fetchpriority: resource.fetchpriority.clone(),
        csp: resource.csp.clone(),
        blocking: resource.blocking.clone(),
        blocking_tokens: resource.blocking_tokens.clone(),
        browsing_context_name: resource.browsing_context_name.clone(),
        loading: resource.loading.clone(),
        sandbox: resource.sandbox.clone(),
        allow: resource.allow.clone(),
        allowfullscreen: resource.allowfullscreen,
        srcdoc: resource.srcdoc.clone(),
        credentialless: resource.credentialless,
        imagesrcset: resource.imagesrcset.clone(),
        resolved_imagesrcset: resource.resolved_imagesrcset.clone(),
        imagesizes: resource.imagesizes.clone(),
        track_kind: resource.track_kind.clone(),
        srclang: resource.srclang.clone(),
        track_label: resource.track_label.clone(),
        default_track: resource.default_track,
        async_script: resource.async_script,
        defer_script: resource.defer_script,
    }
}

fn expected_resource_endpoint_element(kind: &str) -> &str {
    match kind {
        "alternate" | "canonical" | "dns-prefetch" | "icon" | "link" | "manifest"
        | "modulepreload" | "preconnect" | "prefetch" | "preload" | "prerender" | "stylesheet" => {
            "link"
        }
        "frame" => "iframe",
        "image" => "img",
        other => other,
    }
}

fn expected_media_playback_descriptors(
    media: &[BrowserMedia],
) -> Vec<BrowserMediaPlaybackDescriptor> {
    media
        .iter()
        .map(expected_media_playback_descriptor)
        .collect()
}

fn expected_media_playback_descriptor(media: &BrowserMedia) -> BrowserMediaPlaybackDescriptor {
    BrowserMediaPlaybackDescriptor {
        kind: media.kind.clone(),
        src: media.src.clone(),
        resolved_src: media.resolved_src.clone(),
        poster: media.poster.clone(),
        resolved_poster: media.resolved_poster.clone(),
        width: media.width.clone(),
        height: media.height.clone(),
        controls: media.controls,
        autoplay: media.autoplay,
        loop_media: media.loop_media,
        muted: media.muted,
        playsinline: media.playsinline,
        preload: media.preload.clone(),
        crossorigin: media.crossorigin.clone(),
        controlslist: media.controlslist.clone(),
        controlslist_tokens: media.controlslist_tokens.clone(),
        disableremoteplayback: media.disableremoteplayback,
        disablepictureinpicture: media.disablepictureinpicture,
        source_count: media.sources.len(),
        sources: media.sources.clone(),
        track_count: media.tracks.len(),
        default_track_count: media
            .tracks
            .iter()
            .filter(|track| track.default_track)
            .count(),
        tracks: media.tracks.clone(),
    }
}

fn expected_image_candidate_descriptors(
    images: &[BrowserImage],
) -> Vec<BrowserImageCandidateDescriptor> {
    images
        .iter()
        .map(expected_image_candidate_descriptor)
        .collect()
}

fn expected_image_candidate_descriptor(image: &BrowserImage) -> BrowserImageCandidateDescriptor {
    let image_srcset_count = image
        .srcset
        .as_deref()
        .map(expected_srcset_candidate_count)
        .unwrap_or_default();
    let source_srcset_count = image
        .sources
        .iter()
        .filter(|source| source.srcset.is_some())
        .count();
    let source_candidate_count = image
        .sources
        .iter()
        .filter_map(|source| source.srcset.as_deref())
        .map(expected_srcset_candidate_count)
        .sum::<usize>();

    BrowserImageCandidateDescriptor {
        src: image.src.clone(),
        resolved_src: image.resolved_src.clone(),
        srcset: image.srcset.clone(),
        resolved_srcset: image.resolved_srcset.clone(),
        sizes: image.sizes.clone(),
        alt: image.alt.clone(),
        width: image.width.clone(),
        height: image.height.clone(),
        loading: image.loading.clone(),
        decoding: image.decoding.clone(),
        fetchpriority: image.fetchpriority.clone(),
        crossorigin: image.crossorigin.clone(),
        referrerpolicy: image.referrerpolicy.clone(),
        usemap: image.usemap.clone(),
        ismap: image.ismap,
        has_alt: image.alt.is_some(),
        source_count: image.sources.len(),
        source_srcset_count,
        candidate_count: image_srcset_count + source_candidate_count,
        source_type_hints: image
            .sources
            .iter()
            .filter_map(|source| source.type_hint.clone())
            .collect(),
        source_media: image
            .sources
            .iter()
            .filter_map(|source| source.media.clone())
            .collect(),
        sources: image.sources.clone(),
    }
}

fn expected_srcset_candidate_count(srcset: &str) -> usize {
    srcset
        .split(',')
        .map(str::trim)
        .filter(|candidate| !candidate.is_empty())
        .count()
}

fn expected_embedded_policy_descriptors(
    contexts: &[BrowserEmbeddedContext],
) -> Vec<BrowserEmbeddedPolicyDescriptor> {
    contexts
        .iter()
        .map(expected_embedded_policy_descriptor)
        .collect()
}

fn expected_embedded_policy_descriptor(
    context: &BrowserEmbeddedContext,
) -> BrowserEmbeddedPolicyDescriptor {
    BrowserEmbeddedPolicyDescriptor {
        element: context.element.clone(),
        resource_kind: expected_embedded_resource_kind(&context.element).to_string(),
        url: context.url.clone(),
        resolved_url: context.resolved_url.clone(),
        browsing_context_name: context.browsing_context_name.clone(),
        title: context.title.clone(),
        type_hint: context.type_hint.clone(),
        width: context.width.clone(),
        height: context.height.clone(),
        loading: context.loading.clone(),
        fetchpriority: context.fetchpriority.clone(),
        csp: context.csp.clone(),
        sandbox: context.sandbox.clone(),
        sandbox_token_count: context.sandbox.len(),
        allow: context.allow.clone(),
        allowfullscreen: context.allowfullscreen,
        referrerpolicy: context.referrerpolicy.clone(),
        srcdoc: context.srcdoc.clone(),
        has_srcdoc: context.srcdoc.is_some(),
        credentialless: context.credentialless,
        fallback_text: context.fallback_text.clone(),
    }
}

fn expected_embedded_resource_kind(element: &str) -> &'static str {
    match element {
        "iframe" | "frame" => "document",
        "object" => "object",
        "embed" => "embed",
        _ => "embedded",
    }
}

fn expected_disclosure_state_descriptors(
    disclosures: &[BrowserDisclosure],
) -> Vec<BrowserDisclosureStateDescriptor> {
    disclosures
        .iter()
        .map(expected_disclosure_state_descriptor)
        .collect()
}

fn expected_disclosure_state_descriptor(
    disclosure: &BrowserDisclosure,
) -> BrowserDisclosureStateDescriptor {
    let disclosure_kind = if disclosure.element == "dialog" {
        "dialog"
    } else {
        "details"
    };
    let grouped = disclosure.element == "details" && disclosure.name.is_some();

    BrowserDisclosureStateDescriptor {
        element: disclosure.element.clone(),
        id: disclosure.id.clone(),
        name: disclosure.name.clone(),
        disclosure_kind: disclosure_kind.to_string(),
        open: disclosure.open,
        grouped,
        group_name: grouped.then(|| disclosure.name.clone()).flatten(),
        has_summary: disclosure.summary_text.is_some(),
        summary_text: disclosure.summary_text.clone(),
        accessible_name: disclosure.accessible_name.clone(),
        accessible_description: disclosure.accessible_description.clone(),
        aria_label: disclosure.aria_label.clone(),
        aria_labelledby: disclosure.aria_labelledby.clone(),
        aria_describedby: disclosure.aria_describedby.clone(),
        aria_modal: disclosure.aria_modal.clone(),
        modal: disclosure
            .aria_modal
            .as_deref()
            .is_some_and(|value| value.eq_ignore_ascii_case("true")),
        closedby: disclosure.closedby.clone(),
        text: disclosure.text.clone(),
        text_length: disclosure.text.chars().count(),
    }
}

fn expected_activation_descriptors(
    commands: &[BrowserCommandElement],
    popovers: &[BrowserPopover],
    disclosures: &[BrowserDisclosure],
) -> Vec<BrowserActivationDescriptor> {
    commands
        .iter()
        .map(|command| expected_activation_descriptor(command, popovers, disclosures))
        .collect()
}

fn expected_activation_descriptor(
    command: &BrowserCommandElement,
    popovers: &[BrowserPopover],
    disclosures: &[BrowserDisclosure],
) -> BrowserActivationDescriptor {
    BrowserActivationDescriptor {
        element: command.element.clone(),
        id: command.id.clone(),
        role: command.role.clone(),
        authored_role: command.authored_role.clone(),
        command_kind: command.command_kind.clone(),
        activation_kind: expected_activation_kind(command),
        target_id: expected_activation_target_id(command),
        target_kind: expected_activation_target_kind(command, popovers, disclosures),
        text: command.text.clone(),
        accessible_name: command.accessible_name.clone(),
        accessible_description: command.accessible_description.clone(),
        disabled: command.disabled,
        focusable: command.focusable,
        tabindex: command.tabindex.clone(),
        accesskey: command.accesskey.clone(),
        event_handlers: command.event_handlers.clone(),
        handler_count: command.event_handlers.len(),
        command: command.command.clone(),
        command_for: command.command_for.clone(),
        popover_target: command.popover_target.clone(),
        popover_target_action: command.popover_target_action.clone(),
        aria_controls: command.aria_controls.clone(),
        aria_expanded: command.aria_expanded.clone(),
        aria_haspopup: command.aria_haspopup.clone(),
        aria_pressed: command.aria_pressed.clone(),
        aria_current: command.aria_current.clone(),
        aria_disabled: command.aria_disabled.clone(),
        control_type: command.control_type.clone(),
        href: command.href.clone(),
        resolved_href: command.resolved_href.clone(),
        effective_target: command.effective_target.clone(),
        form_owner: command.form_owner.clone(),
        form_action: command.form_action.clone(),
        resolved_form_action: command.resolved_form_action.clone(),
        form_method: command.form_method.clone(),
        form_target: command.form_target.clone(),
        form_novalidate: command.form_novalidate,
    }
}

fn expected_activation_kind(command: &BrowserCommandElement) -> String {
    if let Some(command_value) = &command.command {
        return command_value.clone();
    }
    if command.popover_target.is_some() {
        let action = command.popover_target_action.as_deref().unwrap_or("toggle");
        return format!("popover-{action}");
    }
    if command.href.is_some() {
        return "navigation".to_string();
    }
    if let Some(control_type) = &command.control_type {
        if matches!(
            control_type.as_str(),
            "submit" | "reset" | "button" | "image"
        ) {
            return format!("form-{control_type}");
        }
    }

    command.command_kind.clone()
}

fn expected_activation_target_id(command: &BrowserCommandElement) -> Option<String> {
    command
        .command_for
        .clone()
        .or_else(|| command.popover_target.clone())
        .or_else(|| command.aria_controls.first().cloned())
}

fn expected_activation_target_kind(
    command: &BrowserCommandElement,
    popovers: &[BrowserPopover],
    disclosures: &[BrowserDisclosure],
) -> String {
    if let Some(command_for) = &command.command_for {
        if let Some(disclosure) = disclosures
            .iter()
            .find(|disclosure| disclosure.id.as_deref() == Some(command_for.as_str()))
        {
            return if disclosure.element == "dialog" {
                "dialog"
            } else {
                "disclosure"
            }
            .to_string();
        }
        if popovers
            .iter()
            .any(|popover| popover.id.as_deref() == Some(command_for.as_str()))
        {
            return "popover".to_string();
        }
        return "command-target".to_string();
    }
    if let Some(popover_target) = &command.popover_target {
        if popovers
            .iter()
            .any(|popover| popover.id.as_deref() == Some(popover_target.as_str()))
        {
            return "popover".to_string();
        }
        return "popover-target".to_string();
    }
    if !command.aria_controls.is_empty() {
        return "controlled-region".to_string();
    }
    if command.href.is_some() {
        return "navigation".to_string();
    }
    if command.form_owner.is_some()
        || command.form_action.is_some()
        || matches!(
            command.control_type.as_deref(),
            Some("submit" | "reset" | "image")
        )
    {
        return "form".to_string();
    }
    if command.command_kind == "disclosure" {
        return "disclosure".to_string();
    }

    "command".to_string()
}

fn expected_focus_navigation_descriptors(
    elements: &[BrowserInteractiveElement],
) -> Vec<BrowserFocusNavigationDescriptor> {
    elements
        .iter()
        .filter(|element| expected_has_focus_navigation_state(element))
        .map(expected_focus_navigation_descriptor)
        .collect()
}

fn expected_has_focus_navigation_state(element: &BrowserInteractiveElement) -> bool {
    element.focusable.is_some()
        || element.tabindex.is_some()
        || !element.accesskey.is_empty()
        || element.contenteditable.is_some()
        || element.disabled
        || element.hidden
        || element.inert
        || element.aria_hidden
        || element.aria_disabled.is_some()
}

fn expected_focus_navigation_descriptor(
    element: &BrowserInteractiveElement,
) -> BrowserFocusNavigationDescriptor {
    let focusable = element.focusable.unwrap_or(false);
    let tabindex_order = element
        .tabindex
        .as_deref()
        .and_then(|tabindex| tabindex.trim().parse::<i32>().ok());
    let focus_block_reasons = expected_focus_block_reasons(element);
    let focus_blocked = !focus_block_reasons.is_empty();
    let sequential_focus = focusable && tabindex_order.unwrap_or(0) >= 0;
    let programmatic_focus = focusable || matches!(tabindex_order, Some(value) if value < 0);

    BrowserFocusNavigationDescriptor {
        element: element.element.clone(),
        id: element.id.clone(),
        role: element.role.clone(),
        authored_role: element.authored_role.clone(),
        focus_kind: expected_focus_kind(element, focusable, sequential_focus, programmatic_focus),
        focusable,
        sequential_focus,
        programmatic_focus,
        focus_blocked,
        focus_block_reasons,
        tabindex: element.tabindex.clone(),
        tabindex_order,
        accesskey: element.accesskey.clone(),
        event_handlers: element.event_handlers.clone(),
        contenteditable: element.contenteditable.clone(),
        editing_mode: element.editing_mode.clone(),
        command: element.command.clone(),
        command_for: element.command_for.clone(),
        popover_target: element.popover_target.clone(),
        popover_target_action: element.popover_target_action.clone(),
        aria_controls: element.aria_controls.clone(),
        aria_activedescendant: element.aria_activedescendant.clone(),
        aria_expanded: element.aria_expanded.clone(),
        aria_haspopup: element.aria_haspopup.clone(),
        aria_disabled: element.aria_disabled.clone(),
        disabled: element.disabled,
        hidden: element.hidden,
        inert: element.inert,
        aria_hidden: element.aria_hidden,
        text: element.text.clone(),
    }
}

fn expected_focus_kind(
    element: &BrowserInteractiveElement,
    focusable: bool,
    sequential_focus: bool,
    programmatic_focus: bool,
) -> String {
    if !expected_focus_block_reasons(element).is_empty() {
        return "blocked".to_string();
    }
    if element.editing_mode.is_some() {
        return "editing-host".to_string();
    }
    if sequential_focus {
        return "sequential".to_string();
    }
    if programmatic_focus {
        return "programmatic".to_string();
    }
    if !element.accesskey.is_empty() {
        return "accesskey".to_string();
    }
    if focusable {
        return "focusable".to_string();
    }

    "metadata".to_string()
}

fn expected_focus_block_reasons(element: &BrowserInteractiveElement) -> Vec<String> {
    let mut reasons = Vec::new();
    if element.disabled {
        reasons.push("disabled".to_string());
    }
    if element.hidden {
        reasons.push("hidden".to_string());
    }
    if element.inert {
        reasons.push("inert".to_string());
    }
    if element.aria_hidden {
        reasons.push("aria-hidden".to_string());
    }
    if element.aria_disabled.as_deref() == Some("true") {
        reasons.push("aria-disabled".to_string());
    }
    reasons
}

fn expected_script_execution_descriptors(
    scripts: &[BrowserScript],
) -> Vec<BrowserScriptExecutionDescriptor> {
    scripts
        .iter()
        .map(expected_script_execution_descriptor)
        .collect()
}

fn expected_script_execution_descriptor(
    script: &BrowserScript,
) -> BrowserScriptExecutionDescriptor {
    let text_length = script
        .text
        .as_ref()
        .map(|text| text.chars().count())
        .unwrap_or_default();
    BrowserScriptExecutionDescriptor {
        script_kind: script.script_kind.clone(),
        execution_kind: if script.src.is_some() {
            "external".to_string()
        } else {
            "inline".to_string()
        },
        src: script.src.clone(),
        resolved_src: script.resolved_src.clone(),
        type_hint: script.type_hint.clone(),
        async_script: script.async_script,
        defer_script: script.defer_script,
        nomodule: script.nomodule,
        integrity: script.integrity.clone(),
        crossorigin: script.crossorigin.clone(),
        nonce: script.nonce.clone(),
        referrerpolicy: script.referrerpolicy.clone(),
        fetchpriority: script.fetchpriority.clone(),
        blocking: script.blocking.clone(),
        blocking_tokens: script.blocking_tokens.clone(),
        blocking_token_count: script.blocking_tokens.len(),
        render_blocking: script
            .blocking_tokens
            .iter()
            .any(|token| token.eq_ignore_ascii_case("render")),
        has_text: script.text.is_some(),
        text_length,
    }
}

fn expected_stylesheet_planning_descriptors(
    stylesheets: &[BrowserStylesheet],
) -> Vec<BrowserStylesheetPlanningDescriptor> {
    stylesheets
        .iter()
        .map(expected_stylesheet_planning_descriptor)
        .collect()
}

fn expected_stylesheet_planning_descriptor(
    stylesheet: &BrowserStylesheet,
) -> BrowserStylesheetPlanningDescriptor {
    let text_length = stylesheet
        .text
        .as_ref()
        .map(|text| text.chars().count())
        .unwrap_or_default();
    BrowserStylesheetPlanningDescriptor {
        source: stylesheet.source.clone(),
        stylesheet_kind: if stylesheet.href.is_some() {
            "external".to_string()
        } else {
            "inline".to_string()
        },
        href: stylesheet.href.clone(),
        resolved_href: stylesheet.resolved_href.clone(),
        rel: stylesheet.rel.clone(),
        rel_tokens: stylesheet.rel_tokens.clone(),
        rel_token_count: stylesheet.rel_tokens.len(),
        type_hint: stylesheet.type_hint.clone(),
        media: stylesheet.media.clone(),
        title: stylesheet.title.clone(),
        disabled: stylesheet.disabled,
        alternate: stylesheet.alternate,
        applies_by_default: !stylesheet.disabled && !stylesheet.alternate,
        integrity: stylesheet.integrity.clone(),
        crossorigin: stylesheet.crossorigin.clone(),
        nonce: stylesheet.nonce.clone(),
        referrerpolicy: stylesheet.referrerpolicy.clone(),
        fetchpriority: stylesheet.fetchpriority.clone(),
        blocking: stylesheet.blocking.clone(),
        blocking_tokens: stylesheet.blocking_tokens.clone(),
        blocking_token_count: stylesheet.blocking_tokens.len(),
        render_blocking: stylesheet
            .blocking_tokens
            .iter()
            .any(|token| token.eq_ignore_ascii_case("render")),
        has_text: stylesheet.text.is_some(),
        text_length,
    }
}

impl ExpectedResourceEndpointDescriptor {
    fn into_browser_resource_endpoint_descriptor(self) -> BrowserResourceEndpointDescriptor {
        let rel_tokens = expected_tokens_from_raw(self.rel_tokens, self.rel.as_deref());
        let blocking_tokens =
            expected_tokens_from_raw(self.blocking_tokens, self.blocking.as_deref());
        BrowserResourceEndpointDescriptor {
            endpoint_kind: self.endpoint_kind,
            element: self.element,
            resource_kind: self.resource_kind,
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

impl ExpectedScriptExecutionDescriptor {
    fn into_browser_script_execution_descriptor(self) -> BrowserScriptExecutionDescriptor {
        let blocking_tokens =
            expected_tokens_from_raw(self.blocking_tokens, self.blocking.as_deref());
        BrowserScriptExecutionDescriptor {
            script_kind: self.script_kind,
            execution_kind: self.execution_kind,
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
            blocking_token_count: self.blocking_token_count,
            render_blocking: self.render_blocking,
            has_text: self.has_text,
            text_length: self.text_length,
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

impl ExpectedStylesheetPlanningDescriptor {
    fn into_browser_stylesheet_planning_descriptor(self) -> BrowserStylesheetPlanningDescriptor {
        let rel_tokens = expected_tokens_from_raw(self.rel_tokens, self.rel.as_deref());
        let blocking_tokens =
            expected_tokens_from_raw(self.blocking_tokens, self.blocking.as_deref());
        BrowserStylesheetPlanningDescriptor {
            source: self.source,
            stylesheet_kind: self.stylesheet_kind,
            href: self.href,
            resolved_href: self.resolved_href,
            rel: self.rel,
            rel_tokens,
            rel_token_count: self.rel_token_count,
            type_hint: self.type_hint,
            media: self.media,
            title: self.title,
            disabled: self.disabled,
            alternate: self.alternate,
            applies_by_default: self.applies_by_default,
            integrity: self.integrity,
            crossorigin: self.crossorigin,
            nonce: self.nonce,
            referrerpolicy: self.referrerpolicy,
            fetchpriority: self.fetchpriority,
            blocking: self.blocking,
            blocking_tokens,
            blocking_token_count: self.blocking_token_count,
            render_blocking: self.render_blocking,
            has_text: self.has_text,
            text_length: self.text_length,
        }
    }
}

impl ExpectedLoadingHintDescriptor {
    fn into_browser_loading_hint_descriptor(self) -> BrowserLoadingHintDescriptor {
        let blocking_tokens =
            expected_tokens_from_raw(self.blocking_tokens, self.blocking.as_deref());
        BrowserLoadingHintDescriptor {
            element: self.element,
            id: self.id,
            url: self.url,
            resolved_url: self.resolved_url,
            loading: self.loading,
            decoding: self.decoding,
            fetchpriority: self.fetchpriority,
            blocking: self.blocking,
            blocking_tokens,
            preload: self.preload,
            as_hint: self.as_hint,
            media: self.media,
        }
    }
}

impl ExpectedFetchPolicyDescriptor {
    fn into_browser_fetch_policy_descriptor(self) -> BrowserFetchPolicyDescriptor {
        BrowserFetchPolicyDescriptor {
            element: self.element,
            id: self.id,
            url: self.url,
            resolved_url: self.resolved_url,
            integrity: self.integrity,
            crossorigin: self.crossorigin,
            nonce: self.nonce,
            referrerpolicy: self.referrerpolicy,
            csp: self.csp,
            sandbox: self.sandbox,
            allow: self.allow,
            allowfullscreen: self.allowfullscreen,
            credentialless: self.credentialless,
        }
    }
}

impl ExpectedDocumentPolicyDescriptor {
    fn into_browser_document_policy_descriptor(self) -> BrowserDocumentPolicyDescriptor {
        BrowserDocumentPolicyDescriptor {
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
            content_security_policy: self.content_security_policy,
            permissions_policy: self.permissions_policy,
            origin_trials: self.origin_trials,
            accept_ch: self.accept_ch,
            accept_ch_tokens: self.accept_ch_tokens,
            dns_prefetch_control: self.dns_prefetch_control,
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

impl ExpectedNavigationTargetDescriptor {
    fn into_browser_navigation_target_descriptor(self) -> BrowserNavigationTargetDescriptor {
        BrowserNavigationTargetDescriptor {
            element: self.element,
            id: self.id,
            href: self.href,
            resolved_href: self.resolved_href,
            text: self.text,
            target: self.target,
            effective_target: self.effective_target,
            rel: self.rel,
            rel_tokens: self.rel_tokens,
            rel_external: self.rel_external,
            rel_nofollow: self.rel_nofollow,
            rel_noopener: self.rel_noopener,
            rel_noreferrer: self.rel_noreferrer,
            download: self.download,
            ping: self.ping,
            resolved_ping: self.resolved_ping,
            attributionsrc: self.attributionsrc,
            resolved_attributionsrc: self.resolved_attributionsrc,
            hreflang: self.hreflang,
            type_hint: self.type_hint,
            referrerpolicy: self.referrerpolicy,
            area_shape: self.area_shape,
            area_coords: self.area_coords,
        }
    }
}

impl ExpectedFormPolicyDescriptor {
    fn into_browser_form_policy_descriptor(self) -> BrowserFormPolicyDescriptor {
        BrowserFormPolicyDescriptor {
            id: self.id,
            name: self.name,
            action: self.action,
            resolved_action: self.resolved_action,
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
            submitters: self
                .submitters
                .into_iter()
                .map(
                    ExpectedFormPolicySubmitterDescriptor::into_browser_form_policy_submitter_descriptor,
                )
                .collect(),
        }
    }
}

impl ExpectedFormPolicySubmitterDescriptor {
    fn into_browser_form_policy_submitter_descriptor(self) -> BrowserFormPolicySubmitterDescriptor {
        BrowserFormPolicySubmitterDescriptor {
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

impl ExpectedTextSemantic {
    fn into_browser_text_semantic(self) -> BrowserTextSemantic {
        BrowserTextSemantic {
            element: self.element,
            id: self.id,
            title: self.title,
            role: self.role,
            text: self.text,
            lang: self.lang,
            dir: self.dir,
            quote_cite: self.quote_cite,
            resolved_quote_cite: self.resolved_quote_cite,
            data_value: self.data_value,
            datetime: self.datetime,
            edit_cite: self.edit_cite,
            resolved_edit_cite: self.resolved_edit_cite,
            edit_datetime: self.edit_datetime,
            ruby_kind: self.ruby_kind,
            bidi_kind: self.bidi_kind,
            phrase_kind: self.phrase_kind,
        }
    }
}

impl ExpectedNavigationGroup {
    fn into_browser_navigation_group(self) -> BrowserNavigationGroup {
        BrowserNavigationGroup {
            element: self.element,
            id: self.id,
            role: self.role,
            text: self.text,
            accessible_name: self.accessible_name,
            aria_label: self.aria_label,
            aria_labelledby: self.aria_labelledby,
            landmark_kind: self.landmark_kind,
            list_kind: self.list_kind,
            item_count: self.item_count,
            list_start: self.list_start,
            list_marker_type: self.list_marker_type,
            list_reversed: self.list_reversed,
        }
    }
}

impl ExpectedSectionLandmark {
    fn into_browser_section_landmark(self) -> BrowserSectionLandmark {
        BrowserSectionLandmark {
            element: self.element,
            id: self.id,
            role: self.role,
            authored_role: self.authored_role,
            text: self.text,
            accessible_name: self.accessible_name,
            aria_label: self.aria_label,
            aria_labelledby: self.aria_labelledby,
            section_kind: self.section_kind,
            landmark_kind: self.landmark_kind,
            heading_level: self.heading_level,
            heading_text: self.heading_text,
        }
    }
}

impl ExpectedCommandElement {
    fn into_browser_command_element(self) -> BrowserCommandElement {
        BrowserCommandElement {
            element: self.element,
            id: self.id,
            role: self.role,
            authored_role: self.authored_role,
            command_kind: self.command_kind,
            text: self.text,
            accessible_name: self.accessible_name,
            accessible_description: self.accessible_description,
            href: self.href,
            resolved_href: self.resolved_href,
            target: self.target,
            effective_target: self.effective_target,
            control_type: self.control_type,
            form_owner: self.form_owner,
            form_action: self.form_action,
            resolved_form_action: self.resolved_form_action,
            form_method: self.form_method,
            form_target: self.form_target,
            form_novalidate: self.form_novalidate,
            command: self.command,
            command_for: self.command_for,
            popover_target: self.popover_target,
            popover_target_action: self.popover_target_action,
            aria_controls: self.aria_controls,
            aria_expanded: self.aria_expanded,
            aria_haspopup: self.aria_haspopup,
            aria_pressed: self.aria_pressed,
            aria_current: self.aria_current,
            aria_disabled: self.aria_disabled,
            tabindex: self.tabindex,
            accesskey: self.accesskey,
            event_handlers: self.event_handlers,
            focusable: self.focusable,
            disabled: self.disabled,
        }
    }
}

impl ExpectedActivationDescriptor {
    fn into_browser_activation_descriptor(self) -> BrowserActivationDescriptor {
        BrowserActivationDescriptor {
            element: self.element,
            id: self.id,
            role: self.role,
            authored_role: self.authored_role,
            command_kind: self.command_kind,
            activation_kind: self.activation_kind,
            target_id: self.target_id,
            target_kind: self.target_kind,
            text: self.text,
            accessible_name: self.accessible_name,
            accessible_description: self.accessible_description,
            disabled: self.disabled,
            focusable: self.focusable,
            tabindex: self.tabindex,
            accesskey: self.accesskey,
            event_handlers: self.event_handlers,
            handler_count: self.handler_count,
            command: self.command,
            command_for: self.command_for,
            popover_target: self.popover_target,
            popover_target_action: self.popover_target_action,
            aria_controls: self.aria_controls,
            aria_expanded: self.aria_expanded,
            aria_haspopup: self.aria_haspopup,
            aria_pressed: self.aria_pressed,
            aria_current: self.aria_current,
            aria_disabled: self.aria_disabled,
            control_type: self.control_type,
            href: self.href,
            resolved_href: self.resolved_href,
            effective_target: self.effective_target,
            form_owner: self.form_owner,
            form_action: self.form_action,
            resolved_form_action: self.resolved_form_action,
            form_method: self.form_method,
            form_target: self.form_target,
            form_novalidate: self.form_novalidate,
        }
    }
}

impl ExpectedPopover {
    fn into_browser_popover(self) -> BrowserPopover {
        BrowserPopover {
            element: self.element,
            id: self.id,
            role: self.role,
            text: self.text,
            accessible_name: self.accessible_name,
            accessible_description: self.accessible_description,
            aria_label: self.aria_label,
            aria_labelledby: self.aria_labelledby,
            aria_describedby: self.aria_describedby,
            popover: self.popover,
            invokers: self
                .invokers
                .into_iter()
                .map(ExpectedPopoverInvoker::into_browser_popover_invoker)
                .collect(),
        }
    }
}

impl ExpectedPopoverInvoker {
    fn into_browser_popover_invoker(self) -> BrowserPopoverInvoker {
        BrowserPopoverInvoker {
            element: self.element,
            id: self.id,
            text: self.text,
            accessible_name: self.accessible_name,
            command_kind: self.command_kind,
            command: self.command,
            command_for: self.command_for,
            popover_target: self.popover_target,
            popover_target_action: self.popover_target_action,
            aria_controls: self.aria_controls,
            aria_expanded: self.aria_expanded,
            focusable: self.focusable,
        }
    }
}

impl ExpectedAriaCollection {
    fn into_browser_aria_collection(self) -> BrowserAriaCollection {
        BrowserAriaCollection {
            element: self.element,
            id: self.id,
            role: self.role,
            text: self.text,
            accessible_name: self.accessible_name,
            accessible_description: self.accessible_description,
            aria_label: self.aria_label,
            aria_labelledby: self.aria_labelledby,
            aria_describedby: self.aria_describedby,
            aria_orientation: self.aria_orientation,
            aria_multiselectable: self.aria_multiselectable,
            aria_activedescendant: self.aria_activedescendant,
            aria_owns: self.aria_owns,
            item_count: self.item_count,
            selected_item_count: self.selected_item_count,
            checked_item_count: self.checked_item_count,
            current_item_count: self.current_item_count,
            disabled_item_count: self.disabled_item_count,
            items: self
                .items
                .into_iter()
                .map(ExpectedAriaCollectionItem::into_browser_aria_collection_item)
                .collect(),
        }
    }
}

impl ExpectedAriaCollectionItem {
    fn into_browser_aria_collection_item(self) -> BrowserAriaCollectionItem {
        BrowserAriaCollectionItem {
            element: self.element,
            id: self.id,
            role: self.role,
            text: self.text,
            accessible_name: self.accessible_name,
            aria_selected: self.aria_selected,
            aria_checked: self.aria_checked,
            aria_current: self.aria_current,
            aria_disabled: self.aria_disabled,
            aria_expanded: self.aria_expanded,
            aria_level: self.aria_level,
            aria_posinset: self.aria_posinset,
            aria_setsize: self.aria_setsize,
            aria_rowindex: self.aria_rowindex,
            aria_colindex: self.aria_colindex,
            aria_controls: self.aria_controls,
        }
    }
}

impl ExpectedAriaRange {
    fn into_browser_aria_range(self) -> BrowserAriaRange {
        BrowserAriaRange {
            element: self.element,
            id: self.id,
            role: self.role,
            text: self.text,
            accessible_name: self.accessible_name,
            accessible_description: self.accessible_description,
            aria_label: self.aria_label,
            aria_labelledby: self.aria_labelledby,
            aria_describedby: self.aria_describedby,
            aria_valuenow: self.aria_valuenow,
            aria_valuemin: self.aria_valuemin,
            aria_valuemax: self.aria_valuemax,
            aria_valuetext: self.aria_valuetext,
            aria_orientation: self.aria_orientation,
            aria_disabled: self.aria_disabled,
            aria_readonly: self.aria_readonly,
            aria_required: self.aria_required,
            tabindex: self.tabindex,
            text_value: self.text_value,
        }
    }
}

impl ExpectedAriaRelationDescriptor {
    fn into_browser_aria_relation_descriptor(self) -> BrowserAriaRelationDescriptor {
        BrowserAriaRelationDescriptor {
            element: self.element,
            id: self.id,
            text: self.text,
            aria_details: self.aria_details,
            details_text: self.details_text,
            aria_errormessage: self.aria_errormessage,
            errormessage_text: self.errormessage_text,
            aria_flowto: self.aria_flowto,
            flowto_text: self.flowto_text,
        }
    }
}

impl ExpectedAriaLiveRegion {
    fn into_browser_aria_live_region(self) -> BrowserAriaLiveRegion {
        BrowserAriaLiveRegion {
            element: self.element,
            id: self.id,
            role: self.role,
            text: self.text,
            accessible_name: self.accessible_name,
            accessible_description: self.accessible_description,
            aria_label: self.aria_label,
            aria_labelledby: self.aria_labelledby,
            aria_describedby: self.aria_describedby,
            aria_live: self.aria_live,
            aria_busy: self.aria_busy,
            aria_atomic: self.aria_atomic,
            aria_relevant: self.aria_relevant,
            aria_hidden: self.aria_hidden,
            update_kind: self.update_kind,
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

fn default_track_kind() -> String {
    "subtitles".to_string()
}

fn default_image_map_area_shape() -> String {
    "rect".to_string()
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

impl ExpectedImageCandidateDescriptor {
    fn into_browser_image_candidate_descriptor(self) -> BrowserImageCandidateDescriptor {
        BrowserImageCandidateDescriptor {
            src: self.src,
            resolved_src: self.resolved_src,
            srcset: self.srcset,
            resolved_srcset: self.resolved_srcset,
            sizes: self.sizes,
            alt: self.alt,
            width: self.width,
            height: self.height,
            loading: self.loading,
            decoding: self.decoding,
            fetchpriority: self.fetchpriority,
            crossorigin: self.crossorigin,
            referrerpolicy: self.referrerpolicy,
            usemap: self.usemap,
            ismap: self.ismap,
            has_alt: self.has_alt,
            source_count: self.source_count,
            source_srcset_count: self.source_srcset_count,
            candidate_count: self.candidate_count,
            source_type_hints: self.source_type_hints,
            source_media: self.source_media,
            sources: self
                .sources
                .into_iter()
                .map(ExpectedImageSource::into_browser_image_source)
                .collect(),
        }
    }
}

impl ExpectedImageMap {
    fn into_browser_image_map(self) -> BrowserImageMap {
        BrowserImageMap {
            id: self.id,
            name: self.name,
            areas: self
                .areas
                .into_iter()
                .map(ExpectedImageMapArea::into_browser_image_map_area)
                .collect(),
        }
    }
}

impl ExpectedImageMapArea {
    fn into_browser_image_map_area(self) -> BrowserImageMapArea {
        BrowserImageMapArea {
            id: self.id,
            shape: self.shape,
            coords: self.coords,
            href: self.href,
            resolved_href: self.resolved_href,
            alt: self.alt,
            target: self.target,
            effective_target: self.effective_target,
            rel: self.rel,
            rel_tokens: self.rel_tokens,
            rel_external: self.rel_external,
            rel_nofollow: self.rel_nofollow,
            rel_noopener: self.rel_noopener,
            rel_noreferrer: self.rel_noreferrer,
            ping: self.ping,
            resolved_ping: self.resolved_ping,
            attributionsrc: self.attributionsrc,
            resolved_attributionsrc: self.resolved_attributionsrc,
            download: self.download,
            hreflang: self.hreflang,
            referrerpolicy: self.referrerpolicy,
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
            sources: self
                .sources
                .into_iter()
                .map(ExpectedMediaSource::into_browser_media_source)
                .collect(),
            tracks: self
                .tracks
                .into_iter()
                .map(ExpectedMediaTrack::into_browser_media_track)
                .collect(),
        }
    }
}

impl ExpectedMediaSource {
    fn into_browser_media_source(self) -> BrowserMediaSource {
        BrowserMediaSource {
            src: self.src,
            resolved_src: self.resolved_src,
            type_hint: self.type_hint,
            media: self.media,
        }
    }
}

impl ExpectedMediaTrack {
    fn into_browser_media_track(self) -> BrowserMediaTrack {
        BrowserMediaTrack {
            kind: self.kind,
            src: self.src,
            resolved_src: self.resolved_src,
            srclang: self.srclang,
            label: self.label,
            default_track: self.default_track,
        }
    }
}

impl ExpectedMediaPlaybackDescriptor {
    fn into_browser_media_playback_descriptor(self) -> BrowserMediaPlaybackDescriptor {
        BrowserMediaPlaybackDescriptor {
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
            source_count: self.source_count,
            sources: self
                .sources
                .into_iter()
                .map(ExpectedMediaSource::into_browser_media_source)
                .collect(),
            track_count: self.track_count,
            default_track_count: self.default_track_count,
            tracks: self
                .tracks
                .into_iter()
                .map(ExpectedMediaTrack::into_browser_media_track)
                .collect(),
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

impl ExpectedEmbeddedPolicyDescriptor {
    fn into_browser_embedded_policy_descriptor(self) -> BrowserEmbeddedPolicyDescriptor {
        BrowserEmbeddedPolicyDescriptor {
            element: self.element,
            resource_kind: self.resource_kind,
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
            sandbox_token_count: self.sandbox_token_count,
            allow: self.allow,
            allowfullscreen: self.allowfullscreen,
            referrerpolicy: self.referrerpolicy,
            srcdoc: self.srcdoc,
            has_srcdoc: self.has_srcdoc,
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

impl ExpectedFocusNavigationDescriptor {
    fn into_browser_focus_navigation_descriptor(self) -> BrowserFocusNavigationDescriptor {
        BrowserFocusNavigationDescriptor {
            element: self.element,
            id: self.id,
            role: self.role,
            authored_role: self.authored_role,
            focus_kind: self.focus_kind,
            focusable: self.focusable,
            sequential_focus: self.sequential_focus,
            programmatic_focus: self.programmatic_focus,
            focus_blocked: self.focus_blocked,
            focus_block_reasons: self.focus_block_reasons,
            tabindex: self.tabindex,
            tabindex_order: self.tabindex_order,
            accesskey: self.accesskey,
            event_handlers: self.event_handlers,
            contenteditable: self.contenteditable,
            editing_mode: self.editing_mode,
            command: self.command,
            command_for: self.command_for,
            popover_target: self.popover_target,
            popover_target_action: self.popover_target_action,
            aria_controls: self.aria_controls,
            aria_activedescendant: self.aria_activedescendant,
            aria_expanded: self.aria_expanded,
            aria_haspopup: self.aria_haspopup,
            aria_disabled: self.aria_disabled,
            disabled: self.disabled,
            hidden: self.hidden,
            inert: self.inert,
            aria_hidden: self.aria_hidden,
            text: self.text,
        }
    }
}

impl ExpectedDisclosure {
    fn into_browser_disclosure(self) -> BrowserDisclosure {
        BrowserDisclosure {
            element: self.element,
            id: self.id,
            name: self.name,
            text: self.text,
            summary_text: self.summary_text,
            open: self.open,
            accessible_name: self.accessible_name,
            accessible_description: self.accessible_description,
            aria_label: self.aria_label,
            aria_labelledby: self.aria_labelledby,
            aria_describedby: self.aria_describedby,
            aria_modal: self.aria_modal,
            closedby: self.closedby,
        }
    }
}

impl ExpectedDisclosureStateDescriptor {
    fn into_browser_disclosure_state_descriptor(self) -> BrowserDisclosureStateDescriptor {
        BrowserDisclosureStateDescriptor {
            element: self.element,
            id: self.id,
            name: self.name,
            disclosure_kind: self.disclosure_kind,
            open: self.open,
            grouped: self.grouped,
            group_name: self.group_name,
            has_summary: self.has_summary,
            summary_text: self.summary_text,
            accessible_name: self.accessible_name,
            accessible_description: self.accessible_description,
            aria_label: self.aria_label,
            aria_labelledby: self.aria_labelledby,
            aria_describedby: self.aria_describedby,
            aria_modal: self.aria_modal,
            modal: self.modal,
            closedby: self.closedby,
            text: self.text,
            text_length: self.text_length,
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
            datalists: self
                .datalists
                .into_iter()
                .map(ExpectedFormDatalist::into_browser_form_datalist)
                .collect(),
            selects: self
                .selects
                .into_iter()
                .map(ExpectedFormSelect::into_browser_form_select)
                .collect(),
            outputs: self
                .outputs
                .into_iter()
                .map(ExpectedFormOutput::into_browser_form_output)
                .collect(),
            measurements: self
                .measurements
                .into_iter()
                .map(ExpectedFormMeasurement::into_browser_form_measurement)
                .collect(),
            object_controls: self
                .object_controls
                .into_iter()
                .map(ExpectedFormObject::into_browser_form_object)
                .collect(),
            successful_controls: self
                .successful_controls
                .into_iter()
                .map(ExpectedFormSuccessfulControl::into_browser_form_successful_control)
                .collect(),
            validation_controls: self
                .validation_controls
                .into_iter()
                .map(ExpectedFormValidationControl::into_browser_form_validation_control)
                .collect(),
            buttons: self
                .buttons
                .into_iter()
                .map(ExpectedFormButton::into_browser_form_button)
                .collect(),
            text_entries: self
                .text_entries
                .into_iter()
                .map(ExpectedFormTextEntry::into_browser_form_text_entry)
                .collect(),
            choice_controls: self
                .choice_controls
                .into_iter()
                .map(ExpectedFormChoiceControl::into_browser_form_choice_control)
                .collect(),
            file_controls: self
                .file_controls
                .into_iter()
                .map(ExpectedFormFileControl::into_browser_form_file_control)
                .collect(),
            hidden_controls: self
                .hidden_controls
                .into_iter()
                .map(ExpectedFormHiddenControl::into_browser_form_hidden_control)
                .collect(),
            image_controls: self
                .image_controls
                .into_iter()
                .map(ExpectedFormImageControl::into_browser_form_image_control)
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

impl ExpectedFormDatalist {
    fn into_browser_form_datalist(self) -> BrowserFormDatalist {
        BrowserFormDatalist {
            id: self.id,
            control_ids: self.control_ids,
            control_names: self.control_names,
            options: self
                .options
                .into_iter()
                .map(ExpectedDatalistOption::into_browser_datalist_option)
                .collect(),
        }
    }
}

impl ExpectedDatalistOption {
    fn into_browser_datalist_option(self) -> BrowserDatalistOption {
        BrowserDatalistOption {
            value: self.value,
            label: self.label,
            text: self.text,
            disabled: self.disabled,
        }
    }
}

impl ExpectedFormOutput {
    fn into_browser_form_output(self) -> BrowserFormOutput {
        BrowserFormOutput {
            id: self.id,
            name: self.name,
            form_owner: self.form_owner,
            labels: self.labels,
            accessible_name: self.accessible_name,
            accessible_description: self.accessible_description,
            for_tokens: self.for_tokens,
            for_control_ids: self.for_control_ids,
            for_control_names: self.for_control_names,
            for_control_types: self.for_control_types,
            value: self.value,
            disabled: self.disabled,
            will_validate: self.will_validate,
            validation_barred_reason: self.validation_barred_reason,
            text: self.text,
        }
    }
}

impl ExpectedFormMeasurement {
    fn into_browser_form_measurement(self) -> BrowserFormMeasurement {
        BrowserFormMeasurement {
            id: self.id,
            measurement_type: self.measurement_type,
            labels: self.labels,
            accessible_name: self.accessible_name,
            accessible_description: self.accessible_description,
            value: self.value,
            min: self.min,
            max: self.max,
            low: self.low,
            high: self.high,
            optimum: self.optimum,
            indeterminate: self.indeterminate,
            text: self.text,
        }
    }
}

impl ExpectedFormObject {
    fn into_browser_form_object(self) -> BrowserFormObject {
        BrowserFormObject {
            id: self.id,
            name: self.name,
            form_owner: self.form_owner,
            accessible_name: self.accessible_name,
            accessible_description: self.accessible_description,
            data: self.data,
            resolved_data: self.resolved_data,
            type_hint: self.type_hint,
            width: self.width,
            height: self.height,
            usemap: self.usemap,
            fallback_text: self.fallback_text,
            params: self
                .params
                .into_iter()
                .map(ExpectedFormObjectParam::into_browser_form_object_param)
                .collect(),
        }
    }
}

impl ExpectedFormObjectParam {
    fn into_browser_form_object_param(self) -> BrowserFormObjectParam {
        BrowserFormObjectParam {
            name: self.name,
            value: self.value,
        }
    }
}

impl ExpectedFormSuccessfulControl {
    fn into_browser_form_successful_control(self) -> BrowserFormSuccessfulControl {
        BrowserFormSuccessfulControl {
            id: self.id,
            control_type: self.control_type,
            name: self.name,
            form_owner: self.form_owner,
            submission_values: self.submission_values,
        }
    }
}

impl ExpectedFormValidationControl {
    fn into_browser_form_validation_control(self) -> BrowserFormValidationControl {
        BrowserFormValidationControl {
            id: self.id,
            control_type: self.control_type,
            name: self.name,
            form_owner: self.form_owner,
            will_validate: self.will_validate,
            required: self.required,
            validation_attributes: self.validation_attributes,
            validation_barred_reason: self.validation_barred_reason,
        }
    }
}

impl ExpectedFormSelect {
    fn into_browser_form_select(self) -> BrowserFormSelect {
        BrowserFormSelect {
            id: self.id,
            name: self.name,
            form_owner: self.form_owner,
            labels: self.labels,
            accessible_name: self.accessible_name,
            disabled: self.disabled,
            required: self.required,
            multiple: self.multiple,
            size: self.size,
            value: self.value,
            selected_options: self.selected_options,
            options: self
                .options
                .into_iter()
                .map(ExpectedSelectOption::into_browser_select_option)
                .collect(),
            text: self.text,
        }
    }
}

impl ExpectedFormTextEntry {
    fn into_browser_form_text_entry(self) -> BrowserFormTextEntry {
        BrowserFormTextEntry {
            id: self.id,
            control_type: self.control_type,
            name: self.name,
            form_owner: self.form_owner,
            labels: self.labels,
            accessible_name: self.accessible_name,
            accessible_description: self.accessible_description,
            placeholder: self.placeholder,
            value: self.value,
            text: self.text,
            autocomplete: self.autocomplete,
            autocomplete_tokens: self.autocomplete_tokens,
            autocapitalize: self.autocapitalize,
            enterkeyhint: self.enterkeyhint,
            dirname: self.dirname,
            spellcheck: self.spellcheck,
            autocorrect: self.autocorrect,
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
            disabled: self.disabled,
            required: self.required,
            readonly: self.readonly,
            will_validate: self.will_validate,
            validation_attributes: self.validation_attributes,
            validation_barred_reason: self.validation_barred_reason,
        }
    }
}

impl ExpectedFormChoiceControl {
    fn into_browser_form_choice_control(self) -> BrowserFormChoiceControl {
        BrowserFormChoiceControl {
            id: self.id,
            control_type: self.control_type,
            name: self.name,
            form_owner: self.form_owner,
            labels: self.labels,
            accessible_name: self.accessible_name,
            value: self.value,
            checked: self.checked,
            disabled: self.disabled,
            required: self.required,
            group_required: self.group_required,
            successful: self.successful,
            submission_values: self.submission_values,
            will_validate: self.will_validate,
            validation_attributes: self.validation_attributes,
            validation_barred_reason: self.validation_barred_reason,
            group_name: self.group_name,
            group_checked_ids: self.group_checked_ids,
            group_checked_values: self.group_checked_values,
        }
    }
}

impl ExpectedFormFileControl {
    fn into_browser_form_file_control(self) -> BrowserFormFileControl {
        BrowserFormFileControl {
            id: self.id,
            name: self.name,
            form_owner: self.form_owner,
            labels: self.labels,
            accessible_name: self.accessible_name,
            accept: self.accept,
            accept_tokens: self.accept_tokens,
            capture: self.capture,
            multiple: self.multiple,
            disabled: self.disabled,
            required: self.required,
            successful: self.successful,
            submission_values: self.submission_values,
            will_validate: self.will_validate,
            validation_attributes: self.validation_attributes,
            validation_barred_reason: self.validation_barred_reason,
        }
    }
}

impl ExpectedFormHiddenControl {
    fn into_browser_form_hidden_control(self) -> BrowserFormHiddenControl {
        BrowserFormHiddenControl {
            id: self.id,
            name: self.name,
            form_owner: self.form_owner,
            value: self.value,
            autocomplete: self.autocomplete,
            autocomplete_tokens: self.autocomplete_tokens,
            disabled: self.disabled,
            successful: self.successful,
            submission_values: self.submission_values,
            will_validate: self.will_validate,
            validation_barred_reason: self.validation_barred_reason,
        }
    }
}

impl ExpectedFormImageControl {
    fn into_browser_form_image_control(self) -> BrowserFormImageControl {
        BrowserFormImageControl {
            id: self.id,
            name: self.name,
            form_owner: self.form_owner,
            labels: self.labels,
            accessible_name: self.accessible_name,
            src: self.src,
            resolved_src: self.resolved_src,
            alt: self.alt,
            width: self.width,
            height: self.height,
            disabled: self.disabled,
            autofocus: self.autofocus,
            submitter: self.submitter,
            action: self.action,
            resolved_action: self.resolved_action,
            method: self.method,
            enctype: self.enctype,
            target: self.target,
            effective_target: self.effective_target,
            novalidate: self.novalidate,
            value: self.value,
            coordinate_names: self.coordinate_names,
            will_validate: self.will_validate,
            validation_barred_reason: self.validation_barred_reason,
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

impl ExpectedFormButton {
    fn into_browser_form_button(self) -> BrowserFormButton {
        BrowserFormButton {
            id: self.id,
            control_type: self.control_type,
            name: self.name,
            form_owner: self.form_owner,
            accessible_name: self.accessible_name,
            disabled: self.disabled,
            autofocus: self.autofocus,
            submitter: self.submitter,
            action: self.action,
            resolved_action: self.resolved_action,
            method: self.method,
            enctype: self.enctype,
            target: self.target,
            effective_target: self.effective_target,
            novalidate: self.novalidate,
            value: self.value,
            text: self.text,
            src: self.src,
            resolved_src: self.resolved_src,
            alt: self.alt,
            width: self.width,
            height: self.height,
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

impl ExpectedTableCell {
    fn into_browser_table_cell(self) -> BrowserTableCell {
        BrowserTableCell {
            table_index: self.table_index,
            table_id: self.table_id,
            table_caption: self.table_caption,
            section_kind: self.section_kind,
            row_index: self.row_index,
            column_index: self.column_index,
            element: self.element,
            id: self.id,
            text: self.text,
            accessible_name: self.accessible_name,
            header: self.header,
            scope: self.scope,
            headers: self.headers,
            abbr: self.abbr,
            rowspan: self.rowspan,
            colspan: self.colspan,
        }
    }
}
