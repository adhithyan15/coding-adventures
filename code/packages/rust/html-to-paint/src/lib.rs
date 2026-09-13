//! Compose the browser HTML render tree, shared layout, and shared paint
//! instruction stages without coupling any of them to a platform host.

use coding_adventures_html_parser::BrowserRenderTree;
use html_to_layout::{
    html_render_tree_to_layout_with_link_state, html_render_tree_to_layout_with_style_context,
    HtmlStyleContext, HtmlTheme,
};
use image_codec_gif::decode_gif;
use image_codec_jpeg::decode_jpeg;
use layout_backgrounds::{BackgroundStyle, Rect as BackgroundRect};
use layout_block::layout_block;
use layout_controls::{positioned_control_state, ControlKind};
use layout_effects::{multiply, transform_point, EffectStyle, Transform2D, IDENTITY};
use layout_ir::{Constraints, Content, ExtValue, PositionedNode, TextMeasurer};
use layout_positioned::{scroll_extent, PositionedStyle};
use layout_to_paint::{layout_to_paint, LayoutToPaintOptions};
use paint_instructions::{
    ImageSrc, PaintBase, PaintClip, PaintImage, PaintInstruction, PaintRect, PaintScene, PaintText,
    TextAlign,
};
use text_interfaces::{FontMetrics, FontResolver, TextShaper};

pub const VERSION: &str = "0.6.0";

const HTML_IMAGE_ALT_METADATA: &str = "html.alt";

/// Bytes fetched by a browser host for one resolved image URI.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FetchedImage {
    pub bytes: Vec<u8>,
    pub media_type: Option<String>,
}

impl FetchedImage {
    pub fn new(bytes: Vec<u8>, media_type: Option<String>) -> Self {
        Self { bytes, media_type }
    }
}

/// Host boundary for synchronously fetching an image resource.
///
/// HTTP, file, cache, and security policy stay outside this package. The
/// returned bytes are decoded here into the shared `PixelContainer` contract.
pub trait HtmlImageFetcher {
    fn fetch(&self, uri: &str) -> Result<FetchedImage, String>;
}

/// Current browser-owned state for one URI-backed image.
///
/// Keeping this contract in the shared composition layer lets a browser paint
/// pending resources without teaching layout or a platform toolkit about
/// request scheduling.
#[derive(Clone, Debug, PartialEq)]
pub enum HtmlImageResource {
    Pending,
    Ready(paint_instructions::PixelContainer),
    Failed(HtmlImageResourceError),
}

/// Host-neutral lookup boundary for incrementally available image resources.
pub trait HtmlImageResolver {
    fn resolve(&self, uri: &str) -> HtmlImageResource;
}

impl<F> HtmlImageResolver for F
where
    F: Fn(&str) -> HtmlImageResource,
{
    fn resolve(&self, uri: &str) -> HtmlImageResource {
        self(uri)
    }
}

impl<F> HtmlImageFetcher for F
where
    F: Fn(&str) -> Result<FetchedImage, String>,
{
    fn fetch(&self, uri: &str) -> Result<FetchedImage, String> {
        self(uri)
    }
}

/// Failure while fetching or decoding a URI-backed paint image.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HtmlImageResourceError {
    Fetch {
        uri: String,
        message: String,
    },
    UnsupportedFormat {
        uri: String,
        media_type: Option<String>,
    },
    Decode {
        uri: String,
        format: &'static str,
        message: String,
    },
}

impl std::fmt::Display for HtmlImageResourceError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Fetch { uri, message } => {
                write!(formatter, "failed to fetch image {uri}: {message}")
            }
            Self::UnsupportedFormat { uri, media_type } => match media_type {
                Some(media_type) => {
                    write!(
                        formatter,
                        "unsupported image format for {uri} ({media_type})"
                    )
                }
                None => write!(formatter, "unsupported image format for {uri}"),
            },
            Self::Decode {
                uri,
                format,
                message,
            } => write!(
                formatter,
                "failed to decode {format} image {uri}: {message}"
            ),
        }
    }
}

impl std::error::Error for HtmlImageResourceError {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SupportedImageFormat {
    Gif,
    Jpeg,
}

/// Viewport and device scale used by the composed HTML paint pipeline.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HtmlPaintViewport {
    pub width: f64,
    pub height: f64,
    pub device_pixel_ratio: f64,
}

impl HtmlPaintViewport {
    pub fn new(width: f64, height: f64, device_pixel_ratio: f64) -> Self {
        Self {
            width,
            height,
            device_pixel_ratio,
        }
    }
}

/// A transformed overflow clip carried into link hit testing.
#[derive(Clone, Debug, PartialEq)]
pub struct LinkClip {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub corners: [(f64, f64); 4],
    pub inverse_transform: Transform2D,
}

impl LinkClip {
    fn contains(&self, x: f64, y: f64) -> bool {
        let (x, y) = transform_point(self.inverse_transform, x, y);
        rounded_rect_contains(x, y, self.x, self.y, self.width, self.height, self.corners)
    }
}

