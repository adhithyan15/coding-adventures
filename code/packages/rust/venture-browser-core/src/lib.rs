//! Host-neutral navigation and page-loading orchestration for Venture.
//!
//! The platform shell owns windows, events, font implementations, and the
//! final paint backend. This crate composes the shared network, HTML, layout,
//! paint, and image-resource seams into one synchronous page load.

use coding_adventures_html_parser::{parse_html, BrowserDocument, BrowserRenderTree};
use html_to_layout::HtmlTheme;
use html_to_paint::{
    hit_test_link, html_render_tree_to_paint, resolve_scene_image_resources_with_mosaic_fallback,
    FetchedImage, HtmlImageResourceError, HtmlPaintOutput, HtmlPaintViewport, LinkRegion,
};
use http1_client::HttpClient;
use layout_ir::TextMeasurer;
use paint_instructions::{PaintBase, PaintGroup, PaintInstruction, PaintScene};
use std::fmt;
use text_interfaces::{FontMetrics, FontResolver, TextShaper};

pub const VERSION: &str = "0.6.0";

/// Mosaic `VentureChrome` slot names, in interface declaration order.
pub const VENTURE_CHROME_SLOT_NAMES: [&str; 6] = [
    "address",
    "page-title",
    "status-text",
    "back-disabled",
    "forward-disabled",
    "navigation-disabled",
];

/// Host-owned Mosaic node slot that mounts the native page renderer.
pub const VENTURE_CHROME_HOST_SURFACE_SLOT_NAME: &str = "content-surface";

/// Mosaic `VentureChrome` event names, in interface declaration order.
pub const VENTURE_CHROME_EVENT_NAMES: [&str; 6] = [
    "onBack",
    "onForward",
    "onHome",
    "onReload",
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

/// In-memory browser navigation state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NavigationHistory {
    home_url: String,
    back_stack: Vec<String>,
    current_url: Option<String>,
    forward_stack: Vec<String>,
}

impl NavigationHistory {
    pub fn new(home_url: impl Into<String>) -> Self {
        Self {
            home_url: home_url.into(),
            back_stack: Vec::new(),
            current_url: None,
            forward_stack: Vec::new(),
        }
    }

    pub fn with_current(home_url: impl Into<String>, current_url: impl Into<String>) -> Self {
        let mut history = Self::new(home_url);
        history.current_url = Some(current_url.into());
        history
    }

    pub fn home_url(&self) -> &str {
        &self.home_url
    }

    pub fn current_url(&self) -> Option<&str> {
        self.current_url.as_deref()
    }

    pub fn back_stack(&self) -> &[String] {
        &self.back_stack
    }

    pub fn forward_stack(&self) -> &[String] {
        &self.forward_stack
    }

    pub fn can_go_back(&self) -> bool {
        !self.back_stack.is_empty()
    }

    pub fn can_go_forward(&self) -> bool {
        !self.forward_stack.is_empty()
    }

    /// Record a new navigation and clear stale forward history.
    pub fn navigate(&mut self, url: impl Into<String>) -> &str {
        if let Some(current) = self.current_url.take() {
            self.back_stack.push(current);
        }
        self.current_url = Some(url.into());
        self.forward_stack.clear();
        self.current_url.as_deref().unwrap_or("")
    }

    pub fn back(&mut self) -> Option<&str> {
        let previous = self.back_stack.pop()?;
        if let Some(current) = self.current_url.replace(previous) {
            self.forward_stack.push(current);
        }
        self.current_url()
    }

    pub fn forward(&mut self) -> Option<&str> {
        let next = self.forward_stack.pop()?;
        if let Some(current) = self.current_url.replace(next) {
            self.back_stack.push(current);
        }
        self.current_url()
    }

    pub fn home(&mut self) -> &str {
        self.navigate(self.home_url.clone())
    }

    /// Return the URL that a host should fetch again without changing history.
    pub fn reload(&self) -> Option<&str> {
        self.current_url()
    }

    /// Replace the current entry after a redirect without creating history.
    pub fn replace_current(&mut self, final_url: impl Into<String>) -> Option<&str> {
        self.current_url.as_ref()?;
        self.current_url = Some(final_url.into());
        self.current_url()
    }
}

/// Vertical document scroll state in logical content coordinates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScrollState {
    offset_y: f64,
    viewport_height: f64,
    content_height: f64,
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

/// An event emitted by the shared Mosaic `VentureChrome` component.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BrowserChromeEvent {
    Back,
    Forward,
    Home,
    Reload,
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
    ) -> Option<BrowserNavigation> {
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
                (!address.is_empty()).then(|| BrowserNavigation::Navigate(address.to_string()))
            }
            BrowserChromeEvent::Back if session.history().can_go_back() => {
                Some(BrowserNavigation::Back)
            }
            BrowserChromeEvent::Forward if session.history().can_go_forward() => {
                Some(BrowserNavigation::Forward)
            }
            BrowserChromeEvent::Home => Some(BrowserNavigation::Home),
            BrowserChromeEvent::Reload if session.history().current_url().is_some() => {
                Some(BrowserNavigation::Reload)
            }
            BrowserChromeEvent::Back | BrowserChromeEvent::Forward | BrowserChromeEvent::Reload => {
                None
            }
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
            navigation_disabled,
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
    viewport: Option<BrowserViewport>,
    viewport_height: f64,
}

