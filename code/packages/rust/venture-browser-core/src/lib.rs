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
pub use browser_navigation::{NavigationHistory, VisitedLinks, VisitedUrl};
use coding_adventures_html_parser::{parse_html, BrowserDocument, BrowserRenderTree};
use html_to_layout::{HtmlAuthorStylesheet, HtmlStyleContext, HtmlTheme};
use html_to_paint::{
    decode_image_resource, hit_test_link, html_render_tree_to_paint_with_style_context,
    resolve_scene_image_resources_incrementally, scene_image_resource_uris, FetchedImage,
    HtmlImageResolver, HtmlImageResource, HtmlImageResourceError, HtmlPaintOutput,
    HtmlPaintViewport, LinkRegion,
};
use http1_client::HttpClient;
use layout_ir::TextMeasurer;
use paint_instructions::{PaintBase, PaintGroup, PaintInstruction, PaintScene, PixelContainer};
use std::fmt;
use text_interfaces::{FontMetrics, FontResolver, TextShaper};
use url_parser::Url;

pub const VERSION: &str = "0.8.0";

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
    PaintScene {
        width: scene.width,
        height: scroll.viewport_height,
        background: scene.background.clone(),
        instructions: vec![PaintInstruction::Group(PaintGroup {
            base: PaintBase::default(),
            children: scene.instructions.clone(),
            transform: Some([1.0, 0.0, 0.0, 1.0, 0.0, -scroll.offset_y]),
            opacity: None,
        })],
        id: scene.id.clone(),
        metadata: scene.metadata.clone(),
    }
}

