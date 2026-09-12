//! Host-neutral navigation and page-loading orchestration for Venture.
//!
//! The platform shell owns windows, events, font implementations, and the
//! final paint backend. This crate composes the shared network, HTML, layout,
//! paint, and asynchronous image-resource lifecycle.

use browser_bookmarks::transact as transact_bookmarks;
pub use browser_bookmarks::{
    Bookmark, BookmarkCatalog, BookmarkChange, BookmarkRepository, BookmarkRepositoryError,
    BookmarkUrl, MemoryBookmarkRepository,
};
pub use browser_form_controls::{
    format_typed_value, normalize_color, parse_typed_step, parse_typed_value, step_typed_value,
    typed_constraints, BrowserControlModel, ControlAccessibilityAction, ControlAutofillDescriptor,
    ControlAutofillOutcome, ControlAutofillTransaction, ControlAutofillValue,
    ControlChoiceOptionState, ControlChoiceState, ControlClipboardPayload, ControlDefaultState,
    ControlEditorPresentation, ControlEditorState, ControlEffect, ControlFileItemState,
    ControlFilePickerRequest, ControlFileState, ControlKey, ControlMutationEvent,
    ControlMutationEventKind, ControlMutationSource, ControlNavigationUnit, ControlRect,
    ControlRestorationEntry, ControlSelection, ControlStateDiagnostic, ControlStatePrivacy,
    ControlStateSnapshot, ControlSuggestionDiagnostic, ControlSuggestionOption,
    ControlSuggestionPickerAction, ControlSuggestionState, ControlTextDirection,
    ControlTextMetrics, ControlValueDiagnostic, ControlValueState,
    CustomElementAccessibilityAction, CustomElementAccessibilityProjection,
    CustomElementAccessibilityState, CustomElementAccessibilityValue, CustomElementDiagnostic,
    CustomElementFormAssociation, CustomElementFormEntry, CustomElementFormEntryValue,
    CustomElementFormValue, CustomElementInternalsError, CustomElementLifecycleEvent,
    CustomElementRestorationEntry, CustomElementStateRestoreMode, CustomElementSubmissionGroup,
    CustomElementValidity, FileAcceptFilter, FormAssociatedCustomElementState, HostFileSelection,
    LiveValueKind, LiveValueState, MeterValueRegion, OutputDependencyValue, TypedValue,
    TypedValueConstraints, MAX_DATALIST_OPTIONS, MAX_SUGGESTION_QUERY_BYTES,
    MAX_SUGGESTION_RESULTS,
};
pub use browser_form_submission::{
    check_form_validity, dispatch_activation_with_image_coordinates, dispatch_form_reset,
    dispatch_implicit_submission, dispatch_request_submit, report_form_validity, FormActivation,
    FormDataEntry, FormDataValue, FormDiagnostic, FormDispatchOutcome, FormEntry, FormFileEntry,
    FormLifecycleEvent, FormMethod, FormNavigation, FormPlanningError, FormValidationMode,
    FormValidationReport, ImageSubmitCoordinates,
};
use browser_form_submission::{
    check_form_validity as check_planned_form_validity,
    report_form_validity as report_planned_form_validity,
};
pub use browser_navigation::{NavigationHistory, VisitedLinks, VisitedUrl};
#[cfg(test)]
use coding_adventures_html_parser::BrowserRenderNode;
use coding_adventures_html_parser::{parse_html, BrowserDocument, BrowserRenderTree};
use html_to_layout::{html_media_query_applies, HtmlAuthorStylesheet, HtmlStyleContext, HtmlTheme};
use html_to_paint::{
    decode_image_resource, hit_test_control, hit_test_link,
    html_render_tree_to_paint_with_style_context, resolve_scene_image_resources_incrementally,
    scene_image_resource_uris, ControlRegion, FetchedImage, HtmlImageResolver, HtmlImageResource,
    HtmlImageResourceError, HtmlPaintOutput, HtmlPaintViewport, LinkRegion,
};
use http1_client::HttpClient;
use layout_ir::TextMeasurer;
use paint_instructions::{
    PaintBase, PaintGroup, PaintInstruction, PaintRect, PaintScene, PixelContainer,
};
use std::fmt;
use text_interfaces::{FontMetrics, FontResolver, TextShaper};
use url_parser::Url;

pub const VERSION: &str = "0.8.0";

const CONTROL_TEXT_METRICS: ControlTextMetrics = ControlTextMetrics {
    advance: 8.0,
    line_height: 18.0,
    inset_x: 8.0,
    inset_y: 5.0,
    caret_width: 1.5,
};
const EDITOR_OVERLAY_PREFIX: &str = "venture-editor:";
const FORM_HISTORY_STATE_LIMIT: usize = 64;

/// Mosaic `VentureChrome` slot names, in interface declaration order.
pub const VENTURE_CHROME_SLOT_NAMES: [&str; 9] = [
    "address",
    "page-title",
    "status-text",
    "back-disabled",
    "forward-disabled",
    "bookmark-label",
    "bookmark-disabled",
    "view-source-disabled",
    "navigation-disabled",
];

/// Host-owned Mosaic node slot that mounts the native page renderer.
pub const VENTURE_CHROME_HOST_SURFACE_SLOT_NAME: &str = "content-surface";

/// Mosaic `VentureChrome` event names, in interface declaration order.
pub const VENTURE_CHROME_EVENT_NAMES: [&str; 8] = [
    "onBack",
    "onForward",
    "onHome",
    "onReload",
    "onToggleBookmark",
    "onViewSource",
    "onAddressChange",
    "onNavigate",
];

/// Host-neutral keyboard-scroll commands accepted by Venture's native page
/// surfaces. Platform adapters translate native key codes to these names; the
/// shared session owns their exact scrolling behavior.
pub const VENTURE_SCROLL_COMMAND_NAMES: [&str; 6] = [
    "line-up",
    "line-down",
    "page-up",
    "page-down",
    "document-start",
    "document-end",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BrowserScrollCommand {
    LineUp,
    LineDown,
    PageUp,
    PageDown,
    DocumentStart,
    DocumentEnd,
}

impl BrowserScrollCommand {
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "line-up" => Some(Self::LineUp),
            "line-down" => Some(Self::LineDown),
            "page-up" => Some(Self::PageUp),
            "page-down" => Some(Self::PageDown),
            "document-start" => Some(Self::DocumentStart),
            "document-end" => Some(Self::DocumentEnd),
            _ => None,
        }
    }

    pub const fn name(self) -> &'static str {
        match self {
            Self::LineUp => "line-up",
            Self::LineDown => "line-down",
            Self::PageUp => "page-up",
            Self::PageDown => "page-down",
            Self::DocumentStart => "document-start",
            Self::DocumentEnd => "document-end",
        }
    }
}

/// Vertical document scroll state in logical content coordinates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScrollState {
    offset_y: f64,
    viewport_height: f64,
    content_height: f64,
}

/// Target-neutral scroll geometry projected into native host surfaces.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BrowserScrollMetrics {
    pub offset_y: f64,
    pub viewport_height: f64,
    pub content_height: f64,
    pub max_offset_y: f64,
}

impl From<&ScrollState> for BrowserScrollMetrics {
    fn from(scroll: &ScrollState) -> Self {
        Self {
            offset_y: scroll.offset_y(),
            viewport_height: scroll.viewport_height(),
            content_height: scroll.content_height(),
            max_offset_y: scroll.max_offset_y(),
        }
    }
}

impl ScrollState {
    pub fn new(viewport_height: f64, content_height: f64) -> Self {
        Self {
            offset_y: 0.0,
            viewport_height: finite_non_negative(viewport_height),
            content_height: finite_non_negative(content_height),
        }
    }

    pub fn offset_y(&self) -> f64 {
        self.offset_y
    }

    pub fn viewport_height(&self) -> f64 {
        self.viewport_height
    }

    pub fn content_height(&self) -> f64 {
        self.content_height
    }

    pub fn max_offset_y(&self) -> f64 {
        (self.content_height - self.viewport_height).max(0.0)
    }

    pub fn set_offset_y(&mut self, offset_y: f64) -> f64 {
        self.offset_y = finite_non_negative(offset_y).min(self.max_offset_y());
        self.offset_y
    }

    pub fn scroll_by(&mut self, delta_y: f64) -> f64 {
        let delta_y = if delta_y.is_finite() { delta_y } else { 0.0 };
        self.set_offset_y(self.offset_y + delta_y)
    }

    /// Apply one semantic keyboard-scroll command.
    pub fn apply_command(&mut self, command: BrowserScrollCommand) -> f64 {
        match command {
            BrowserScrollCommand::LineUp => self.scroll_by(-40.0),
            BrowserScrollCommand::LineDown => self.scroll_by(40.0),
            BrowserScrollCommand::PageUp => self.scroll_by(-self.viewport_height * 0.9),
            BrowserScrollCommand::PageDown => self.scroll_by(self.viewport_height * 0.9),
            BrowserScrollCommand::DocumentStart => self.set_offset_y(0.0),
            BrowserScrollCommand::DocumentEnd => self.set_offset_y(self.max_offset_y()),
        }
    }

    /// Update page or viewport geometry and re-clamp the current offset.
    pub fn set_dimensions(&mut self, viewport_height: f64, content_height: f64) -> f64 {
        self.viewport_height = finite_non_negative(viewport_height);
        self.content_height = finite_non_negative(content_height);
        self.set_offset_y(self.offset_y)
    }

    pub fn hit_test<'a>(
        &self,
        links: &'a [LinkRegion],
        viewport_x: f64,
        viewport_y: f64,
    ) -> Option<&'a LinkRegion> {
        hit_test_link(links, viewport_x, viewport_y, self.offset_y)
    }
}

/// Build the viewport scene a paint backend should render at the current scroll.
///
/// The document instructions remain unchanged beneath a translated group. The
/// viewport-sized output surface provides the clip boundary at the backend.
pub fn scrolled_viewport_scene(scene: &PaintScene, scroll: &ScrollState) -> PaintScene {
    let mut fixed = Vec::new();
    let mut document = extract_fixed_instructions(scene.instructions.clone(), &mut fixed);
    for instruction in &mut document {
        apply_sticky_offset(instruction, scroll.offset_y);
    }
    let mut instructions = vec![PaintInstruction::Group(PaintGroup {
        base: PaintBase::default(),
        children: document,
        transform: Some([1.0, 0.0, 0.0, 1.0, 0.0, -scroll.offset_y]),
        opacity: None,
    })];
    instructions.extend(fixed);
    PaintScene {
        width: scene.width,
        height: scroll.viewport_height,
        background: scene.background.clone(),
        instructions,
        id: scene.id.clone(),
        metadata: scene.metadata.clone(),
    }
}

fn extract_fixed_instructions(
    instructions: Vec<PaintInstruction>,
    fixed: &mut Vec<PaintInstruction>,
) -> Vec<PaintInstruction> {
    let mut document = Vec::new();
    for mut instruction in instructions {
        if is_fixed_instruction(&instruction) {
            fixed.push(instruction);
            continue;
        }
        match &mut instruction {
            PaintInstruction::Group(group) => {
                group.children =
                    extract_fixed_instructions(std::mem::take(&mut group.children), fixed);
            }
            PaintInstruction::Clip(clip) => {
                clip.children =
                    extract_fixed_instructions(std::mem::take(&mut clip.children), fixed);
            }
            PaintInstruction::Layer(layer) => {
                layer.children =
                    extract_fixed_instructions(std::mem::take(&mut layer.children), fixed);
            }
            _ => {}
        }
        document.push(instruction);
    }
    document
}

fn apply_sticky_offset(instruction: &mut PaintInstruction, scroll_y: f64) {
    match instruction {
        PaintInstruction::Group(group) => {
            if let Some(metadata) = &group.base.metadata {
                if metadata
                    .get("layout.position")
                    .is_some_and(|value| value == "sticky")
                {
                    let top = metadata
                        .get("layout.sticky.top")
                        .and_then(|value| value.parse::<f64>().ok())
                        .unwrap_or(0.0);
                    let original_y = metadata
                        .get("layout.sticky.y")
                        .and_then(|value| value.parse::<f64>().ok())
                        .unwrap_or(0.0);
                    let offset = (scroll_y + top - original_y).max(0.0);
                    group.transform = Some([1.0, 0.0, 0.0, 1.0, 0.0, offset]);
                }
            }
            for child in &mut group.children {
                apply_sticky_offset(child, scroll_y);
            }
        }
        PaintInstruction::Clip(clip) => {
            for child in &mut clip.children {
                apply_sticky_offset(child, scroll_y);
            }
        }
        PaintInstruction::Layer(layer) => {
            for child in &mut layer.children {
                apply_sticky_offset(child, scroll_y);
            }
        }
        _ => {}
    }
}

fn is_fixed_instruction(instruction: &PaintInstruction) -> bool {
    let PaintInstruction::Group(group) = instruction else {
        return false;
    };
    group
        .base
        .metadata
        .as_ref()
        .and_then(|metadata| metadata.get("layout.position"))
        .is_some_and(|position| position == "fixed")
}

/// One resource returned by a browser-owned transport.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrowserFetchResponse {
    pub final_url: String,
    pub status: u16,
    pub media_type: Option<String>,
    pub body: Vec<u8>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BrowserFetchMethod {
    Get,
    Post,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrowserFetchRequest {
    pub method: BrowserFetchMethod,
    pub url: String,
    pub content_type: Option<String>,
    pub body: Vec<u8>,
}

impl BrowserFetchRequest {
    pub fn get(url: impl Into<String>) -> Self {
        Self {
            method: BrowserFetchMethod::Get,
            url: url.into(),
            content_type: None,
            body: Vec::new(),
        }
    }
}

impl BrowserFetchResponse {
    pub fn new(
        final_url: impl Into<String>,
        status: u16,
        media_type: Option<String>,
        body: Vec<u8>,
    ) -> Self {
        Self {
            final_url: final_url.into(),
            status,
            media_type,
            body,
        }
    }
}

/// Replaceable transport boundary for page and inline-image bytes.
pub trait BrowserResourceFetcher {
    fn fetch(&self, url: &str) -> Result<BrowserFetchResponse, String>;

    fn fetch_request(&self, request: &BrowserFetchRequest) -> Result<BrowserFetchResponse, String> {
        match request.method {
            BrowserFetchMethod::Get => self.fetch(&request.url),
            BrowserFetchMethod::Post => Err("browser fetcher does not support POST".to_string()),
        }
    }
}

impl<F> BrowserResourceFetcher for F
where
    F: Fn(&str) -> Result<BrowserFetchResponse, String>,
{
    fn fetch(&self, url: &str) -> Result<BrowserFetchResponse, String> {
        self(url)
    }
}

/// Concrete HTTP/1.0 transport adapter for a Venture host.
#[derive(Clone, Debug, Default)]
pub struct HttpBrowserFetcher {
    pub client: HttpClient,
}

impl HttpBrowserFetcher {
    pub fn new(client: HttpClient) -> Self {
        Self { client }
    }
}

impl BrowserResourceFetcher for HttpBrowserFetcher {
    fn fetch(&self, url: &str) -> Result<BrowserFetchResponse, String> {
        let response = self.client.get(url).map_err(|error| error.to_string())?;
        let media_type = response.head.header("Content-Type").map(ToOwned::to_owned);
        Ok(BrowserFetchResponse::new(
            response.final_url,
            response.head.status,
            media_type,
            response.body,
        ))
    }

    fn fetch_request(&self, request: &BrowserFetchRequest) -> Result<BrowserFetchResponse, String> {
        let response = match request.method {
            BrowserFetchMethod::Get => self.client.get(&request.url),
            BrowserFetchMethod::Post => self.client.post_form(&request.url, &request.body),
        }
        .map_err(|error| error.to_string())?;
        let media_type = response.head.header("Content-Type").map(ToOwned::to_owned);
        Ok(BrowserFetchResponse::new(
            response.final_url,
            response.head.status,
            media_type,
            response.body,
        ))
    }
}

/// A loaded HTML document ready for a platform paint backend.
#[derive(Clone, Debug, PartialEq)]
pub struct BrowserPage {
    pub requested_url: String,
    pub final_url: String,
    pub status: u16,
    pub source: String,
    pub document: BrowserDocument,
    pub render_tree: BrowserRenderTree,
    pub paint: HtmlPaintOutput,
    pub image_failures: Vec<HtmlImageResourceError>,
    pub image_resources: Vec<BrowserImageResource>,
    pub stylesheet_failures: Vec<BrowserStylesheetError>,
    pub stylesheet_resources: Vec<BrowserStylesheetResource>,
}

impl BrowserPage {
    pub fn pending_image_urls(&self) -> impl Iterator<Item = &str> {
        self.image_resources.iter().filter_map(|resource| {
            matches!(resource.state, BrowserImageResourceState::Pending)
                .then_some(resource.url.as_str())
        })
    }
}

/// Retained state for one deduplicated inline image, in DOM paint order.
#[derive(Clone, Debug, PartialEq)]
pub struct BrowserImageResource {
    pub url: String,
    pub state: BrowserImageResourceState,
}

#[derive(Clone, Debug, PartialEq)]
pub enum BrowserImageResourceState {
    Pending,
    Ready(PixelContainer),
    Failed(HtmlImageResourceError),
}

/// Retained author stylesheet in parser-defined document order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrowserStylesheetResource {
    pub url: Option<String>,
    pub base_url: String,
    pub media: Option<String>,
    pub render_blocking: bool,
    pub imported_by: Option<usize>,
    pub imports: Vec<usize>,
    pub state: BrowserStylesheetResourceState,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BrowserStylesheetResourceState {
    Ready(String),
    Pending,
    Failed(BrowserStylesheetError),
    Inactive,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BrowserStylesheetError {
    Fetch {
        url: String,
        message: String,
    },
    HttpStatus {
        url: String,
        status: u16,
    },
    UnsupportedMediaType {
        url: String,
        media_type: String,
    },
    Parse {
        url: Option<String>,
        message: String,
    },
    ImportCycle {
        url: String,
    },
}

impl fmt::Display for BrowserStylesheetError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Fetch { url, message } => {
                write!(formatter, "failed to fetch stylesheet {url}: {message}")
            }
            Self::HttpStatus { url, status } => {
                write!(formatter, "stylesheet {url} returned HTTP {status}")
            }
            Self::UnsupportedMediaType { url, media_type } => {
                write!(
                    formatter,
                    "stylesheet {url} used unsupported media type {media_type}"
                )
            }
            Self::Parse { url, message } => match url {
                Some(url) => write!(formatter, "failed to parse stylesheet {url}: {message}"),
                None => write!(formatter, "failed to parse inline stylesheet: {message}"),
            },
            Self::ImportCycle { url } => {
                write!(formatter, "stylesheet import cycle stopped at {url}")
            }
        }
    }
}

impl std::error::Error for BrowserStylesheetError {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BrowserSubresourceKind {
    Stylesheet,
    Image,
}

/// One scheduler request belonging to a particular committed navigation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrowserSubresourceRequest {
    pub navigation_id: u64,
    pub kind: BrowserSubresourceKind,
    pub ordinal: usize,
    pub url: String,
}

/// Host-delivered result for an earlier subresource request.
#[derive(Clone, Debug, PartialEq)]
pub struct BrowserSubresourceCompletion {
    pub request: BrowserSubresourceRequest,
    pub result: Result<BrowserSubresourcePayload, BrowserSubresourceError>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum BrowserSubresourcePayload {
    Stylesheet(String),
    Image(PixelContainer),
}

#[derive(Clone, Debug, PartialEq)]
pub enum BrowserSubresourceError {
    Stylesheet(BrowserStylesheetError),
    Image(HtmlImageResourceError),
}

impl BrowserSubresourceRequest {
    /// Execute fetch and decode work on the host scheduler, before delivery.
    pub fn resolve<F>(&self, fetcher: &F) -> BrowserSubresourceCompletion
    where
        F: BrowserResourceFetcher,
    {
        BrowserSubresourceCompletion {
            request: self.clone(),
            result: match self.kind {
                BrowserSubresourceKind::Stylesheet => fetch_browser_stylesheet(&self.url, fetcher)
                    .map(BrowserSubresourcePayload::Stylesheet)
                    .map_err(BrowserSubresourceError::Stylesheet),
                BrowserSubresourceKind::Image => fetch_and_decode_browser_image(&self.url, fetcher)
                    .map(BrowserSubresourcePayload::Image)
                    .map_err(BrowserSubresourceError::Image),
            },
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BrowserSubresourceDisposition {
    Applied,
    IgnoredDuplicate,
    IgnoredStaleNavigation,
}

/// Incremental repaint decision returned after a host completion.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrowserSubresourceUpdate {
    pub disposition: BrowserSubresourceDisposition,
    pub repaint_required: bool,
    pub pending_count: usize,
    /// Newly discovered effects, such as a validated stylesheet's imports.
    pub requests: Vec<BrowserSubresourceRequest>,
}

/// Effects produced when a navigation commits its document before images.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BrowserNavigationUpdate {
    pub viewport_changed: bool,
    pub requests: Vec<BrowserSubresourceRequest>,
    pub cancelled: Vec<BrowserSubresourceRequest>,
}

/// Reusable host scheduling seam for navigation-owned subresource work.
///
/// Implementations may use threads, an async runtime, browser fetch, or a
/// deterministic test queue. Core always delivers cancellations before new
/// requests, and completion generation checks remain the final safety net.
pub trait BrowserSubresourceScheduler {
    fn cancel(&mut self, request: &BrowserSubresourceRequest);
    fn request(&mut self, request: BrowserSubresourceRequest);
}

impl BrowserNavigationUpdate {
    pub fn dispatch_to(&self, scheduler: &mut dyn BrowserSubresourceScheduler) {
        for request in &self.cancelled {
            scheduler.cancel(request);
        }
        for request in &self.requests {
            scheduler.request(request.clone());
        }
    }
}

/// A synthetic browser document that a host presents outside the primary
/// navigation session.
///
/// The core owns the document bytes and escaping policy. Platform shells only
/// decide how to present the requested auxiliary window, so no toolkit needs
/// to parse or reconstruct source text independently.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrowserAuxiliaryDocument {
    pub kind: BrowserAuxiliaryDocumentKind,
    pub address: String,
    pub title: String,
    pub html: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BrowserAuxiliaryDocumentKind {
    ViewSource,
}

impl BrowserAuxiliaryDocumentKind {
    pub const fn name(self) -> &'static str {
        match self {
            Self::ViewSource => "view-source",
        }
    }
}

impl BrowserAuxiliaryDocument {
    /// Build a preformatted source document from an already-loaded page.
    ///
    /// This is deliberately pure: it never invokes the resource fetcher and
    /// therefore reflects the exact response text used by the current page.
    pub fn view_source(page: &BrowserPage) -> Self {
        let title = format!("Source: {}", page.final_url);
        let html = format!(
            "<!doctype html><html><head><title>{}</title></head><body><pre>{}</pre></body></html>",
            escape_html_text(&title),
            escape_html_text(&page.source),
        );
        Self {
            kind: BrowserAuxiliaryDocumentKind::ViewSource,
            address: format!("view-source:{}", page.final_url),
            title,
            html,
        }
    }
}

/// An operation the platform shell owns after shared browser state reduction.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BrowserHostEffect {
    OpenAuxiliaryDocument(BrowserAuxiliaryDocument),
}

/// Complete result of dispatching one shared chrome event.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BrowserHostEventOutcome {
    pub changed: bool,
    pub effect: Option<BrowserHostEffect>,
}

impl BrowserHostEventOutcome {
    pub const fn changed(changed: bool) -> Self {
        Self {
            changed,
            effect: None,
        }
    }

    pub fn effect(effect: BrowserHostEffect) -> Self {
        Self {
            changed: false,
            effect: Some(effect),
        }
    }
}

/// The current loaded page and its viewport interaction state.
///
/// This is the value a native content-area host keeps between input and paint
/// events. Replacing the page preserves viewport height while resetting scroll
/// to the top of the new document.
#[derive(Clone, Debug, PartialEq)]
pub struct BrowserViewport {
    page: BrowserPage,
    scroll: ScrollState,
}

impl BrowserViewport {
    pub fn new(page: BrowserPage, viewport_height: f64) -> Self {
        let scroll = ScrollState::new(viewport_height, page.paint.scene.height);
        Self { page, scroll }
    }

    pub fn page(&self) -> &BrowserPage {
        &self.page
    }

    pub fn scroll_state(&self) -> &ScrollState {
        &self.scroll
    }

    pub fn set_scroll_offset_y(&mut self, offset_y: f64) -> f64 {
        self.scroll.set_offset_y(offset_y)
    }

    pub fn scroll_by(&mut self, delta_y: f64) -> f64 {
        self.scroll.scroll_by(delta_y)
    }

    pub fn scroll_command(&mut self, command: BrowserScrollCommand) -> f64 {
        self.scroll.apply_command(command)
    }

    pub fn resize(&mut self, viewport_height: f64) -> f64 {
        self.scroll
            .set_dimensions(viewport_height, self.page.paint.scene.height)
    }

    pub fn replace_page(&mut self, page: BrowserPage) {
        self.scroll = ScrollState::new(self.scroll.viewport_height, page.paint.scene.height);
        self.page = page;
    }

    /// Replace the current page after viewport reflow while preserving the
    /// current logical scroll position, clamped to the new document geometry.
    pub fn reflow_page(&mut self, page: BrowserPage, viewport_height: f64) -> f64 {
        self.page = page;
        self.scroll
            .set_dimensions(viewport_height, self.page.paint.scene.height)
    }

    pub fn hit_test_link(&self, viewport_x: f64, viewport_y: f64) -> Option<&LinkRegion> {
        self.scroll
            .hit_test(&self.page.paint.links, viewport_x, viewport_y)
    }

    pub fn hit_test_control(&self, viewport_x: f64, viewport_y: f64) -> Option<&ControlRegion> {
        hit_test_control(
            &self.page.paint.controls,
            viewport_x,
            viewport_y,
            self.scroll.offset_y(),
        )
    }

    fn control_local_point(
        &self,
        viewport_x: f64,
        viewport_y: f64,
    ) -> Option<(ControlRegion, f64, f64)> {
        let region = self.hit_test_control(viewport_x, viewport_y)?.clone();
        let content_y = if region.fixed {
            viewport_y
        } else {
            viewport_y + self.scroll.offset_y()
        };
        Some((region.clone(), viewport_x - region.x, content_y - region.y))
    }

    pub fn viewport_scene(&self) -> PaintScene {
        scrolled_viewport_scene(&self.page.paint.scene, &self.scroll)
    }

    pub fn into_page(self) -> BrowserPage {
        self.page
    }
}

/// A browser navigation command emitted by native controls or content input.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BrowserNavigation {
    Navigate(String),
    Back,
    Forward,
    Home,
    Reload,
}

/// A host-neutral command emitted by Venture's shared browser chrome.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BrowserChromeAction {
    Navigate(BrowserNavigation),
    ToggleCurrentBookmark,
    ViewSource,
}