/// A clickable link rectangle in logical document-content coordinates.
#[derive(Clone, Debug, PartialEq)]
pub struct LinkRegion {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub key: Option<String>,
    pub accessible_name: Option<String>,
    pub focus_order: Option<usize>,
    pub tab_index: i32,
    pub top_layer_index: Option<usize>,
    pub image_map_order: Option<ImageMapRegionOrder>,
    pub url: String,
    pub target: Option<String>,
    pub effective_target: Option<String>,
    pub download: Option<String>,
    pub rel_opener: bool,
    pub rel_noopener: bool,
    pub rel_noreferrer: bool,
    /// Optional non-rectangular geometry for a client-side image-map area.
    pub shape: Option<LinkRegionShape>,
    /// Fixed regions stay in viewport coordinates and ignore document scroll.
    pub fixed: bool,
    /// Ancestor overflow clips, including transformed elliptical corners.
    pub clips: Vec<LinkClip>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ImageMapRegionOrder {
    pub image_index: usize,
    pub area_index: usize,
}

/// Image-map geometry retained in pre-transform document coordinates.
#[derive(Clone, Debug, PartialEq)]
pub enum LinkRegionShape {
    Polygon {
        points: Vec<(f64, f64)>,
        inverse_transform: Transform2D,
    },
    Ellipse {
        center_x: f64,
        center_y: f64,
        radius_x: f64,
        radius_y: f64,
        inverse_transform: Transform2D,
    },
}

impl LinkRegionShape {
    fn contains(&self, x: f64, y: f64) -> bool {
        match self {
            Self::Polygon {
                points,
                inverse_transform,
            } => {
                let point = transform_point(*inverse_transform, x, y);
                polygon_contains(points, point)
            }
            Self::Ellipse {
                center_x,
                center_y,
                radius_x,
                radius_y,
                inverse_transform,
            } => {
                let (x, y) = transform_point(*inverse_transform, x, y);
                *radius_x > 0.0
                    && *radius_y > 0.0
                    && ((x - center_x) / radius_x).powi(2) + ((y - center_y) / radius_y).powi(2)
                        <= 1.0
            }
        }
    }
}

impl LinkRegion {
    /// Return whether a content-space point falls inside the region.
    ///
    /// The right and bottom edges are exclusive so adjacent regions do not
    /// both claim a point on their shared boundary.
    pub fn contains(&self, x: f64, y: f64) -> bool {
        x.is_finite()
            && y.is_finite()
            && x >= self.x
            && x < self.x + self.width
            && y >= self.y
            && y < self.y + self.height
            && self.shape.as_ref().is_none_or(|shape| shape.contains(x, y))
            && self.clips.iter().all(|clip| clip.contains(x, y))
    }
}

fn polygon_contains(points: &[(f64, f64)], point: (f64, f64)) -> bool {
    if points.len() < 3 {
        return false;
    }
    let mut inside = false;
    let mut previous = points[points.len() - 1];
    for &current in points {
        let crosses = (current.1 > point.1) != (previous.1 > point.1)
            && point.0
                < (previous.0 - current.0) * (point.1 - current.1) / (previous.1 - current.1)
                    + current.0;
        if crosses {
            inside = !inside;
        }
        previous = current;
    }
    inside
}

/// A host-neutral form-control hit region in logical document coordinates.
#[derive(Clone, Debug, PartialEq)]
pub struct ControlRegion {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub key: String,
    pub accessible_name: Option<String>,
    pub focus_order: Option<usize>,
    pub tab_index: i32,
    pub top_layer_index: Option<usize>,
    pub kind: ControlKind,
    pub disabled: bool,
    /// Label regions focus or activate their associated control rather than
    /// placing a text caret relative to the label box.
    pub label_activation: bool,
    pub fixed: bool,
    pub clips: Vec<LinkClip>,
}

/// A host-neutral hit region for the first summary of a details disclosure.
#[derive(Clone, Debug, PartialEq)]
pub struct DisclosureRegion {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub key: String,
    pub focus_order: Option<usize>,
    pub tab_index: i32,
    pub top_layer_index: Option<usize>,
    pub disclosure_index: usize,
    pub open: bool,
    pub fixed: bool,
    pub clips: Vec<LinkClip>,
}

/// Geometry and semantics for an authored focus target that is not a native
/// link, form control, or details summary.
#[derive(Clone, Debug, PartialEq)]
pub struct FocusRegion {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub key: String,
    pub role: String,
    pub accessible_name: Option<String>,
    pub editing_mode: Option<String>,
    pub editing_host: bool,
    pub focus_order: usize,
    pub tab_index: i32,
    pub top_layer_index: Option<usize>,
    pub fixed: bool,
    pub clips: Vec<LinkClip>,
}

/// Geometry for one open dialog or popover surface in shared top-layer order.
#[derive(Clone, Debug, PartialEq)]
pub struct TopLayerRegion {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub key: String,
    pub top_layer_index: usize,
    pub kind: String,
    pub modal: bool,
    pub fixed: bool,
    pub clips: Vec<LinkClip>,
}

impl TopLayerRegion {
    pub fn contains(&self, x: f64, y: f64) -> bool {
        x.is_finite()
            && y.is_finite()
            && x >= self.x
            && x < self.x + self.width
            && y >= self.y
            && y < self.y + self.height
            && self.clips.iter().all(|clip| clip.contains(x, y))
    }
}

impl DisclosureRegion {
    pub fn contains(&self, x: f64, y: f64) -> bool {
        x.is_finite()
            && y.is_finite()
            && x >= self.x
            && x < self.x + self.width
            && y >= self.y
            && y < self.y + self.height
            && self.clips.iter().all(|clip| clip.contains(x, y))
    }
}

impl ControlRegion {
    pub fn contains(&self, x: f64, y: f64) -> bool {
        x.is_finite()
            && y.is_finite()
            && x >= self.x
            && x < self.x + self.width
            && y >= self.y
            && y < self.y + self.height
            && self.clips.iter().all(|clip| clip.contains(x, y))
    }
}

impl FocusRegion {
    pub fn contains(&self, x: f64, y: f64) -> bool {
        x.is_finite()
            && y.is_finite()
            && x >= self.x
            && x < self.x + self.width
            && y >= self.y
            && y < self.y + self.height
            && self.clips.iter().all(|clip| clip.contains(x, y))
    }
}

/// Geometry, link hit regions, and paint output for a browser host.
#[derive(Clone, Debug, PartialEq)]
pub struct HtmlPaintOutput {
    pub positioned: PositionedNode,
    pub links: Vec<LinkRegion>,
    pub controls: Vec<ControlRegion>,
    pub disclosures: Vec<DisclosureRegion>,
    pub focus_regions: Vec<FocusRegion>,
    pub top_layers: Vec<TopLayerRegion>,
    pub scene: PaintScene,
}

/// A scene resolved as far as possible plus recoverable image failures.
#[derive(Clone, Debug, PartialEq)]
pub struct HtmlImageResolution {
    pub scene: PaintScene,
    pub failures: Vec<HtmlImageResourceError>,
    pub pending: Vec<String>,
}

/// Return URI-backed image resources in stable first-paint order.
///
/// Repeated image URLs are deduplicated so the browser scheduler can issue one
/// request and repaint every matching instruction from the same completion.
pub fn scene_image_resource_uris(scene: &PaintScene) -> Vec<String> {
    let mut uris = Vec::new();
    collect_instruction_image_uris(&scene.instructions, &mut uris);
    uris
}

/// Fetch and decode every URI-backed image in a paint scene.
///
/// The input scene is never mutated. Resolution happens in a clone and the
/// fully decoded scene is returned only after every image succeeds, so a
/// failed resource cannot leave the caller with a partially resolved scene.
/// Existing `ImageSrc::Pixels` values pass through unchanged.
pub fn resolve_scene_image_resources<F>(
    scene: &PaintScene,
    fetcher: &F,
) -> Result<PaintScene, HtmlImageResourceError>
where
    F: HtmlImageFetcher,
{
    let mut resolved = scene.clone();
    resolve_instruction_images(&mut resolved.instructions, fetcher)?;
    Ok(resolved)
}

/// Resolve URI-backed images and replace failures with Mosaic-style fallbacks.
///
/// Unlike [`resolve_scene_image_resources`], this tolerant browser path keeps
/// processing after fetch, format, or decode failures. Each failed image is
/// replaced by a clipped one-pixel black border and its HTML `alt` text, while
/// successful images still become decoded pixels. All failures are returned
/// for status-bar or diagnostic reporting by the host.
pub fn resolve_scene_image_resources_with_mosaic_fallback<F>(
    scene: &PaintScene,
    fetcher: &F,
) -> HtmlImageResolution
where
    F: HtmlImageFetcher,
{
    let mut resolved = scene.clone();
    let mut failures = Vec::new();
    resolve_instruction_images_with_mosaic_fallback(
        &mut resolved.instructions,
        fetcher,
        &mut failures,
    );
    HtmlImageResolution {
        scene: resolved,
        failures,
        pending: Vec::new(),
    }
}

/// Resolve the currently available subset of URI-backed images.
///
/// Pending resources receive the same stable geometry and alt-text placeholder
/// as failures but are reported separately and do not become diagnostics.
pub fn resolve_scene_image_resources_incrementally<R>(
    scene: &PaintScene,
    resolver: &R,
) -> HtmlImageResolution
where
    R: HtmlImageResolver,
{
    let mut resolved = scene.clone();
    let mut failures = Vec::new();
    let mut pending = Vec::new();
    resolve_instruction_images_incrementally(
        &mut resolved.instructions,
        resolver,
        &mut failures,
        &mut pending,
    );
    HtmlImageResolution {
        scene: resolved,
        failures,
        pending,
    }
}

/// Convert a browser render tree into positioned geometry and a paint scene.
///
/// The caller supplies both layout measurement and the matching TXT00 paint
/// trio. This keeps platform font selection outside the composition package
/// while making the browser pipeline executable through the paint boundary.
pub fn html_render_tree_to_paint<M, S, FM, R>(
    render_tree: &BrowserRenderTree,
    theme: &HtmlTheme,
    viewport: HtmlPaintViewport,
    measurer: &M,
    shaper: &S,
    metrics: &FM,
    resolver: &R,
) -> HtmlPaintOutput
where
    M: TextMeasurer,
    S: TextShaper,
    FM: FontMetrics<Handle = S::Handle>,
    R: FontResolver<Handle = S::Handle>,
{
    html_render_tree_to_paint_with_link_state(
        render_tree,
        theme,
        &never_visited,
        viewport,
        measurer,
        shaper,
        metrics,
        resolver,
    )
}

/// Compose HTML while resolving visited state through a host-neutral callback.
#[allow(clippy::too_many_arguments)]
pub fn html_render_tree_to_paint_with_link_state<M, S, FM, R, F>(
    render_tree: &BrowserRenderTree,
    theme: &HtmlTheme,
    is_visited: &F,
    viewport: HtmlPaintViewport,
    measurer: &M,
    shaper: &S,
    metrics: &FM,
    resolver: &R,
) -> HtmlPaintOutput
where
    M: TextMeasurer,
    S: TextShaper,
    FM: FontMetrics<Handle = S::Handle>,
    R: FontResolver<Handle = S::Handle>,
    F: Fn(&str) -> bool + ?Sized,
{
    let width = finite_non_negative(viewport.width);
    let viewport_height = finite_non_negative(viewport.height);
    let layout = html_render_tree_to_layout_with_link_state(render_tree, theme, is_visited);
    compose_layout_to_paint(
        layout,
        theme,
        viewport,
        width,
        viewport_height,
        measurer,
        shaper,
        metrics,
        resolver,
    )
}

/// Compose HTML using a pre-parsed UA/author style context.
#[allow(clippy::too_many_arguments)]
pub fn html_render_tree_to_paint_with_style_context<M, S, FM, R, F>(
    render_tree: &BrowserRenderTree,
    context: &HtmlStyleContext,
    is_visited: &F,
    viewport: HtmlPaintViewport,
    measurer: &M,
    shaper: &S,
    metrics: &FM,
    resolver: &R,
) -> HtmlPaintOutput
where
    M: TextMeasurer,
    S: TextShaper,
    FM: FontMetrics<Handle = S::Handle>,
    R: FontResolver<Handle = S::Handle>,
    F: Fn(&str) -> bool + ?Sized,
{
    let width = finite_non_negative(viewport.width);
    let viewport_height = finite_non_negative(viewport.height);
    let layout = html_render_tree_to_layout_with_style_context(render_tree, context, is_visited);
    compose_layout_to_paint(
        layout,
        &context.theme,
        viewport,
        width,
        viewport_height,
        measurer,
        shaper,
        metrics,
        resolver,
    )
}

#[allow(clippy::too_many_arguments)]
fn compose_layout_to_paint<M, S, FM, R>(
    layout: layout_ir::LayoutNode,
    theme: &HtmlTheme,
    viewport: HtmlPaintViewport,
    width: f64,
    viewport_height: f64,
    measurer: &M,
    shaper: &S,
    metrics: &FM,
    resolver: &R,
) -> HtmlPaintOutput
where
    M: TextMeasurer,
    S: TextShaper,
    FM: FontMetrics<Handle = S::Handle>,
    R: FontResolver<Handle = S::Handle>,
{
    let positioned = layout_block(
        &layout,
        Constraints {
            min_width: 0.0,
            max_width: width,
            min_height: 0.0,
            max_height: viewport_height,
        },
        measurer,
    );
    let scene_height = scroll_extent(&positioned).1.max(viewport_height);
    let options = LayoutToPaintOptions {
        width,
        height: scene_height,
        background: theme.page_background,
        device_pixel_ratio: viewport.device_pixel_ratio,
        shaper,
        metrics,
        resolver,
    };
    let mut scene = layout_to_paint(&positioned, &options);
    annotate_html_image_metadata(&positioned, &mut scene);
    let regions = extract_interactive_regions(&positioned);

    HtmlPaintOutput {
        positioned,
        links: regions.links,
        controls: regions.controls,
        disclosures: regions.disclosures,
        focus_regions: regions.focus_regions,
        top_layers: regions.top_layers,
        scene,
    }
}

fn never_visited(_url: &str) -> bool {
    false
}

/// Extract resolved link rectangles while accumulating parent-relative layout
/// coordinates into absolute logical document coordinates.
pub fn extract_link_regions(root: &PositionedNode) -> Vec<LinkRegion> {
    extract_interactive_regions(root).links
}

/// Extract form controls using the same transforms and clips as link regions.
pub fn extract_control_regions(root: &PositionedNode) -> Vec<ControlRegion> {
    extract_interactive_regions(root).controls
}

/// Extract visible details-summary regions in document order.
pub fn extract_disclosure_regions(root: &PositionedNode) -> Vec<DisclosureRegion> {
    extract_interactive_regions(root).disclosures
}

/// Extract open dialog and popover surfaces in shared paint order.
pub fn extract_top_layer_regions(root: &PositionedNode) -> Vec<TopLayerRegion> {
    extract_interactive_regions(root).top_layers
}

/// Extract authored focus targets that do not have a more specific region.
pub fn extract_focus_regions(root: &PositionedNode) -> Vec<FocusRegion> {
    extract_interactive_regions(root).focus_regions
}

struct InteractiveRegions {
    links: Vec<LinkRegion>,
    controls: Vec<ControlRegion>,
    disclosures: Vec<DisclosureRegion>,
    focus_regions: Vec<FocusRegion>,
    top_layers: Vec<TopLayerRegion>,
}

fn extract_interactive_regions(root: &PositionedNode) -> InteractiveRegions {
    let mut links = Vec::new();
    let mut controls = Vec::new();
    let mut disclosures = Vec::new();
    let mut focus_regions = Vec::new();
    let mut top_layers = Vec::new();
    let mut control_targets = Vec::new();
    let mut labels = Vec::new();
    let mut stack = vec![(
        root,
        0.0,
        0.0,
        None::<(f64, f64, f64, f64)>,
        Vec::new(),
        false,
        IDENTITY,
        None::<(usize, String, bool)>,
        None::<usize>,
        false,
    )];

    while let Some((
        node,
        parent_x,
        parent_y,
        inherited_clip,
        inherited_clips,
        inherited_fixed,
        inherited_transform,
        parent_disclosure,
        inherited_top_layer,
        inherited_editable,
    )) = stack.pop()
    {
        let absolute_x = parent_x + node.x;
        let absolute_y = parent_y + node.y;
        let style = PositionedStyle::from_positioned(node);
        let effects = EffectStyle::from_positioned(node);
        let transform = effects
            .resolved_transform(absolute_x, absolute_y, node.width, node.height, 1.0)
            .map(|local| multiply(inherited_transform, local))
            .unwrap_or(inherited_transform);
        let fixed = inherited_fixed || style.position == layout_positioned::Position::Fixed;
        let editing_mode = positioned_html_string(node, "editingMode");
        let explicitly_editable = matches!(editing_mode, Some("plaintext" | "richtext"));
        let editing_host = explicitly_editable && !inherited_editable;
        let current_editable = match editing_mode {
            Some("plaintext" | "richtext") => true,
            Some("false") => false,
            _ => inherited_editable,
        };
        let node_top_layer = positioned_html_int(node, "topLayerIndex")
            .and_then(|value| usize::try_from(value).ok());
        let current_top_layer = node_top_layer.or(inherited_top_layer);
        if let (Some(kind), Some(index)) =
            (positioned_html_string(node, "topLayerKind"), node_top_layer)
        {
            let region = clipped_box(
                transformed_box((absolute_x, absolute_y, node.width, node.height), transform),
                inherited_clip,
            );
            if let Some((x, y, width, height)) =
                region.filter(|(x, y, width, height)| valid_link_box(*x, *y, *width, *height))
            {
                let key = positioned_html_string(node, "id")
                    .filter(|id| !id.is_empty())
                    .map(|id| format!("top-layer:id:{id}"))
                    .unwrap_or_else(|| format!("top-layer:{index}"));
                top_layers.push(TopLayerRegion {
                    x,
                    y,
                    width,
                    height,
                    key,
                    top_layer_index: index,
                    kind: kind.to_string(),
                    modal: positioned_html_bool(node, "topLayerModal").unwrap_or(false),
                    fixed,
                    clips: inherited_clips.clone(),
                });
            }
        }
        let current_disclosure =
            (positioned_html_string(node, "disclosureKind") == Some("details")).then(|| {
                let index = positioned_html_int(node, "disclosureIndex")
                    .and_then(|value| usize::try_from(value).ok())
                    .unwrap_or_default();
                let key = positioned_html_string(node, "id")
                    .filter(|id| !id.is_empty())
                    .map(|id| format!("disclosure:id:{id}"))
                    .unwrap_or_else(|| format!("disclosure:{index}"));
                (
                    index,
                    key,
                    positioned_html_bool(node, "open").unwrap_or(false),
                )
            });
        if positioned_html_string(node, "disclosureKind") == Some("summary") {
            if let Some((disclosure_index, key, open)) = parent_disclosure.as_ref() {
                let region = clipped_box(
                    transformed_box((absolute_x, absolute_y, node.width, node.height), transform),
                    inherited_clip,
                );
                if let Some((x, y, width, height)) =
                    region.filter(|(x, y, width, height)| valid_link_box(*x, *y, *width, *height))
                {
                    disclosures.push(DisclosureRegion {
                        x,
                        y,
                        width,
                        height,
                        key: key.clone(),
                        focus_order: positioned_html_int(node, "focusOrder")
                            .and_then(|value| usize::try_from(value).ok()),
                        tab_index: positioned_html_int(node, "tabIndex")
                            .and_then(|value| i32::try_from(value).ok())
                            .unwrap_or_default(),
                        top_layer_index: current_top_layer,
                        disclosure_index: *disclosure_index,
                        open: *open,
                        fixed,
                        clips: inherited_clips.clone(),
                    });
                }
            }
        }
        let semantic_role = positioned_html_string(node, "role").unwrap_or("generic");
        if let Some(focus_order) = positioned_html_int(node, "focusOrder")
            .and_then(|value| usize::try_from(value).ok())
            .filter(|_| {
                !(semantic_role == "link"
                    && positioned_html_string(node, "href").is_some_and(|href| !href.is_empty()))
                    && positioned_control_state(node).is_none()
                    && positioned_html_string(node, "disclosureKind") != Some("summary")
            })
        {
            let region = clipped_box(
                transformed_box((absolute_x, absolute_y, node.width, node.height), transform),
                inherited_clip,
            );
            if let Some((x, y, width, height)) =
                region.filter(|(x, y, width, height)| valid_link_box(*x, *y, *width, *height))
            {
                let role = if explicitly_editable {
                    "textbox"
                } else {
                    positioned_html_string(node, "authoredRole").unwrap_or(semantic_role)
                };
                let key = positioned_html_string(node, "id")
                    .filter(|id| !id.is_empty())
                    .map(|id| format!("focus:id:{id}"))
                    .unwrap_or_else(|| format!("focus:{focus_order}"));
                focus_regions.push(FocusRegion {
                    x,
                    y,
                    width,
                    height,
                    key,
                    role: role.to_string(),
                    accessible_name: positioned_accessible_name(node),
                    editing_mode: editing_mode.map(str::to_string),
                    editing_host,
                    focus_order,
                    tab_index: positioned_html_int(node, "tabIndex")
                        .and_then(|value| i32::try_from(value).ok())
                        .unwrap_or_default(),
                    top_layer_index: current_top_layer,
                    fixed,
                    clips: inherited_clips.clone(),
                });
            }
        }
        if positioned_html_string(node, "role") == Some("link") {
            if let Some(url) = positioned_html_string(node, "href") {
                let region = clipped_box(
                    transformed_box((absolute_x, absolute_y, node.width, node.height), transform),
                    inherited_clip,
                );
                if let Some((x, y, width, height)) = region.filter(|(x, y, width, height)| {
                    valid_link_box(*x, *y, *width, *height) && !url.is_empty()
                }) {
                    links.push(LinkRegion {
                        x,
                        y,
                        width,
                        height,
                        key: positioned_html_string(node, "id")
                            .filter(|id| !id.is_empty())
                            .map(|id| format!("link:id:{id}"))
                            .or_else(|| {
                                positioned_html_int(node, "focusOrder")
                                    .map(|order| format!("link:{order}"))
                            }),
                        accessible_name: positioned_accessible_name(node),
                        focus_order: positioned_html_int(node, "focusOrder")
                            .and_then(|value| usize::try_from(value).ok()),
                        tab_index: positioned_html_int(node, "tabIndex")
                            .and_then(|value| i32::try_from(value).ok())
                            .unwrap_or_default(),
                        top_layer_index: current_top_layer,
                        image_map_order: None,
                        url: url.to_string(),
                        target: positioned_html_string(node, "target").map(ToOwned::to_owned),
                        effective_target: positioned_html_string(node, "effectiveTarget")
                            .map(ToOwned::to_owned),
                        download: positioned_html_string(node, "download").map(ToOwned::to_owned),
                        rel_opener: positioned_html_bool(node, "relOpener").unwrap_or(false),
                        rel_noopener: positioned_html_bool(node, "relNoopener").unwrap_or(false),
                        rel_noreferrer: positioned_html_bool(node, "relNoreferrer")
                            .unwrap_or(false),
                        shape: None,
                        fixed,
                        clips: inherited_clips.clone(),
                    });
                }
            }
        }
        if positioned_html_string(node, "role") == Some("image") {
            let mut mapped = image_map_link_regions(
                node,
                absolute_x,
                absolute_y,
                transform,
                inherited_clip,
                &inherited_clips,
                fixed,
                current_top_layer,
            );
            // Link hit testing is topmost-first. Reversing here preserves the
            // HTML rule that the first matching area wins when shapes overlap.
            mapped.reverse();
            links.extend(mapped);
        }
        if let Some(control) = positioned_control_state(node) {
            let region = clipped_box(
                transformed_box((absolute_x, absolute_y, node.width, node.height), transform),
                inherited_clip,
            );
            if let Some((x, y, width, height)) = region.filter(|(x, y, width, height)| {
                valid_link_box(*x, *y, *width, *height) && !control.key.is_empty()
            }) {
                let region = ControlRegion {
                    x,
                    y,
                    width,
                    height,
                    key: control.key,
                    accessible_name: positioned_html_string(node, "accessibleName")
                        .map(ToOwned::to_owned),
                    focus_order: positioned_html_int(node, "focusOrder")
                        .and_then(|value| usize::try_from(value).ok()),
                    tab_index: positioned_html_int(node, "tabIndex")
                        .and_then(|value| i32::try_from(value).ok())
                        .unwrap_or_default(),
                    top_layer_index: current_top_layer,
                    kind: control.kind,
                    disabled: control.disabled,
                    label_activation: false,
                    fixed,
                    clips: inherited_clips.clone(),
                };
                if let Some(id) = node.id.as_deref() {
                    if !control_targets
                        .iter()
                        .any(|(candidate, _): &(String, ControlRegion)| candidate == id)
                    {
                        control_targets.push((id.to_string(), region.clone()));
                    }
                }
                controls.push(region);
            }
        }
        if positioned_html_string(node, "role") == Some("label") {
            let target = positioned_html_string(node, "labelFor")
                .map(|id| LabelTarget::Explicit(id.to_string()))
                .or_else(|| first_descendant_control_region(node).map(LabelTarget::Implicit));
            let region = clipped_box(
                transformed_box((absolute_x, absolute_y, node.width, node.height), transform),
                inherited_clip,
            );
            if let (Some(target), Some((x, y, width, height))) = (target, region) {
                if valid_link_box(x, y, width, height) {
                    labels.push(LabelRegion {
                        x,
                        y,
                        width,
                        height,
                        target,
                        fixed,
                        clips: inherited_clips.clone(),
                    });
                }
            }
        }

        let clips_children = style.clips_x() || style.clips_y();
        let child_clip = if clips_children {
            clipped_box(
                transformed_box((absolute_x, absolute_y, node.width, node.height), transform),
                inherited_clip,
            )
        } else {
            inherited_clip
        };
        let mut child_clips = inherited_clips;
        if clips_children {
            if let Some(inverse_transform) = invert_transform(transform) {
                let backgrounds = BackgroundStyle::from_positioned(node);
                child_clips.push(LinkClip {
                    x: absolute_x,
                    y: absolute_y,
                    width: node.width,
                    height: node.height,
                    corners: backgrounds.resolved_corners(BackgroundRect {
                        x: absolute_x,
                        y: absolute_y,
                        width: node.width,
                        height: node.height,
                    }),
                    inverse_transform,
                });
            }
        }
        for child in node.children.iter().rev() {
            stack.push((
                child,
                absolute_x,
                absolute_y,
                child_clip,
                child_clips.clone(),
                fixed,
                transform,
                current_disclosure.clone(),
                current_top_layer,
                current_editable,
            ));
        }
    }

    let mut label_controls = labels
        .into_iter()
        .filter_map(|label| {
            let target = match label.target {
                LabelTarget::Explicit(id) => control_targets
                    .iter()
                    .find(|(candidate, _)| candidate == &id)
                    .map(|(_, region)| region.clone()),
                LabelTarget::Implicit(region) => Some(region),
            }?;
            Some(ControlRegion {
                x: label.x,
                y: label.y,
                width: label.width,
                height: label.height,
                key: target.key,
                accessible_name: target.accessible_name,
                focus_order: target.focus_order,
                tab_index: target.tab_index,
                top_layer_index: target.top_layer_index,
                kind: target.kind,
                disabled: target.disabled,
                label_activation: true,
                fixed: label.fixed,
                clips: label.clips,
            })
        })
        .collect::<Vec<_>>();
    // Direct controls remain topmost when a wrapping label contains its target.
    label_controls.extend(controls);
    InteractiveRegions {
        links,
        controls: label_controls,
        disclosures,
        focus_regions,
        top_layers,
    }
}

#[derive(Clone, Debug)]
struct ImageMapAreaMetadata {
    key: String,
    accessible_name: Option<String>,
    shape: String,
    coords: Option<String>,
    url: String,
    target: Option<String>,
    effective_target: Option<String>,
    download: Option<String>,
    rel_opener: bool,
    rel_noopener: bool,
    rel_noreferrer: bool,
    area_index: usize,
    focus_order: Option<usize>,
    tab_index: i32,
}

#[derive(Clone, Debug)]
struct ImageMapMetadata {
    image_index: usize,
    coordinate_width: Option<f64>,
    coordinate_height: Option<f64>,
    areas: Vec<ImageMapAreaMetadata>,
}

#[allow(clippy::too_many_arguments)]
fn image_map_link_regions(
    node: &PositionedNode,
    absolute_x: f64,
    absolute_y: f64,
    transform: Transform2D,
    inherited_clip: Option<(f64, f64, f64, f64)>,
    inherited_clips: &[LinkClip],
    fixed: bool,
    top_layer_index: Option<usize>,
) -> Vec<LinkRegion> {
    let Some(image_map) = positioned_image_map(node) else {
        return Vec::new();
    };
    if node.width <= 0.0 || node.height <= 0.0 {
        return Vec::new();
    }
    let coordinate_width = image_map.coordinate_width.unwrap_or(node.width).max(1.0);
    let coordinate_height = image_map.coordinate_height.unwrap_or(node.height).max(1.0);
    let scale_x = node.width / coordinate_width;
    let scale_y = node.height / coordinate_height;
    let Some(inverse_transform) = invert_transform(transform) else {
        return Vec::new();
    };
    let image_bounds =
        transformed_box((absolute_x, absolute_y, node.width, node.height), transform);
    let mut image_clips = inherited_clips.to_vec();
    image_clips.push(LinkClip {
        x: absolute_x,
        y: absolute_y,
        width: node.width,
        height: node.height,
        corners: [(0.0, 0.0); 4],
        inverse_transform,
    });

    image_map
        .areas
        .into_iter()
        .filter_map(|area| {
            let shape = image_map_shape(
                &area,
                absolute_x,
                absolute_y,
                coordinate_width,
                coordinate_height,
                scale_x,
                scale_y,
                inverse_transform,
            )?;
            let bounds = image_map_shape_bounds(&shape, transform)?;
            let bounds = clipped_box(bounds, Some(image_bounds))?;
            let (x, y, width, height) = clipped_box(bounds, inherited_clip)?;
            if !valid_link_box(x, y, width, height) {
                return None;
            }
            Some(LinkRegion {
                x,
                y,
                width,
                height,
                key: Some(area.key),
                accessible_name: area.accessible_name,
                focus_order: area.focus_order,
                tab_index: area.tab_index,
                top_layer_index,
                image_map_order: Some(ImageMapRegionOrder {
                    image_index: image_map.image_index,
                    area_index: area.area_index,
                }),
                url: area.url,
                target: area.target,
                effective_target: area.effective_target,
                download: area.download,
                rel_opener: area.rel_opener,
                rel_noopener: area.rel_noopener,
                rel_noreferrer: area.rel_noreferrer,
                shape: Some(shape),
                fixed,
                clips: image_clips.clone(),
            })
        })
        .collect()
}

#[allow(clippy::too_many_arguments)]
fn image_map_shape(
    area: &ImageMapAreaMetadata,
    image_x: f64,
    image_y: f64,
    coordinate_width: f64,
    coordinate_height: f64,
    scale_x: f64,
    scale_y: f64,
    inverse_transform: Transform2D,
) -> Option<LinkRegionShape> {
    let coords = area
        .coords
        .as_deref()
        .map(parse_image_map_coords)
        .unwrap_or_default();
    let point = |x: f64, y: f64| (image_x + x * scale_x, image_y + y * scale_y);
    match area.shape.as_str() {
        "default" => Some(LinkRegionShape::Polygon {
            points: vec![
                point(0.0, 0.0),
                point(coordinate_width, 0.0),
                point(coordinate_width, coordinate_height),
                point(0.0, coordinate_height),
            ],
            inverse_transform,
        }),
        "circle" | "circ" if coords.len() >= 3 && coords[2] > 0.0 => {
            Some(LinkRegionShape::Ellipse {
                center_x: image_x + coords[0] * scale_x,
                center_y: image_y + coords[1] * scale_y,
                radius_x: coords[2] * scale_x.abs(),
                radius_y: coords[2] * scale_y.abs(),
                inverse_transform,
            })
        }
        "poly" | "polygon" if coords.len() >= 6 => {
            let points = coords
                .as_chunks::<2>()
                .0
                .iter()
                .map(|pair| point(pair[0], pair[1]))
                .collect();
            Some(LinkRegionShape::Polygon {
                points,
                inverse_transform,
            })
        }
        "rect" | "rectangle" | "" if coords.len() >= 4 => {
            let left = coords[0].min(coords[2]);
            let right = coords[0].max(coords[2]);
            let top = coords[1].min(coords[3]);
            let bottom = coords[1].max(coords[3]);
            (right > left && bottom > top).then(|| LinkRegionShape::Polygon {
                points: vec![
                    point(left, top),
                    point(right, top),
                    point(right, bottom),
                    point(left, bottom),
                ],
                inverse_transform,
            })
        }
        _ => None,
    }
}

fn parse_image_map_coords(value: &str) -> Vec<f64> {
    value
        .split(|character: char| character == ',' || character.is_ascii_whitespace())
        .filter(|part| !part.is_empty())
        .filter_map(|part| part.parse::<f64>().ok().filter(|value| value.is_finite()))
        .collect()
}

fn image_map_shape_bounds(
    shape: &LinkRegionShape,
    transform: Transform2D,
) -> Option<(f64, f64, f64, f64)> {
    match shape {
        LinkRegionShape::Polygon { points, .. } => {
            let transformed = points
                .iter()
                .map(|&(x, y)| transform_point(transform, x, y))
                .collect::<Vec<_>>();
            point_bounds(&transformed)
        }
        LinkRegionShape::Ellipse {
            center_x,
            center_y,
            radius_x,
            radius_y,
            ..
        } => Some(transformed_box(
            (
                center_x - radius_x,
                center_y - radius_y,
                radius_x * 2.0,
                radius_y * 2.0,
            ),
            transform,
        )),
    }
}

fn point_bounds(points: &[(f64, f64)]) -> Option<(f64, f64, f64, f64)> {
    let first = *points.first()?;
    let (mut min_x, mut max_x, mut min_y, mut max_y) = (first.0, first.0, first.1, first.1);
    for &(x, y) in &points[1..] {
        min_x = min_x.min(x);
        max_x = max_x.max(x);
        min_y = min_y.min(y);
        max_y = max_y.max(y);
    }
    Some((min_x, min_y, max_x - min_x, max_y - min_y))
}

fn positioned_image_map(node: &PositionedNode) -> Option<ImageMapMetadata> {
    let ExtValue::Map(map) = node.ext.get("imageMap")? else {
        return None;
    };
    let coordinate_width = ext_float(map.get("coordinateWidth"));
    let coordinate_height = ext_float(map.get("coordinateHeight"));
    let image_index = ext_usize(map.get("imageIndex"))?;
    let ExtValue::List(areas) = map.get("areas")? else {
        return None;
    };
    let areas = areas
        .iter()
        .filter_map(|area| {
            let ExtValue::Map(values) = area else {
                return None;
            };
            Some(ImageMapAreaMetadata {
                key: ext_string(values.get("key"))?.to_string(),
                accessible_name: ext_string(values.get("name")).map(ToOwned::to_owned),
                shape: ext_string(values.get("shape"))?.to_string(),
                coords: ext_string(values.get("coords")).map(ToOwned::to_owned),
                url: ext_string(values.get("href"))?.to_string(),
                target: ext_string(values.get("target")).map(ToOwned::to_owned),
                effective_target: ext_string(values.get("effectiveTarget")).map(ToOwned::to_owned),
                download: ext_string(values.get("download")).map(ToOwned::to_owned),
                rel_opener: ext_bool(values.get("relOpener")),
                rel_noopener: ext_bool(values.get("relNoopener")),
                rel_noreferrer: ext_bool(values.get("relNoreferrer")),
                area_index: ext_usize(values.get("areaIndex"))?,
                focus_order: ext_usize(values.get("focusOrder")),
                tab_index: ext_i32(values.get("tabIndex")).unwrap_or_default(),
            })
        })
        .collect();
    Some(ImageMapMetadata {
        image_index,
        coordinate_width,
        coordinate_height,
        areas,
    })
}

fn ext_string(value: Option<&ExtValue>) -> Option<&str> {
    match value? {
        ExtValue::Str(value) => Some(value),
        _ => None,
    }
}

fn ext_float(value: Option<&ExtValue>) -> Option<f64> {
    match value? {
        ExtValue::Float(value) if value.is_finite() => Some(*value),
        ExtValue::Int(value) => Some(*value as f64),
        _ => None,
    }
}

fn ext_bool(value: Option<&ExtValue>) -> bool {
    matches!(value, Some(ExtValue::Bool(true)))
}

fn ext_usize(value: Option<&ExtValue>) -> Option<usize> {
    match value? {
        ExtValue::Int(value) => usize::try_from(*value).ok(),
        _ => None,
    }
}

fn ext_i32(value: Option<&ExtValue>) -> Option<i32> {
    match value? {
        ExtValue::Int(value) => i32::try_from(*value).ok(),
        _ => None,
    }
}

#[derive(Clone, Debug)]
enum LabelTarget {
    Explicit(String),
    Implicit(ControlRegion),
}

#[derive(Clone, Debug)]
struct LabelRegion {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    target: LabelTarget,
    fixed: bool,
    clips: Vec<LinkClip>,
}

fn first_descendant_control_region(node: &PositionedNode) -> Option<ControlRegion> {
    for child in &node.children {
        if let Some(control) = positioned_control_state(child) {
            return Some(ControlRegion {
                x: 0.0,
                y: 0.0,
                width: 0.0,
                height: 0.0,
                key: control.key,
                accessible_name: positioned_html_string(node, "accessibleName")
                    .map(ToOwned::to_owned),
                focus_order: None,
                tab_index: 0,
                top_layer_index: None,
                kind: control.kind,
                disabled: control.disabled,
                label_activation: false,
                fixed: false,
                clips: Vec::new(),
            });
        }
        if let Some(control) = first_descendant_control_region(child) {
            return Some(control);
        }
    }
    None
}

fn invert_transform(transform: Transform2D) -> Option<Transform2D> {
    let [a, b, c, d, e, f] = transform;
    let determinant = a * d - b * c;
    if !determinant.is_finite() || determinant.abs() <= f64::EPSILON {
        return None;
    }
    Some([
        d / determinant,
        -b / determinant,
        -c / determinant,
        a / determinant,
        (c * f - d * e) / determinant,
        (b * e - a * f) / determinant,
    ])
}

#[allow(clippy::too_many_arguments)]
fn rounded_rect_contains(
    point_x: f64,
    point_y: f64,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    corners: [(f64, f64); 4],
) -> bool {
    if point_x < x || point_x >= x + width || point_y < y || point_y >= y + height {
        return false;
    }
    let right = x + width;
    let bottom = y + height;
    let tests = [
        (
            x + corners[0].0,
            y + corners[0].1,
            corners[0],
            point_x < x + corners[0].0 && point_y < y + corners[0].1,
        ),
        (
            right - corners[1].0,
            y + corners[1].1,
            corners[1],
            point_x >= right - corners[1].0 && point_y < y + corners[1].1,
        ),
        (
            right - corners[2].0,
            bottom - corners[2].1,
            corners[2],
            point_x >= right - corners[2].0 && point_y >= bottom - corners[2].1,
        ),
        (
            x + corners[3].0,
            bottom - corners[3].1,
            corners[3],
            point_x < x + corners[3].0 && point_y >= bottom - corners[3].1,
        ),
    ];
    tests
        .into_iter()
        .all(|(center_x, center_y, (rx, ry), applies)| {
            !applies
                || rx <= 0.0
                || ry <= 0.0
                || ((point_x - center_x) / rx).powi(2) + ((point_y - center_y) / ry).powi(2) <= 1.0
        })
}

fn transformed_box(rect: (f64, f64, f64, f64), transform: Transform2D) -> (f64, f64, f64, f64) {
    let corners = [
        transform_point(transform, rect.0, rect.1),
        transform_point(transform, rect.0 + rect.2, rect.1),
        transform_point(transform, rect.0, rect.1 + rect.3),
        transform_point(transform, rect.0 + rect.2, rect.1 + rect.3),
    ];
    let min_x = corners
        .iter()
        .map(|point| point.0)
        .fold(f64::INFINITY, f64::min);
    let max_x = corners
        .iter()
        .map(|point| point.0)
        .fold(f64::NEG_INFINITY, f64::max);
    let min_y = corners
        .iter()
        .map(|point| point.1)
        .fold(f64::INFINITY, f64::min);
    let max_y = corners
        .iter()
        .map(|point| point.1)
        .fold(f64::NEG_INFINITY, f64::max);
    (min_x, min_y, max_x - min_x, max_y - min_y)
}

/// Hit-test a viewport-space point against logical document link regions.
///
/// `scroll_y` is added to the viewport y coordinate to recover document
/// content coordinates. Negative or non-finite scroll offsets are treated as
/// zero; scrolling policy and clamping remain the browser host's concern.
pub fn hit_test_link(
    regions: &[LinkRegion],
    viewport_x: f64,
    viewport_y: f64,
    scroll_y: f64,
) -> Option<&LinkRegion> {
    let scroll_y = finite_non_negative(scroll_y);
    regions.iter().rev().find(|region| {
        let y = if region.fixed {
            viewport_y
        } else {
            viewport_y + scroll_y
        };
        region.contains(viewport_x, y)
    })
}

/// Hit-test a viewport-space point against form controls, topmost first.
pub fn hit_test_control(
    regions: &[ControlRegion],
    viewport_x: f64,
    viewport_y: f64,
    scroll_y: f64,
) -> Option<&ControlRegion> {
    let scroll_y = finite_non_negative(scroll_y);
    regions.iter().rev().find(|region| {
        let y = if region.fixed {
            viewport_y
        } else {
            viewport_y + scroll_y
        };
        region.contains(viewport_x, y)
    })
}

/// Hit-test a viewport-space point against details summaries, topmost first.
pub fn hit_test_disclosure(
    regions: &[DisclosureRegion],
    viewport_x: f64,
    viewport_y: f64,
    scroll_y: f64,
) -> Option<&DisclosureRegion> {
    let scroll_y = finite_non_negative(scroll_y);
    regions.iter().rev().find(|region| {
        let y = if region.fixed {
            viewport_y
        } else {
            viewport_y + scroll_y
        };
        region.contains(viewport_x, y)
    })
}

/// Hit-test a viewport-space point against open top-layer surfaces.
pub fn hit_test_top_layer(
    regions: &[TopLayerRegion],
    viewport_x: f64,
    viewport_y: f64,
    scroll_y: f64,
) -> Option<&TopLayerRegion> {
    let scroll_y = finite_non_negative(scroll_y);
    regions.iter().rev().find(|region| {
        let y = if region.fixed {
            viewport_y
        } else {
            viewport_y + scroll_y
        };
        region.contains(viewport_x, y)
    })
}

fn clipped_box(
    rect: (f64, f64, f64, f64),
    clip: Option<(f64, f64, f64, f64)>,
) -> Option<(f64, f64, f64, f64)> {
    let Some((clip_x, clip_y, clip_width, clip_height)) = clip else {
        return Some(rect);
    };
    let x = rect.0.max(clip_x);
    let y = rect.1.max(clip_y);
    let right = (rect.0 + rect.2).min(clip_x + clip_width);
    let bottom = (rect.1 + rect.3).min(clip_y + clip_height);
    (right > x && bottom > y).then_some((x, y, right - x, bottom - y))
}

fn finite_non_negative(value: f64) -> f64 {
    if value.is_finite() {
        value.max(0.0)
    } else {
        0.0
    }
}

fn valid_link_box(x: f64, y: f64, width: f64, height: f64) -> bool {
    x.is_finite()
        && y.is_finite()
        && width.is_finite()
        && height.is_finite()
        && width > 0.0
        && height > 0.0
}

fn positioned_html_string<'a>(node: &'a PositionedNode, key: &str) -> Option<&'a str> {
    let ExtValue::Map(values) = node.ext.get("html")? else {
        return None;
    };
    let ExtValue::Str(value) = values.get(key)? else {
        return None;
    };
    Some(value)
}

