use coding_adventures_html_parser::{
    parse_browser_document, BrowserAnchor, BrowserDocument, BrowserDocumentMetadata, BrowserForm,
    BrowserFormControl, BrowserHeading, BrowserImage, BrowserImageSource, BrowserLink,
    BrowserMedia, BrowserMeta, BrowserRefresh, BrowserResource, BrowserScript, BrowserStylesheet,
    BrowserTable, BrowserThemeColor,
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
    description: Option<String>,
    #[serde(default)]
    application_name: Option<String>,
    #[serde(default)]
    referrer_policy: Option<String>,
    #[serde(default)]
    robots: Option<String>,
    #[serde(default)]
    color_scheme: Option<String>,
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
    imagesrcset: Option<String>,
    #[serde(default)]
    resolved_imagesrcset: Option<String>,
    #[serde(default)]
    imagesizes: Option<String>,
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
    href: Option<String>,
    resolved_href: Option<String>,
    name: Option<String>,
    target: Option<String>,
    rel: Option<String>,
    title: Option<String>,
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
struct ExpectedForm {
    action: Option<String>,
    resolved_action: Option<String>,
    method: String,
    enctype: Option<String>,
    target: Option<String>,
    controls: Vec<ExpectedFormControl>,
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
    value: Option<String>,
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
            description: self.description,
            application_name: self.application_name,
            referrer_policy: self.referrer_policy,
            robots: self.robots,
            color_scheme: self.color_scheme,
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
            imagesrcset: self.imagesrcset,
            resolved_imagesrcset: self.resolved_imagesrcset,
            imagesizes: self.imagesizes,
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
            href: self.href,
            resolved_href: self.resolved_href,
            name: self.name,
            target: self.target,
            rel: self.rel,
            title: self.title,
            text: self.text,
        }
    }
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

impl ExpectedForm {
    fn into_browser_form(self) -> BrowserForm {
        BrowserForm {
            action: self.action,
            resolved_action: self.resolved_action,
            method: self.method,
            enctype: self.enctype,
            target: self.target,
            controls: self
                .controls
                .into_iter()
                .map(ExpectedFormControl::into_browser_form_control)
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
            value: self.value,
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