/// An event emitted by the shared Mosaic `VentureChrome` component.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BrowserChromeEvent {
    Back,
    Forward,
    Home,
    Reload,
    ToggleBookmark,
    ViewSource,
    AddressChange(String),
    Navigate,
}

impl BrowserChromeEvent {
    pub const fn mosaic_name(&self) -> &'static str {
        match self {
            Self::Back => "onBack",
            Self::Forward => "onForward",
            Self::Home => "onHome",
            Self::Reload => "onReload",
            Self::ToggleBookmark => "onToggleBookmark",
            Self::ViewSource => "onViewSource",
            Self::AddressChange(_) => "onAddressChange",
            Self::Navigate => "onNavigate",
        }
    }
}

/// Values projected into the shared Mosaic `VentureChrome` slots.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrowserChromeProps {
    pub address: String,
    pub page_title: String,
    pub status_text: String,
    pub back_disabled: bool,
    pub forward_disabled: bool,
    pub bookmark_label: String,
    pub bookmark_disabled: bool,
    pub view_source_disabled: bool,
    pub navigation_disabled: bool,
}

/// Host-neutral reducer for Venture's Mosaic-authored browser chrome.
///
/// Address edits remain a draft until the host successfully executes the
/// returned navigation command and calls [`Self::synchronize`]. This keeps a
/// failed load from replacing the user's input or the session's current URL.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrowserChromeController {
    address_draft: String,
}

impl BrowserChromeController {
    pub fn new(session: &BrowserSession) -> Self {
        Self {
            address_draft: session
                .history()
                .current_url()
                .unwrap_or_else(|| session.history().home_url())
                .to_string(),
        }
    }

    pub fn address_draft(&self) -> &str {
        &self.address_draft
    }

    /// Synchronize the address slot after a successful page load or redirect.
    pub fn synchronize(&mut self, session: &BrowserSession) {
        if let Some(current_url) = session.history().current_url() {
            self.address_draft = current_url.to_string();
        }
    }

    /// Reduce a Mosaic event to a Venture navigation command when appropriate.
    pub fn handle_event(
        &mut self,
        event: BrowserChromeEvent,
        session: &BrowserSession,
        navigation_disabled: bool,
    ) -> Option<BrowserChromeAction> {
        if navigation_disabled {
            return None;
        }

        match event {
            BrowserChromeEvent::AddressChange(value) => {
                self.address_draft = value;
                None
            }
            BrowserChromeEvent::Navigate => {
                let address = self.address_draft.trim();
                (!address.is_empty()).then(|| {
                    BrowserChromeAction::Navigate(BrowserNavigation::Navigate(address.to_string()))
                })
            }
            BrowserChromeEvent::Back if session.history().can_go_back() => {
                Some(BrowserChromeAction::Navigate(BrowserNavigation::Back))
            }
            BrowserChromeEvent::Forward if session.history().can_go_forward() => {
                Some(BrowserChromeAction::Navigate(BrowserNavigation::Forward))
            }
            BrowserChromeEvent::Home => {
                Some(BrowserChromeAction::Navigate(BrowserNavigation::Home))
            }
            BrowserChromeEvent::Reload if session.history().current_url().is_some() => {
                Some(BrowserChromeAction::Navigate(BrowserNavigation::Reload))
            }
            BrowserChromeEvent::ToggleBookmark if session.history().current_url().is_some() => {
                Some(BrowserChromeAction::ToggleCurrentBookmark)
            }
            BrowserChromeEvent::ViewSource if session.viewport().is_some() => {
                Some(BrowserChromeAction::ViewSource)
            }
            BrowserChromeEvent::Back | BrowserChromeEvent::Forward | BrowserChromeEvent::Reload => {
                None
            }
            BrowserChromeEvent::ToggleBookmark | BrowserChromeEvent::ViewSource => None,
        }
    }

    /// Project one coherent snapshot for all six Mosaic chrome slots.
    pub fn props(
        &self,
        session: &BrowserSession,
        status_text: impl Into<String>,
        navigation_disabled: bool,
    ) -> BrowserChromeProps {
        let page_title = session
            .viewport()
            .and_then(|viewport| viewport.page().document.title.as_deref())
            .map(str::trim)
            .filter(|title| !title.is_empty())
            .unwrap_or("")
            .to_string();

        BrowserChromeProps {
            address: self.address_draft.clone(),
            page_title,
            status_text: status_text.into(),
            back_disabled: navigation_disabled || !session.history().can_go_back(),
            forward_disabled: navigation_disabled || !session.history().can_go_forward(),
            bookmark_label: if session.current_is_bookmarked() {
                "Remove Bookmark"
            } else {
                "Bookmark"
            }
            .to_string(),
            bookmark_disabled: navigation_disabled || session.history().current_url().is_none(),
            view_source_disabled: navigation_disabled || session.viewport().is_none(),
            navigation_disabled,
        }
    }
}

/// Shared host state behind Venture's native Mosaic adapters.
///
/// Platform crates remain responsible for constructing their native text and
/// paint pipeline. This controller owns the behavior that must not drift
/// between those adapters: Mosaic event reduction, transactional status and
/// chrome synchronization, scrolling, absolute scrollbar projection, link
/// activation, and hover status.
#[derive(Clone, Debug, PartialEq)]
pub struct BrowserHostController {
    session: BrowserSession,
    chrome: BrowserChromeController,
    status_text: String,
    hovered_link_url: Option<String>,
}

impl BrowserHostController {
    pub fn new(session: BrowserSession) -> Self {
        let chrome = BrowserChromeController::new(&session);
        Self {
            session,
            chrome,
            status_text: "Ready".to_string(),
            hovered_link_url: None,
        }
    }

    pub fn session(&self) -> &BrowserSession {
        &self.session
    }

    /// Mutable session access for the platform-owned reflow and paint seam.
    pub fn session_mut(&mut self) -> &mut BrowserSession {
        &mut self.session
    }

    /// Refresh chrome after a platform-owned session action such as form
    /// activation, which may have completed a navigation internally.
    pub fn synchronize_session_state(&mut self) {
        self.hovered_link_url = None;
        self.chrome.synchronize(&self.session);
        self.status_text = "Ready".to_string();
    }

    pub fn props(&self) -> BrowserChromeProps {
        self.chrome.props(
            &self.session,
            self.hovered_link_url
                .clone()
                .unwrap_or_else(|| self.status_text.clone()),
            false,
        )
    }

    /// Reduce a Mosaic event and execute any resulting navigation through the
    /// platform's native page-composition pipeline.
    pub fn handle_event<F>(
        &mut self,
        event: BrowserChromeEvent,
        bookmarks: &mut dyn BookmarkRepository,
        execute: F,
    ) -> Result<bool, BrowserCommandError>
    where
        F: FnOnce(&mut BrowserSession, BrowserNavigation) -> Result<bool, BrowserLoadError>,
    {
        Ok(self
            .handle_event_with_effect(event, bookmarks, execute)?
            .changed)
    }

    /// Dispatch an event while preserving any platform-owned presentation
    /// effect produced by the shared state machine.
    pub fn handle_event_with_effect<F>(
        &mut self,
        event: BrowserChromeEvent,
        bookmarks: &mut dyn BookmarkRepository,
        execute: F,
    ) -> Result<BrowserHostEventOutcome, BrowserCommandError>
    where
        F: FnOnce(&mut BrowserSession, BrowserNavigation) -> Result<bool, BrowserLoadError>,
    {
        self.hovered_link_url = None;
        let Some(action) = self.chrome.handle_event(event, &self.session, false) else {
            return Ok(BrowserHostEventOutcome::default());
        };
        match action {
            BrowserChromeAction::Navigate(navigation) => self
                .execute_navigation(navigation, execute)
                .map(BrowserHostEventOutcome::changed)
                .map_err(BrowserCommandError::Load),
            BrowserChromeAction::ToggleCurrentBookmark => {
                self.status_text = "Saving bookmark".to_string();
                match self.session.toggle_current_bookmark(bookmarks) {
                    Ok(change) => {
                        self.status_text = "Ready".to_string();
                        Ok(BrowserHostEventOutcome::changed(change.changed()))
                    }
                    Err(error) => {
                        self.status_text = format!("Bookmark failed: {error}");
                        Err(BrowserCommandError::Bookmark(error))
                    }
                }
            }
            BrowserChromeAction::ViewSource => {
                let page = self
                    .session
                    .viewport()
                    .expect("view-source action requires a retained viewport")
                    .page();
                Ok(BrowserHostEventOutcome::effect(
                    BrowserHostEffect::OpenAuxiliaryDocument(
                        BrowserAuxiliaryDocument::view_source(page),
                    ),
                ))
            }
        }
    }

    pub fn scroll_by(&mut self, delta_y: f64) -> bool {
        self.hovered_link_url = None;
        let Some(viewport) = self.session.viewport_mut() else {
            return false;
        };
        let before = viewport.scroll_state().offset_y();
        viewport.scroll_by(delta_y);
        viewport.scroll_state().offset_y() != before
    }

    pub fn scroll_command(&mut self, command: BrowserScrollCommand) -> bool {
        self.hovered_link_url = None;
        let Some(viewport) = self.session.viewport_mut() else {
            return false;
        };
        let before = viewport.scroll_state().offset_y();
        viewport.scroll_command(command);
        viewport.scroll_state().offset_y() != before
    }

    pub fn scroll_metrics(&self) -> Option<BrowserScrollMetrics> {
        self.session.scroll_metrics()
    }

    pub fn scroll_to(&mut self, offset_y: f64) -> bool {
        self.hovered_link_url = None;
        let before = self
            .session
            .scroll_metrics()
            .map(|metrics| metrics.offset_y);
        let after = self.session.set_scroll_offset_y(offset_y);
        before
            .zip(after)
            .is_some_and(|(before, after)| before != after)
    }

    /// Activate the shared-session link under a native surface coordinate and
    /// execute it through the same platform navigation closure as chrome.
    pub fn activate_link<F>(
        &mut self,
        viewport_x: f64,
        viewport_y: f64,
        execute: F,
    ) -> Result<bool, BrowserLoadError>
    where
        F: FnOnce(&mut BrowserSession, BrowserNavigation) -> Result<bool, BrowserLoadError>,
    {
        self.hovered_link_url = None;
        let Some(url) = self
            .session
            .hovered_link_url(viewport_x, viewport_y)
            .map(str::to_owned)
        else {
            return Ok(false);
        };
        self.execute_navigation(BrowserNavigation::Navigate(url), execute)
    }

    pub fn update_hover(&mut self, viewport_x: f64, viewport_y: f64) -> bool {
        self.hovered_link_url = if viewport_x.is_finite() && viewport_y.is_finite() {
            self.session
                .hovered_link_url(viewport_x, viewport_y)
                .map(str::to_owned)
        } else {
            None
        };
        self.hovered_link_url.is_some()
    }

    /// Clear transient hover projection before a platform-owned resize.
    pub fn clear_hover(&mut self) {
        self.hovered_link_url = None;
    }

    fn execute_navigation<F>(
        &mut self,
        navigation: BrowserNavigation,
        execute: F,
    ) -> Result<bool, BrowserLoadError>
    where
        F: FnOnce(&mut BrowserSession, BrowserNavigation) -> Result<bool, BrowserLoadError>,
    {
        self.status_text = "Loading".to_string();
        match execute(&mut self.session, navigation) {
            Ok(changed) => {
                if changed {
                    self.chrome.synchronize(&self.session);
                }
                self.status_text = "Ready".to_string();
                Ok(changed)
            }
            Err(error) => {
                self.status_text = format!("Load failed: {error}");
                Err(error)
            }
        }
    }
}

/// Host-neutral browser state spanning navigation, loading, and the viewport.
///
/// Navigation is transactional: a failed page load leaves both history and the
/// current viewport untouched. Successful redirects replace the current
/// history entry with the final fetched URL.
#[derive(Clone, Debug, PartialEq)]
pub struct BrowserSession {
    history: NavigationHistory,
    visited_links: VisitedLinks,
    bookmarks: BookmarkCatalog,
    viewport: Option<BrowserViewport>,
    controls: BrowserControlModel,
    form_diagnostics: Vec<FormDiagnostic>,
    form_lifecycle_events: Vec<FormLifecycleEvent>,
    control_mutation_events: Vec<ControlMutationEvent>,
    form_history_states: Vec<(String, ControlStateSnapshot)>,
    viewport_height: f64,
    navigation_id: u64,
}

impl BrowserSession {
    pub fn new(home_url: impl Into<String>, viewport_height: f64) -> Self {
        Self {
            history: NavigationHistory::new(home_url),
            visited_links: VisitedLinks::new(),
            bookmarks: BookmarkCatalog::new(),
            viewport: None,
            controls: BrowserControlModel::default(),
            form_diagnostics: Vec::new(),
            form_lifecycle_events: Vec::new(),
            control_mutation_events: Vec::new(),
            form_history_states: Vec::new(),
            viewport_height: finite_non_negative(viewport_height),
            navigation_id: 0,
        }
    }

    pub fn history(&self) -> &NavigationHistory {
        &self.history
    }

    pub fn visited_links(&self) -> &VisitedLinks {
        &self.visited_links
    }

    pub fn bookmarks(&self) -> &BookmarkCatalog {
        &self.bookmarks
    }

    pub fn replace_bookmarks(&mut self, bookmarks: BookmarkCatalog) {
        self.bookmarks = bookmarks;
    }

    pub fn current_is_bookmarked(&self) -> bool {
        self.history
            .current_url()
            .is_some_and(|url| self.bookmarks.contains(url))
    }

    /// Toggle the current final URL and commit memory only after persistence.
    pub fn toggle_current_bookmark(
        &mut self,
        repository: &mut dyn BookmarkRepository,
    ) -> Result<BookmarkChange, BookmarkRepositoryError> {
        let Some(url) = self.history.current_url().map(str::to_owned) else {
            return Ok(BookmarkChange::Unchanged);
        };
        let title = self
            .viewport
            .as_ref()
            .and_then(|viewport| viewport.page().document.title.as_deref())
            .map(str::trim)
            .filter(|title| !title.is_empty())
            .unwrap_or(&url)
            .to_string();
        transact_bookmarks(&mut self.bookmarks, repository, |candidate| {
            candidate.toggle(&url, title)
        })
    }

    pub fn viewport(&self) -> Option<&BrowserViewport> {
        self.viewport.as_ref()
    }

    pub fn viewport_mut(&mut self) -> Option<&mut BrowserViewport> {
        self.viewport.as_mut()
    }

    pub const fn navigation_id(&self) -> u64 {
        self.navigation_id
    }

    /// Pending requests for the current document in deterministic paint order.
    pub fn pending_subresource_requests(&self) -> Vec<BrowserSubresourceRequest> {
        let Some(viewport) = &self.viewport else {
            return Vec::new();
        };
        let page = viewport.page();
        let mut requests = pending_stylesheet_ordinals(&page.stylesheet_resources)
            .into_iter()
            .map(|ordinal| BrowserSubresourceRequest {
                navigation_id: self.navigation_id,
                kind: BrowserSubresourceKind::Stylesheet,
                ordinal,
                url: page.stylesheet_resources[ordinal]
                    .url
                    .clone()
                    .expect("pending stylesheet must have a URL"),
            })
            .collect::<Vec<_>>();
        requests.extend(
            page.image_resources
                .iter()
                .enumerate()
                .filter(|(_, resource)| {
                    matches!(resource.state, BrowserImageResourceState::Pending)
                })
                .map(|(ordinal, resource)| BrowserSubresourceRequest {
                    navigation_id: self.navigation_id,
                    kind: BrowserSubresourceKind::Image,
                    ordinal,
                    url: resource.url.clone(),
                }),
        );
        requests
    }

    pub fn resize(&mut self, viewport_height: f64) -> f64 {
        self.viewport_height = finite_non_negative(viewport_height);
        self.viewport
            .as_mut()
            .map_or(0.0, |viewport| viewport.resize(self.viewport_height))
    }

    pub fn scroll_metrics(&self) -> Option<BrowserScrollMetrics> {
        self.viewport
            .as_ref()
            .map(|viewport| BrowserScrollMetrics::from(viewport.scroll_state()))
    }

    pub fn set_scroll_offset_y(&mut self, offset_y: f64) -> Option<f64> {
        self.viewport
            .as_mut()
            .map(|viewport| viewport.set_scroll_offset_y(offset_y))
    }

    /// Resolve the link under a viewport coordinate without mutating browser
    /// state. Native hosts use this for hover status and cursor selection.
    pub fn hovered_link_url(&self, viewport_x: f64, viewport_y: f64) -> Option<&str> {
        self.viewport
            .as_ref()?
            .hit_test_link(viewport_x, viewport_y)
            .map(|link| link.url.as_str())
    }

    pub fn controls(&self) -> &BrowserControlModel {
        &self.controls
    }

    pub fn live_value_states(&self) -> Vec<LiveValueState> {
        self.controls.live_value_states()
    }

    pub fn live_value_state(&self, key: &str) -> Option<LiveValueState> {
        self.controls.live_value_state(key)
    }

    pub fn live_value_states_host_json(&self) -> String {
        self.controls.live_value_states_host_json()
    }

    pub fn set_live_value<M, S, FM, R>(
        &mut self,
        key: &str,
        value: Option<&str>,
        pipeline: &BrowserPagePipeline<'_, M, S, FM, R>,
    ) -> Option<ControlEffect>
    where
        M: TextMeasurer,
        S: TextShaper,
        FM: FontMetrics<Handle = S::Handle>,
        R: FontResolver<Handle = S::Handle>,
    {
        let effect = self.controls.set_live_value(key, value)?;
        self.reflow_controls(pipeline)?;
        Some(effect)
    }

    pub fn recalculate_output<M, S, FM, R, F>(
        &mut self,
        key: &str,
        calculate: F,
        pipeline: &BrowserPagePipeline<'_, M, S, FM, R>,
    ) -> Option<ControlEffect>
    where
        M: TextMeasurer,
        S: TextShaper,
        FM: FontMetrics<Handle = S::Handle>,
        R: FontResolver<Handle = S::Handle>,
        F: FnOnce(&[OutputDependencyValue]) -> String,
    {
        let effect = self.controls.recalculate_output(key, calculate)?;
        self.reflow_controls(pipeline)?;
        Some(effect)
    }

    /// Return reusable validation and accessibility value metadata for one
    /// control without exposing host-specific widget state.
    pub fn control_value_state(&self, key: &str) -> Option<ControlValueState> {
        self.controls.value_state(key)
    }

    pub fn focused_control_value_state(&self) -> Option<ControlValueState> {
        self.controls
            .focused_key()
            .and_then(|key| self.controls.value_state(key))
    }

    pub fn control_choice_state(&self, key: &str) -> Option<ControlChoiceState> {
        self.controls.choice_state(key)
    }

    pub fn focused_control_choice_state(&self) -> Option<ControlChoiceState> {
        self.controls
            .focused_key()
            .and_then(|key| self.controls.choice_state(key))
    }

    pub fn control_suggestion_state(&self, key: &str) -> Option<&ControlSuggestionState> {
        self.controls.suggestion_state(key)
    }

    pub fn focused_control_suggestion_state(&self) -> Option<&ControlSuggestionState> {
        self.controls.focused_suggestion_state()
    }

    pub fn open_control_suggestions<M, S, FM, R>(
        &mut self,
        key: &str,
        query: &str,
        limit: usize,
        pipeline: &BrowserPagePipeline<'_, M, S, FM, R>,
    ) -> Option<ControlEffect>
    where
        M: TextMeasurer,
        S: TextShaper,
        FM: FontMetrics<Handle = S::Handle>,
        R: FontResolver<Handle = S::Handle>,
    {
        let effect = self.controls.open_suggestions(key, query, limit)?;
        self.reflow_controls(pipeline)?;
        Some(effect)
    }

    pub fn control_suggestion_picker_action<M, S, FM, R>(
        &mut self,
        key: &str,
        action: ControlSuggestionPickerAction,
        pipeline: &BrowserPagePipeline<'_, M, S, FM, R>,
    ) -> Option<ControlEffect>
    where
        M: TextMeasurer,
        S: TextShaper,
        FM: FontMetrics<Handle = S::Handle>,
        R: FontResolver<Handle = S::Handle>,
    {
        let effect = self.controls.apply_suggestion_picker_action(key, action)?;
        self.record_control_effect_events(&effect);
        self.form_diagnostics.clear();
        self.controls.clear_validation();
        self.reflow_controls(pipeline)?;
        Some(effect)
    }

    pub fn control_default_state(&self, key: &str) -> Option<ControlDefaultState> {
        self.controls.default_state(key)
    }

    pub fn control_autofill_descriptors(&self) -> Vec<ControlAutofillDescriptor> {
        self.controls.autofill_descriptors()
    }

    pub fn capture_form_state(&self, privacy: ControlStatePrivacy) -> ControlStateSnapshot {
        self.controls.capture_state(privacy)
    }

    pub fn restore_form_state<M, S, FM, R>(
        &mut self,
        snapshot: &ControlStateSnapshot,
        pipeline: &BrowserPagePipeline<'_, M, S, FM, R>,
    ) -> Vec<ControlEffect>
    where
        M: TextMeasurer,
        S: TextShaper,
        FM: FontMetrics<Handle = S::Handle>,
        R: FontResolver<Handle = S::Handle>,
    {
        let effects = self.controls.restore_state(snapshot);
        self.form_diagnostics.clear();
        self.controls.clear_validation();
        self.reflow_controls(pipeline);
        effects
    }

    pub fn apply_control_autofill<M, S, FM, R>(
        &mut self,
        transaction: &ControlAutofillTransaction,
        pipeline: &BrowserPagePipeline<'_, M, S, FM, R>,
    ) -> ControlAutofillOutcome
    where
        M: TextMeasurer,
        S: TextShaper,
        FM: FontMetrics<Handle = S::Handle>,
        R: FontResolver<Handle = S::Handle>,
    {
        let outcome = self.controls.apply_autofill(transaction);
        self.control_mutation_events
            .extend(outcome.events.iter().cloned());
        self.form_diagnostics.clear();
        self.controls.clear_validation();
        self.reflow_controls(pipeline);
        outcome
    }

    pub fn take_control_mutation_events(&mut self) -> Vec<ControlMutationEvent> {
        std::mem::take(&mut self.control_mutation_events)
    }

    fn record_control_effect_events(&mut self, effect: &ControlEffect) {
        let ControlEffect::SuggestionCommitted { key, .. } = effect else {
            return;
        };
        self.control_mutation_events.extend([
            ControlMutationEvent {
                key: key.clone(),
                kind: ControlMutationEventKind::Input,
                source: ControlMutationSource::SuggestionPicker,
            },
            ControlMutationEvent {
                key: key.clone(),
                kind: ControlMutationEventKind::Change,
                source: ControlMutationSource::SuggestionPicker,
            },
        ]);
    }

    pub fn control_file_state(&self, key: &str) -> Option<ControlFileState> {
        self.controls.file_state(key)
    }

    pub fn focused_control_file_state(&self) -> Option<ControlFileState> {
        self.controls
            .focused_key()
            .and_then(|key| self.controls.file_state(key))
    }

    pub fn focused_file_picker_request(&self) -> Option<ControlFilePickerRequest> {
        self.controls
            .focused_key()
            .and_then(|key| self.controls.file_picker_request(key))
    }

    pub fn form_associated_custom_elements(&self) -> &[FormAssociatedCustomElementState] {
        self.controls.form_associated_custom_elements()
    }

    pub fn attach_form_associated_custom_element(
        &mut self,
        key: &str,
    ) -> Result<(), CustomElementInternalsError> {
        self.controls.attach_form_associated_custom_element(key)
    }

    pub fn reassociate_form_associated_custom_element(
        &mut self,
        key: &str,
        association: CustomElementFormAssociation,
    ) -> Result<(), CustomElementInternalsError> {
        self.controls
            .reassociate_form_associated_custom_element(key, association)
    }

    pub fn set_custom_element_form_value(
        &mut self,
        key: &str,
        value: Option<CustomElementFormValue>,
        restoration_state: Option<CustomElementFormValue>,
    ) -> Result<(), CustomElementInternalsError> {
        self.controls
            .set_custom_element_form_value(key, value, restoration_state)
    }

    pub fn set_custom_element_validity(
        &mut self,
        key: &str,
        validity: CustomElementValidity,
        message: Option<String>,
        anchor: Option<String>,
    ) -> Result<(), CustomElementInternalsError> {
        self.controls
            .set_custom_element_validity(key, validity, message, anchor)
    }

    pub fn set_custom_element_disabled(
        &mut self,
        key: &str,
        disabled: bool,
    ) -> Result<(), CustomElementInternalsError> {
        self.controls.set_custom_element_disabled(key, disabled)
    }

    pub fn set_custom_element_accessibility_value(
        &mut self,
        key: &str,
        value: CustomElementAccessibilityValue,
    ) -> Result<(), CustomElementInternalsError> {
        self.controls
            .set_custom_element_accessibility_value(key, value)
    }

    pub fn set_custom_element_accessibility_projection(
        &mut self,
        key: &str,
        projection: CustomElementAccessibilityProjection,
    ) -> Result<(), CustomElementInternalsError> {
        self.controls
            .set_custom_element_accessibility_projection(key, projection)
    }

    pub fn custom_element_accessibility_action(
        &mut self,
        key: &str,
        action: CustomElementAccessibilityAction,
    ) -> Result<(), CustomElementInternalsError> {
        self.controls
            .custom_element_accessibility_action(key, action)
    }

    pub fn custom_element_accessibility_state(
        &self,
        key: &str,
    ) -> Option<CustomElementAccessibilityState> {
        self.controls.custom_element_accessibility_state(key)
    }

    pub fn restore_custom_element_state(
        &mut self,
        key: &str,
        mode: CustomElementStateRestoreMode,
    ) -> Result<(), CustomElementInternalsError> {
        self.controls.restore_custom_element_state(key, mode)
    }

    pub fn take_custom_element_lifecycle_events(&mut self) -> Vec<CustomElementLifecycleEvent> {
        self.controls.take_custom_element_lifecycle_events()
    }

    /// Deliver a path-free picker result into the shared reducer. Native and
    /// web hosts retain responsibility only for opening their picker and
    /// reading the selected bytes.
    pub fn control_files_selected<M, S, FM, R>(
        &mut self,
        key: &str,
        files: Vec<HostFileSelection>,
        pipeline: &BrowserPagePipeline<'_, M, S, FM, R>,
    ) -> Option<ControlEffect>
    where
        M: TextMeasurer,
        S: TextShaper,
        FM: FontMetrics<Handle = S::Handle>,
        R: FontResolver<Handle = S::Handle>,
    {
        let effect = self.controls.apply_file_selection(key, files)?;
        self.form_diagnostics.clear();
        self.controls.clear_validation();
        self.reflow_controls(pipeline)?;
        Some(effect)
    }

    pub fn form_diagnostics(&self) -> &[FormDiagnostic] {
        &self.form_diagnostics
    }

    pub fn take_form_lifecycle_events(&mut self) -> Vec<FormLifecycleEvent> {
        std::mem::take(&mut self.form_lifecycle_events)
    }