fn positioned_accessible_name(node: &PositionedNode) -> Option<String> {
    fn collect(node: &PositionedNode, parts: &mut Vec<String>) {
        if let Some(name) = positioned_html_string(node, "accessibleName") {
            let name = name.split_whitespace().collect::<Vec<_>>().join(" ");
            if !name.is_empty() {
                parts.push(name);
                return;
            }
        }
        if let Some(Content::Text(text)) = &node.content {
            let text = text.value.split_whitespace().collect::<Vec<_>>().join(" ");
            if !text.is_empty() {
                parts.push(text);
            }
        }
        for child in &node.children {
            collect(child, parts);
        }
    }

    let mut parts = Vec::new();
    collect(node, &mut parts);
    (!parts.is_empty()).then(|| parts.join(" "))
}

fn positioned_html_bool(node: &PositionedNode, key: &str) -> Option<bool> {
    let ExtValue::Map(values) = node.ext.get("html")? else {
        return None;
    };
    let ExtValue::Bool(value) = values.get(key)? else {
        return None;
    };
    Some(*value)
}

fn positioned_html_int(node: &PositionedNode, key: &str) -> Option<i64> {
    let ExtValue::Map(values) = node.ext.get("html")? else {
        return None;
    };
    let ExtValue::Int(value) = values.get(key)? else {
        return None;
    };
    Some(*value)
}