/// One resource returned by a browser-owned transport.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrowserFetchResponse {
    pub final_url: String,
    pub status: u16,
    pub media_type: Option<String>,
    pub body: Vec<u8>,
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
    pub media: Option<String>,
    pub render_blocking: bool,
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
        let mut requests = page
            .stylesheet_resources
            .iter()
            .enumerate()
            .filter(|(_, resource)| {
                matches!(resource.state, BrowserStylesheetResourceState::Pending)
            })
            .map(|(ordinal, resource)| BrowserSubresourceRequest {
                navigation_id: self.navigation_id,
                kind: BrowserSubresourceKind::Stylesheet,
                ordinal,
                url: resource
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
        history.replace_current(page.final_url.clone());
        if let Some(viewport) = self.viewport.as_mut() {
            viewport.replace_page(page);
        } else {
            self.viewport = Some(BrowserViewport::new(page, self.viewport_height));
        }
        self.history = history;
        self.visited_links = visited_links;
        self.navigation_id = self.navigation_id.wrapping_add(1).max(1);
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
        if completion.request.navigation_id != self.navigation_id {
            return BrowserSubresourceUpdate {
                disposition: BrowserSubresourceDisposition::IgnoredStaleNavigation,
                repaint_required: false,
                pending_count: self.pending_subresource_requests().len(),
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
                updated.stylesheet_resources[completion.request.ordinal].state =
                    match completion.result {
                        Ok(BrowserSubresourcePayload::Stylesheet(source)) => {
                            match HtmlAuthorStylesheet::parse(&source) {
                                Ok(_) => BrowserStylesheetResourceState::Ready(source),
                                Err(error) => BrowserStylesheetResourceState::Failed(
                                    BrowserStylesheetError::Parse {
                                        url: Some(completion.request.url.clone()),
                                        message: error.to_string(),
                                    },
                                ),
                            }
                        }
                        Err(BrowserSubresourceError::Stylesheet(error)) => {
                            BrowserStylesheetResourceState::Failed(error)
                        }
                        _ => return self.ignored_duplicate_update(),
                    };
                active_stylesheet_source_count(&updated.stylesheet_resources) != before
            }
        };
        let updated = pipeline.reflow_retained_with_visited(&updated, &self.visited_links);
        if let Some(viewport) = self.viewport.as_mut() {
            viewport.reflow_page(updated, self.viewport_height);
        }
        BrowserSubresourceUpdate {
            disposition: BrowserSubresourceDisposition::Applied,
            repaint_required,
            pending_count: self.pending_subresource_requests().len(),
        }
    }

    fn ignored_duplicate_update(&self) -> BrowserSubresourceUpdate {
        BrowserSubresourceUpdate {
            disposition: BrowserSubresourceDisposition::IgnoredDuplicate,
            repaint_required: false,
            pending_count: self.pending_subresource_requests().len(),
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BrowserLoadError {
    Fetch { url: String, message: String },
    HttpStatus { url: String, status: u16 },
    UnsupportedMediaType { url: String, media_type: String },
    Parse { url: String, message: String },
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
        }
    }
}

impl std::error::Error for BrowserLoadError {}

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
        let stylesheet_resources = stylesheet_resources_for_document(&document, &auxiliary.address);
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
        for resource in &mut page.stylesheet_resources {
            if matches!(resource.state, BrowserStylesheetResourceState::Pending) {
                let url = resource.url.as_deref().expect("pending stylesheet URL");
                resource.state = match fetch_browser_stylesheet(url, fetcher) {
                    Ok(source) => match HtmlAuthorStylesheet::parse(&source) {
                        Ok(_) => BrowserStylesheetResourceState::Ready(source),
                        Err(error) => {
                            BrowserStylesheetResourceState::Failed(BrowserStylesheetError::Parse {
                                url: Some(url.to_string()),
                                message: error.to_string(),
                            })
                        }
                    },
                    Err(error) => BrowserStylesheetResourceState::Failed(error),
                };
            }
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
        let response =
            document_fetcher
                .fetch(requested_url)
                .map_err(|message| BrowserLoadError::Fetch {
                    url: requested_url.to_string(),
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
        let stylesheet_resources =
            stylesheet_resources_for_document(&document, &response.final_url);
        let render_tree =
            BrowserRenderTree::from_document_with_document_url(&parsed, &response.final_url);
        let mut prospective_visited = visited_links.clone();
        let _ = prospective_visited.record(&response.final_url);
        let (style_context, stylesheet_failures) =
            style_context_for_resources(self.theme, &stylesheet_resources);
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
            requested_url: requested_url.to_string(),
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
        let (style_context, stylesheet_failures) =
            style_context_for_resources(self.theme, stylesheet_resources);
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
) -> Vec<BrowserStylesheetResource> {
    document
        .stylesheets
        .iter()
        .map(|stylesheet| {
            let active = !stylesheet.disabled
                && !stylesheet.alternate
                && stylesheet_media_applies(stylesheet.media.as_deref());
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
                url,
                media: stylesheet.media.clone(),
                render_blocking: active && stylesheet.href.is_some(),
                state,
            }
        })
        .collect()
}

fn resolve_subresource_url(document_url: &str, resource_url: &str) -> Option<String> {
    Url::parse(document_url)
        .and_then(|base| base.resolve(resource_url))
        .map(|url| url.to_url_string())
        .ok()
        .or_else(|| resource_url.contains(':').then(|| resource_url.to_string()))
}

fn stylesheet_media_applies(media: Option<&str>) -> bool {
    let Some(media) = media.map(str::trim).filter(|media| !media.is_empty()) else {
        return true;
    };
    media.split(',').any(|query| {
        let query = query.trim().to_ascii_lowercase();
        (query == "all" || query == "screen" || query.starts_with("screen "))
            && !query.starts_with("not ")
    })
}

fn active_stylesheet_source_count(resources: &[BrowserStylesheetResource]) -> usize {
    resources
        .iter()
        .take_while(|resource| !matches!(resource.state, BrowserStylesheetResourceState::Pending))
        .count()
}

fn style_context_for_resources(
    theme: &HtmlTheme,
    resources: &[BrowserStylesheetResource],
) -> (HtmlStyleContext, Vec<BrowserStylesheetError>) {
    let mut context = HtmlStyleContext::new(theme.clone());
    let mut failures = Vec::new();
    for resource in resources {
        match &resource.state {
            BrowserStylesheetResourceState::Ready(source) => {
                match HtmlAuthorStylesheet::parse(source) {
                    Ok(stylesheet) => context.author_stylesheets.push(stylesheet),
                    Err(error) => failures.push(BrowserStylesheetError::Parse {
                        url: resource.url.clone(),
                        message: error.to_string(),
                    }),
                }
            }
            BrowserStylesheetResourceState::Failed(error) => failures.push(error.clone()),
            BrowserStylesheetResourceState::Pending => break,
            BrowserStylesheetResourceState::Inactive => {}
        }
    }
    (context, failures)
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
}
