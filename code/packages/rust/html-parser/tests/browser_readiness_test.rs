use coding_adventures_html_parser::{
    parse_browser_document, BrowserAnchor, BrowserDocument, BrowserDocumentMetadata,
    BrowserEmbeddedContext, BrowserForm, BrowserFormControl, BrowserFormSubmitter, BrowserHeading,
    BrowserHttpEquivHint, BrowserImage, BrowserImageSource, BrowserLink, BrowserMedia, BrowserMeta,
    BrowserMetadataDirective, BrowserRefresh, BrowserResource, BrowserResourceHint, BrowserScript,
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
    referrerpolicy: Option<String>,
    #[serde(default)]
    fetchpriority: Option<String>,
    #[serde(default)]
    blocking: Option<String>,
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
struct ExpectedResource {
    kind: String,
    url: String,
    resolved_url: Option<String>,
    rel: Option<String>,
    #[serde(default)]
    as_hint: Option<String>,
    type_hint: Option<String>,
    media: Option<String>,
    title: Option<String>,
    #[serde(default)]
    width: Option<String>,
    #[serde(default)]
    height: Option<String>,
    #[serde(default)]
    integrity: Option<String>,
    #[serde(default)]
    crossorigin: Option<String>,
    #[serde(default)]
    referrerpolicy: Option<String>,
    #[serde(default)]
    fetchpriority: Option<String>,
    #[serde(default)]
    blocking: Option<String>,
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
    referrerpolicy: Option<String>,
    #[serde(default)]
    fetchpriority: Option<String>,
    #[serde(default)]
    blocking: Option<String>,
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
    referrerpolicy: Option<String>,
    #[serde(default)]
    fetchpriority: Option<String>,
    #[serde(default)]
    blocking: Option<String>,
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
    accept_charset: Option<String>,
    #[serde(default)]
    autocomplete: Option<String>,
    #[serde(default)]
    rel: Option<String>,
    #[serde(default)]
    novalidate: bool,
    controls: Vec<ExpectedFormControl>,
    #[serde(default)]
    submitters: Vec<ExpectedFormSubmitter>,
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
    autofocus: bool,
    disabled: bool,
    #[serde(default)]
    required: bool,
    #[serde(default)]
    readonly: bool,
    checked: bool,
    #[serde(default)]
    multiple: bool,
    text: String,
    options: Vec<String>,
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
        BrowserResourceHint {
            kind: self.kind,
            url: self.url,
            resolved_url: self.resolved_url,
            rel: self.rel,
            as_hint: self.as_hint,
            type_hint: self.type_hint,
            media: self.media,
            integrity: self.integrity,
            crossorigin: self.crossorigin,
            referrerpolicy: self.referrerpolicy,
            fetchpriority: self.fetchpriority,
            blocking: self.blocking,
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

impl ExpectedResource {
    fn into_browser_resource(self) -> BrowserResource {
        BrowserResource {
            kind: self.kind,
            url: self.url,
            resolved_url: self.resolved_url,
            rel: self.rel,
            as_hint: self.as_hint,
            type_hint: self.type_hint,
            media: self.media,
            title: self.title,
            width: self.width,
            height: self.height,
            integrity: self.integrity,
            crossorigin: self.crossorigin,
            referrerpolicy: self.referrerpolicy,
            fetchpriority: self.fetchpriority,
            blocking: self.blocking,
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
            referrerpolicy: self.referrerpolicy,
            fetchpriority: self.fetchpriority,
            blocking: self.blocking,
            text: self.text,
        }
    }
}

impl ExpectedStylesheet {
    fn into_browser_stylesheet(self) -> BrowserStylesheet {
        BrowserStylesheet {
            source: self.source,
            href: self.href,
            resolved_href: self.resolved_href,
            rel: self.rel,
            type_hint: self.type_hint,
            media: self.media,
            title: self.title,
            disabled: self.disabled,
            alternate: self.alternate,
            integrity: self.integrity,
            crossorigin: self.crossorigin,
            referrerpolicy: self.referrerpolicy,
            fetchpriority: self.fetchpriority,
            blocking: self.blocking,
            text: self.text,
        }
    }
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
            accept_charset: self.accept_charset,
            autocomplete: self.autocomplete,
            rel: self.rel,
            novalidate: self.novalidate,
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

impl ExpectedFormControl {
    fn into_browser_form_control(self) -> BrowserFormControl {
        BrowserFormControl {
            id: self.id,
            control_type: self.control_type,
            name: self.name,
            form_owner: self.form_owner,
            labels: self.labels,
            accessible_name: self.accessible_name,
            placeholder: self.placeholder,
            autocomplete: self.autocomplete,
            autocapitalize: self.autocapitalize,
            enterkeyhint: self.enterkeyhint,
            dirname: self.dirname,
            accept: self.accept,
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
            autofocus: self.autofocus,
            disabled: self.disabled,
            required: self.required,
            readonly: self.readonly,
            checked: self.checked,
            multiple: self.multiple,
            text: self.text,
            options: self.options,
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