    /// Execute script-style `requestSubmit()` through validation, submit,
    /// mutable form-data dispatch, and transactional navigation.
    pub fn request_submit<F, D, M, S, FM, R>(
        &mut self,
        form_index: usize,
        submitter_key: Option<&str>,
        dispatch: D,
        pipeline: &BrowserPagePipeline<'_, M, S, FM, R>,
        fetcher: &F,
    ) -> Result<(), BrowserLoadError>
    where
        F: BrowserResourceFetcher,
        D: FnMut(&mut FormLifecycleEvent),
        M: TextMeasurer,
        S: TextShaper,
        FM: FontMetrics<Handle = S::Handle>,
        R: FontResolver<Handle = S::Handle>,
    {
        let page = self
            .viewport
            .as_ref()
            .map(BrowserViewport::page)
            .ok_or_else(|| BrowserLoadError::Form {
                message: "requestSubmit requires a loaded page".into(),
            })?;
        let outcome = dispatch_request_submit(
            &page.document,
            &self.controls,
            form_index,
            submitter_key,
            &page.final_url,
            dispatch,
        )
        .map_err(form_load_error)?;
        self.apply_form_dispatch(outcome, pipeline, fetcher)
    }

    /// Dispatch a cancelable script-style form reset before mutating controls.
    pub fn request_form_reset<D, M, S, FM, R>(
        &mut self,
        form_index: usize,
        dispatch: D,
        pipeline: &BrowserPagePipeline<'_, M, S, FM, R>,
    ) -> Result<(), BrowserLoadError>
    where
        D: FnMut(&mut FormLifecycleEvent),
        M: TextMeasurer,
        S: TextShaper,
        FM: FontMetrics<Handle = S::Handle>,
        R: FontResolver<Handle = S::Handle>,
    {
        let page = self
            .viewport
            .as_ref()
            .map(BrowserViewport::page)
            .ok_or_else(|| BrowserLoadError::Form {
                message: "form reset requires a loaded page".into(),
            })?;
        let outcome =
            dispatch_form_reset(&page.document, form_index, dispatch).map_err(form_load_error)?;
        self.apply_form_dispatch_without_navigation(outcome, pipeline)
    }

    /// Run non-interactive `checkValidity()` and retain its invalid events.
    pub fn check_form_validity<D>(
        &mut self,
        form_index: usize,
        dispatch: D,
    ) -> Result<bool, BrowserLoadError>
    where
        D: FnMut(&mut FormLifecycleEvent),
    {
        let page = self
            .viewport
            .as_ref()
            .map(BrowserViewport::page)
            .ok_or_else(|| BrowserLoadError::Form {
                message: "form validation requires a loaded page".into(),
            })?;
        let report =
            check_planned_form_validity(&page.document, &self.controls, form_index, dispatch)
                .map_err(form_load_error)?;
        let valid = report.valid;
        self.form_lifecycle_events.extend(report.events);
        Ok(valid)
    }

    /// Run interactive `reportValidity()` and project only diagnostics whose
    /// cancelable invalid events were not prevented.
    pub fn report_form_validity<D, M, S, FM, R>(
        &mut self,
        form_index: usize,
        dispatch: D,
        pipeline: &BrowserPagePipeline<'_, M, S, FM, R>,
    ) -> Result<bool, BrowserLoadError>
    where
        D: FnMut(&mut FormLifecycleEvent),
        M: TextMeasurer,
        S: TextShaper,
        FM: FontMetrics<Handle = S::Handle>,
        R: FontResolver<Handle = S::Handle>,
    {
        let page = self
            .viewport
            .as_ref()
            .map(BrowserViewport::page)
            .ok_or_else(|| BrowserLoadError::Form {
                message: "form validation requires a loaded page".into(),
            })?;
        let report =
            report_planned_form_validity(&page.document, &self.controls, form_index, dispatch)
                .map_err(form_load_error)?;
        let valid = report.valid;
        let diagnostics = report.reportable_diagnostics();
        self.form_lifecycle_events.extend(report.events);
        self.controls.clear_validation();
        for diagnostic in &diagnostics {
            if let Some(key) = &diagnostic.key {
                self.controls.set_invalid(key, diagnostic.message.clone());
            }
        }
        self.controls.focus_first_invalid();
        self.form_diagnostics = diagnostics;
        self.reflow_controls(pipeline);
        Ok(valid)
    }

    pub fn hovered_control_key(&self, viewport_x: f64, viewport_y: f64) -> Option<&str> {
        self.viewport
            .as_ref()?
            .hit_test_control(viewport_x, viewport_y)
            .map(|control| control.key.as_str())
    }

    pub fn activate_control<M, S, FM, R>(
        &mut self,
        viewport_x: f64,
        viewport_y: f64,
        pipeline: &BrowserPagePipeline<'_, M, S, FM, R>,
    ) -> Option<ControlEffect>
    where
        M: TextMeasurer,
        S: TextShaper,
        FM: FontMetrics<Handle = S::Handle>,
        R: FontResolver<Handle = S::Handle>,
    {
        let (region, x, y) = self
            .viewport
            .as_ref()?
            .control_local_point(viewport_x, viewport_y)?;
        let effect = if region.label_activation {
            self.controls.pointer_activate(&region.key)?
        } else if region.kind.accepts_text() {
            self.controls.pointer_place(
                &region.key,
                x - CONTROL_TEXT_METRICS.inset_x,
                y - CONTROL_TEXT_METRICS.inset_y,
                CONTROL_TEXT_METRICS,
            )?
        } else {
            self.controls.pointer_activate(&region.key)?
        };
        self.reflow_controls(pipeline)?;
        Some(effect)
    }

    /// Place a text caret using viewport coordinates. Hosts use the same
    /// method for mouse, touch, and stylus presses.
    pub fn control_pointer_down<M, S, FM, R>(
        &mut self,
        viewport_x: f64,
        viewport_y: f64,
        pipeline: &BrowserPagePipeline<'_, M, S, FM, R>,
    ) -> Option<ControlEffect>
    where
        M: TextMeasurer,
        S: TextShaper,
        FM: FontMetrics<Handle = S::Handle>,
        R: FontResolver<Handle = S::Handle>,
    {
        let (region, x, y) = self
            .viewport
            .as_ref()?
            .control_local_point(viewport_x, viewport_y)?;
        if region.label_activation || !region.kind.accepts_text() {
            return self.activate_control(viewport_x, viewport_y, pipeline);
        }
        let effect = self.controls.pointer_place(
            &region.key,
            x - CONTROL_TEXT_METRICS.inset_x,
            y - CONTROL_TEXT_METRICS.inset_y,
            CONTROL_TEXT_METRICS,
        )?;
        self.reflow_controls(pipeline)?;
        Some(effect)
    }

    pub fn control_pointer_down_with_click_count<M, S, FM, R>(
        &mut self,
        viewport_x: f64,
        viewport_y: f64,
        click_count: u8,
        pipeline: &BrowserPagePipeline<'_, M, S, FM, R>,
    ) -> Option<ControlEffect>
    where
        M: TextMeasurer,
        S: TextShaper,
        FM: FontMetrics<Handle = S::Handle>,
        R: FontResolver<Handle = S::Handle>,
    {
        let (region, x, y) = self
            .viewport
            .as_ref()?
            .control_local_point(viewport_x, viewport_y)?;
        if region.label_activation || !region.kind.accepts_text() {
            return self.activate_control(viewport_x, viewport_y, pipeline);
        }
        let effect = self.controls.pointer_select(
            &region.key,
            x - CONTROL_TEXT_METRICS.inset_x,
            y - CONTROL_TEXT_METRICS.inset_y,
            CONTROL_TEXT_METRICS,
            click_count,
        )?;
        self.reflow_controls(pipeline)?;
        Some(effect)
    }

    /// Continue selection from the most recent pointer press. Coordinates
    /// outside the control clamp to the nearest scalar position.
    pub fn control_pointer_drag<M, S, FM, R>(
        &mut self,
        viewport_x: f64,
        viewport_y: f64,
        pipeline: &BrowserPagePipeline<'_, M, S, FM, R>,
    ) -> Option<ControlEffect>
    where
        M: TextMeasurer,
        S: TextShaper,
        FM: FontMetrics<Handle = S::Handle>,
        R: FontResolver<Handle = S::Handle>,
    {
        let key = self.controls.focused_key()?.to_string();
        let viewport = self.viewport.as_ref()?;
        let region = viewport
            .page()
            .paint
            .controls
            .iter()
            .find(|region| region.key == key)?
            .clone();
        let content_y = if region.fixed {
            viewport_y
        } else {
            viewport_y + viewport.scroll_state().offset_y()
        };
        let effect = self.controls.pointer_drag_autoscroll(
            viewport_x - region.x - CONTROL_TEXT_METRICS.inset_x,
            content_y - region.y - CONTROL_TEXT_METRICS.inset_y,
            (region.width - CONTROL_TEXT_METRICS.inset_x * 2.0).max(0.0),
            (region.height - CONTROL_TEXT_METRICS.inset_y * 2.0).max(0.0),
            CONTROL_TEXT_METRICS,
        )?;
        self.reflow_controls(pipeline)?;
        Some(effect)
    }

    pub fn control_pointer_up(&mut self) {
        self.controls.pointer_release();
    }

    /// Read selected text for a host clipboard. Password values are rejected
    /// by the shared model before they can cross the host boundary.
    pub fn control_copy(&self) -> Option<String> {
        self.controls.copy_selection()
    }

    pub fn control_copy_payload(&self) -> Option<ControlClipboardPayload> {
        self.controls.copy_selection_payload()
    }

    pub fn control_cut<M, S, FM, R>(
        &mut self,
        pipeline: &BrowserPagePipeline<'_, M, S, FM, R>,
    ) -> Option<String>
    where
        M: TextMeasurer,
        S: TextShaper,
        FM: FontMetrics<Handle = S::Handle>,
        R: FontResolver<Handle = S::Handle>,
    {
        let cut = self.controls.cut_selection()?;
        self.form_diagnostics.clear();
        self.controls.clear_validation();
        self.reflow_controls(pipeline)?;
        Some(cut.text)
    }

    pub fn control_paste<M, S, FM, R>(
        &mut self,
        text: &str,
        pipeline: &BrowserPagePipeline<'_, M, S, FM, R>,
    ) -> Option<ControlEffect>
    where
        M: TextMeasurer,
        S: TextShaper,
        FM: FontMetrics<Handle = S::Handle>,
        R: FontResolver<Handle = S::Handle>,
    {
        let effect = self.controls.paste_text(text)?;
        self.form_diagnostics.clear();
        self.controls.clear_validation();
        self.reflow_controls(pipeline)?;
        Some(effect)
    }

    pub fn control_paste_payload<M, S, FM, R>(
        &mut self,
        payload: &ControlClipboardPayload,
        pipeline: &BrowserPagePipeline<'_, M, S, FM, R>,
    ) -> Option<ControlEffect>
    where
        M: TextMeasurer,
        S: TextShaper,
        FM: FontMetrics<Handle = S::Handle>,
        R: FontResolver<Handle = S::Handle>,
    {
        let effect = self.controls.paste_payload(payload)?;
        self.form_diagnostics.clear();
        self.controls.clear_validation();
        self.reflow_controls(pipeline)?;
        Some(effect)
    }

    pub fn control_accessibility_action<M, S, FM, R>(
        &mut self,
        action: ControlAccessibilityAction,
        pipeline: &BrowserPagePipeline<'_, M, S, FM, R>,
    ) -> Option<ControlEffect>
    where
        M: TextMeasurer,
        S: TextShaper,
        FM: FontMetrics<Handle = S::Handle>,
        R: FontResolver<Handle = S::Handle>,
    {
        let effect = self.controls.accessibility_action(action)?;
        self.record_control_effect_events(&effect);
        self.form_diagnostics.clear();
        self.controls.clear_validation();
        self.reflow_controls(pipeline)?;
        Some(effect)
    }

    /// Apply an accessibility action and execute submit/reset activation through
    /// the same planner used by pointer and keyboard input.
    pub fn control_accessibility_action_and_submit<F, M, S, FM, R>(
        &mut self,
        action: ControlAccessibilityAction,
        pipeline: &BrowserPagePipeline<'_, M, S, FM, R>,
        fetcher: &F,
    ) -> Result<Option<ControlEffect>, BrowserLoadError>
    where
        F: BrowserResourceFetcher,
        M: TextMeasurer,
        S: TextShaper,
        FM: FontMetrics<Handle = S::Handle>,
        R: FontResolver<Handle = S::Handle>,
    {
        let effect = self.controls.accessibility_action(action);
        if let Some(effect) = &effect {
            self.record_control_effect_events(effect);
        }
        if let Some(ControlEffect::Activated(key)) = &effect {
            let outcome = self.dispatch_form_activation(key, |_| {})?;
            self.apply_form_dispatch(outcome, pipeline, fetcher)?;
        } else if effect.is_some() {
            self.form_diagnostics.clear();
            self.controls.clear_validation();
            self.reflow_controls(pipeline);
        }
        Ok(effect)
    }

    /// Advance caret animation from a host-provided monotonic duration.
    pub fn control_advance_caret_blink(&mut self, elapsed_ms: u64) -> bool {
        let changed = self.controls.advance_caret_blink(elapsed_ms);
        if changed {
            self.refresh_control_editor_presentation();
        }
        changed
    }

    pub fn focused_ime_candidate_rect(&mut self) -> Option<ControlRect> {
        let key = self.controls.focused_key()?.to_string();
        let mut rect = self
            .refresh_control_editor_presentation()
            .into_iter()
            .find(|presentation| presentation.key == key)?
            .candidate_rect?;
        let viewport = self.viewport.as_ref()?;
        let region = viewport
            .page()
            .paint
            .controls
            .iter()
            .find(|region| region.key == key)?;
        if !region.fixed {
            rect.y -= viewport.scroll_state().offset_y();
        }
        Some(rect)
    }

    pub fn activate_control_and_submit<F, M, S, FM, R>(
        &mut self,
        viewport_x: f64,
        viewport_y: f64,
        pipeline: &BrowserPagePipeline<'_, M, S, FM, R>,
        fetcher: &F,
    ) -> Result<Option<ControlEffect>, BrowserLoadError>
    where
        F: BrowserResourceFetcher,
        M: TextMeasurer,
        S: TextShaper,
        FM: FontMetrics<Handle = S::Handle>,
        R: FontResolver<Handle = S::Handle>,
    {
        let Some((region, x, y)) = self
            .viewport
            .as_ref()
            .and_then(|viewport| viewport.control_local_point(viewport_x, viewport_y))
        else {
            return Ok(None);
        };
        let key = region.key.clone();
        let effect = if region.label_activation {
            self.controls.pointer_activate(&key)
        } else if region.kind.accepts_text() {
            self.controls.pointer_place(
                &key,
                x - CONTROL_TEXT_METRICS.inset_x,
                y - CONTROL_TEXT_METRICS.inset_y,
                CONTROL_TEXT_METRICS,
            )
        } else {
            self.controls.pointer_activate(&key)
        };
        let Some(effect) = effect else {
            return Ok(None);
        };
        if matches!(effect, ControlEffect::Activated(_)) {
            let image_coordinates = self
                .controls
                .binding(&key)
                .is_some_and(|binding| binding.control_type == "image")
                .then(|| {
                    if region.label_activation {
                        ImageSubmitCoordinates::KEYBOARD
                    } else {
                        ImageSubmitCoordinates::from_local_point(x, y)
                    }
                });
            let outcome = self.dispatch_form_activation_with_image_coordinates(
                &key,
                image_coordinates.unwrap_or(ImageSubmitCoordinates::KEYBOARD),
                |_| {},
            )?;
            self.apply_form_dispatch(outcome, pipeline, fetcher)?;
        } else {
            self.form_diagnostics.clear();
            self.controls.clear_validation();
            self.reflow_controls(pipeline);
        }
        Ok(Some(effect))
    }

    pub fn control_key_down_and_submit<F, M, S, FM, R>(
        &mut self,
        key: ControlKey,
        pipeline: &BrowserPagePipeline<'_, M, S, FM, R>,
        fetcher: &F,
    ) -> Result<Option<ControlEffect>, BrowserLoadError>
    where
        F: BrowserResourceFetcher,
        M: TextMeasurer,
        S: TextShaper,
        FM: FontMetrics<Handle = S::Handle>,
        R: FontResolver<Handle = S::Handle>,
    {
        self.control_key_down_with_shift_and_submit(key, false, pipeline, fetcher)
    }

    pub fn control_key_down_with_shift_and_submit<F, M, S, FM, R>(
        &mut self,
        key: ControlKey,
        shift: bool,
        pipeline: &BrowserPagePipeline<'_, M, S, FM, R>,
        fetcher: &F,
    ) -> Result<Option<ControlEffect>, BrowserLoadError>
    where
        F: BrowserResourceFetcher,
        M: TextMeasurer,
        S: TextShaper,
        FM: FontMetrics<Handle = S::Handle>,
        R: FontResolver<Handle = S::Handle>,
    {
        let focused_key = self.controls.focused_key().map(str::to_owned);
        let focused_accepts_implicit = focused_key
            .as_deref()
            .and_then(|focused| self.controls.control(focused))
            .is_some_and(|control| {
                control.kind.accepts_text() && control.kind.name() != "textarea"
            });
        let effect = self.controls.key_down_with_shift(key, shift);
        if let Some(effect) = &effect {
            self.record_control_effect_events(effect);
        }
        let activation = match &effect {
            Some(ControlEffect::Activated(activated)) => {
                Some(self.dispatch_form_activation(activated, |_| {})?)
            }
            Some(
                ControlEffect::SuggestionCommitted { .. }
                | ControlEffect::SuggestionPickerChanged { .. },
            ) => None,
            _ if key == ControlKey::Enter && focused_accepts_implicit => {
                let focused = focused_key.as_deref().expect("focused key checked above");
                Some(self.dispatch_implicit_form_activation(focused, |_| {})?)
            }
            _ => None,
        };
        if let Some(outcome) = activation {
            self.apply_form_dispatch(outcome, pipeline, fetcher)?;
        } else if effect.is_some() {
            self.form_diagnostics.clear();
            self.controls.clear_validation();
            self.reflow_controls(pipeline);
        }
        Ok(effect)
    }

    fn dispatch_form_activation<F>(
        &self,
        key: &str,
        dispatch: F,
    ) -> Result<FormDispatchOutcome, BrowserLoadError>
    where
        F: FnMut(&mut FormLifecycleEvent),
    {
        self.dispatch_form_activation_with_image_coordinates(
            key,
            ImageSubmitCoordinates::KEYBOARD,
            dispatch,
        )
    }

    fn dispatch_form_activation_with_image_coordinates<F>(
        &self,
        key: &str,
        image_coordinates: ImageSubmitCoordinates,
        dispatch: F,
    ) -> Result<FormDispatchOutcome, BrowserLoadError>
    where
        F: FnMut(&mut FormLifecycleEvent),
    {
        let page = self
            .viewport
            .as_ref()
            .map(BrowserViewport::page)
            .ok_or_else(|| BrowserLoadError::Form {
                message: "form activation requires a loaded page".into(),
            })?;
        dispatch_activation_with_image_coordinates(
            &page.document,
            &self.controls,
            key,
            &page.final_url,
            image_coordinates,
            dispatch,
        )
        .map_err(form_load_error)
    }

    fn dispatch_implicit_form_activation<F>(
        &self,
        focused_key: &str,
        dispatch: F,
    ) -> Result<FormDispatchOutcome, BrowserLoadError>
    where
        F: FnMut(&mut FormLifecycleEvent),
    {
        let page = self
            .viewport
            .as_ref()
            .map(BrowserViewport::page)
            .ok_or_else(|| BrowserLoadError::Form {
                message: "form activation requires a loaded page".into(),
            })?;
        dispatch_implicit_submission(
            &page.document,
            &self.controls,
            focused_key,
            &page.final_url,
            dispatch,
        )
        .map_err(form_load_error)
    }

    fn apply_form_dispatch_without_navigation<M, S, FM, R>(
        &mut self,
        outcome: FormDispatchOutcome,
        pipeline: &BrowserPagePipeline<'_, M, S, FM, R>,
    ) -> Result<(), BrowserLoadError>
    where
        M: TextMeasurer,
        S: TextShaper,
        FM: FontMetrics<Handle = S::Handle>,
        R: FontResolver<Handle = S::Handle>,
    {
        self.form_lifecycle_events.extend(outcome.events);
        self.form_diagnostics.clear();
        self.controls.clear_validation();
        match outcome.activation {
            FormActivation::None => {}
            FormActivation::Reset {
                form_id,
                form_index,
            } => {
                self.controls
                    .reset_form(form_id.as_deref(), Some(form_index));
            }
            _ => {
                return Err(BrowserLoadError::Form {
                    message: "non-navigation form dispatch produced navigation".into(),
                });
            }
        }
        self.reflow_controls(pipeline);
        Ok(())
    }

    fn apply_form_dispatch<F, M, S, FM, R>(
        &mut self,
        outcome: FormDispatchOutcome,
        pipeline: &BrowserPagePipeline<'_, M, S, FM, R>,
        fetcher: &F,
    ) -> Result<(), BrowserLoadError>
    where
        F: BrowserResourceFetcher,
        M: TextMeasurer,
        S: TextShaper,
        FM: FontMetrics<Handle = S::Handle>,
        R: FontResolver<Handle = S::Handle>,
    {
        let reportable_diagnostics = outcome.reportable_diagnostics();
        self.form_lifecycle_events.extend(outcome.events);
        match outcome.activation {
            FormActivation::None => {
                self.form_diagnostics.clear();
                self.controls.clear_validation();
                self.reflow_controls(pipeline);
            }
            FormActivation::Reset {
                form_id,
                form_index,
            } => {
                self.form_diagnostics.clear();
                self.controls.clear_validation();
                self.controls
                    .reset_form(form_id.as_deref(), Some(form_index));
                self.reflow_controls(pipeline);
            }
            FormActivation::Invalid(_) => {
                self.controls.clear_validation();
                for diagnostic in &reportable_diagnostics {
                    if let Some(key) = &diagnostic.key {
                        self.controls.set_invalid(key, diagnostic.message.clone());
                    }
                }
                self.controls.focus_first_invalid();
                self.form_diagnostics = reportable_diagnostics;
                self.reflow_controls(pipeline);
            }
            FormActivation::Navigate(navigation) => {
                self.form_diagnostics.clear();
                self.controls.clear_validation();
                self.execute_form_navigation(navigation, pipeline, fetcher)?;
            }
        }
        Ok(())
    }

    fn execute_form_navigation<F, M, S, FM, R>(
        &mut self,
        navigation: FormNavigation,
        pipeline: &BrowserPagePipeline<'_, M, S, FM, R>,
        fetcher: &F,
    ) -> Result<(), BrowserLoadError>
    where
        F: BrowserResourceFetcher,
        M: TextMeasurer,
        S: TextShaper,
        FM: FontMetrics<Handle = S::Handle>,
        R: FontResolver<Handle = S::Handle>,
    {
        let departing_state = self.current_form_history_state();
        let request = BrowserFetchRequest {
            method: match navigation.method {
                FormMethod::Get => BrowserFetchMethod::Get,
                FormMethod::Post => BrowserFetchMethod::Post,
            },
            url: navigation.url,
            content_type: navigation.content_type,
            body: navigation.body,
        };
        let mut history = self.history.clone();
        history.navigate(request.url.clone());
        let page =
            pipeline.load_request_pending_with_visited(&request, fetcher, &self.visited_links)?;
        let mut visited_links = self.visited_links.clone();
        let _ = visited_links.record(&page.final_url);
        let controls = BrowserControlModel::from_render_tree(&page.render_tree);
        history.replace_current(page.final_url.clone());
        self.viewport = Some(BrowserViewport::new(page, self.viewport_height));
        self.history = history;
        self.visited_links = visited_links;
        self.controls = controls;
        if let Some((url, snapshot)) = departing_state {
            self.remember_form_history_state(url, snapshot);
        }
        self.form_diagnostics.clear();
        self.navigation_id = self.navigation_id.wrapping_add(1).max(1);
        self.refresh_control_editor_presentation();
        for request in self.pending_subresource_requests() {
            let completion = request.resolve(fetcher);
            self.complete_subresource(completion, pipeline);
        }
        Ok(())
    }

    pub fn focus_control<M, S, FM, R>(
        &mut self,
        reverse: bool,
        pipeline: &BrowserPagePipeline<'_, M, S, FM, R>,
    ) -> Option<ControlEffect>
    where
        M: TextMeasurer,
        S: TextShaper,
        FM: FontMetrics<Handle = S::Handle>,
        R: FontResolver<Handle = S::Handle>,
    {
        let effect = self.controls.focus_next(reverse)?;
        self.reflow_controls(pipeline)?;
        Some(effect)
    }

    pub fn control_key_down<M, S, FM, R>(
        &mut self,
        key: ControlKey,
        pipeline: &BrowserPagePipeline<'_, M, S, FM, R>,
    ) -> Option<ControlEffect>
    where
        M: TextMeasurer,
        S: TextShaper,
        FM: FontMetrics<Handle = S::Handle>,
        R: FontResolver<Handle = S::Handle>,
    {
        self.control_key_down_with_shift(key, false, pipeline)
    }

    pub fn control_key_down_with_shift<M, S, FM, R>(
        &mut self,
        key: ControlKey,
        shift: bool,
        pipeline: &BrowserPagePipeline<'_, M, S, FM, R>,
    ) -> Option<ControlEffect>
    where
        M: TextMeasurer,
        S: TextShaper,
        FM: FontMetrics<Handle = S::Handle>,
        R: FontResolver<Handle = S::Handle>,
    {
        let effect = self.controls.key_down_with_shift(key, shift)?;
        self.record_control_effect_events(&effect);
        self.form_diagnostics.clear();
        self.controls.clear_validation();
        self.reflow_controls(pipeline)?;
        Some(effect)
    }

    pub fn control_text_input<M, S, FM, R>(
        &mut self,
        text: &str,
        pipeline: &BrowserPagePipeline<'_, M, S, FM, R>,
    ) -> Option<ControlEffect>
    where
        M: TextMeasurer,
        S: TextShaper,
        FM: FontMetrics<Handle = S::Handle>,
        R: FontResolver<Handle = S::Handle>,
    {
        let effect = self.controls.text_input(text)?;
        self.form_diagnostics.clear();
        self.controls.clear_validation();
        self.reflow_controls(pipeline)?;
        Some(effect)
    }

    pub fn control_set_selection<M, S, FM, R>(
        &mut self,
        key: &str,
        anchor: usize,
        focus: usize,
        pipeline: &BrowserPagePipeline<'_, M, S, FM, R>,
    ) -> Option<ControlEffect>
    where
        M: TextMeasurer,
        S: TextShaper,
        FM: FontMetrics<Handle = S::Handle>,
        R: FontResolver<Handle = S::Handle>,
    {
        let effect = self.controls.set_selection(key, anchor, focus)?;
        self.reflow_controls(pipeline)?;
        Some(effect)
    }