fn resolve_instruction_images<F>(
    instructions: &mut [PaintInstruction],
    fetcher: &F,
) -> Result<(), HtmlImageResourceError>
where
    F: HtmlImageFetcher,
{
    for instruction in instructions {
        match instruction {
            PaintInstruction::Image(image) => {
                if !matches!(image.src, ImageSrc::Uri(_)) {
                    continue;
                }
                image.src = ImageSrc::Pixels(fetch_and_decode_image(image, fetcher)?);
            }
            PaintInstruction::Group(group) => {
                resolve_instruction_images(&mut group.children, fetcher)?;
            }
            PaintInstruction::Layer(layer) => {
                resolve_instruction_images(&mut layer.children, fetcher)?;
            }
            PaintInstruction::Clip(clip) => {
                resolve_instruction_images(&mut clip.children, fetcher)?;
            }
            _ => {}
        }
    }
    Ok(())
}

fn resolve_instruction_images_with_mosaic_fallback<F>(
    instructions: &mut [PaintInstruction],
    fetcher: &F,
    failures: &mut Vec<HtmlImageResourceError>,
) where
    F: HtmlImageFetcher,
{
    for instruction in instructions {
        let replacement = match instruction {
            PaintInstruction::Image(image) if matches!(image.src, ImageSrc::Uri(_)) => {
                match fetch_and_decode_image(image, fetcher) {
                    Ok(pixels) => {
                        image.src = ImageSrc::Pixels(pixels);
                        None
                    }
                    Err(error) => {
                        let fallback = mosaic_broken_image_fallback(image);
                        failures.push(error);
                        Some(fallback)
                    }
                }
            }
            PaintInstruction::Group(group) => {
                resolve_instruction_images_with_mosaic_fallback(
                    &mut group.children,
                    fetcher,
                    failures,
                );
                None
            }
            PaintInstruction::Layer(layer) => {
                resolve_instruction_images_with_mosaic_fallback(
                    &mut layer.children,
                    fetcher,
                    failures,
                );
                None
            }
            PaintInstruction::Clip(clip) => {
                resolve_instruction_images_with_mosaic_fallback(
                    &mut clip.children,
                    fetcher,
                    failures,
                );
                None
            }
            _ => None,
        };
        if let Some(replacement) = replacement {
            *instruction = replacement;
        }
    }
}