impl BrowserSession {
    pub fn new(home_url: impl Into<String>, viewport_height: f64) -> Self {
        Self {
            history: NavigationHistory::new(home_url),
            viewport: None,
            viewport_height: finite_non_negative(viewport_height),
        }
    }

    pub fn history(&self) -> &NavigationHistory {
        &self.history
    }

    pub fn viewport(&self) -> Option<&BrowserViewport> {
        self.viewport.as_ref()
    }

    pub fn viewport_mut(&mut self) -> Option<&mut BrowserViewport> {
        self.viewport.as_mut()
    }

    pub fn resize(&mut self, viewport_height: f64) -> f64 {
        self.viewport_height = finite_non_negative(viewport_height);
        self.viewport
            .as_mut()
            .map_or(0.0, |viewport| viewport.resize(self.viewport_height))
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
        fetcher: &F,
        viewport_height: f64,
    ) -> Option<&'session BrowserViewport>
    where
        F: BrowserResourceFetcher,
        M: TextMeasurer,
        S: TextShaper,
        FM: FontMetrics<Handle = S::Handle>,
        R: FontResolver<Handle = S::Handle>,
    {
        let page = pipeline.reflow(self.viewport.as_ref()?.page(), fetcher);
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
        let mut history = self.history.clone();
        let requested_url = match navigation {
            BrowserNavigation::Navigate(url) => Some(history.navigate(url).to_string()),
            BrowserNavigation::Back => history.back().map(str::to_owned),
            BrowserNavigation::Forward => history.forward().map(str::to_owned),
            BrowserNavigation::Home => Some(history.home().to_string()),
            BrowserNavigation::Reload => history.reload().map(str::to_owned),
        };
        let Some(requested_url) = requested_url else {
            return Ok(None);
        };

        let page = pipeline.load(&requested_url, fetcher)?;
        history.replace_current(page.final_url.clone());
        if let Some(viewport) = self.viewport.as_mut() {
            viewport.replace_page(page);
        } else {
            self.viewport = Some(BrowserViewport::new(page, self.viewport_height));
        }
        self.history = history;
        Ok(self.viewport.as_ref())
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
        let response = fetcher
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
        let render_tree =
            BrowserRenderTree::from_document_with_document_url(&parsed, &response.final_url);
        let (paint, image_failures) = self.compose(&render_tree, fetcher);

        Ok(BrowserPage {
            requested_url: requested_url.to_string(),
            final_url: response.final_url,
            status: response.status,
            source,
            document,
            render_tree,
            paint,
            image_failures,
        })
    }

    /// Recompose a previously loaded page for this pipeline's viewport.
    /// Document bytes, parse output, navigation metadata, and history identity
    /// are retained; only layout, paint, links, and image placement change.
    pub fn reflow<F>(&self, page: &BrowserPage, fetcher: &F) -> BrowserPage
    where
        F: BrowserResourceFetcher,
    {
        let (paint, image_failures) = self.compose(&page.render_tree, fetcher);
        let mut reflowed = page.clone();
        reflowed.paint = paint;
        reflowed.image_failures = image_failures;
        reflowed
    }

    fn compose<F>(
        &self,
        render_tree: &BrowserRenderTree,
        fetcher: &F,
    ) -> (HtmlPaintOutput, Vec<HtmlImageResourceError>)
    where
        F: BrowserResourceFetcher,
    {
        let mut paint = html_render_tree_to_paint(
            render_tree,
            self.theme,
            self.viewport,
            self.measurer,
            self.shaper,
            self.metrics,
            self.resolver,
        );
        let image_resolution =
            resolve_scene_image_resources_with_mosaic_fallback(&paint.scene, &|url: &str| {
                let resource = fetcher.fetch(url)?;
                if !is_success(resource.status) {
                    return Err(format!("HTTP status {}", resource.status));
                }
                Ok(FetchedImage::new(resource.body, resource.media_type))
            });
        paint.scene = image_resolution.scene;
        (paint, image_resolution.failures)
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
        let navigation = chrome
            .handle_event(BrowserChromeEvent::Navigate, &session, false)
            .expect("non-empty address should navigate");
        assert_eq!(navigation, BrowserNavigation::Navigate(page_url.into()));
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
                navigation_disabled: false,
            }
        );
        assert_eq!(
            chrome.handle_event(BrowserChromeEvent::Back, &session, false),
            Some(BrowserNavigation::Back)
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
        assert!(disabled.navigation_disabled);
    }

    #[test]
    fn mosaic_chrome_event_names_match_the_generated_bridge_contract() {
        let events = [
            BrowserChromeEvent::Back,
            BrowserChromeEvent::Forward,
            BrowserChromeEvent::Home,
            BrowserChromeEvent::Reload,
            BrowserChromeEvent::AddressChange(String::new()),
            BrowserChromeEvent::Navigate,
        ];
        assert_eq!(
            events.map(|event| event.mosaic_name()),
            VENTURE_CHROME_EVENT_NAMES
        );
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
    fn scroll_state_clamps_offsets_and_reacts_to_geometry_changes() {
        let mut scroll = ScrollState::new(100.0, 260.0);
        assert_eq!(scroll.max_offset_y(), 160.0);
        assert_eq!(scroll.scroll_by(-20.0), 0.0);
        assert_eq!(scroll.set_offset_y(75.0), 75.0);
        assert_eq!(scroll.scroll_by(200.0), 160.0);
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
            .chunks_exact(4)
            .any(|pixel| pixel != [192, 192, 192, 255]));

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