    pub fn control_update_composition<M, S, FM, R>(
        &mut self,
        text: &str,
        pipeline: &BrowserPagePipeline<'_, M, S, FM, R>,
    ) -> Option<ControlEffect>
    where
        M: TextMeasurer,
        S: TextShaper,
        FM: FontMetrics<Handle = S::Handle>,
        R: FontResolver<Handle = S::Handle>,
    {
        let effect = self.controls.update_composition(text)?;
        self.reflow_controls(pipeline)?;
        Some(effect)
    }

    pub fn control_commit_composition<M, S, FM, R>(
        &mut self,
        pipeline: &BrowserPagePipeline<'_, M, S, FM, R>,
    ) -> Option<ControlEffect>
    where
        M: TextMeasurer,
        S: TextShaper,
        FM: FontMetrics<Handle = S::Handle>,
        R: FontResolver<Handle = S::Handle>,
    {
        let effect = self.controls.commit_composition()?;
        self.form_diagnostics.clear();
        self.controls.clear_validation();
        self.reflow_controls(pipeline)?;
        Some(effect)
    }

    pub fn control_cancel_composition<M, S, FM, R>(
        &mut self,
        pipeline: &BrowserPagePipeline<'_, M, S, FM, R>,
    ) -> Option<ControlEffect>
    where
        M: TextMeasurer,
        S: TextShaper,
        FM: FontMetrics<Handle = S::Handle>,
        R: FontResolver<Handle = S::Handle>,
    {
        let effect = self.controls.cancel_composition()?;
        self.reflow_controls(pipeline)?;
        Some(effect)
    }

    /// Rebuild retained editor overlays and return their host-facing
    /// geometry, including the IME candidate rectangle and accessibility
    /// description.
    fn refresh_control_editor_presentation(&mut self) -> Vec<ControlEditorPresentation> {
        let regions = self
            .viewport
            .as_ref()
            .map(|viewport| viewport.page.paint.controls.clone())
            .unwrap_or_default();
        let mut presentations = Vec::new();
        let mut overlays = Vec::new();
        for region in regions {
            let bounds = ControlRect {
                x: region.x,
                y: region.y,
                width: region.width,
                height: region.height,
            };
            let Some(presentation) =
                self.controls
                    .editor_presentation(&region.key, bounds, CONTROL_TEXT_METRICS)
            else {
                continue;
            };
            let mut children = Vec::new();
            for rect in &presentation.selection {
                if let Some(rect) = clipped_editor_rect(*rect, presentation.viewport) {
                    children.push(PaintInstruction::Rect(PaintRect::filled(
                        rect.x,
                        rect.y,
                        rect.width,
                        rect.height,
                        "rgba(37, 99, 235, 0.32)",
                    )));
                }
            }
            for rect in &presentation.composition_underlines {
                if let Some(rect) = clipped_editor_rect(*rect, presentation.viewport) {
                    children.push(PaintInstruction::Rect(PaintRect::filled(
                        rect.x,
                        rect.y,
                        rect.width,
                        rect.height,
                        "#2563eb",
                    )));
                }
            }
            if let Some(rect) = presentation
                .caret
                .and_then(|rect| clipped_editor_rect(rect, presentation.viewport))
            {
                children.push(PaintInstruction::Rect(PaintRect::filled(
                    rect.x,
                    rect.y,
                    rect.width,
                    rect.height,
                    "#111827",
                )));
            }
            if presentation.invalid_message.is_some() {
                children.push(PaintInstruction::Rect(PaintRect {
                    base: PaintBase::default(),
                    x: region.x + 1.0,
                    y: region.y + 1.0,
                    width: (region.width - 2.0).max(0.0),
                    height: (region.height - 2.0).max(0.0),
                    fill: None,
                    stroke: Some("#dc2626".into()),
                    stroke_width: Some(2.0),
                    corner_radius: Some(3.0),
                    stroke_dash: None,
                    stroke_dash_offset: None,
                }));
            }
            let mut metadata = std::collections::HashMap::new();
            if region.fixed {
                metadata.insert("layout.position".into(), "fixed".into());
            }
            overlays.push(PaintInstruction::Group(PaintGroup {
                base: PaintBase {
                    id: Some(format!("{EDITOR_OVERLAY_PREFIX}{}", region.key)),
                    metadata: (!metadata.is_empty()).then_some(metadata),
                },
                children,
                transform: None,
                opacity: None,
            }));
            presentations.push(presentation);
        }
        if let Some(viewport) = self.viewport.as_mut() {
            viewport.page.paint.scene.instructions.retain(|instruction| {
                !matches!(instruction,
                    PaintInstruction::Group(group)
                        if group.base.id.as_deref().is_some_and(|id| id.starts_with(EDITOR_OVERLAY_PREFIX)))
            });
            viewport.page.paint.scene.instructions.extend(overlays);
        }
        presentations
    }

    fn reflow_controls<M, S, FM, R>(
        &mut self,
        pipeline: &BrowserPagePipeline<'_, M, S, FM, R>,
    ) -> Option<()>
    where
        M: TextMeasurer,
        S: TextShaper,
        FM: FontMetrics<Handle = S::Handle>,
        R: FontResolver<Handle = S::Handle>,
    {
        let mut current = self.viewport.as_ref()?.page().clone();
        self.controls.sync_render_tree(&mut current.render_tree);
        let updated = pipeline.reflow_retained_with_visited(&current, &self.visited_links);
        self.viewport
            .as_mut()?
            .reflow_page(updated, self.viewport_height);
        self.refresh_control_editor_presentation();
        Some(())
    }

    fn current_form_history_state(&self) -> Option<(String, ControlStateSnapshot)> {
        Some((
            self.history.current_url()?.to_string(),
            self.controls.capture_state(ControlStatePrivacy::Public),
        ))
    }

    fn remember_form_history_state(&mut self, url: String, snapshot: ControlStateSnapshot) {
        if let Some(position) = self
            .form_history_states
            .iter()
            .position(|(candidate, _)| candidate == &url)
        {
            self.form_history_states.remove(position);
        }
        self.form_history_states.push((url, snapshot));
        if self.form_history_states.len() > FORM_HISTORY_STATE_LIMIT {
            self.form_history_states.remove(0);
        }
    }

    /// Recompose the retained document for a new layout viewport without
    /// refetching or reparsing the page. Inline image resources continue to use
    /// the browser-owned fetch seam, and failures remain recoverable paint
    /// fallbacks just as they are during the initial page load.
    pub fn reflow<'session, F, M, S, FM, R>(
        &'session mut self,
        pipeline: &BrowserPagePipeline<'_, M, S, FM, R>,
        _fetcher: &F,
        viewport_height: f64,
    ) -> Option<&'session BrowserViewport>
    where
        F: BrowserResourceFetcher,
        M: TextMeasurer,
        S: TextShaper,
        FM: FontMetrics<Handle = S::Handle>,
        R: FontResolver<Handle = S::Handle>,
    {
        let page = pipeline
            .reflow_retained_with_visited(self.viewport.as_ref()?.page(), &self.visited_links);
        self.viewport_height = finite_non_negative(viewport_height);
        self.viewport
            .as_mut()?
            .reflow_page(page, self.viewport_height);
        self.viewport.as_ref()
    }

    pub fn execute<'session, F, M, S, FM, R>(
        &'session mut self,
        navigation: BrowserNavigation,
        pipeline: &BrowserPagePipeline<'_, M, S, FM, R>,
        fetcher: &F,
    ) -> Result<Option<&'session BrowserViewport>, BrowserLoadError>
    where
        F: BrowserResourceFetcher,
        M: TextMeasurer,
        S: TextShaper,
        FM: FontMetrics<Handle = S::Handle>,
        R: FontResolver<Handle = S::Handle>,
    {
        let update = self.begin_execute(navigation, pipeline, fetcher)?;
        for request in update.requests {
            let completion = request.resolve(fetcher);
            let _ = self.complete_subresource(completion, pipeline);
        }
        Ok(update.viewport_changed.then(|| {
            self.viewport
                .as_ref()
                .expect("committed navigation must retain a viewport")
        }))
    }

    /// Commit a document and emit its image requests without fetching them.
    pub fn begin_execute<F, M, S, FM, R>(
        &mut self,
        navigation: BrowserNavigation,
        pipeline: &BrowserPagePipeline<'_, M, S, FM, R>,
        document_fetcher: &F,
    ) -> Result<BrowserNavigationUpdate, BrowserLoadError>
    where
        F: BrowserResourceFetcher,
        M: TextMeasurer,
        S: TextShaper,
        FM: FontMetrics<Handle = S::Handle>,
        R: FontResolver<Handle = S::Handle>,
    {
        let restores_form_state = matches!(
            &navigation,
            BrowserNavigation::Back | BrowserNavigation::Forward
        );
        let departing_state = (!matches!(&navigation, BrowserNavigation::Reload))
            .then(|| self.current_form_history_state())
            .flatten();
        let mut history = self.history.clone();
        let requested_url = match navigation {
            BrowserNavigation::Navigate(url) => Some(history.navigate(url).to_string()),
            BrowserNavigation::Back => history.back().map(str::to_owned),
            BrowserNavigation::Forward => history.forward().map(str::to_owned),
            BrowserNavigation::Home => Some(history.home().to_string()),
            BrowserNavigation::Reload => history.reload().map(str::to_owned),
        };
        let Some(requested_url) = requested_url else {
            return Ok(BrowserNavigationUpdate::default());
        };

        let page = pipeline.load_pending_with_visited(
            &requested_url,
            document_fetcher,
            &self.visited_links,
        )?;
        let cancelled = self.pending_subresource_requests();
        let mut visited_links = self.visited_links.clone();
        let _ = visited_links.record(&page.final_url);
        let mut controls = BrowserControlModel::from_render_tree(&page.render_tree);
        if restores_form_state {
            if let Some((_, snapshot)) = self
                .form_history_states
                .iter()
                .rev()
                .find(|(url, _)| url == &page.final_url || url == &requested_url)
            {
                controls.restore_state(snapshot);
            }
        }
        history.replace_current(page.final_url.clone());
        if let Some(viewport) = self.viewport.as_mut() {
            viewport.replace_page(page);
        } else {
            self.viewport = Some(BrowserViewport::new(page, self.viewport_height));
        }
        self.history = history;
        self.visited_links = visited_links;
        self.controls = controls;
        if let Some((url, snapshot)) = departing_state {
            self.remember_form_history_state(url, snapshot);
        }
        self.form_diagnostics.clear();
        self.navigation_id = self.navigation_id.wrapping_add(1).max(1);
        self.refresh_control_editor_presentation();
        Ok(BrowserNavigationUpdate {
            viewport_changed: true,
            requests: self.pending_subresource_requests(),
            cancelled,
        })
    }

    /// Apply one completion to the current retained page and recompose paint.
    /// Stale navigation results and duplicate deliveries are harmless no-ops.
    pub fn complete_subresource<M, S, FM, R>(
        &mut self,
        completion: BrowserSubresourceCompletion,
        pipeline: &BrowserPagePipeline<'_, M, S, FM, R>,
    ) -> BrowserSubresourceUpdate
    where
        M: TextMeasurer,
        S: TextShaper,
        FM: FontMetrics<Handle = S::Handle>,
        R: FontResolver<Handle = S::Handle>,
    {
        let pending_before = self.pending_subresource_requests();
        if completion.request.navigation_id != self.navigation_id {
            return BrowserSubresourceUpdate {
                disposition: BrowserSubresourceDisposition::IgnoredStaleNavigation,
                repaint_required: false,
                pending_count: self.pending_subresource_requests().len(),
                requests: Vec::new(),
            };
        }
        let Some(current) = self
            .viewport
            .as_ref()
            .map(|viewport| viewport.page().clone())
        else {
            return BrowserSubresourceUpdate {
                disposition: BrowserSubresourceDisposition::IgnoredStaleNavigation,
                repaint_required: false,
                pending_count: 0,
                requests: Vec::new(),
            };
        };
        let mut updated = current;
        let repaint_required = match completion.request.kind {
            BrowserSubresourceKind::Image => {
                let Some(resource) = updated.image_resources.get(completion.request.ordinal) else {
                    return self.ignored_duplicate_update();
                };
                if resource.url != completion.request.url
                    || !matches!(resource.state, BrowserImageResourceState::Pending)
                {
                    return self.ignored_duplicate_update();
                }
                updated.image_resources[completion.request.ordinal].state = match completion.result
                {
                    Ok(BrowserSubresourcePayload::Image(pixels)) => {
                        BrowserImageResourceState::Ready(pixels)
                    }
                    Err(BrowserSubresourceError::Image(error)) => {
                        BrowserImageResourceState::Failed(error)
                    }
                    _ => return self.ignored_duplicate_update(),
                };
                true
            }
            BrowserSubresourceKind::Stylesheet => {
                let before = active_stylesheet_source_count(&updated.stylesheet_resources);
                let Some(resource) = updated.stylesheet_resources.get(completion.request.ordinal)
                else {
                    return self.ignored_duplicate_update();
                };
                if resource.url.as_deref() != Some(&completion.request.url)
                    || !matches!(resource.state, BrowserStylesheetResourceState::Pending)
                {
                    return self.ignored_duplicate_update();
                }
                match completion.result {
                    Ok(BrowserSubresourcePayload::Stylesheet(source)) => {
                        complete_stylesheet_source(
                            &mut updated.stylesheet_resources,
                            completion.request.ordinal,
                            source,
                            pipeline.viewport.width,
                            pipeline.viewport.height,
                        );
                    }
                    Err(BrowserSubresourceError::Stylesheet(error)) => {
                        updated.stylesheet_resources[completion.request.ordinal].state =
                            BrowserStylesheetResourceState::Failed(error);
                    }
                    _ => return self.ignored_duplicate_update(),
                }
                active_stylesheet_source_count(&updated.stylesheet_resources) != before
            }
        };
        let updated = pipeline.reflow_retained_with_visited(&updated, &self.visited_links);
        if let Some(viewport) = self.viewport.as_mut() {
            viewport.reflow_page(updated, self.viewport_height);
        }
        self.refresh_control_editor_presentation();
        let pending_after = self.pending_subresource_requests();
        let requests = pending_after
            .iter()
            .filter(|request| !pending_before.contains(request))
            .cloned()
            .collect();
        BrowserSubresourceUpdate {
            disposition: BrowserSubresourceDisposition::Applied,
            repaint_required,
            pending_count: pending_after.len(),
            requests,
        }
    }

    fn ignored_duplicate_update(&self) -> BrowserSubresourceUpdate {
        BrowserSubresourceUpdate {
            disposition: BrowserSubresourceDisposition::IgnoredDuplicate,
            repaint_required: false,
            pending_count: self.pending_subresource_requests().len(),
            requests: Vec::new(),
        }
    }

    pub fn activate_link<'session, F, M, S, FM, R>(
        &'session mut self,
        viewport_x: f64,
        viewport_y: f64,
        pipeline: &BrowserPagePipeline<'_, M, S, FM, R>,
        fetcher: &F,
    ) -> Result<Option<&'session BrowserViewport>, BrowserLoadError>
    where
        F: BrowserResourceFetcher,
        M: TextMeasurer,
        S: TextShaper,
        FM: FontMetrics<Handle = S::Handle>,
        R: FontResolver<Handle = S::Handle>,
    {
        let Some(url) = self
            .hovered_link_url(viewport_x, viewport_y)
            .map(str::to_owned)
        else {
            return Ok(None);
        };
        self.execute(BrowserNavigation::Navigate(url), pipeline, fetcher)
    }
}

fn clipped_editor_rect(rect: ControlRect, viewport: ControlRect) -> Option<ControlRect> {
    let x = rect.x.max(viewport.x);
    let y = rect.y.max(viewport.y);
    let right = (rect.x + rect.width).min(viewport.x + viewport.width);
    let bottom = (rect.y + rect.height).min(viewport.y + viewport.height);
    (right > x && bottom > y).then_some(ControlRect {
        x,
        y,
        width: right - x,
        height: bottom - y,
    })
}

fn pending_stylesheet_ordinals(resources: &[BrowserStylesheetResource]) -> Vec<usize> {
    let mut pending = Vec::new();
    for index in resources
        .iter()
        .enumerate()
        .filter_map(|(index, resource)| resource.imported_by.is_none().then_some(index))
    {
        collect_pending_stylesheet_ordinals(resources, index, &mut pending);
    }
    pending
}

fn collect_pending_stylesheet_ordinals(
    resources: &[BrowserStylesheetResource],
    index: usize,
    pending: &mut Vec<usize>,
) {
    let Some(resource) = resources.get(index) else {
        return;
    };
    if matches!(resource.state, BrowserStylesheetResourceState::Pending) {
        pending.push(index);
        return;
    }
    for imported in &resource.imports {
        collect_pending_stylesheet_ordinals(resources, *imported, pending);
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BrowserLoadError {
    Fetch { url: String, message: String },
    HttpStatus { url: String, status: u16 },
    UnsupportedMediaType { url: String, media_type: String },
    Parse { url: String, message: String },
    Form { message: String },
}

impl fmt::Display for BrowserLoadError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Fetch { url, message } => write!(formatter, "failed to fetch {url}: {message}"),
            Self::HttpStatus { url, status } => {
                write!(formatter, "HTTP status {status} while loading {url}")
            }
            Self::UnsupportedMediaType { url, media_type } => {
                write!(formatter, "unsupported media type {media_type} for {url}")
            }
            Self::Parse { url, message } => write!(formatter, "failed to parse {url}: {message}"),
            Self::Form { message } => write!(formatter, "form activation failed: {message}"),
        }
    }
}

impl std::error::Error for BrowserLoadError {}

fn form_load_error(error: FormPlanningError) -> BrowserLoadError {
    BrowserLoadError::Form {
        message: error.to_string(),
    }
}

/// Failure while executing a browser command through a native host.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BrowserCommandError {
    Load(BrowserLoadError),
    Bookmark(BookmarkRepositoryError),
}

impl fmt::Display for BrowserCommandError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Load(error) => error.fmt(formatter),
            Self::Bookmark(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for BrowserCommandError {}

impl From<BrowserLoadError> for BrowserCommandError {
    fn from(error: BrowserLoadError) -> Self {
        Self::Load(error)
    }
}

impl From<BookmarkRepositoryError> for BrowserCommandError {
    fn from(error: BookmarkRepositoryError) -> Self {
        Self::Bookmark(error)
    }
}

/// Reusable layout and font services for loading pages into paint scenes.
pub struct BrowserPagePipeline<'a, M, S, FM, R> {
    pub theme: &'a HtmlTheme,
    pub viewport: HtmlPaintViewport,
    pub measurer: &'a M,
    pub shaper: &'a S,
    pub metrics: &'a FM,
    pub resolver: &'a R,
}

impl<'a, M, S, FM, R> BrowserPagePipeline<'a, M, S, FM, R>
where
    M: TextMeasurer,
    S: TextShaper,
    FM: FontMetrics<Handle = S::Handle>,
    R: FontResolver<Handle = S::Handle>,
{
    pub fn new(
        theme: &'a HtmlTheme,
        viewport: HtmlPaintViewport,
        measurer: &'a M,
        shaper: &'a S,
        metrics: &'a FM,
        resolver: &'a R,
    ) -> Self {
        Self {
            theme,
            viewport,
            measurer,
            shaper,
            metrics,
            resolver,
        }
    }

    /// Fetch and compose one HTML page through a resolved `PaintScene`.
    ///
    /// Missing `Content-Type` is accepted for compatibility with early web
    /// servers. Explicit non-HTML content is left to standalone-image or
    /// raw-text host policies.
    pub fn load<F>(&self, requested_url: &str, fetcher: &F) -> Result<BrowserPage, BrowserLoadError>
    where
        F: BrowserResourceFetcher,
    {
        self.load_with_visited(requested_url, fetcher, &VisitedLinks::new())
    }

    /// Compose a core-owned synthetic document without network access.
    pub fn compose_auxiliary_document(
        &self,
        auxiliary: &BrowserAuxiliaryDocument,
    ) -> Result<BrowserPage, BrowserLoadError> {
        let parsed = parse_html(&auxiliary.html).map_err(|error| BrowserLoadError::Parse {
            url: auxiliary.address.clone(),
            message: error.to_string(),
        })?;
        let document = BrowserDocument::from_document(&parsed);
        let render_tree =
            BrowserRenderTree::from_document_with_document_url(&parsed, &auxiliary.address);
        let stylesheet_resources = stylesheet_resources_for_document(
            &document,
            &auxiliary.address,
            self.viewport.width,
            self.viewport.height,
        );
        let (paint, image_failures, stylesheet_failures) = self.compose(
            &render_tree,
            &[],
            &stylesheet_resources,
            &VisitedLinks::new(),
        );

        Ok(BrowserPage {
            requested_url: auxiliary.address.clone(),
            final_url: auxiliary.address.clone(),
            status: 200,
            source: auxiliary.html.clone(),
            document,
            render_tree,
            paint,
            image_failures,
            image_resources: Vec::new(),
            stylesheet_failures,
            stylesheet_resources,
        })
    }

    /// Fetch and compose one HTML page against existing session link state.
    /// The final response URL is included prospectively for self-links, but
    /// the caller's set is never mutated; `BrowserSession` commits it only
    /// after the complete page load succeeds.
    pub fn load_with_visited<F>(
        &self,
        requested_url: &str,
        fetcher: &F,
        visited_links: &VisitedLinks,
    ) -> Result<BrowserPage, BrowserLoadError>
    where
        F: BrowserResourceFetcher,
    {
        let mut page = self.load_pending_with_visited(requested_url, fetcher, visited_links)?;
        let mut stylesheet_index = 0;
        while stylesheet_index < page.stylesheet_resources.len() {
            if matches!(
                page.stylesheet_resources[stylesheet_index].state,
                BrowserStylesheetResourceState::Pending
            ) {
                let url = page.stylesheet_resources[stylesheet_index]
                    .url
                    .clone()
                    .expect("pending stylesheet URL");
                match fetch_browser_stylesheet(&url, fetcher) {
                    Ok(source) => complete_stylesheet_source(
                        &mut page.stylesheet_resources,
                        stylesheet_index,
                        source,
                        self.viewport.width,
                        self.viewport.height,
                    ),
                    Err(error) => {
                        page.stylesheet_resources[stylesheet_index].state =
                            BrowserStylesheetResourceState::Failed(error);
                    }
                }
            }
            stylesheet_index += 1;
        }
        for resource in &mut page.image_resources {
            resource.state = match fetch_and_decode_browser_image(&resource.url, fetcher) {
                Ok(pixels) => BrowserImageResourceState::Ready(pixels),
                Err(error) => BrowserImageResourceState::Failed(error),
            };
        }
        let mut prospective_visited = visited_links.clone();
        let _ = prospective_visited.record(&page.final_url);
        Ok(self.reflow_retained_with_visited(&page, &prospective_visited))
    }

    /// Fetch and parse only the main document, leaving inline images pending.
    pub fn load_pending_with_visited<F>(
        &self,
        requested_url: &str,
        document_fetcher: &F,
        visited_links: &VisitedLinks,
    ) -> Result<BrowserPage, BrowserLoadError>
    where
        F: BrowserResourceFetcher,
    {
        self.load_request_pending_with_visited(
            &BrowserFetchRequest::get(requested_url),
            document_fetcher,
            visited_links,
        )
    }

    pub fn load_request_pending_with_visited<F>(
        &self,
        request: &BrowserFetchRequest,
        document_fetcher: &F,
        visited_links: &VisitedLinks,
    ) -> Result<BrowserPage, BrowserLoadError>
    where
        F: BrowserResourceFetcher,
    {
        let response =
            document_fetcher
                .fetch_request(request)
                .map_err(|message| BrowserLoadError::Fetch {
                    url: request.url.clone(),
                    message,
                })?;
        ensure_success(&response)?;
        ensure_html_media_type(&response)?;

        let source = String::from_utf8_lossy(&response.body).into_owned();
        let parsed = parse_html(&source).map_err(|error| BrowserLoadError::Parse {
            url: response.final_url.clone(),
            message: error.to_string(),
        })?;
        let document = BrowserDocument::from_document(&parsed);
        let stylesheet_resources = stylesheet_resources_for_document(
            &document,
            &response.final_url,
            self.viewport.width,
            self.viewport.height,
        );
        let render_tree =
            BrowserRenderTree::from_document_with_document_url(&parsed, &response.final_url);
        let mut prospective_visited = visited_links.clone();
        let _ = prospective_visited.record(&response.final_url);
        let (style_context, stylesheet_failures) = style_context_for_resources(
            self.theme,
            &stylesheet_resources,
            self.viewport.width,
            self.viewport.height,
        );
        let mut paint = html_render_tree_to_paint_with_style_context(
            &render_tree,
            &style_context,
            &|url| prospective_visited.contains(url),
            self.viewport,
            self.measurer,
            self.shaper,
            self.metrics,
            self.resolver,
        );
        let image_resources = scene_image_resource_uris(&paint.scene)
            .into_iter()
            .map(|url| BrowserImageResource {
                url,
                state: BrowserImageResourceState::Pending,
            })
            .collect::<Vec<_>>();
        let image_resolution = resolve_scene_image_resources_incrementally(
            &paint.scene,
            &RetainedImageResolver(&image_resources),
        );
        paint.scene = image_resolution.scene;

        Ok(BrowserPage {
            requested_url: request.url.clone(),
            final_url: response.final_url,
            status: response.status,
            source,
            document,
            render_tree,
            paint,
            image_failures: image_resolution.failures,
            image_resources,
            stylesheet_failures,
            stylesheet_resources,
        })
    }

    /// Recompose a previously loaded page for this pipeline's viewport.
    /// Document bytes, parse output, navigation metadata, and history identity
    /// are retained; only layout, paint, links, and image placement change.
    pub fn reflow<F>(&self, page: &BrowserPage, fetcher: &F) -> BrowserPage
    where
        F: BrowserResourceFetcher,
    {
        self.reflow_with_visited(page, fetcher, &VisitedLinks::new())
    }

    pub fn reflow_with_visited<F>(
        &self,
        page: &BrowserPage,
        _fetcher: &F,
        visited_links: &VisitedLinks,
    ) -> BrowserPage
    where
        F: BrowserResourceFetcher,
    {
        self.reflow_retained_with_visited(page, visited_links)
    }

    /// Recompose from retained document and subresource state only.
    pub fn reflow_retained_with_visited(
        &self,
        page: &BrowserPage,
        visited_links: &VisitedLinks,
    ) -> BrowserPage {
        let (paint, image_failures, stylesheet_failures) = self.compose(
            &page.render_tree,
            &page.image_resources,
            &page.stylesheet_resources,
            visited_links,
        );
        let mut reflowed = page.clone();
        reflowed.paint = paint;
        reflowed.image_failures = image_failures;
        reflowed.stylesheet_failures = stylesheet_failures;
        reflowed
    }

    fn compose(
        &self,
        render_tree: &BrowserRenderTree,
        image_resources: &[BrowserImageResource],
        stylesheet_resources: &[BrowserStylesheetResource],
        visited_links: &VisitedLinks,
    ) -> (
        HtmlPaintOutput,
        Vec<HtmlImageResourceError>,
        Vec<BrowserStylesheetError>,
    ) {
        let (style_context, stylesheet_failures) = style_context_for_resources(
            self.theme,
            stylesheet_resources,
            self.viewport.width,
            self.viewport.height,
        );
        let style_context =
            style_context.with_image_intrinsics(image_resources.iter().filter_map(|resource| {
                match &resource.state {
                    BrowserImageResourceState::Ready(pixels) => Some((
                        resource.url.clone(),
                        f64::from(pixels.width),
                        f64::from(pixels.height),
                    )),
                    BrowserImageResourceState::Pending | BrowserImageResourceState::Failed(_) => {
                        None
                    }
                }
            }));
        let mut paint = html_render_tree_to_paint_with_style_context(
            render_tree,
            &style_context,
            &|url| visited_links.contains(url),
            self.viewport,
            self.measurer,
            self.shaper,
            self.metrics,
            self.resolver,
        );
        let image_resolution = resolve_scene_image_resources_incrementally(
            &paint.scene,
            &RetainedImageResolver(image_resources),
        );
        paint.scene = image_resolution.scene;
        (paint, image_resolution.failures, stylesheet_failures)
    }
}