fn collect_instruction_image_uris(instructions: &[PaintInstruction], uris: &mut Vec<String>) {
    for instruction in instructions {
        match instruction {
            PaintInstruction::Image(image) => {
                if let ImageSrc::Uri(uri) = &image.src {
                    if !uris.contains(uri) {
                        uris.push(uri.clone());
                    }
                }
            }
            PaintInstruction::Group(group) => {
                collect_instruction_image_uris(&group.children, uris);
            }
            PaintInstruction::Layer(layer) => {
                collect_instruction_image_uris(&layer.children, uris);
            }
            PaintInstruction::Clip(clip) => {
                collect_instruction_image_uris(&clip.children, uris);
            }
            _ => {}
        }
    }
}

fn resolve_instruction_images_incrementally<R>(
    instructions: &mut [PaintInstruction],
    resolver: &R,
    failures: &mut Vec<HtmlImageResourceError>,
    pending: &mut Vec<String>,
) where
    R: HtmlImageResolver,
{
    for instruction in instructions {
        let replacement = match instruction {
            PaintInstruction::Image(image) => {
                let ImageSrc::Uri(uri) = &image.src else {
                    continue;
                };
                match resolver.resolve(uri) {
                    HtmlImageResource::Ready(pixels) => {
                        image.src = ImageSrc::Pixels(pixels);
                        None
                    }
                    HtmlImageResource::Failed(error) => {
                        failures.push(error);
                        Some(mosaic_broken_image_fallback(image))
                    }
                    HtmlImageResource::Pending => {
                        if !pending.contains(uri) {
                            pending.push(uri.clone());
                        }
                        Some(mosaic_broken_image_fallback(image))
                    }
                }
            }
            PaintInstruction::Group(group) => {
                resolve_instruction_images_incrementally(
                    &mut group.children,
                    resolver,
                    failures,
                    pending,
                );
                None
            }
            PaintInstruction::Layer(layer) => {
                resolve_instruction_images_incrementally(
                    &mut layer.children,
                    resolver,
                    failures,
                    pending,
                );
                None
            }
            PaintInstruction::Clip(clip) => {
                resolve_instruction_images_incrementally(
                    &mut clip.children,
                    resolver,
                    failures,
                    pending,
                );
                None
            }
            _ => None,
        };
        if let Some(replacement) = replacement {
            *instruction = replacement;
        }
    }
}

fn fetch_and_decode_image<F>(
    image: &PaintImage,
    fetcher: &F,
) -> Result<paint_instructions::PixelContainer, HtmlImageResourceError>
where
    F: HtmlImageFetcher,
{
    let ImageSrc::Uri(uri) = &image.src else {
        unreachable!("pixel-backed images are filtered before resource resolution");
    };
    let fetched = fetcher
        .fetch(uri)
        .map_err(|message| HtmlImageResourceError::Fetch {
            uri: uri.clone(),
            message,
        })?;
    decode_image_resource(uri, fetched)
}

/// Decode fetched image bytes before delivering a browser completion.
pub fn decode_image_resource(
    uri: &str,
    fetched: FetchedImage,
) -> Result<paint_instructions::PixelContainer, HtmlImageResourceError> {
    match detect_image_format(uri, &fetched) {
        Some(SupportedImageFormat::Gif) => {
            decode_gif(&fetched.bytes).map_err(|message| HtmlImageResourceError::Decode {
                uri: uri.to_string(),
                format: "GIF",
                message,
            })
        }
        Some(SupportedImageFormat::Jpeg) => {
            decode_jpeg(&fetched.bytes).map_err(|message| HtmlImageResourceError::Decode {
                uri: uri.to_string(),
                format: "JPEG",
                message,
            })
        }
        None => Err(HtmlImageResourceError::UnsupportedFormat {
            uri: uri.to_string(),
            media_type: fetched.media_type,
        }),
    }
}

fn mosaic_broken_image_fallback(image: &PaintImage) -> PaintInstruction {
    let width = image.width.max(0.0);
    let height = image.height.max(0.0);
    let mut children = vec![PaintInstruction::Rect(PaintRect {
        base: PaintBase::default(),
        x: image.x + 0.5,
        y: image.y + 0.5,
        width: (width - 1.0).max(0.0),
        height: (height - 1.0).max(0.0),
        fill: None,
        stroke: Some("#000000".into()),
        stroke_width: Some(1.0),
        corner_radius: None,
        stroke_dash: None,
        stroke_dash_offset: None,
    })];

    let alt = image
        .base
        .metadata
        .as_ref()
        .and_then(|metadata| metadata.get(HTML_IMAGE_ALT_METADATA));
    if let Some(alt) = alt.filter(|alt| !alt.is_empty()) {
        let font_size = 12.0_f64.min((height - 4.0).max(1.0));
        children.push(PaintInstruction::Text(PaintText {
            base: PaintBase::default(),
            x: image.x + 3.0,
            y: image.y + 2.0 + font_size,
            text: alt.clone(),
            font_ref: Some("serif".into()),
            font_size,
            fill: Some("#000000".into()),
            text_align: Some(TextAlign::Left),
        }));
    }

    PaintInstruction::Clip(PaintClip {
        base: image.base.clone(),
        x: image.x,
        y: image.y,
        width,
        height,
        path: None,
        children,
    })
}