fn stylesheet_resources_for_document(
    document: &BrowserDocument,
    document_url: &str,
    viewport_width: f64,
    viewport_height: f64,
) -> Vec<BrowserStylesheetResource> {
    let mut resources = document
        .stylesheets
        .iter()
        .map(|stylesheet| {
            let active = !stylesheet.disabled
                && !stylesheet.alternate
                && html_media_query_applies(
                    stylesheet.media.as_deref(),
                    viewport_width,
                    viewport_height,
                );
            let authored_url = stylesheet
                .resolved_href
                .as_deref()
                .or(stylesheet.href.as_deref());
            let url = authored_url.and_then(|href| resolve_subresource_url(document_url, href));
            let state = if !active {
                BrowserStylesheetResourceState::Inactive
            } else if let Some(source) = &stylesheet.text {
                match HtmlAuthorStylesheet::parse(source) {
                    Ok(_) => BrowserStylesheetResourceState::Ready(source.clone()),
                    Err(error) => {
                        BrowserStylesheetResourceState::Failed(BrowserStylesheetError::Parse {
                            url: None,
                            message: error.to_string(),
                        })
                    }
                }
            } else if url.is_some() {
                BrowserStylesheetResourceState::Pending
            } else {
                BrowserStylesheetResourceState::Inactive
            };
            BrowserStylesheetResource {
                base_url: url.clone().unwrap_or_else(|| document_url.to_string()),
                url,
                media: stylesheet.media.clone(),
                render_blocking: active && stylesheet.href.is_some(),
                imported_by: None,
                imports: Vec::new(),
                state,
            }
        })
        .collect::<Vec<_>>();
    let root_count = resources.len();
    for index in 0..root_count {
        let source = match &resources[index].state {
            BrowserStylesheetResourceState::Ready(source) => Some(source.clone()),
            _ => None,
        };
        if let Some(source) = source {
            complete_stylesheet_source(
                &mut resources,
                index,
                source,
                viewport_width,
                viewport_height,
            );
        }
    }
    resources
}

fn resolve_subresource_url(document_url: &str, resource_url: &str) -> Option<String> {
    Url::parse(document_url)
        .and_then(|base| base.resolve(resource_url))
        .map(|url| url.to_url_string())
        .ok()
        .or_else(|| resource_url.contains(':').then(|| resource_url.to_string()))
}

fn complete_stylesheet_source(
    resources: &mut Vec<BrowserStylesheetResource>,
    index: usize,
    source: String,
    viewport_width: f64,
    viewport_height: f64,
) {
    let parsed = match HtmlAuthorStylesheet::parse(&source) {
        Ok(parsed) => parsed,
        Err(error) => {
            let url = resources
                .get(index)
                .and_then(|resource| resource.url.clone());
            if let Some(resource) = resources.get_mut(index) {
                resource.state =
                    BrowserStylesheetResourceState::Failed(BrowserStylesheetError::Parse {
                        url,
                        message: error.to_string(),
                    });
            }
            return;
        }
    };
    let Some(parent_url) = resources
        .get(index)
        .map(|resource| resource.base_url.clone())
    else {
        return;
    };
    if let Some(resource) = resources.get_mut(index) {
        resource.state = BrowserStylesheetResourceState::Ready(source);
    }
    let mut imported_ordinals = Vec::new();
    for import in parsed.imports() {
        let resolved = resolve_subresource_url(&parent_url, &import.href);
        let active =
            html_media_query_applies(import.media.as_deref(), viewport_width, viewport_height);
        let state = if !active {
            BrowserStylesheetResourceState::Inactive
        } else if let Some(url) = resolved.as_deref() {
            if stylesheet_import_would_cycle(resources, index, url) {
                BrowserStylesheetResourceState::Failed(BrowserStylesheetError::ImportCycle {
                    url: url.to_string(),
                })
            } else {
                BrowserStylesheetResourceState::Pending
            }
        } else {
            BrowserStylesheetResourceState::Failed(BrowserStylesheetError::Fetch {
                url: import.href.clone(),
                message: "could not resolve imported stylesheet URL".into(),
            })
        };
        let ordinal = resources.len();
        resources.push(BrowserStylesheetResource {
            base_url: resolved.clone().unwrap_or_else(|| parent_url.clone()),
            url: resolved,
            media: import.media.clone(),
            render_blocking: active,
            imported_by: Some(index),
            imports: Vec::new(),
            state,
        });
        imported_ordinals.push(ordinal);
    }
    if let Some(resource) = resources.get_mut(index) {
        resource.imports = imported_ordinals;
    }
}

fn stylesheet_import_would_cycle(
    resources: &[BrowserStylesheetResource],
    parent: usize,
    imported_url: &str,
) -> bool {
    let mut cursor = Some(parent);
    while let Some(index) = cursor {
        let Some(resource) = resources.get(index) else {
            break;
        };
        if resource.url.as_deref() == Some(imported_url) {
            return true;
        }
        cursor = resource.imported_by;
    }
    false
}

fn active_stylesheet_source_count(resources: &[BrowserStylesheetResource]) -> usize {
    let mut count = 0;
    for index in resources
        .iter()
        .enumerate()
        .filter_map(|(index, resource)| resource.imported_by.is_none().then_some(index))
    {
        if !count_active_stylesheets(resources, index, &mut count) {
            break;
        }
    }
    count
}

fn count_active_stylesheets(
    resources: &[BrowserStylesheetResource],
    index: usize,
    count: &mut usize,
) -> bool {
    let Some(resource) = resources.get(index) else {
        return true;
    };
    if matches!(resource.state, BrowserStylesheetResourceState::Pending) {
        return false;
    }
    for imported in &resource.imports {
        if !count_active_stylesheets(resources, *imported, count) {
            return false;
        }
    }
    if matches!(resource.state, BrowserStylesheetResourceState::Ready(_)) {
        *count += 1;
    }
    true
}

fn style_context_for_resources(
    theme: &HtmlTheme,
    resources: &[BrowserStylesheetResource],
    viewport_width: f64,
    viewport_height: f64,
) -> (HtmlStyleContext, Vec<BrowserStylesheetError>) {
    let mut context =
        HtmlStyleContext::new(theme.clone()).with_viewport(viewport_width, viewport_height);
    let mut failures = Vec::new();
    for index in resources
        .iter()
        .enumerate()
        .filter_map(|(index, resource)| resource.imported_by.is_none().then_some(index))
    {
        if !append_stylesheet_context(
            resources,
            index,
            &mut context.author_stylesheets,
            &mut failures,
        ) {
            break;
        }
    }
    (context, failures)
}

fn append_stylesheet_context(
    resources: &[BrowserStylesheetResource],
    index: usize,
    stylesheets: &mut Vec<HtmlAuthorStylesheet>,
    failures: &mut Vec<BrowserStylesheetError>,
) -> bool {
    let Some(resource) = resources.get(index) else {
        return true;
    };
    if matches!(resource.state, BrowserStylesheetResourceState::Pending) {
        return false;
    }
    for imported in &resource.imports {
        if !append_stylesheet_context(resources, *imported, stylesheets, failures) {
            return false;
        }
    }
    match &resource.state {
        BrowserStylesheetResourceState::Ready(source) => {
            match HtmlAuthorStylesheet::parse(source) {
                Ok(stylesheet) => stylesheets.push(stylesheet),
                Err(error) => failures.push(BrowserStylesheetError::Parse {
                    url: resource.url.clone(),
                    message: error.to_string(),
                }),
            }
        }
        BrowserStylesheetResourceState::Failed(error) => failures.push(error.clone()),
        BrowserStylesheetResourceState::Pending => return false,
        BrowserStylesheetResourceState::Inactive => {}
    }
    true
}

fn image_resource_state(resources: &[BrowserImageResource], url: &str) -> HtmlImageResource {
    match resources.iter().find(|resource| resource.url == url) {
        Some(BrowserImageResource {
            state: BrowserImageResourceState::Ready(pixels),
            ..
        }) => HtmlImageResource::Ready(pixels.clone()),
        Some(BrowserImageResource {
            state: BrowserImageResourceState::Failed(error),
            ..
        }) => HtmlImageResource::Failed(error.clone()),
        _ => HtmlImageResource::Pending,
    }
}

fn fetch_and_decode_browser_image<F>(
    url: &str,
    fetcher: &F,
) -> Result<PixelContainer, HtmlImageResourceError>
where
    F: BrowserResourceFetcher,
{
    let response = fetcher
        .fetch(url)
        .map_err(|message| HtmlImageResourceError::Fetch {
            uri: url.to_string(),
            message,
        })?;
    if !is_success(response.status) {
        return Err(HtmlImageResourceError::Fetch {
            uri: url.to_string(),
            message: format!("HTTP status {}", response.status),
        });
    }
    decode_image_resource(url, FetchedImage::new(response.body, response.media_type))
}

fn fetch_browser_stylesheet<F>(url: &str, fetcher: &F) -> Result<String, BrowserStylesheetError>
where
    F: BrowserResourceFetcher,
{
    let response = fetcher
        .fetch(url)
        .map_err(|message| BrowserStylesheetError::Fetch {
            url: url.to_string(),
            message,
        })?;
    if !(200..300).contains(&response.status) {
        return Err(BrowserStylesheetError::HttpStatus {
            url: url.to_string(),
            status: response.status,
        });
    }
    if let Some(media_type) = response.media_type.as_deref() {
        let essence = media_type.split(';').next().unwrap_or_default().trim();
        if !essence.eq_ignore_ascii_case("text/css") {
            return Err(BrowserStylesheetError::UnsupportedMediaType {
                url: url.to_string(),
                media_type: media_type.to_string(),
            });
        }
    }
    Ok(String::from_utf8_lossy(&response.body).into_owned())
}

struct RetainedImageResolver<'a>(&'a [BrowserImageResource]);

impl HtmlImageResolver for RetainedImageResolver<'_> {
    fn resolve(&self, uri: &str) -> HtmlImageResource {
        image_resource_state(self.0, uri)
    }
}

fn ensure_success(response: &BrowserFetchResponse) -> Result<(), BrowserLoadError> {
    if is_success(response.status) {
        Ok(())
    } else {
        Err(BrowserLoadError::HttpStatus {
            url: response.final_url.clone(),
            status: response.status,
        })
    }
}

fn is_success(status: u16) -> bool {
    (200..300).contains(&status)
}

fn finite_non_negative(value: f64) -> f64 {
    if value.is_finite() {
        value.max(0.0)
    } else {
        0.0
    }
}

fn escape_html_text(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&#39;"),
            character => escaped.push(character),
        }
    }
    escaped
}

fn ensure_html_media_type(response: &BrowserFetchResponse) -> Result<(), BrowserLoadError> {
    let Some(media_type) = response.media_type.as_deref() else {
        return Ok(());
    };
    let essence = media_type.split(';').next().unwrap_or("").trim();
    if essence.eq_ignore_ascii_case("text/html")
        || essence.eq_ignore_ascii_case("application/xhtml+xml")
    {
        Ok(())
    } else {
        Err(BrowserLoadError::UnsupportedMediaType {
            url: response.final_url.clone(),
            media_type: media_type.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use html_to_layout::mosaic_html_theme;
    use html_to_paint::HtmlPaintViewport;
    use image_codec_gif::encode_gif;
    use layout_ir::{FontSpec, MeasureResult};
    use paint_instructions::{ImageSrc, PaintInstruction, PixelContainer};
    use std::cell::RefCell;

    fn count_pixel_images(instructions: &[PaintInstruction]) -> usize {
        instructions
            .iter()
            .map(|instruction| match instruction {
                PaintInstruction::Image(image) if matches!(image.src, ImageSrc::Pixels(_)) => 1,
                PaintInstruction::Group(group) => count_pixel_images(&group.children),
                PaintInstruction::Layer(layer) => count_pixel_images(&layer.children),
                PaintInstruction::Clip(clip) => count_pixel_images(&clip.children),
                _ => 0,
            })
            .sum()
    }

    fn positioned_text_color(
        node: &layout_ir::PositionedNode,
        value: &str,
    ) -> Option<layout_ir::Color> {
        if let Some(layout_ir::Content::Text(text)) = &node.content {
            if text.value == value {
                return Some(text.color);
            }
        }
        node.children
            .iter()
            .find_map(|child| positioned_text_color(child, value))
    }

    fn positioned_by_id<'a>(
        node: &'a layout_ir::PositionedNode,
        id: &str,
    ) -> Option<&'a layout_ir::PositionedNode> {
        if node.id.as_deref() == Some(id) {
            return Some(node);
        }
        node.children
            .iter()
            .find_map(|child| positioned_by_id(child, id))
    }

    #[derive(Default)]
    struct RecordingScheduler(Vec<String>);

    impl BrowserSubresourceScheduler for RecordingScheduler {
        fn cancel(&mut self, request: &BrowserSubresourceRequest) {
            self.0.push(format!("cancel:{}", request.url));
        }

        fn request(&mut self, request: BrowserSubresourceRequest) {
            self.0.push(format!("request:{}", request.url));
        }
    }
    use text_interfaces::{
        Direction, FontQuery, FontResolutionError, Glyph, ShapeOptions, ShapedRun, ShapedText,
        ShapingError,
    };

    #[test]
    fn navigation_history_matches_back_forward_home_and_reload_model() {
        let mut history =
            NavigationHistory::with_current("http://home.test/", "http://example.test/a");
        history.navigate("http://example.test/b");
        history.navigate("http://example.test/c");

        assert_eq!(history.back(), Some("http://example.test/b"));
        assert_eq!(history.forward_stack(), &["http://example.test/c"]);
        assert_eq!(history.forward(), Some("http://example.test/c"));
        assert_eq!(history.reload(), Some("http://example.test/c"));

        history.back();
        history.navigate("http://example.test/d");
        assert!(history.forward_stack().is_empty());
        assert_eq!(history.home(), "http://home.test/");
        assert_eq!(
            history.replace_current("http://home.test/index.html"),
            Some("http://home.test/index.html")
        );
    }

    #[test]
    fn mosaic_chrome_reduces_events_and_projects_session_state() {
        let home_url = "http://home.test/";
        let page_url = "http://example.test/guide";
        let fetcher = |url: &str| {
            Ok(BrowserFetchResponse::new(
                url,
                200,
                Some("text/html".into()),
                b"<title> Venture Guide </title><p>Ready</p>".to_vec(),
            ))
        };
        let theme = mosaic_html_theme();
        let pipeline = BrowserPagePipeline::new(
            &theme,
            HtmlPaintViewport::new(220.0, 100.0, 1.0),
            &MonoMeasurer,
            &FakeShaper,
            &FakeMetrics,
            &FakeResolver,
        );
        let mut session = BrowserSession::new(home_url, 40.0);
        let mut chrome = BrowserChromeController::new(&session);

        assert_eq!(chrome.address_draft(), home_url);
        assert_eq!(
            chrome.props(&session, "Ready", false),
            BrowserChromeProps {
                address: home_url.into(),
                page_title: String::new(),
                status_text: "Ready".into(),
                back_disabled: true,
                forward_disabled: true,
                bookmark_label: "Bookmark".into(),
                bookmark_disabled: true,
                view_source_disabled: true,
                navigation_disabled: false,
            }
        );
        assert_eq!(
            chrome.handle_event(BrowserChromeEvent::Back, &session, false),
            None
        );
        assert_eq!(
            chrome.handle_event(BrowserChromeEvent::Reload, &session, false),
            None
        );

        session
            .execute(BrowserNavigation::Home, &pipeline, &fetcher)
            .expect("home should load")
            .expect("home should create a viewport");
        chrome.synchronize(&session);

        assert_eq!(
            chrome.handle_event(
                BrowserChromeEvent::AddressChange(format!("  {page_url}  ")),
                &session,
                false,
            ),
            None
        );
        let action = chrome
            .handle_event(BrowserChromeEvent::Navigate, &session, false)
            .expect("non-empty address should navigate");
        assert_eq!(
            action,
            BrowserChromeAction::Navigate(BrowserNavigation::Navigate(page_url.into()))
        );
        let BrowserChromeAction::Navigate(navigation) = action else {
            unreachable!("Navigate should reduce to navigation")
        };
        session
            .execute(navigation, &pipeline, &fetcher)
            .expect("chrome navigation should load")
            .expect("chrome navigation should create a viewport");
        chrome.synchronize(&session);

        assert_eq!(
            chrome.props(&session, "Status: 200", false),
            BrowserChromeProps {
                address: page_url.into(),
                page_title: "Venture Guide".into(),
                status_text: "Status: 200".into(),
                back_disabled: false,
                forward_disabled: true,
                bookmark_label: "Bookmark".into(),
                bookmark_disabled: false,
                view_source_disabled: false,
                navigation_disabled: false,
            }
        );
        assert_eq!(
            chrome.handle_event(BrowserChromeEvent::Back, &session, false),
            Some(BrowserChromeAction::Navigate(BrowserNavigation::Back))
        );

        chrome.handle_event(
            BrowserChromeEvent::AddressChange("http://draft.test/".into()),
            &session,
            false,
        );
        chrome.handle_event(
            BrowserChromeEvent::AddressChange("ignored while loading".into()),
            &session,
            true,
        );
        assert_eq!(chrome.address_draft(), "http://draft.test/");
        let disabled = chrome.props(&session, "Loading", true);
        assert!(disabled.back_disabled);
        assert!(disabled.forward_disabled);
        assert!(disabled.bookmark_disabled);
        assert!(disabled.view_source_disabled);
        assert!(disabled.navigation_disabled);
    }

    #[test]
    fn mosaic_chrome_event_names_match_the_generated_bridge_contract() {
        let events = [
            BrowserChromeEvent::Back,
            BrowserChromeEvent::Forward,
            BrowserChromeEvent::Home,
            BrowserChromeEvent::Reload,
            BrowserChromeEvent::ToggleBookmark,
            BrowserChromeEvent::ViewSource,
            BrowserChromeEvent::AddressChange(String::new()),
            BrowserChromeEvent::Navigate,
        ];
        assert_eq!(
            events.map(|event| event.mosaic_name()),
            VENTURE_CHROME_EVENT_NAMES
        );
    }

    #[test]
    fn shared_host_controller_keeps_native_adapter_behavior_in_one_state_machine() {
        let home_url = "http://example.test/";
        let next_url = "http://example.test/next";
        let fetcher = |url: &str| match url {
            "http://example.test/" => Ok(BrowserFetchResponse::new(
                url,
                200,
                Some("text/html".into()),
                b"<title>Home</title><p><a href='/next'>Next</a></p>".to_vec(),
            )),
            "http://example.test/next" => Ok(BrowserFetchResponse::new(
                url,
                200,
                Some("text/html".into()),
                format!(
                    "<title>Next</title>{}",
                    (0..40)
                        .map(|index| format!("<p>Scrollable row {index}</p>"))
                        .collect::<String>()
                )
                .into_bytes(),
            )),
            _ => Err("offline".to_string()),
        };
        let theme = mosaic_html_theme();
        let pipeline = BrowserPagePipeline::new(
            &theme,
            HtmlPaintViewport::new(220.0, 80.0, 1.0),
            &MonoMeasurer,
            &FakeShaper,
            &FakeMetrics,
            &FakeResolver,
        );
        let mut session = BrowserSession::new(home_url, 40.0);
        session
            .execute(BrowserNavigation::Home, &pipeline, &fetcher)
            .expect("home should load");
        let mut host = BrowserHostController::new(session);
        let mut bookmarks = MemoryBookmarkRepository::default();

        assert_eq!(host.props().page_title, "Home");
        let link = host.session().viewport().unwrap().page().paint.links[0].clone();
        assert!(host.update_hover(link.x + 1.0, link.y + 1.0));
        assert_eq!(host.props().status_text, next_url);
        assert!(host
            .activate_link(link.x + 1.0, link.y + 1.0, |session, navigation| {
                Ok(session.execute(navigation, &pipeline, &fetcher)?.is_some())
            })
            .expect("link should load"));
        assert_eq!(host.props().page_title, "Next");
        assert_eq!(host.props().address, next_url);
        assert!(host.scroll_by(40.0));
        assert!(host.scroll_metrics().unwrap().offset_y > 0.0);

        assert!(!host
            .handle_event(
                BrowserChromeEvent::AddressChange("http://missing.test/".into()),
                &mut bookmarks,
                |_, _| unreachable!("address edits do not execute navigation"),
            )
            .unwrap());
        let error = host
            .handle_event(
                BrowserChromeEvent::Navigate,
                &mut bookmarks,
                |session, navigation| {
                    Ok(session.execute(navigation, &pipeline, &fetcher)?.is_some())
                },
            )
            .expect_err("missing page should fail transactionally");
        assert!(matches!(
            error,
            BrowserCommandError::Load(BrowserLoadError::Fetch { .. })
        ));
        assert_eq!(host.session().history().current_url(), Some(next_url));
        assert_eq!(host.props().address, "http://missing.test/");
        assert!(host.props().status_text.starts_with("Load failed:"));
    }

    #[test]
    fn view_source_emits_a_network_free_preformatted_auxiliary_document() {
        let url = "http://example.test/source?mode=raw&lang=html";
        let raw_source = "<title>Source test</title>\n<pre>&lt;already escaped&gt;</pre>\n<p data-note=\"'quoted'\">Ready & waiting</p>";
        let fetcher = |requested: &str| {
            Ok(BrowserFetchResponse::new(
                requested,
                200,
                Some("text/html".into()),
                raw_source.as_bytes().to_vec(),
            ))
        };
        let theme = mosaic_html_theme();
        let pipeline = BrowserPagePipeline::new(
            &theme,
            HtmlPaintViewport::new(640.0, 480.0, 1.0),
            &MonoMeasurer,
            &FakeShaper,
            &FakeMetrics,
            &FakeResolver,
        );
        let mut session = BrowserSession::new(url, 480.0);
        session
            .execute(BrowserNavigation::Home, &pipeline, &fetcher)
            .unwrap();
        let mut host = BrowserHostController::new(session);
        let mut bookmarks = MemoryBookmarkRepository::default();

        let outcome = host
            .handle_event_with_effect(BrowserChromeEvent::ViewSource, &mut bookmarks, |_, _| {
                unreachable!("view source must not navigate or refetch")
            })
            .unwrap();
        assert!(!outcome.changed);
        let BrowserHostEffect::OpenAuxiliaryDocument(auxiliary) = outcome.effect.unwrap();
        assert_eq!(auxiliary.kind, BrowserAuxiliaryDocumentKind::ViewSource);
        assert_eq!(
            auxiliary.address,
            "view-source:http://example.test/source?mode=raw&lang=html"
        );
        assert_eq!(auxiliary.title, format!("Source: {url}"));
        assert!(auxiliary.html.contains("<body><pre>"));
        assert!(auxiliary
            .html
            .contains("&lt;title&gt;Source test&lt;/title&gt;"));
        assert!(auxiliary.html.contains("&amp;lt;already escaped&amp;gt;"));

        let source_page = pipeline.compose_auxiliary_document(&auxiliary).unwrap();
        assert_eq!(source_page.final_url, auxiliary.address);
        assert_eq!(
            source_page.document.title.as_deref(),
            Some(auxiliary.title.as_str())
        );
        assert_eq!(
            source_page.document.body_text,
            raw_source.replace('\n', " ")
        );
        assert!(source_page.image_failures.is_empty());
        assert_eq!(host.session().history().current_url(), Some(url));
        assert_eq!(host.session().viewport().unwrap().page().source, raw_source);
    }

    #[test]
    fn bookmark_command_persists_before_chrome_state_changes() {
        let url = "http://example.test/guide#chapter";
        let fetcher = |requested: &str| {
            Ok(BrowserFetchResponse::new(
                requested,
                200,
                Some("text/html".into()),
                b"<title>Guide chapter</title><p>Ready</p>".to_vec(),
            ))
        };
        let theme = mosaic_html_theme();
        let pipeline = BrowserPagePipeline::new(
            &theme,
            HtmlPaintViewport::new(220.0, 80.0, 1.0),
            &MonoMeasurer,
            &FakeShaper,
            &FakeMetrics,
            &FakeResolver,
        );
        let mut session = BrowserSession::new(url, 40.0);
        session
            .execute(BrowserNavigation::Home, &pipeline, &fetcher)
            .unwrap();
        let mut host = BrowserHostController::new(session);
        let mut repository = MemoryBookmarkRepository::default();

        assert_eq!(host.props().bookmark_label, "Bookmark");
        assert!(host
            .handle_event(
                BrowserChromeEvent::ToggleBookmark,
                &mut repository,
                |_, _| unreachable!("bookmark action does not navigate"),
            )
            .unwrap());
        assert_eq!(host.props().bookmark_label, "Remove Bookmark");
        assert_eq!(repository.stored().entries()[0].title(), "Guide chapter");
        assert_eq!(repository.stored().entries()[0].url().as_str(), url);

        repository.fail_saves_with("disk full");
        let error = host
            .handle_event(
                BrowserChromeEvent::ToggleBookmark,
                &mut repository,
                |_, _| unreachable!("bookmark action does not navigate"),
            )
            .unwrap_err();
        assert!(matches!(error, BrowserCommandError::Bookmark(_)));
        assert_eq!(host.props().bookmark_label, "Remove Bookmark");
        assert!(host.props().status_text.starts_with("Bookmark failed:"));
    }

    #[test]
    fn browser_session_dispatches_navigation_transactionally() {
        let requested = "http://example.test/start";
        let first_page = "http://example.test/guide/index.html";
        let next_page = "http://example.test/guide/next.html";
        let home_page = "http://home.test/";
        let fetched_urls = RefCell::new(Vec::new());
        let fetcher = |url: &str| {
            fetched_urls.borrow_mut().push(url.to_string());
            match url {
                "http://example.test/start" => Ok(BrowserFetchResponse::new(
                    first_page,
                    200,
                    Some("text/html".into()),
                    b"<title>Guide</title><p><a href='next.html'>Next</a></p>".to_vec(),
                )),
                "http://example.test/guide/index.html" => Ok(BrowserFetchResponse::new(
                    url,
                    200,
                    Some("text/html".into()),
                    b"<title>Guide</title><p><a href='next.html'>Next</a></p>".to_vec(),
                )),
                "http://example.test/guide/next.html" => Ok(BrowserFetchResponse::new(
                    url,
                    200,
                    Some("text/html".into()),
                    b"<title>Next</title><p>Destination</p>".to_vec(),
                )),
                "http://home.test/" => Ok(BrowserFetchResponse::new(
                    url,
                    200,
                    Some("text/html".into()),
                    b"<title>Home</title><p>Venture home</p>".to_vec(),
                )),
                "http://example.test/broken" => Err("offline".into()),
                _ => Err(format!("unexpected URL {url}")),
            }
        };
        let theme = mosaic_html_theme();
        let pipeline = BrowserPagePipeline::new(
            &theme,
            HtmlPaintViewport::new(220.0, 100.0, 1.0),
            &MonoMeasurer,
            &FakeShaper,
            &FakeMetrics,
            &FakeResolver,
        );
        let mut session = BrowserSession::new(home_page, 40.0);

        assert!(session
            .execute(BrowserNavigation::Back, &pipeline, &fetcher)
            .expect("empty Back should be a no-op")
            .is_none());
        session
            .execute(
                BrowserNavigation::Navigate(requested.into()),
                &pipeline,
                &fetcher,
            )
            .expect("initial navigation should load")
            .expect("initial navigation should create a viewport");
        assert_eq!(session.history().current_url(), Some(first_page));
        assert!(session.visited_links().contains(first_page));
        assert_eq!(session.visited_links().len(), 1);
        assert_eq!(
            session
                .viewport()
                .map(|viewport| viewport.page().final_url.as_str()),
            Some(first_page)
        );

        let link = session
            .viewport()
            .and_then(|viewport| viewport.page().paint.links.first())
            .cloned()
            .expect("first page should expose its resolved link");
        let offset = session
            .viewport_mut()
            .expect("loaded page should have a viewport")
            .set_scroll_offset_y(link.y);
        session
            .activate_link(
                link.x + link.width / 2.0,
                link.y - offset + link.height / 2.0,
                &pipeline,
                &fetcher,
            )
            .expect("link activation should load")
            .expect("link activation should replace the viewport");
        assert_eq!(session.history().current_url(), Some(next_page));
        assert_eq!(session.history().back_stack(), &[first_page.to_string()]);
        assert!(session.visited_links().contains(next_page));
        assert_eq!(session.visited_links().len(), 2);

        session
            .viewport_mut()
            .expect("loaded page should have a viewport")
            .scroll_by(20.0);
        let before_failure = session.clone();
        assert_eq!(
            session.execute(
                BrowserNavigation::Navigate("http://example.test/broken".into()),
                &pipeline,
                &fetcher,
            ),
            Err(BrowserLoadError::Fetch {
                url: "http://example.test/broken".into(),
                message: "offline".into(),
            })
        );
        assert_eq!(session, before_failure);

        session
            .execute(BrowserNavigation::Back, &pipeline, &fetcher)
            .expect("Back should reload the prior page")
            .expect("Back should replace the viewport");
        assert_eq!(session.history().current_url(), Some(first_page));
        assert_eq!(
            session
                .viewport()
                .map(|viewport| viewport.scroll_state().offset_y()),
            Some(0.0)
        );
        session
            .execute(BrowserNavigation::Forward, &pipeline, &fetcher)
            .expect("Forward should reload the next page")
            .expect("Forward should replace the viewport");
        assert_eq!(session.history().current_url(), Some(next_page));
        session
            .execute(BrowserNavigation::Home, &pipeline, &fetcher)
            .expect("Home should load")
            .expect("Home should replace the viewport");
        assert_eq!(session.history().current_url(), Some(home_page));
        let history_before_reload = session.history().clone();
        session
            .execute(BrowserNavigation::Reload, &pipeline, &fetcher)
            .expect("Reload should load")
            .expect("Reload should replace the viewport");
        assert_eq!(session.history(), &history_before_reload);
        assert_eq!(session.visited_links().len(), 3);

        assert_eq!(
            fetched_urls.into_inner(),
            vec![
                requested,
                next_page,
                "http://example.test/broken",
                first_page,
                next_page,
                home_page,
                home_page,
            ]
        );
    }

    #[test]
    fn visited_links_follow_final_urls_and_repaint_across_history_and_reflow() {
        let requested = "http://example.test/start";
        let first = "http://example.test:80/guide/../index.html#intro";
        let next_request = "http://example.test/next";
        let next = "http://example.test/next#top";
        let broken = "http://example.test/broken";
        let fetcher = |url: &str| match url {
            "http://example.test/start" => Ok(BrowserFetchResponse::new(
                first,
                200,
                Some("text/html".into()),
                b"<p><a href='/index.html#details'>Self</a> \
                    <a href='/next'>Next</a></p>"
                    .to_vec(),
            )),
            "http://example.test:80/guide/../index.html#intro" => Ok(BrowserFetchResponse::new(
                first,
                200,
                Some("text/html".into()),
                b"<p><a href='/index.html#details'>Self</a> \
                        <a href='/next'>Next</a></p>"
                    .to_vec(),
            )),
            "http://example.test/next" => Ok(BrowserFetchResponse::new(
                next,
                200,
                Some("text/html".into()),
                b"<p><a href='/index.html'>First</a> \
                    <a href='/next#other'>Self</a></p>"
                    .to_vec(),
            )),
            "http://example.test/next#top" => Ok(BrowserFetchResponse::new(
                next,
                200,
                Some("text/html".into()),
                b"<p><a href='/index.html'>First</a> \
                    <a href='/next#other'>Self</a></p>"
                    .to_vec(),
            )),
            "http://example.test/broken" => Err("offline".into()),
            _ => Err(format!("unexpected URL {url}")),
        };
        let theme = mosaic_html_theme();
        let wide = BrowserPagePipeline::new(
            &theme,
            HtmlPaintViewport::new(240.0, 80.0, 1.0),
            &MonoMeasurer,
            &FakeShaper,
            &FakeMetrics,
            &FakeResolver,
        );
        let mut session = BrowserSession::new(requested, 80.0);

        session
            .execute(
                BrowserNavigation::Navigate(requested.into()),
                &wide,
                &fetcher,
            )
            .expect("redirected page should load");
        assert_eq!(session.history().current_url(), Some(first));
        assert_eq!(session.visited_links().len(), 1);
        assert!(session
            .visited_links()
            .contains("http://example.test/index.html#another"));
        assert!(!session.visited_links().contains(requested));
        assert_scene_has_fill(session.viewport().unwrap(), "rgb(85, 26, 139)");
        assert_scene_has_fill(session.viewport().unwrap(), "rgb(0, 0, 238)");

        session
            .execute(
                BrowserNavigation::Navigate(next_request.into()),
                &wide,
                &fetcher,
            )
            .expect("next page should load");
        assert_eq!(session.visited_links().len(), 2);
        assert!(session
            .visited_links()
            .contains("http://example.test/next#different"));
        assert_eq!(
            scene_fill_count(session.viewport().unwrap(), "rgb(0, 0, 238)"),
            0
        );

        session
            .execute(BrowserNavigation::Back, &wide, &fetcher)
            .expect("back should reload the first page");
        assert_eq!(session.visited_links().len(), 2);
        assert_eq!(
            scene_fill_count(session.viewport().unwrap(), "rgb(0, 0, 238)"),
            0
        );

        let narrow = BrowserPagePipeline::new(
            &theme,
            HtmlPaintViewport::new(90.0, 80.0, 1.0),
            &MonoMeasurer,
            &FakeShaper,
            &FakeMetrics,
            &FakeResolver,
        );
        session.reflow(&narrow, &fetcher, 80.0);
        assert_eq!(session.visited_links().len(), 2);
        assert_eq!(
            scene_fill_count(session.viewport().unwrap(), "rgb(0, 0, 238)"),
            0
        );

        let before_failure = session.clone();
        assert!(session
            .execute(BrowserNavigation::Navigate(broken.into()), &wide, &fetcher)
            .is_err());
        assert_eq!(session, before_failure);
    }

    #[test]
    fn scroll_state_clamps_offsets_and_reacts_to_geometry_changes() {
        let mut scroll = ScrollState::new(100.0, 260.0);
        assert_eq!(scroll.max_offset_y(), 160.0);
        assert_eq!(scroll.scroll_by(-20.0), 0.0);
        assert_eq!(scroll.set_offset_y(75.0), 75.0);
        assert_eq!(scroll.scroll_by(200.0), 160.0);
        assert_eq!(
            BrowserScrollMetrics::from(&scroll),
            BrowserScrollMetrics {
                offset_y: 160.0,
                viewport_height: 100.0,
                content_height: 260.0,
                max_offset_y: 160.0,
            }
        );
        assert_eq!(scroll.scroll_by(f64::NAN), 160.0);

        assert_eq!(scroll.set_dimensions(120.0, 80.0), 0.0);
        assert_eq!(scroll.max_offset_y(), 0.0);
        assert_eq!(scroll.set_dimensions(f64::NAN, f64::INFINITY), 0.0);
        assert_eq!(scroll.viewport_height(), 0.0);
        assert_eq!(scroll.content_height(), 0.0);
    }

    #[test]
    fn semantic_scroll_commands_share_exact_names_and_clamped_behavior() {
        let commands = [
            BrowserScrollCommand::LineUp,
            BrowserScrollCommand::LineDown,
            BrowserScrollCommand::PageUp,
            BrowserScrollCommand::PageDown,
            BrowserScrollCommand::DocumentStart,
            BrowserScrollCommand::DocumentEnd,
        ];
        assert_eq!(
            commands.map(BrowserScrollCommand::name),
            VENTURE_SCROLL_COMMAND_NAMES
        );
        for command in commands {
            assert_eq!(
                BrowserScrollCommand::from_name(command.name()),
                Some(command)
            );
        }
        assert_eq!(BrowserScrollCommand::from_name("page-sideways"), None);

        let mut scroll = ScrollState::new(100.0, 260.0);
        assert_eq!(scroll.apply_command(BrowserScrollCommand::LineDown), 40.0);
        assert_eq!(scroll.apply_command(BrowserScrollCommand::PageDown), 130.0);
        assert_eq!(
            scroll.apply_command(BrowserScrollCommand::DocumentEnd),
            160.0
        );
        assert_eq!(scroll.apply_command(BrowserScrollCommand::LineDown), 160.0);
        assert_eq!(scroll.apply_command(BrowserScrollCommand::PageUp), 70.0);
        assert_eq!(scroll.apply_command(BrowserScrollCommand::LineUp), 30.0);
        assert_eq!(
            scroll.apply_command(BrowserScrollCommand::DocumentStart),
            0.0
        );
    }

    #[test]
    fn scroll_state_hit_tests_content_and_wraps_a_translated_viewport_scene() {
        let link = LinkRegion {
            x: 10.0,
            y: 80.0,
            width: 30.0,
            height: 12.0,
            url: "http://example.test/next".into(),
            fixed: false,
            clips: Vec::new(),
        };
        let mut scroll = ScrollState::new(60.0, 140.0);
        scroll.set_offset_y(60.0);
        assert_eq!(
            scroll.hit_test(std::slice::from_ref(&link), 10.0, 20.0),
            Some(&link)
        );

        let mut document = PaintScene::new(100.0, 140.0);
        document.background = "rgb(192, 192, 192)".into();
        document.instructions.push(PaintInstruction::Rect(
            paint_instructions::PaintRect::filled(0.0, 70.0, 20.0, 20.0, "#000000"),
        ));
        let viewport = scrolled_viewport_scene(&document, &scroll);

        assert_eq!(viewport.width, 100.0);
        assert_eq!(viewport.height, 60.0);
        assert_eq!(viewport.background, document.background);
        let [PaintInstruction::Group(group)] = viewport.instructions.as_slice() else {
            panic!("viewport should contain one translated group");
        };
        assert_eq!(group.transform, Some([1.0, 0.0, 0.0, 1.0, 0.0, -60.0]));
        assert_eq!(group.children, document.instructions);
    }

    #[test]
    fn viewport_scene_keeps_fixed_content_and_clamps_sticky_content() {
        let sticky = PaintInstruction::Group(PaintGroup {
            base: PaintBase {
                id: None,
                metadata: Some(std::collections::HashMap::from([
                    ("layout.position".into(), "sticky".into()),
                    ("layout.sticky.top".into(), "5".into()),
                    ("layout.sticky.y".into(), "20".into()),
                ])),
            },
            children: Vec::new(),
            transform: None,
            opacity: None,
        });
        let fixed = PaintInstruction::Group(PaintGroup {
            base: PaintBase {
                id: None,
                metadata: Some(std::collections::HashMap::from([(
                    "layout.position".into(),
                    "fixed".into(),
                )])),
            },
            children: Vec::new(),
            transform: None,
            opacity: None,
        });
        let scene = PaintScene {
            width: 100.0,
            height: 300.0,
            background: "white".into(),
            instructions: vec![
                sticky,
                PaintInstruction::Group(PaintGroup {
                    base: PaintBase::default(),
                    children: vec![fixed],
                    transform: None,
                    opacity: None,
                }),
            ],
            id: None,
            metadata: None,
        };
        let mut scroll = ScrollState::new(80.0, 300.0);
        scroll.set_offset_y(40.0);
        let viewport = scrolled_viewport_scene(&scene, &scroll);

        assert_eq!(viewport.instructions.len(), 2);
        let PaintInstruction::Group(document) = &viewport.instructions[0] else {
            panic!("expected translated document group");
        };
        assert_eq!(document.transform.unwrap()[5], -40.0);
        let PaintInstruction::Group(sticky) = &document.children[0] else {
            panic!("expected sticky group");
        };
        assert_eq!(sticky.transform.unwrap()[5], 25.0);
        assert!(is_fixed_instruction(&viewport.instructions[1]));
    }

    #[test]
    fn redirected_html_and_image_fetch_reach_cairo_pixels() {
        let mut source_pixels = PixelContainer::new(2, 2);
        source_pixels.fill(255, 0, 255, 255);
        let gif = encode_gif(&source_pixels);
        let requested = "http://example.test/start";
        let final_url = "http://example.test/guide/index.html";
        let fetched_urls = RefCell::new(Vec::new());
        let fetcher = |url: &str| {
            fetched_urls.borrow_mut().push(url.to_string());
            match url {
                "http://example.test/start" => Ok(BrowserFetchResponse::new(
                    final_url,
                    200,
                    Some("text/html; charset=utf-8".into()),
                    b"<title>Venture guide</title><h1>Venture</h1>\
                      <p><a href='next.html'>Next</a></p>\
                      <img src='logo.gif' alt='logo' width='20' height='20'>"
                        .to_vec(),
                )),
                "http://example.test/guide/logo.gif" => Ok(BrowserFetchResponse::new(
                    url,
                    200,
                    Some("image/gif".into()),
                    gif.clone(),
                )),
                _ => Err(format!("unexpected URL {url}")),
            }
        };

        let theme = mosaic_html_theme();
        let pipeline = BrowserPagePipeline::new(
            &theme,
            HtmlPaintViewport::new(220.0, 100.0, 1.0),
            &MonoMeasurer,
            &FakeShaper,
            &FakeMetrics,
            &FakeResolver,
        );
        let page = pipeline
            .load(requested, &fetcher)
            .expect("canned navigation should load");

        assert_eq!(page.requested_url, requested);
        assert_eq!(page.final_url, final_url);
        assert_eq!(page.document.title.as_deref(), Some("Venture guide"));
        assert!(page.image_failures.is_empty());
        assert_eq!(page.paint.links.len(), 1);
        assert_eq!(
            page.paint.links[0].url,
            "http://example.test/guide/next.html"
        );
        assert!(page.paint.scene.instructions.iter().any(|instruction| {
            matches!(
                instruction,
                PaintInstruction::Image(image)
                    if matches!(&image.src, ImageSrc::Pixels(pixels)
                        if pixels.width == 2 && pixels.height == 2)
            )
        }));
        assert_eq!(
            fetched_urls.into_inner(),
            vec![
                "http://example.test/start",
                "http://example.test/guide/logo.gif"
            ]
        );

        let pixels =
            paint_vm_cairo::render(&page.paint.scene).expect("page scene should rasterize");
        assert_eq!(pixels.width, 220);
        assert_eq!(
            pixels.data.len(),
            pixels.width as usize * pixels.height as usize * 4
        );
        assert!(pixels
            .data
            .as_chunks::<4>()
            .0
            .iter()
            .any(|pixel| *pixel != [192, 192, 192, 255]));

        let link = page.paint.links[0].clone();
        let mut viewport = BrowserViewport::new(page, 40.0);
        let offset = viewport.set_scroll_offset_y(link.y);
        let viewport_y = link.y - offset + link.height / 2.0;
        assert_eq!(
            viewport.hit_test_link(link.x + link.width / 2.0, viewport_y),
            Some(&link)
        );
        let scene = viewport.viewport_scene();
        assert_eq!(scene.height, 40.0);
        let [PaintInstruction::Group(group)] = scene.instructions.as_slice() else {
            panic!("browser viewport should project one translated group");
        };
        assert_eq!(group.transform, Some([1.0, 0.0, 0.0, 1.0, 0.0, -offset]));

        viewport.scroll_by(100.0);
        let mut replacement = viewport.page().clone();
        replacement.final_url = "http://example.test/replacement".into();
        replacement.paint.scene.height = 20.0;
        viewport.replace_page(replacement);
        assert_eq!(viewport.page().final_url, "http://example.test/replacement");
        assert_eq!(viewport.scroll_state().offset_y(), 0.0);
        assert_eq!(viewport.scroll_state().content_height(), 20.0);
        assert_eq!(viewport.scroll_state().viewport_height(), 40.0);
    }

    #[test]
    fn async_images_request_once_and_repaint_from_out_of_order_completions() {
        let mut source_pixels = PixelContainer::new(2, 2);
        source_pixels.fill(255, 0, 255, 255);
        let gif = encode_gif(&source_pixels);
        let document_fetches = RefCell::new(Vec::new());
        let fetcher = |url: &str| {
            document_fetches.borrow_mut().push(url.to_string());
            assert_eq!(url, "http://example.test/page.html");
            Ok(BrowserFetchResponse::new(
                url,
                200,
                Some("text/html".into()),
                b"<img src='a.gif' alt='a' width='10' height='10'>\
                  <img src='b.gif' alt='b' width='10' height='10'>\
                  <img src='a.gif' alt='again' width='10' height='10'>"
                    .to_vec(),
            ))
        };
        let theme = mosaic_html_theme();
        let pipeline = BrowserPagePipeline::new(
            &theme,
            HtmlPaintViewport::new(120.0, 40.0, 1.0),
            &MonoMeasurer,
            &FakeShaper,
            &FakeMetrics,
            &FakeResolver,
        );
        let mut session = BrowserSession::new("http://example.test/page.html", 40.0);
        let update = session
            .begin_execute(BrowserNavigation::Home, &pipeline, &fetcher)
            .expect("document should commit before images");

        assert_eq!(document_fetches.into_inner().len(), 1);
        assert_eq!(
            update
                .requests
                .iter()
                .map(|request| request.url.as_str())
                .collect::<Vec<_>>(),
            vec!["http://example.test/a.gif", "http://example.test/b.gif"]
        );
        assert!(update.cancelled.is_empty());
        assert_eq!(session.pending_subresource_requests(), update.requests);
        assert!(session.viewport().unwrap().page().image_failures.is_empty());

        let failed_completion = update.requests[1]
            .resolve(&|_: &str| -> Result<BrowserFetchResponse, String> { Err("offline".into()) });
        let failed = session.complete_subresource(failed_completion, &pipeline);
        assert_eq!(failed.disposition, BrowserSubresourceDisposition::Applied);
        assert!(failed.repaint_required);
        assert_eq!(failed.pending_count, 1);
        assert_eq!(session.viewport().unwrap().page().image_failures.len(), 1);

        let loaded_completion = update.requests[0].resolve(&|_: &str| {
            Ok(BrowserFetchResponse::new(
                "http://cdn.example.test/a.gif",
                200,
                Some("image/gif".into()),
                gif.clone(),
            ))
        });
        let loaded = session.complete_subresource(loaded_completion, &pipeline);
        assert_eq!(loaded.pending_count, 0);
        let page = session.viewport().unwrap().page();
        assert_eq!(
            count_pixel_images(&page.paint.scene.instructions),
            2,
            "one completion should repaint every duplicate URL"
        );

        let duplicate = session.complete_subresource(
            BrowserSubresourceCompletion {
                request: update.requests[0].clone(),
                result: Err(BrowserSubresourceError::Image(
                    HtmlImageResourceError::Fetch {
                        uri: update.requests[0].url.clone(),
                        message: "late duplicate".into(),
                    },
                )),
            },
            &pipeline,
        );
        assert_eq!(
            duplicate.disposition,
            BrowserSubresourceDisposition::IgnoredDuplicate
        );
        assert!(!duplicate.repaint_required);
    }

    #[test]
    fn decoded_image_intrinsics_reflow_replaced_geometry() {
        let fetcher = |url: &str| {
            assert_eq!(url, "http://example.test/page.html");
            Ok(BrowserFetchResponse::new(
                url,
                200,
                Some("text/html".into()),
                b"<img id='hero' src='hero.gif' alt='hero' style='max-width:80px'>".to_vec(),
            ))
        };
        let theme = mosaic_html_theme();
        let pipeline = BrowserPagePipeline::new(
            &theme,
            HtmlPaintViewport::new(160.0, 120.0, 1.0),
            &MonoMeasurer,
            &FakeShaper,
            &FakeMetrics,
            &FakeResolver,
        );
        let mut session = BrowserSession::new("http://example.test/page.html", 120.0);
        let update = session
            .begin_execute(BrowserNavigation::Home, &pipeline, &fetcher)
            .unwrap();
        let pending =
            positioned_by_id(&session.viewport().unwrap().page().paint.positioned, "hero").unwrap();
        assert_eq!((pending.width, pending.height), (80.0, 40.0));

        let mut pixels = PixelContainer::new(40, 80);
        pixels.fill(255, 0, 255, 255);
        let gif = encode_gif(&pixels);
        let completion = update.requests[0].resolve(&|_: &str| {
            Ok(BrowserFetchResponse::new(
                "http://example.test/hero.gif",
                200,
                Some("image/gif".into()),
                gif.clone(),
            ))
        });
        session.complete_subresource(completion, &pipeline);
        let ready =
            positioned_by_id(&session.viewport().unwrap().page().paint.positioned, "hero").unwrap();
        assert_eq!((ready.width, ready.height), (40.0, 80.0));
    }

    #[test]
    fn external_stylesheets_block_in_document_order_and_restyle_retained_page() {
        let fetcher = |url: &str| {
            assert_eq!(url, "http://example.test/page.html");
            Ok(BrowserFetchResponse::new(
                url,
                200,
                Some("text/html".into()),
                b"<link rel='stylesheet' href='a.css'>\
                  <style>p { color: red; }</style>\
                  <link rel='stylesheet' href='b.css' media='screen'>\
                  <link rel='stylesheet' href='print.css' media='print'>\
                  <p id='target'>Styled later</p>"
                    .to_vec(),
            ))
        };
        let theme = mosaic_html_theme();
        let pipeline = BrowserPagePipeline::new(
            &theme,
            HtmlPaintViewport::new(160.0, 60.0, 1.0),
            &MonoMeasurer,
            &FakeShaper,
            &FakeMetrics,
            &FakeResolver,
        );
        let mut session = BrowserSession::new("http://example.test/page.html", 60.0);
        let update = session
            .begin_execute(BrowserNavigation::Home, &pipeline, &fetcher)
            .unwrap();

        assert_eq!(
            update
                .requests
                .iter()
                .map(|request| (request.kind, request.url.as_str()))
                .collect::<Vec<_>>(),
            vec![
                (
                    BrowserSubresourceKind::Stylesheet,
                    "http://example.test/a.css"
                ),
                (
                    BrowserSubresourceKind::Stylesheet,
                    "http://example.test/b.css"
                ),
            ]
        );
        assert_eq!(
            positioned_text_color(
                &session.viewport().unwrap().page().paint.positioned,
                "Styled later"
            ),
            Some(layout_ir::rgb(0, 0, 0)),
            "inline rules after a pending blocking sheet must not jump ahead"
        );

        let later = update.requests[1].resolve(&|url: &str| {
            Ok(BrowserFetchResponse::new(
                url,
                200,
                Some("text/css".into()),
                b"p { color: green; }".to_vec(),
            ))
        });
        let later_update = session.complete_subresource(later, &pipeline);
        assert!(!later_update.repaint_required);

        let first = update.requests[0].resolve(&|url: &str| {
            Ok(BrowserFetchResponse::new(
                url,
                200,
                Some("text/css; charset=utf-8".into()),
                b"p { color: blue; }".to_vec(),
            ))
        });
        let first_update = session.complete_subresource(first, &pipeline);
        assert!(first_update.repaint_required);
        assert_eq!(first_update.pending_count, 0);
        let page = session.viewport().unwrap().page();
        assert_eq!(
            positioned_text_color(&page.paint.positioned, "Styled later"),
            Some(layout_ir::rgb(0, 128, 0)),
            "ordered author sheets must cascade after the earlier blocker settles"
        );
        assert!(page.stylesheet_failures.is_empty());
        assert!(matches!(
            page.stylesheet_resources[3].state,
            BrowserStylesheetResourceState::Inactive
        ));
    }

    #[test]
    fn imported_stylesheets_use_stable_requests_depth_first_cascade_and_cycle_diagnostics() {
        let fetcher = |url: &str| {
            assert_eq!(url, "http://example.test/page.html");
            Ok(BrowserFetchResponse::new(
                url,
                200,
                Some("text/html".into()),
                b"<link rel='stylesheet' href='main.css'>\
                  <style>@import 'inline.css';</style><p>Imported</p>"
                    .to_vec(),
            ))
        };
        let theme = mosaic_html_theme();
        let pipeline = BrowserPagePipeline::new(
            &theme,
            HtmlPaintViewport::new(480.0, 240.0, 1.0),
            &MonoMeasurer,
            &FakeShaper,
            &FakeMetrics,
            &FakeResolver,
        );
        let mut session = BrowserSession::new("http://example.test/page.html", 240.0);
        let initial = session
            .begin_execute(BrowserNavigation::Home, &pipeline, &fetcher)
            .unwrap();
        assert_eq!(
            initial
                .requests
                .iter()
                .map(|request| request.url.as_str())
                .collect::<Vec<_>>(),
            vec![
                "http://example.test/main.css",
                "http://example.test/inline.css"
            ]
        );

        let inline = initial.requests[1].resolve(&|url: &str| {
            Ok(BrowserFetchResponse::new(
                url,
                200,
                Some("text/css".into()),
                b"p { color: green; }".to_vec(),
            ))
        });
        assert!(
            !session
                .complete_subresource(inline, &pipeline)
                .repaint_required
        );

        let main = initial.requests[0].resolve(&|url: &str| {
            Ok(BrowserFetchResponse::new(
                url,
                200,
                Some("text/css".into()),
                b"@import 'base.css'; p { color: blue; }".to_vec(),
            ))
        });
        let main_update = session.complete_subresource(main, &pipeline);
        assert!(!main_update.repaint_required);
        let pending = session.pending_subresource_requests();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].url, "http://example.test/base.css");

        let base = pending[0].resolve(&|url: &str| {
            Ok(BrowserFetchResponse::new(
                url,
                200,
                Some("text/css".into()),
                b"@import 'main.css'; p { color: red; }".to_vec(),
            ))
        });
        let base_update = session.complete_subresource(base, &pipeline);
        assert!(base_update.repaint_required);
        assert_eq!(base_update.pending_count, 0);
        let page = session.viewport().unwrap().page();
        assert_eq!(
            positioned_text_color(&page.paint.positioned, "Imported"),
            Some(layout_ir::rgb(0, 128, 0)),
            "imports precede their parent and later DOM sheets still win"
        );
        assert!(page
            .stylesheet_failures
            .iter()
            .any(|error| matches!(error, BrowserStylesheetError::ImportCycle { url } if url == "http://example.test/main.css")));
    }

    #[test]
    fn link_media_queries_share_the_pipeline_viewport() {
        let fetcher = |url: &str| {
            assert_eq!(url, "http://example.test/page.html");
            Ok(BrowserFetchResponse::new(
                url,
                200,
                Some("text/html".into()),
                b"<link rel='stylesheet' href='narrow.css' media='screen and (max-width: 500px)'>\
                  <link rel='stylesheet' href='wide.css' media='screen and (min-width: 501px)'><p>Viewport</p>"
                    .to_vec(),
            ))
        };
        let theme = mosaic_html_theme();
        let pipeline = BrowserPagePipeline::new(
            &theme,
            HtmlPaintViewport::new(480.0, 240.0, 1.0),
            &MonoMeasurer,
            &FakeShaper,
            &FakeMetrics,
            &FakeResolver,
        );
        let mut session = BrowserSession::new("http://example.test/page.html", 240.0);
        let update = session
            .begin_execute(BrowserNavigation::Home, &pipeline, &fetcher)
            .unwrap();
        assert_eq!(update.requests.len(), 1);
        assert_eq!(update.requests[0].url, "http://example.test/narrow.css");
        assert!(matches!(
            session.viewport().unwrap().page().stylesheet_resources[1].state,
            BrowserStylesheetResourceState::Inactive
        ));
    }

    #[test]
    fn stylesheet_failure_unblocks_fallback_and_navigation_cancels_stale_work() {
        let fetcher = |url: &str| {
            let source = match url {
                "http://example.test/one.html" => {
                    "<link rel='stylesheet' href='bad.css'><link rel='stylesheet' href='good.css'><p>One</p>"
                }
                "http://example.test/two.html" => "<p>Two</p>",
                _ => panic!("subresources must stay on the scheduler: {url}"),
            };
            Ok(BrowserFetchResponse::new(
                url,
                200,
                Some("text/html".into()),
                source.as_bytes().to_vec(),
            ))
        };
        let theme = mosaic_html_theme();
        let pipeline = BrowserPagePipeline::new(
            &theme,
            HtmlPaintViewport::new(120.0, 40.0, 1.0),
            &MonoMeasurer,
            &FakeShaper,
            &FakeMetrics,
            &FakeResolver,
        );
        let mut session = BrowserSession::new("http://example.test/one.html", 40.0);
        let first = session
            .begin_execute(BrowserNavigation::Home, &pipeline, &fetcher)
            .unwrap();

        let good = first.requests[1].resolve(&|url: &str| {
            Ok(BrowserFetchResponse::new(
                url,
                200,
                Some("text/css".into()),
                b"p { color: green; }".to_vec(),
            ))
        });
        assert!(
            !session
                .complete_subresource(good, &pipeline)
                .repaint_required
        );
        let bad = first.requests[0].resolve(&|url: &str| {
            Ok(BrowserFetchResponse::new(
                url,
                503,
                Some("text/css".into()),
                Vec::new(),
            ))
        });
        assert!(
            session
                .complete_subresource(bad, &pipeline)
                .repaint_required
        );
        let page = session.viewport().unwrap().page();
        assert_eq!(page.stylesheet_failures.len(), 1);
        assert_eq!(
            positioned_text_color(&page.paint.positioned, "One"),
            Some(layout_ir::rgb(0, 128, 0))
        );

        let pending = session
            .begin_execute(BrowserNavigation::Reload, &pipeline, &fetcher)
            .unwrap();
        let next = session
            .begin_execute(
                BrowserNavigation::Navigate("http://example.test/two.html".into()),
                &pipeline,
                &fetcher,
            )
            .unwrap();
        assert_eq!(next.cancelled, pending.requests);
        let stale = session.complete_subresource(
            pending.requests[0].resolve(&|url: &str| {
                Ok(BrowserFetchResponse::new(
                    url,
                    200,
                    Some("text/css".into()),
                    b"p { color: red; }".to_vec(),
                ))
            }),
            &pipeline,
        );
        assert_eq!(
            stale.disposition,
            BrowserSubresourceDisposition::IgnoredStaleNavigation
        );
        assert!(!stale.repaint_required);
    }

    #[test]
    fn navigation_cancels_pending_images_and_ignores_stale_completion() {
        let fetcher = |url: &str| {
            let source = match url {
                "http://example.test/one.html" => "<img src='one.gif'>",
                "http://example.test/two.html" => "<p>Two</p>",
                _ => panic!("subresources must be scheduled, not fetched inline: {url}"),
            };
            Ok(BrowserFetchResponse::new(
                url,
                200,
                Some("text/html".into()),
                source.as_bytes().to_vec(),
            ))
        };
        let theme = mosaic_html_theme();
        let pipeline = BrowserPagePipeline::new(
            &theme,
            HtmlPaintViewport::new(100.0, 40.0, 1.0),
            &MonoMeasurer,
            &FakeShaper,
            &FakeMetrics,
            &FakeResolver,
        );
        let mut session = BrowserSession::new("http://example.test/one.html", 40.0);
        let first = session
            .begin_execute(BrowserNavigation::Home, &pipeline, &fetcher)
            .unwrap();
        let second = session
            .begin_execute(
                BrowserNavigation::Navigate("http://example.test/two.html".into()),
                &pipeline,
                &fetcher,
            )
            .unwrap();
        assert_eq!(second.cancelled, first.requests);
        assert!(second.requests.is_empty());
        let mut scheduler = RecordingScheduler::default();
        second.dispatch_to(&mut scheduler);
        assert_eq!(
            scheduler.0,
            vec!["cancel:http://example.test/one.gif"],
            "navigation effects must cancel old work before scheduling new work"
        );

        let stale = session.complete_subresource(
            BrowserSubresourceCompletion {
                request: first.requests[0].clone(),
                result: Err(BrowserSubresourceError::Image(
                    HtmlImageResourceError::Fetch {
                        uri: first.requests[0].url.clone(),
                        message: "arrived after cancellation".into(),
                    },
                )),
            },
            &pipeline,
        );
        assert_eq!(
            stale.disposition,
            BrowserSubresourceDisposition::IgnoredStaleNavigation
        );
        assert!(!stale.repaint_required);
        assert_eq!(
            session.viewport().unwrap().page().final_url,
            "http://example.test/two.html"
        );
    }

    #[test]
    fn session_reflows_retained_document_without_refetching_page() {
        let page_fetches = RefCell::new(0usize);
        let body = (0..40)
            .map(|index| {
                format!("<p>Venture resize paragraph {index} has enough words to wrap.</p>")
            })
            .collect::<String>();
        let fetcher = |url: &str| {
            assert_eq!(url, "http://example.test/");
            *page_fetches.borrow_mut() += 1;
            Ok(BrowserFetchResponse::new(
                url,
                200,
                Some("text/html".into()),
                format!("<title>Resize</title>{body}").into_bytes(),
            ))
        };

        let theme = mosaic_html_theme();
        let wide = BrowserPagePipeline::new(
            &theme,
            HtmlPaintViewport::new(320.0, 120.0, 1.0),
            &MonoMeasurer,
            &FakeShaper,
            &FakeMetrics,
            &FakeResolver,
        );
        let mut session = BrowserSession::new("http://example.test/", 120.0);
        session
            .execute(
                BrowserNavigation::Navigate("http://example.test/".into()),
                &wide,
                &fetcher,
            )
            .expect("initial page should load");
        session
            .viewport_mut()
            .expect("page should create a viewport")
            .scroll_by(80.0);
        let history = session.history().clone();
        let source = session
            .viewport()
            .expect("page should remain loaded")
            .page()
            .source
            .clone();

        let narrow = BrowserPagePipeline::new(
            &theme,
            HtmlPaintViewport::new(140.0, 72.0, 1.0),
            &MonoMeasurer,
            &FakeShaper,
            &FakeMetrics,
            &FakeResolver,
        );
        assert!(session.reflow(&narrow, &fetcher, 72.0).is_some());

        assert_eq!(*page_fetches.borrow(), 1, "resize must not refetch HTML");
        assert_eq!(session.history(), &history);
        let viewport = session
            .viewport()
            .expect("loaded document should remain after reflow");
        assert_eq!(viewport.page().source, source);
        assert_eq!(viewport.page().document.title.as_deref(), Some("Resize"));
        assert_eq!(viewport.viewport_scene().width, 140.0);
        assert_eq!(viewport.viewport_scene().height, 72.0);
        assert_eq!(viewport.scroll_state().viewport_height(), 72.0);
        assert!(viewport.scroll_state().offset_y() > 0.0);
        assert!(viewport.scroll_state().offset_y() <= viewport.scroll_state().max_offset_y());
    }

    #[test]
    fn broken_inline_image_is_recoverable() {
        let fetcher = |url: &str| match url {
            "http://example.test/" => Ok(BrowserFetchResponse::new(
                url,
                200,
                None,
                b"<img src='missing.gif' alt='missing' width='40' height='20'>".to_vec(),
            )),
            _ => Err("offline".into()),
        };

        let theme = mosaic_html_theme();
        let pipeline = BrowserPagePipeline::new(
            &theme,
            HtmlPaintViewport::new(100.0, 40.0, 1.0),
            &MonoMeasurer,
            &FakeShaper,
            &FakeMetrics,
            &FakeResolver,
        );
        let page = pipeline
            .load("http://example.test/", &fetcher)
            .expect("broken image should not fail page load");

        assert_eq!(page.image_failures.len(), 1);
        assert!(page.paint.scene.instructions.iter().all(|instruction| {
            !matches!(
                instruction,
                PaintInstruction::Image(image) if matches!(image.src, ImageSrc::Uri(_))
            )
        }));
    }

    #[test]
    fn rejects_http_errors_and_explicit_non_html_pages() {
        let theme = mosaic_html_theme();
        let pipeline = BrowserPagePipeline::new(
            &theme,
            HtmlPaintViewport::new(100.0, 40.0, 1.0),
            &MonoMeasurer,
            &FakeShaper,
            &FakeMetrics,
            &FakeResolver,
        );
        let not_found = |url: &str| {
            Ok(BrowserFetchResponse::new(
                url,
                404,
                Some("text/html".into()),
                Vec::new(),
            ))
        };
        assert_eq!(
            pipeline.load("http://example.test/missing", &not_found),
            Err(BrowserLoadError::HttpStatus {
                url: "http://example.test/missing".into(),
                status: 404,
            })
        );

        let image = |url: &str| {
            Ok(BrowserFetchResponse::new(
                url,
                200,
                Some("image/gif".into()),
                Vec::new(),
            ))
        };
        assert_eq!(
            pipeline.load("http://example.test/logo.gif", &image),
            Err(BrowserLoadError::UnsupportedMediaType {
                url: "http://example.test/logo.gif".into(),
                media_type: "image/gif".into(),
            })
        );
    }

    #[test]
    fn session_controls_share_pointer_keyboard_state_and_reflow() {
        let fetcher = |url: &str| {
            Ok(BrowserFetchResponse::new(
                url,
                200,
                Some("text/html".into()),
                b"<input id='q' value='go'><input id='off' disabled><input id='check' type='checkbox'>"
                    .to_vec(),
            ))
        };
        let theme = mosaic_html_theme();
        let pipeline = BrowserPagePipeline::new(
            &theme,
            HtmlPaintViewport::new(420.0, 160.0, 1.0),
            &MonoMeasurer,
            &FakeShaper,
            &FakeMetrics,
            &FakeResolver,
        );
        let mut session = BrowserSession::new("http://example.test/", 160.0);
        session
            .execute(BrowserNavigation::Home, &pipeline, &fetcher)
            .unwrap();

        let query = session.viewport().unwrap().page().paint.controls[0].clone();
        assert_eq!(
            session.activate_control(query.x + 1.0, query.y + 1.0, &pipeline),
            Some(ControlEffect::SelectionChanged {
                key: "control:0:id:q".into(),
                selection: ControlSelection::collapsed(0),
            })
        );
        assert!(matches!(
            session.control_text_input("!", &pipeline),
            Some(ControlEffect::ValueChanged { value, .. }) if value == "!go"
        ));
        assert_eq!(
            session.focus_control(false, &pipeline),
            Some(ControlEffect::Focused("control:2:id:check".into()))
        );
        assert_eq!(
            session.control_key_down(ControlKey::Space, &pipeline),
            Some(ControlEffect::CheckedChanged {
                key: "control:2:id:check".into(),
                checked: true,
            })
        );
        assert_eq!(session.controls().controls()[0].value, "!go");
        assert!(session.controls().controls()[2].checked);
        let off =
            positioned_by_id(&session.viewport().unwrap().page().paint.positioned, "off").unwrap();
        assert!(
            off.width > 0.0 && off.height > 0.0,
            "disabled control geometry: {off:?}"
        );
        assert_eq!(
            session
                .viewport()
                .unwrap()
                .page()
                .paint
                .controls
                .iter()
                .map(|control| control.key.as_str())
                .collect::<Vec<_>>(),
            vec!["control:0:id:q", "control:1:id:off", "control:2:id:check"]
        );
    }

    #[test]
    fn session_routes_label_activation_and_fieldset_disabledness_through_shared_controls() {
        let fetcher = |url: &str| {
            Ok(BrowserFetchResponse::new(
                url,
                200,
                Some("text/html".into()),
                b"<label for='q'>Search</label><input id='q' value='go'>\
                  <label><input id='check' type='checkbox'> Accept</label>\
                  <fieldset disabled><legend><label><input id='legend' type='checkbox'> Legend</label></legend>\
                  <label for='blocked'>Blocked</label><input id='blocked'></fieldset>"
                    .to_vec(),
            ))
        };
        let theme = mosaic_html_theme();
        let pipeline = BrowserPagePipeline::new(
            &theme,
            HtmlPaintViewport::new(520.0, 180.0, 1.0),
            &MonoMeasurer,
            &FakeShaper,
            &FakeMetrics,
            &FakeResolver,
        );
        let mut session = BrowserSession::new("http://example.test/", 180.0);
        session
            .execute(BrowserNavigation::Home, &pipeline, &fetcher)
            .unwrap();

        let label = |session: &BrowserSession, key: &str| {
            session
                .viewport()
                .unwrap()
                .page()
                .paint
                .controls
                .iter()
                .find(|region| region.key == key && region.label_activation)
                .cloned()
                .unwrap()
        };
        let query_label = label(&session, "control:0:id:q");
        assert_eq!(
            session.activate_control(query_label.x + 1.0, query_label.y + 1.0, &pipeline),
            Some(ControlEffect::Focused("control:0:id:q".into()))
        );
        assert_eq!(session.controls().focused_key(), Some("control:0:id:q"));

        let checkbox_label = label(&session, "control:1:id:check");
        assert_eq!(
            session.activate_control(
                checkbox_label.x + checkbox_label.width - 1.0,
                checkbox_label.y + 1.0,
                &pipeline,
            ),
            Some(ControlEffect::CheckedChanged {
                key: "control:1:id:check".into(),
                checked: true,
            })
        );
        let legend_label = label(&session, "control:2:id:legend");
        assert!(!legend_label.disabled);
        assert!(session
            .controls()
            .control("control:2:id:legend")
            .is_some_and(|control| !control.disabled));

        let blocked_label = label(&session, "control:3:id:blocked");
        assert!(blocked_label.disabled);
        assert_eq!(
            session.activate_control(blocked_label.x + 1.0, blocked_label.y + 1.0, &pipeline,),
            None
        );
    }

    #[test]
    fn session_projects_editor_geometry_pointer_clipboard_blink_and_ime_state() {
        let fetcher = |url: &str| {
            Ok(BrowserFetchResponse::new(
                url,
                200,
                Some("text/html".into()),
                b"<textarea id='notes' cols='8' rows='2'>abcdef\nsecond line</textarea>".to_vec(),
            ))
        };
        let theme = mosaic_html_theme();
        let pipeline = BrowserPagePipeline::new(
            &theme,
            HtmlPaintViewport::new(420.0, 160.0, 1.0),
            &MonoMeasurer,
            &FakeShaper,
            &FakeMetrics,
            &FakeResolver,
        );
        let mut session = BrowserSession::new("http://example.test/", 160.0);
        session
            .execute(BrowserNavigation::Home, &pipeline, &fetcher)
            .unwrap();
        let region = session.viewport().unwrap().page().paint.controls[0].clone();

        session
            .control_pointer_down(
                region.x + CONTROL_TEXT_METRICS.inset_x + 8.0,
                region.y + CONTROL_TEXT_METRICS.inset_y + 1.0,
                &pipeline,
            )
            .unwrap();
        session
            .control_pointer_drag(
                region.x + CONTROL_TEXT_METRICS.inset_x + 24.0,
                region.y + CONTROL_TEXT_METRICS.inset_y + 1.0,
                &pipeline,
            )
            .unwrap();
        session.control_pointer_up();
        assert_eq!(session.control_copy().as_deref(), Some("bc"));
        assert_eq!(session.control_cut(&pipeline).as_deref(), Some("bc"));
        session.control_paste("BC", &pipeline).unwrap();
        assert_eq!(
            session
                .controls()
                .control("control:0:id:notes")
                .unwrap()
                .value,
            "aBCdef\nsecond line"
        );

        session.control_update_composition("界", &pipeline).unwrap();
        let candidate = session.focused_ime_candidate_rect().unwrap();
        assert!(candidate.width > 0.0 && candidate.height > 0.0);
        let overlay = session
            .viewport()
            .unwrap()
            .page()
            .paint
            .scene
            .instructions
            .iter()
            .find_map(|instruction| match instruction {
                PaintInstruction::Group(group)
                    if group
                        .base
                        .id
                        .as_deref()
                        .is_some_and(|id| id.starts_with(EDITOR_OVERLAY_PREFIX)) =>
                {
                    Some(group)
                }
                _ => None,
            })
            .unwrap();
        assert!(
            overlay.children.len() >= 2,
            "caret and composition underline"
        );
        assert!(session.control_advance_caret_blink(500));
    }

    #[test]
    fn session_routes_advanced_editing_through_one_host_neutral_contract() {
        let fetcher = |url: &str| {
            Ok(BrowserFetchResponse::new(
                url,
                200,
                Some("text/html".into()),
                b"<textarea id='notes' cols='6' rows='1'>one two three four</textarea>".to_vec(),
            ))
        };
        let theme = mosaic_html_theme();
        let pipeline = BrowserPagePipeline::new(
            &theme,
            HtmlPaintViewport::new(320.0, 120.0, 1.0),
            &MonoMeasurer,
            &FakeShaper,
            &FakeMetrics,
            &FakeResolver,
        );
        let mut session = BrowserSession::new("http://example.test/", 120.0);
        session
            .execute(BrowserNavigation::Home, &pipeline, &fetcher)
            .unwrap();
        let region = session.viewport().unwrap().page().paint.controls[0].clone();

        session
            .control_pointer_down_with_click_count(
                region.x + CONTROL_TEXT_METRICS.inset_x + 5.0 * CONTROL_TEXT_METRICS.advance,
                region.y + CONTROL_TEXT_METRICS.inset_y + 1.0,
                2,
                &pipeline,
            )
            .unwrap();
        let payload = session.control_copy_payload().unwrap();
        assert_eq!(payload.plain_text.as_deref(), Some("two"));
        assert_eq!(payload.html.as_deref(), Some("<span>two</span>"));

        session
            .control_paste_payload(
                &ControlClipboardPayload {
                    plain_text: None,
                    html: Some("<strong>second</strong>".into()),
                },
                &pipeline,
            )
            .unwrap();
        assert_eq!(
            session
                .controls()
                .control("control:0:id:notes")
                .unwrap()
                .value,
            "one second three four"
        );
        session
            .control_accessibility_action(ControlAccessibilityAction::Undo, &pipeline)
            .unwrap();
        assert_eq!(
            session
                .controls()
                .control("control:0:id:notes")
                .unwrap()
                .value,
            "one two three four"
        );
        session
            .control_key_down_with_shift_and_submit(
                ControlKey::WordRight,
                true,
                &pipeline,
                &fetcher,
            )
            .unwrap();
        assert!(!session
            .controls()
            .editor("control:0:id:notes")
            .unwrap()
            .selection
            .is_collapsed());
    }

    #[test]
    fn session_routes_typed_number_state_and_value_actions_through_reflow() {
        let fetcher = |url: &str| {
            Ok(BrowserFetchResponse::new(
                url,
                200,
                Some("text/html".into()),
                b"<input id='count' type='number' min='0' max='6' step='2' value='3'>".to_vec(),
            ))
        };
        let theme = mosaic_html_theme();
        let pipeline = BrowserPagePipeline::new(
            &theme,
            HtmlPaintViewport::new(320.0, 120.0, 1.0),
            &MonoMeasurer,
            &FakeShaper,
            &FakeMetrics,
            &FakeResolver,
        );
        let mut session = BrowserSession::new("http://example.test/", 120.0);
        session
            .execute(BrowserNavigation::Home, &pipeline, &fetcher)
            .unwrap();
        let region = session.viewport().unwrap().page().paint.controls[0].clone();
        session
            .control_pointer_down(region.x + 1.0, region.y + 1.0, &pipeline)
            .unwrap();

        let initial = session.focused_control_value_state().unwrap();
        assert_eq!(initial.diagnostics[0].code, "step-mismatch");
        assert!(!initial.selection_supported);
        session
            .control_key_down(ControlKey::ArrowUp, &pipeline)
            .unwrap();
        let aligned = session.control_value_state("control:0:id:count").unwrap();
        assert_eq!(aligned.value, "4");
        assert!(aligned.is_valid());
        session
            .control_accessibility_action(ControlAccessibilityAction::Increment, &pipeline)
            .unwrap();
        assert_eq!(
            session
                .control_value_state("control:0:id:count")
                .unwrap()
                .numeric_value,
            Some(6.0)
        );
    }

    #[test]
    fn session_routes_choice_and_range_actions_through_shared_reflow() {
        let fetcher = |url: &str| {
            Ok(BrowserFetchResponse::new(
                url,
                200,
                Some("text/html".into()),
                b"<select id='tags' multiple><option value='a' selected>A</option><option value='b' disabled>B</option><option value='c'>C</option></select><input id='mixed' type='checkbox'><input id='level' type='range' min='0' max='10' step='2' value='4'>".to_vec(),
            ))
        };
        let theme = mosaic_html_theme();
        let pipeline = BrowserPagePipeline::new(
            &theme,
            HtmlPaintViewport::new(320.0, 180.0, 1.0),
            &MonoMeasurer,
            &FakeShaper,
            &FakeMetrics,
            &FakeResolver,
        );
        let mut session = BrowserSession::new("http://example.test/", 180.0);
        session
            .execute(BrowserNavigation::Home, &pipeline, &fetcher)
            .unwrap();
        let regions = session.viewport().unwrap().page().paint.controls.clone();

        let select = regions
            .iter()
            .find(|region| region.key.contains("tags"))
            .unwrap();
        session
            .control_pointer_down(select.x + 1.0, select.y + 1.0, &pipeline)
            .unwrap();
        session
            .control_accessibility_action(
                ControlAccessibilityAction::SelectOption {
                    index: 2,
                    extend: false,
                    toggle: true,
                },
                &pipeline,
            )
            .unwrap();
        let select_state = session.focused_control_choice_state().unwrap();
        assert_eq!(select_state.role, "listbox");
        assert!(select_state.options[1].disabled);
        assert!(select_state.options[2].selected);

        let checkbox = regions
            .iter()
            .find(|region| region.key.contains("mixed"))
            .unwrap();
        session
            .control_pointer_down(checkbox.x + 1.0, checkbox.y + 1.0, &pipeline)
            .unwrap();
        session
            .control_accessibility_action(
                ControlAccessibilityAction::SetIndeterminate(true),
                &pipeline,
            )
            .unwrap();
        assert!(
            session
                .focused_control_choice_state()
                .unwrap()
                .indeterminate
        );

        let range = regions
            .iter()
            .find(|region| region.key.contains("level"))
            .unwrap();
        session
            .control_pointer_down(range.x + 1.0, range.y + 1.0, &pipeline)
            .unwrap();
        session
            .control_accessibility_action(ControlAccessibilityAction::Increment, &pipeline)
            .unwrap();
        assert_eq!(
            session.focused_control_choice_state().unwrap().value,
            Some(6.0)
        );
    }

    #[test]
    fn session_routes_temporal_and_color_actions_through_shared_reflow() {
        let fetcher = |url: &str| {
            Ok(BrowserFetchResponse::new(
                url,
                200,
                Some("text/html".into()),
                b"<input id='day' type='date' min='2024-01-01' max='2024-01-09' step='2' value='2024-01-02'><input id='clock' type='time' value='09:30:05.120'><input id='ink' type='color' value='#A0b1C2'>".to_vec(),
            ))
        };
        let theme = mosaic_html_theme();
        let pipeline = BrowserPagePipeline::new(
            &theme,
            HtmlPaintViewport::new(420.0, 140.0, 1.0),
            &MonoMeasurer,
            &FakeShaper,
            &FakeMetrics,
            &FakeResolver,
        );
        let mut session = BrowserSession::new("http://example.test/", 140.0);
        session
            .execute(BrowserNavigation::Home, &pipeline, &fetcher)
            .unwrap();
        let regions = session.viewport().unwrap().page().paint.controls.clone();

        let date = regions
            .iter()
            .find(|region| region.key.contains("day"))
            .unwrap();
        session
            .control_pointer_down(date.x + 1.0, date.y + 1.0, &pipeline)
            .unwrap();
        assert_eq!(
            session.focused_control_value_state().unwrap().diagnostics[0].code,
            "step-mismatch"
        );
        session
            .control_key_down(ControlKey::ArrowUp, &pipeline)
            .unwrap();
        let date_state = session.focused_control_value_state().unwrap();
        assert_eq!(date_state.value_text.as_deref(), Some("2024-01-03"));
        assert!(date_state.is_valid());

        let color = regions
            .iter()
            .find(|region| region.key.contains("ink"))
            .unwrap();
        session
            .control_pointer_down(color.x + 1.0, color.y + 1.0, &pipeline)
            .unwrap();
        session
            .control_accessibility_action(
                ControlAccessibilityAction::SetValue("#00FF7f".into()),
                &pipeline,
            )
            .unwrap();
        assert_eq!(
            session
                .focused_control_value_state()
                .unwrap()
                .value_text
                .as_deref(),
            Some("#00ff7f")
        );
    }

    #[test]
    fn session_validates_resets_and_posts_forms_transactionally() {
        struct FormFetcher {
            requests: RefCell<Vec<BrowserFetchRequest>>,
        }

        impl BrowserResourceFetcher for FormFetcher {
            fn fetch(&self, url: &str) -> Result<BrowserFetchResponse, String> {
                self.fetch_request(&BrowserFetchRequest::get(url))
            }

            fn fetch_request(
                &self,
                request: &BrowserFetchRequest,
            ) -> Result<BrowserFetchResponse, String> {
                self.requests.borrow_mut().push(request.clone());
                let body = match request.url.as_str() {
                    "http://example.test/form" => b"<form method='post' action='/result'><input id='q' name='q' required><button id='reset' type='reset'>Reset</button><button id='submit' type='submit' name='intent' value='save'>Save</button></form>".to_vec(),
                    "http://example.test/result" => b"<title>Submitted</title><p>accepted</p>".to_vec(),
                    other => return Err(format!("unexpected form request {other}")),
                };
                Ok(BrowserFetchResponse::new(
                    request.url.clone(),
                    200,
                    Some("text/html".into()),
                    body,
                ))
            }
        }

        fn control_point(session: &BrowserSession, key: &str) -> (f64, f64) {
            let control = session
                .viewport()
                .unwrap()
                .page()
                .paint
                .controls
                .iter()
                .find(|control| control.key == key)
                .unwrap();
            (control.x + 1.0, control.y + 1.0)
        }

        fn node_by_id<'a>(
            nodes: &'a [BrowserRenderNode],
            id: &str,
        ) -> Option<&'a BrowserRenderNode> {
            nodes.iter().find_map(|node| {
                (node.id.as_deref() == Some(id))
                    .then_some(node)
                    .or_else(|| node_by_id(&node.children, id))
            })
        }

        let fetcher = FormFetcher {
            requests: RefCell::new(Vec::new()),
        };
        let theme = mosaic_html_theme();
        let pipeline = BrowserPagePipeline::new(
            &theme,
            HtmlPaintViewport::new(420.0, 160.0, 1.0),
            &MonoMeasurer,
            &FakeShaper,
            &FakeMetrics,
            &FakeResolver,
        );
        let mut session = BrowserSession::new("http://example.test/form", 160.0);
        session
            .execute(BrowserNavigation::Home, &pipeline, &fetcher)
            .unwrap();

        let submit = control_point(&session, "control:2:id:submit");
        session
            .activate_control_and_submit(submit.0, submit.1, &pipeline, &fetcher)
            .unwrap();
        assert_eq!(session.form_diagnostics()[0].code, "value-missing");
        assert_eq!(session.controls().focused_key(), Some("control:0:id:q"));
        assert_eq!(
            session
                .controls()
                .editor("control:0:id:q")
                .and_then(|editor| editor.invalid_message.as_deref()),
            Some("required control has no value")
        );
        let query_node = node_by_id(
            &session.viewport().unwrap().page().render_tree.children,
            "q",
        )
        .unwrap();
        assert!(query_node.control_focused);
        assert_eq!(query_node.aria_invalid.as_deref(), Some("true"));
        assert_eq!(session.history().back_stack().len(), 0);
        assert_eq!(fetcher.requests.borrow().len(), 1);

        let query = control_point(&session, "control:0:id:q");
        session
            .activate_control_and_submit(query.0, query.1, &pipeline, &fetcher)
            .unwrap();
        session.control_text_input("venture", &pipeline).unwrap();
        assert!(session.form_diagnostics().is_empty());
        assert_eq!(
            session
                .controls()
                .editor("control:0:id:q")
                .unwrap()
                .invalid_message,
            None
        );
        let reset = control_point(&session, "control:1:id:reset");
        session
            .activate_control_and_submit(reset.0, reset.1, &pipeline, &fetcher)
            .unwrap();
        assert_eq!(session.controls().controls()[0].value, "");
        assert_eq!(fetcher.requests.borrow().len(), 1);

        let query = control_point(&session, "control:0:id:q");
        session
            .activate_control_and_submit(query.0, query.1, &pipeline, &fetcher)
            .unwrap();
        session.control_text_input("venture", &pipeline).unwrap();
        let submit = control_point(&session, "control:2:id:submit");
        session
            .activate_control_and_submit(submit.0, submit.1, &pipeline, &fetcher)
            .unwrap();

        assert_eq!(
            session.history().current_url(),
            Some("http://example.test/result")
        );
        assert_eq!(
            session.history().back_stack(),
            &["http://example.test/form"]
        );
        let request = fetcher.requests.borrow()[1].clone();
        assert_eq!(request.method, BrowserFetchMethod::Post);
        assert_eq!(
            request.content_type.as_deref(),
            Some("application/x-www-form-urlencoded")
        );
        assert_eq!(request.body, b"q=venture&intent=save");
        assert_eq!(
            session.viewport().unwrap().page().document.title.as_deref(),
            Some("Submitted")
        );
    }

    #[test]
    fn session_routes_pointer_keyboard_and_accessible_image_submission() {
        struct ImageFormFetcher {
            requests: RefCell<Vec<BrowserFetchRequest>>,
        }

        impl BrowserResourceFetcher for ImageFormFetcher {
            fn fetch(&self, url: &str) -> Result<BrowserFetchResponse, String> {
                self.fetch_request(&BrowserFetchRequest::get(url))
            }

            fn fetch_request(
                &self,
                request: &BrowserFetchRequest,
            ) -> Result<BrowserFetchResponse, String> {
                self.requests.borrow_mut().push(request.clone());
                let body = if request.url == "http://example.test/form" {
                    b"<form action='/map' dir='rtl'><input id='q' name='q' dirname='q.dir' dir='auto' value='\xD7\xA9\xD7\x9C\xD7\x95\xD7\x9D'><input id='pin' type='image' name='pin' alt='Choose location'></form>".to_vec()
                } else {
                    b"<title>Submitted</title>".to_vec()
                };
                Ok(BrowserFetchResponse::new(
                    request.url.clone(),
                    200,
                    Some("text/html".into()),
                    body,
                ))
            }
        }

        let theme = mosaic_html_theme();
        let pipeline = BrowserPagePipeline::new(
            &theme,
            HtmlPaintViewport::new(420.0, 140.0, 1.0),
            &MonoMeasurer,
            &FakeShaper,
            &FakeMetrics,
            &FakeResolver,
        );

        let pointer_fetcher = ImageFormFetcher {
            requests: RefCell::new(Vec::new()),
        };
        let mut pointer_session = BrowserSession::new("http://example.test/form", 140.0);
        pointer_session
            .execute(BrowserNavigation::Home, &pipeline, &pointer_fetcher)
            .unwrap();
        let image = pointer_session
            .viewport()
            .unwrap()
            .page()
            .paint
            .controls
            .iter()
            .find(|region| region.key.contains("pin"))
            .unwrap();
        let effect = pointer_session
            .activate_control_and_submit(image.x + 1.9, image.y + 1.2, &pipeline, &pointer_fetcher)
            .unwrap();
        assert!(
            matches!(effect, Some(ControlEffect::Activated(_))),
            "{effect:?}"
        );
        assert_eq!(
            pointer_fetcher.requests.borrow()[1].url,
            "http://example.test/map?q=%D7%A9%D7%9C%D7%95%D7%9D&q.dir=rtl&pin.x=1&pin.y=1"
        );

        let keyboard_fetcher = ImageFormFetcher {
            requests: RefCell::new(Vec::new()),
        };
        let mut keyboard_session = BrowserSession::new("http://example.test/form", 140.0);
        keyboard_session
            .execute(BrowserNavigation::Home, &pipeline, &keyboard_fetcher)
            .unwrap();
        keyboard_session.focus_control(false, &pipeline);
        keyboard_session.focus_control(false, &pipeline);
        keyboard_session
            .control_key_down_and_submit(ControlKey::Enter, &pipeline, &keyboard_fetcher)
            .unwrap();
        assert!(keyboard_fetcher.requests.borrow()[1]
            .url
            .ends_with("&pin.x=0&pin.y=0"));

        let accessible_fetcher = ImageFormFetcher {
            requests: RefCell::new(Vec::new()),
        };
        let mut accessible_session = BrowserSession::new("http://example.test/form", 140.0);
        accessible_session
            .execute(BrowserNavigation::Home, &pipeline, &accessible_fetcher)
            .unwrap();
        accessible_session.focus_control(false, &pipeline);
        accessible_session.focus_control(false, &pipeline);
        assert!(matches!(
            accessible_session
                .control_accessibility_action_and_submit(
                    ControlAccessibilityAction::Activate,
                    &pipeline,
                    &accessible_fetcher,
                )
                .unwrap(),
            Some(ControlEffect::Activated(_))
        ));
        assert!(accessible_fetcher.requests.borrow()[1]
            .url
            .ends_with("&pin.x=0&pin.y=0"));
    }

    fn scene_fill_count(viewport: &BrowserViewport, fill: &str) -> usize {
        viewport
            .page()
            .paint
            .scene
            .instructions
            .iter()
            .filter(|instruction| match instruction {
                PaintInstruction::GlyphRun(run) => run.fill.as_deref() == Some(fill),
                PaintInstruction::Rect(rect) => rect.fill.as_deref() == Some(fill),
                _ => false,
            })
            .count()
    }

    fn assert_scene_has_fill(viewport: &BrowserViewport, fill: &str) {
        assert!(
            scene_fill_count(viewport, fill) > 0,
            "expected scene to contain {fill}"
        );
    }

    struct MonoMeasurer;

    impl TextMeasurer for MonoMeasurer {
        fn measure(&self, text: &str, font: &FontSpec, max_width: Option<f64>) -> MeasureResult {
            let char_width = font.size * 0.5;
            let full_width = text.chars().count() as f64 * char_width;
            let width_limit = max_width.unwrap_or(full_width).max(char_width);
            let line_count = (full_width / width_limit).ceil().max(1.0);
            MeasureResult {
                width: full_width.min(width_limit),
                height: line_count * font.size * font.line_height,
                baseline: font.size * 0.8,
                line_count: line_count as u32,
            }
        }
    }

    #[derive(Clone)]
    struct FakeHandle;

    struct FakeResolver;

    impl FontResolver for FakeResolver {
        type Handle = FakeHandle;

        fn resolve(&self, query: &FontQuery) -> Result<Self::Handle, FontResolutionError> {
            if query.family_names.is_empty() {
                Err(FontResolutionError::EmptyQuery)
            } else {
                Ok(FakeHandle)
            }
        }
    }

    struct FakeMetrics;

    impl FontMetrics for FakeMetrics {
        type Handle = FakeHandle;

        fn units_per_em(&self, _: &Self::Handle) -> u32 {
            1000
        }

        fn ascent(&self, _: &Self::Handle) -> i32 {
            800
        }

        fn descent(&self, _: &Self::Handle) -> i32 {
            200
        }

        fn line_gap(&self, _: &Self::Handle) -> i32 {
            0
        }

        fn x_height(&self, _: &Self::Handle) -> Option<i32> {
            Some(500)
        }

        fn cap_height(&self, _: &Self::Handle) -> Option<i32> {
            Some(700)
        }

        fn family_name(&self, _: &Self::Handle) -> String {
            "Fake".into()
        }
    }

    struct FakeShaper;

    impl TextShaper for FakeShaper {
        type Handle = FakeHandle;

        fn shape(
            &self,
            text: &str,
            _: &Self::Handle,
            size: f32,
            options: &ShapeOptions,
        ) -> Result<ShapedText, ShapingError> {
            if options.direction != Direction::Ltr {
                return Err(ShapingError::UnsupportedDirection(options.direction));
            }
            let advance = size / 2.0;
            let glyphs: Vec<_> = text
                .char_indices()
                .map(|(cluster, character)| Glyph {
                    glyph_id: character as u32,
                    cluster: cluster as u32,
                    x_advance: advance,
                    y_advance: 0.0,
                    x_offset: 0.0,
                    y_offset: 0.0,
                })
                .collect();
            Ok(ShapedText::single(ShapedRun {
                x_advance_total: glyphs.len() as f32 * advance,
                glyphs,
                font_ref: "fake:mosaic".into(),
            }))
        }

        fn font_ref(&self, _: &Self::Handle) -> String {
            "fake:mosaic".into()
        }
    }

    #[test]
    fn session_exposes_custom_element_internals_to_every_host_adapter() {
        struct CustomFormFetcher {
            requests: RefCell<Vec<BrowserFetchRequest>>,
        }

        impl BrowserResourceFetcher for CustomFormFetcher {
            fn fetch(&self, url: &str) -> Result<BrowserFetchResponse, String> {
                self.fetch_request(&BrowserFetchRequest::get(url))
            }

            fn fetch_request(
                &self,
                request: &BrowserFetchRequest,
            ) -> Result<BrowserFetchResponse, String> {
                self.requests.borrow_mut().push(request.clone());
                let body = if request.url.starts_with("http://example.test/save") {
                    b"<title>Saved</title>".to_vec()
                } else {
                    b"<label for='rating'>Rating</label><form id='review' action='/save'>\
                      <input name='before' value='a'>\
                      <x-rating id='rating' name='score' role='slider'></x-rating>\
                      <input name='after' value='z'>\
                      <button id='submit'>Save</button></form>"
                        .to_vec()
                };
                Ok(BrowserFetchResponse::new(
                    request.url.clone(),
                    200,
                    Some("text/html".into()),
                    body,
                ))
            }
        }

        let fetcher = CustomFormFetcher {
            requests: RefCell::new(Vec::new()),
        };
        let theme = mosaic_html_theme();
        let pipeline = BrowserPagePipeline::new(
            &theme,
            HtmlPaintViewport::new(420.0, 180.0, 1.0),
            &MonoMeasurer,
            &FakeShaper,
            &FakeMetrics,
            &FakeResolver,
        );
        let mut session = BrowserSession::new("http://example.test/form", 180.0);
        session
            .execute(BrowserNavigation::Home, &pipeline, &fetcher)
            .unwrap();

        let key = session.form_associated_custom_elements()[0].key.clone();
        session.attach_form_associated_custom_element(&key).unwrap();
        session
            .set_custom_element_form_value(
                &key,
                Some(CustomElementFormValue::Text("4".into())),
                Some(CustomElementFormValue::Text("rating:4".into())),
            )
            .unwrap();
        session
            .set_custom_element_accessibility_value(
                &key,
                CustomElementAccessibilityValue {
                    value_text: Some("4 of 5".into()),
                    minimum: Some(1.0),
                    maximum: Some(5.0),
                    step: Some(1.0),
                },
            )
            .unwrap();
        session
            .custom_element_accessibility_action(&key, CustomElementAccessibilityAction::Increment)
            .unwrap();
        let accessibility = session.custom_element_accessibility_state(&key).unwrap();
        assert_eq!(accessibility.name.as_deref(), Some("Rating"));
        assert_eq!(accessibility.value.as_deref(), Some("5"));

        let submit = session
            .viewport()
            .unwrap()
            .page()
            .paint
            .controls
            .iter()
            .find(|control| control.key == "control:2:id:submit")
            .unwrap()
            .clone();
        session
            .activate_control_and_submit(submit.x + 1.0, submit.y + 1.0, &pipeline, &fetcher)
            .unwrap();
        assert_eq!(
            fetcher.requests.borrow()[1].url,
            "http://example.test/save?before=a&score=5&after=z"
        );
    }

    #[test]
    fn session_dispatches_scripted_form_lifecycle_before_navigation() {
        struct LifecycleFetcher {
            requests: RefCell<Vec<BrowserFetchRequest>>,
        }

        impl BrowserResourceFetcher for LifecycleFetcher {
            fn fetch(&self, url: &str) -> Result<BrowserFetchResponse, String> {
                self.fetch_request(&BrowserFetchRequest::get(url))
            }

            fn fetch_request(
                &self,
                request: &BrowserFetchRequest,
            ) -> Result<BrowserFetchResponse, String> {
                self.requests.borrow_mut().push(request.clone());
                let body = if request.url.starts_with("http://example.test/save") {
                    b"<title>Saved</title>".to_vec()
                } else {
                    b"<form id='editor' action='/save'><input id='q' name='q' required>\
                      <button id='reset' type='reset'>Reset</button>\
                      <button id='submit' name='intent' value='save'>Save</button></form>"
                        .to_vec()
                };
                Ok(BrowserFetchResponse::new(
                    request.url.clone(),
                    200,
                    Some("text/html".into()),
                    body,
                ))
            }
        }

        let fetcher = LifecycleFetcher {
            requests: RefCell::new(Vec::new()),
        };
        let theme = mosaic_html_theme();
        let pipeline = BrowserPagePipeline::new(
            &theme,
            HtmlPaintViewport::new(420.0, 160.0, 1.0),
            &MonoMeasurer,
            &FakeShaper,
            &FakeMetrics,
            &FakeResolver,
        );
        let mut session = BrowserSession::new("http://example.test/form", 160.0);
        session
            .execute(BrowserNavigation::Home, &pipeline, &fetcher)
            .unwrap();

        assert!(!session
            .check_form_validity(0, |event| {
                event.prevent_default();
            })
            .unwrap());
        assert!(session.form_diagnostics().is_empty());
        assert!(matches!(
            session.take_form_lifecycle_events()[0],
            FormLifecycleEvent::Invalid {
                mode: FormValidationMode::Check,
                default_prevented: true,
                ..
            }
        ));

        assert!(!session.report_form_validity(0, |_| {}, &pipeline).unwrap());
        assert_eq!(session.form_diagnostics()[0].code, "value-missing");
        assert_eq!(session.controls().focused_key(), Some("control:0:id:q"));
        session.take_form_lifecycle_events();
        session.control_text_input("venture", &pipeline).unwrap();

        session
            .request_form_reset(
                0,
                |event| {
                    event.prevent_default();
                },
                &pipeline,
            )
            .unwrap();
        assert_eq!(session.controls().controls()[0].value, "venture");
        assert!(matches!(
            session.take_form_lifecycle_events()[0],
            FormLifecycleEvent::Reset {
                default_prevented: true,
                ..
            }
        ));

        session
            .request_submit(
                0,
                Some("control:2:id:submit"),
                |event| {
                    if let Some(entries) = event.form_data_mut() {
                        for entry in entries.iter_mut() {
                            if entry.name == "q" {
                                entry.value = FormDataValue::Text("scripted".into());
                            }
                        }
                        entries.push(FormDataEntry {
                            name: "phase".into(),
                            value: FormDataValue::Text("dispatch".into()),
                        });
                    }
                },
                &pipeline,
                &fetcher,
            )
            .unwrap();
        assert_eq!(
            fetcher.requests.borrow()[1].url,
            "http://example.test/save?q=scripted&intent=save&phase=dispatch"
        );
        let events = session.take_form_lifecycle_events();
        assert!(matches!(events[0], FormLifecycleEvent::Submit { .. }));
        assert!(matches!(events[1], FormLifecycleEvent::FormData { .. }));
    }

    #[test]
    fn session_autofill_and_history_restoration_share_one_state_policy() {
        let profile = "http://example.test/profile";
        let next = "http://example.test/next";
        let fetcher = |url: &str| {
            let body = if url == profile {
                b"<form autocomplete='on'>\
                  <input id='given' name='given-name' value='Ada' autocomplete='section-user given-name'>\
                  <input id='password' type='password' autocomplete='current-password'>\
                  <x-rating id='rating' name='rating'></x-rating></form>"
                    .to_vec()
            } else {
                b"<title>Next</title><p>Leave and return</p>".to_vec()
            };
            Ok(BrowserFetchResponse::new(
                url,
                200,
                Some("text/html".into()),
                body,
            ))
        };
        let theme = mosaic_html_theme();
        let pipeline = BrowserPagePipeline::new(
            &theme,
            HtmlPaintViewport::new(420.0, 160.0, 1.0),
            &MonoMeasurer,
            &FakeShaper,
            &FakeMetrics,
            &FakeResolver,
        );
        let mut session = BrowserSession::new(profile, 160.0);
        session
            .execute(BrowserNavigation::Home, &pipeline, &fetcher)
            .unwrap();

        let custom_key = session.form_associated_custom_elements()[0].key.clone();
        session
            .attach_form_associated_custom_element(&custom_key)
            .unwrap();
        session
            .set_custom_element_form_value(
                &custom_key,
                Some(CustomElementFormValue::Text("5".into())),
                Some(CustomElementFormValue::Text("restore:5".into())),
            )
            .unwrap();
        session.take_custom_element_lifecycle_events();
        let outcome = session.apply_control_autofill(
            &ControlAutofillTransaction {
                privacy: ControlStatePrivacy::Public,
                values: vec![
                    ControlAutofillValue {
                        section: Some("section-user".into()),
                        purpose: "given-name".into(),
                        value: "Grace".into(),
                    },
                    ControlAutofillValue {
                        section: None,
                        purpose: "current-password".into(),
                        value: "not-shared".into(),
                    },
                ],
            },
            &pipeline,
        );
        assert_eq!(outcome.effects.len(), 1);
        assert_eq!(
            session
                .controls()
                .control("control:0:id:given")
                .unwrap()
                .value,
            "Grace"
        );
        assert_eq!(
            session
                .controls()
                .control("control:1:id:password")
                .unwrap()
                .value,
            ""
        );
        assert!(
            session
                .control_default_state("control:0:id:given")
                .unwrap()
                .dirty_value
        );
        assert_eq!(
            session
                .take_control_mutation_events()
                .iter()
                .map(|event| event.kind)
                .collect::<Vec<_>>(),
            vec![
                ControlMutationEventKind::Input,
                ControlMutationEventKind::Change
            ]
        );

        session
            .execute(
                BrowserNavigation::Navigate(next.into()),
                &pipeline,
                &fetcher,
            )
            .unwrap();
        session
            .execute(BrowserNavigation::Back, &pipeline, &fetcher)
            .unwrap();
        assert_eq!(
            session
                .controls()
                .control("control:0:id:given")
                .unwrap()
                .value,
            "Grace"
        );
        let restored_custom_key = session.form_associated_custom_elements()[0].key.clone();
        session
            .attach_form_associated_custom_element(&restored_custom_key)
            .unwrap();
        assert!(session
            .take_custom_element_lifecycle_events()
            .iter()
            .any(|event| matches!(
                event,
                CustomElementLifecycleEvent::FormStateRestore {
                    mode: CustomElementStateRestoreMode::Restore,
                    state: CustomElementFormValue::Text(state),
                    ..
                } if state == "restore:5"
            )));
    }

    #[test]
    fn session_commits_datalist_choices_without_triggering_implicit_submission() {
        struct SuggestionFetcher {
            requests: RefCell<Vec<String>>,
        }

        impl BrowserResourceFetcher for SuggestionFetcher {
            fn fetch(&self, url: &str) -> Result<BrowserFetchResponse, String> {
                self.requests.borrow_mut().push(url.to_string());
                Ok(BrowserFetchResponse::new(
                    url,
                    200,
                    Some("text/html".into()),
                    b"<form action='/search'><input id='q' name='q' list='terms'>\
                      <datalist id='terms'><option value='alpha'><option value='beta'></datalist>\
                      <button>Search</button></form>"
                        .to_vec(),
                ))
            }
        }

        let fetcher = SuggestionFetcher {
            requests: RefCell::new(Vec::new()),
        };
        let theme = mosaic_html_theme();
        let pipeline = BrowserPagePipeline::new(
            &theme,
            HtmlPaintViewport::new(420.0, 160.0, 1.0),
            &MonoMeasurer,
            &FakeShaper,
            &FakeMetrics,
            &FakeResolver,
        );
        let mut session = BrowserSession::new("http://example.test/form", 160.0);
        session
            .execute(BrowserNavigation::Home, &pipeline, &fetcher)
            .unwrap();
        session.focus_control(false, &pipeline).unwrap();
        session
            .control_key_down(ControlKey::ArrowDown, &pipeline)
            .unwrap();

        assert_eq!(
            session
                .focused_control_suggestion_state()
                .unwrap()
                .active_index,
            Some(0)
        );
        assert!(matches!(
            session
                .control_key_down_and_submit(ControlKey::Enter, &pipeline, &fetcher)
                .unwrap(),
            Some(ControlEffect::SuggestionCommitted { value, .. }) if value == "alpha"
        ));
        assert_eq!(fetcher.requests.borrow().len(), 1);
        assert_eq!(
            session
                .take_control_mutation_events()
                .into_iter()
                .map(|event| (event.kind, event.source))
                .collect::<Vec<_>>(),
            vec![
                (
                    ControlMutationEventKind::Input,
                    ControlMutationSource::SuggestionPicker,
                ),
                (
                    ControlMutationEventKind::Change,
                    ControlMutationSource::SuggestionPicker,
                ),
            ]
        );
    }

    #[test]
    fn session_reflows_scripted_output_and_normalized_measurements() {
        struct LiveValueFetcher;

        impl BrowserResourceFetcher for LiveValueFetcher {
            fn fetch(&self, url: &str) -> Result<BrowserFetchResponse, String> {
                Ok(BrowserFetchResponse::new(
                    url,
                    200,
                    Some("text/html".into()),
                    b"<form><input id='quantity' value='4'><output id='total' for='quantity'>0</output></form>\
                      <meter id='quality' min='0' max='10' low='3' high='7' optimum='9' value='8'>8</meter>\
                      <progress id='load' max='5'>Loading</progress>"
                        .to_vec(),
                ))
            }
        }

        let theme = mosaic_html_theme();
        let pipeline = BrowserPagePipeline::new(
            &theme,
            HtmlPaintViewport::new(420.0, 160.0, 1.0),
            &MonoMeasurer,
            &FakeShaper,
            &FakeMetrics,
            &FakeResolver,
        );
        let mut session = BrowserSession::new("http://example.test/live", 160.0);
        session
            .execute(BrowserNavigation::Home, &pipeline, &LiveValueFetcher)
            .unwrap();

        session
            .recalculate_output(
                "live:0:id:total",
                |dependencies| format!("${}", dependencies[0].value),
                &pipeline,
            )
            .unwrap();
        session
            .set_live_value("live:2:id:load", Some("2"), &pipeline)
            .unwrap();
        assert_eq!(
            session.live_value_state("live:0:id:total").unwrap().text,
            "$4"
        );
        assert_eq!(
            session
                .live_value_state("live:1:id:quality")
                .unwrap()
                .meter_region,
            Some(MeterValueRegion::Optimum)
        );
        assert_eq!(
            session.live_value_state("live:2:id:load").unwrap().position,
            Some(0.4)
        );
    }
}