fn annotate_html_image_metadata(positioned: &PositionedNode, scene: &mut PaintScene) {
    let mut image_alts = Vec::new();
    let mut stack = vec![positioned];
    while let Some(node) = stack.pop() {
        if matches!(node.content, Some(Content::Image(_))) {
            image_alts.push(positioned_html_string(node, "alt").map(ToOwned::to_owned));
        }
        for child in node.children.iter().rev() {
            stack.push(child);
        }
    }

    let mut image_alts = image_alts.into_iter();
    for instruction in &mut scene.instructions {
        let PaintInstruction::Image(image) = instruction else {
            continue;
        };
        let Some(alt) = image_alts.next().flatten() else {
            continue;
        };
        image
            .base
            .metadata
            .get_or_insert_with(Default::default)
            .insert(HTML_IMAGE_ALT_METADATA.into(), alt);
    }
}

fn detect_image_format(uri: &str, fetched: &FetchedImage) -> Option<SupportedImageFormat> {
    if fetched.bytes.starts_with(b"GIF87a") || fetched.bytes.starts_with(b"GIF89a") {
        return Some(SupportedImageFormat::Gif);
    }
    if fetched.bytes.starts_with(&[0xff, 0xd8, 0xff]) {
        return Some(SupportedImageFormat::Jpeg);
    }

    if let Some(media_type) = fetched.media_type.as_deref() {
        let media_type = media_type.split(';').next().unwrap_or("").trim();
        if media_type.eq_ignore_ascii_case("image/gif") {
            return Some(SupportedImageFormat::Gif);
        }
        if media_type.eq_ignore_ascii_case("image/jpeg")
            || media_type.eq_ignore_ascii_case("image/jpg")
            || media_type.eq_ignore_ascii_case("image/pjpeg")
        {
            return Some(SupportedImageFormat::Jpeg);
        }
    }

    let path = uri
        .split(['?', '#'])
        .next()
        .unwrap_or(uri)
        .to_ascii_lowercase();
    if path.ends_with(".gif") {
        Some(SupportedImageFormat::Gif)
    } else if path.ends_with(".jpg") || path.ends_with(".jpeg") {
        Some(SupportedImageFormat::Jpeg)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use coding_adventures_html_parser::parse_browser_render_tree;
    use html_to_layout::mosaic_html_theme;
    use layout_ir::{Color, FontSpec, MeasureResult};
    use paint_instructions::PixelContainer;
    use text_interfaces::{
        Direction, FontQuery, FontResolutionError, Glyph, ShapeOptions, ShapedRun, ShapedText,
        ShapingError,
    };

    struct PendingImageResolver;

    impl HtmlImageResolver for PendingImageResolver {
        fn resolve(&self, _uri: &str) -> HtmlImageResource {
            HtmlImageResource::Pending
        }
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
    fn normalizes_invalid_viewport_dimensions() {
        let render = parse_browser_render_tree("<p>Hello</p>").unwrap();
        let output = html_render_tree_to_paint(
            &render,
            &mosaic_html_theme(),
            HtmlPaintViewport::new(f64::NAN, f64::NEG_INFINITY, 1.0),
            &MonoMeasurer,
            &FakeShaper,
            &FakeMetrics,
            &FakeResolver,
        );

        assert_eq!(output.scene.width, 0.0);
        assert!(output.scene.height >= 0.0);
    }

    #[test]
    fn extracts_visible_details_summary_regions() {
        let render = parse_browser_render_tree(
            "<details id='shipping'><summary>Shipping</summary><p>Hidden</p></details>\
             <details open><summary>Billing</summary><p>Shown</p></details>",
        )
        .unwrap();
        let output = html_render_tree_to_paint(
            &render,
            &mosaic_html_theme(),
            HtmlPaintViewport::new(320.0, 200.0, 1.0),
            &MonoMeasurer,
            &FakeShaper,
            &FakeMetrics,
            &FakeResolver,
        );

        assert_eq!(output.disclosures.len(), 2);
        assert_eq!(output.disclosures[0].key, "disclosure:id:shipping");
        assert!(!output.disclosures[0].open);
        assert!(output.disclosures[1].open);
        assert!(hit_test_disclosure(
            &output.disclosures,
            output.disclosures[0].x + 1.0,
            output.disclosures[0].y + 1.0,
            0.0,
        )
        .is_some());
    }

    #[test]
    fn canned_html_reaches_a_drawable_paint_scene() {
        let render = parse_browser_render_tree(
            "<base href='https://example.test/assets/' target='reports'><h1>Mosaic lives</h1>\
             <p>The browser pipeline is <a href='../status' rel='noopener' download='status.html'>connected</a>.</p>\
             <img src='logo.gif' width='32' height='24'>",
        )
        .unwrap();
        let theme = mosaic_html_theme();
        let output = html_render_tree_to_paint(
            &render,
            &theme,
            HtmlPaintViewport::new(240.0, 40.0, 1.0),
            &MonoMeasurer,
            &FakeShaper,
            &FakeMetrics,
            &FakeResolver,
        );

        assert_eq!(output.scene.width, 240.0);
        assert!(output.scene.height >= output.positioned.height);
        assert!(output.scene.height > 40.0);
        assert_eq!(output.scene.background, "rgb(192, 192, 192)");

        let glyph_runs: Vec<_> = output
            .scene
            .instructions
            .iter()
            .filter_map(|instruction| match instruction {
                PaintInstruction::GlyphRun(run) => Some(run),
                _ => None,
            })
            .collect();
        assert!(!glyph_runs.is_empty());
        assert!(glyph_runs
            .iter()
            .any(|run| run.fill == Some(color_css(theme.link_color))));
        assert!(output.scene.instructions.iter().any(|instruction| matches!(
            instruction,
            PaintInstruction::Image(image)
                if image.src == ImageSrc::Uri("https://example.test/assets/logo.gif".into())
                    && image.width == 32.0
                    && image.height == 24.0
        )));
        let link = find_positioned_by_html_role(&output.positioned, "link").unwrap();
        assert_eq!(
            positioned_html_string(link, "href"),
            Some("https://example.test/status")
        );
        assert_eq!(output.links.len(), 1);
        assert_eq!(output.links[0].url, "https://example.test/status");
        assert_eq!(output.links[0].effective_target.as_deref(), Some("reports"));
        assert_eq!(output.links[0].download.as_deref(), Some("status.html"));
        assert!(output.links[0].rel_noopener);
        assert_eq!(
            hit_test_link(
                &output.links,
                output.links[0].x + output.links[0].width / 2.0,
                output.links[0].y + output.links[0].height / 2.0,
                0.0,
            ),
            Some(&output.links[0])
        );
        assert!(positioned_texts(&output.positioned).contains(&"connected"));
    }

    #[test]
    fn visited_link_state_reaches_glyphs_and_underlines() {
        let render = parse_browser_render_tree(
            "<base href='https://example.test/'><p>\
             <a href='seen'>seen link</a> <a href='new'>new link</a></p>",
        )
        .unwrap();
        let theme = mosaic_html_theme();
        let output = html_render_tree_to_paint_with_link_state(
            &render,
            &theme,
            &|url| url == "https://example.test/seen",
            HtmlPaintViewport::new(240.0, 40.0, 1.0),
            &MonoMeasurer,
            &FakeShaper,
            &FakeMetrics,
            &FakeResolver,
        );
        let visited = color_css(theme.visited_link_color);
        let unvisited = color_css(theme.link_color);

        assert!(output.scene.instructions.iter().any(|instruction| matches!(
            instruction,
            PaintInstruction::GlyphRun(run) if run.fill.as_deref() == Some(visited.as_str())
        )));
        assert!(output.scene.instructions.iter().any(|instruction| matches!(
            instruction,
            PaintInstruction::GlyphRun(run) if run.fill.as_deref() == Some(unvisited.as_str())
        )));
        assert!(output.scene.instructions.iter().any(|instruction| matches!(
            instruction,
            PaintInstruction::Rect(rect)
                if rect.fill.as_deref() == Some(visited.as_str()) && rect.height > 0.0
        )));
        assert!(output.scene.instructions.iter().any(|instruction| matches!(
            instruction,
            PaintInstruction::Rect(rect)
                if rect.fill.as_deref() == Some(unvisited.as_str()) && rect.height > 0.0
        )));
    }

    #[test]
    fn wrapped_link_exposes_one_tight_hit_region_per_line() {
        let render = parse_browser_render_tree(
            "<p><a href='https://example.test/long'>one two three four five six</a></p>",
        )
        .unwrap();
        let output = html_render_tree_to_paint(
            &render,
            &mosaic_html_theme(),
            HtmlPaintViewport::new(90.0, 40.0, 1.0),
            &MonoMeasurer,
            &FakeShaper,
            &FakeMetrics,
            &FakeResolver,
        );

        assert!(output.links.len() >= 3, "regions: {:?}", output.links);
        assert!(output.links.windows(2).all(|pair| pair[0].y < pair[1].y));
        assert!(output
            .links
            .iter()
            .all(|region| region.width > 0.0 && region.width < 90.0));
        for region in &output.links {
            assert_eq!(region.url, "https://example.test/long");
            assert_eq!(
                hit_test_link(
                    &output.links,
                    region.x + region.width / 2.0,
                    region.y + region.height / 2.0,
                    0.0,
                ),
                Some(region)
            );
        }
    }

    #[test]
    fn preformatted_text_preserves_spaces_and_hard_line_geometry() {
        let render = parse_browser_render_tree("<pre>one  two\n\nthree</pre>").unwrap();
        let output = html_render_tree_to_paint(
            &render,
            &mosaic_html_theme(),
            HtmlPaintViewport::new(200.0, 40.0, 1.0),
            &MonoMeasurer,
            &FakeShaper,
            &FakeMetrics,
            &FakeResolver,
        );

        let first = find_positioned_text(&output.positioned, "one  two").unwrap();
        let second = find_positioned_text(&output.positioned, "three").unwrap();
        assert_eq!(first.x, second.x);
        assert!(second.y >= first.y + first.height * 2.0);
        assert!(positioned_texts(&output.positioned).contains(&"one  two"));
    }

    #[test]
    fn extraction_accumulates_parent_coordinates_and_skips_empty_boxes() {
        let visible = positioned_link(5.0, 7.0, 20.0, 10.0, "https://example.test/visible");
        let empty = positioned_link(30.0, 7.0, 0.0, 10.0, "https://example.test/empty");
        let root = PositionedNode {
            x: 10.0,
            y: 20.0,
            width: 100.0,
            height: 100.0,
            id: None,
            content: None,
            children: vec![visible, empty],
            ext: Default::default(),
        };

        assert_eq!(
            extract_link_regions(&root),
            vec![LinkRegion {
                x: 15.0,
                y: 27.0,
                width: 20.0,
                height: 10.0,
                key: None,
                accessible_name: None,
                focus_order: None,
                tab_index: 0,
                top_layer_index: None,
                image_map_order: None,
                url: "https://example.test/visible".into(),
                target: None,
                effective_target: None,
                download: None,
                rel_opener: false,
                rel_noopener: false,
                rel_noreferrer: false,
                shape: None,
                fixed: false,
                clips: Vec::new(),
            }]
        );
    }

    #[test]
    fn extraction_transforms_link_regions_with_their_visual_boxes() {
        let mut link = positioned_link(5.0, 7.0, 20.0, 10.0, "https://example.test/moved");
        let effects = layout_effects::EffectStyle {
            transform: layout_effects::translation(12.0, 4.0),
            transform_origin: layout_effects::TransformOrigin {
                x: layout_effects::OriginComponent::percent(0.0),
                y: layout_effects::OriginComponent::percent(0.0),
            },
            ..layout_effects::EffectStyle::default()
        };
        link.ext.insert("effects".into(), effects.to_ext());

        assert_eq!(
            extract_link_regions(&link),
            vec![LinkRegion {
                x: 17.0,
                y: 11.0,
                width: 20.0,
                height: 10.0,
                key: None,
                accessible_name: None,
                focus_order: None,
                tab_index: 0,
                top_layer_index: None,
                image_map_order: None,
                url: "https://example.test/moved".into(),
                target: None,
                effective_target: None,
                download: None,
                rel_opener: false,
                rel_noopener: false,
                rel_noreferrer: false,
                shape: None,
                fixed: false,
                clips: Vec::new(),
            }]
        );
    }

    #[test]
    fn hit_testing_applies_scroll_and_uses_half_open_edges() {
        let region = LinkRegion {
            x: 10.0,
            y: 80.0,
            width: 30.0,
            height: 12.0,
            key: None,
            accessible_name: None,
            focus_order: None,
            tab_index: 0,
            top_layer_index: None,
            image_map_order: None,
            url: "https://example.test/next".into(),
            target: None,
            effective_target: None,
            download: None,
            rel_opener: false,
            rel_noopener: false,
            rel_noreferrer: false,
            shape: None,
            fixed: false,
            clips: Vec::new(),
        };

        assert_eq!(
            hit_test_link(std::slice::from_ref(&region), 10.0, 20.0, 60.0),
            Some(&region)
        );
        assert_eq!(
            hit_test_link(std::slice::from_ref(&region), 40.0, 20.0, 60.0),
            None
        );
        assert_eq!(
            hit_test_link(std::slice::from_ref(&region), 10.0, 32.0, 60.0),
            None
        );
        assert_eq!(
            hit_test_link(std::slice::from_ref(&region), f64::NAN, 20.0, 60.0),
            None
        );
    }

    #[test]
    fn rounded_overflow_clip_excludes_link_corner_hits() {
        let link = positioned_link(0.0, 0.0, 100.0, 80.0, "https://example.test/rounded");
        let mut root = PositionedNode {
            x: 10.0,
            y: 20.0,
            width: 100.0,
            height: 80.0,
            id: None,
            content: None,
            children: vec![link],
            ext: Default::default(),
        };
        root.ext.insert(
            "positioned".into(),
            layout_positioned::PositionedStyle {
                overflow_x: layout_positioned::Overflow::Hidden,
                overflow_y: layout_positioned::Overflow::Hidden,
                ..Default::default()
            }
            .to_ext(),
        );
        let backgrounds = layout_backgrounds::BackgroundStyle {
            corners: [layout_backgrounds::CornerRadius {
                x: layout_backgrounds::LengthPercent::length(20.0),
                y: layout_backgrounds::LengthPercent::length(20.0),
            }; 4],
            ..Default::default()
        };
        root.ext.insert("backgrounds".into(), backgrounds.to_ext());

        let regions = extract_link_regions(&root);
        assert_eq!(regions.len(), 1);
        assert!(!regions[0].contains(10.0, 20.0));
        assert!(regions[0].contains(30.0, 40.0));
    }

    #[test]
    fn canned_html_rasterizes_to_rgba_pixels_with_cairo() {
        let render = parse_browser_render_tree(
            "<h1>Mosaic</h1><p>Hello <a href='https://example.test/'>world</a>.</p>",
        )
        .unwrap();
        let output = html_render_tree_to_paint(
            &render,
            &mosaic_html_theme(),
            HtmlPaintViewport::new(200.0, 96.0, 1.0),
            &MonoMeasurer,
            &FakeShaper,
            &FakeMetrics,
            &FakeResolver,
        );

        let pixels = paint_vm_cairo::render(&output.scene)
            .expect("canned HTML paint scene should rasterize");
        assert_eq!(pixels.width, 200);
        assert_eq!(pixels.height, output.scene.height.ceil() as u32);
        assert_eq!(
            pixels.data.len(),
            pixels.width as usize * pixels.height as usize * 4
        );
        assert!(pixels
            .data
            .as_chunks::<4>()
            .0
            .iter()
            .any(|pixel| pixel != &[192, 192, 192, 255]));
    }

    #[test]
    fn canned_html_image_fetches_decodes_and_rasterizes_with_cairo() {
        let render = parse_browser_render_tree(
            "<base href='https://example.test/assets/'><p>Inline image:</p>\
             <img src='logo.gif' width='32' height='24'>",
        )
        .unwrap();
        let output = html_render_tree_to_paint(
            &render,
            &mosaic_html_theme(),
            HtmlPaintViewport::new(160.0, 96.0, 1.0),
            &MonoMeasurer,
            &FakeShaper,
            &FakeMetrics,
            &FakeResolver,
        );

        let mut source = PixelContainer::new(2, 2);
        source.fill(255, 0, 255, 255);
        let gif = image_codec_gif::encode_gif(&source);
        let resolved = resolve_scene_image_resources(&output.scene, &|uri: &str| {
            assert_eq!(uri, "https://example.test/assets/logo.gif");
            Ok(FetchedImage::new(gif.clone(), Some("image/gif".into())))
        })
        .expect("canned image should resolve");

        let image = first_image(&resolved.instructions).expect("resolved image instruction");
        assert!(matches!(
            &image.src,
            ImageSrc::Pixels(pixels)
                if pixels.width == 2
                    && pixels.height == 2
                    && pixels.pixel_at(0, 0) == (255, 0, 255, 255)
        ));

        let pixels =
            paint_vm_cairo::render(&resolved).expect("resolved canned HTML image should rasterize");
        let sample_x = (image.x + image.width / 2.0).floor() as u32;
        let sample_y = (image.y + image.height / 2.0).floor() as u32;
        let (red, green, blue, alpha) = pixels.pixel_at(sample_x, sample_y);
        assert!(red > 200 && green < 50 && blue > 200 && alpha == 255);
    }

    #[test]
    fn jpeg_resources_are_detected_from_bytes_and_decoded() {
        let mut source = PixelContainer::new(2, 2);
        source.fill(20, 100, 180, 255);
        let jpeg = image_codec_jpeg::encode_jpeg(&source);
        let scene = scene_with_uri_image("https://example.test/photo.bin");

        let resolved = resolve_scene_image_resources(&scene, &|_: &str| {
            Ok(FetchedImage::new(
                jpeg.clone(),
                Some("application/octet-stream".into()),
            ))
        })
        .expect("JPEG signature should select the JPEG decoder");

        let image = first_image(&resolved.instructions).unwrap();
        assert!(matches!(
            &image.src,
            ImageSrc::Pixels(pixels) if pixels.width == 2 && pixels.height == 2
        ));
    }

    #[test]
    fn image_resolution_failure_leaves_the_input_scene_unchanged() {
        let scene = scene_with_uri_image("https://example.test/missing.gif");
        let original = scene.clone();

        let error = resolve_scene_image_resources(&scene, &|_: &str| Err("offline".to_string()))
            .unwrap_err();

        assert_eq!(
            error,
            HtmlImageResourceError::Fetch {
                uri: "https://example.test/missing.gif".into(),
                message: "offline".into(),
            }
        );
        assert_eq!(scene, original);
    }

    #[test]
    fn incremental_resolution_deduplicates_pending_urls_without_failures() {
        let render = parse_browser_render_tree(
            "<base href='https://example.test/assets/'>\
             <img src='logo.gif' alt='first' width='20' height='10'>\
             <img src='other.gif' alt='second' width='20' height='10'>\
             <img src='logo.gif' alt='third' width='20' height='10'>",
        )
        .unwrap();
        let output = html_render_tree_to_paint(
            &render,
            &mosaic_html_theme(),
            HtmlPaintViewport::new(100.0, 40.0, 1.0),
            &MonoMeasurer,
            &FakeShaper,
            &FakeMetrics,
            &FakeResolver,
        );

        assert_eq!(
            scene_image_resource_uris(&output.scene),
            vec![
                "https://example.test/assets/logo.gif",
                "https://example.test/assets/other.gif"
            ]
        );
        let resolution =
            resolve_scene_image_resources_incrementally(&output.scene, &PendingImageResolver);
        assert_eq!(
            resolution.pending,
            vec![
                "https://example.test/assets/logo.gif",
                "https://example.test/assets/other.gif"
            ]
        );
        assert!(resolution.failures.is_empty());
        assert!(first_image(&resolution.scene.instructions).is_none());
    }

    #[test]
    fn failed_html_image_rasterizes_mosaic_alt_text_fallback() {
        let render = parse_browser_render_tree(
            "<base href='https://example.test/assets/'>\
             <p>Image follows:</p><img src='missing.gif' alt='Mosaic logo' width='80' height='24'>",
        )
        .unwrap();
        let output = html_render_tree_to_paint(
            &render,
            &mosaic_html_theme(),
            HtmlPaintViewport::new(180.0, 96.0, 1.0),
            &MonoMeasurer,
            &FakeShaper,
            &FakeMetrics,
            &FakeResolver,
        );

        let image = first_image(&output.scene.instructions).expect("HTML image instruction");
        assert_eq!(
            image
                .base
                .metadata
                .as_ref()
                .and_then(|metadata| metadata.get(HTML_IMAGE_ALT_METADATA))
                .map(String::as_str),
            Some("Mosaic logo")
        );
        let fallback_x = image.x.floor() as u32;
        let fallback_y = image.y.floor() as u32;
        let fallback_width = image.width.ceil().max(1.0) as u32;
        let fallback_height = image.height.ceil().max(1.0) as u32;

        let resolution =
            resolve_scene_image_resources_with_mosaic_fallback(&output.scene, &|_: &str| {
                Err("offline".into())
            });
        assert_eq!(
            resolution.failures,
            vec![HtmlImageResourceError::Fetch {
                uri: "https://example.test/assets/missing.gif".into(),
                message: "offline".into(),
            }]
        );
        assert!(!contains_uri_image(&resolution.scene.instructions));
        assert!(contains_text(&resolution.scene.instructions, "Mosaic logo"));

        let pixels = paint_vm_cairo::render(&resolution.scene)
            .expect("broken-image fallback should rasterize");
        let contains_dark_fallback_pixel = (fallback_y
            ..fallback_y
                .saturating_add(fallback_height)
                .min(pixels.height))
            .any(|y| {
                (fallback_x..fallback_x.saturating_add(fallback_width).min(pixels.width)).any(|x| {
                    let (red, green, blue, alpha) = pixels.pixel_at(x, y);
                    red < 192 && green < 192 && blue < 192 && alpha == 255
                })
            });
        assert!(contains_dark_fallback_pixel);
    }

    fn color_css(color: Color) -> String {
        format!("rgb({}, {}, {})", color.r, color.g, color.b)
    }

    fn positioned_texts(node: &PositionedNode) -> Vec<&str> {
        let mut texts = Vec::new();
        if let Some(Content::Text(text)) = &node.content {
            texts.push(text.value.as_str());
        }
        for child in &node.children {
            texts.extend(positioned_texts(child));
        }
        texts
    }

    fn find_positioned_text<'a>(
        node: &'a PositionedNode,
        value: &str,
    ) -> Option<&'a PositionedNode> {
        if matches!(&node.content, Some(Content::Text(text)) if text.value == value) {
            return Some(node);
        }
        node.children
            .iter()
            .find_map(|child| find_positioned_text(child, value))
    }

    fn find_positioned_by_html_role<'a>(
        node: &'a PositionedNode,
        role: &str,
    ) -> Option<&'a PositionedNode> {
        if positioned_html_string(node, "role") == Some(role) {
            return Some(node);
        }
        node.children
            .iter()
            .find_map(|child| find_positioned_by_html_role(child, role))
    }

    fn positioned_html_string<'a>(node: &'a PositionedNode, key: &str) -> Option<&'a str> {
        super::positioned_html_string(node, key)
    }

    #[test]
    fn form_controls_emit_clipped_hit_regions_and_ignore_disabled_activation() {
        let render = parse_browser_render_tree(
            "<div style='overflow:hidden;width:260px'>\
             <input id='query' value='hello'><button disabled>Save</button></div>",
        )
        .unwrap();
        let output = html_render_tree_to_paint(
            &render,
            &mosaic_html_theme(),
            HtmlPaintViewport::new(300.0, 120.0, 1.0),
            &MonoMeasurer,
            &FakeShaper,
            &FakeMetrics,
            &FakeResolver,
        );

        assert_eq!(output.controls.len(), 2);
        assert_eq!(output.controls[0].key, "control:0:id:query");
        assert!(!output.controls[0].disabled);
        assert!(output.controls[1].disabled);
        let first = &output.controls[0];
        assert_eq!(
            hit_test_control(&output.controls, first.x + 1.0, first.y + 1.0, 0.0)
                .map(|region| region.key.as_str()),
            Some("control:0:id:query")
        );
    }

    #[test]
    fn top_layer_surfaces_emit_transformed_hit_regions() {
        let render = parse_browser_render_tree(
            "<dialog id='confirm' open aria-modal='true' style='transform:translate(8px, 6px)'>Confirm</dialog>\
             <div id='menu' popover='manual'>Menu</div>",
        )
        .unwrap();
        let output = html_render_tree_to_paint(
            &render,
            &mosaic_html_theme(),
            HtmlPaintViewport::new(320.0, 180.0, 1.0),
            &MonoMeasurer,
            &FakeShaper,
            &FakeMetrics,
            &FakeResolver,
        );

        assert_eq!(output.top_layers.len(), 1);
        let dialog = &output.top_layers[0];
        assert_eq!(dialog.key, "top-layer:id:confirm");
        assert_eq!(dialog.kind, "dialog");
        assert!(dialog.modal && dialog.fixed);
        assert_eq!(
            hit_test_top_layer(&output.top_layers, dialog.x + 1.0, dialog.y + 1.0, 0.0)
                .map(|region| region.key.as_str()),
            Some("top-layer:id:confirm")
        );
    }

    #[test]
    fn explicit_and_implicit_labels_share_control_hit_regions() {
        let render = parse_browser_render_tree(
            "<label for='query'>Search</label><input id='query' value='hello'>\
             <label>Accept<input id='accept' type='checkbox'></label>\
             <label for='missing'>Missing</label>",
        )
        .unwrap();
        let output = html_render_tree_to_paint(
            &render,
            &mosaic_html_theme(),
            HtmlPaintViewport::new(420.0, 160.0, 1.0),
            &MonoMeasurer,
            &FakeShaper,
            &FakeMetrics,
            &FakeResolver,
        );

        let labels = output
            .controls
            .iter()
            .filter(|region| region.label_activation)
            .collect::<Vec<_>>();
        assert_eq!(labels.len(), 2);
        assert_eq!(labels[0].key, "control:0:id:query");
        assert_eq!(labels[1].key, "control:1:id:accept");
        for label in labels {
            assert!(label.width > 0.0 && label.height > 0.0);
            assert_eq!(
                hit_test_control(&output.controls, label.x + 1.0, label.y + 1.0, 0.0,)
                    .map(|region| (region.key.as_str(), region.label_activation)),
                Some((label.key.as_str(), true))
            );
        }
        let direct_checkbox = output
            .controls
            .iter()
            .find(|region| region.key == "control:1:id:accept" && !region.label_activation)
            .unwrap();
        assert_eq!(
            hit_test_control(
                &output.controls,
                direct_checkbox.x + direct_checkbox.width / 2.0,
                direct_checkbox.y + direct_checkbox.height / 2.0,
                0.0,
            )
            .map(|region| region.label_activation),
            Some(false),
            "a nested control remains topmost inside its wrapping label"
        );
    }

    fn positioned_link(x: f64, y: f64, width: f64, height: f64, url: &str) -> PositionedNode {
        PositionedNode {
            x,
            y,
            width,
            height,
            id: None,
            content: None,
            children: Vec::new(),
            ext: std::collections::HashMap::from([(
                "html".into(),
                ExtValue::Map(std::collections::HashMap::from([
                    ("role".into(), ExtValue::Str("link".into())),
                    ("href".into(), ExtValue::Str(url.into())),
                ])),
            )]),
        }
    }

    fn scene_with_uri_image(uri: &str) -> PaintScene {
        let mut scene = PaintScene::new(8.0, 8.0);
        scene.instructions.push(PaintInstruction::Image(PaintImage {
            base: PaintBase::default(),
            x: 0.0,
            y: 0.0,
            width: 8.0,
            height: 8.0,
            src: ImageSrc::Uri(uri.into()),
            opacity: None,
        }));
        scene
    }

    fn first_image(instructions: &[PaintInstruction]) -> Option<&PaintImage> {
        for instruction in instructions {
            match instruction {
                PaintInstruction::Image(image) => return Some(image),
                PaintInstruction::Group(group) => {
                    if let Some(image) = first_image(&group.children) {
                        return Some(image);
                    }
                }
                PaintInstruction::Layer(layer) => {
                    if let Some(image) = first_image(&layer.children) {
                        return Some(image);
                    }
                }
                PaintInstruction::Clip(clip) => {
                    if let Some(image) = first_image(&clip.children) {
                        return Some(image);
                    }
                }
                _ => {}
            }
        }
        None
    }

    fn contains_uri_image(instructions: &[PaintInstruction]) -> bool {
        instructions.iter().any(|instruction| match instruction {
            PaintInstruction::Image(image) => matches!(image.src, ImageSrc::Uri(_)),
            PaintInstruction::Group(group) => contains_uri_image(&group.children),
            PaintInstruction::Layer(layer) => contains_uri_image(&layer.children),
            PaintInstruction::Clip(clip) => contains_uri_image(&clip.children),
            _ => false,
        })
    }

    fn contains_text(instructions: &[PaintInstruction], expected: &str) -> bool {
        instructions.iter().any(|instruction| match instruction {
            PaintInstruction::Text(text) => text.text == expected,
            PaintInstruction::Group(group) => contains_text(&group.children, expected),
            PaintInstruction::Layer(layer) => contains_text(&layer.children, expected),
            PaintInstruction::Clip(clip) => contains_text(&clip.children, expected),
            _ => false,
        })
    }

    #[test]
    fn image_map_shapes_scale_transform_and_keep_first_match_precedence() {
        let render = parse_browser_render_tree(
            "<img src='plan.gif' width='200' height='100' usemap='#zones' \
             style='width:400px;height:200px;transform:translate(10px, 5px)'>\
             <map name='zones'>\
               <area id='circle' shape='circle' coords='50,50,25' href='circle.html' alt='Circle'>\
               <area id='overlap' shape='rect' coords='0,0,100,100' href='rect.html' alt='Rectangle'>\
               <area id='triangle' shape='poly' coords='120,10,190,50,120,90' href='poly.html' alt='Triangle'>\
               <area id='fallback' shape='default' href='fallback.html' alt='Fallback'>\
             </map>",
        )
        .unwrap();
        let output = html_render_tree_to_paint(
            &render,
            &mosaic_html_theme(),
            HtmlPaintViewport::new(500.0, 260.0, 1.0),
            &MonoMeasurer,
            &FakeShaper,
            &FakeMetrics,
            &FakeResolver,
        );
        let circle = output
            .links
            .iter()
            .find(|link| link.key.as_deref() == Some("image-map:index:0:map:zones:id:circle"))
            .unwrap();
        assert_eq!(circle.focus_order, Some(0));
        assert_eq!(circle.tab_index, 0);
        assert_eq!(circle.accessible_name.as_deref(), Some("Circle"));
        assert!(circle.contains(
            circle.x + circle.width / 2.0,
            circle.y + circle.height / 2.0
        ));
        assert!(!circle.contains(circle.x, circle.y));

        let hit = hit_test_link(
            &output.links,
            circle.x + circle.width / 2.0,
            circle.y + circle.height / 2.0,
            0.0,
        )
        .unwrap();
        assert_eq!(hit.url, "circle.html", "the first overlapping area wins");
        let triangle = output
            .links
            .iter()
            .find(|link| link.key.as_deref() == Some("image-map:index:0:map:zones:id:triangle"))
            .unwrap();
        assert!(triangle.contains(
            triangle.x + triangle.width / 2.0,
            triangle.y + triangle.height / 2.0
        ));
        assert!(!triangle.contains(triangle.x, triangle.y));
        let fallback = output
            .links
            .iter()
            .find(|link| link.key.as_deref() == Some("image-map:index:0:map:zones:id:fallback"))
            .unwrap();
        let fallback_hit = hit_test_link(
            &output.links,
            fallback.x + fallback.width * 0.55,
            fallback.y + fallback.height * 0.5,
            0.0,
        )
        .unwrap();
        assert_eq!(fallback_hit.url, "fallback.html");
    }

    #[test]
    fn generic_focus_regions_keep_authored_semantics_and_transformed_geometry() {
        let render = parse_browser_render_tree(
            "<div id='action' role='button' tabindex='2' aria-label='Run report' \
             style='width:80px;height:30px;transform:translate(12px, 7px)'>Run</div>\
             <section id='editor' contenteditable aria-label='Notes'>Draft</section>\
             <a id='anchor' tabindex='0'>Anchor target</a>\
             <div id='skipped' tabindex='-1'>Skip</div>",
        )
        .unwrap();
        let output = html_render_tree_to_paint(
            &render,
            &mosaic_html_theme(),
            HtmlPaintViewport::new(300.0, 180.0, 1.0),
            &MonoMeasurer,
            &FakeShaper,
            &FakeMetrics,
            &FakeResolver,
        );
        assert_eq!(output.focus_regions.len(), 3);
        let action = output
            .focus_regions
            .iter()
            .find(|region| region.key == "focus:id:action")
            .unwrap();
        assert_eq!(action.role, "button");
        assert_eq!(action.accessible_name.as_deref(), Some("Run report"));
        assert_eq!(action.tab_index, 2);
        assert!(action.x >= 12.0);
        let editor = output
            .focus_regions
            .iter()
            .find(|region| region.key == "focus:id:editor")
            .unwrap();
        assert_eq!(editor.role, "textbox");
        assert_eq!(editor.accessible_name.as_deref(), Some("Notes"));
        assert!(output
            .focus_regions
            .iter()
            .any(|region| region.key == "focus:id:anchor"));
        assert!(!output
            .focus_regions
            .iter()
            .any(|region| region.key == "focus:id:skipped"));
    }
}
